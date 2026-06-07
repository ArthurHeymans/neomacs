use super::*;
use crate::buffer::{CharLen, CharPos0, EmacsBytePos, EmacsByteRange, LispCharPos1};
use crate::emacs_core::regex::{MatchGroup, char_pos_to_byte, char_pos_to_byte_lisp_string};
use crate::emacs_core::value::{ValueKind, VecLikeType};

// ===========================================================================
// Search / Regex builtins (evaluator-dependent)
// ===========================================================================

/// GNU `search.c:282, 376, 1168, 2053` — every search path reads
/// `Vinhibit_changing_match_data` at the top:
///
///     bool modify_match_data = NILP (Vinhibit_changing_match_data)
///                              && modify_data;
///
/// When the variable is non-nil, the match data must stay pinned to
/// its prior state across the search. Returns `true` when the
/// variable is currently set (i.e. do NOT modify match data).
/// Routes through `dynamic_or_global_symbol_value` so let-bindings
/// and per-buffer overrides are observed, matching the audit #3 fix.
fn read_inhibit_changing_match_data(eval: &super::eval::Context) -> bool {
    dynamic_or_global_symbol_value(eval, "inhibit-changing-match-data").is_some_and(|v| !v.is_nil())
}

fn buffer_byte_to_lisp_char(buf: &crate::buffer::Buffer, byte_pos: EmacsBytePos) -> i64 {
    buf.emacs_byte_pos_to_lisp_char_pos(byte_pos).as_i64()
}

fn buffer_byte_to_char_pos(buf: &crate::buffer::Buffer, byte_pos: EmacsBytePos) -> CharPos0 {
    buf.emacs_byte_pos_to_char_pos_clamped(byte_pos)
}

fn record_buffer_search_success(
    buffers: &mut crate::buffer::BufferManager,
    buffer_id: crate::buffer::BufferId,
    pos: usize,
) -> Result<usize, Flow> {
    buffers
        .goto_buffer_emacs_byte_pos(buffer_id, EmacsBytePos::new(pos))
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    Ok(pos)
}

pub(crate) fn builtin_search_forward(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let case_fold = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|v| !v.is_nil())
        .unwrap_or(true);
    let inhibit_changing = read_inhibit_changing_match_data(eval);
    // GNU search.c:1168 — `search_buffer_non_re` checks
    // `preserve_match_data = NILP(Vinhibit_changing_match_data)` at
    // the top. When set, the search runs against a throwaway
    // match-data slot so the evaluator's match data is untouched.
    let mut throwaway: Option<super::regex::MatchData> = None;
    let md_slot: &mut Option<super::regex::MatchData> = if inhibit_changing {
        &mut throwaway
    } else {
        &mut eval.match_data
    };
    builtin_search_forward_with_state(case_fold, &mut eval.buffers, md_slot, &args)
}

pub(crate) fn builtin_search_forward_with_state(
    case_fold: bool,
    buffers: &mut crate::buffer::BufferManager,
    match_data: &mut Option<super::regex::MatchData>,
    args: &[Value],
) -> EvalResult {
    expect_range_args("search-forward", args, 1, 4)?;
    let pattern = expect_string(&args[0])?;
    let (current_id, opts, start_pt, start_char) =
        current_search_context_in_manager(buffers, args, SearchKind::ForwardLiteral)?;
    if opts.steps == 0 {
        return Ok(Value::fixnum(start_char));
    }

    let mut last_pos = None;
    for _ in 0..opts.steps {
        let result = {
            let buf = buffers
                .get_mut(current_id)
                .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
            match opts.direction {
                SearchDirection::Forward => super::regex::search_forward(
                    buf,
                    &pattern,
                    opts.bound.map(|bound| bound.get()),
                    false,
                    case_fold,
                    match_data,
                ),
                SearchDirection::Backward => super::regex::search_backward(
                    buf,
                    &pattern,
                    opts.bound.map(|bound| bound.get()),
                    false,
                    case_fold,
                    match_data,
                ),
            }
        };
        match result {
            Ok(Some(pos)) => {
                last_pos = Some(record_buffer_search_success(buffers, current_id, pos)?)
            }
            Ok(None) => {
                // regex::search_* with `noerror = false` never returns None.
                return Err(signal("search-failed", vec![Value::string(pattern)]));
            }
            Err(_) => {
                return handle_search_failure_in_manager(
                    buffers,
                    current_id,
                    Value::string(pattern),
                    opts,
                    start_pt,
                    SearchErrorKind::NotFound,
                );
            }
        }
    }

    let end = last_pos.expect("search loop should produce at least one match");
    buffer_byte_to_char_result_in_manager(buffers, current_id, EmacsBytePos::new(end))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SearchDirection {
    Forward,
    Backward,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SearchNoErrorMode {
    Signal,
    KeepPoint,
    MoveToBound,
}

#[derive(Clone, Copy)]
enum SearchKind {
    ForwardLiteral,
    BackwardLiteral,
    ForwardRegexp,
    BackwardRegexp,
}

#[derive(Clone, Copy)]
enum SearchErrorKind {
    NotFound,
}

#[derive(Clone, Copy)]
struct SearchOptions {
    bound: Option<EmacsBytePos>,
    direction: SearchDirection,
    noerror_mode: SearchNoErrorMode,
    steps: usize,
}

#[derive(Clone, Copy)]
struct SearchBound {
    lisp_pos: LispCharPos1,
    byte_pos: EmacsBytePos,
}

fn search_count_arg(args: &[Value]) -> Result<i64, Flow> {
    match args.get(3) {
        None => Ok(1),
        Some(v) if v.is_nil() => Ok(1),
        Some(v) => match v.kind() {
            ValueKind::Fixnum(n) => Ok(n),
            _ => Err(signal(
                "wrong-type-argument",
                vec![Value::symbol("fixnump"), *v],
            )),
        },
    }
}

fn search_bound_in_manager(
    buffers: &crate::buffer::BufferManager,
    buf: &crate::buffer::Buffer,
    value: &Value,
) -> Result<SearchBound, Flow> {
    let lisp_pos = LispCharPos1::new(super::buffers::expect_integer_or_marker_in_buffers(
        buffers, value,
    )?);
    Ok(SearchBound {
        lisp_pos,
        byte_pos: buf.lisp_pos_to_accessible_emacs_byte_pos(lisp_pos),
    })
}

fn parse_search_options_in_manager(
    buffers: &crate::buffer::BufferManager,
    buf: &crate::buffer::Buffer,
    args: &[Value],
    kind: SearchKind,
) -> Result<SearchOptions, Flow> {
    let count = search_count_arg(args)?;
    let noerror_mode = match args.get(2) {
        None => SearchNoErrorMode::Signal,
        Some(v) if v.is_nil() => SearchNoErrorMode::Signal,
        Some(v) if v.is_t() => SearchNoErrorMode::KeepPoint,
        Some(_) => SearchNoErrorMode::MoveToBound,
    };
    let bound = match args.get(1) {
        Some(v) if !v.is_nil() => Some(search_bound_in_manager(buffers, buf, v)?),
        _ => None,
    };

    let direction = match kind {
        SearchKind::ForwardLiteral | SearchKind::ForwardRegexp => {
            if count > 0 {
                SearchDirection::Forward
            } else {
                SearchDirection::Backward
            }
        }
        SearchKind::BackwardLiteral | SearchKind::BackwardRegexp => {
            if count < 0 {
                SearchDirection::Forward
            } else {
                SearchDirection::Backward
            }
        }
    };
    let steps = count.unsigned_abs() as usize;

    if let Some(limit) = bound.map(|bound| bound.lisp_pos.as_i64()) {
        let point_lisp = buffer_byte_to_lisp_char(buf, buf.point_emacs_byte_pos());
        match direction {
            SearchDirection::Forward if limit < point_lisp => {
                return Err(signal(
                    "error",
                    vec![Value::string("Invalid search bound (wrong side of point)")],
                ));
            }
            SearchDirection::Backward if limit > point_lisp => {
                return Err(signal(
                    "error",
                    vec![Value::string("Invalid search bound (wrong side of point)")],
                ));
            }
            _ => {}
        }
    }

    Ok(SearchOptions {
        bound: bound.map(|bound| bound.byte_pos),
        direction,
        noerror_mode,
        steps,
    })
}

fn current_search_context_in_manager(
    buffers: &crate::buffer::BufferManager,
    args: &[Value],
    kind: SearchKind,
) -> Result<(crate::buffer::BufferId, SearchOptions, EmacsBytePos, i64), Flow> {
    let current_id = buffers
        .current_buffer_id()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    let buf = buffers
        .get(current_id)
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    let opts = parse_search_options_in_manager(buffers, buf, args, kind)?;
    let start_pt = buf.point_emacs_byte_pos();
    let start_char = buffer_byte_to_lisp_char(buf, start_pt);
    Ok((current_id, opts, start_pt, start_char))
}

fn buffer_byte_to_char_result_in_manager(
    buffers: &crate::buffer::BufferManager,
    buffer_id: crate::buffer::BufferId,
    byte: EmacsBytePos,
) -> EvalResult {
    let buf = buffers
        .get(buffer_id)
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    Ok(Value::fixnum(buffer_byte_to_lisp_char(buf, byte)))
}

fn search_failure_position(buf: &crate::buffer::Buffer, opts: SearchOptions) -> EmacsBytePos {
    let accessible = buf.accessible_emacs_byte_region();
    match opts.bound {
        Some(limit) => accessible.clamp(limit),
        None => match opts.direction {
            SearchDirection::Forward => accessible.end(),
            SearchDirection::Backward => accessible.start(),
        },
    }
}

fn handle_search_failure_in_manager(
    buffers: &mut crate::buffer::BufferManager,
    buffer_id: crate::buffer::BufferId,
    pattern: Value,
    opts: SearchOptions,
    start_pt: EmacsBytePos,
    kind: SearchErrorKind,
) -> EvalResult {
    match kind {
        SearchErrorKind::NotFound => match opts.noerror_mode {
            SearchNoErrorMode::Signal => {
                let _ = buffers.goto_buffer_emacs_byte_pos(buffer_id, start_pt);
                Err(signal("search-failed", vec![pattern]))
            }
            SearchNoErrorMode::KeepPoint => {
                let _ = buffers.goto_buffer_emacs_byte_pos(buffer_id, start_pt);
                Ok(Value::NIL)
            }
            SearchNoErrorMode::MoveToBound => {
                let target = buffers
                    .get(buffer_id)
                    .map(|buf| search_failure_position(buf, opts))
                    .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
                let _ = buffers.goto_buffer_emacs_byte_pos(buffer_id, target);
                Ok(Value::NIL)
            }
        },
    }
}

pub(crate) fn builtin_search_backward(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let case_fold = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|v| !v.is_nil())
        .unwrap_or(true);
    let inhibit_changing = read_inhibit_changing_match_data(eval);
    let mut throwaway: Option<super::regex::MatchData> = None;
    let md_slot: &mut Option<super::regex::MatchData> = if inhibit_changing {
        &mut throwaway
    } else {
        &mut eval.match_data
    };
    builtin_search_backward_with_state(case_fold, &mut eval.buffers, md_slot, &args)
}

pub(crate) fn builtin_search_backward_with_state(
    case_fold: bool,
    buffers: &mut crate::buffer::BufferManager,
    match_data: &mut Option<super::regex::MatchData>,
    args: &[Value],
) -> EvalResult {
    expect_range_args("search-backward", args, 1, 4)?;
    let pattern = expect_string(&args[0])?;
    let (current_id, opts, start_pt, start_char) =
        current_search_context_in_manager(buffers, args, SearchKind::BackwardLiteral)?;
    if opts.steps == 0 {
        return Ok(Value::fixnum(start_char));
    }

    let mut last_pos = None;
    for _ in 0..opts.steps {
        let result = {
            let buf = buffers
                .get_mut(current_id)
                .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
            match opts.direction {
                SearchDirection::Forward => super::regex::search_forward(
                    buf,
                    &pattern,
                    opts.bound.map(|bound| bound.get()),
                    false,
                    case_fold,
                    match_data,
                ),
                SearchDirection::Backward => super::regex::search_backward(
                    buf,
                    &pattern,
                    opts.bound.map(|bound| bound.get()),
                    false,
                    case_fold,
                    match_data,
                ),
            }
        };
        match result {
            Ok(Some(pos)) => {
                last_pos = Some(record_buffer_search_success(buffers, current_id, pos)?)
            }
            Ok(None) => {
                return Err(signal("search-failed", vec![args[0]]));
            }
            Err(_) => {
                return handle_search_failure_in_manager(
                    buffers,
                    current_id,
                    Value::string(pattern),
                    opts,
                    start_pt,
                    SearchErrorKind::NotFound,
                );
            }
        }
    }

    let end = last_pos.expect("search loop should produce at least one match");
    buffer_byte_to_char_result_in_manager(buffers, current_id, EmacsBytePos::new(end))
}

pub(crate) fn builtin_re_search_forward(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let case_fold = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|v| !v.is_nil())
        .unwrap_or(true);
    let inhibit_changing = read_inhibit_changing_match_data(eval);
    let mut throwaway: Option<super::regex::MatchData> = None;
    let md_slot: &mut Option<super::regex::MatchData> = if inhibit_changing {
        &mut throwaway
    } else {
        &mut eval.match_data
    };
    let result = builtin_re_search_forward_with_state(case_fold, &mut eval.buffers, md_slot, &args);
    // Mirrors GNU `search.c:1247,1291`: poll quit after each search
    // call so a `C-g` that set `tls_quit_pending()` during the match
    // surfaces as a `quit` signal rather than being interpreted as
    // `search-failed`. The matcher itself returned None on the TLS
    // flag; here we promote it.
    eval.maybe_quit()?;
    result
}

pub(crate) fn builtin_re_search_forward_with_state(
    case_fold: bool,
    buffers: &mut crate::buffer::BufferManager,
    match_data: &mut Option<super::regex::MatchData>,
    args: &[Value],
) -> EvalResult {
    re_search_forward_with_state_posix(case_fold, false, buffers, match_data, args)
}

/// Shared body for `re-search-forward` and `posix-search-forward`.
/// When `posix` is true, the matcher runs the GNU POSIX longest-match
/// algorithm (regex-emacs.c:4143-4344). See audit #2 in
/// `drafts/regex-search-audit.md`; before this fix the posix builtins
/// were silent aliases.
pub(crate) fn re_search_forward_with_state_posix(
    case_fold: bool,
    posix: bool,
    buffers: &mut crate::buffer::BufferManager,
    match_data: &mut Option<super::regex::MatchData>,
    args: &[Value],
) -> EvalResult {
    let name = if posix {
        "posix-search-forward"
    } else {
        "re-search-forward"
    };
    expect_range_args(name, args, 1, 4)?;
    let pattern = expect_lisp_string(&args[0])?;
    let (current_id, opts, start_pt, start_char) =
        current_search_context_in_manager(buffers, args, SearchKind::ForwardRegexp)?;
    if opts.steps == 0 {
        return Ok(Value::fixnum(start_char));
    }

    let mut last_pos = None;
    for _ in 0..opts.steps {
        let result = {
            let buf = buffers
                .get_mut(current_id)
                .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
            match opts.direction {
                SearchDirection::Forward => super::regex::re_search_forward_lisp_with_posix(
                    buf,
                    pattern,
                    opts.bound.map(|bound| bound.get()),
                    false,
                    case_fold,
                    posix,
                    match_data,
                ),
                SearchDirection::Backward => super::regex::re_search_backward_lisp_with_posix(
                    buf,
                    pattern,
                    opts.bound.map(|bound| bound.get()),
                    false,
                    case_fold,
                    posix,
                    match_data,
                ),
            }
        };

        match result {
            Ok(Some(pos)) => {
                last_pos = Some(record_buffer_search_success(buffers, current_id, pos)?)
            }
            Ok(None) => {
                return Err(signal("search-failed", vec![args[0]]));
            }
            Err(msg) if msg != "Search failed" => {
                let _ = buffers.goto_buffer_emacs_byte_pos(current_id, start_pt);
                return Err(signal("invalid-regexp", vec![Value::string(msg)]));
            }
            Err(_) => {
                return handle_search_failure_in_manager(
                    buffers,
                    current_id,
                    args[0],
                    opts,
                    start_pt,
                    SearchErrorKind::NotFound,
                );
            }
        }
    }

    let end = last_pos.expect("search loop should produce at least one match");
    buffer_byte_to_char_result_in_manager(buffers, current_id, EmacsBytePos::new(end))
}

pub(crate) fn builtin_re_search_backward(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let case_fold = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|v| !v.is_nil())
        .unwrap_or(true);
    let inhibit_changing = read_inhibit_changing_match_data(eval);
    let mut throwaway: Option<super::regex::MatchData> = None;
    let md_slot: &mut Option<super::regex::MatchData> = if inhibit_changing {
        &mut throwaway
    } else {
        &mut eval.match_data
    };
    let result =
        builtin_re_search_backward_with_state(case_fold, &mut eval.buffers, md_slot, &args);
    // See `builtin_re_search_forward`: promote a TLS-detected quit.
    eval.maybe_quit()?;
    result
}

pub(crate) fn builtin_re_search_backward_with_state(
    case_fold: bool,
    buffers: &mut crate::buffer::BufferManager,
    match_data: &mut Option<super::regex::MatchData>,
    args: &[Value],
) -> EvalResult {
    re_search_backward_with_state_posix(case_fold, false, buffers, match_data, args)
}

/// Shared body for `re-search-backward` and `posix-search-backward`.
/// See [`re_search_forward_with_state_posix`] for the POSIX longest-
/// match rationale (audit #2).
pub(crate) fn re_search_backward_with_state_posix(
    case_fold: bool,
    posix: bool,
    buffers: &mut crate::buffer::BufferManager,
    match_data: &mut Option<super::regex::MatchData>,
    args: &[Value],
) -> EvalResult {
    let name = if posix {
        "posix-search-backward"
    } else {
        "re-search-backward"
    };
    expect_range_args(name, args, 1, 4)?;
    let pattern = expect_lisp_string(&args[0])?;
    let (current_id, opts, start_pt, start_char) =
        current_search_context_in_manager(buffers, args, SearchKind::BackwardRegexp)?;
    if opts.steps == 0 {
        return Ok(Value::fixnum(start_char));
    }

    let mut last_pos = None;
    for _ in 0..opts.steps {
        let result = {
            let buf = buffers
                .get_mut(current_id)
                .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
            match opts.direction {
                SearchDirection::Forward => super::regex::re_search_forward_lisp_with_posix(
                    buf,
                    pattern,
                    opts.bound.map(|bound| bound.get()),
                    false,
                    case_fold,
                    posix,
                    match_data,
                ),
                SearchDirection::Backward => super::regex::re_search_backward_lisp_with_posix(
                    buf,
                    pattern,
                    opts.bound.map(|bound| bound.get()),
                    false,
                    case_fold,
                    posix,
                    match_data,
                ),
            }
        };

        match result {
            Ok(Some(pos)) => {
                last_pos = Some(record_buffer_search_success(buffers, current_id, pos)?)
            }
            Ok(None) => {
                return Err(signal("search-failed", vec![args[0]]));
            }
            Err(msg) if msg != "Search failed" => {
                let _ = buffers.goto_buffer_emacs_byte_pos(current_id, start_pt);
                return Err(signal("invalid-regexp", vec![Value::string(msg)]));
            }
            Err(_) => {
                return handle_search_failure_in_manager(
                    buffers,
                    current_id,
                    args[0],
                    opts,
                    start_pt,
                    SearchErrorKind::NotFound,
                );
            }
        }
    }

    let end = last_pos.expect("search loop should produce at least one match");
    buffer_byte_to_char_result_in_manager(buffers, current_id, EmacsBytePos::new(end))
}

pub(crate) fn builtin_posix_search_forward(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let case_fold = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|v| !v.is_nil())
        .unwrap_or(true);
    let inhibit_changing = read_inhibit_changing_match_data(eval);
    let mut throwaway: Option<super::regex::MatchData> = None;
    let md_slot: &mut Option<super::regex::MatchData> = if inhibit_changing {
        &mut throwaway
    } else {
        &mut eval.match_data
    };
    re_search_forward_with_state_posix(case_fold, true, &mut eval.buffers, md_slot, &args)
}

pub(crate) fn builtin_posix_search_backward(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let case_fold = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|v| !v.is_nil())
        .unwrap_or(true);
    let inhibit_changing = read_inhibit_changing_match_data(eval);
    let mut throwaway: Option<super::regex::MatchData> = None;
    let md_slot: &mut Option<super::regex::MatchData> = if inhibit_changing {
        &mut throwaway
    } else {
        &mut eval.match_data
    };
    re_search_backward_with_state_posix(case_fold, true, &mut eval.buffers, md_slot, &args)
}

pub(crate) fn builtin_search_forward_regexp(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let case_fold = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|v| !v.is_nil())
        .unwrap_or(true);
    builtin_search_forward_regexp_with_state(
        case_fold,
        &mut eval.buffers,
        &mut eval.match_data,
        &args,
    )
}

pub(crate) fn builtin_search_forward_regexp_with_state(
    case_fold: bool,
    buffers: &mut crate::buffer::BufferManager,
    match_data: &mut Option<super::regex::MatchData>,
    args: &[Value],
) -> EvalResult {
    expect_range_args("search-forward-regexp", args, 1, 4)?;
    builtin_re_search_forward_with_state(case_fold, buffers, match_data, args)
}

pub(crate) fn builtin_search_backward_regexp(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let case_fold = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|v| !v.is_nil())
        .unwrap_or(true);
    builtin_search_backward_regexp_with_state(
        case_fold,
        &mut eval.buffers,
        &mut eval.match_data,
        &args,
    )
}

pub(crate) fn builtin_search_backward_regexp_with_state(
    case_fold: bool,
    buffers: &mut crate::buffer::BufferManager,
    match_data: &mut Option<super::regex::MatchData>,
    args: &[Value],
) -> EvalResult {
    expect_range_args("search-backward-regexp", args, 1, 4)?;
    builtin_re_search_backward_with_state(case_fold, buffers, match_data, args)
}

pub(crate) fn builtin_looking_at(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    let case_fold = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|v| !v.is_nil())
        .unwrap_or(true);
    let inhibit_changing = read_inhibit_changing_match_data(eval);
    // GNU search.c:282 `bool modify_match_data = NILP(Vinhibit_changing_match_data) && modify_data`
    // — when the global is set, route the search through a throwaway
    // match-data slot so neither the per-buffer match_data nor any
    // match data stored on the evaluator changes.
    let mut throwaway: Option<super::regex::MatchData> = None;
    let md_slot: &mut Option<super::regex::MatchData> = if inhibit_changing {
        &mut throwaway
    } else {
        &mut eval.match_data
    };
    let result = builtin_looking_at_with_state(case_fold, &eval.buffers, md_slot, &args);
    // Promote a TLS-detected quit to a `quit` signal (see
    // `builtin_re_search_forward`).
    eval.maybe_quit()?;
    result
}

pub(crate) fn builtin_looking_at_with_state(
    case_fold: bool,
    buffers: &crate::buffer::BufferManager,
    match_data: &mut Option<super::regex::MatchData>,
    args: &[Value],
) -> EvalResult {
    expect_range_args("looking-at", args, 1, 2)?;
    let pattern = expect_lisp_string(&args[0])?;
    let inhibit_modify = args.get(1).is_some_and(|arg| !arg.is_nil());

    let buf = buffers
        .current_buffer()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    let result = if inhibit_modify {
        let mut preserved_match_data = match_data.clone();
        super::regex::looking_at_lisp_with_posix(
            buf,
            pattern,
            case_fold,
            false,
            &mut preserved_match_data,
        )
    } else {
        super::regex::looking_at_lisp_with_posix(buf, pattern, case_fold, false, match_data)
    };

    match result {
        Ok(matched) => Ok(Value::bool_val(matched)),
        Err(msg) => Err(signal("invalid-regexp", vec![Value::string(msg)])),
    }
}

pub(crate) fn builtin_looking_at_p(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let case_fold = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|v| !v.is_nil())
        .unwrap_or(true);
    builtin_looking_at_p_with_state(case_fold, &eval.buffers, &args)
}

pub(crate) fn builtin_looking_at_p_with_state(
    case_fold: bool,
    buffers: &crate::buffer::BufferManager,
    args: &[Value],
) -> EvalResult {
    expect_args("looking-at-p", args, 1)?;
    let pattern = expect_lisp_string(&args[0])?;

    let buf = buffers
        .current_buffer()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;

    let mut throwaway_match_data = None;
    match super::regex::looking_at_lisp_with_posix(
        buf,
        pattern,
        case_fold,
        false,
        &mut throwaway_match_data,
    ) {
        Ok(matched) => Ok(Value::bool_val(matched)),
        Err(msg) => Err(signal("invalid-regexp", vec![Value::string(msg)])),
    }
}

pub(crate) fn builtin_posix_looking_at(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let case_fold = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|v| !v.is_nil())
        .unwrap_or(true);
    let inhibit_changing = read_inhibit_changing_match_data(eval);
    let mut throwaway: Option<super::regex::MatchData> = None;
    let md_slot: &mut Option<super::regex::MatchData> = if inhibit_changing {
        &mut throwaway
    } else {
        &mut eval.match_data
    };
    builtin_posix_looking_at_with_state(case_fold, &eval.buffers, md_slot, &args)
}

pub(crate) fn builtin_posix_looking_at_with_state(
    case_fold: bool,
    buffers: &crate::buffer::BufferManager,
    match_data: &mut Option<super::regex::MatchData>,
    args: &[Value],
) -> EvalResult {
    // GNU `src/search.c:Fposix_looking_at` calls `looking_at_1`
    // with `posix = 1`, which threads into `compile_pattern` and
    // ultimately into `re_match_2_internal` to enable POSIX
    // longest-match (regex-emacs.c:4143-4344). See audit #2 in
    // `drafts/regex-search-audit.md`; this wrapper used to be a
    // silent alias for `looking-at`.
    expect_range_args("posix-looking-at", args, 1, 2)?;
    let pattern = expect_lisp_string(&args[0])?;
    let inhibit_modify = args.get(1).is_some_and(|arg| !arg.is_nil());

    let buf = buffers
        .current_buffer()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    let result = if inhibit_modify {
        let mut preserved_match_data = match_data.clone();
        super::regex::looking_at_lisp_with_posix(
            buf,
            pattern,
            case_fold,
            true,
            &mut preserved_match_data,
        )
    } else {
        super::regex::looking_at_lisp_with_posix(buf, pattern, case_fold, true, match_data)
    };

    match result {
        Ok(matched) => Ok(Value::bool_val(matched)),
        Err(msg) => Err(signal("invalid-regexp", vec![Value::string(msg)])),
    }
}

pub(crate) fn builtin_string_match_with_state(
    case_fold: bool,
    case_translation: Option<crate::emacs_core::regex_emacs::CaseTranslation>,
    syntax_table: Option<&crate::emacs_core::syntax::SyntaxTable>,
    category_table: Option<Value>,
    match_data: &mut Option<super::regex::MatchData>,
    args: &[Value],
) -> EvalResult {
    crate::emacs_core::perf_trace::time_op(
        crate::emacs_core::perf_trace::HotpathOp::StringMatch,
        || {
            expect_range_args("string-match", args, 2, 4)?;
            let inhibit_modify = args.get(3).is_some_and(|v| v.is_truthy());

            match (args[0].kind(), args[1].kind()) {
                (ValueKind::String, ValueKind::String) => {
                    let pattern = expect_lisp_string(&args[0])?;
                    let string = args[1].as_lisp_string().unwrap();
                    let start = crate::emacs_core::search::normalize_lisp_string_start_arg(
                        string,
                        args.get(2),
                    )?;
                    let mut throwaway = None;
                    let target = if inhibit_modify {
                        &mut throwaway
                    } else {
                        match_data
                    };
                    let default_syntax = crate::emacs_core::regex_emacs::DefaultSyntaxLookup;
                    let buffer_syntax = syntax_table.map(|syntax_table| {
                        crate::emacs_core::regex_emacs::BufferSyntaxLookup {
                            syntax_table: syntax_table.clone(),
                            category_table,
                        }
                    });
                    let syntax: &dyn crate::emacs_core::regex_emacs::SyntaxLookup =
                        buffer_syntax.as_ref().map_or(&default_syntax, |s| s);
                    match super::regex::string_match_full_with_case_fold_source_lisp_pattern_posix_syntax(
                        pattern,
                        string,
                        super::regex::SearchedString::Heap(args[1]),
                        start,
                        case_fold,
                        false,
                        case_translation.clone(),
                        syntax,
                        target,
                    ) {
                        Ok(Some(char_pos)) => Ok(Value::fixnum(char_pos as i64)),
                        Ok(None) => Ok(Value::NIL),
                        Err(msg) => Err(signal("invalid-regexp", vec![Value::string(msg)])),
                    }
                }
                _ => {
                    let pattern = expect_string(&args[0])?;
                    let s = expect_string(&args[1])?;
                    let start = normalize_string_start_arg(&s, args.get(2))?;
                    let mut throwaway = None;
                    let target = if inhibit_modify {
                        &mut throwaway
                    } else {
                        match_data
                    };
                    match super::regex::string_match_full_with_case_fold(
                        &pattern, &s, start, case_fold, target,
                    ) {
                        Ok(Some(char_pos)) => Ok(Value::fixnum(char_pos as i64)),
                        Ok(None) => Ok(Value::NIL),
                        Err(msg) => Err(signal("invalid-regexp", vec![Value::string(msg)])),
                    }
                }
            }
        },
    )
}

/// Context-dependent `string-match`: updates match data on the evaluator.
pub(crate) fn builtin_string_match(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_string_match_slice(eval, &args)
}

pub(crate) fn builtin_string_match_slice(
    eval: &mut super::eval::Context,
    args: &[Value],
) -> EvalResult {
    let case_fold = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|v| !v.is_nil())
        .unwrap_or(true);
    let case_translation = if case_fold {
        let canon = crate::emacs_core::casetab::current_case_canon_table(eval)?;
        Some(crate::emacs_core::regex_emacs::CaseTranslation::from_char_table(canon))
    } else {
        None
    };
    let current_buffer = eval.buffers.current_buffer();
    let syntax_table = current_buffer.map(crate::emacs_core::syntax::SyntaxTable::for_buffer);
    let category_table =
        Some(crate::emacs_core::category::active_category_table_for_buffer(current_buffer)?);
    let inhibit_changing = read_inhibit_changing_match_data(eval);
    let mut throwaway: Option<super::regex::MatchData> = None;
    let md_slot: &mut Option<super::regex::MatchData> = if inhibit_changing {
        &mut throwaway
    } else {
        &mut eval.match_data
    };
    let result = builtin_string_match_with_state(
        case_fold,
        case_translation,
        syntax_table.as_ref(),
        category_table,
        md_slot,
        args,
    );
    // Promote a TLS-detected quit (see `builtin_re_search_forward`).
    eval.maybe_quit()?;
    result
}

pub(crate) fn builtin_posix_string_match_with_state(
    case_fold: bool,
    case_translation: Option<crate::emacs_core::regex_emacs::CaseTranslation>,
    syntax_table: Option<&crate::emacs_core::syntax::SyntaxTable>,
    category_table: Option<Value>,
    match_data: &mut Option<super::regex::MatchData>,
    args: &[Value],
) -> EvalResult {
    // GNU `src/search.c:Fposix_string_match` calls `string_match_1`
    // with `posix = 1`. Before this fix the neomacs builtin was a
    // silent alias for `string-match` (audit #2). We duplicate the
    // body of `builtin_string_match_with_state` and route through
    // the `*_posix` compile helpers so `CompiledPattern::posix` is
    // set for the matcher.
    crate::emacs_core::perf_trace::time_op(
        crate::emacs_core::perf_trace::HotpathOp::StringMatch,
        || {
            expect_range_args("posix-string-match", args, 2, 4)?;
            let inhibit_modify = args.get(3).is_some_and(|v| v.is_truthy());

            match (args[0].kind(), args[1].kind()) {
                (ValueKind::String, ValueKind::String) => {
                    let pattern = expect_lisp_string(&args[0])?;
                    let string = args[1].as_lisp_string().unwrap();
                    let start = crate::emacs_core::search::normalize_lisp_string_start_arg(
                        string,
                        args.get(2),
                    )?;
                    let mut throwaway = None;
                    let target = if inhibit_modify {
                        &mut throwaway
                    } else {
                        match_data
                    };
                    let default_syntax = crate::emacs_core::regex_emacs::DefaultSyntaxLookup;
                    let buffer_syntax = syntax_table.map(|syntax_table| {
                        crate::emacs_core::regex_emacs::BufferSyntaxLookup {
                            syntax_table: syntax_table.clone(),
                            category_table,
                        }
                    });
                    let syntax: &dyn crate::emacs_core::regex_emacs::SyntaxLookup =
                        buffer_syntax.as_ref().map_or(&default_syntax, |s| s);
                    match super::regex::string_match_full_with_case_fold_source_lisp_pattern_posix_syntax(
                        pattern,
                        string,
                        super::regex::SearchedString::Heap(args[1]),
                        start,
                        case_fold,
                        true,
                        case_translation.clone(),
                        syntax,
                        target,
                    ) {
                        Ok(Some(char_pos)) => Ok(Value::fixnum(char_pos as i64)),
                        Ok(None) => Ok(Value::NIL),
                        Err(msg) => Err(signal("invalid-regexp", vec![Value::string(msg)])),
                    }
                }
                _ => {
                    let pattern = expect_string(&args[0])?;
                    let s = expect_string(&args[1])?;
                    let start = normalize_string_start_arg(&s, args.get(2))?;
                    let mut throwaway = None;
                    let target = if inhibit_modify {
                        &mut throwaway
                    } else {
                        match_data
                    };
                    match super::regex::string_match_full_with_case_fold_and_posix(
                        &pattern, &s, start, case_fold, true, target,
                    ) {
                        Ok(Some(char_pos)) => Ok(Value::fixnum(char_pos as i64)),
                        Ok(None) => Ok(Value::NIL),
                        Err(msg) => Err(signal("invalid-regexp", vec![Value::string(msg)])),
                    }
                }
            }
        },
    )
}

pub(crate) fn builtin_string_match_p_with_case_fold(
    case_fold: bool,
    case_translation: Option<crate::emacs_core::regex_emacs::CaseTranslation>,
    syntax_table: Option<&crate::emacs_core::syntax::SyntaxTable>,
    category_table: Option<Value>,
    args: &[Value],
) -> EvalResult {
    expect_range_args("string-match-p", args, 2, 3)?;
    match (args[0].kind(), args[1].kind()) {
        (ValueKind::String, ValueKind::String) => {
            let pattern = expect_lisp_string(&args[0])?;
            let string = args[1].as_lisp_string().unwrap();
            let start =
                crate::emacs_core::search::normalize_lisp_string_start_arg(string, args.get(2))?;
            let mut throwaway = None;
            let default_syntax = crate::emacs_core::regex_emacs::DefaultSyntaxLookup;
            let buffer_syntax = syntax_table.map(|syntax_table| {
                crate::emacs_core::regex_emacs::BufferSyntaxLookup {
                    syntax_table: syntax_table.clone(),
                    category_table,
                }
            });
            let syntax: &dyn crate::emacs_core::regex_emacs::SyntaxLookup =
                buffer_syntax.as_ref().map_or(&default_syntax, |s| s);
            match super::regex::string_match_full_with_case_fold_source_lisp_pattern_posix_syntax(
                pattern,
                string,
                super::regex::SearchedString::Heap(args[1]),
                start,
                case_fold,
                false,
                case_translation,
                syntax,
                &mut throwaway,
            ) {
                Ok(Some(char_pos)) => Ok(Value::fixnum(char_pos as i64)),
                Ok(None) => Ok(Value::NIL),
                Err(msg) => Err(signal("invalid-regexp", vec![Value::string(msg)])),
            }
        }
        _ => {
            let pattern = expect_string(&args[0])?;
            let s = expect_string(&args[1])?;
            let start = normalize_string_start_arg(&s, args.get(2))?;
            let mut throwaway = None;

            match super::regex::string_match_full_with_case_fold(
                &pattern,
                &s,
                start,
                case_fold,
                &mut throwaway,
            ) {
                Ok(Some(char_pos)) => Ok(Value::fixnum(char_pos as i64)),
                Ok(None) => Ok(Value::NIL),
                Err(msg) => Err(signal("invalid-regexp", vec![Value::string(msg)])),
            }
        }
    }
}

pub(crate) fn builtin_string_match_p(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let case_fold = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|v| !v.is_nil())
        .unwrap_or(true);
    let case_translation = if case_fold {
        let canon = crate::emacs_core::casetab::current_case_canon_table(eval)?;
        Some(crate::emacs_core::regex_emacs::CaseTranslation::from_char_table(canon))
    } else {
        None
    };
    let current_buffer = eval.buffers.current_buffer();
    let syntax_table = current_buffer.map(crate::emacs_core::syntax::SyntaxTable::for_buffer);
    let category_table =
        Some(crate::emacs_core::category::active_category_table_for_buffer(current_buffer)?);
    builtin_string_match_p_with_case_fold(
        case_fold,
        case_translation,
        syntax_table.as_ref(),
        category_table,
        &args,
    )
}

pub(crate) fn builtin_posix_string_match(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let case_fold = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|v| !v.is_nil())
        .unwrap_or(true);
    let case_translation = if case_fold {
        let canon = crate::emacs_core::casetab::current_case_canon_table(eval)?;
        Some(crate::emacs_core::regex_emacs::CaseTranslation::from_char_table(canon))
    } else {
        None
    };
    let current_buffer = eval.buffers.current_buffer();
    let syntax_table = current_buffer.map(crate::emacs_core::syntax::SyntaxTable::for_buffer);
    let category_table =
        Some(crate::emacs_core::category::active_category_table_for_buffer(current_buffer)?);
    let inhibit_changing = read_inhibit_changing_match_data(eval);
    let mut throwaway: Option<super::regex::MatchData> = None;
    let md_slot: &mut Option<super::regex::MatchData> = if inhibit_changing {
        &mut throwaway
    } else {
        &mut eval.match_data
    };
    builtin_posix_string_match_with_state(
        case_fold,
        case_translation,
        syntax_table.as_ref(),
        category_table,
        md_slot,
        &args,
    )
}

pub(crate) fn builtin_match_string(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_range_args("match-string", &args, 1, 2)?;
    let group = expect_int(&args[0])?;
    if group < 0 {
        return Err(signal(
            "args-out-of-range",
            vec![Value::fixnum(group), Value::fixnum(0)],
        ));
    }
    let group = group as usize;

    let md = match &eval.match_data {
        Some(md) => md,
        None => return Ok(Value::NIL),
    };

    let group = match md.groups.get(group) {
        Some(Some(group)) => *group,
        _ => return Ok(Value::NIL),
    };
    let start = group.start();
    let end = group.end();

    let slice_lisp_string = |string: &crate::heap_types::LispString, use_char_positions: bool| {
        let (byte_start, byte_end) = if use_char_positions {
            (
                char_pos_to_byte_lisp_string(string, start),
                char_pos_to_byte_lisp_string(string, end),
            )
        } else {
            (start, end)
        };
        if byte_end <= string.byte_len() && byte_start <= byte_end {
            string.slice(byte_start, byte_end).map(Value::heap_string)
        } else {
            None
        }
    };

    // If an optional second arg is a string, use that first.
    if args.len() > 1 {
        if let Some(string) = args[1].as_lisp_string() {
            if let Some(slice) = slice_lisp_string(string, md.searched_string.is_some()) {
                return Ok(slice);
            }
            return Ok(Value::NIL);
        }

        if let Some(s) = args[1].as_utf8_str() {
            let (byte_start, byte_end) = if md.searched_string.is_some() {
                (char_pos_to_byte(s, start), char_pos_to_byte(s, end))
            } else {
                (
                    crate::emacs_core::string_escape::storage_logical_byte_to_storage_byte(
                        s, start,
                    ),
                    crate::emacs_core::string_escape::storage_logical_byte_to_storage_byte(s, end),
                )
            };
            if byte_end <= s.len() && byte_start <= byte_end {
                return Ok(Value::string(&s[byte_start..byte_end]));
            }
            return Ok(Value::NIL);
        }
    }

    // Otherwise, if the match was against a string, use that string.
    if let Some(ref searched) = md.searched_string {
        if let super::regex::SearchedString::Heap(val) = searched {
            if let Some(string) = val.as_lisp_string() {
                if let Some(slice) = slice_lisp_string(string, true) {
                    return Ok(slice);
                }
                return Ok(Value::NIL);
            }
        }

        if let Some(string) = searched.as_lisp_string() {
            if let Some(slice) = slice_lisp_string(string, true) {
                return Ok(slice);
            }
        }
        return Ok(Value::NIL);
    }

    let buf = match eval.buffers.current_buffer() {
        Some(b) => b,
        None => return Ok(Value::NIL),
    };
    if md.uses_buffer_byte_positions() {
        if end <= buf.total_emacs_byte_len().get() {
            return Ok(buf.buffer_substring_value_range(EmacsByteRange::new(
                EmacsBytePos::new(start),
                EmacsBytePos::new(end),
            )));
        }
        return Ok(Value::NIL);
    }

    let start_byte = buf.lisp_pos_to_emacs_byte_pos(LispCharPos1::from_one_based_usize(start));
    let end_byte = buf.lisp_pos_to_emacs_byte_pos(LispCharPos1::from_one_based_usize(end));
    if end_byte.get() <= buf.total_emacs_byte_len().get() && start_byte <= end_byte {
        Ok(buf.buffer_substring_value_range(EmacsByteRange::new(start_byte, end_byte)))
    } else {
        Ok(Value::NIL)
    }
}

pub(crate) fn builtin_match_beginning_with_state(
    buffers: Option<&crate::buffer::BufferManager>,
    match_data: &Option<super::regex::MatchData>,
    args: &[Value],
) -> EvalResult {
    crate::emacs_core::perf_trace::time_op(
        crate::emacs_core::perf_trace::HotpathOp::MatchBeginning,
        || {
            expect_args("match-beginning", args, 1)?;
            let group = expect_int(&args[0])?;
            if group < 0 {
                return Err(signal(
                    "args-out-of-range",
                    vec![Value::fixnum(group), Value::fixnum(0)],
                ));
            }
            let group = group as usize;

            let md = match match_data {
                Some(md) => md,
                None => return Ok(Value::NIL),
            };

            match md.groups.get(group) {
                Some(Some(group)) => {
                    let start = group.start();
                    if md.searched_string.is_some() {
                        Ok(Value::fixnum(start as i64))
                    } else if md.uses_buffer_byte_positions()
                        && let Some(buf) = md
                            .searched_buffer
                            .and_then(|buffer_id| buffers.and_then(|bufs| bufs.get(buffer_id)))
                    {
                        if start <= buf.total_emacs_byte_len().get() {
                            let pos = buffer_byte_to_lisp_char(buf, EmacsBytePos::new(start));
                            Ok(Value::fixnum(pos))
                        } else {
                            Ok(Value::fixnum(start as i64))
                        }
                    } else {
                        Ok(Value::fixnum(start as i64))
                    }
                }
                Some(None) => Ok(Value::NIL),
                None => Ok(Value::NIL),
            }
        },
    )
}

pub(crate) fn builtin_match_beginning(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_match_beginning_with_state(Some(&eval.buffers), &eval.match_data, &args)
}

pub(crate) fn builtin_match_end_with_state(
    buffers: Option<&crate::buffer::BufferManager>,
    match_data: &Option<super::regex::MatchData>,
    args: &[Value],
) -> EvalResult {
    crate::emacs_core::perf_trace::time_op(
        crate::emacs_core::perf_trace::HotpathOp::MatchEnd,
        || {
            expect_args("match-end", args, 1)?;
            let group = expect_int(&args[0])?;
            if group < 0 {
                return Err(signal(
                    "args-out-of-range",
                    vec![Value::fixnum(group), Value::fixnum(0)],
                ));
            }
            let group = group as usize;

            let md = match match_data {
                Some(md) => md,
                None => return Ok(Value::NIL),
            };

            match md.groups.get(group) {
                Some(Some(group)) => {
                    let end = group.end();
                    if md.searched_string.is_some() {
                        Ok(Value::fixnum(end as i64))
                    } else if md.uses_buffer_byte_positions()
                        && let Some(buf) = md
                            .searched_buffer
                            .and_then(|buffer_id| buffers.and_then(|bufs| bufs.get(buffer_id)))
                    {
                        if end <= buf.total_emacs_byte_len().get() {
                            let pos = buffer_byte_to_lisp_char(buf, EmacsBytePos::new(end));
                            Ok(Value::fixnum(pos))
                        } else {
                            Ok(Value::fixnum(end as i64))
                        }
                    } else {
                        Ok(Value::fixnum(end as i64))
                    }
                }
                Some(None) => Ok(Value::NIL),
                None => Ok(Value::NIL),
            }
        },
    )
}

pub(crate) fn builtin_match_end(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    builtin_match_end_with_state(Some(&eval.buffers), &eval.match_data, &args)
}

pub(crate) fn builtin_match_data_with_state(
    mut buffers: Option<&mut crate::buffer::BufferManager>,
    match_data: &Option<super::regex::MatchData>,
    args: &[Value],
) -> EvalResult {
    if args.len() > 3 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("match-data"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }

    let reuse = args.get(1).copied().unwrap_or(Value::NIL);
    if args.get(2).is_some_and(|arg| arg.is_truthy()) {
        if let Some(bufs) = buffers.as_deref_mut() {
            reseat_match_data_markers(bufs, reuse, None);
        }
    }

    let Some(md) = match_data else {
        return Ok(Value::NIL);
    };
    let integers = args.first().is_some_and(|arg| arg.is_truthy());
    let searched_buffer_id = if md.searched_string.is_none() {
        md.searched_buffer
    } else {
        None
    };

    // Emacs trims trailing unmatched groups from match-data output.
    let mut trailing = md.groups.len();
    while trailing > 0 && md.groups[trailing - 1].is_none() {
        trailing -= 1;
    }

    let mut flat: Vec<Value> = Vec::with_capacity(trailing * 2);
    for grp in md.groups.iter().take(trailing) {
        match grp {
            Some(group) => {
                let start = group.start();
                let end = group.end();
                if md.searched_string.is_some() {
                    flat.push(Value::fixnum(start as i64));
                    flat.push(Value::fixnum(end as i64));
                    continue;
                }

                let buffer_positions = if md.uses_buffer_byte_positions() {
                    searched_buffer_id.and_then(|buffer_id| {
                        buffers.as_deref().and_then(|bufs| {
                            bufs.get(buffer_id).and_then(|buffer| {
                                if start <= end && end <= buffer.total_emacs_byte_len().get() {
                                    Some((
                                        buffer_byte_to_lisp_char(buffer, EmacsBytePos::new(start)),
                                        buffer_byte_to_lisp_char(buffer, EmacsBytePos::new(end)),
                                    ))
                                } else {
                                    None
                                }
                            })
                        })
                    })
                } else if searched_buffer_id.is_some() {
                    Some((start as i64, end as i64))
                } else {
                    None
                };

                if integers {
                    if let Some((start_pos, end_pos)) = buffer_positions {
                        flat.push(Value::fixnum(start_pos));
                        flat.push(Value::fixnum(end_pos));
                    } else {
                        flat.push(Value::fixnum(start as i64));
                        flat.push(Value::fixnum(end as i64));
                    }
                    continue;
                }

                if let (Some((start_pos, end_pos)), Some(bufs), Some(buffer_id)) =
                    (buffer_positions, buffers.as_deref_mut(), searched_buffer_id)
                {
                    flat.push(super::marker::make_registered_buffer_marker(
                        bufs,
                        buffer_id,
                        LispCharPos1::new(start_pos),
                        false,
                    ));
                    flat.push(super::marker::make_registered_buffer_marker(
                        bufs,
                        buffer_id,
                        LispCharPos1::new(end_pos),
                        false,
                    ));
                    continue;
                }

                flat.push(Value::fixnum(start as i64));
                flat.push(Value::fixnum(end as i64));
            }
            None => {
                flat.push(Value::NIL);
                flat.push(Value::NIL);
            }
        }
    }

    if integers && md.searched_string.is_none() {
        if let Some(buffer_id) = searched_buffer_id {
            flat.push(Value::make_buffer(buffer_id));
        }
    }
    Ok(store_match_data_in_reuse(reuse, &flat))
}

fn store_match_data_in_reuse(reuse: Value, data: &[Value]) -> Value {
    if !reuse.is_cons() {
        return Value::list_from_slice(data);
    }

    let mut index = 0usize;
    let mut tail = reuse;
    let mut prev = Value::NIL;
    while tail.is_cons() {
        tail.set_car(data.get(index).copied().unwrap_or(Value::NIL));
        prev = tail;
        tail = tail.cons_cdr();
        index += 1;
    }

    if index < data.len() {
        prev.set_cdr(Value::list_from_slice(&data[index..]));
    }

    reuse
}

fn match_data_item_buffer_id_in_manager(
    buffers: &crate::buffer::BufferManager,
    value: &Value,
) -> Option<crate::buffer::BufferId> {
    if value.is_buffer() {
        return value.as_buffer_id();
    }
    if super::marker::is_marker(value) {
        return super::marker::marker_logical_fields(value)
            .and_then(|(buffer_id, _, _)| buffer_id)
            .filter(|buffer_id| buffers.get(*buffer_id).is_some());
    }
    None
}

fn expect_match_data_item_in_manager(
    buffers: &crate::buffer::BufferManager,
    value: &Value,
) -> Result<(i64, Option<crate::buffer::BufferId>), Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) => Ok((n, None)),
        _ if super::marker::is_marker(value) => Ok((
            super::marker::marker_position_as_int_with_buffers(buffers, value)?,
            match_data_item_buffer_id_in_manager(buffers, value),
        )),
        _ => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("integer-or-marker-p"), *value],
        )),
    }
}

pub(crate) fn builtin_match_data(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    builtin_match_data_with_state(Some(&mut eval.buffers), &eval.match_data, &args)
}

pub(crate) fn builtin_set_match_data_with_state(
    buffers: &mut crate::buffer::BufferManager,
    match_data: &mut Option<super::regex::MatchData>,
    args: &[Value],
) -> EvalResult {
    expect_min_args("set-match-data", args, 1)?;
    if args.len() > 2 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("set-match-data"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }

    if args[0].is_nil() {
        *match_data = None;
        return Ok(Value::NIL);
    }

    let items = list_to_vec(&args[0])
        .ok_or_else(|| signal("wrong-type-argument", vec![Value::symbol("listp"), args[0]]))?;

    let explicit_buffer_id = if items.len() % 2 == 1 {
        items.last().and_then(|value| value.as_buffer_id())
    } else {
        None
    };
    let pair_len = items.len() - usize::from(explicit_buffer_id.is_some());

    let mut groups: Vec<Option<MatchGroup>> = Vec::with_capacity(pair_len / 2);
    let mut searched_buffer = explicit_buffer_id;
    let mut i = 0usize;
    while i + 1 < pair_len {
        let start_v = &items[i];
        let end_v = &items[i + 1];

        if start_v.is_nil() && end_v.is_nil() {
            groups.push(None);
            i += 2;
            continue;
        }

        let (start, start_buffer) = expect_match_data_item_in_manager(buffers, start_v)?;
        let (end, end_buffer) = expect_match_data_item_in_manager(buffers, end_v)?;
        if searched_buffer.is_none() {
            searched_buffer = start_buffer.or(end_buffer);
        }

        // Emacs treats negative marker positions as an end sentinel and
        // truncates remaining groups.
        if start < 0 || end < 0 {
            break;
        }

        groups.push(Some(MatchGroup::new(start as usize, end as usize)));
        i += 2;
    }

    if groups.is_empty() {
        *match_data = None;
    } else {
        *match_data = Some(super::regex::MatchData {
            groups,
            searched_string: None,
            searched_buffer,
            buffer_positions_are_bytes: false,
        });
    }

    if args.get(1).is_some_and(|arg| arg.is_truthy()) {
        reseat_match_data_markers(buffers, args[0], Some(pair_len));
    }

    Ok(Value::NIL)
}

fn reseat_match_data_markers(
    buffers: &mut crate::buffer::BufferManager,
    list: Value,
    max_cells: Option<usize>,
) {
    let mut tail = list;
    let mut seen = 0usize;
    while tail.is_cons() && max_cells.is_none_or(|limit| seen < limit) {
        let item = tail.cons_car();
        if super::marker::is_marker(&item) {
            super::marker::detach_marker_in_buffers(buffers, &item);
            tail.set_car(Value::NIL);
        }
        tail = tail.cons_cdr();
        seen += 1;
    }
}

pub(crate) fn builtin_set_match_data(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_set_match_data_with_state(&mut eval.buffers, &mut eval.match_data, &args)
}

fn translate_match_data(match_data: &mut Option<super::regex::MatchData>, delta: i64) {
    if let Some(md) = match_data {
        for group in md.groups.iter_mut() {
            if let Some(group) = group {
                *group = group.translate_saturating(delta);
            }
        }
    }
}

pub(crate) fn builtin_match_data_translate_with_state(
    match_data: &mut Option<super::regex::MatchData>,
    args: &[Value],
) -> EvalResult {
    expect_args("match-data--translate", args, 1)?;
    let delta = expect_fixnum(&args[0])?;
    translate_match_data(match_data, delta);
    Ok(Value::NIL)
}

pub(crate) fn builtin_match_data_translate(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_match_data_translate_with_state(&mut eval.match_data, &args)
}

fn update_match_data_after_buffer_replace(
    match_data: &mut Option<super::regex::MatchData>,
    old_byte_range: EmacsByteRange,
    new_byte_range: EmacsByteRange,
) {
    let Some(md) = match_data else {
        return;
    };

    let oldstart = old_byte_range.start().get();
    let oldend = old_byte_range.end().get();
    let newend = new_byte_range.end().get();
    let change = newend as i64 - oldend as i64;
    for group in md.groups.iter_mut() {
        let Some(match_group) = group.as_mut() else {
            continue;
        };
        let mut start = match_group.start();
        let mut end = match_group.end();

        if start <= oldstart {
            // Keep starts for enclosing groups, matching GNU's optimistic
            // `update_search_regs` heuristic.
        } else if start >= oldend {
            start = (start as i64 + change) as usize;
        } else {
            start = oldstart;
        }

        if end >= oldend {
            end = (end as i64 + change) as usize;
        } else if end > oldstart {
            end = oldstart;
        }
        *match_group = MatchGroup::new(start, end);
    }
}

pub(crate) fn builtin_replace_match_with_state(
    obarray: &crate::emacs_core::symbol::Obarray,
    buffers: &mut crate::buffer::BufferManager,
    match_data: &mut Option<super::regex::MatchData>,
    args: &[Value],
) -> EvalResult {
    builtin_replace_match_with_state_and_flags(obarray, buffers, match_data, args, false)
}

/// Variant that also carries the current value of
/// `case-symbols-as-words` into the case-preservation decision for
/// `replace-match` with FIXEDCASE=nil. Audit findings #14/#20 in
/// `drafts/regex-search-audit.md`.
pub(crate) fn builtin_replace_match_with_state_and_flags(
    obarray: &crate::emacs_core::symbol::Obarray,
    buffers: &mut crate::buffer::BufferManager,
    match_data: &mut Option<super::regex::MatchData>,
    args: &[Value],
    case_symbols_as_words: bool,
) -> EvalResult {
    expect_min_args("replace-match", args, 1)?;
    if args.len() > 5 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("replace-match"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }

    let newtext_lisp = expect_lisp_string(&args[0])?;
    let newtext = expect_strict_string(&args[0])?;
    let fixedcase = args.get(1).is_some_and(|arg| arg.is_truthy());
    let literal = args.get(2).is_some_and(|arg| arg.is_truthy());
    let raw_subexp = args.get(4).copied().unwrap_or(Value::NIL);
    let string_arg = if args.get(3).is_some_and(|arg| !arg.is_nil()) {
        Some(expect_lisp_string(&args[3])?)
    } else {
        None
    };
    let subexp = if args.get(4).is_some_and(|arg| !arg.is_nil()) {
        let n = expect_int(&args[4])?;
        if n < 0 {
            return if let Some(source) = string_arg.as_ref() {
                Err(signal(
                    "args-out-of-range",
                    vec![
                        Value::fixnum(n),
                        Value::fixnum(0),
                        Value::fixnum(source.schars() as i64),
                    ],
                ))
            } else {
                Err(signal("args-out-of-range", vec![Value::fixnum(n)]))
            };
        }
        n as usize
    } else {
        0usize
    };

    let md_snapshot = match_data.clone();
    let missing_subexp_error = super::regex::REPLACE_MATCH_SUBEXP_MISSING;
    let missing_subexp_signal = |subexp_value: Value| {
        signal(
            "error",
            vec![Value::string(missing_subexp_error), subexp_value],
        )
    };

    if let Some(source) = string_arg {
        if md_snapshot.is_none() {
            return Err(missing_subexp_signal(raw_subexp));
        }
        if let Some(md) = md_snapshot.as_ref()
            && subexp >= md.groups.len()
        {
            return Err(signal(
                "args-out-of-range",
                vec![
                    Value::fixnum(subexp as i64),
                    Value::fixnum(0),
                    Value::fixnum(md.groups.len().saturating_sub(1) as i64),
                ],
            ));
        }
        return match crate::emacs_core::search::replace_match_lisp_string_with_syntax(
            source,
            newtext_lisp,
            fixedcase,
            literal,
            subexp,
            &md_snapshot,
        ) {
            Ok(result) => Ok(Value::heap_string(result)),
            Err(msg) if msg == missing_subexp_error => Err(missing_subexp_signal(raw_subexp)),
            Err(msg) => Err(signal("error", vec![Value::string(msg)])),
        };
    }

    if md_snapshot
        .as_ref()
        .is_some_and(|m| m.searched_string.is_some())
    {
        return Err(signal("args-out-of-range", vec![Value::fixnum(0)]));
    }
    if let Some(md) = md_snapshot.as_ref()
        && subexp >= md.groups.len()
    {
        return Err(signal(
            "args-out-of-range",
            vec![
                Value::fixnum(subexp as i64),
                Value::fixnum(0),
                Value::fixnum(md.groups.len().saturating_sub(1) as i64),
            ],
        ));
    }

    let current_id = buffers
        .current_buffer_id()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    let (old_byte_range, replacement, case_action) = {
        let buf = buffers
            .get(current_id)
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
        let replacement = crate::emacs_core::search::compute_buffer_replacement_lisp_string(
            buf,
            newtext_lisp,
            fixedcase,
            literal,
            subexp,
            &md_snapshot,
        )
        .map_err(|msg| {
            if msg == missing_subexp_error {
                missing_subexp_signal(raw_subexp)
            } else {
                signal("error", vec![Value::string(msg)])
            }
        })?;
        let case_action = if fixedcase {
            crate::emacs_core::casefiddle::ReplaceMatchCaseAction::NoChange
        } else {
            let matched_range = EmacsByteRange::new(
                EmacsBytePos::new(replacement.0),
                EmacsBytePos::new(replacement.1),
            );
            let matched = buf.buffer_substring_lisp_string_range(matched_range);
            crate::emacs_core::casefiddle::replace_match_case_action(
                &crate::emacs_core::search::storage_string_from_lisp_string(&matched),
            )
        };
        (
            EmacsByteRange::new(
                EmacsBytePos::new(replacement.0),
                EmacsBytePos::new(replacement.1),
            ),
            replacement.2,
            case_action,
        )
    };
    let replacement_len = replacement.sbytes();
    let old_range = buffers
        .edit_range_for_buffer_emacs_byte_range(current_id, old_byte_range)
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;

    super::super::fns::replace_buffer_region_lisp_string_in_manager(
        buffers,
        current_id,
        old_range,
        &replacement,
    )?;
    let replacement_byte_range = EmacsByteRange::new(
        old_byte_range.start(),
        old_byte_range
            .start()
            .add_len(crate::buffer::EmacsByteLen::new(replacement_len)),
    );
    if case_action != crate::emacs_core::casefiddle::ReplaceMatchCaseAction::NoChange
        && old_byte_range.start() < replacement_byte_range.end()
        && let Some(buf) = buffers.get_mut(current_id)
    {
        let start_char = buffer_byte_to_char_pos(buf, replacement_byte_range.start());
        let cased_text = buf.buffer_substring_lisp_string_range(replacement_byte_range);
        let mut undo_list = buf.get_undo_list();
        if !crate::buffer::undo::undo_list_is_disabled(&undo_list) {
            let end_char = start_char.add_len(CharLen::new(cased_text.schars()));
            crate::buffer::undo::undo_list_record_delete(
                &mut undo_list,
                start_char,
                cased_text.clone(),
                end_char,
                None,
            );
            crate::buffer::undo::undo_list_record_insert(
                &mut undo_list,
                start_char,
                CharLen::new(cased_text.schars()),
                None,
            );
            buf.set_undo_list(undo_list);
        }
    }
    // GNU `src/search.c:Freplace_match` records the caller's old point while
    // editing, then "officially" moves point to NEWPOINT, the end of the
    // replacement text.  Lisp parsers such as `xml-parse-string` depend on
    // this to continue after an expanded entity rather than re-reading the
    // replacement from its beginning.
    let _ = buffers.goto_buffer_emacs_byte_pos(current_id, replacement_byte_range.end());
    update_match_data_after_buffer_replace(match_data, old_byte_range, replacement_byte_range);
    Ok(Value::NIL)
}

pub(crate) fn builtin_replace_match(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    // GNU `src/search.c:2485-2505` consults `case-symbols-as-words`
    // when classifying the matched text for FIXEDCASE=nil. Read it
    // from the current dynamic environment once and thread it down.
    // See audit finding #20 in `drafts/regex-search-audit.md`.
    let case_symbols_as_words = dynamic_or_global_symbol_value(eval, "case-symbols-as-words")
        .map(|v| !v.is_nil())
        .unwrap_or(false);

    // Determine whether this is a buffer replacement (4th arg nil/absent) so we
    // can fire modification hooks.  String replacements don't touch the buffer.
    let is_buffer_replace = args.len() < 4 || args[3].is_nil();

    if is_buffer_replace {
        // Try to compute the match region for before-change signalling.
        let subexp = if args.len() >= 5 && !args[4].is_nil() {
            match expect_int(&args[4]) {
                Ok(n) if n >= 0 => n as usize,
                _ => {
                    return builtin_replace_match_with_state_and_flags(
                        &eval.obarray,
                        &mut eval.buffers,
                        &mut eval.match_data,
                        &args,
                        case_symbols_as_words,
                    );
                }
            }
        } else {
            0usize
        };
        if let Some(ref md) = eval.match_data {
            if md.searched_string.is_none() {
                if md.groups.get(subexp).is_some_and(|group| group.is_some()) {
                    let newtext_lisp = expect_lisp_string(&args[0])?;
                    let fixedcase = args.get(1).is_some_and(|arg| arg.is_truthy());
                    let literal = args.get(2).is_some_and(|arg| arg.is_truthy());
                    let raw_subexp = args.get(4).copied().unwrap_or(Value::NIL);
                    let missing_subexp_error = super::regex::REPLACE_MATCH_SUBEXP_MISSING;
                    let change = {
                        let buf = eval.buffers.current_buffer().ok_or_else(|| {
                            signal("error", vec![Value::string("No current buffer")])
                        })?;
                        let (oldstart, oldend, replacement) =
                            crate::emacs_core::search::compute_buffer_replacement_lisp_string(
                                buf,
                                newtext_lisp,
                                fixedcase,
                                literal,
                                subexp,
                                &eval.match_data,
                            )
                            .map_err(|msg| {
                                if msg == missing_subexp_error {
                                    signal(
                                        "error",
                                        vec![Value::string(missing_subexp_error), raw_subexp],
                                    )
                                } else {
                                    signal("error", vec![Value::string(msg)])
                                }
                            })?;
                        let current_id = eval.buffers.current_buffer_id().ok_or_else(|| {
                            signal("error", vec![Value::string("No current buffer")])
                        })?;
                        let change =
                            super::editfns::text_change_for_lisp_string_replacement_in_manager(
                                &eval.buffers,
                                current_id,
                                EmacsByteRange::new(
                                    EmacsBytePos::new(oldstart),
                                    EmacsBytePos::new(oldend),
                                ),
                                &replacement,
                            )?;
                        change
                    };
                    super::editfns::signal_before_text_change(eval, change)?;
                    let result = builtin_replace_match_with_state_and_flags(
                        &eval.obarray,
                        &mut eval.buffers,
                        &mut eval.match_data,
                        &args,
                        case_symbols_as_words,
                    )?;
                    super::editfns::signal_after_text_change(eval, change)?;
                    return Ok(result);
                }
            }
        }
    }

    // Fallback: string replacement or no match data — no buffer hooks needed.
    builtin_replace_match_with_state_and_flags(
        &eval.obarray,
        &mut eval.buffers,
        &mut eval.match_data,
        &args,
        case_symbols_as_words,
    )
}

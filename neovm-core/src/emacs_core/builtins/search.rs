use super::*;
use crate::buffer::{
    BufferId, CharLen, CharPos0, CharRange, EmacsBytePos, EmacsByteRange, LispCharPos1,
};
use crate::emacs_core::error::{expect_args, expect_args_range, expect_fixnum, expect_min_args};
use crate::emacs_core::regex::{
    BufferRegexpMatchContext, BufferRegexpSyntaxProperties, MatchDataSource, MatchGroup,
    char_pos_to_byte, char_pos_to_byte_lisp_string,
};
use crate::emacs_core::value::ValueKind;

/// Map a regex front-end error string to its Lisp signal.  Compile
/// errors are `invalid-regexp`; the matcher's fail-stack overflow is a
/// plain `error` in GNU (`search.c:matcher_overflow`: `error ("Stack
/// overflow in regexp matcher")`).
pub(crate) fn regex_error_signal(msg: String) -> crate::emacs_core::error::Flow {
    if msg == crate::emacs_core::regex_emacs::MATCHER_OVERFLOW_MESSAGE {
        signal("error", vec![Value::string(msg)])
    } else {
        signal(LispCondition::InvalidRegexp, vec![Value::string(msg)])
    }
}

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

pub(crate) fn current_word_boundary_lookup(
    eval: &super::eval::Context,
) -> crate::emacs_core::regex_emacs::WordBoundaryLookup {
    crate::emacs_core::regex_emacs::WordBoundaryLookup::new(
        dynamic_or_global_symbol_value(eval, "char-script-table").filter(|value| !value.is_nil()),
        dynamic_or_global_symbol_value(eval, "word-combining-categories").unwrap_or(Value::NIL),
        dynamic_or_global_symbol_value(eval, "word-separating-categories").unwrap_or(Value::NIL),
    )
}

/// Snapshot the match-time state, borrowing only the obarray so the caller can
/// still take `&mut eval.buffers` for the search itself.
fn current_buffer_regexp_match_context<'a>(
    obarray: &'a crate::emacs_core::symbol::Obarray,
    buffers: &crate::buffer::BufferManager,
    word_boundary: crate::emacs_core::regex_emacs::WordBoundaryLookup,
    syntax_properties: BufferRegexpSyntaxProperties,
) -> BufferRegexpMatchContext<'a> {
    BufferRegexpMatchContext::new(
        crate::emacs_core::syntax::SyntaxProperties::for_scan(
            syntax_properties.is_honor(),
            obarray,
            buffers,
        ),
        word_boundary,
    )
}

fn buffer_byte_to_lisp_char(buf: &crate::buffer::Buffer, byte_pos: EmacsBytePos) -> i64 {
    buf.emacs_byte_pos_to_lisp_char_pos(byte_pos).as_i64()
}

fn match_data_for_explicit_string_arg(md: &super::regex::MatchData) -> super::regex::MatchData {
    super::regex::MatchData::string(md.groups_snapshot(), None)
}

fn buffer_byte_to_char_pos(buf: &crate::buffer::Buffer, byte_pos: EmacsBytePos) -> CharPos0 {
    buf.emacs_byte_pos_to_char_pos_clamped(byte_pos)
}

fn commit_buffer_search_success(
    buffers: &mut crate::buffer::BufferManager,
    success: super::regex::BufferSearchSuccess,
    match_data: Option<&mut Option<super::regex::MatchData>>,
) -> Result<EmacsBytePos, Flow> {
    let (buffer_id, point, published_match_data) = success.into_parts();
    buffers
        .goto_buffer_emacs_byte_pos(buffer_id, point)
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    if let Some(match_data) = match_data {
        *match_data = Some(published_match_data);
    }
    Ok(point)
}

pub(crate) fn builtin_search_forward(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let case_fold = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|v| !v.is_nil())
        .unwrap_or(true);
    let inhibit_changing = read_inhibit_changing_match_data(eval);
    let match_data = (!inhibit_changing).then_some(&mut eval.match_data);
    builtin_search_forward_with_state(case_fold, &mut eval.buffers, match_data, &args)
}

pub(crate) fn builtin_search_forward_with_state(
    case_fold: bool,
    buffers: &mut crate::buffer::BufferManager,
    mut match_data: Option<&mut Option<super::regex::MatchData>>,
    args: &[Value],
) -> EvalResult {
    expect_args_range("search-forward", args, 1, 4)?;
    let pattern = expect_lisp_string(&args[0])?;
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
                    pattern,
                    opts.bound.map(|bound| bound.get()),
                    false,
                    case_fold,
                ),
                SearchDirection::Backward => super::regex::search_backward(
                    buf,
                    pattern,
                    opts.bound.map(|bound| bound.get()),
                    false,
                    case_fold,
                ),
            }
        };
        match result {
            Ok(Some(success)) => {
                last_pos = Some(commit_buffer_search_success(
                    buffers,
                    success,
                    match_data.as_deref_mut(),
                )?)
            }
            Ok(None) => {
                // regex::search_* with `noerror = false` never returns None.
                return Err(signal(LispCondition::SearchFailed, vec![args[0]]));
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
    buffer_byte_to_char_result_in_manager(buffers, current_id, end)
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
                LispCondition::WrongTypeArgument,
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
    let lisp_pos = LispCharPos1::new(super::super::buffer::expect_integer_or_marker_in_buffers(
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
                Err(signal(LispCondition::SearchFailed, vec![pattern]))
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

fn prepare_current_buffer_regexp_syntax(
    eval: &mut super::eval::Context,
    pattern: &crate::heap_types::LispString,
    case_fold: bool,
    posix: bool,
) -> Result<BufferRegexpSyntaxProperties, Flow> {
    prepare_current_buffer_regexp_syntax_to(eval, pattern, case_fold, posix, None)
}

/// Like [`prepare_current_buffer_regexp_syntax`], but propertizing only up to
/// `propertize_target_char` (exclusive-ish; the last position the matcher can
/// examine, plus one). GNU's matcher propertizes LAZILY as it scans
/// (parse_sexp_propertize stops at charpos + 1); neomacs pre-propertizes
/// because its Rust matcher cannot run re-entrant Lisp, so the target must be
/// the SEARCH RANGE end — pre-propertizing to point-max made every bounded
/// syntax-dependent search (looking-back, font-lock anchors) re-propertize
/// the whole buffer tail after each edit flushed syntax-propertize--done:
/// O(buffer) per keystroke. `None` keeps the conservative whole-accessible
/// target (patterns whose scan range is genuinely unbounded).
fn prepare_current_buffer_regexp_syntax_to(
    eval: &mut super::eval::Context,
    pattern: &crate::heap_types::LispString,
    case_fold: bool,
    posix: bool,
    propertize_target_char: Option<i64>,
) -> Result<BufferRegexpSyntaxProperties, Flow> {
    let dependency = {
        let buf = eval
            .buffers
            .current_buffer()
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
        super::regex::buffer_regexp_syntax_dependency(buf, pattern, case_fold, posix)
            .map_err(regex_error_signal)?
    };
    let syntax_properties = if crate::emacs_core::syntax::parse_sexp_lookup_properties_enabled(eval)
    {
        BufferRegexpSyntaxProperties::Honor
    } else {
        BufferRegexpSyntaxProperties::Ignore
    };

    if dependency.is_buffer_syntax_dependent() && syntax_properties.is_honor() {
        let accessible_target = eval
            .buffers
            .current_buffer()
            .map(|buf| buf.accessible_char_region().end().get().saturating_add(1))
            .unwrap_or(1);
        let target = match propertize_target_char {
            Some(explicit) => explicit.clamp(1, accessible_target as i64) as usize,
            None => accessible_target,
        };
        crate::emacs_core::syntax::maybe_syntax_propertize_for_scan(eval, target)?;
    }

    Ok(syntax_properties)
}

fn prepare_buffer_regexp_search(
    eval: &mut super::eval::Context,
    args: &[Value],
    kind: SearchKind,
    case_fold: bool,
    posix: bool,
) -> Result<BufferRegexpSyntaxProperties, Flow> {
    let pattern = expect_lisp_string(&args[0])?;
    let (_, opts, _, start_char) = current_search_context_in_manager(&eval.buffers, args, kind)?;
    if opts.steps == 0 {
        return Ok(
            if crate::emacs_core::syntax::parse_sexp_lookup_properties_enabled(eval) {
                BufferRegexpSyntaxProperties::Honor
            } else {
                BufferRegexpSyntaxProperties::Ignore
            },
        );
    }

    // The matcher's reachable range: a backward search only examines
    // positions at or before the starting point (matches end at or before
    // point), so propertizing through point suffices; a forward search is
    // capped by its BOUND argument when given.
    let target = match opts.direction {
        SearchDirection::Backward => Some(start_char.saturating_add(1)),
        SearchDirection::Forward => match args.get(1) {
            Some(v) if v.is_fixnum() => v.as_fixnum().map(|bound| bound.saturating_add(1)),
            _ => None,
        },
    };
    prepare_current_buffer_regexp_syntax_to(eval, pattern, case_fold, posix, target)
}

pub(crate) fn builtin_search_backward(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let case_fold = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|v| !v.is_nil())
        .unwrap_or(true);
    let inhibit_changing = read_inhibit_changing_match_data(eval);
    let match_data = (!inhibit_changing).then_some(&mut eval.match_data);
    builtin_search_backward_with_state(case_fold, &mut eval.buffers, match_data, &args)
}

pub(crate) fn builtin_search_backward_with_state(
    case_fold: bool,
    buffers: &mut crate::buffer::BufferManager,
    mut match_data: Option<&mut Option<super::regex::MatchData>>,
    args: &[Value],
) -> EvalResult {
    expect_args_range("search-backward", args, 1, 4)?;
    let pattern = expect_lisp_string(&args[0])?;
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
                    pattern,
                    opts.bound.map(|bound| bound.get()),
                    false,
                    case_fold,
                ),
                SearchDirection::Backward => super::regex::search_backward(
                    buf,
                    pattern,
                    opts.bound.map(|bound| bound.get()),
                    false,
                    case_fold,
                ),
            }
        };
        match result {
            Ok(Some(success)) => {
                last_pos = Some(commit_buffer_search_success(
                    buffers,
                    success,
                    match_data.as_deref_mut(),
                )?)
            }
            Ok(None) => {
                return Err(signal(LispCondition::SearchFailed, vec![args[0]]));
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
    buffer_byte_to_char_result_in_manager(buffers, current_id, end)
}

pub(crate) fn builtin_re_search_forward(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args_range("re-search-forward", &args, 1, 4)?;
    let case_fold = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|v| !v.is_nil())
        .unwrap_or(true);
    let syntax_properties =
        prepare_buffer_regexp_search(eval, &args, SearchKind::ForwardRegexp, case_fold, false)?;
    let match_context = current_buffer_regexp_match_context(
        &eval.obarray,
        &eval.buffers,
        current_word_boundary_lookup(eval),
        syntax_properties,
    );
    let inhibit_changing = read_inhibit_changing_match_data(eval);
    let match_data = (!inhibit_changing).then_some(&mut eval.match_data);
    let result = re_search_forward_with_state_posix_and_syntax_properties(
        case_fold,
        false,
        match_context,
        &mut eval.buffers,
        match_data,
        &args,
    );
    // Mirrors GNU `search.c:1247,1291`: poll quit after each search
    // call so a `C-g` that set `tls_quit_pending()` during the match
    // surfaces as a `quit` signal rather than being interpreted as
    // `search-failed`. The matcher itself returned None on the TLS
    // flag; here we promote it.
    eval.maybe_quit()?;
    result
}

/// Shared body for `re-search-forward` and `posix-search-forward`.
/// When `posix` is true, the matcher runs the GNU POSIX longest-match
/// algorithm (regex-emacs.c:4143-4344). See audit #2 in
/// `drafts/regex-search-audit.md`; before this fix the posix builtins
/// were silent aliases.
fn re_search_forward_with_state_posix_and_syntax_properties(
    case_fold: bool,
    posix: bool,
    match_context: BufferRegexpMatchContext<'_>,
    buffers: &mut crate::buffer::BufferManager,
    mut match_data: Option<&mut Option<super::regex::MatchData>>,
    args: &[Value],
) -> EvalResult {
    let name = if posix {
        "posix-search-forward"
    } else {
        "re-search-forward"
    };
    expect_args_range(name, args, 1, 4)?;
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
                    match_context,
                ),
                SearchDirection::Backward => super::regex::re_search_backward_lisp_with_posix(
                    buf,
                    pattern,
                    opts.bound.map(|bound| bound.get()),
                    false,
                    case_fold,
                    posix,
                    match_context,
                ),
            }
        };

        match result {
            Ok(Some(success)) => {
                last_pos = Some(commit_buffer_search_success(
                    buffers,
                    success,
                    match_data.as_deref_mut(),
                )?)
            }
            Ok(None) => {
                return Err(signal(LispCondition::SearchFailed, vec![args[0]]));
            }
            Err(msg) if msg != "Search failed" => {
                let _ = buffers.goto_buffer_emacs_byte_pos(current_id, start_pt);
                return Err(regex_error_signal(msg));
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
    buffer_byte_to_char_result_in_manager(buffers, current_id, end)
}

pub(crate) fn builtin_re_search_backward(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args_range("re-search-backward", &args, 1, 4)?;
    let case_fold = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|v| !v.is_nil())
        .unwrap_or(true);
    let syntax_properties =
        prepare_buffer_regexp_search(eval, &args, SearchKind::BackwardRegexp, case_fold, false)?;
    let match_context = current_buffer_regexp_match_context(
        &eval.obarray,
        &eval.buffers,
        current_word_boundary_lookup(eval),
        syntax_properties,
    );
    let inhibit_changing = read_inhibit_changing_match_data(eval);
    let match_data = (!inhibit_changing).then_some(&mut eval.match_data);
    let result = re_search_backward_with_state_posix_and_syntax_properties(
        case_fold,
        false,
        match_context,
        &mut eval.buffers,
        match_data,
        &args,
    );
    // See `builtin_re_search_forward`: promote a TLS-detected quit.
    eval.maybe_quit()?;
    result
}

/// Shared body for `re-search-backward` and `posix-search-backward`.
/// See [`re_search_forward_with_state_posix_and_syntax_properties`] for the POSIX longest-
/// match rationale (audit #2).
fn re_search_backward_with_state_posix_and_syntax_properties(
    case_fold: bool,
    posix: bool,
    match_context: BufferRegexpMatchContext<'_>,
    buffers: &mut crate::buffer::BufferManager,
    mut match_data: Option<&mut Option<super::regex::MatchData>>,
    args: &[Value],
) -> EvalResult {
    let name = if posix {
        "posix-search-backward"
    } else {
        "re-search-backward"
    };
    expect_args_range(name, args, 1, 4)?;
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
                    match_context,
                ),
                SearchDirection::Backward => super::regex::re_search_backward_lisp_with_posix(
                    buf,
                    pattern,
                    opts.bound.map(|bound| bound.get()),
                    false,
                    case_fold,
                    posix,
                    match_context,
                ),
            }
        };

        match result {
            Ok(Some(success)) => {
                last_pos = Some(commit_buffer_search_success(
                    buffers,
                    success,
                    match_data.as_deref_mut(),
                )?)
            }
            Ok(None) => {
                return Err(signal(LispCondition::SearchFailed, vec![args[0]]));
            }
            Err(msg) if msg != "Search failed" => {
                let _ = buffers.goto_buffer_emacs_byte_pos(current_id, start_pt);
                return Err(regex_error_signal(msg));
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
    buffer_byte_to_char_result_in_manager(buffers, current_id, end)
}

pub(crate) fn builtin_posix_search_forward(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args_range("posix-search-forward", &args, 1, 4)?;
    let case_fold = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|v| !v.is_nil())
        .unwrap_or(true);
    let syntax_properties =
        prepare_buffer_regexp_search(eval, &args, SearchKind::ForwardRegexp, case_fold, true)?;
    let match_context = current_buffer_regexp_match_context(
        &eval.obarray,
        &eval.buffers,
        current_word_boundary_lookup(eval),
        syntax_properties,
    );
    let inhibit_changing = read_inhibit_changing_match_data(eval);
    let match_data = (!inhibit_changing).then_some(&mut eval.match_data);
    re_search_forward_with_state_posix_and_syntax_properties(
        case_fold,
        true,
        match_context,
        &mut eval.buffers,
        match_data,
        &args,
    )
}

pub(crate) fn builtin_posix_search_backward(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args_range("posix-search-backward", &args, 1, 4)?;
    let case_fold = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|v| !v.is_nil())
        .unwrap_or(true);
    let syntax_properties =
        prepare_buffer_regexp_search(eval, &args, SearchKind::BackwardRegexp, case_fold, true)?;
    let match_context = current_buffer_regexp_match_context(
        &eval.obarray,
        &eval.buffers,
        current_word_boundary_lookup(eval),
        syntax_properties,
    );
    let inhibit_changing = read_inhibit_changing_match_data(eval);
    let match_data = (!inhibit_changing).then_some(&mut eval.match_data);
    re_search_backward_with_state_posix_and_syntax_properties(
        case_fold,
        true,
        match_context,
        &mut eval.buffers,
        match_data,
        &args,
    )
}

pub(crate) fn builtin_looking_at(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("looking-at", &args, 1, 2)?;
    let case_fold = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|v| !v.is_nil())
        .unwrap_or(true);
    let pattern = expect_lisp_string(&args[0])?;
    let syntax_properties = prepare_current_buffer_regexp_syntax(eval, pattern, case_fold, false)?;
    let match_context = current_buffer_regexp_match_context(
        &eval.obarray,
        &eval.buffers,
        current_word_boundary_lookup(eval),
        syntax_properties,
    );
    let inhibit_changing = read_inhibit_changing_match_data(eval);
    let match_data = (!inhibit_changing).then_some(&mut eval.match_data);
    let result = builtin_looking_at_with_state_and_syntax_properties(
        case_fold,
        match_context,
        &eval.buffers,
        match_data,
        &args,
    );
    // Promote a TLS-detected quit to a `quit` signal (see
    // `builtin_re_search_forward`).
    eval.maybe_quit()?;
    result
}

fn builtin_looking_at_with_state_and_syntax_properties(
    case_fold: bool,
    match_context: BufferRegexpMatchContext<'_>,
    buffers: &crate::buffer::BufferManager,
    match_data: Option<&mut Option<super::regex::MatchData>>,
    args: &[Value],
) -> EvalResult {
    expect_args_range("looking-at", args, 1, 2)?;
    let pattern = expect_lisp_string(&args[0])?;
    let inhibit_modify = args.get(1).is_some_and(|arg| !arg.is_nil());

    let buf = buffers
        .current_buffer()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    let result =
        super::regex::looking_at_lisp_with_posix(buf, pattern, case_fold, false, match_context);

    match result {
        Ok(published_match_data) => {
            let matched = published_match_data.is_some();
            if !inhibit_modify
                && let (Some(match_data), Some(published_match_data)) =
                    (match_data, published_match_data)
            {
                *match_data = Some(published_match_data);
            }
            Ok(Value::bool_val(matched))
        }
        Err(msg) => Err(regex_error_signal(msg)),
    }
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn builtin_looking_at_p(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("looking-at-p", &args, 1)?;
    let case_fold = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|v| !v.is_nil())
        .unwrap_or(true);
    let pattern = expect_lisp_string(&args[0])?;
    let syntax_properties = prepare_current_buffer_regexp_syntax(eval, pattern, case_fold, false)?;
    let match_context = current_buffer_regexp_match_context(
        &eval.obarray,
        &eval.buffers,
        current_word_boundary_lookup(eval),
        syntax_properties,
    );
    builtin_looking_at_p_with_state_and_syntax_properties(
        case_fold,
        match_context,
        &eval.buffers,
        &args,
    )
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn builtin_looking_at_p_with_state(
    case_fold: bool,
    buffers: &crate::buffer::BufferManager,
    args: &[Value],
) -> EvalResult {
    builtin_looking_at_p_with_state_and_syntax_properties(
        case_fold,
        BufferRegexpMatchContext::new(
            crate::emacs_core::syntax::SyntaxProperties::Ignore,
            crate::emacs_core::regex_emacs::WordBoundaryLookup::default(),
        ),
        buffers,
        args,
    )
}

fn builtin_looking_at_p_with_state_and_syntax_properties(
    case_fold: bool,
    match_context: BufferRegexpMatchContext<'_>,
    buffers: &crate::buffer::BufferManager,
    args: &[Value],
) -> EvalResult {
    expect_args("looking-at-p", args, 1)?;
    let pattern = expect_lisp_string(&args[0])?;

    let buf = buffers
        .current_buffer()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;

    match super::regex::looking_at_lisp_with_posix(buf, pattern, case_fold, false, match_context) {
        Ok(published_match_data) => Ok(Value::bool_val(published_match_data.is_some())),
        Err(msg) => Err(regex_error_signal(msg)),
    }
}

pub(crate) fn builtin_posix_looking_at(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args_range("posix-looking-at", &args, 1, 2)?;
    let case_fold = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|v| !v.is_nil())
        .unwrap_or(true);
    let pattern = expect_lisp_string(&args[0])?;
    let syntax_properties = prepare_current_buffer_regexp_syntax(eval, pattern, case_fold, true)?;
    let match_context = current_buffer_regexp_match_context(
        &eval.obarray,
        &eval.buffers,
        current_word_boundary_lookup(eval),
        syntax_properties,
    );
    let inhibit_changing = read_inhibit_changing_match_data(eval);
    let match_data = (!inhibit_changing).then_some(&mut eval.match_data);
    builtin_posix_looking_at_with_state_and_syntax_properties(
        case_fold,
        match_context,
        &eval.buffers,
        match_data,
        &args,
    )
}

fn builtin_posix_looking_at_with_state_and_syntax_properties(
    case_fold: bool,
    match_context: BufferRegexpMatchContext<'_>,
    buffers: &crate::buffer::BufferManager,
    match_data: Option<&mut Option<super::regex::MatchData>>,
    args: &[Value],
) -> EvalResult {
    // GNU `src/search.c:Fposix_looking_at` calls `looking_at_1`
    // with `posix = 1`, which threads into `compile_pattern` and
    // ultimately into `re_match_2_internal` to enable POSIX
    // longest-match (regex-emacs.c:4143-4344). See audit #2 in
    // `drafts/regex-search-audit.md`; this wrapper used to be a
    // silent alias for `looking-at`.
    expect_args_range("posix-looking-at", args, 1, 2)?;
    let pattern = expect_lisp_string(&args[0])?;
    let inhibit_modify = args.get(1).is_some_and(|arg| !arg.is_nil());

    let buf = buffers
        .current_buffer()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    let result =
        super::regex::looking_at_lisp_with_posix(buf, pattern, case_fold, true, match_context);

    match result {
        Ok(published_match_data) => {
            let matched = published_match_data.is_some();
            if !inhibit_modify
                && let (Some(match_data), Some(published_match_data)) =
                    (match_data, published_match_data)
            {
                *match_data = Some(published_match_data);
            }
            Ok(Value::bool_val(matched))
        }
        Err(msg) => Err(regex_error_signal(msg)),
    }
}

fn commit_string_search_success(
    result: Result<Option<super::regex::StringSearchSuccess>, String>,
    match_data: Option<&mut Option<super::regex::MatchData>>,
) -> EvalResult {
    match result {
        Ok(Some(success)) => {
            let (start, published_match_data) = success.into_parts();
            if let Some(match_data) = match_data {
                *match_data = Some(published_match_data);
            }
            Ok(Value::fixnum(start.get() as i64))
        }
        Ok(None) => Ok(Value::NIL),
        Err(msg) => Err(regex_error_signal(msg)),
    }
}

pub(crate) fn builtin_string_match_with_state(
    case_fold: bool,
    case_translation: Option<crate::emacs_core::regex_emacs::CaseTranslation>,
    syntax_table: Option<&crate::emacs_core::syntax::SyntaxTable>,
    category_table: Option<Value>,
    word_boundary: crate::emacs_core::regex_emacs::WordBoundaryLookup,
    match_data: Option<&mut Option<super::regex::MatchData>>,
    args: &[Value],
) -> EvalResult {
    crate::emacs_core::perf_trace::time_op(
        crate::emacs_core::perf_trace::HotpathOp::StringMatch,
        || {
            expect_args_range("string-match", args, 2, 4)?;
            let inhibit_modify = args.get(3).is_some_and(|v| v.is_truthy());

            match (args[0].kind(), args[1].kind()) {
                (ValueKind::String, ValueKind::String) => {
                    let pattern = expect_lisp_string(&args[0])?;
                    let string = args[1].as_lisp_string().unwrap();
                    let start = crate::emacs_core::search::normalize_lisp_string_start_arg(
                        string,
                        args.get(2),
                    )?;
                    let default_syntax = crate::emacs_core::regex_emacs::DefaultSyntaxLookup;
                    let buffer_syntax = syntax_table.map(|syntax_table| {
                        crate::emacs_core::regex_emacs::BufferSyntaxLookup {
                            syntax_table: *syntax_table,
                            category_table,
                            word_boundary,
                        }
                    });
                    let syntax: &dyn crate::emacs_core::regex_emacs::SyntaxLookup =
                        buffer_syntax.as_ref().map_or(&default_syntax, |s| s);
                    let result = super::regex::string_search_full_with_case_fold_source_lisp_pattern_posix_syntax(
                        pattern,
                        string,
                        super::regex::SearchedString::Heap(args[1]),
                        start,
                        case_fold,
                        false,
                        case_translation.clone(),
                        syntax,
                    );
                    let target = if inhibit_modify { None } else { match_data };
                    commit_string_search_success(result, target)
                }
                _ => {
                    let pattern = expect_string_lossy(&args[0])?;
                    let s = expect_string_lossy(&args[1])?;
                    let start = normalize_string_start_arg(&s, args.get(2))?;
                    let result = super::regex::string_search_full_with_case_fold_and_posix(
                        &pattern, &s, start, case_fold, false,
                    );
                    let target = if inhibit_modify { None } else { match_data };
                    commit_string_search_success(result, target)
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
    let word_boundary = current_word_boundary_lookup(eval);
    let inhibit_changing = read_inhibit_changing_match_data(eval);
    let match_data = (!inhibit_changing).then_some(&mut eval.match_data);
    let result = builtin_string_match_with_state(
        case_fold,
        case_translation,
        syntax_table.as_ref(),
        category_table,
        word_boundary,
        match_data,
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
    word_boundary: crate::emacs_core::regex_emacs::WordBoundaryLookup,
    match_data: Option<&mut Option<super::regex::MatchData>>,
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
            expect_args_range("posix-string-match", args, 2, 4)?;
            let inhibit_modify = args.get(3).is_some_and(|v| v.is_truthy());

            match (args[0].kind(), args[1].kind()) {
                (ValueKind::String, ValueKind::String) => {
                    let pattern = expect_lisp_string(&args[0])?;
                    let string = args[1].as_lisp_string().unwrap();
                    let start = crate::emacs_core::search::normalize_lisp_string_start_arg(
                        string,
                        args.get(2),
                    )?;
                    let default_syntax = crate::emacs_core::regex_emacs::DefaultSyntaxLookup;
                    let buffer_syntax = syntax_table.map(|syntax_table| {
                        crate::emacs_core::regex_emacs::BufferSyntaxLookup {
                            syntax_table: *syntax_table,
                            category_table,
                            word_boundary,
                        }
                    });
                    let syntax: &dyn crate::emacs_core::regex_emacs::SyntaxLookup =
                        buffer_syntax.as_ref().map_or(&default_syntax, |s| s);
                    let result = super::regex::string_search_full_with_case_fold_source_lisp_pattern_posix_syntax(
                        pattern,
                        string,
                        super::regex::SearchedString::Heap(args[1]),
                        start,
                        case_fold,
                        true,
                        case_translation.clone(),
                        syntax,
                    );
                    let target = if inhibit_modify { None } else { match_data };
                    commit_string_search_success(result, target)
                }
                _ => {
                    let pattern = expect_string_lossy(&args[0])?;
                    let s = expect_string_lossy(&args[1])?;
                    let start = normalize_string_start_arg(&s, args.get(2))?;
                    let result = super::regex::string_search_full_with_case_fold_and_posix(
                        &pattern, &s, start, case_fold, true,
                    );
                    let target = if inhibit_modify { None } else { match_data };
                    commit_string_search_success(result, target)
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
    word_boundary: crate::emacs_core::regex_emacs::WordBoundaryLookup,
    args: &[Value],
) -> EvalResult {
    expect_args_range("string-match-p", args, 2, 3)?;
    match (args[0].kind(), args[1].kind()) {
        (ValueKind::String, ValueKind::String) => {
            let pattern = expect_lisp_string(&args[0])?;
            let string = args[1].as_lisp_string().unwrap();
            let start =
                crate::emacs_core::search::normalize_lisp_string_start_arg(string, args.get(2))?;
            let default_syntax = crate::emacs_core::regex_emacs::DefaultSyntaxLookup;
            let buffer_syntax = syntax_table.map(|syntax_table| {
                crate::emacs_core::regex_emacs::BufferSyntaxLookup {
                    syntax_table: *syntax_table,
                    category_table,
                    word_boundary,
                }
            });
            let syntax: &dyn crate::emacs_core::regex_emacs::SyntaxLookup =
                buffer_syntax.as_ref().map_or(&default_syntax, |s| s);
            commit_string_search_success(
                super::regex::string_search_full_with_case_fold_source_lisp_pattern_posix_syntax(
                    pattern,
                    string,
                    super::regex::SearchedString::Heap(args[1]),
                    start,
                    case_fold,
                    false,
                    case_translation,
                    syntax,
                ),
                None,
            )
        }
        _ => {
            let pattern = expect_string_lossy(&args[0])?;
            let s = expect_string_lossy(&args[1])?;
            let start = normalize_string_start_arg(&s, args.get(2))?;
            commit_string_search_success(
                super::regex::string_search_full_with_case_fold_and_posix(
                    &pattern, &s, start, case_fold, false,
                ),
                None,
            )
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
    let word_boundary = current_word_boundary_lookup(eval);
    builtin_string_match_p_with_case_fold(
        case_fold,
        case_translation,
        syntax_table.as_ref(),
        category_table,
        word_boundary,
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
    let word_boundary = current_word_boundary_lookup(eval);
    let inhibit_changing = read_inhibit_changing_match_data(eval);
    let match_data = (!inhibit_changing).then_some(&mut eval.match_data);
    builtin_posix_string_match_with_state(
        case_fold,
        case_translation,
        syntax_table.as_ref(),
        category_table,
        word_boundary,
        match_data,
        &args,
    )
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn builtin_match_string(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args_range("match-string", &args, 1, 2)?;
    let group_index = expect_int(&args[0])?;
    if group_index < 0 {
        return Err(signal(
            LispCondition::ArgsOutOfRange,
            vec![Value::fixnum(group_index), Value::fixnum(0)],
        ));
    }
    let group_index = group_index as usize;

    let md = match &eval.match_data {
        Some(md) => md,
        None => return Ok(Value::NIL),
    };

    let Some(group) = md.group(group_index) else {
        return Ok(Value::NIL);
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
        let explicit_md = match_data_for_explicit_string_arg(md);
        let Some(group) = explicit_md.group(group_index) else {
            return Ok(Value::NIL);
        };
        let start = group.start();
        let end = group.end();
        if let Some(string) = args[1].as_lisp_string() {
            let (byte_start, byte_end) = (
                char_pos_to_byte_lisp_string(string, start),
                char_pos_to_byte_lisp_string(string, end),
            );
            if byte_end <= string.byte_len()
                && byte_start <= byte_end
                && let Some(slice) = string.slice(byte_start, byte_end).map(Value::heap_string)
            {
                return Ok(slice);
            }
            return Ok(Value::NIL);
        }

        if let Some(s) = args[1].as_utf8_str() {
            let (byte_start, byte_end) = (char_pos_to_byte(s, start), char_pos_to_byte(s, end));
            if byte_end <= s.len() && byte_start <= byte_end {
                return Ok(Value::string(&s[byte_start..byte_end]));
            }
            return Ok(Value::NIL);
        }
    }

    // Otherwise, if the match was against a string, use that string.
    if let Some(searched) = md.searched_string() {
        if let super::regex::SearchedString::Heap(val) = searched
            && let Some(string) = val.as_lisp_string()
        {
            if let Some(slice) = slice_lisp_string(string, true) {
                return Ok(slice);
            }
            return Ok(Value::NIL);
        }

        if let Some(string) = searched.as_lisp_string()
            && let Some(slice) = slice_lisp_string(string, true)
        {
            return Ok(slice);
        }
        return Ok(Value::NIL);
    }

    let buf = match eval.buffers.current_buffer() {
        Some(b) => b,
        None => return Ok(Value::NIL),
    };
    let start_byte = buf.lisp_pos_to_emacs_byte_pos(LispCharPos1::from_one_based_usize(start));
    let end_byte = buf.lisp_pos_to_emacs_byte_pos(LispCharPos1::from_one_based_usize(end));
    if end_byte.get() <= buf.total_emacs_byte_len().get() && start_byte <= end_byte {
        Ok(buf.buffer_substring_value_range(EmacsByteRange::new(start_byte, end_byte)))
    } else {
        Ok(Value::NIL)
    }
}

pub(crate) fn builtin_match_beginning_with_state(
    match_data: &Option<super::regex::MatchData>,
    args: &[Value],
) -> EvalResult {
    crate::emacs_core::perf_trace::time_op(
        crate::emacs_core::perf_trace::HotpathOp::MatchBeginning,
        || {
            expect_args("match-beginning", args, 1)?;
            // GNU `match_limit` (search.c) runs SUBEXP through `CHECK_FIXNUM`,
            // signalling `(wrong-type-argument fixnump …)` — not `integerp`.
            let group = expect_fixnum(&args[0])?;
            if group < 0 {
                return Err(signal(
                    LispCondition::ArgsOutOfRange,
                    vec![Value::fixnum(group), Value::fixnum(0)],
                ));
            }
            let group = group as usize;

            let md = match match_data {
                Some(md) => md,
                None => return Ok(Value::NIL),
            };

            match md.group(group) {
                Some(group) => Ok(Value::fixnum(group.start() as i64)),
                None => Ok(Value::NIL),
            }
        },
    )
}

pub(crate) fn builtin_match_beginning(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_match_beginning_with_state(&eval.match_data, &args)
}

pub(crate) fn builtin_match_end_with_state(
    match_data: &Option<super::regex::MatchData>,
    args: &[Value],
) -> EvalResult {
    crate::emacs_core::perf_trace::time_op(
        crate::emacs_core::perf_trace::HotpathOp::MatchEnd,
        || {
            expect_args("match-end", args, 1)?;
            // GNU `match_limit` (search.c) runs SUBEXP through `CHECK_FIXNUM`,
            // signalling `(wrong-type-argument fixnump …)` — not `integerp`.
            let group = expect_fixnum(&args[0])?;
            if group < 0 {
                return Err(signal(
                    LispCondition::ArgsOutOfRange,
                    vec![Value::fixnum(group), Value::fixnum(0)],
                ));
            }
            let group = group as usize;

            let md = match match_data {
                Some(md) => md,
                None => return Ok(Value::NIL),
            };

            match md.group(group) {
                Some(group) => Ok(Value::fixnum(group.end() as i64)),
                None => Ok(Value::NIL),
            }
        },
    )
}

pub(crate) fn builtin_match_end(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    builtin_match_end_with_state(&eval.match_data, &args)
}

/// How buffer provenance should materialize in a `(match-data)` result.
///
/// GNU retains the searched buffer object after it dies, but a marker cannot
/// attach to that dead buffer.  Keeping those states distinct prevents a
/// marker from carrying the contradictory combination of a dead buffer ID and
/// a live-looking position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatchDataMaterializationSource {
    String,
    LiveBuffer(BufferId),
    DeadBuffer(BufferId),
}

impl MatchDataMaterializationSource {
    fn classify(md: &super::regex::MatchData, buffers: &crate::buffer::BufferManager) -> Self {
        match md.source() {
            MatchDataSource::String => Self::String,
            MatchDataSource::Buffer(buffer_id) if buffers.get(buffer_id).is_some() => {
                Self::LiveBuffer(buffer_id)
            }
            MatchDataSource::Buffer(buffer_id) => Self::DeadBuffer(buffer_id),
        }
    }

    fn buffer_id(self) -> Option<BufferId> {
        match self {
            Self::String => None,
            Self::LiveBuffer(buffer_id) | Self::DeadBuffer(buffer_id) => Some(buffer_id),
        }
    }
}

pub(crate) fn builtin_match_data_with_state(
    buffers: &mut crate::buffer::BufferManager,
    match_data: &Option<super::regex::MatchData>,
    args: &[Value],
) -> EvalResult {
    if args.len() > 3 {
        return Err(signal(
            LispCondition::WrongNumberOfArguments,
            vec![
                Value::symbol("match-data"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }

    let reuse = args.get(1).copied().unwrap_or(Value::NIL);
    if args.get(2).is_some_and(|arg| arg.is_truthy()) {
        reseat_match_data_markers(buffers, reuse, None);
    }

    let Some(md) = match_data else {
        return Ok(Value::NIL);
    };
    let integers = args.first().is_some_and(|arg| arg.is_truthy());
    let source = MatchDataMaterializationSource::classify(md, buffers);

    // Emacs trims trailing unmatched groups from match-data output.
    let mut trailing = md.group_count();
    while trailing > 0 && md.group(trailing - 1).is_none() {
        trailing -= 1;
    }

    let mut flat: Vec<Value> = Vec::with_capacity(trailing * 2);
    for group_index in 0..trailing {
        let grp = md.group(group_index);
        match grp {
            Some(group) => {
                let start = group.start() as i64;
                let end = group.end() as i64;
                match source {
                    MatchDataMaterializationSource::String
                    | MatchDataMaterializationSource::LiveBuffer(_)
                    | MatchDataMaterializationSource::DeadBuffer(_)
                        if integers =>
                    {
                        flat.push(Value::fixnum(start));
                        flat.push(Value::fixnum(end));
                    }
                    MatchDataMaterializationSource::String => {
                        flat.push(Value::fixnum(start));
                        flat.push(Value::fixnum(end));
                    }
                    MatchDataMaterializationSource::LiveBuffer(buffer_id) => {
                        flat.push(super::marker::make_registered_buffer_marker(
                            buffers,
                            buffer_id,
                            LispCharPos1::new(start),
                            false,
                        ));
                        flat.push(super::marker::make_registered_buffer_marker(
                            buffers,
                            buffer_id,
                            LispCharPos1::new(end),
                            false,
                        ));
                    }
                    MatchDataMaterializationSource::DeadBuffer(_) => {
                        // GNU Fmatch_data creates fresh markers and asks
                        // Fset_marker to attach them to last_thing_searched.
                        // A dead buffer makes Fset_marker leave them fully
                        // detached, with no saved last position.
                        flat.push(super::marker::make_marker_value(None, None, false));
                        flat.push(super::marker::make_marker_value(None, None, false));
                    }
                }
            }
            None => {
                flat.push(Value::NIL);
                flat.push(Value::NIL);
            }
        }
    }

    if integers && let Some(buffer_id) = source.buffer_id() {
        flat.push(Value::make_buffer(buffer_id));
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

/// A position accepted by GNU `set-match-data`.
///
/// Detached markers are not errors here: search.c explicitly coerces them to
/// integer zero.  Model that case instead of losing it in an
/// `(i64, Option<BufferId>)` pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatchDataInputPosition {
    Integer(i64),
    LiveMarker { position: i64, buffer_id: BufferId },
    DetachedMarker,
}

impl MatchDataInputPosition {
    fn position(self) -> i64 {
        match self {
            Self::Integer(position) | Self::LiveMarker { position, .. } => position,
            Self::DetachedMarker => 0,
        }
    }

    fn buffer_id(self) -> Option<BufferId> {
        match self {
            Self::LiveMarker { buffer_id, .. } => Some(buffer_id),
            Self::Integer(_) | Self::DetachedMarker => None,
        }
    }
}

fn expect_match_data_position_in_manager(
    buffers: &crate::buffer::BufferManager,
    value: &Value,
) -> Result<MatchDataInputPosition, Flow> {
    match value.kind() {
        ValueKind::Fixnum(position) => Ok(MatchDataInputPosition::Integer(position)),
        _ if super::marker::is_marker(value) => {
            let fields = super::marker::marker_logical_fields(value)
                .expect("marker predicate guarantees marker fields");
            match fields {
                (Some(buffer_id), Some(position), _) if buffers.get(buffer_id).is_some() => {
                    Ok(MatchDataInputPosition::LiveMarker {
                        position: position.as_i64(),
                        buffer_id,
                    })
                }
                _ => Ok(MatchDataInputPosition::DetachedMarker),
            }
        }
        _ => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("integer-or-marker-p"), *value],
        )),
    }
}

pub(crate) fn builtin_match_data(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    builtin_match_data_with_state(&mut eval.buffers, &eval.match_data, &args)
}

/// A marker-backed snapshot produced by GNU-compatible `(match-data)`.
///
/// Keep this private newtype instead of passing a bare `Value` through native
/// unwind code: only a snapshot captured here may be restored with reseating.
/// Buffer positions are markers, so the saved ranges continue to track edits
/// made while the protected operation runs.
#[derive(Clone, Copy)]
struct SavedMatchData(Value);

impl SavedMatchData {
    fn capture(eval: &mut super::eval::Context) -> Result<Self, Flow> {
        builtin_match_data(eval, Vec::new()).map(Self)
    }

    fn root(self, eval: &mut super::eval::Context) {
        eval.push_specpdl_root(self.0);
    }

    fn restore(self, eval: &mut super::eval::Context) -> EvalResult {
        builtin_set_match_data(eval, vec![self.0, Value::T])
    }
}

/// Run native evaluator work with GNU `record_unwind_save_match_data`
/// semantics.
///
/// This is the Rust-side equivalent of GNU `search.c` saving `(match-data)` on
/// the unwind stack and restoring it with `(set-match-data SAVED t)`.  The
/// closure boundary makes restoration unavoidable on both `Ok` and `Flow`
/// exits, while the rooted [`SavedMatchData`] keeps its marker list alive
/// across arbitrary Lisp execution and GC.
pub(crate) fn with_preserved_match_data<T>(
    eval: &mut super::eval::Context,
    operation: impl FnOnce(&mut super::eval::Context) -> Result<T, Flow>,
) -> Result<T, Flow> {
    let roots = eval.save_specpdl_roots();
    let saved = match SavedMatchData::capture(eval) {
        Ok(saved) => saved,
        Err(flow) => {
            eval.restore_specpdl_roots(roots);
            return Err(flow);
        }
    };
    saved.root(eval);

    let operation_result = operation(eval);
    let restore_result = saved.restore(eval);
    eval.restore_specpdl_roots(roots);

    match operation_result {
        Err(flow) => Err(flow),
        Ok(value) => restore_result.map(|_| value),
    }
}

pub(crate) fn builtin_set_match_data_with_state(
    buffers: &mut crate::buffer::BufferManager,
    match_data: &mut Option<super::regex::MatchData>,
    args: &[Value],
) -> EvalResult {
    expect_min_args("set-match-data", args, 1)?;
    if args.len() > 2 {
        return Err(signal(
            LispCondition::WrongNumberOfArguments,
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

    let items = list_to_vec(&args[0]).ok_or_else(|| {
        signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("listp"), args[0]],
        )
    })?;

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

        let start = expect_match_data_position_in_manager(buffers, start_v)?;
        let end = expect_match_data_position_in_manager(buffers, end_v)?;
        if searched_buffer.is_none() {
            searched_buffer = start.buffer_id().or(end.buffer_id());
        }
        let start = start.position();
        let end = end.position();

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
    } else if let Some(searched_buffer) = searched_buffer {
        *match_data = Some(super::regex::MatchData::buffer_lisp_chars(
            groups,
            searched_buffer,
        ));
    } else {
        *match_data = Some(super::regex::MatchData::string(groups, None));
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
        md.translate_positions(delta);
    }
}

pub(crate) fn builtin_match_data_translate_with_state(
    _buffers: &crate::buffer::BufferManager,
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
    builtin_match_data_translate_with_state(&eval.buffers, &mut eval.match_data, &args)
}

#[derive(Clone, Copy)]
struct BufferReplacementCoordinates {
    old_char_range: CharRange,
    replacement_char_len: CharLen,
}

impl BufferReplacementCoordinates {
    fn published_match_data_bounds(self) -> (usize, usize, usize) {
        let oldstart = self.old_char_range.start_lisp().to_one_based_usize();
        let oldend = self.old_char_range.end_lisp().to_one_based_usize();
        (
            oldstart,
            oldend,
            oldstart.saturating_add(self.replacement_char_len.get()),
        )
    }
}

fn update_match_data_after_buffer_replace(
    match_data: &mut Option<super::regex::MatchData>,
    replacement: BufferReplacementCoordinates,
) {
    let Some(md) = match_data else {
        return;
    };

    let (oldstart, oldend, newend) = replacement.published_match_data_bounds();
    let change = newend as i64 - oldend as i64;
    md.map_lisp_positions(|match_group| {
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
        MatchGroup::new(start, end)
    });
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
    _case_symbols_as_words: bool,
) -> EvalResult {
    expect_min_args("replace-match", args, 1)?;
    if args.len() > 5 {
        return Err(signal(
            LispCondition::WrongNumberOfArguments,
            vec![
                Value::symbol("replace-match"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }

    let newtext_lisp = expect_lisp_string(&args[0])?;
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
                    LispCondition::ArgsOutOfRange,
                    vec![
                        Value::fixnum(n),
                        Value::fixnum(0),
                        Value::fixnum(source.schars() as i64),
                    ],
                ))
            } else {
                Err(signal(
                    LispCondition::ArgsOutOfRange,
                    vec![Value::fixnum(n)],
                ))
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
    // C-level `error()' messages are requoted via `text-quoting-style' by
    // GNU's doprnt (e.g. "Invalid use of `\\' ..." -> curly quotes).
    let quoting_style = crate::emacs_core::coding::effective_text_quoting_style(obarray);
    let c_error = |msg: String| {
        signal(
            "error",
            vec![Value::string(
                crate::emacs_core::coding::requote_c_error_message(&msg, quoting_style),
            )],
        )
    };

    if let Some(source) = string_arg {
        if md_snapshot.is_none() {
            return Err(missing_subexp_signal(raw_subexp));
        }
        if let Some(md) = md_snapshot.as_ref()
            && subexp >= md.group_count()
        {
            return Err(signal(
                LispCondition::ArgsOutOfRange,
                vec![
                    Value::fixnum(subexp as i64),
                    Value::fixnum(0),
                    Value::fixnum(md.group_count().saturating_sub(1) as i64),
                ],
            ));
        }
        let string_md_snapshot = md_snapshot.as_ref().map(match_data_for_explicit_string_arg);
        return match crate::emacs_core::search::replace_match_lisp_string_with_syntax(
            source,
            newtext_lisp,
            fixedcase,
            literal,
            subexp,
            &string_md_snapshot,
        ) {
            Ok(result) => Ok(Value::heap_string(result)),
            Err(msg) if msg == missing_subexp_error => Err(missing_subexp_signal(raw_subexp)),
            Err(msg) => Err(c_error(msg)),
        };
    }

    if let Some(md) = md_snapshot.as_ref()
        && subexp >= md.group_count()
    {
        return Err(signal(
            LispCondition::ArgsOutOfRange,
            vec![
                Value::fixnum(subexp as i64),
                Value::fixnum(0),
                Value::fixnum(md.group_count().saturating_sub(1) as i64),
            ],
        ));
    }

    let current_id = buffers
        .current_buffer_id()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    let (old_byte_range, old_char_range, replacement, case_action) = {
        let buf = buffers
            .get(current_id)
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
        // GNU checks the subexpression against the *accessible* portion of the
        // current buffer and reports both endpoints (search.c:2418-2427).
        if let Some(group) = md_snapshot.as_ref().and_then(|md| md.group(subexp)) {
            let begv = buf.point_min_lisp_char_pos().to_one_based_usize();
            let zv = buf.point_max_lisp_char_pos().to_one_based_usize();
            if group.start() < begv || group.end() > zv {
                return Err(signal(
                    LispCondition::ArgsOutOfRange,
                    vec![
                        Value::fixnum(group.start() as i64),
                        Value::fixnum(group.end() as i64),
                    ],
                ));
            }
        }
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
                c_error(msg)
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
            crate::emacs_core::casefiddle::replace_match_case_action_lisp_default(&matched)
        };
        (
            EmacsByteRange::new(
                EmacsBytePos::new(replacement.0),
                EmacsBytePos::new(replacement.1),
            ),
            CharRange::new(
                buf.emacs_byte_pos_to_char_pos_clamped(EmacsBytePos::new(replacement.0)),
                buf.emacs_byte_pos_to_char_pos_clamped(EmacsBytePos::new(replacement.1)),
            ),
            replacement.2,
            case_action,
        )
    };
    let replacement_len = replacement.sbytes();
    let replacement_char_len = CharLen::new(replacement.schars());
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
    update_match_data_after_buffer_replace(
        match_data,
        BufferReplacementCoordinates {
            old_char_range,
            replacement_char_len,
        },
    );
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
        if let Some(ref md) = eval.match_data
            && !md.source().is_string()
            && md.group(subexp).is_some()
        {
            let newtext_lisp = expect_lisp_string(&args[0])?;
            let fixedcase = args.get(1).is_some_and(|arg| arg.is_truthy());
            let literal = args.get(2).is_some_and(|arg| arg.is_truthy());
            let raw_subexp = args.get(4).copied().unwrap_or(Value::NIL);
            let missing_subexp_error = super::regex::REPLACE_MATCH_SUBEXP_MISSING;
            // C-level `error()' messages are requoted via
            // `text-quoting-style' by GNU's doprnt.
            let quoting_style =
                crate::emacs_core::coding::effective_text_quoting_style(&eval.obarray);
            let change = {
                let buf = eval
                    .buffers
                    .current_buffer()
                    .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
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
                            signal(
                                "error",
                                vec![Value::string(
                                    crate::emacs_core::coding::requote_c_error_message(
                                        &msg,
                                        quoting_style,
                                    ),
                                )],
                            )
                        }
                    })?;
                let current_id = eval
                    .buffers
                    .current_buffer_id()
                    .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;

                super::editfns::text_change_for_lisp_string_replacement_in_manager(
                    &eval.buffers,
                    current_id,
                    EmacsByteRange::new(EmacsBytePos::new(oldstart), EmacsBytePos::new(oldend)),
                    &replacement,
                )?
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

    // Fallback: string replacement or no match data — no buffer hooks needed.
    builtin_replace_match_with_state_and_flags(
        &eval.obarray,
        &mut eval.buffers,
        &mut eval.match_data,
        &args,
        case_symbols_as_words,
    )
}

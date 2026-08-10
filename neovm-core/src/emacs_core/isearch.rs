//! Interactive search and query-replace.
//!
//! Implements:
//! - Incremental search (isearch) state machine
//! - Search history
//! - Search highlighting (overlay tracking)
//! - Query-replace with interactive responses
//! - Regular expression search variants
//! - Lazy highlight (deferred search matches)

use crate::emacs_core::error::LispCondition;
use std::collections::VecDeque;

use super::error::{EvalResult, Flow, signal};
use super::regex::MatchGroup;
use super::value::{Value, ValueKind};
use crate::buffer::{Buffer, EmacsByteLen, EmacsBytePos, EmacsByteRange, LispCharPos1};
use crate::heap_types::LispString;

// ---------------------------------------------------------------------------
// Argument helpers (local copies, matching builtins.rs convention)
// ---------------------------------------------------------------------------

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn expect_min_max_args(name: &str, args: &[Value], min: usize, max: usize) -> Result<(), Flow> {
    if args.len() < min || args.len() > max {
        Err(signal(
            LispCondition::WrongNumberOfArguments,
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

/// Render a search/replace string for a human-readable echo-area prompt only.
///
/// Issue #131: this is lossy by design (eight-bit / undecodable Emacs bytes
/// become U+FFFD), so it MUST NOT be used for any matched/inserted content —
/// only for echo-area display, where GNU likewise shows a best-effort glyph.
fn lisp_string_for_display(value: &LispString) -> String {
    crate::emacs_core::emacs_char::to_utf8_lossy(value.as_bytes())
}

fn empty_lisp_string(multibyte: bool) -> LispString {
    if multibyte {
        LispString::from_utf8("")
    } else {
        LispString::from_unibyte(Vec::new())
    }
}

fn promote_unibyte_lisp_string(value: &LispString) -> LispString {
    // Promote a unibyte string to multibyte Emacs bytes directly (raw bytes
    // become eight-bit chars), instead of round-tripping through a storage String.
    LispString::from_emacs_bytes(crate::emacs_core::emacs_char::str_to_multibyte(
        value.as_bytes(),
    ))
}

fn append_char_to_lisp_string(value: &mut LispString, ch: char) {
    if !value.is_multibyte() && (ch as u32) <= 0xFF {
        value.mutate_bytes(|bytes| bytes.push(ch as u8));
        return;
    }

    if !value.is_multibyte() {
        *value = promote_unibyte_lisp_string(value);
    }

    let mut buf = [0u8; crate::emacs_core::emacs_char::MAX_MULTIBYTE_LENGTH];
    let len = crate::emacs_core::emacs_char::char_string(ch as u32, &mut buf);
    value.mutate_bytes(|bytes| bytes.extend_from_slice(&buf[..len]));
}

fn pop_char_from_lisp_string(value: &mut LispString) {
    if value.is_empty() {
        return;
    }

    if value.is_multibyte() {
        let new_len = crate::emacs_core::emacs_char::char_to_byte_pos(
            value.as_bytes(),
            value.schars().saturating_sub(1),
        );
        value.mutate_bytes(|bytes| bytes.truncate(new_len));
    } else {
        value.mutate_bytes(|bytes| {
            bytes.pop();
        });
    }
}

fn append_runtime_fragment_to_lisp_string(value: &mut LispString, fragment: &str) {
    if fragment.is_empty() {
        return;
    }
    let fragment = storage_string_to_lisp_string(fragment, value.is_multibyte());
    *value = value.concat(&fragment);
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn expect_string(val: &Value) -> Result<&'static LispString, Flow> {
    // Issue #131: keep the search/replace argument byte-faithful — hand the
    // caller the real LispString (Emacs bytes), not a PUA-sentinel storage form.
    val.as_lisp_string().ok_or_else(|| {
        signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("stringp"), *val],
        )
    })
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn expect_integer_or_marker(
    buffers: &crate::buffer::BufferManager,
    val: &Value,
) -> Result<i64, Flow> {
    match val.kind() {
        ValueKind::Fixnum(n) => Ok(n),
        _ if super::marker::is_marker(val) => {
            super::marker::marker_position_as_int_with_buffers(buffers, val)
        }
        _ => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("integer-or-marker-p"), *val],
        )),
    }
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn expect_sequence_string(val: &Value) -> Result<&'static LispString, Flow> {
    // Issue #131: byte-faithful — see `expect_string`.
    val.as_lisp_string().ok_or_else(|| {
        signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("sequencep"), *val],
        )
    })
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn lisp_pos_to_byte(buf: &crate::buffer::Buffer, pos: LispCharPos1) -> EmacsBytePos {
    buf.lisp_pos_to_accessible_emacs_byte_pos(pos)
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn lisp_pos_value_to_byte(
    buffers: &crate::buffer::BufferManager,
    buf: &crate::buffer::Buffer,
    value: &Value,
) -> Result<EmacsBytePos, Flow> {
    Ok(lisp_pos_to_byte(
        buf,
        LispCharPos1::new(expect_integer_or_marker(buffers, value)?),
    ))
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn replacement_region_bounds(
    buffers: &crate::buffer::BufferManager,
    buf: &crate::buffer::Buffer,
    start_arg: Option<&Value>,
    end_arg: Option<&Value>,
    backward: bool,
    region_noncontiguous: bool,
) -> Result<EmacsByteRange, Flow> {
    let accessible = buf.accessible_emacs_byte_region();
    if region_noncontiguous {
        let mark = buf.mark_emacs_byte_pos().ok_or_else(|| {
            signal(
                "error",
                vec![Value::string(
                    "The mark is not set now, so there is no region",
                )],
            )
        })?;
        let pt = buf.point_emacs_byte_pos();
        return Ok(EmacsByteRange::ordered(pt, mark));
    }

    let start = match start_arg {
        Some(v) if !v.is_nil() => lisp_pos_value_to_byte(buffers, buf, v)?,
        _ if backward => accessible.start(),
        _ => buf.point_emacs_byte_pos(),
    };
    let end = match end_arg {
        Some(v) if !v.is_nil() => lisp_pos_value_to_byte(buffers, buf, v)?,
        _ if backward => buf.point_emacs_byte_pos(),
        _ => accessible.end(),
    };
    Ok(EmacsByteRange::ordered(start, end))
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn line_operation_region_bounds(
    buffers: &crate::buffer::BufferManager,
    buf: &crate::buffer::Buffer,
    start_arg: Option<&Value>,
    end_arg: Option<&Value>,
) -> Result<EmacsByteRange, Flow> {
    let accessible = buf.accessible_emacs_byte_region();
    let start = match start_arg {
        Some(v) if !v.is_nil() => lisp_pos_value_to_byte(buffers, buf, v)?,
        _ => buf.point_emacs_byte_pos(),
    };
    let end = match end_arg {
        Some(v) if !v.is_nil() => lisp_pos_value_to_byte(buffers, buf, v)?,
        _ => accessible.end(),
    };
    Ok(EmacsByteRange::ordered(start, end))
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn line_start_at_or_before(source: &[u8], at: usize) -> usize {
    let pos = at.min(source.len());
    match source[..pos].iter().rposition(|&b| b == b'\n') {
        Some(idx) => idx + 1,
        None => 0,
    }
}

fn lisp_string_from_buffer_bytes(bytes: Vec<u8>, multibyte: bool) -> crate::heap_types::LispString {
    if multibyte {
        crate::heap_types::LispString::from_emacs_bytes(bytes)
    } else {
        crate::heap_types::LispString::from_unibyte(bytes)
    }
}

fn storage_string_to_lisp_string(source: &str, multibyte: bool) -> crate::heap_types::LispString {
    let bytes = crate::emacs_core::string_escape::storage_string_to_buffer_bytes(source, multibyte);
    lisp_string_from_buffer_bytes(bytes, multibyte)
}

/// Whether PATTERN contains an uppercase letter, scanning real Emacs char codes
/// (issue #131). Mirrors GNU `isearch-no-upper-case-p`: eight-bit raw bytes and
/// PUA glyphs are caseless and never count as uppercase.
fn lisp_pattern_has_uppercase(pattern: &crate::heap_types::LispString) -> bool {
    let bytes = pattern.as_bytes();
    let mut pos = 0;
    while pos < bytes.len() {
        let (code, len) = crate::emacs_core::emacs_char::string_char(&bytes[pos..]);
        pos += len.max(1);
        if char::from_u32(code).is_some_and(char::is_uppercase) {
            return true;
        }
    }
    false
}

/// Route symbol-value reads through the full GNU lookup path so
/// LOCALIZED BLV / FORWARDED slot / specpdl let-binding state is
/// observed. See the extended comment on the identical helper in
/// `builtins/misc_eval.rs` (audit finding #3 in
/// `drafts/regex-search-audit.md`).
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn dynamic_or_global_symbol_value(eval: &super::eval::Context, name: &str) -> Option<Value> {
    let id = crate::emacs_core::intern::intern(name);
    eval.eval_symbol_by_id(id).ok()
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn buffer_read_only_active(eval: &super::eval::Context, buf: &Buffer) -> bool {
    if buf.get_read_only() {
        return true;
    }

    if let Some(value) = buf.get_buffer_local("buffer-read-only") {
        return value.is_truthy();
    }

    eval.obarray
        .symbol_value("buffer-read-only")
        .is_some_and(|value| value.is_truthy())
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn case_fold_for_pattern(
    eval: &super::eval::Context,
    pattern: &crate::heap_types::LispString,
) -> bool {
    let case_fold_search_enabled = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|value| !value.is_nil())
        .unwrap_or(true);
    if !case_fold_search_enabled {
        return false;
    }
    // Emacs honors `search-upper-case`: nil disables smart-case and keeps
    // folding even when PATTERN contains uppercase characters.
    let smart_case_enabled = dynamic_or_global_symbol_value(eval, "search-upper-case")
        .map(|value| !value.is_nil())
        .unwrap_or(true);
    if !smart_case_enabled {
        return true;
    }
    // GNU `isearch-no-upper-case-p`: fold unless PATTERN has an uppercase letter.
    !lisp_pattern_has_uppercase(pattern)
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn case_replace_enabled(eval: &super::eval::Context) -> bool {
    dynamic_or_global_symbol_value(eval, "case-replace")
        .map(|value| !value.is_nil())
        .unwrap_or(true)
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn replace_lax_whitespace_enabled(eval: &super::eval::Context) -> bool {
    dynamic_or_global_symbol_value(eval, "replace-lax-whitespace")
        .map(|value| !value.is_nil())
        .unwrap_or(false)
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn resolve_search_whitespace_regexp(eval: &super::eval::Context) -> Option<LispString> {
    // Issue #131: return the whitespace regexp as a byte-faithful LispString. The
    // default `[ \t\n\r]+` is ASCII, but a user-set `search-whitespace-regexp`
    // may carry eight-bit/PUA bytes, so keep the real bytes (no storage form).
    let raw = match dynamic_or_global_symbol_value(eval, "search-whitespace-regexp") {
        Some(v) => match v.as_lisp_string() {
            Some(ls) => ls.clone(),
            None if v.is_nil() => LispString::from_utf8("[ \t\n\r]+"),
            None => return None,
        },
        None => LispString::from_utf8("[ \t\n\r]+"),
    };
    Some(raw)
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn quote_emacs_regexp_literal_bytes(literal: &[u8], result: &mut Vec<u8>) {
    // Issue #131: quote Emacs regexp metacharacters byte-by-byte. All quoted
    // metacharacters are ASCII; non-ASCII Emacs bytes pass through untouched.
    for &byte in literal {
        match byte {
            b'.' | b'*' | b'+' | b'?' | b'[' | b'^' | b'$' | b'\\' => {
                result.push(b'\\');
                result.push(byte);
            }
            _ => result.push(byte),
        }
    }
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn build_lax_whitespace_pattern(pattern: &[u8], whitespace_regex: &[u8]) -> Vec<u8> {
    // Issue #131: operate on Emacs bytes — a literal run is byte-faithful, and the
    // ASCII space (0x20) that separates runs cannot appear inside a multibyte
    // Emacs char, so byte iteration is exact.
    let mut raw: Vec<u8> = Vec::new();
    let mut literal: Vec<u8> = Vec::new();
    let mut in_space_run = false;

    for &byte in pattern {
        if byte == b' ' {
            if !literal.is_empty() {
                quote_emacs_regexp_literal_bytes(&literal, &mut raw);
                literal.clear();
            }
            if !in_space_run {
                raw.extend_from_slice(b"\\(");
                raw.extend_from_slice(whitespace_regex);
                raw.extend_from_slice(b"\\)");
                in_space_run = true;
            }
        } else {
            in_space_run = false;
            literal.push(byte);
        }
    }

    if !literal.is_empty() {
        quote_emacs_regexp_literal_bytes(&literal, &mut raw);
    }

    raw
}

/// Match one buffer line, lifted out as a string, against PATTERN.
///
/// GNU `keep-lines`/`flush-lines` (lisp/replace.el) run `re-search-forward`
/// over the buffer, so their matches see the current buffer's syntax state;
/// this string-shaped analogue carries the same state via `syntax`.
fn string_matches_regexp(
    syntax: super::builtins::search::FastStringMatchSyntax,
    obarray: &super::symbol::Obarray,
    buffers: &crate::buffer::BufferManager,
    line: &[u8],
    multibyte: bool,
    pattern: &crate::heap_types::LispString,
    case_fold: bool,
) -> Result<bool, Flow> {
    // Issue #131: match the line's Emacs bytes against the faithful LispString
    // pattern, no storage round-trip.
    let text = lisp_string_from_buffer_bytes(line.to_vec(), multibyte);
    syntax
        .search(
            obarray,
            buffers,
            pattern,
            &text,
            super::regex::SearchedString::Owned(text.clone()),
            0,
            case_fold,
        )
        .map(|matched| matched.is_some())
        .map_err(|e| signal(LispCondition::InvalidRegexp, vec![Value::string(e)]))
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn delete_line_operation_byte_range(
    eval: &mut super::eval::Context,
    current_id: crate::buffer::BufferId,
    range: EmacsByteRange,
) -> Result<(), Flow> {
    if range.is_empty() {
        return Ok(());
    }

    {
        let buf = eval
            .buffers
            .get(current_id)
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
        if buffer_read_only_active(eval, buf) {
            return Err(signal(
                LispCondition::BufferReadOnly,
                vec![Value::make_buffer(current_id)],
            ));
        }
    }

    crate::emacs_core::textprop::verify_text_read_only_in_state(
        &eval.obarray,
        &eval.buffers,
        current_id,
        range.start().get(),
        range.end().get(),
    )?;

    let change =
        super::editfns::text_change_for_deletion_in_manager(&eval.buffers, current_id, range)?;
    super::editfns::signal_before_text_change(eval, change)?;
    let _ = eval
        .buffers
        .delete_buffer_measured_region(current_id, change.old_range());
    super::editfns::signal_after_text_change(eval, change)
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn delete_line_operation_ranges(
    eval: &mut super::eval::Context,
    current_id: crate::buffer::BufferId,
    delete_ranges: Vec<EmacsByteRange>,
) -> Result<usize, Flow> {
    let mut deleted_so_far = 0usize;
    let mut deleted_ranges = 0usize;
    for original in delete_ranges {
        if original.is_empty() {
            continue;
        }
        let deleted_len = EmacsByteLen::new(deleted_so_far);
        let shifted = EmacsByteRange::new(
            original.start().saturating_sub_len(deleted_len),
            original.end().saturating_sub_len(deleted_len),
        );
        delete_line_operation_byte_range(eval, current_id, shifted)?;
        deleted_so_far = deleted_so_far.saturating_add(original.len().get());
        deleted_ranges += 1;
    }
    Ok(deleted_ranges)
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn count_string_regexp_matches(
    text: &[u8],
    pattern: &LispString,
    case_fold: bool,
) -> Result<i64, Flow> {
    let iterated = super::regex::iterate_string_matches_with_case_fold(pattern, text, 0, case_fold)
        .map_err(|e| signal(LispCondition::InvalidRegexp, vec![Value::string(e)]))?;
    Ok(iterated
        .matches
        .into_iter()
        .filter_map(|groups| groups.first().and_then(|group| *group))
        .filter(|group| !(group.start() == group.end() && group.start() >= text.len()))
        .count() as i64)
}

// ---------------------------------------------------------------------------
// Search direction
// ---------------------------------------------------------------------------

/// Direction of an incremental search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchDirection {
    Forward,
    Backward,
}

// ---------------------------------------------------------------------------
// IsearchState — tracks incremental search session
// ---------------------------------------------------------------------------

/// Full state for one active incremental search session.
pub struct IsearchState {
    /// Whether the search session is still running.
    pub active: bool,
    /// Current search direction.
    pub direction: SearchDirection,
    /// The string being searched for (built up character by character).
    pub search_string: LispString,
    /// Whether this is a regexp search.
    pub regexp: bool,
    /// Case folding: `None` = auto-detect, `Some(true)` = fold, `Some(false)` = exact.
    pub case_fold: Option<bool>,
    /// Whether the search has wrapped around the buffer.
    pub wrapped: bool,
    /// Whether the last incremental search step succeeded.
    pub success: bool,
    /// Byte position of the start of the current match (if any).
    pub match_start: Option<usize>,
    /// Byte position of the end of the current match (if any).
    pub match_end: Option<usize>,
    /// Point position when the search was started (for abort restoration).
    pub origin: usize,
    /// Position where wrapping resets to.
    pub barrier: usize,
    /// Index into the history ring, if navigating history.
    pub history_index: Option<usize>,
    /// All visible matches for lazy-highlight overlays.
    pub lazy_matches: Vec<MatchGroup>,
}

// ---------------------------------------------------------------------------
// SearchHistory
// ---------------------------------------------------------------------------

/// Ring of previous search strings, kept separately for literal and regexp.
pub struct SearchHistory {
    strings: VecDeque<LispString>,
    regexp_strings: VecDeque<LispString>,
    max_length: usize,
}

impl SearchHistory {
    /// Create an empty history with default capacity of 100 entries per ring.
    pub fn new() -> Self {
        Self {
            strings: VecDeque::new(),
            regexp_strings: VecDeque::new(),
            max_length: 100,
        }
    }

    /// Push a search string onto the appropriate ring.
    /// Duplicates are moved to the front rather than stored twice.
    pub fn push(&mut self, string: LispString, regexp: bool) {
        let ring = if regexp {
            &mut self.regexp_strings
        } else {
            &mut self.strings
        };
        // Remove duplicate if present
        if let Some(pos) = ring.iter().position(|s| *s == string) {
            ring.remove(pos);
        }
        ring.push_front(string);
        if ring.len() > self.max_length {
            ring.pop_back();
        }
    }

    /// Get the search string at `index` (0 = most recent).
    pub fn get(&self, index: usize, regexp: bool) -> Option<&LispString> {
        let ring = if regexp {
            &self.regexp_strings
        } else {
            &self.strings
        };
        ring.get(index)
    }

    /// Number of entries in the chosen ring.
    pub fn len(&self, regexp: bool) -> usize {
        if regexp {
            self.regexp_strings.len()
        } else {
            self.strings.len()
        }
    }

    /// Borrow the underlying deque.
    pub fn strings(&self, regexp: bool) -> &VecDeque<LispString> {
        if regexp {
            &self.regexp_strings
        } else {
            &self.strings
        }
    }
}

impl Default for SearchHistory {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// IsearchManager
// ---------------------------------------------------------------------------

/// Manages the lifecycle of incremental search sessions.
pub struct IsearchManager {
    state: Option<IsearchState>,
    history: SearchHistory,
    last_search_string: Option<LispString>,
    last_search_regexp: bool,
}

impl IsearchManager {
    pub fn new() -> Self {
        Self {
            state: None,
            history: SearchHistory::new(),
            last_search_string: None,
            last_search_regexp: false,
        }
    }

    // -- Start/end ----------------------------------------------------------

    /// Begin a new incremental search session.
    pub fn begin_search(&mut self, direction: SearchDirection, regexp: bool, origin: usize) {
        self.state = Some(IsearchState {
            active: true,
            direction,
            search_string: empty_lisp_string(true),
            regexp,
            case_fold: None, // auto
            wrapped: false,
            success: true,
            match_start: None,
            match_end: None,
            origin,
            barrier: origin,
            history_index: None,
            lazy_matches: Vec::new(),
        });
    }

    /// End the search session normally, optionally saving the string to history.
    pub fn end_search(&mut self, save_to_history: bool) {
        if let Some(state) = self.state.take() {
            if save_to_history && !state.search_string.is_empty() {
                self.history.push(state.search_string.clone(), state.regexp);
            }
            if !state.search_string.is_empty() {
                self.last_search_regexp = state.regexp;
                self.last_search_string = Some(state.search_string);
            }
        }
    }

    /// Abort the search session.  Returns the original point position so that
    /// the caller can restore it.
    pub fn abort_search(&mut self) -> usize {
        let origin = self.state.as_ref().map(|s| s.origin).unwrap_or(0);
        self.state = None;
        origin
    }

    // -- Modify search ------------------------------------------------------

    /// Append a character to the search string.
    pub fn add_char(&mut self, ch: char) {
        if let Some(state) = self.state.as_mut() {
            append_char_to_lisp_string(&mut state.search_string, ch);
            state.history_index = None;
        }
    }

    /// Remove the last character from the search string.
    pub fn delete_char(&mut self) {
        if let Some(state) = self.state.as_mut() {
            pop_char_from_lisp_string(&mut state.search_string);
            state.history_index = None;
        }
    }

    /// Replace the search string wholesale.
    pub fn set_string(&mut self, s: LispString) {
        if let Some(state) = self.state.as_mut() {
            state.search_string = s;
            state.history_index = None;
        }
    }

    /// Toggle between literal and regexp search.
    pub fn toggle_regexp(&mut self) {
        if let Some(state) = self.state.as_mut() {
            state.regexp = !state.regexp;
        }
    }

    /// Toggle case-fold cycling: auto -> fold -> exact -> auto.
    pub fn toggle_case_fold(&mut self) {
        if let Some(state) = self.state.as_mut() {
            state.case_fold = match state.case_fold {
                None => Some(true),
                Some(true) => Some(false),
                Some(false) => None,
            };
        }
    }

    /// Reverse the search direction.
    pub fn reverse_direction(&mut self) {
        if let Some(state) = self.state.as_mut() {
            state.direction = match state.direction {
                SearchDirection::Forward => SearchDirection::Backward,
                SearchDirection::Backward => SearchDirection::Forward,
            };
        }
    }

    // -- Search operations --------------------------------------------------

    /// Perform one incremental search step in the current direction, starting
    /// from the current match position (or the barrier after a wrap).
    ///
    /// `text` is the full buffer contents.  Returns the match range if
    /// found.  The caller is responsible for moving point.
    pub fn search_next(&mut self, text: &str) -> Option<MatchGroup> {
        let state = self.state.as_mut()?;
        if state.search_string.is_empty() {
            state.success = true;
            state.match_start = None;
            state.match_end = None;
            return None;
        }

        // Issue #131: match the buffer text against the search string's real
        // Emacs bytes — no storage-String round-trip.
        let case_fold = resolve_case_fold(state.case_fold, &state.search_string);

        // Determine the starting position for the next search step.
        let from = match state.direction {
            SearchDirection::Forward => state.match_end.unwrap_or(state.barrier),
            SearchDirection::Backward => state.match_start.unwrap_or(state.barrier),
        };

        let forward = state.direction == SearchDirection::Forward;

        if let Some(range) = find_match(
            text.as_bytes(),
            state.search_string.as_bytes(),
            from,
            forward,
            state.regexp,
            case_fold,
        ) {
            state.success = true;
            state.match_start = Some(range.start());
            state.match_end = Some(range.end());
            return Some(range);
        }

        // Not found from current position — try wrapping.
        if !state.wrapped {
            state.wrapped = true;
            let wrap_from = if forward { 0 } else { text.len() };
            if let Some(range) = find_match(
                text.as_bytes(),
                state.search_string.as_bytes(),
                wrap_from,
                forward,
                state.regexp,
                case_fold,
            ) {
                state.success = true;
                state.match_start = Some(range.start());
                state.match_end = Some(range.end());
                return Some(range);
            }
        }

        state.success = false;
        None
    }

    /// Re-run the search from the origin for the current search string (used
    /// after each character addition/deletion to update the match).
    ///
    /// `text` is the full buffer contents.  Returns the match range if
    /// found.
    pub fn search_update(&mut self, text: &str) -> Option<MatchGroup> {
        let state = self.state.as_mut()?;
        if state.search_string.is_empty() {
            state.success = true;
            state.match_start = None;
            state.match_end = None;
            state.wrapped = false;
            return None;
        }

        // Issue #131: match against the search string's real Emacs bytes.
        let case_fold = resolve_case_fold(state.case_fold, &state.search_string);
        let forward = state.direction == SearchDirection::Forward;

        // Search from origin first.
        if let Some(range) = find_match(
            text.as_bytes(),
            state.search_string.as_bytes(),
            state.origin,
            forward,
            state.regexp,
            case_fold,
        ) {
            state.success = true;
            state.wrapped = false;
            state.match_start = Some(range.start());
            state.match_end = Some(range.end());
            return Some(range);
        }

        // Wrap around.
        let wrap_from = if forward { 0 } else { text.len() };
        if let Some(range) = find_match(
            text.as_bytes(),
            state.search_string.as_bytes(),
            wrap_from,
            forward,
            state.regexp,
            case_fold,
        ) {
            state.success = true;
            state.wrapped = true;
            state.match_start = Some(range.start());
            state.match_end = Some(range.end());
            return Some(range);
        }

        state.success = false;
        state.wrapped = false;
        state.match_start = None;
        state.match_end = None;
        None
    }

    /// Compute all matches within the visible region for lazy-highlight
    /// overlays.
    pub fn compute_lazy_matches(&mut self, text: &str, visible_start: usize, visible_end: usize) {
        let state = match self.state.as_mut() {
            Some(s) => s,
            None => return,
        };

        state.lazy_matches.clear();

        if state.search_string.is_empty() {
            return;
        }

        // Issue #131: match the search string's real Emacs bytes against the
        // visible region with the byte-native engine (`find_match`), the same
        // path used by `search_next`/`search_update`. The previous literal branch
        // lowercased through Rust `str` ops, which is not byte-faithful for
        // eight-bit content; `find_match` folds via Emacs char codes instead.
        let case_fold = resolve_case_fold(state.case_fold, &state.search_string);
        let start = visible_start.min(text.len());
        let end = visible_end.min(text.len());

        if start >= end {
            return;
        }

        let region = &text.as_bytes()[start..end];
        let needle = state.search_string.as_bytes();

        if state.regexp {
            if let Ok(iterated) = super::regex::iterate_string_matches_with_case_fold(
                &state.search_string,
                region,
                0,
                case_fold,
            ) {
                for groups in iterated.matches {
                    let Some(group) = groups.first().and_then(|group| *group) else {
                        continue;
                    };
                    if group.start() == group.end() {
                        continue;
                    }
                    state.lazy_matches.push(group.shift(start));
                }
            }
        } else {
            let mut search_from = 0;
            while let Some(range) = find_match(region, needle, search_from, true, false, case_fold)
            {
                if range.start() == range.end() {
                    break;
                }
                state
                    .lazy_matches
                    .push(MatchGroup::new(start + range.start(), start + range.end()));
                search_from = range.end();
            }
        }
    }

    // -- History navigation -------------------------------------------------

    /// Move to the previous (older) history entry.
    pub fn history_previous(&mut self) {
        let state = match self.state.as_mut() {
            Some(s) => s,
            None => return,
        };
        let ring_len = self.history.len(state.regexp);
        if ring_len == 0 {
            return;
        }
        let new_index = match state.history_index {
            None => 0,
            Some(i) => {
                if i + 1 < ring_len {
                    i + 1
                } else {
                    return;
                }
            }
        };
        if let Some(s) = self.history.get(new_index, state.regexp) {
            state.search_string = s.clone();
            state.history_index = Some(new_index);
        }
    }

    /// Move to the next (newer) history entry.
    pub fn history_next(&mut self) {
        let state = match self.state.as_mut() {
            Some(s) => s,
            None => return,
        };
        match state.history_index {
            None => {}
            Some(0) => {
                state.search_string = empty_lisp_string(state.search_string.is_multibyte());
                state.history_index = None;
            }
            Some(i) => {
                let new_index = i - 1;
                if let Some(s) = self.history.get(new_index, state.regexp) {
                    state.search_string = s.clone();
                    state.history_index = Some(new_index);
                }
            }
        }
    }

    /// Yank the word (or character) at point into the search string.
    ///
    /// `text` is the full buffer text; `point` is the current cursor position.
    /// Appends text from `point` up to the next word boundary.
    pub fn yank_word_or_char(&mut self, text: &str, point: usize) {
        let state = match self.state.as_mut() {
            Some(s) => s,
            None => return,
        };

        if point >= text.len() {
            return;
        }

        let rest = &text[point..];
        let mut end = 0;
        let mut chars = rest.chars();

        // Grab at least one char; then continue while alphanumeric.
        if let Some(ch) = chars.next() {
            end += ch.len_utf8();
            if ch.is_alphanumeric() || ch == '_' {
                for ch2 in chars {
                    if ch2.is_alphanumeric() || ch2 == '_' {
                        end += ch2.len_utf8();
                    } else {
                        break;
                    }
                }
            }
        }

        append_runtime_fragment_to_lisp_string(&mut state.search_string, &rest[..end]);
        state.history_index = None;
    }

    // -- State queries ------------------------------------------------------

    /// Whether an incremental search is currently active.
    pub fn is_active(&self) -> bool {
        self.state.as_ref().is_some_and(|s| s.active)
    }

    /// Borrow the current state (if any).
    pub fn state(&self) -> Option<&IsearchState> {
        self.state.as_ref()
    }

    /// Build the minibuffer prompt string for the current search.
    pub fn prompt(&self) -> String {
        let state = match self.state.as_ref() {
            Some(s) => s,
            None => return String::new(),
        };

        let mut parts = Vec::new();

        if !state.success {
            parts.push("Failing");
        }
        if state.wrapped {
            parts.push("Wrapped");
        }
        if state.regexp {
            parts.push("Regexp");
        }

        let dir = match state.direction {
            SearchDirection::Forward => "I-search",
            SearchDirection::Backward => "I-search backward",
        };
        parts.push(dir);

        let prompt = parts.join(" ");
        format!(
            "{}: {}",
            prompt,
            lisp_string_for_display(&state.search_string)
        )
    }
}

impl Default for IsearchManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Query-replace response
// ---------------------------------------------------------------------------

/// Possible user responses during a query-replace session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryReplaceResponse {
    /// `y` / `SPC` — replace this match.
    Yes,
    /// `n` / `DEL` — skip this match.
    No,
    /// `!` — replace all remaining matches without asking.
    ReplaceAll,
    /// `q` / `RET` — stop replacing.
    Quit,
    /// `e` — edit the replacement string for this match.
    Edit,
    /// `d` — delete the match text without inserting the replacement.
    Delete,
    /// `u` — undo the last replacement.
    Undo,
    /// `?` — show help text.
    Help,
}

// ---------------------------------------------------------------------------
// QueryReplaceUndo
// ---------------------------------------------------------------------------

/// Record of a single replacement for undo purposes.
#[derive(Clone, Debug)]
pub struct QueryReplaceUndo {
    /// Byte position where the replacement was made.
    pub position: usize,
    /// Original matched text.
    pub original: LispString,
    /// Replacement text that was inserted.
    pub replacement: LispString,
}

// ---------------------------------------------------------------------------
// QueryReplaceState
// ---------------------------------------------------------------------------

/// Full state for one active query-replace session.
pub struct QueryReplaceState {
    /// Pattern to search for.
    pub from_string: LispString,
    /// Replacement string.
    pub to_string: LispString,
    /// Whether `from_string` is a regular expression.
    pub regexp: bool,
    /// Whether to match only whole delimited words.
    pub delimited: bool,
    /// Case folding override: `None` = auto, `Some(true)` = fold, `Some(false)` = exact.
    pub case_fold: Option<bool>,
    /// Whether to preserve the case pattern of the matched text.
    pub preserve_case: bool,
    /// Optional region restriction (start byte).
    pub region_start: Option<usize>,
    /// Optional region restriction (end byte).
    pub region_end: Option<usize>,
    /// Current match being presented to the user.
    pub current_match: Option<MatchGroup>,
    /// Number of replacements made so far.
    pub replaced_count: usize,
    /// Number of matches skipped so far.
    pub skipped_count: usize,
    /// Stack of undoable replacements (most recent last).
    pub undo_stack: Vec<QueryReplaceUndo>,
}

// ---------------------------------------------------------------------------
// QueryReplaceAction
// ---------------------------------------------------------------------------

/// Action the caller should take after a query-replace response.
#[derive(Clone, Debug)]
pub enum QueryReplaceAction {
    /// Replace the region `[start, end)` with the given string.
    Replace(usize, usize, LispString),
    /// Skip the current match.
    Skip,
    /// The session is finished.
    Done(QueryReplaceSummary),
    /// Display this help text to the user.
    ShowHelp(String),
    /// The user asked to edit the replacement — caller should prompt for input.
    NeedInput,
}

// ---------------------------------------------------------------------------
// QueryReplaceSummary
// ---------------------------------------------------------------------------

/// Summary statistics returned when a query-replace session ends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryReplaceSummary {
    pub replaced: usize,
    pub skipped: usize,
}

// ---------------------------------------------------------------------------
// QueryReplaceManager
// ---------------------------------------------------------------------------

/// Manages query-replace sessions.
pub struct QueryReplaceManager {
    state: Option<QueryReplaceState>,
}

impl QueryReplaceManager {
    pub fn new() -> Self {
        Self { state: None }
    }

    /// Begin a new query-replace session (whole buffer).
    pub fn begin(&mut self, from: LispString, to: LispString, regexp: bool) {
        self.state = Some(QueryReplaceState {
            from_string: from,
            to_string: to,
            regexp,
            delimited: false,
            case_fold: None,
            preserve_case: true,
            region_start: None,
            region_end: None,
            current_match: None,
            replaced_count: 0,
            skipped_count: 0,
            undo_stack: Vec::new(),
        });
    }

    /// Begin a query-replace session restricted to a region.
    pub fn begin_in_region(
        &mut self,
        from: LispString,
        to: LispString,
        regexp: bool,
        start: usize,
        end: usize,
    ) {
        self.begin(from, to, regexp);
        if let Some(state) = self.state.as_mut() {
            state.region_start = Some(start);
            state.region_end = Some(end);
        }
    }

    /// Find the next match at or after `from_pos`.
    ///
    /// `text` is the full buffer contents.  Returns the range of the
    /// match, also storing it in `current_match`.  Returns `None` when there
    /// are no more matches.
    pub fn find_next(&mut self, text: &str, from_pos: usize) -> Option<MatchGroup> {
        let state = self.state.as_mut()?;

        let limit = state.region_end.unwrap_or(text.len()).min(text.len());
        let start = from_pos.max(state.region_start.unwrap_or(0));

        if start > limit {
            state.current_match = None;
            return None;
        }

        // Issue #131: match against FROM's real Emacs bytes — no storage form.
        let case_fold = resolve_case_fold(state.case_fold, &state.from_string);
        let result = find_match(
            text.as_bytes(),
            state.from_string.as_bytes(),
            start,
            true,
            state.regexp,
            case_fold,
        );

        if let Some(range) = result
            && range.end() <= limit
        {
            state.current_match = Some(range);
            return Some(range);
        }

        state.current_match = None;
        None
    }

    /// Apply the user's response to the current match.
    pub fn respond(&mut self, response: QueryReplaceResponse) -> QueryReplaceAction {
        let state = match self.state.as_mut() {
            Some(s) => s,
            None => {
                return QueryReplaceAction::Done(QueryReplaceSummary {
                    replaced: 0,
                    skipped: 0,
                });
            }
        };

        match response {
            QueryReplaceResponse::Yes => {
                if let Some(current_match) = state.current_match {
                    let start = current_match.start();
                    let end = current_match.end();
                    let matched_text = empty_lisp_string(state.from_string.is_multibyte()); // caller fills this in
                    let replacement = state.to_string.clone();
                    let replacement = if state.preserve_case {
                        // We cannot access the matched text here; the caller
                        // should use `compute_replacement` before calling
                        // `respond`.  Return the raw replacement.
                        replacement
                    } else {
                        replacement
                    };
                    state.replaced_count += 1;
                    state.undo_stack.push(QueryReplaceUndo {
                        position: start,
                        original: matched_text,
                        replacement: replacement.clone(),
                    });
                    state.current_match = None;
                    QueryReplaceAction::Replace(start, end, replacement)
                } else {
                    QueryReplaceAction::Skip
                }
            }
            QueryReplaceResponse::No => {
                state.skipped_count += 1;
                state.current_match = None;
                QueryReplaceAction::Skip
            }
            QueryReplaceResponse::ReplaceAll => {
                // Signal the caller to replace the current match and all
                // remaining ones.  We handle the *current* match here; the
                // caller should loop `find_next` + `respond(Yes)` for the rest.
                if let Some(current_match) = state.current_match {
                    let start = current_match.start();
                    let end = current_match.end();
                    let replacement = state.to_string.clone();
                    state.replaced_count += 1;
                    state.undo_stack.push(QueryReplaceUndo {
                        position: start,
                        original: empty_lisp_string(state.from_string.is_multibyte()),
                        replacement: replacement.clone(),
                    });
                    state.current_match = None;
                    QueryReplaceAction::Replace(start, end, replacement)
                } else {
                    QueryReplaceAction::Skip
                }
            }
            QueryReplaceResponse::Quit => {
                let summary = QueryReplaceSummary {
                    replaced: state.replaced_count,
                    skipped: state.skipped_count,
                };
                self.state = None;
                QueryReplaceAction::Done(summary)
            }
            QueryReplaceResponse::Edit => QueryReplaceAction::NeedInput,
            QueryReplaceResponse::Delete => {
                if let Some(current_match) = state.current_match {
                    let start = current_match.start();
                    let end = current_match.end();
                    state.replaced_count += 1;
                    state.undo_stack.push(QueryReplaceUndo {
                        position: start,
                        original: empty_lisp_string(state.from_string.is_multibyte()),
                        replacement: empty_lisp_string(state.to_string.is_multibyte()),
                    });
                    state.current_match = None;
                    // Replace with empty string = delete
                    QueryReplaceAction::Replace(
                        start,
                        end,
                        empty_lisp_string(state.to_string.is_multibyte()),
                    )
                } else {
                    QueryReplaceAction::Skip
                }
            }
            QueryReplaceResponse::Undo => {
                // Return the last undo entry via Done-like mechanism.
                // The actual undo application is done by `undo_last`.
                QueryReplaceAction::Skip
            }
            QueryReplaceResponse::Help => {
                let help = concat!(
                    "y/SPC - replace this match\n",
                    "n/DEL - skip this match\n",
                    "! - replace all remaining matches\n",
                    "q/RET - quit\n",
                    "e - edit replacement\n",
                    "d - delete match (no replacement)\n",
                    "u - undo last replacement\n",
                    "? - show this help",
                );
                QueryReplaceAction::ShowHelp(help.to_string())
            }
        }
    }

    /// Compute the replacement text for a given matched string.
    ///
    /// Handles `preserve_case` logic.  For regexp replacements the caller
    /// should additionally process `\&` and `\N` references (see
    /// `regex::build_replacement`).
    pub fn compute_replacement(&self, matched: &str) -> LispString {
        let state = match self.state.as_ref() {
            Some(s) => s,
            None => return empty_lisp_string(true),
        };

        if state.preserve_case {
            // Issue #131: preserve the matched text's case over TO's real Emacs
            // bytes, instead of round-tripping through a storage String.
            let matched_ls = LispString::from_utf8(matched);
            super::casefiddle::apply_replace_match_case_lisp(&state.to_string, &matched_ls)
        } else {
            state.to_string.clone()
        }
    }

    /// Pop and return the most recent undo entry.
    pub fn undo_last(&mut self) -> Option<QueryReplaceUndo> {
        let state = self.state.as_mut()?;
        let entry = state.undo_stack.pop();
        if entry.is_some() {
            // Decrement replaced count since we are undoing.
            state.replaced_count = state.replaced_count.saturating_sub(1);
        }
        entry
    }

    /// End the session and return a summary.
    pub fn finish(&mut self) -> QueryReplaceSummary {
        let state = match self.state.take() {
            Some(s) => s,
            None => {
                return QueryReplaceSummary {
                    replaced: 0,
                    skipped: 0,
                };
            }
        };
        QueryReplaceSummary {
            replaced: state.replaced_count,
            skipped: state.skipped_count,
        }
    }

    /// Whether a query-replace session is currently active.
    pub fn is_active(&self) -> bool {
        self.state.is_some()
    }

    /// Borrow the current state (if any).
    pub fn state(&self) -> Option<&QueryReplaceState> {
        self.state.as_ref()
    }

    /// Build the minibuffer prompt for the current session.
    pub fn prompt(&self) -> String {
        let state = match self.state.as_ref() {
            Some(s) => s,
            None => return String::new(),
        };
        let kind = if state.regexp {
            "Query replacing regexp"
        } else {
            "Query replacing"
        };
        format!(
            "{} {} with {}: (y/n/!/q/?)",
            kind,
            lisp_string_for_display(&state.from_string),
            lisp_string_for_display(&state.to_string)
        )
    }
}

impl Default for QueryReplaceManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Helper: resolve case folding
// ---------------------------------------------------------------------------

/// Determine effective case folding.
///
/// When `override_val` is `None` (auto), we fold if the search string is
/// entirely lowercase (Emacs `isearch-no-upper-case-p` heuristic).
///
/// Issue #131: scan the search string's real Emacs char codes (eight-bit raw
/// bytes and PUA glyphs are caseless) instead of a storage-String round-trip.
fn resolve_case_fold(override_val: Option<bool>, search_string: &LispString) -> bool {
    match override_val {
        Some(v) => v,
        None => {
            // Auto: fold if no uppercase letters in the search string.
            !lisp_pattern_has_uppercase(search_string)
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: build regex pattern with optional case-insensitive flag
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Helper: find_match — general-purpose substring/regex search
// ---------------------------------------------------------------------------

/// Search for `pattern` in `text`.
///
/// - `from`:    byte offset to start searching from.
/// - `forward`: direction of search.
/// - `regexp`:  treat `pattern` as an Emacs regular expression.
/// - `case_fold`: perform case-insensitive matching.
///
/// Returns the match byte range into `text`, or `None`.
/// Forward byte-substring search: first index of `needle` in `hay`.
fn byte_find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if needle.len() > hay.len() {
        return None;
    }
    hay.windows(needle.len())
        .position(|window| window == needle)
}

/// Backward byte-substring search: last index of `needle` in `hay`.
fn byte_rfind(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(hay.len());
    }
    if needle.len() > hay.len() {
        return None;
    }
    hay.windows(needle.len())
        .rposition(|window| window == needle)
}

/// Emacs-downcase a single char code for case-insensitive comparison (issue
/// #131): codes outside the Unicode scalar range (eight-bit raw bytes, extended
/// codes) are caseless; rare multi-char lowercase mappings leave the code as-is.
fn find_match(
    text: &[u8],
    pattern: &[u8],
    from: usize,
    forward: bool,
    regexp: bool,
    case_fold: bool,
) -> Option<MatchGroup> {
    if pattern.is_empty() {
        return None;
    }

    let text_len = text.len();

    if regexp {
        let pattern_ls = crate::heap_types::LispString::from_emacs_bytes(pattern.to_vec());
        if forward {
            let start = from.min(text_len);
            let iterated = super::regex::iterate_string_matches_with_case_fold(
                &pattern_ls,
                text,
                start,
                case_fold,
            )
            .ok()?;
            iterated
                .matches
                .into_iter()
                .find_map(|groups| groups.first().and_then(|group| *group))
        } else {
            let end = from.min(text_len);
            let iterated = super::regex::iterate_string_matches_with_case_fold(
                &pattern_ls,
                &text[..end],
                0,
                case_fold,
            )
            .ok()?;
            iterated
                .matches
                .into_iter()
                .filter_map(|groups| groups.first().and_then(|group| *group))
                .next_back()
        }
    } else {
        // Literal search over Emacs bytes (issue #131): byte-exact when not
        // folding; when folding, compare Emacs-downcased char codes in place so
        // match offsets stay in the text's own byte space.
        if forward {
            let start = from.min(text_len);
            if case_fold {
                let mut p = start;
                loop {
                    if let Some(end) =
                        crate::emacs_core::emacs_char::case_fold_match_len(text, p, pattern)
                    {
                        return Some(MatchGroup::new(p, end));
                    }
                    if p >= text_len {
                        return None;
                    }
                    let (_code, len) = crate::emacs_core::emacs_char::string_char(&text[p..]);
                    p += len.max(1);
                }
            } else {
                let region = &text[start..];
                let pos = byte_find(region, pattern)?;
                Some(MatchGroup::new(start + pos, start + pos + pattern.len()))
            }
        } else {
            let end = from.min(text_len);
            if case_fold {
                // Rightmost match within text[..end], matching the prior `rfind`.
                let mut best = None;
                let mut p = 0;
                while p < end {
                    if let Some(match_end) =
                        crate::emacs_core::emacs_char::case_fold_match_len(&text[..end], p, pattern)
                    {
                        best = Some(MatchGroup::new(p, match_end));
                    }
                    let (_code, len) = crate::emacs_core::emacs_char::string_char(&text[p..]);
                    p += len.max(1);
                }
                best
            } else {
                let region = &text[..end];
                let pos = byte_rfind(region, pattern)?;
                Some(MatchGroup::new(pos, pos + pattern.len()))
            }
        }
    }
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn is_delimited_word_char(ch: char) -> bool {
    ch.is_alphanumeric()
}

// ---------------------------------------------------------------------------
// Helper: case-preserving replacement
// ---------------------------------------------------------------------------

/// Produce a replacement string that preserves the case pattern of the
/// matched text.
///
/// Rules (matching Emacs `replace-match` behavior):
/// - If `matched` is all-uppercase, upcase the entire replacement.
/// - If `matched` starts with an uppercase letter and the rest is lowercase
///   (capitalized), uppercase the first char of replacement and keep the rest.
/// - Otherwise return `replacement` unmodified.
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn preserve_case(replacement: &str, matched: &str) -> String {
    super::casefiddle::apply_replace_match_case(replacement, matched)
}

// ---------------------------------------------------------------------------
// Issue #131: Emacs-byte-native variants of the replace helpers. These operate
// over real Emacs bytes (and char codes via `emacs_char`) instead of the legacy
// PUA-sentinel storage form, so eight-bit content and Private-Use-Area glyphs
// round-trip faithfully through delimited-match, case-preservation, and
// `\N`/`\&` replacement expansion.
// ---------------------------------------------------------------------------

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn code_is_delimited_word_char(code: u32) -> bool {
    char::from_u32(code).is_some_and(is_delimited_word_char)
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn is_delimited_match_bytes(text: &[u8], start: usize, end: usize) -> bool {
    let left = if start > 0 {
        let prev_len = crate::emacs_core::emacs_char::raw_prev_char_len(text, start);
        let prev_start = start.saturating_sub(prev_len);
        Some(crate::emacs_core::emacs_char::string_char(&text[prev_start..start]).0)
    } else {
        None
    };
    let right = if end < text.len() {
        Some(crate::emacs_core::emacs_char::string_char(&text[end..]).0)
    } else {
        None
    };
    let left_ok = left.is_none_or(|code| !code_is_delimited_word_char(code));
    let right_ok = right.is_none_or(|code| !code_is_delimited_word_char(code));
    left_ok && right_ok
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn preserve_case_emacs_bytes(replacement: &[u8], matched: &[u8], multibyte: bool) -> Vec<u8> {
    let rep = lisp_string_from_buffer_bytes(replacement.to_vec(), multibyte);
    let mat = lisp_string_from_buffer_bytes(matched.to_vec(), multibyte);
    super::casefiddle::apply_replace_match_case_lisp(&rep, &mat)
        .as_bytes()
        .to_vec()
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn expand_emacs_replacement_bytes(
    rep: &[u8],
    groups: &[Option<MatchGroup>],
    source: &[u8],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(rep.len());
    let mut i = 0;
    while i < rep.len() {
        let ch = rep[i];
        if ch != b'\\' {
            out.push(ch);
            i += 1;
            continue;
        }
        if i + 1 >= rep.len() {
            out.push(b'\\');
            break;
        }
        let next = rep[i + 1];
        match next {
            b'&' => {
                if let Some(Some(group)) = groups.first()
                    && let Some(text) = source.get(group.start()..group.end())
                {
                    out.extend_from_slice(text);
                }
                i += 2;
            }
            b'1'..=b'9' => {
                let idx = (next - b'0') as usize;
                if let Some(Some(group)) = groups.get(idx)
                    && let Some(text) = source.get(group.start()..group.end())
                {
                    out.extend_from_slice(text);
                }
                i += 2;
            }
            b'\\' => {
                out.push(b'\\');
                i += 2;
            }
            _ => {
                // `\X` for any other X drops the backslash and emits X verbatim,
                // matching the &str variant. Emit the whole (possibly multibyte)
                // Emacs char so a char following the backslash is not split.
                let (_code, clen) = crate::emacs_core::emacs_char::string_char(&rep[i + 1..]);
                let clen = clen.max(1);
                out.extend_from_slice(&rep[i + 1..i + 1 + clen]);
                i += 1 + clen;
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Builtin functions (stubs for evaluator dispatch)
// ---------------------------------------------------------------------------

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn replace_string_eval_impl(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
    query_style_point: bool,
) -> EvalResult {
    expect_min_max_args("replace-string", &args, 2, 7)?;
    let from_ls = expect_sequence_string(&args[0])?;
    let to_ls = expect_string(&args[1])?;
    let to = to_ls.as_bytes();
    let delimited = args.get(2).is_some_and(|v| !v.is_nil());
    let backward = args.get(5).is_some_and(|v| !v.is_nil());
    let region_noncontiguous = args.get(6).is_some_and(|v| !v.is_nil());
    if region_noncontiguous && !backward {
        let point_max = {
            let buf = eval
                .buffers
                .current_buffer()
                .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
            if buf.mark_emacs_byte_pos().is_none() {
                return Err(signal(
                    "error",
                    vec![Value::string(
                        "The mark is not set now, so there is no region",
                    )],
                ));
            }
            buf.accessible_emacs_byte_region().end()
        };
        let current_id = eval
            .buffers
            .current_buffer_id()
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
        let _ = eval
            .buffers
            .goto_buffer_emacs_byte_pos(current_id, point_max);
        return Ok(Value::NIL);
    }
    let (range, source_text, read_only, buffer_name) = {
        let buf = eval
            .buffers
            .current_buffer()
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
        let range = replacement_region_bounds(
            &eval.buffers,
            buf,
            args.get(3),
            args.get(4),
            backward,
            region_noncontiguous,
        )?;
        let source_text = buf.buffer_substring_lisp_string_range(range);
        (
            range,
            source_text,
            buffer_read_only_active(eval, buf),
            buf.name_value(),
        )
    };
    let range_start = range.start();
    let source_multibyte = source_text.is_multibyte();
    // Issue #131: operate on the region's Emacs bytes directly — match offsets are
    // logical Emacs byte offsets, so the old storage->logical mapping is identity.
    let source = source_text.as_bytes();

    if from_ls.is_empty() {
        if source.is_empty() {
            return Ok(Value::NIL);
        }
        if read_only {
            return Err(signal(LispCondition::BufferReadOnly, vec![buffer_name]));
        }
        // Issue #131: iterate the region's Emacs chars as (byte-start, byte-len)
        // pairs instead of the storage-unit scan; for Emacs bytes the char's byte
        // range is its logical byte range.
        let mut units: Vec<(usize, usize)> = Vec::new();
        let mut scan = 0usize;
        while scan < source.len() {
            let (_code, len) = crate::emacs_core::emacs_char::string_char(&source[scan..]);
            let len = len.max(1);
            units.push((scan, len));
            scan += len;
        }
        let mut out: Vec<u8> = Vec::with_capacity(source.len() + to.len() * units.len());
        if backward {
            for &(start, len) in &units {
                out.extend_from_slice(&source[start..start + len]);
                out.extend_from_slice(to);
            }
        } else {
            for &(start, len) in &units {
                out.extend_from_slice(to);
                out.extend_from_slice(&source[start..start + len]);
            }
        }
        if out.as_slice() == source {
            return Ok(Value::NIL);
        }
        let current_id = eval
            .buffers
            .current_buffer_id()
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
        let out_text = lisp_string_from_buffer_bytes(out, source_multibyte);
        let change = super::editfns::text_change_for_lisp_string_replacement_in_manager(
            &eval.buffers,
            current_id,
            range,
            &out_text,
        )?;
        let new_len = change.new_extent().emacs_bytes();
        super::editfns::signal_before_text_change(eval, change)?;
        let _ = eval
            .buffers
            .delete_buffer_measured_region(current_id, change.old_range());
        let _ = eval
            .buffers
            .goto_buffer_emacs_byte_pos(current_id, range_start);
        let _ = eval
            .buffers
            .insert_lisp_string_into_buffer(current_id, &out_text);
        super::editfns::signal_after_text_change(eval, change)?;
        if backward {
            if let Some(&(_, first_len)) = units.first() {
                let _ = eval.buffers.goto_buffer_emacs_byte_pos(
                    current_id,
                    range_start.add_len(EmacsByteLen::new(first_len)),
                );
            } else {
                let _ = eval
                    .buffers
                    .goto_buffer_emacs_byte_pos(current_id, range_start);
            }
        } else if query_style_point {
            if let Some(&(_, last_len)) = units.last() {
                let _ = eval.buffers.goto_buffer_emacs_byte_pos(
                    current_id,
                    range_start.add_len(EmacsByteLen::new(new_len.get().saturating_sub(last_len))),
                );
            } else {
                let _ = eval
                    .buffers
                    .goto_buffer_emacs_byte_pos(current_id, range_start);
            }
        } else {
            let _ = eval
                .buffers
                .goto_buffer_emacs_byte_pos(current_id, range_start.add_len(new_len));
        }
        return Ok(Value::NIL);
    }

    let case_fold = case_fold_for_pattern(eval, from_ls);
    let preserve_match_case = case_fold && case_replace_enabled(eval);
    let lax_whitespace_regex =
        if replace_lax_whitespace_enabled(eval) && from_ls.as_bytes().contains(&b' ') {
            resolve_search_whitespace_regexp(eval)
        } else {
            None
        };
    let mut out: Vec<u8> = Vec::with_capacity(source.len());
    let mut replaced = 0usize;
    let mut backward_point: Option<EmacsByteLen> = None;
    let mut query_forward_point: Option<EmacsByteLen> = None;

    if let Some(whitespace_regex) = lax_whitespace_regex {
        // Issue #131: build the lax-whitespace pattern from the real Emacs bytes
        // of FROM and the whitespace regexp, then wrap as a byte-faithful pattern
        // LispString for the byte-native match engine.
        let pattern = build_lax_whitespace_pattern(from_ls.as_bytes(), whitespace_regex.as_bytes());
        let pattern_ls = LispString::from_emacs_bytes(pattern);
        let iterated =
            super::regex::iterate_string_matches_with_case_fold(&pattern_ls, source, 0, case_fold)
                .map_err(|e| signal(LispCondition::InvalidRegexp, vec![Value::string(e)]))?;
        let mut last = 0usize;
        for groups in iterated.matches {
            let Some(group) = groups.first().and_then(|group| *group) else {
                continue;
            };
            let m_start = group.start();
            let m_end = group.end();
            if delimited && !is_delimited_match_bytes(source, m_start, m_end) {
                continue;
            }
            out.extend_from_slice(&source[last..m_start]);
            let matched = &source[m_start..m_end];
            if preserve_match_case {
                out.extend_from_slice(&preserve_case_emacs_bytes(to, matched, source_multibyte));
            } else {
                out.extend_from_slice(to);
            }
            query_forward_point = Some(EmacsByteLen::new(out.len()));
            if backward && backward_point.is_none() {
                backward_point = Some(EmacsByteLen::new(m_start));
            }
            replaced += 1;
            last = m_end;
        }
        out.extend_from_slice(&source[last..]);
    } else {
        let mut cursor = 0usize;
        while let Some(range) =
            find_match(source, from_ls.as_bytes(), cursor, true, false, case_fold)
        {
            let m_start = range.start();
            let m_end = range.end();
            if delimited && !is_delimited_match_bytes(source, m_start, m_end) {
                out.extend_from_slice(&source[cursor..m_end]);
                cursor = m_end;
                continue;
            }
            out.extend_from_slice(&source[cursor..m_start]);
            let matched = &source[m_start..m_end];
            if preserve_match_case {
                out.extend_from_slice(&preserve_case_emacs_bytes(to, matched, source_multibyte));
            } else {
                out.extend_from_slice(to);
            }
            query_forward_point = Some(EmacsByteLen::new(out.len()));
            if backward && backward_point.is_none() {
                backward_point = Some(EmacsByteLen::new(m_start));
            }
            replaced += 1;
            cursor = m_end;
        }
        out.extend_from_slice(&source[cursor..]);
    }

    if replaced == 0 {
        return Ok(Value::NIL);
    }
    if read_only {
        return Err(signal(LispCondition::BufferReadOnly, vec![buffer_name]));
    }

    let current_id = eval
        .buffers
        .current_buffer_id()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    let out_text = lisp_string_from_buffer_bytes(out, source_multibyte);
    let change = super::editfns::text_change_for_lisp_string_replacement_in_manager(
        &eval.buffers,
        current_id,
        range,
        &out_text,
    )?;
    let new_len = change.new_extent().emacs_bytes();
    super::editfns::signal_before_text_change(eval, change)?;
    let _ = eval
        .buffers
        .delete_buffer_measured_region(current_id, change.old_range());
    let _ = eval
        .buffers
        .goto_buffer_emacs_byte_pos(current_id, range_start);
    let _ = eval
        .buffers
        .insert_lisp_string_into_buffer(current_id, &out_text);
    super::editfns::signal_after_text_change(eval, change)?;
    if backward {
        if let Some(pos) = backward_point {
            let _ = eval
                .buffers
                .goto_buffer_emacs_byte_pos(current_id, range_start.add_len(pos));
        } else {
            let _ = eval
                .buffers
                .goto_buffer_emacs_byte_pos(current_id, range_start);
        }
    } else if query_style_point {
        if let Some(pos) = query_forward_point {
            let _ = eval
                .buffers
                .goto_buffer_emacs_byte_pos(current_id, range_start.add_len(pos));
        } else {
            let _ = eval
                .buffers
                .goto_buffer_emacs_byte_pos(current_id, range_start);
        }
    } else {
        let _ = eval
            .buffers
            .goto_buffer_emacs_byte_pos(current_id, range_start.add_len(new_len));
    }

    Ok(Value::NIL)
}

/// `(replace-string FROM-STRING TO-STRING &optional DELIMITED START END BACKWARD REGION-NONCONTIGUOUS-P)` —
/// evaluator-backed non-interactive replace subset.
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn builtin_replace_string(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    replace_string_eval_impl(eval, args, false)
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn replace_regexp_eval_impl(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
    query_style_point: bool,
) -> EvalResult {
    expect_min_max_args("replace-regexp", &args, 2, 7)?;
    let from_ls = expect_sequence_string(&args[0])?;
    let to_ls = expect_string(&args[1])?;
    let to = to_ls.as_bytes();
    let delimited = args.get(2).is_some_and(|v| !v.is_nil());
    let backward = args.get(5).is_some_and(|v| !v.is_nil());
    let region_noncontiguous = args.get(6).is_some_and(|v| !v.is_nil());
    if region_noncontiguous && !backward {
        let point_max = {
            let buf = eval
                .buffers
                .current_buffer()
                .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
            if buf.mark_emacs_byte_pos().is_none() {
                return Err(signal(
                    "error",
                    vec![Value::string(
                        "The mark is not set now, so there is no region",
                    )],
                ));
            }
            buf.accessible_emacs_byte_region().end()
        };
        let current_id = eval
            .buffers
            .current_buffer_id()
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
        let _ = eval
            .buffers
            .goto_buffer_emacs_byte_pos(current_id, point_max);
        return Ok(Value::NIL);
    }

    let (range, source_text, read_only, buffer_name) = {
        let buf = eval
            .buffers
            .current_buffer()
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
        let range = replacement_region_bounds(
            &eval.buffers,
            buf,
            args.get(3),
            args.get(4),
            backward,
            region_noncontiguous,
        )?;
        let source_text = buf.buffer_substring_lisp_string_range(range);
        (
            range,
            source_text,
            buffer_read_only_active(eval, buf),
            buf.name_value(),
        )
    };
    let range_start = range.start();
    let source_multibyte = source_text.is_multibyte();
    // Issue #131: iterate the region's Emacs bytes directly — match offsets are
    // logical Emacs byte offsets, so the old storage->logical mapping is identity.
    let source = source_text.as_bytes();

    let case_fold = case_fold_for_pattern(eval, from_ls);
    let preserve_match_case = case_fold && case_replace_enabled(eval);
    let iterated =
        super::regex::iterate_string_matches_with_case_fold(from_ls, source, 0, case_fold)
            .map_err(|e| signal(LispCondition::InvalidRegexp, vec![Value::string(e)]))?;

    let mut out: Vec<u8> = Vec::with_capacity(source.len());
    let mut last = 0usize;
    let mut replaced = 0usize;
    let mut backward_point: Option<EmacsByteLen> = None;
    let mut query_forward_point: Option<EmacsByteLen> = None;
    for groups in iterated.matches {
        let Some(group) = groups.first().and_then(|group| *group) else {
            continue;
        };
        let match_start = group.start();
        let match_end = group.end();
        if delimited && !is_delimited_match_bytes(source, match_start, match_end) {
            continue;
        }
        if match_start == match_end {
            if backward {
                // Backward path inserts after each character and at region end, not at start.
                if match_start == 0 {
                    continue;
                }
            } else {
                // Forward path inserts before each character, not at end.
                if match_start >= source.len() {
                    continue;
                }
            }
            out.extend_from_slice(&source[last..match_start]);
            let expanded = expand_emacs_replacement_bytes(to, &groups, source);
            if preserve_match_case {
                out.extend_from_slice(&preserve_case_emacs_bytes(
                    &expanded,
                    &source[match_start..match_end],
                    source_multibyte,
                ));
            } else {
                out.extend_from_slice(&expanded);
            }
            query_forward_point = Some(EmacsByteLen::new(out.len()));
            last = match_start;
            if backward && backward_point.is_none() {
                backward_point = Some(EmacsByteLen::new(match_start));
            }
            replaced += 1;
            continue;
        }

        out.extend_from_slice(&source[last..match_start]);
        let expanded = expand_emacs_replacement_bytes(to, &groups, source);
        if preserve_match_case {
            out.extend_from_slice(&preserve_case_emacs_bytes(
                &expanded,
                &source[match_start..match_end],
                source_multibyte,
            ));
        } else {
            out.extend_from_slice(&expanded);
        }
        query_forward_point = Some(EmacsByteLen::new(out.len()));
        last = match_end;
        if backward && backward_point.is_none() {
            backward_point = Some(EmacsByteLen::new(match_start));
        }
        replaced += 1;
    }
    out.extend_from_slice(&source[last..]);

    if replaced == 0 {
        return Ok(Value::NIL);
    }
    if read_only {
        return Err(signal(LispCondition::BufferReadOnly, vec![buffer_name]));
    }

    let current_id = eval
        .buffers
        .current_buffer_id()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    let out_text = lisp_string_from_buffer_bytes(out, source_multibyte);
    let change = super::editfns::text_change_for_lisp_string_replacement_in_manager(
        &eval.buffers,
        current_id,
        range,
        &out_text,
    )?;
    let new_len = change.new_extent().emacs_bytes();
    super::editfns::signal_before_text_change(eval, change)?;
    let _ = eval
        .buffers
        .delete_buffer_measured_region(current_id, change.old_range());
    let _ = eval
        .buffers
        .goto_buffer_emacs_byte_pos(current_id, range_start);
    let _ = eval
        .buffers
        .insert_lisp_string_into_buffer(current_id, &out_text);
    super::editfns::signal_after_text_change(eval, change)?;
    if backward {
        if let Some(pos) = backward_point {
            let _ = eval
                .buffers
                .goto_buffer_emacs_byte_pos(current_id, range_start.add_len(pos));
        } else {
            let _ = eval
                .buffers
                .goto_buffer_emacs_byte_pos(current_id, range_start);
        }
    } else if query_style_point {
        if let Some(pos) = query_forward_point {
            let _ = eval
                .buffers
                .goto_buffer_emacs_byte_pos(current_id, range_start.add_len(pos));
        } else {
            let _ = eval
                .buffers
                .goto_buffer_emacs_byte_pos(current_id, range_start);
        }
    } else {
        let _ = eval
            .buffers
            .goto_buffer_emacs_byte_pos(current_id, range_start.add_len(new_len));
    }

    Ok(Value::NIL)
}

/// `(replace-regexp REGEXP TO-STRING &optional DELIMITED START END BACKWARD REGION-NONCONTIGUOUS-P)` —
/// evaluator-backed non-interactive regexp replacement subset.
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn builtin_replace_regexp(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    replace_regexp_eval_impl(eval, args, false)
}

/// `(keep-lines REGEXP &optional RSTART REND INTERACTIVE)` —
/// evaluator-backed non-interactive line filtering subset.
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn builtin_keep_lines(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_min_max_args("keep-lines", &args, 1, 4)?;
    let regexp_ls = expect_sequence_string(&args[0])?;

    let (point_min, range, source_text) = {
        let buf = eval
            .buffers
            .current_buffer()
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
        let range = line_operation_region_bounds(&eval.buffers, buf, args.get(1), args.get(2))?;
        let accessible = buf.accessible_emacs_byte_region();
        let source_text = buf.buffer_substring_lisp_string_range(accessible.range());
        (accessible.start(), range, source_text)
    };
    // Issue #131: iterate the region's Emacs bytes directly — offsets are logical
    // Emacs byte offsets, so the old storage->logical mapping is the identity.
    let source = source_text.as_bytes();
    let source_multibyte = source_text.is_multibyte();
    let source_byte_len = source.len();

    let case_fold = case_fold_for_pattern(eval, regexp_ls);
    let syntax = super::builtins::search::FastStringMatchSyntax::for_current_buffer(eval);

    let rel_start = range
        .start()
        .saturating_offset_from(point_min)
        .get()
        .min(source_byte_len);
    let rel_end = range
        .end()
        .saturating_offset_from(point_min)
        .get()
        .min(source_byte_len);
    let end_pos = point_min.add_len(EmacsByteLen::new(rel_end));
    let mut rel_cursor = line_start_at_or_before(source, rel_start);
    let mut delete_ranges: Vec<EmacsByteRange> = Vec::new();

    while rel_cursor < source.len() {
        let abs_line_start = point_min.add_len(EmacsByteLen::new(rel_cursor));
        if abs_line_start >= end_pos {
            break;
        }

        let line_tail = &source[rel_cursor..];
        let line_len = match line_tail.iter().position(|&b| b == b'\n') {
            Some(idx) => idx + 1,
            None => line_tail.len(),
        };
        let rel_line_end = rel_cursor + line_len;
        let line = if source.get(rel_line_end.wrapping_sub(1)) == Some(&b'\n') {
            &source[rel_cursor..rel_line_end - 1]
        } else {
            &source[rel_cursor..rel_line_end]
        };

        let keep_line = match string_matches_regexp(
            syntax,
            &eval.obarray,
            &eval.buffers,
            line,
            source_multibyte,
            regexp_ls,
            case_fold,
        ) {
            Ok(matched) => matched,
            Err(Flow::Signal(sig)) if sig.symbol_name() == "invalid-regexp" => {
                return Ok(Value::NIL);
            }
            Err(err) => return Err(err),
        };
        if !keep_line {
            delete_ranges.push(EmacsByteRange::new(
                point_min.add_len(EmacsByteLen::new(rel_cursor)),
                point_min.add_len(EmacsByteLen::new(rel_line_end)),
            ));
        }
        rel_cursor = rel_line_end;
    }

    let current_id = eval
        .buffers
        .current_buffer_id()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    if !delete_ranges.is_empty() {
        delete_line_operation_ranges(eval, current_id, delete_ranges)?;
    }
    let _ = eval
        .buffers
        .goto_buffer_emacs_byte_pos(current_id, range.start());

    Ok(Value::NIL)
}

/// `(flush-lines REGEXP &optional RSTART REND INTERACTIVE)` —
/// evaluator-backed non-interactive line filtering subset.
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn builtin_flush_lines(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_min_max_args("flush-lines", &args, 1, 4)?;
    let regexp_ls = expect_sequence_string(&args[0])?;

    let (point_min, range, source_text) = {
        let buf = eval
            .buffers
            .current_buffer()
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
        let range = line_operation_region_bounds(&eval.buffers, buf, args.get(1), args.get(2))?;
        let accessible = buf.accessible_emacs_byte_region();
        let source_text = buf.buffer_substring_lisp_string_range(accessible.range());
        (accessible.start(), range, source_text)
    };
    // Issue #131: iterate the region's Emacs bytes directly — offsets are logical
    // Emacs byte offsets, so the old storage->logical mapping is the identity.
    let source = source_text.as_bytes();
    let source_multibyte = source_text.is_multibyte();
    let source_byte_len = source.len();

    let case_fold = case_fold_for_pattern(eval, regexp_ls);
    let syntax = super::builtins::search::FastStringMatchSyntax::for_current_buffer(eval);

    let rel_start = range
        .start()
        .saturating_offset_from(point_min)
        .get()
        .min(source_byte_len);
    let rel_end = range
        .end()
        .saturating_offset_from(point_min)
        .get()
        .min(source_byte_len);
    let end_pos = point_min.add_len(EmacsByteLen::new(rel_end));
    let mut rel_cursor = line_start_at_or_before(source, rel_start);
    let mut delete_ranges: Vec<EmacsByteRange> = Vec::new();

    while rel_cursor < source.len() {
        let abs_line_start = point_min.add_len(EmacsByteLen::new(rel_cursor));
        if abs_line_start >= end_pos {
            break;
        }

        let line_tail = &source[rel_cursor..];
        let line_len = match line_tail.iter().position(|&b| b == b'\n') {
            Some(idx) => idx + 1,
            None => line_tail.len(),
        };
        let rel_line_end = rel_cursor + line_len;
        let line = if source.get(rel_line_end.wrapping_sub(1)) == Some(&b'\n') {
            &source[rel_cursor..rel_line_end - 1]
        } else {
            &source[rel_cursor..rel_line_end]
        };

        if string_matches_regexp(
            syntax,
            &eval.obarray,
            &eval.buffers,
            line,
            source_multibyte,
            regexp_ls,
            case_fold,
        )? {
            delete_ranges.push(EmacsByteRange::new(
                point_min.add_len(EmacsByteLen::new(rel_cursor)),
                point_min.add_len(EmacsByteLen::new(rel_line_end)),
            ));
        }
        rel_cursor = rel_line_end;
    }

    let current_id = eval
        .buffers
        .current_buffer_id()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    let mut deleted_count = 0usize;
    if !delete_ranges.is_empty() {
        deleted_count = delete_line_operation_ranges(eval, current_id, delete_ranges)?;
    }
    let _ = eval
        .buffers
        .goto_buffer_emacs_byte_pos(current_id, range.start());

    Ok(Value::fixnum(deleted_count as i64))
}

/// `(how-many REGEXP &optional RSTART REND INTERACTIVE)` —
/// evaluator-backed regexp match counting subset.
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn builtin_how_many(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_min_max_args("how-many", &args, 1, 4)?;
    let regexp_ls = expect_sequence_string(&args[0])?;

    let source_text = {
        let buf = eval
            .buffers
            .current_buffer()
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
        let range = line_operation_region_bounds(&eval.buffers, buf, args.get(1), args.get(2))?;
        buf.buffer_substring_lisp_string_range(range)
    };

    if regexp_ls.is_empty() {
        return Ok(Value::fixnum(source_text.schars() as i64));
    }

    // Issue #131: match over the region's Emacs bytes with the real pattern
    // LispString, instead of the PUA-sentinel storage form.
    let case_fold = case_fold_for_pattern(eval, regexp_ls);
    Ok(Value::fixnum(count_string_regexp_matches(
        source_text.as_bytes(),
        regexp_ls,
        case_fold,
    )?))
}

/// `(count-matches REGEXP &optional START END INTERACTIVE)` —
/// evaluator-backed regexp match counting subset.
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn builtin_count_matches(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_max_args("count-matches", &args, 1, 4)?;
    let regexp_ls = expect_sequence_string(&args[0])?;

    let source_text = {
        let buf = eval
            .buffers
            .current_buffer()
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
        let range = line_operation_region_bounds(&eval.buffers, buf, args.get(1), args.get(2))?;
        buf.buffer_substring_lisp_string_range(range)
    };

    if regexp_ls.is_empty() {
        return Ok(Value::fixnum(source_text.schars() as i64));
    }

    // Issue #131: match over the region's Emacs bytes with the real pattern
    // LispString, instead of the PUA-sentinel storage form.
    let case_fold = case_fold_for_pattern(eval, regexp_ls);
    Ok(Value::fixnum(count_string_regexp_matches(
        source_text.as_bytes(),
        regexp_ls,
        case_fold,
    )?))
}

/// `(isearch-forward)` — interactive command; returns batch-mode error in
/// non-interactive contexts.
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn builtin_isearch_forward(args: Vec<Value>) -> EvalResult {
    expect_min_max_args("isearch-forward", &args, 0, 2)?;
    Err(signal(
        "error",
        vec![Value::string(
            "move-to-window-line called from unrelated buffer",
        )],
    ))
}

/// `(isearch-backward)` — interactive command; returns batch-mode error in
/// non-interactive contexts.
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn builtin_isearch_backward(args: Vec<Value>) -> EvalResult {
    expect_min_max_args("isearch-backward", &args, 0, 2)?;
    Err(signal(
        "error",
        vec![Value::string(
            "move-to-window-line called from unrelated buffer",
        )],
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
#[path = "isearch_test.rs"]
mod tests;

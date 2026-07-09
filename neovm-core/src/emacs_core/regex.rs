//! Regex engine and search primitives for the Elisp VM.
//!
//! Uses a direct translation of GNU Emacs's `regex-emacs.c` as the backend.
//! All pattern compilation, matching, and searching goes through the
//! `regex_emacs` module, ensuring 100% semantic compatibility with GNU.
//!
//! # Audit-tracked boundaries vs GNU
//!
//! The audit at `drafts/regex-search-audit.md` tracks divergences
//! from GNU `src/search.c`. Audit findings 1, 2, 3, 4, 7, 8, 9, 10,
//! 11, 12, 14, 16, and 20 have been addressed. The remaining
//! intentionally-deferred items are:
//!
//! - **#5** (translate table byte-only) — full Unicode case-canon
//!   table refactor; covered by a doc comment in `regex_compile`.
//! - **#13** (replace-match multibyte/unibyte) — still routed
//!   through a storage-string compatibility seam instead of a
//!   direct GNU-style byte conversion loop; documented inline.
//! - **#15** (cache key narrow) — extra cache axes (syntax table
//!   identity, multibyte flag) are placeholders for features
//!   neomacs does not have yet; documented inline.
//! - **#17** (gap-aware `re_match_2`) — perf, not correctness.
//!   Each search currently materializes the buffer text via
//!   `Buffer::buffer_substring_range(Buffer::accessible_emacs_byte_range())`
//!   instead of walking across the gap boundary. GNU `regex-emacs.c:re_match_2` would save
//!   a buffer-sized allocation per search. Audit Phase D Task 4.1.
//! - **#18** (boyer_moore literal search) — perf, not correctness.
//!   `literal_find` uses naive `str::find` instead of GNU's
//!   Boyer-Moore-with-skip-table from `src/search.c:1761+`. Audit
//!   Phase D Task 4.2.
//! - **#19** (internal C helpers) — `save_search_regs`,
//!   `update_search_regs`, `freeze_pattern` / `unfreeze_pattern`,
//!   `find_newline1`, and `wordify` are GNU-internal helpers used
//!   on the `running_asynch_code` path and during async-signal
//!   handling. neomacs does not run user signal handlers in C
//!   code or expose `running_asynch_code`, so these helpers have
//!   no consumers. `wordify` (`\bword\b` from a literal word) is
//!   already implemented in elisp via the `wordify` function
//!   defined in `subr.el`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::buffer::{Buffer, BufferId, CharLen, CharPos0, CharRange, EmacsBytePos, EmacsByteRange};
use crate::emacs_core::casefiddle::apply_replace_match_case;
use crate::emacs_core::regex_emacs::{
    self, BufferSyntaxLookup, CaseTranslation, CompiledPattern, DefaultSyntaxLookup,
    MatchRegisters, SyntaxCacheKey, SyntaxLookup,
};
use crate::heap_types::LispString;

pub(crate) const REPLACE_MATCH_SUBEXP_MISSING: &str = "replace-match subexpression does not exist";
const GNU_SEARCH_REGS_BASE_CAPACITY: usize = 7;
const SEARCH_PATTERN_CACHE_SIZE: usize = 20;
// GNU's `\=` assertion compares against the current buffer's PT_BYTE even
// during `string-match` (`regex-emacs.c:5201`). A standalone string has no
// matching buffer byte address, so pass an unreachable point to the translated
// matcher instead of treating the START argument as point.
const STRING_MATCH_AT_DOT_UNREACHABLE: usize = usize::MAX;

fn buffer_syntax_lookup(buf: &Buffer) -> BufferSyntaxLookup {
    let category_table = crate::emacs_core::category::active_category_table_for_buffer(Some(buf))
        .ok()
        .filter(|table| !table.is_nil());
    BufferSyntaxLookup {
        syntax_table: crate::emacs_core::syntax::SyntaxTable::for_buffer(buf),
        category_table,
    }
}

#[inline]
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn buffer_lisp_match_char_pos_to_byte_pos(buf: &Buffer, lisp_char_pos: usize) -> EmacsBytePos {
    buf.char_pos_to_emacs_byte_pos_clamped(
        CharPos0::new(lisp_char_pos).saturating_sub_len(CharLen::new(1)),
    )
}

#[inline]
fn lisp_char_pos_to_zero_based_index(lisp_char_pos: usize) -> usize {
    CharPos0::new(lisp_char_pos)
        .saturating_sub_len(CharLen::new(1))
        .get()
}

/// Convert `MatchRegisters` (from the GNU-translated engine) into `MatchData`
/// (the public representation used by Elisp builtins).
fn match_data_from_registers(regs: &MatchRegisters, offset: usize) -> MatchData {
    let num_groups = regs.num_regs();
    let mut groups = Vec::with_capacity(gnu_search_regs_capacity(num_groups));
    for i in 0..num_groups {
        if regs.start[i] >= 0 && regs.end[i] >= 0 {
            groups.push(Some(MatchGroup::new(
                regs.start[i] as usize + offset,
                regs.end[i] as usize + offset,
            )));
        } else {
            groups.push(None);
        }
    }
    extend_to_gnu_search_regs_capacity(&mut groups);
    MatchData {
        groups,
        source: MatchSource::None,
    }
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn storage_rel_to_emacs_byte(text: &str, base_emacs_byte: usize, storage_pos: usize) -> usize {
    base_emacs_byte
        + crate::emacs_core::string_escape::storage_byte_to_logical_byte(text, storage_pos)
}

fn buffer_match_data_from_registers(regs: &MatchRegisters, base_emacs_byte: usize) -> MatchData {
    let num_groups = regs.num_regs();
    let mut groups = Vec::with_capacity(gnu_search_regs_capacity(num_groups));
    for i in 0..num_groups {
        if regs.start[i] >= 0 && regs.end[i] >= 0 {
            groups.push(Some(MatchGroup::new(
                base_emacs_byte + regs.start[i] as usize,
                base_emacs_byte + regs.end[i] as usize,
            )));
        } else {
            groups.push(None);
        }
    }
    extend_to_gnu_search_regs_capacity(&mut groups);
    MatchData {
        groups,
        source: MatchSource::Buffer {
            id: None,
            positions: BufferMatchPositions::EmacsBytes,
        },
    }
}

fn gnu_search_regs_capacity(required: usize) -> usize {
    if required <= GNU_SEARCH_REGS_BASE_CAPACITY {
        GNU_SEARCH_REGS_BASE_CAPACITY
    } else {
        required.max(GNU_SEARCH_REGS_BASE_CAPACITY + (GNU_SEARCH_REGS_BASE_CAPACITY >> 1))
    }
}

fn extend_to_gnu_search_regs_capacity(groups: &mut Vec<Option<MatchGroup>>) {
    groups.resize(gnu_search_regs_capacity(groups.len()), None);
}

fn gnu_single_group_vec(group: Option<MatchGroup>) -> Vec<Option<MatchGroup>> {
    let mut groups = vec![group];
    extend_to_gnu_search_regs_capacity(&mut groups);
    groups
}

#[derive(Clone)]
enum CompiledSearchPattern {
    /// GNU-translated engine (primary path for all patterns).
    Emacs(Rc<CompiledPattern>),
    /// Simple literal search. Holds the literal as Emacs internal-encoding bytes
    /// (issue #131: no storage round-trip).
    Literal(Vec<u8>),
}

pub(crate) struct IteratedStringMatches {
    #[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
    pub capture_count: usize,
    pub matches: Vec<Vec<Option<MatchGroup>>>,
}

/// Entry of [`SEARCH_PATTERN_CACHE`]:
/// `(posix, case_fold, pattern_multibyte, pattern_bytes, syntax_key, compiled)`.
type SearchPatternCacheEntry = (
    bool,
    bool,
    bool,
    Vec<u8>,
    Option<SyntaxCacheKey>,
    CompiledSearchPattern,
);

/// Entry of [`LISP_REGEX_PATTERN_CACHE`]:
/// `(posix, case_fold, translation_key, pattern_multibyte, target_multibyte,
///   pattern_bytes, syntax_key, compiled)`.
type LispRegexPatternCacheEntry = (
    bool,
    bool,
    Option<usize>,
    bool,
    bool,
    Vec<u8>,
    Option<SyntaxCacheKey>,
    Rc<CompiledPattern>,
);

/// Does a cached entry compiled under `stored` serve a request running
/// under `current`?
///
/// Mirrors GNU `compile_pattern`'s probe (search.c:222-224):
/// `EQ (cp->syntax_table, Qt) || EQ (cp->syntax_table, BVAR
/// (current_buffer, syntax_table))` — `None` is GNU's `Qt`
/// (table-independent pattern), and the epoch inside
/// [`SyntaxCacheKey::Table`] stands in for GNU's `clear_regexp_cache`
/// on `modify-syntax-entry`.
fn syntax_key_matches(stored: Option<SyntaxCacheKey>, current: SyntaxCacheKey) -> bool {
    match stored {
        None => true,
        Some(key) => key == current,
    }
}

thread_local! {
    // GNU `src/search.c:61` (`searchbuf_head`) uses a `regexp_cache`
    // record keyed on:
    //
    //   - the pattern Lisp string,
    //   - the buffer's syntax table for `used_syntax` patterns (the
    //     `[[:word:]]` / `[[:space:]]` classes bake table content into
    //     the compiled artifacts — for neomacs, into the fastmap),
    //   - the `whitespace-regexp` transform flag,
    //   - the `posix` flag,
    //   - the `multibyte` flag,
    //   - the translate table identity.
    //
    // Neomacs tracks (posix, case_fold[/translate identity], pattern
    // multibyteness, pattern bytes, syntax-table key). Remaining
    // intentional gaps vs GNU:
    //
    //   - We don't expose `whitespace-regexp`.
    //   - `charset_unibyte` has no neomacs analog (UTF-8 internal).
    static SEARCH_PATTERN_CACHE: RefCell<Vec<SearchPatternCacheEntry>> =
        const { RefCell::new(Vec::new()) };

    static LISP_REGEX_PATTERN_CACHE: RefCell<Vec<LispRegexPatternCacheEntry>> =
        const { RefCell::new(Vec::new()) };
}

// ---------------------------------------------------------------------------
// MatchData
// ---------------------------------------------------------------------------

/// Match data from the last successful search.
#[derive(Clone, Debug)]
pub struct MatchData {
    /// Full match and capture groups in GNU register order.
    ///
    /// The stored numeric coordinate depends on the match source:
    ///
    /// - string matches store zero-based character positions, as GNU
    ///   `string-match` does after `string_byte_to_char`;
    /// - engine-produced buffer matches store zero-based Emacs byte positions
    ///   until a Lisp-facing builtin converts them to buffer positions;
    /// - `set-match-data` buffer restores store Lisp buffer character
    ///   positions, matching GNU's `search_regs` after restore.
    ///
    /// Callers should use `MatchData`/`MatchGroup` accessors instead of
    /// interpreting the raw pair directly.
    /// Index 0 = full match, 1+ = capture groups.
    pub groups: Vec<Option<MatchGroup>>,
    /// Provenance and coordinate system for `groups`.
    pub source: MatchSource,
}

#[derive(Clone, Debug)]
pub enum MatchSource {
    None,
    /// GNU uses `last_thing_searched = Qt` for string match data.  A searched
    /// string object is available after `string-match`, but not after
    /// `set-match-data` restores an integer list saved by `match-data`.
    String {
        searched: Option<SearchedString>,
    },
    /// Buffer matches can be engine-produced byte positions or GNU-restored
    /// Lisp character positions. Keep that distinction explicit.
    Buffer {
        id: Option<BufferId>,
        positions: BufferMatchPositions,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferMatchPositions {
    EmacsBytes,
    LispChars,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchGroup {
    start: usize,
    end: usize,
}

impl MatchGroup {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub const fn start(self) -> usize {
        self.start
    }

    pub const fn end(self) -> usize {
        self.end
    }

    pub const fn string_char_range(self) -> CharRange {
        CharRange::new(CharPos0::new(self.start), CharPos0::new(self.end))
    }

    pub const fn emacs_byte_range(self) -> EmacsByteRange {
        EmacsByteRange::new(EmacsBytePos::new(self.start), EmacsBytePos::new(self.end))
    }

    pub fn shift(self, delta: usize) -> Self {
        Self::new(self.start + delta, self.end + delta)
    }

    pub fn saturating_sub(self, delta: usize) -> Self {
        Self::new(
            self.start.saturating_sub(delta),
            self.end.saturating_sub(delta),
        )
    }

    pub fn translate_saturating(self, delta: i64) -> Self {
        if delta >= 0 {
            let delta = delta as usize;
            Self::new(
                self.start.saturating_add(delta),
                self.end.saturating_add(delta),
            )
        } else {
            let delta = (-delta) as usize;
            Self::new(
                self.start.saturating_sub(delta),
                self.end.saturating_sub(delta),
            )
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchedString {
    Heap(super::value::Value),
    Owned(LispString),
}

impl SearchedString {
    pub(crate) fn as_lisp_string(&self) -> Option<&LispString> {
        match self {
            Self::Heap(val) => val.as_lisp_string(),
            Self::Owned(text) => Some(text),
        }
    }

    fn byte_to_char_pos(&self, byte_pos: usize) -> usize {
        let Some(string) = self.as_lisp_string() else {
            return 0;
        };
        if string.is_multibyte() {
            crate::emacs_core::emacs_char::byte_to_char_pos(string.as_bytes(), byte_pos)
        } else {
            byte_pos.min(string.byte_len())
        }
    }

    #[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
    pub(crate) fn to_owned(&self) -> String {
        let Some(string) = self.as_lisp_string() else {
            return String::new();
        };
        string
            .as_utf8_str()
            .map(str::to_owned)
            .unwrap_or_else(|| String::from_utf8_lossy(string.as_bytes()).into_owned())
    }
}

pub fn char_pos_to_byte_lisp_string(s: &crate::heap_types::LispString, char_pos: usize) -> usize {
    if !s.is_multibyte() {
        return char_pos.min(s.byte_len());
    }
    if char_pos >= s.schars() {
        return s.byte_len();
    }
    crate::emacs_core::emacs_char::char_to_byte_pos(s.as_bytes(), char_pos)
}

impl MatchData {
    pub(crate) fn none(groups: Vec<Option<MatchGroup>>) -> Self {
        Self {
            groups,
            source: MatchSource::None,
        }
    }

    pub(crate) fn string(
        groups: Vec<Option<MatchGroup>>,
        searched: Option<SearchedString>,
    ) -> Self {
        Self {
            groups,
            source: MatchSource::String { searched },
        }
    }

    pub(crate) fn buffer_bytes(
        groups: Vec<Option<MatchGroup>>,
        buffer_id: Option<BufferId>,
    ) -> Self {
        Self {
            groups,
            source: MatchSource::Buffer {
                id: buffer_id,
                positions: BufferMatchPositions::EmacsBytes,
            },
        }
    }

    pub(crate) fn buffer_lisp_chars(
        groups: Vec<Option<MatchGroup>>,
        buffer_id: Option<BufferId>,
    ) -> Self {
        Self {
            groups,
            source: MatchSource::Buffer {
                id: buffer_id,
                positions: BufferMatchPositions::LispChars,
            },
        }
    }

    pub(crate) fn is_string_match(&self) -> bool {
        matches!(self.source, MatchSource::String { .. })
    }

    pub(crate) fn searched_string(&self) -> Option<&SearchedString> {
        match &self.source {
            MatchSource::String { searched } => searched.as_ref(),
            _ => None,
        }
    }

    pub(crate) fn searched_buffer_id(&self) -> Option<BufferId> {
        match self.source {
            MatchSource::Buffer { id, .. } => id,
            _ => None,
        }
    }

    pub(crate) fn set_buffer_id(&mut self, buffer_id: BufferId) {
        if let MatchSource::Buffer { id, .. } = &mut self.source {
            *id = Some(buffer_id);
        }
    }

    pub(crate) fn set_string_source(&mut self, searched: Option<SearchedString>) {
        self.source = MatchSource::String { searched };
    }

    pub(crate) fn uses_buffer_byte_positions(&self) -> bool {
        matches!(
            self.source,
            MatchSource::Buffer {
                positions: BufferMatchPositions::EmacsBytes,
                ..
            }
        )
    }

    pub(crate) fn uses_buffer_lisp_char_positions(&self) -> bool {
        matches!(
            self.source,
            MatchSource::Buffer {
                positions: BufferMatchPositions::LispChars,
                ..
            }
        )
    }

    #[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
    pub(crate) fn searched_string_text(&self) -> Option<String> {
        self.searched_string().map(SearchedString::to_owned)
    }
}

// ---------------------------------------------------------------------------
// Emacs → Rust regex translation
// ---------------------------------------------------------------------------

/// Translate basic Emacs regex syntax to Rust regex syntax.
///
/// Key differences handled:
/// - Emacs `\(` `\)` for groups  →  Rust `(` `)`
/// - Emacs `\|` for alternation  →  Rust `|`
/// - Emacs `\{` `\}` for repetition  →  Rust `{` `}`
/// - Emacs `\1`..`\9` for back-references  →  not supported by `regex` crate,
///   but we translate the syntax anyway for completeness
/// - Emacs literal `(` `)` `{` `}` `|`  →  Rust `\(` `\)` `\{` `\}` `\|`
/// - Emacs `\w` (word char)  →  Rust `\w`
/// - Emacs `\W` (non-word char)  →  Rust `\W`
/// - Emacs `\b` (word boundary)  →  Rust `\b`
/// - Emacs `\B` (non-word boundary)  →  Rust `\B`
/// - Emacs `\s-` etc. (syntax classes)  →  simplified to `\s` (whitespace)
/// - Emacs `\<` `\>` (word boundaries)  →  Rust `\b`
/// - Emacs character classes inside `[...]` are kept as-is.
pub fn translate_emacs_regex(pattern: &str) -> String {
    fn next_char_at(s: &str, byte_idx: usize) -> Option<(char, usize)> {
        s.get(byte_idx..)
            .and_then(|tail| tail.chars().next().map(|ch| (ch, ch.len_utf8())))
    }

    fn push_rust_class_char(out: &mut String, ch: char) {
        match ch {
            '\\' => out.push_str("\\\\"),
            '[' => out.push_str("\\["),
            _ => out.push(ch),
        }
    }

    let mut out = String::with_capacity(pattern.len() + 8);
    let bytes = pattern.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut in_bracket = false;
    let mut bracket_negated = false;
    // Position in `out` where bracket content starts (after `[` / `[^`).
    // Used to detect empty classes after removing reversed ranges.
    let mut bracket_content_start: usize = 0;

    while i < len {
        let (ch, ch_len) = next_char_at(pattern, i).expect("byte index must be char boundary");

        // Non-ASCII literal bytes should be preserved as full UTF-8 scalar values.
        if !ch.is_ascii() {
            out.push(ch);
            i += ch_len;
            continue;
        }

        // Inside a character class [...], handle Emacs/Rust differences:
        //  - `\` is literal in Emacs and can still participate in ranges
        //  - Reversed ranges like `z-a` are empty in Emacs but error in Rust → remove
        //  - `]` at first position is literal in Emacs → escape it for Rust
        if in_bracket {
            if ch == ']' {
                in_bracket = false;
                if out.len() == bracket_content_start {
                    // Bracket has no content (all ranges were reversed/removed).
                    // [^] → matches anything, [] → matches nothing.
                    // Truncate the opening `[` or `[^` and emit a replacement.
                    let open_len = if bracket_negated { 2 } else { 1 };
                    out.truncate(bracket_content_start - open_len);
                    if bracket_negated {
                        out.push_str("[\\s\\S]");
                    } else {
                        // Empty positive class — can never match.
                        // Use a character class that accepts no character to
                        // avoid unsupported look-around constructs.
                        out.push_str("[^\\s\\S]");
                    }
                } else {
                    out.push(']');
                }
                i += 1;
                continue;
            }
            if ch == '\\' {
                if i + 1 < len && bytes[i + 1] == b']' {
                    // GNU Emacs does not treat \] inside [...] as a literal ].
                    // Keep the backslash as a literal class member and let the
                    // following ] close the class on the next iteration.
                    push_rust_class_char(&mut out, ch);
                    i += 1;
                    continue;
                }
                if i + 2 < len && bytes[i + 1] == b'-' && bytes[i + 2] != b']' {
                    let (end_ch, end_len) =
                        next_char_at(pattern, i + 2).expect("byte index must be char boundary");
                    if ch > end_ch {
                        // GNU Emacs treats `\-x` like a range from `\` to `x`.
                        // If the range is reversed, it is empty.
                        i += 1 + 1 + end_len;
                        continue;
                    }
                    push_rust_class_char(&mut out, ch);
                    out.push('-');
                    push_rust_class_char(&mut out, end_ch);
                    i += 1 + 1 + end_len;
                } else {
                    push_rust_class_char(&mut out, ch);
                    i += 1;
                }
                continue;
            }
            if ch == '[' {
                // In Emacs, `[` inside [...] is literal.  In Rust regex
                // it starts a nested character class.  Escape it.
                // Exception: POSIX classes like [:alpha:] — pass through.
                if i + 1 < len && bytes[i + 1] == b':' {
                    // Looks like a POSIX class `[:...:` — pass through.
                    out.push('[');
                } else {
                    out.push_str("\\[");
                }
                i += 1;
                continue;
            }
            // Check for ranges: if next is `-` and then a non-`]` char,
            // validate the range direction.
            if i + 2 < len && bytes[i + 1] == b'-' && bytes[i + 2] != b']' {
                let (end_ch, end_len) =
                    next_char_at(pattern, i + 2).expect("byte index must be char boundary");
                if ch > end_ch {
                    // Reversed range (e.g. z-a): empty in Emacs, skip entirely.
                    i += 1 + 1 + end_len;
                    continue;
                }
            }
            out.push(ch);
            i += ch_len;
            continue;
        }

        match ch {
            '[' => {
                in_bracket = true;
                bracket_negated = false;
                out.push('[');
                i += 1;
                // Handle `[^` — consume the negation prefix.
                if i < len && bytes[i] == b'^' {
                    out.push('^');
                    bracket_negated = true;
                    i += 1;
                }
                bracket_content_start = out.len();
                // `]` as first char (or first after `^`) is literal in Emacs.
                // In Rust regex it would close the class.  Escape it.
                if i < len && bytes[i] == b']' {
                    out.push_str("\\]");
                    i += 1;
                }
            }
            // Emacs uses literal `(`, `)`, `{`, `}`, `|` — escape them for Rust regex.
            '(' => {
                out.push_str("\\(");
                i += 1;
            }
            ')' => {
                out.push_str("\\)");
                i += 1;
            }
            '{' => {
                out.push_str("\\{");
                i += 1;
            }
            '}' => {
                out.push_str("\\}");
                i += 1;
            }
            '|' => {
                out.push_str("\\|");
                i += 1;
            }
            '\\' if i + 1 < len => {
                let (next, next_len) =
                    next_char_at(pattern, i + 1).expect("byte index must be char boundary");
                match next {
                    // Emacs group → Rust group
                    '(' => {
                        let group_idx = i + 1 + next_len;
                        if group_idx < len && bytes[group_idx] == b'?' {
                            if group_idx + 1 < len && bytes[group_idx + 1] == b':' {
                                out.push_str("(?:");
                                i = group_idx + 2;
                                continue;
                            }

                            let digits_start = group_idx + 1;
                            let mut digits_end = digits_start;
                            while digits_end < len && bytes[digits_end].is_ascii_digit() {
                                digits_end += 1;
                            }
                            if digits_end > digits_start
                                && digits_end < len
                                && bytes[digits_end] == b':'
                            {
                                out.push('(');
                                i = digits_end + 1;
                                continue;
                            }
                        }

                        out.push('(');
                        i += 1 + next_len;
                    }
                    ')' => {
                        out.push(')');
                        i += 1 + next_len;
                    }
                    // Emacs alternation → Rust alternation
                    '|' => {
                        out.push('|');
                        i += 1 + next_len;
                    }
                    // Emacs repetition braces → Rust repetition braces
                    '{' => {
                        let interval_start = i + 1 + next_len;
                        let mut scan = interval_start;
                        let mut closed_interval = false;
                        while scan < len {
                            if bytes[scan] == b'\\' && scan + 1 < len && bytes[scan + 1] == b'}' {
                                let interval = &pattern[interval_start..scan];
                                out.push('{');
                                if let Some(rest) = interval.strip_prefix(',') {
                                    out.push('0');
                                    out.push(',');
                                    out.push_str(rest);
                                } else {
                                    out.push_str(interval);
                                }
                                out.push('}');
                                i = scan + 2;
                                closed_interval = true;
                                break;
                            }
                            scan += 1;
                        }
                        if closed_interval {
                            continue;
                        }
                        out.push('{');
                        i += 1 + next_len;
                    }
                    '}' => {
                        out.push('}');
                        i += 1 + next_len;
                    }
                    // GNU regex.c: \` matches beginning of string (like \A in PCRE)
                    '`' => {
                        out.push_str("\\A");
                        i += 1 + next_len;
                    }
                    // GNU regex.c: \' matches end of string (like \z in PCRE)
                    '\'' => {
                        out.push_str("\\z");
                        i += 1 + next_len;
                    }
                    // Word boundaries
                    '<' => {
                        out.push_str("\\b");
                        i += 1 + next_len;
                    }
                    '>' => {
                        out.push_str("\\b");
                        i += 1 + next_len;
                    }
                    '_' => {
                        i += 1 + next_len;
                        if i < len {
                            let (boundary_ch, boundary_len) =
                                next_char_at(pattern, i).expect("byte index must be char boundary");
                            match boundary_ch {
                                '<' | '>' => {
                                    i += boundary_len;
                                    out.push_str("\\b");
                                }
                                _ => {
                                    out.push('_');
                                }
                            }
                        } else {
                            out.push('_');
                        }
                    }
                    // Back-references (1-9) — not supported by `regex` crate,
                    // but translate the syntax for pattern acceptance.
                    '1'..='9' => {
                        // Rust `regex` doesn't support back-refs; drop silently.
                        // In practice, patterns using \1..\9 will fail to compile
                        // which is acceptable for now.
                        out.push('\\');
                        out.push(next);
                        i += 1 + next_len;
                    }
                    // Emacs syntax classes (\s-, \sw, etc.)
                    // Map to the closest Rust regex equivalents.
                    's' => {
                        i += 1 + next_len;
                        // Consume the syntax-class character and map appropriately
                        if i < len {
                            let (class_ch, class_len) =
                                next_char_at(pattern, i).expect("byte index must be char boundary");
                            match class_ch {
                                '-' | ' ' => {
                                    // \s- or \s  → whitespace
                                    i += class_len;
                                    out.push_str("\\s");
                                }
                                'w' => {
                                    // \sw → word constituent
                                    i += class_len;
                                    out.push_str("\\w");
                                }
                                '_' => {
                                    // \s_ → symbol constituent (word + underscore)
                                    i += class_len;
                                    out.push_str("[\\w_]");
                                }
                                '.' => {
                                    // \s. → punctuation
                                    i += class_len;
                                    out.push_str("[[:punct:]]");
                                }
                                '(' => {
                                    // \s( → open delimiter
                                    i += class_len;
                                    out.push_str("[\\[\\(\\{]");
                                }
                                ')' => {
                                    // \s) → close delimiter
                                    i += class_len;
                                    out.push_str("[\\]\\)\\}]");
                                }
                                '"' => {
                                    // \s" → string quote character
                                    i += class_len;
                                    out.push_str("[\"']");
                                }
                                '\'' | '<' | '>' | '!' | '|' | '/' => {
                                    // Other syntax classes — approximate as whitespace
                                    i += class_len;
                                    out.push_str("\\s");
                                }
                                _ => {
                                    // No valid syntax-class char follows; treat as bare \s
                                    out.push_str("\\s");
                                }
                            }
                        } else {
                            out.push_str("\\s");
                        }
                    }
                    'S' => {
                        i += 1 + next_len;
                        // Consume the syntax-class character and map appropriately
                        if i < len {
                            let (class_ch, class_len) =
                                next_char_at(pattern, i).expect("byte index must be char boundary");
                            match class_ch {
                                '-' | ' ' => {
                                    // \S- or \S  → non-whitespace
                                    i += class_len;
                                    out.push_str("\\S");
                                }
                                'w' => {
                                    // \Sw → non-word constituent
                                    i += class_len;
                                    out.push_str("\\W");
                                }
                                '_' => {
                                    // \S_ → non-symbol constituent
                                    i += class_len;
                                    out.push_str("[^\\w_]");
                                }
                                '.' => {
                                    // \S. → non-punctuation
                                    i += class_len;
                                    out.push_str("[^[:punct:]]");
                                }
                                '(' => {
                                    // \S( → non-open-delimiter
                                    i += class_len;
                                    out.push_str("[^\\[\\(\\{]");
                                }
                                ')' => {
                                    // \S) → non-close-delimiter
                                    i += class_len;
                                    out.push_str("[^\\]\\)\\}]");
                                }
                                '"' => {
                                    // \S" → non-string-quote
                                    i += class_len;
                                    out.push_str("[^\"']");
                                }
                                '\'' | '<' | '>' | '!' | '|' | '/' => {
                                    // Other syntax classes — approximate as non-whitespace
                                    i += class_len;
                                    out.push_str("\\S");
                                }
                                _ => {
                                    // No valid syntax-class char follows; treat as bare \S
                                    out.push_str("\\S");
                                }
                            }
                        } else {
                            out.push_str("\\S");
                        }
                    }
                    'c' => {
                        i += 1 + next_len;
                        if i < len {
                            let (_, class_len) =
                                next_char_at(pattern, i).expect("byte index must be char boundary");
                            i += class_len;
                        }
                        // GNU Emacs category regexps are implemented in C and depend on
                        // the active category table. Rust's `regex` backend has no
                        // equivalent dynamic character-category predicate, so approximate
                        // category escapes as non-ASCII until the native engine is ported.
                        out.push_str("[^\\x00-\\x7F]");
                    }
                    // \= (match at point) → \A (match at start of search region)
                    '=' => {
                        out.push_str("\\A");
                        i += 1 + next_len;
                    }
                    // Known escape sequences — pass through
                    'w' | 'W' | 'b' | 'B' | 'd' | 'D' | 'n' | 't' | 'r' => match next {
                        _ => {
                            out.push('\\');
                            out.push(next);
                            i += 1 + next_len;
                        }
                    },
                    // Literal backslash
                    '\\' => {
                        out.push_str("\\\\");
                        i += 1 + next_len;
                    }
                    // Anything else after `\` — pass through the escape
                    _ => {
                        if next.is_ascii() {
                            out.push('\\');
                        }
                        out.push(next);
                        i += 1 + next_len;
                    }
                }
            }
            // Lone trailing backslash — pass through
            '\\' => {
                out.push('\\');
                i += 1;
            }
            // All other chars — pass through as-is
            _ => {
                out.push(ch);
                i += 1;
            }
        }
    }

    out
}

fn trivial_regexp_p(pattern: &[u8]) -> bool {
    // Issue #131: pattern is Emacs internal-encoding bytes; every regex
    // metacharacter is ASCII (< 0x80) and UTF-8 / eight-bit bytes are >= 0x80,
    // so scanning raw bytes never false-matches a metacharacter.
    let mut i = 0;
    while i < pattern.len() {
        match pattern[i] {
            b'.' | b'*' | b'+' | b'?' | b'[' | b'^' | b'$' => return false,
            b'\\' => {
                i += 1;
                let Some(&next) = pattern.get(i) else {
                    return false;
                };
                match next {
                    b'|' | b'(' | b')' | b'`' | b'\'' | b'b' | b'B' | b'<' | b'>' | b'w' | b'W'
                    | b's' | b'S' | b'=' | b'{' | b'}' | b'_' | b'c' | b'C' | b'1' | b'2'
                    | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9' | b'n' | b't' | b'r' => {
                        return false;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        i += 1;
    }
    true
}

fn literal_from_trivial_regexp(pattern: &[u8]) -> Option<Vec<u8>> {
    if !trivial_regexp_p(pattern) {
        return None;
    }

    let mut out = Vec::with_capacity(pattern.len());
    let mut pos = 0;
    while pos < pattern.len() {
        let (code, len) = crate::emacs_core::emacs_char::string_char(&pattern[pos..]);
        if code == '\\' as u32 {
            pos += len;
            if pos >= pattern.len() {
                return None;
            }
            let (_next, next_len) = crate::emacs_core::emacs_char::string_char(&pattern[pos..]);
            out.extend_from_slice(&pattern[pos..pos + next_len]);
            pos += next_len;
        } else {
            out.extend_from_slice(&pattern[pos..pos + len]);
            pos += len;
        }
    }
    Some(out)
}

/// Issue #131: build the multibyte `LispString` to compile a search pattern from
/// (GNU/the old `from_utf8` path always compiled multibyte; a unibyte pattern
/// promotes its raw bytes to eight-bit characters).
fn pattern_for_compile(pattern: &LispString) -> LispString {
    if pattern.is_multibyte() {
        LispString::from_emacs_bytes(pattern.as_bytes().to_vec())
    } else {
        LispString::from_emacs_bytes(crate::emacs_core::emacs_char::str_to_multibyte(
            pattern.as_bytes(),
        ))
    }
}

fn compile_search_pattern(
    pattern: &LispString,
    case_fold: bool,
) -> Result<CompiledSearchPattern, String> {
    compile_search_pattern_with_posix(pattern, case_fold, false, &DefaultSyntaxLookup)
}

/// Compile PATTERN for a `posix-*` search builtin.
///
/// GNU's `posix-looking-at`, `posix-search-forward`, `posix-search-backward`,
/// and `posix-string-match` all pass `posix=1` to the underlying
/// `looking_at_1` / `search_buffer` / `string_match_1` helpers
/// (see GNU `src/search.c:Fposix_looking_at` etc.). That flag is then
/// threaded through `compile_pattern` into `regex_compile` and
/// ultimately into `re_match_2_internal`, where the POSIX longest-
/// match algorithm (regex-emacs.c:4143-4344, 5278) kicks in.
///
/// Neomacs's `compile_search_pattern` used to hardcode `posix=false`
/// on the call to `regex_emacs::regex_compile`, which is audit
/// finding #2 in `drafts/regex-search-audit.md`. Callers that want
/// POSIX semantics must go through this helper. The pattern cache is
/// keyed on `(posix, case_fold, pattern)` so a non-POSIX entry never
/// satisfies a POSIX request or vice versa.
fn compile_search_pattern_with_posix(
    pattern: &LispString,
    case_fold: bool,
    posix: bool,
    syntax: &dyn SyntaxLookup,
) -> Result<CompiledSearchPattern, String> {
    let syntax_key = syntax.cache_key();
    if let Some(cached) = crate::emacs_core::perf_trace::time_op(
        crate::emacs_core::perf_trace::HotpathOp::RegexCompileHit,
        || {
            SEARCH_PATTERN_CACHE.with(|cache| {
                let mut cache = cache.borrow_mut();
                let index = cache.iter().position(
                    |(
                        cached_posix,
                        cached_case_fold,
                        cached_multibyte,
                        cached_pattern,
                        cached_syntax_key,
                        _,
                    )| {
                        *cached_posix == posix
                            && *cached_case_fold == case_fold
                            && *cached_multibyte == pattern.is_multibyte()
                            && cached_pattern.as_slice() == pattern.as_bytes()
                            && syntax_key_matches(*cached_syntax_key, syntax_key)
                    },
                )?;
                let entry = cache.remove(index);
                cache.insert(0, entry.clone());
                Some(entry.5)
            })
        },
    ) {
        return Ok(cached);
    }

    let compiled = crate::emacs_core::perf_trace::time_op(
        crate::emacs_core::perf_trace::HotpathOp::RegexCompileMiss,
        || {
            // Use the GNU-translated engine for all patterns.
            // Only optimize plain literals (no regex metacharacters).
            // A trivial literal is unaffected by POSIX vs non-POSIX
            // semantics because there is nothing to backtrack over,
            // so we can keep the Literal fast-path even when posix
            // is requested.
            if let Some(literal) = literal_from_trivial_regexp(pattern.as_bytes())
                && (!case_fold || literal.is_ascii())
            {
                Ok(CompiledSearchPattern::Literal(literal))
            } else {
                regex_emacs::regex_compile_lisp(pattern, posix, case_fold)
                    .map_err(|e| e.message)
                    .map(|mut cp| {
                        // `used_syntax`: the fastmap bakes ASCII
                        // membership of `[[:word:]]`/`[[:space:]]`.
                        // `regex_compile_lisp` baked the standard
                        // mapping; rebake against the active table
                        // (GNU bakes the buffer table directly at
                        // compile time, regex-emacs.c:2081-2092).
                        if cp.used_syntax && syntax_key != SyntaxCacheKey::Standard {
                            regex_emacs::recompute_fastmap(&mut cp, syntax);
                        }
                        CompiledSearchPattern::Emacs(Rc::new(cp))
                    })
            }
        },
    )?;

    // GNU `compile_pattern_1`: `cp->syntax_table = cp->buf.used_syntax
    // ? BVAR (current_buffer, syntax_table) : Qt;` — only patterns whose
    // compiled artifacts hardcode table content get the syntax axis.
    let entry_syntax_key = match &compiled {
        CompiledSearchPattern::Emacs(cp) if cp.used_syntax => Some(syntax_key),
        _ => None,
    };

    SEARCH_PATTERN_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.insert(
            0,
            (
                posix,
                case_fold,
                pattern.is_multibyte(),
                pattern.as_bytes().to_vec(),
                entry_syntax_key,
                compiled.clone(),
            ),
        );
        if cache.len() > SEARCH_PATTERN_CACHE_SIZE {
            cache.truncate(SEARCH_PATTERN_CACHE_SIZE);
        }
    });

    Ok(compiled)
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn compile_lisp_pattern_with_posix(
    pattern: &LispString,
    case_fold: bool,
    posix: bool,
    target_multibyte: bool,
) -> Result<Rc<CompiledPattern>, String> {
    compile_lisp_pattern_with_posix_translation(
        pattern,
        case_fold,
        posix,
        target_multibyte,
        None,
        &DefaultSyntaxLookup,
    )
}

fn compile_lisp_pattern_with_posix_translation(
    pattern: &LispString,
    case_fold: bool,
    posix: bool,
    target_multibyte: bool,
    translation: Option<CaseTranslation>,
    syntax: &dyn SyntaxLookup,
) -> Result<Rc<CompiledPattern>, String> {
    let effective_translation = if case_fold {
        translation.or_else(|| Some(CaseTranslation::standard()))
    } else {
        None
    };
    let translation_key = effective_translation
        .as_ref()
        .map(CaseTranslation::cache_key);
    let syntax_key = syntax.cache_key();

    if let Some(cached) = crate::emacs_core::perf_trace::time_op(
        crate::emacs_core::perf_trace::HotpathOp::RegexCompileHit,
        || {
            LISP_REGEX_PATTERN_CACHE.with(|cache| {
                let mut cache = cache.borrow_mut();
                let index = cache.iter().position(
                    |(
                        cached_posix,
                        cached_case_fold,
                        cached_translation_key,
                        cached_pattern_multibyte,
                        cached_target_multibyte,
                        cached_pattern,
                        cached_syntax_key,
                        _,
                    )| {
                        *cached_posix == posix
                            && *cached_case_fold == case_fold
                            && *cached_translation_key == translation_key
                            && *cached_pattern_multibyte == pattern.is_multibyte()
                            && *cached_target_multibyte == target_multibyte
                            && cached_pattern.as_slice() == pattern.as_bytes()
                            && syntax_key_matches(*cached_syntax_key, syntax_key)
                    },
                )?;
                let entry = cache.remove(index);
                cache.insert(0, entry.clone());
                Some(entry.7)
            })
        },
    ) {
        return Ok(cached);
    }

    let mut compiled = crate::emacs_core::perf_trace::time_op(
        crate::emacs_core::perf_trace::HotpathOp::RegexCompileMiss,
        || {
            regex_emacs::regex_compile_lisp_with_translation(pattern, posix, effective_translation)
                .map_err(|e| e.message)
        },
    )?;
    compiled.target_multibyte = target_multibyte;
    // Rebake the fastmap of `[[:word:]]`/`[[:space:]]` patterns against
    // the active syntax table (GNU compiles them with the buffer table
    // directly; see `compile_search_pattern_with_posix`).
    if compiled.used_syntax && syntax_key != SyntaxCacheKey::Standard {
        regex_emacs::recompute_fastmap(&mut compiled, syntax);
    }
    let entry_syntax_key = compiled.used_syntax.then_some(syntax_key);
    let compiled = Rc::new(compiled);

    LISP_REGEX_PATTERN_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.insert(
            0,
            (
                posix,
                case_fold,
                translation_key,
                pattern.is_multibyte(),
                target_multibyte,
                pattern.as_bytes().to_vec(),
                entry_syntax_key,
                compiled.clone(),
            ),
        );
        if cache.len() > SEARCH_PATTERN_CACHE_SIZE {
            cache.truncate(SEARCH_PATTERN_CACHE_SIZE);
        }
    });

    Ok(compiled)
}

fn compiled_capture_count(compiled: &CompiledSearchPattern) -> usize {
    match compiled {
        CompiledSearchPattern::Literal(_) => 1,
        CompiledSearchPattern::Emacs(cp) => cp.re_nsub + 1,
    }
}

fn find_forward_match_data_compiled(
    compiled: &CompiledSearchPattern,
    text: &[u8],
    start: usize,
    limit: usize,
    offset: usize,
    case_fold: bool,
) -> Option<MatchData> {
    match compiled {
        CompiledSearchPattern::Literal(literal) => {
            let matched = literal_find_emacs_bytes(&text[start..limit], literal, true, case_fold)?;
            let matched = matched.shift(offset + start);
            Some(MatchData::none(gnu_single_group_vec(Some(matched))))
        }
        CompiledSearchPattern::Emacs(cp) => {
            let syn = DefaultSyntaxLookup;
            let text_bytes = text;
            let range = (limit - start) as isize;
            let result =
                regex_emacs::re_search(cp, &text_bytes[..limit], start, range, &syn, start);
            result.map(|(_pos, regs)| match_data_from_registers(&regs, offset))
        }
    }
}

pub(crate) fn iterate_string_matches_with_case_fold(
    pattern: &LispString,
    string: &[u8],
    start: usize,
    case_fold: bool,
) -> Result<IteratedStringMatches, String> {
    let compiled = compile_search_pattern(pattern, case_fold)?;
    let capture_count = compiled_capture_count(&compiled);
    if start > string.len() {
        return Ok(IteratedStringMatches {
            capture_count,
            matches: Vec::new(),
        });
    }
    let mut matches = Vec::new();
    let mut search_at = start;

    while search_at <= string.len() {
        let Some(md) = find_forward_match_data_compiled(
            &compiled,
            string,
            search_at,
            string.len(),
            0,
            case_fold,
        ) else {
            break;
        };
        let Some(group) = md.groups.first().and_then(|group| *group) else {
            break;
        };
        matches.push(md.groups);

        if group.end() > search_at {
            search_at = group.end();
            continue;
        }

        let Some(next_at) = next_search_char_boundary(string, group.end()) else {
            break;
        };
        if next_at <= search_at {
            break;
        }
        search_at = next_at;
        if group.start() == group.end() && search_at > string.len() {
            break;
        }
    }

    Ok(IteratedStringMatches {
        capture_count,
        matches,
    })
}

fn string_char_match_data(searched_string: SearchedString, byte_md: MatchData) -> MatchData {
    crate::emacs_core::perf_trace::time_op(
        crate::emacs_core::perf_trace::HotpathOp::RegexMatchDataChars,
        || {
            let char_groups = byte_md
                .groups
                .iter()
                .map(|g| {
                    g.map(|group| {
                        MatchGroup::new(
                            searched_string.byte_to_char_pos(group.start()),
                            searched_string.byte_to_char_pos(group.end()),
                        )
                    })
                })
                .collect();

            MatchData {
                groups: char_groups,
                source: MatchSource::String {
                    searched: Some(searched_string),
                },
            }
        },
    )
}

fn single_group_match_data(start: usize, end: usize) -> MatchData {
    MatchData::none(gnu_single_group_vec(Some(MatchGroup::new(start, end))))
}

fn ascii_case_fold_find(haystack: &str, needle: &str) -> Option<usize> {
    let needle_len = needle.len();
    if needle_len == 0 {
        return Some(0);
    }
    let haystack_bytes = haystack.as_bytes();
    let needle_bytes = needle.as_bytes();
    if needle_len > haystack_bytes.len() {
        return None;
    }

    haystack_bytes.windows(needle_len).position(|window| {
        window
            .iter()
            .zip(needle_bytes.iter())
            .all(|(lhs, rhs)| lhs.eq_ignore_ascii_case(rhs))
    })
}

fn ascii_case_fold_rfind(haystack: &str, needle: &str) -> Option<usize> {
    let needle_len = needle.len();
    if needle_len == 0 {
        return Some(haystack.len());
    }
    let haystack_bytes = haystack.as_bytes();
    let needle_bytes = needle.as_bytes();
    if needle_len > haystack_bytes.len() {
        return None;
    }

    haystack_bytes.windows(needle_len).rposition(|window| {
        window
            .iter()
            .zip(needle_bytes.iter())
            .all(|(lhs, rhs)| lhs.eq_ignore_ascii_case(rhs))
    })
}

fn unicode_case_fold_literal_find(text: &str, literal: &str) -> Option<MatchGroup> {
    let needle: Vec<char> = literal.chars().flat_map(|ch| ch.to_lowercase()).collect();
    if needle.is_empty() {
        return Some(MatchGroup::new(0, 0));
    }
    let mut window = std::collections::VecDeque::with_capacity(needle.len());
    let mut ranges = std::collections::VecDeque::with_capacity(needle.len());
    for (byte_start, ch) in text.char_indices() {
        let byte_end = byte_start + ch.len_utf8();
        for folded_ch in ch.to_lowercase() {
            window.push_back(folded_ch);
            ranges.push_back((byte_start, byte_end));
            if window.len() > needle.len() {
                window.pop_front();
                ranges.pop_front();
            }
            if window.len() == needle.len()
                && window
                    .iter()
                    .zip(needle.iter())
                    .all(|(lhs, rhs)| lhs == rhs)
            {
                return Some(MatchGroup::new(ranges.front()?.0, ranges.back()?.1));
            }
        }
    }
    None
}

fn unicode_case_fold_literal_rfind(text: &str, literal: &str) -> Option<MatchGroup> {
    let needle: Vec<char> = literal.chars().flat_map(|ch| ch.to_lowercase()).collect();
    if needle.is_empty() {
        return Some(MatchGroup::new(text.len(), text.len()));
    }
    let mut last_match = None;
    let mut window = std::collections::VecDeque::with_capacity(needle.len());
    let mut ranges = std::collections::VecDeque::with_capacity(needle.len());
    for (byte_start, ch) in text.char_indices() {
        let byte_end = byte_start + ch.len_utf8();
        for folded_ch in ch.to_lowercase() {
            window.push_back(folded_ch);
            ranges.push_back((byte_start, byte_end));
            if window.len() > needle.len() {
                window.pop_front();
                ranges.pop_front();
            }
            if window.len() == needle.len()
                && window
                    .iter()
                    .zip(needle.iter())
                    .all(|(lhs, rhs)| lhs == rhs)
            {
                last_match = Some(MatchGroup::new(ranges.front()?.0, ranges.back()?.1));
            }
        }
    }
    last_match
}

/// Find LITERAL inside TEXT, optionally case-folded.
///
/// GNU `src/search.c:1761+` ports a Boyer-Moore implementation
/// (`boyer_moore`) with case-fold-aware skip table generation. For
/// long literal needles in large buffers Boyer-Moore is roughly
/// O(n/m) instead of O(n). neomacs uses naive substring scanning
/// here (delegating to `str::find` and a tiny ASCII case-fold
/// helper). Audit finding #18 in `drafts/regex-search-audit.md`
/// flags this as a perf gap, not a correctness gap; the audit's
/// Phase D Task 4.2 covers porting boyer_moore (~1 day).
fn literal_find(text: &str, literal: &str, case_fold: bool) -> Option<MatchGroup> {
    crate::emacs_core::perf_trace::time_op(
        crate::emacs_core::perf_trace::HotpathOp::RegexLiteralFind,
        || {
            let start = if case_fold {
                if literal.is_ascii() {
                    ascii_case_fold_find(text, literal)?
                } else {
                    return unicode_case_fold_literal_find(text, literal);
                }
            } else {
                text.find(literal)?
            };
            Some(MatchGroup::new(start, start + literal.len()))
        },
    )
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn literal_find_lisp_string(
    text: &crate::heap_types::LispString,
    literal: &str,
    start: usize,
    case_fold: bool,
) -> Option<MatchGroup> {
    crate::emacs_core::perf_trace::time_op(
        crate::emacs_core::perf_trace::HotpathOp::RegexLiteralFind,
        || {
            if start > text.byte_len() {
                return None;
            }

            if !text.is_multibyte() {
                let haystack = &text.as_bytes()[start..];
                let needle = literal.as_bytes();
                if needle.is_empty() {
                    return Some(MatchGroup::new(start, start));
                }
                if needle.len() > haystack.len() {
                    return None;
                }
                let match_start = haystack.windows(needle.len()).position(|window| {
                    if case_fold {
                        window
                            .iter()
                            .zip(needle.iter())
                            .all(|(lhs, rhs)| lhs.eq_ignore_ascii_case(rhs))
                    } else {
                        window == needle
                    }
                })?;
                let match_end = match_start + needle.len();
                return Some(MatchGroup::new(start + match_start, start + match_end));
            }

            let text = text.as_utf8_str()?;
            literal_find(&text[start..], literal, case_fold).map(|matched| matched.shift(start))
        },
    )
}

fn literal_rfind(text: &str, literal: &str, case_fold: bool) -> Option<MatchGroup> {
    crate::emacs_core::perf_trace::time_op(
        crate::emacs_core::perf_trace::HotpathOp::RegexLiteralFind,
        || {
            let start = if case_fold {
                if literal.is_ascii() {
                    ascii_case_fold_rfind(text, literal)?
                } else {
                    return unicode_case_fold_literal_rfind(text, literal);
                }
            } else {
                text.rfind(literal)?
            };
            Some(MatchGroup::new(start, start + literal.len()))
        },
    )
}

fn bytes_equal_ascii_case_fold(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right.iter())
            .all(|(l, r)| l.eq_ignore_ascii_case(r))
}

/// Decode the next character at byte offset `at`. In a unibyte target every
/// byte is its own character (raw bytes must not be decoded as multibyte leads,
/// see the unibyte note in `literal_find_emacs_bytes`); in a multibyte target
/// decode one Emacs char.
fn next_char_at(text: &[u8], at: usize, multibyte: bool) -> (u32, usize) {
    if multibyte {
        let (code, len) = crate::emacs_core::emacs_char::string_char(&text[at..]);
        (code, len.max(1))
    } else {
        (text[at] as u32, 1)
    }
}

/// Canonicalize `literal` char-by-char through the buffer case-canon table.
fn canon_fold_pattern(literal: &[u8], multibyte: bool, trt: &CaseTranslation) -> Vec<u32> {
    let mut pat = Vec::new();
    let mut i = 0;
    while i < literal.len() {
        let (code, len) = next_char_at(literal, i, multibyte);
        pat.push(trt.translate(code));
        i += len;
    }
    pat
}

/// True if `pat` (already canonicalized) matches `text` at byte offset `at`,
/// canonicalizing each text char through `trt`. Returns the end byte offset.
fn canon_fold_match_at(
    text: &[u8],
    at: usize,
    pat: &[u32],
    multibyte: bool,
    trt: &CaseTranslation,
) -> Option<usize> {
    let mut ti = at;
    for &pc in pat {
        if ti >= text.len() {
            return None;
        }
        let (code, len) = next_char_at(text, ti, multibyte);
        if trt.translate(code) != pc {
            return None;
        }
        ti += len;
    }
    Some(ti)
}

/// Forward literal search that folds through the buffer's case-canon table
/// (used only when a custom `set-case-syntax-pair` table is installed). GNU's
/// `simple_search` canonicalizes each char through the search `trt`.
fn canon_fold_literal_find(
    text: &[u8],
    literal: &[u8],
    multibyte: bool,
    trt: &CaseTranslation,
) -> Option<MatchGroup> {
    let pat = canon_fold_pattern(literal, multibyte, trt);
    if pat.is_empty() {
        return Some(MatchGroup::new(0, 0));
    }
    let mut at = 0;
    loop {
        if let Some(end) = canon_fold_match_at(text, at, &pat, multibyte, trt) {
            return Some(MatchGroup::new(at, end));
        }
        if at >= text.len() {
            return None;
        }
        let (_code, len) = next_char_at(text, at, multibyte);
        at += len;
    }
}

/// Backward analogue of `canon_fold_literal_find`: returns the rightmost match.
fn canon_fold_literal_rfind(
    text: &[u8],
    literal: &[u8],
    multibyte: bool,
    trt: &CaseTranslation,
) -> Option<MatchGroup> {
    let pat = canon_fold_pattern(literal, multibyte, trt);
    if pat.is_empty() {
        return Some(MatchGroup::new(text.len(), text.len()));
    }
    let mut last = None;
    let mut at = 0;
    loop {
        if let Some(end) = canon_fold_match_at(text, at, &pat, multibyte, trt) {
            last = Some(MatchGroup::new(at, end));
        }
        if at >= text.len() {
            return last;
        }
        let (_code, len) = next_char_at(text, at, multibyte);
        at += len;
    }
}

fn literal_find_emacs_bytes(
    text: &[u8],
    literal: &[u8],
    multibyte: bool,
    case_fold: bool,
) -> Option<MatchGroup> {
    crate::emacs_core::perf_trace::time_op(
        crate::emacs_core::perf_trace::HotpathOp::RegexLiteralFind,
        || {
            if literal.is_empty() {
                return Some(MatchGroup::new(0, 0));
            }
            if !case_fold {
                return text
                    .windows(literal.len())
                    .position(|window| window == literal)
                    .map(|start| MatchGroup::new(start, start + literal.len()));
            }
            if literal.is_ascii() {
                return text
                    .windows(literal.len())
                    .position(|window| bytes_equal_ascii_case_fold(window, literal))
                    .map(|start| MatchGroup::new(start, start + literal.len()));
            }
            // Unibyte target: GNU `simple_search` (search.c:1622-1633) advances
            // one BYTE per position and case-folds each byte through the case
            // table.  We must NOT decode multibyte sequences here, or a raw byte
            // embedded in what looks like a multibyte lead (e.g. the 0xA9 in a
            // C3 A9 pair) would be skipped — that was the unibyte search bug.
            if !multibyte {
                if text.len() < literal.len() {
                    return None;
                }
                for at in 0..=text.len() - literal.len() {
                    if let Some(end) = crate::emacs_core::emacs_char::unibyte_case_fold_match_len(
                        text, at, literal,
                    ) {
                        return Some(MatchGroup::new(at, end));
                    }
                }
                return None;
            }
            if let (Some(text_utf8), Some(literal_utf8)) = (
                crate::emacs_core::emacs_char::try_as_utf8(text),
                crate::emacs_core::emacs_char::try_as_utf8(literal),
            ) {
                return literal_find(text_utf8, literal_utf8, true);
            }

            // Non-ASCII case-fold over raw Emacs bytes: compare Emacs-downcased
            // char codes in place so offsets stay in the text's own byte space
            // (eight-bit chars are caseless, matching GNU) — no storage round-trip.
            let mut at = 0;
            loop {
                if let Some(end) =
                    crate::emacs_core::emacs_char::case_fold_match_len(text, at, literal)
                {
                    return Some(MatchGroup::new(at, end));
                }
                if at >= text.len() {
                    return None;
                }
                let (_code, len) = crate::emacs_core::emacs_char::string_char(&text[at..]);
                at += len.max(1);
            }
        },
    )
}

fn literal_rfind_emacs_bytes(
    text: &[u8],
    literal: &[u8],
    multibyte: bool,
    case_fold: bool,
) -> Option<MatchGroup> {
    crate::emacs_core::perf_trace::time_op(
        crate::emacs_core::perf_trace::HotpathOp::RegexLiteralFind,
        || {
            if literal.is_empty() {
                return Some(MatchGroup::new(text.len(), text.len()));
            }
            if !case_fold {
                return text
                    .windows(literal.len())
                    .enumerate()
                    .rev()
                    .find(|(_, window)| *window == literal)
                    .map(|(start, _)| MatchGroup::new(start, start + literal.len()));
            }
            if literal.is_ascii() {
                return text
                    .windows(literal.len())
                    .enumerate()
                    .rev()
                    .find(|(_, window)| bytes_equal_ascii_case_fold(window, literal))
                    .map(|(start, _)| MatchGroup::new(start, start + literal.len()));
            }
            // Unibyte target: byte-by-byte rightmost case-fold scan, mirroring
            // GNU `simple_search` reverse search.  See `literal_find_emacs_bytes`.
            if !multibyte {
                if text.len() < literal.len() {
                    return None;
                }
                for at in (0..=text.len() - literal.len()).rev() {
                    if let Some(end) = crate::emacs_core::emacs_char::unibyte_case_fold_match_len(
                        text, at, literal,
                    ) {
                        return Some(MatchGroup::new(at, end));
                    }
                }
                return None;
            }
            if let (Some(text_utf8), Some(literal_utf8)) = (
                crate::emacs_core::emacs_char::try_as_utf8(text),
                crate::emacs_core::emacs_char::try_as_utf8(literal),
            ) {
                return literal_rfind(text_utf8, literal_utf8, true);
            }

            // Rightmost non-ASCII case-fold match over raw Emacs bytes, matching
            // the prior rfind: compare Emacs-downcased char codes in place.
            let mut best = None;
            let mut at = 0;
            while at < text.len() {
                if let Some(end) =
                    crate::emacs_core::emacs_char::case_fold_match_len(text, at, literal)
                {
                    best = Some(MatchGroup::new(at, end));
                }
                let (_code, len) = crate::emacs_core::emacs_char::string_char(&text[at..]);
                at += len.max(1);
            }
            best
        },
    )
}

fn next_search_char_boundary(text: &[u8], pos: usize) -> Option<usize> {
    if pos >= text.len() {
        return None;
    }
    let (_code, len) = crate::emacs_core::emacs_char::string_char(&text[pos..]);
    Some(pos + len)
}

// ---------------------------------------------------------------------------
// Buffer search primitives
// ---------------------------------------------------------------------------

fn with_buffer_emacs_bytes<R>(
    buf: &Buffer,
    range: EmacsByteRange,
    f: impl FnOnce(&[u8]) -> R,
) -> R {
    if buf.has_contiguous_emacs_byte_range(range) {
        return buf
            .with_contiguous_emacs_byte_range(range, f)
            .expect("checked contiguous buffer range should borrow");
    }

    let mut text = Vec::new();
    buf.copy_emacs_byte_range_to(range, &mut text);
    f(&text)
}

/// [`with_buffer_emacs_bytes`] for the regex-engine search paths.
///
/// When the gap-buffer gap sits inside the searched range, every search
/// used to copy the whole accessible region (audit #17).  GNU never
/// copies: `re_search_2` walks the two gap segments in place.  The
/// port's engine is single-segment, so the clean equivalent is one gap
/// motion out of the range (amortized: after an edit parks the gap
/// mid-buffer, the first search moves it once and the rest of the
/// font-lock pass borrows zero-copy).  Chunked backends (piece tree,
/// rope) still take the copy fallback inside `with_buffer_emacs_bytes`.
fn with_buffer_emacs_bytes_for_search<R>(
    buf: &Buffer,
    range: EmacsByteRange,
    f: impl FnOnce(&[u8]) -> R,
) -> R {
    buf.try_make_emacs_byte_range_contiguous(range);
    with_buffer_emacs_bytes(buf, range, f)
}

/// Search forward from point for a literal string PATTERN.
///
/// If found, returns the end of match as the point position the caller should
/// apply. If not found, behaviour depends on `noerror`:
/// - `noerror` false: signals `search-failed`
/// - `noerror` true: returns `None` without signaling
///
/// `bound` optionally limits the search to positions <= bound.
pub fn search_forward(
    buf: &mut Buffer,
    pattern: &crate::heap_types::LispString,
    bound: Option<usize>,
    noerror: bool,
    case_fold: bool,
    match_data: &mut Option<MatchData>,
) -> Result<Option<usize>, String> {
    let start = buf.point_emacs_byte_pos();
    let accessible = buf.accessible_emacs_byte_region();
    let limit = bound
        .map(EmacsBytePos::new)
        .unwrap_or(accessible.end())
        .min(accessible.end());

    if start > limit {
        if noerror {
            return Ok(None);
        }
        return Err(format!(
            "Search failed: \"{}\"",
            crate::emacs_core::emacs_char::to_utf8_lossy(pattern.as_bytes())
        ));
    }

    let multibyte = buf.get_multibyte();
    let literal = coerce_pattern_to_buffer_bytes(pattern, multibyte);
    // A custom `set-case-syntax-pair` table folds custom pairs (e.g. [/])
    // during search; route through the buffer's case-canon table then, else
    // keep the fast hardwired ASCII/Unicode folding.
    let translation = buffer_search_translation(buf, case_fold);
    let found =
        with_buffer_emacs_bytes(
            buf,
            EmacsByteRange::new(start, limit),
            |text| match &translation {
                Some(trt) => canon_fold_literal_find(text, &literal, multibyte, trt),
                None => literal_find_emacs_bytes(text, &literal, multibyte, case_fold),
            },
        );

    if let Some(found) = found {
        let matched = found.shift(start.get());
        let match_end = matched.end();
        *match_data = Some(MatchData::buffer_bytes(
            gnu_single_group_vec(Some(matched)),
            Some(buf.id),
        ));
        Ok(Some(match_end))
    } else if noerror {
        // When noerror is t, don't move point.
        // When noerror is a value, move point to bound.
        Ok(None)
    } else {
        Err(format!(
            "Search failed: \"{}\"",
            crate::emacs_core::emacs_char::to_utf8_lossy(pattern.as_bytes())
        ))
    }
}

/// Search backward from point for a literal string PATTERN.
///
/// If found, returns the beginning of match as the point position the caller
/// should apply.
pub fn search_backward(
    buf: &mut Buffer,
    pattern: &crate::heap_types::LispString,
    bound: Option<usize>,
    noerror: bool,
    case_fold: bool,
    match_data: &mut Option<MatchData>,
) -> Result<Option<usize>, String> {
    let end = buf.point_emacs_byte_pos();
    let accessible = buf.accessible_emacs_byte_region();
    let limit = bound
        .map(EmacsBytePos::new)
        .unwrap_or(accessible.start())
        .max(accessible.start());

    if end < limit {
        if noerror {
            return Ok(None);
        }
        return Err(format!(
            "Search failed: \"{}\"",
            crate::emacs_core::emacs_char::to_utf8_lossy(pattern.as_bytes())
        ));
    }

    let multibyte = buf.get_multibyte();
    let literal = coerce_pattern_to_buffer_bytes(pattern, multibyte);
    let translation = buffer_search_translation(buf, case_fold);
    let found =
        with_buffer_emacs_bytes(
            buf,
            EmacsByteRange::new(limit, end),
            |text| match &translation {
                Some(trt) => canon_fold_literal_rfind(text, &literal, multibyte, trt),
                None => literal_rfind_emacs_bytes(text, &literal, multibyte, case_fold),
            },
        );

    if let Some(found) = found {
        let matched = found.shift(limit.get());
        *match_data = Some(MatchData::buffer_bytes(
            gnu_single_group_vec(Some(matched)),
            Some(buf.id),
        ));
        Ok(Some(matched.start()))
    } else if noerror {
        Ok(None)
    } else {
        Err(format!(
            "Search failed: \"{}\"",
            crate::emacs_core::emacs_char::to_utf8_lossy(pattern.as_bytes())
        ))
    }
}

/// Issue #131: coerce a literal search pattern to the buffer's multibyteness the
/// way GNU does, producing Emacs internal-encoding bytes directly — no storage
/// round-trip.
///
/// This mirrors GNU `search_buffer_non_re` (search.c:1319-1343), which coerces
/// the *pattern* to the *buffer's* multibyteness via `copy_text` before the
/// byte/char comparison:
///   - same multibyteness  → raw bytes as-is;
///   - unibyte pattern, multibyte buffer → widen via `str_to_multibyte`
///     (each raw byte becomes an eight-bit character);
///   - multibyte pattern, unibyte buffer → narrow via `str_to_unibyte`
///     (each char collapses to its low byte `c & 0xFF`, one byte per char).
///
/// The last case is what makes a genuine multibyte sequence fail to match the
/// equal raw bytes in a unibyte buffer: e.g. searching for the multibyte char
/// é (internal bytes C3 A9) in a unibyte buffer narrows the pattern to the
/// single byte 0xE9, so it cannot spuriously match the C3 A9 byte pair. (GNU
/// uses `copy_text`, NOT `string-as-unibyte`, which would reinterpret the
/// internal bytes and produce the wrong, raw-byte match.)
fn coerce_pattern_to_buffer_bytes(
    pattern: &crate::heap_types::LispString,
    buf_multibyte: bool,
) -> Vec<u8> {
    if pattern.is_multibyte() == buf_multibyte {
        pattern.as_bytes().to_vec()
    } else if buf_multibyte {
        crate::emacs_core::emacs_char::str_to_multibyte(pattern.as_bytes())
    } else {
        crate::emacs_core::emacs_char::str_to_unibyte(pattern.as_bytes())
    }
}

/// Search forward from point for a regex PATTERN.
///
/// If found, returns the end of match as the point position the caller should
/// apply.
/// Updates match data with capture groups.
pub fn re_search_forward(
    buf: &mut Buffer,
    pattern: &crate::heap_types::LispString,
    bound: Option<usize>,
    noerror: bool,
    case_fold: bool,
    match_data: &mut Option<MatchData>,
) -> Result<Option<usize>, String> {
    re_search_forward_with_posix(buf, pattern, bound, noerror, case_fold, false, match_data)
}

/// POSIX longest-match variant of [`re_search_forward`] used by
/// `posix-search-forward`. See GNU `src/search.c:Fposix_search_forward`.
pub fn re_search_forward_with_posix(
    buf: &mut Buffer,
    pattern: &crate::heap_types::LispString,
    bound: Option<usize>,
    noerror: bool,
    case_fold: bool,
    posix: bool,
    match_data: &mut Option<MatchData>,
) -> Result<Option<usize>, String> {
    let start = buf.point_emacs_byte_pos();
    let accessible = buf.accessible_emacs_byte_region();
    let limit = bound
        .map(EmacsBytePos::new)
        .unwrap_or(accessible.end())
        .min(accessible.end());

    if start > limit {
        if noerror {
            return Ok(None);
        }
        return Err(format!(
            "Search failed: \"{}\"",
            crate::emacs_core::emacs_char::to_utf8_lossy(pattern.as_bytes())
        ));
    }

    let region_start = accessible.start();
    let start_rel = start.get() - region_start.get();
    let limit_rel = limit.get() - region_start.get();
    let buffer_id = buf.id;
    let multibyte = buf.get_multibyte();
    let syn = buffer_syntax_lookup(buf);
    let compiled =
        compile_search_pattern_with_posix(&pattern_for_compile(pattern), case_fold, posix, &syn)?;

    let md_opt =
        with_buffer_emacs_bytes_for_search(buf, accessible.range(), |text| match &compiled {
            CompiledSearchPattern::Literal(literal) => {
                literal_find_emacs_bytes(&text[start_rel..limit_rel], literal, multibyte, case_fold)
                    .map(|matched| {
                        MatchData::buffer_bytes(
                            gnu_single_group_vec(Some(matched.shift(start.get()))),
                            Some(buffer_id),
                        )
                    })
            }
            CompiledSearchPattern::Emacs(cp) => {
                let range = (limit_rel - start_rel) as isize;
                regex_emacs::re_search(cp.as_ref(), text, start_rel, range, &syn, start_rel).map(
                    |(_pos, regs)| {
                        let mut md = buffer_match_data_from_registers(&regs, region_start.get());
                        md.set_buffer_id(buffer_id);
                        md
                    },
                )
            }
        });

    if md_opt.is_none()
        && matches!(compiled, CompiledSearchPattern::Emacs(_))
        && regex_emacs::take_matcher_overflow()
    {
        // GNU search.c:matcher_overflow — a -2 from the matcher is an
        // error, not a search failure.
        return Err(regex_emacs::MATCHER_OVERFLOW_MESSAGE.to_string());
    }
    if let Some(md) = md_opt {
        let full_match = md.groups[0].unwrap();
        *match_data = Some(md);
        Ok(Some(full_match.end()))
    } else if noerror {
        Ok(None)
    } else {
        Err(format!(
            "Search failed: \"{}\"",
            crate::emacs_core::emacs_char::to_utf8_lossy(pattern.as_bytes())
        ))
    }
}

/// Search backward from point for a regex PATTERN.
///
/// If found, returns the beginning of match as the point position the caller
/// should apply.
/// Updates match data with capture groups.
pub fn re_search_backward(
    buf: &mut Buffer,
    pattern: &crate::heap_types::LispString,
    bound: Option<usize>,
    noerror: bool,
    case_fold: bool,
    match_data: &mut Option<MatchData>,
) -> Result<Option<usize>, String> {
    re_search_backward_with_posix(buf, pattern, bound, noerror, case_fold, false, match_data)
}

/// POSIX longest-match variant of [`re_search_backward`] used by
/// `posix-search-backward`. See GNU `src/search.c:Fposix_search_backward`.
pub fn re_search_backward_with_posix(
    buf: &mut Buffer,
    pattern: &crate::heap_types::LispString,
    bound: Option<usize>,
    noerror: bool,
    case_fold: bool,
    posix: bool,
    match_data: &mut Option<MatchData>,
) -> Result<Option<usize>, String> {
    let end = buf.point_emacs_byte_pos();
    let accessible = buf.accessible_emacs_byte_region();
    let limit = bound
        .map(EmacsBytePos::new)
        .unwrap_or(accessible.start())
        .max(accessible.start());

    if end < limit {
        if noerror {
            return Ok(None);
        }
        return Err(format!(
            "Search failed: \"{}\"",
            crate::emacs_core::emacs_char::to_utf8_lossy(pattern.as_bytes())
        ));
    }

    let region_start = accessible.start();
    let start_rel = end.get() - region_start.get();
    let limit_rel = limit.get() - region_start.get();
    let buffer_id = buf.id;
    let multibyte = buf.get_multibyte();
    let syn = buffer_syntax_lookup(buf);
    let compiled =
        compile_search_pattern_with_posix(&pattern_for_compile(pattern), case_fold, posix, &syn)?;

    let md_opt =
        with_buffer_emacs_bytes_for_search(buf, accessible.range(), |text| match &compiled {
            CompiledSearchPattern::Literal(literal) => literal_rfind_emacs_bytes(
                &text[limit_rel..start_rel],
                literal,
                multibyte,
                case_fold,
            )
            .map(|matched| {
                MatchData::buffer_bytes(
                    gnu_single_group_vec(Some(matched.shift(region_start.get() + limit_rel))),
                    Some(buffer_id),
                )
            }),
            CompiledSearchPattern::Emacs(cp) => {
                // Backward search: negative range means search backward.
                let range = -((start_rel - limit_rel) as isize);
                regex_emacs::re_search(cp.as_ref(), &text, start_rel, range, &syn, start_rel).map(
                    |(_pos, regs)| {
                        let mut md = buffer_match_data_from_registers(&regs, region_start.get());
                        md.set_buffer_id(buffer_id);
                        md
                    },
                )
            }
        });

    if md_opt.is_none()
        && matches!(compiled, CompiledSearchPattern::Emacs(_))
        && regex_emacs::take_matcher_overflow()
    {
        return Err(regex_emacs::MATCHER_OVERFLOW_MESSAGE.to_string());
    }
    if let Some(md) = md_opt {
        let full_match = md.groups[0].unwrap();
        *match_data = Some(md);
        Ok(Some(full_match.start()))
    } else if noerror {
        Ok(None)
    } else {
        Err(format!(
            "Search failed: \"{}\"",
            crate::emacs_core::emacs_char::to_utf8_lossy(pattern.as_bytes())
        ))
    }
}

/// Build the case-fold translate table for a search in `buf`: the buffer's
/// case-canon table (GNU's search `trt`) when a custom `set-case-syntax-pair`
/// table is installed, else `None` so the engine's fast hardwired folding is
/// used. Mirrors GNU search.c installing `BVAR (current_buffer, case_canon_table)`.
fn buffer_search_translation(buf: &Buffer, case_fold: bool) -> Option<CaseTranslation> {
    if !case_fold {
        return None;
    }
    crate::emacs_core::casetab::buffer_case_canon_table(buf).map(CaseTranslation::from_char_table)
}

pub(crate) fn re_search_forward_lisp_with_posix(
    buf: &mut Buffer,
    pattern: &LispString,
    bound: Option<usize>,
    noerror: bool,
    case_fold: bool,
    posix: bool,
    match_data: &mut Option<MatchData>,
) -> Result<Option<usize>, String> {
    // GNU `re-search-forward` on a buffer drives the matcher with raw buffer
    // byte positions (`PT_BYTE`, `BEGV_BYTE`, `ZV_BYTE`), even when the
    // pattern itself is a Lisp string.
    let start = buf.point_emacs_byte_pos();
    let accessible = buf.accessible_emacs_byte_region();
    let limit = bound
        .map(EmacsBytePos::new)
        .unwrap_or(accessible.end())
        .min(accessible.end());

    if start > limit {
        if noerror {
            return Ok(None);
        }
        return Err("Search failed".to_string());
    }

    let region_start = accessible.start();
    let start_rel = start.get() - region_start.get();
    let limit_rel = limit.get() - region_start.get();
    let buffer_id = buf.id;
    let translation = buffer_search_translation(buf, case_fold);
    let syn = buffer_syntax_lookup(buf);
    let compiled = compile_lisp_pattern_with_posix_translation(
        pattern,
        case_fold,
        posix,
        buf.get_multibyte(),
        translation,
        &syn,
    )?;

    let search_result = with_buffer_emacs_bytes_for_search(buf, accessible.range(), |text| {
        regex_emacs::re_search(
            compiled.as_ref(),
            text,
            start_rel,
            (limit_rel - start_rel) as isize,
            &syn,
            start_rel,
        )
    });
    if search_result.is_none() && regex_emacs::take_matcher_overflow() {
        return Err(regex_emacs::MATCHER_OVERFLOW_MESSAGE.to_string());
    }
    if let Some((_pos, regs)) = search_result {
        let mut md = buffer_match_data_from_registers(&regs, region_start.get());
        md.set_buffer_id(buffer_id);
        let full_match = md.groups[0].unwrap();
        *match_data = Some(md);
        Ok(Some(full_match.end()))
    } else if noerror {
        Ok(None)
    } else {
        Err("Search failed".to_string())
    }
}

pub(crate) fn re_search_backward_lisp_with_posix(
    buf: &mut Buffer,
    pattern: &LispString,
    bound: Option<usize>,
    noerror: bool,
    case_fold: bool,
    posix: bool,
    match_data: &mut Option<MatchData>,
) -> Result<Option<usize>, String> {
    // GNU `re-search-backward` likewise uses buffer byte positions
    // throughout, not character positions.
    let end = buf.point_emacs_byte_pos();
    let accessible = buf.accessible_emacs_byte_region();
    let limit = bound
        .map(EmacsBytePos::new)
        .unwrap_or(accessible.start())
        .max(accessible.start());

    if end < limit {
        if noerror {
            return Ok(None);
        }
        return Err("Search failed".to_string());
    }

    let region_start = accessible.start();
    let start_rel = end.get() - region_start.get();
    let limit_rel = limit.get() - region_start.get();
    let buffer_id = buf.id;
    let syn = buffer_syntax_lookup(buf);
    let compiled = compile_lisp_pattern_with_posix_translation(
        pattern,
        case_fold,
        posix,
        buf.get_multibyte(),
        buffer_search_translation(buf, case_fold),
        &syn,
    )?;

    let search_result = with_buffer_emacs_bytes_for_search(buf, accessible.range(), |text| {
        regex_emacs::re_search(
            compiled.as_ref(),
            text,
            start_rel,
            -((start_rel - limit_rel) as isize),
            &syn,
            start_rel,
        )
    });
    if search_result.is_none() && regex_emacs::take_matcher_overflow() {
        return Err(regex_emacs::MATCHER_OVERFLOW_MESSAGE.to_string());
    }
    if let Some((_pos, regs)) = search_result {
        let mut md = buffer_match_data_from_registers(&regs, region_start.get());
        md.set_buffer_id(buffer_id);
        let full_match = md.groups[0].unwrap();
        *match_data = Some(md);
        Ok(Some(full_match.start()))
    } else if noerror {
        Ok(None)
    } else {
        Err("Search failed".to_string())
    }
}

/// Test if text after point matches PATTERN (without moving point).
///
/// Returns `true` if the regex matches starting exactly at point, and
/// updates match data.
pub fn looking_at(
    buf: &Buffer,
    pattern: &crate::heap_types::LispString,
    case_fold: bool,
    match_data: &mut Option<MatchData>,
) -> Result<bool, String> {
    looking_at_with_posix(buf, pattern, case_fold, false, match_data)
}

/// POSIX longest-match variant of [`looking_at`] used by
/// `posix-looking-at`. See GNU `src/search.c:Fposix_looking_at`.
pub fn looking_at_with_posix(
    buf: &Buffer,
    pattern: &crate::heap_types::LispString,
    case_fold: bool,
    posix: bool,
    match_data: &mut Option<MatchData>,
) -> Result<bool, String> {
    let start = buf.point_emacs_byte_pos();
    let accessible = buf.accessible_emacs_byte_region();
    if start > accessible.end() {
        return Ok(false);
    }

    let region_start = accessible.start();
    let start_rel = start.get() - region_start.get();
    let buffer_id = buf.id;
    let multibyte = buf.get_multibyte();
    let syn = buffer_syntax_lookup(buf);
    let compiled =
        compile_search_pattern_with_posix(&pattern_for_compile(pattern), case_fold, posix, &syn)?;

    match with_buffer_emacs_bytes_for_search(buf, accessible.range(), |text| match &compiled {
        CompiledSearchPattern::Literal(literal) => {
            let tail = &text[start_rel..];
            let matched = literal_find_emacs_bytes(tail, literal, multibyte, case_fold)
                .is_some_and(|matched| matched.start() == 0);
            if !matched {
                return Ok(false);
            }
            let full_match = MatchGroup::new(start.get(), start.get() + literal.len());
            *match_data = Some(MatchData::buffer_bytes(
                gnu_single_group_vec(Some(full_match)),
                Some(buffer_id),
            ));
            Ok(true)
        }
        CompiledSearchPattern::Emacs(cp) => {
            if let Some((_end, regs)) =
                regex_emacs::re_match(cp.as_ref(), &text, start_rel, text.len(), &syn, start_rel)
            {
                let mut md = buffer_match_data_from_registers(&regs, region_start.get());
                md.set_buffer_id(buffer_id);
                *match_data = Some(md);
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }) {
        Ok(matched) => Ok(matched),
        Err(err) => Err(err),
    }
}

pub(crate) fn looking_at_lisp_with_posix(
    buf: &Buffer,
    pattern: &LispString,
    case_fold: bool,
    posix: bool,
    match_data: &mut Option<MatchData>,
) -> Result<bool, String> {
    // GNU `Flooking_at` (`src/search.c:fast_looking_at`) operates on
    // byte offsets throughout: `BEGV_BYTE`, `PT_BYTE`, `ZV_BYTE`, and
    // the matcher's start/limit are all byte positions into the raw
    // gap-buffer text. Neomacs `buf.pt` / `buf.begv` / `buf.zv` are
    // *character* positions, so feeding them straight into the
    // byte-based regex engine breaks on any multibyte buffer — the
    // start position lands mid-UTF-8-sequence and the pattern fails
    // to match even when the char at `buf.pt` would have matched.
    let start = buf.point_emacs_byte_pos();
    let accessible = buf.accessible_emacs_byte_region();
    if start > accessible.end() {
        return Ok(false);
    }

    let region_start = accessible.start();
    let start_rel = start.get() - region_start.get();
    let buffer_id = buf.id;
    let syn = buffer_syntax_lookup(buf);
    let compiled = compile_lisp_pattern_with_posix_translation(
        pattern,
        case_fold,
        posix,
        buf.get_multibyte(),
        buffer_search_translation(buf, case_fold),
        &syn,
    )?;

    if let Some((_end, regs)) =
        with_buffer_emacs_bytes_for_search(buf, accessible.range(), |text| {
            regex_emacs::re_match(
                compiled.as_ref(),
                text,
                start_rel,
                text.len(),
                &syn,
                start_rel,
            )
        })
    {
        let mut md = buffer_match_data_from_registers(&regs, region_start.get());
        md.set_buffer_id(buffer_id);
        *match_data = Some(md);
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Test whether STRING matches PATTERN starting at byte offset 0.
///
/// Returns `true` if the regex matches at the beginning of STRING and updates
/// match data using character positions, mirroring `looking-at` semantics on a
/// string-backed source.
pub fn looking_at_string(
    pattern: &str,
    string: &str,
    case_fold: bool,
    match_data: &mut Option<MatchData>,
) -> Result<bool, String> {
    match compile_search_pattern(
        &crate::heap_types::LispString::from_utf8(pattern),
        case_fold,
    )? {
        CompiledSearchPattern::Literal(literal) => {
            let matched = literal_find_emacs_bytes(string.as_bytes(), &literal, true, case_fold)
                .is_some_and(|matched| matched.start() == 0);
            if !matched {
                return Ok(false);
            }
            *match_data = Some(string_char_match_data(
                SearchedString::Owned(LispString::from_utf8(string)),
                single_group_match_data(0, literal.len()),
            ));
            Ok(true)
        }
        CompiledSearchPattern::Emacs(cp) => {
            let syn = DefaultSyntaxLookup;
            let text_bytes = string.as_bytes();
            if let Some((_end, regs)) =
                regex_emacs::re_match(cp.as_ref(), text_bytes, 0, text_bytes.len(), &syn, 0)
            {
                let byte_md = match_data_from_registers(&regs, 0);
                *match_data = Some(string_char_match_data(
                    SearchedString::Owned(LispString::from_utf8(string)),
                    byte_md,
                ));
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }
}

/// Match a regex against a string (not a buffer).
///
/// `start` is the byte offset within `string` to begin matching.
/// Returns the CHARACTER position of the start of the match (relative
/// to the whole string, not `start`), or `None` if no match.
/// Updates match data with capture groups in CHARACTER positions;
/// stores the searched string.
pub fn string_match_full_with_case_fold(
    pattern: &str,
    string: &str,
    start: usize,
    case_fold: bool,
    match_data: &mut Option<MatchData>,
) -> Result<Option<usize>, String> {
    string_match_full_with_case_fold_and_posix(pattern, string, start, case_fold, false, match_data)
}

/// POSIX longest-match variant of [`string_match_full_with_case_fold`]
/// used by `posix-string-match`. See GNU `src/search.c:Fposix_string_match`.
pub fn string_match_full_with_case_fold_and_posix(
    pattern: &str,
    string: &str,
    start: usize,
    case_fold: bool,
    posix: bool,
    match_data: &mut Option<MatchData>,
) -> Result<Option<usize>, String> {
    string_match_full_with_case_fold_source_posix(
        pattern,
        string,
        SearchedString::Owned(LispString::from_utf8(string)),
        start,
        case_fold,
        posix,
        match_data,
    )
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn string_match_full_with_case_fold_source_lisp(
    pattern: &str,
    string: &crate::heap_types::LispString,
    searched_string: SearchedString,
    start: usize,
    case_fold: bool,
    match_data: &mut Option<MatchData>,
) -> Result<Option<usize>, String> {
    string_match_full_with_case_fold_source_lisp_posix(
        pattern,
        string,
        searched_string,
        start,
        case_fold,
        false,
        match_data,
    )
}

/// POSIX longest-match variant of
/// [`string_match_full_with_case_fold_source_lisp`] used by
/// `posix-string-match` on Lisp strings. See GNU
/// `src/search.c:Fposix_string_match`.
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn string_match_full_with_case_fold_source_lisp_posix(
    pattern: &str,
    string: &crate::heap_types::LispString,
    searched_string: SearchedString,
    start: usize,
    case_fold: bool,
    posix: bool,
    match_data: &mut Option<MatchData>,
) -> Result<Option<usize>, String> {
    let pattern = LispString::from_utf8(pattern);
    string_match_full_with_case_fold_source_lisp_pattern_posix(
        &pattern,
        string,
        searched_string,
        start,
        case_fold,
        posix,
        match_data,
    )
}

pub(crate) fn string_match_full_with_case_fold_source_lisp_pattern_posix(
    pattern: &LispString,
    string: &crate::heap_types::LispString,
    searched_string: SearchedString,
    start: usize,
    case_fold: bool,
    posix: bool,
    match_data: &mut Option<MatchData>,
) -> Result<Option<usize>, String> {
    string_match_full_with_case_fold_source_lisp_pattern_posix_syntax(
        pattern,
        string,
        searched_string,
        start,
        case_fold,
        posix,
        None,
        &DefaultSyntaxLookup,
        match_data,
    )
}

pub(crate) fn string_match_full_with_case_fold_source_lisp_pattern_posix_syntax(
    pattern: &LispString,
    string: &crate::heap_types::LispString,
    searched_string: SearchedString,
    start: usize,
    case_fold: bool,
    posix: bool,
    translation: Option<CaseTranslation>,
    syntax: &dyn SyntaxLookup,
    match_data: &mut Option<MatchData>,
) -> Result<Option<usize>, String> {
    if start > string.byte_len() {
        return Ok(None);
    }

    let compiled = compile_lisp_pattern_with_posix_translation(
        pattern,
        case_fold,
        posix,
        string.is_multibyte(),
        translation,
        syntax,
    )?;
    let text_bytes = string.as_bytes();
    let range = (text_bytes.len() - start) as isize;
    if let Some((_pos, regs)) = regex_emacs::re_search(
        compiled.as_ref(),
        text_bytes,
        start,
        range,
        syntax,
        STRING_MATCH_AT_DOT_UNREACHABLE,
    ) {
        let byte_md = match_data_from_registers(&regs, 0);
        let char_md = string_char_match_data(searched_string, byte_md);
        let result_pos = char_md.groups[0].unwrap().start();
        *match_data = Some(char_md);
        Ok(Some(result_pos))
    } else if regex_emacs::take_matcher_overflow() {
        Err(regex_emacs::MATCHER_OVERFLOW_MESSAGE.to_string())
    } else {
        Ok(None)
    }
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn string_match_full_with_case_fold_source(
    pattern: &str,
    string: &str,
    searched_string: SearchedString,
    start: usize,
    case_fold: bool,
    match_data: &mut Option<MatchData>,
) -> Result<Option<usize>, String> {
    string_match_full_with_case_fold_source_posix(
        pattern,
        string,
        searched_string,
        start,
        case_fold,
        false,
        match_data,
    )
}

pub(crate) fn string_match_full_with_case_fold_source_posix(
    pattern: &str,
    string: &str,
    searched_string: SearchedString,
    start: usize,
    case_fold: bool,
    posix: bool,
    match_data: &mut Option<MatchData>,
) -> Result<Option<usize>, String> {
    if start > string.len() {
        return Ok(None);
    }

    string_match_full_with_case_fold_source_compiled_syntax(
        compile_search_pattern_with_posix(
            &crate::heap_types::LispString::from_utf8(pattern),
            case_fold,
            posix,
            &DefaultSyntaxLookup,
        )?,
        string,
        searched_string,
        start,
        case_fold,
        &DefaultSyntaxLookup,
        match_data,
    )
}

fn string_match_full_with_case_fold_source_compiled_syntax(
    compiled: CompiledSearchPattern,
    string: &str,
    searched_string: SearchedString,
    start: usize,
    _case_fold: bool,
    syntax: &dyn SyntaxLookup,
    match_data: &mut Option<MatchData>,
) -> Result<Option<usize>, String> {
    match compiled {
        CompiledSearchPattern::Literal(literal) => {
            let byte_match =
                literal_find_emacs_bytes(&string.as_bytes()[start..], &literal, true, _case_fold)
                    .map(|matched| matched.shift(start));
            if let Some(byte_match) = byte_match {
                let char_md = string_char_match_data(
                    searched_string,
                    single_group_match_data(byte_match.start(), byte_match.end()),
                );
                let result_pos = char_md.groups[0].unwrap().start();
                *match_data = Some(char_md);
                Ok(Some(result_pos))
            } else {
                Ok(None)
            }
        }
        CompiledSearchPattern::Emacs(cp) => {
            let text_bytes = string.as_bytes();
            let range = (text_bytes.len() - start) as isize;
            if let Some((_pos, regs)) = regex_emacs::re_search(
                cp.as_ref(),
                text_bytes,
                start,
                range,
                syntax,
                STRING_MATCH_AT_DOT_UNREACHABLE,
            ) {
                let byte_md = match_data_from_registers(&regs, 0);
                let char_md = string_char_match_data(searched_string, byte_md);
                let result_pos = char_md.groups[0].unwrap().start();
                *match_data = Some(char_md);
                Ok(Some(result_pos))
            } else if regex_emacs::take_matcher_overflow() {
                Err(regex_emacs::MATCHER_OVERFLOW_MESSAGE.to_string())
            } else {
                Ok(None)
            }
        }
    }
}

/// Match a regex against a string using Emacs default case-fold behavior.
pub fn string_match_full(
    pattern: &str,
    string: &str,
    start: usize,
    match_data: &mut Option<MatchData>,
) -> Result<Option<usize>, String> {
    string_match_full_with_case_fold(pattern, string, start, true, match_data)
}

/// Replace the last match in a buffer and return `nil`-style success.
#[cfg(test)]
pub fn replace_match_buffer(
    buf: &mut Buffer,
    newtext: &str,
    fixedcase: bool,
    literal: bool,
    subexp: usize,
    match_data: &Option<MatchData>,
) -> Result<(), String> {
    replace_match_buffer_with_syntax(buf, newtext, fixedcase, literal, subexp, match_data, false)
}

/// Variant that also honors `case-symbols-as-words` for the
/// `fixedcase=nil` path. Mirrors GNU `src/search.c:2485,2494,2504`;
/// the buffer's own syntax table is always consulted via `buf`.
/// See audit findings #14/#20 in `drafts/regex-search-audit.md`.
#[cfg(test)]
pub fn replace_match_buffer_with_syntax(
    buf: &mut Buffer,
    newtext: &str,
    fixedcase: bool,
    literal: bool,
    subexp: usize,
    match_data: &Option<MatchData>,
    case_symbols_as_words: bool,
) -> Result<(), String> {
    let (match_start, match_end, replacement) = compute_buffer_replacement_with_syntax(
        buf,
        newtext,
        fixedcase,
        literal,
        subexp,
        match_data,
        case_symbols_as_words,
    )?;

    let match_range =
        EmacsByteRange::new(EmacsBytePos::new(match_start), EmacsBytePos::new(match_end));
    buf.replace_emacs_byte_range_lisp_string(match_range, &replacement);
    Ok(())
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn compute_buffer_replacement_with_syntax(
    buf: &Buffer,
    newtext: &str,
    fixedcase: bool,
    literal: bool,
    subexp: usize,
    match_data: &Option<MatchData>,
    case_symbols_as_words: bool,
) -> Result<(usize, usize, crate::heap_types::LispString), String> {
    let md = match match_data {
        Some(md) => md,
        None => return Err(REPLACE_MATCH_SUBEXP_MISSING.to_string()),
    };

    // Faithful Emacs-bytes view of the whole buffer; the replace core now
    // indexes/slices it by Emacs-byte offsets directly (issue #131).
    let source = buf.buffer_substring_bytes_range(buf.full_emacs_byte_range());
    let buf_syntax = crate::emacs_core::syntax::SyntaxTable::for_buffer(buf);
    let Some(match_group) = md.groups.get(subexp).and_then(|group| *group) else {
        return Err(REPLACE_MATCH_SUBEXP_MISSING.to_string());
    };
    let (buffer_start, buffer_end) = if md.is_string_match() {
        (
            buf.char_pos_to_emacs_byte_pos_clamped(CharPos0::new(match_group.start()))
                .get(),
            buf.char_pos_to_emacs_byte_pos_clamped(CharPos0::new(match_group.end()))
                .get(),
        )
    } else if md.uses_buffer_lisp_char_positions() {
        (
            buffer_lisp_match_char_pos_to_byte_pos(buf, match_group.start()).get(),
            buffer_lisp_match_char_pos_to_byte_pos(buf, match_group.end()).get(),
        )
    } else {
        (match_group.start(), match_group.end())
    };

    let (_byte_start, _byte_end, replacement_bytes) = compute_replacement_with_syntax(
        newtext,
        fixedcase,
        literal,
        subexp,
        match_data,
        &source,
        buf.get_multibyte(),
        Some(&buf_syntax),
        case_symbols_as_words,
    )?;
    let replacement = if buf.get_multibyte() {
        crate::heap_types::LispString::from_emacs_bytes(replacement_bytes)
    } else {
        crate::heap_types::LispString::from_unibyte(replacement_bytes)
    };

    Ok((buffer_start, buffer_end, replacement))
}

/// Replace the last match in SOURCE (Emacs-bytes) and return the resulting
/// Emacs-bytes. `source_multibyte` mirrors the source Lisp string's
/// `STRING_MULTIBYTE` flag and governs how the result is reassembled.
pub fn replace_match_string(
    source: &[u8],
    source_multibyte: bool,
    newtext: &str,
    fixedcase: bool,
    literal: bool,
    subexp: usize,
    match_data: &Option<MatchData>,
) -> Result<Vec<u8>, String> {
    replace_match_string_with_syntax(
        source,
        source_multibyte,
        newtext,
        fixedcase,
        literal,
        subexp,
        match_data,
        None,
        false,
    )
}

/// Variant of [`replace_match_string`] that threads the syntax table
/// and `case-symbols-as-words` into the case-preservation decision.
/// For pure string replacement (no buffer in scope), pass `None` for
/// the table to get GNU's standard-table baseline behavior.
pub fn replace_match_string_with_syntax(
    source: &[u8],
    source_multibyte: bool,
    newtext: &str,
    fixedcase: bool,
    literal: bool,
    subexp: usize,
    match_data: &Option<MatchData>,
    syntax_table: Option<&crate::emacs_core::syntax::SyntaxTable>,
    case_symbols_as_words: bool,
) -> Result<Vec<u8>, String> {
    let (byte_start, byte_end, replacement) = compute_replacement_with_syntax(
        newtext,
        fixedcase,
        literal,
        subexp,
        match_data,
        source,
        source_multibyte,
        syntax_table,
        case_symbols_as_words,
    )?;
    if byte_end > source.len() || byte_start > byte_end {
        return Err(REPLACE_MATCH_SUBEXP_MISSING.to_string());
    }
    let mut out = Vec::with_capacity(byte_start + replacement.len() + (source.len() - byte_end));
    out.extend_from_slice(&source[..byte_start]);
    out.extend_from_slice(&replacement);
    out.extend_from_slice(&source[byte_end..]);
    Ok(out)
}

/// Convert a character position to a byte offset in a string.
pub fn char_pos_to_byte(s: &str, char_pos: usize) -> usize {
    s.char_indices()
        .nth(char_pos)
        .map(|(byte_pos, _)| byte_pos)
        .unwrap_or(s.len())
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn compute_replacement(
    newtext: &str,
    fixedcase: bool,
    literal: bool,
    subexp: usize,
    match_data: &Option<MatchData>,
    source: &[u8],
    source_multibyte: bool,
) -> Result<(usize, usize, Vec<u8>), String> {
    compute_replacement_with_syntax(
        newtext,
        fixedcase,
        literal,
        subexp,
        match_data,
        source,
        source_multibyte,
        None,
        false,
    )
}

/// Variant of [`compute_replacement`] that also threads a syntax
/// table and the `case-symbols-as-words` flag into
/// `apply_replace_match_case`.
///
/// GNU `src/search.c:2485-2505` checks `SYNTAX(prevc) == Sword` (or
/// `Ssymbol` when `case-symbols-as-words` is non-nil) on the buffer
/// syntax table. Audit findings #14 and #20 in
/// `drafts/regex-search-audit.md` track neomacs's divergence; this
/// helper is the threading point callers must hit to keep parity.
///
/// # Multibyte / unibyte handling (audit #13, issue #131)
///
/// GNU `src/search.c:2622-2720` runs an explicit byte conversion
/// loop over the replacement, branching on both the replacement
/// string's representation and the target buffer's
/// `enable-multibyte-characters` flag.
///
/// `source` is now the faithful Emacs-bytes view of the searched
/// text and is indexed/sliced directly by Emacs-byte offsets. The
/// match-group positions are converted to Emacs-byte offsets the same
/// way the matcher decodes characters (`emacs_char::string_char`), so
/// eight-bit raw bytes and Private-Use-Area glyphs survive intact
/// instead of round-tripping through the legacy PUA-sentinel storage
/// form.
fn compute_replacement_with_syntax(
    newtext: &str,
    fixedcase: bool,
    literal: bool,
    subexp: usize,
    match_data: &Option<MatchData>,
    source: &[u8],
    source_multibyte: bool,
    syntax_table: Option<&crate::emacs_core::syntax::SyntaxTable>,
    case_symbols_as_words: bool,
) -> Result<(usize, usize, Vec<u8>), String> {
    use crate::emacs_core::emacs_char::char_to_byte_pos;

    let md = match match_data {
        Some(md) => md,
        None => return Err(REPLACE_MATCH_SUBEXP_MISSING.to_string()),
    };

    let Some(match_group) = md.groups.get(subexp).and_then(|group| *group) else {
        return Err(REPLACE_MATCH_SUBEXP_MISSING.to_string());
    };
    // String searches, and GNU-style `set-match-data` restores on buffers,
    // expose character positions. Engine-produced buffer match data stays on
    // Emacs byte positions until the Lisp boundary.
    let string_positions_are_chars = md.is_string_match();
    let buffer_positions_are_lisp_chars = md.uses_buffer_lisp_char_positions();
    let (byte_start, byte_end) = if string_positions_are_chars {
        (
            char_to_byte_pos(source, match_group.start()),
            char_to_byte_pos(source, match_group.end()),
        )
    } else if buffer_positions_are_lisp_chars {
        // `set-match-data` restores buffer positions in Lisp character
        // coordinates, which are 1-based. Convert them back to 0-based
        // character offsets before locating their Emacs-byte boundaries.
        (
            char_to_byte_pos(
                source,
                lisp_char_pos_to_zero_based_index(match_group.start()),
            ),
            char_to_byte_pos(source, lisp_char_pos_to_zero_based_index(match_group.end())),
        )
    } else {
        // Engine-produced buffer match data already carries Emacs-byte
        // offsets, which now index `source` directly.
        (match_group.start(), match_group.end())
    };

    if byte_end > source.len() || byte_start > byte_end {
        return Err(REPLACE_MATCH_SUBEXP_MISSING.to_string());
    }

    let mut replacement = if literal {
        newtext.as_bytes().to_vec()
    } else {
        build_replacement(
            newtext,
            md,
            source,
            string_positions_are_chars,
            buffer_positions_are_lisp_chars,
        )?
    };

    if !fixedcase {
        let matched = &source[byte_start..byte_end];
        replacement = apply_match_case_with_syntax(
            replacement,
            matched,
            source_multibyte,
            syntax_table,
            case_symbols_as_words,
        );
    }

    Ok((byte_start, byte_end, replacement))
}

/// Build a replacement string handling `\&` (whole match) and
/// `\N` (group N, 1-9 only).
///
/// Error semantics mirror GNU `src/search.c:2545-2714` exactly:
///
/// - `\&` → the whole match (`md.groups[0]`). See search.c:2560
///   and search.c:2701.
/// - `\1`..`\9` → the Nth subgroup. `\0` is NOT accepted: GNU's
///   `Freplace_match` loop at search.c:2565 explicitly checks
///   `c >= '1' && c <= '9'`, mirrored at search.c:2703. Any `\0`
///   falls into the `"Invalid use of \\ in replacement text"`
///   error branch at search.c:2584 and 2713. This was audit
///   finding #11 in `drafts/regex-search-audit.md`: before this
///   fix, our `'0'..='9'` range accepted `\0` and returned the
///   whole match.
/// - `\\` → a literal backslash (search.c:2581-2582 and 2708-2709).
/// - `\?` → GNU's string path at search.c:2583 has an explicit
///   `else if (c != '?')` exception: when `c == '?'` neither
///   `substart >= 0` nor `delbackslash` is set, so `lastpos`
///   doesn't advance and the `\?` bytes fall through into the
///   next "middle" copy, effectively emitting the literal `\?`.
///   We mirror that here for both code paths (buffer/string).
/// - Any other `\X` → the same "Invalid use of `\\' in replacement
///   text" error. This was audit finding #12: before this fix, our
///   catch-all silently emitted the literal `\X`.
///
/// The caller (`compute_replacement`) propagates the error; the
/// outer search builtin signals a Lisp error with the GNU-shaped
/// message.
fn build_replacement(
    template: &str,
    md: &MatchData,
    source: &[u8],
    string_char_positions: bool,
    buffer_lisp_char_positions: bool,
) -> Result<Vec<u8>, String> {
    use crate::emacs_core::emacs_char::char_to_byte_pos;

    const INVALID_BACKSLASH_MSG: &str = "Invalid use of `\\' in replacement text";

    fn next_char_at(s: &str, byte_idx: usize) -> Option<(char, usize)> {
        s.get(byte_idx..)
            .and_then(|tail| tail.chars().next().map(|ch| (ch, ch.len_utf8())))
    }

    /// Convert a match-group endpoint to an Emacs-byte offset in `source`,
    /// mirroring the coordinate handling in `compute_replacement_with_syntax`:
    /// string searches use 0-based char positions, `set-match-data` restores
    /// use 1-based Lisp char positions, and engine-produced buffer data already
    /// carries Emacs-byte offsets.
    fn group_pos_to_byte(
        source: &[u8],
        pos: usize,
        string_char_positions: bool,
        buffer_lisp_char_positions: bool,
    ) -> usize {
        if string_char_positions {
            char_to_byte_pos(source, pos)
        } else if buffer_lisp_char_positions {
            char_to_byte_pos(source, lisp_char_pos_to_zero_based_index(pos))
        } else {
            pos
        }
    }

    /// Extract the matched group's Emacs-bytes from `source`.
    fn extract_group(
        source: &[u8],
        s: usize,
        e: usize,
        string_char_positions: bool,
        buffer_lisp_char_positions: bool,
    ) -> Option<&[u8]> {
        let bs = group_pos_to_byte(source, s, string_char_positions, buffer_lisp_char_positions);
        let be = group_pos_to_byte(source, e, string_char_positions, buffer_lisp_char_positions);
        if be <= source.len() && bs <= be {
            Some(&source[bs..be])
        } else {
            None
        }
    }

    let mut out: Vec<u8> = Vec::with_capacity(template.len());
    let bytes = template.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'\\' && i + 1 < len {
            let (next, next_len) =
                next_char_at(template, i + 1).expect("byte index must be char boundary");
            match next {
                '&' => {
                    // Whole match
                    if let Some(Some(group)) = md.groups.first() {
                        if let Some(text) = extract_group(
                            source,
                            group.start(),
                            group.end(),
                            string_char_positions,
                            buffer_lisp_char_positions,
                        ) {
                            out.extend_from_slice(text);
                        }
                    }
                    i += 1 + next_len;
                }
                '1'..='9' => {
                    // GNU search.c:2549 — explicit `c >= '1' && c <= '9'`.
                    // `\0` intentionally falls through to the error arm.
                    let group = (next as u8 - b'0') as usize;
                    if let Some(Some(group)) = md.groups.get(group) {
                        if let Some(text) = extract_group(
                            source,
                            group.start(),
                            group.end(),
                            string_char_positions,
                            buffer_lisp_char_positions,
                        ) {
                            out.extend_from_slice(text);
                        }
                    }
                    i += 1 + next_len;
                }
                '\\' => {
                    // GNU search.c:2581-2582, 2708-2709.
                    out.push(b'\\');
                    i += 1 + next_len;
                }
                '?' => {
                    // GNU search.c:2583 `else if (c != '?')`.
                    // `\?` is passed through literally in the
                    // string path; we honor that for both paths.
                    out.push(b'\\');
                    out.push(b'?');
                    i += 1 + next_len;
                }
                _ => {
                    // GNU search.c:2584, 2713 — any other backslash
                    // sequence (`\0`, `\n`, `\X`, …) signals an
                    // `error ("Invalid use of `\\' in replacement
                    // text")`.
                    return Err(INVALID_BACKSLASH_MSG.to_string());
                }
            }
        } else {
            // Template bytes are UTF-8 (valid Emacs-bytes); copy verbatim.
            out.push(bytes[i]);
            i += 1;
        }
    }

    Ok(out)
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn apply_match_case(replacement: &str, matched: &str) -> String {
    apply_replace_match_case(replacement, matched)
}

/// Byte-faithful case preservation for the replace core (issue #131).
///
/// `replacement` and `matched` are Emacs-bytes; `source_multibyte` mirrors the
/// searched text's `STRING_MULTIBYTE` flag so eight-bit raw bytes and
/// Private-Use-Area glyphs are analyzed/cased through the LispString case
/// primitives instead of the legacy PUA-sentinel storage form.
fn apply_match_case_with_syntax(
    replacement: Vec<u8>,
    matched: &[u8],
    source_multibyte: bool,
    syntax_table: Option<&crate::emacs_core::syntax::SyntaxTable>,
    case_symbols_as_words: bool,
) -> Vec<u8> {
    use crate::emacs_core::casefiddle::{
        apply_replace_match_case_lisp, apply_replace_match_case_lisp_with,
    };
    use crate::emacs_core::syntax::SyntaxClass;
    use crate::heap_types::LispString;

    let make_lisp = |bytes: Vec<u8>| {
        if source_multibyte {
            LispString::from_emacs_bytes(bytes)
        } else {
            LispString::from_unibyte(bytes)
        }
    };
    let replacement_lisp = make_lisp(replacement);
    let matched_lisp = make_lisp(matched.to_vec());

    let result = match syntax_table {
        None => apply_replace_match_case_lisp(&replacement_lisp, &matched_lisp),
        Some(table) => {
            apply_replace_match_case_lisp_with(&replacement_lisp, &matched_lisp, move |ch| {
                let class = table.char_syntax(ch);
                class == SyntaxClass::Word
                    || (case_symbols_as_words && class == SyntaxClass::Symbol)
            })
        }
    };
    result.as_bytes().to_vec()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
#[path = "regex_test.rs"]
mod tests;

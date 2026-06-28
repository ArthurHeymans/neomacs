//! GNU Emacs regex engine translated to Rust.
//!
//! This is a direct translation of GNU Emacs's `regex-emacs.c` — the same
//! algorithm, same bytecode format, same semantics.  The engine compiles
//! Emacs regex patterns to bytecode and executes them with syntax-table
//! awareness, backreference support, and POSIX backtracking.
//!
//! ## Architecture
//!
//! ```text
//! Pattern string
//!     ↓
//! regex_compile()     →  CompiledPattern (bytecode + fastmap)
//!     ↓
//! re_search()         →  Find match position (uses fastmap for skipping)
//!     ↓
//! re_match_internal() →  Execute bytecode against text (backtracking)
//!     ↓
//! MatchRegisters      →  Group start/end positions
//! ```
//!
//! ## Reference
//!
//! - GNU source: `src/regex-emacs.c` (5355 lines)
//! - GNU header: `src/regex-emacs.h`
//! - GNU search: `src/search.c` (3514 lines)

use std::collections::{HashMap, HashSet};

use crate::emacs_core::{emacs_char, syntax::SyntaxClass};
use smallvec::SmallVec;

const INLINE_REGEX_REGISTERS: usize = 8;
type RegisterScratch = SmallVec<[Option<usize>; INLINE_REGEX_REGISTERS]>;
type SavedRegisters = SmallVec<[(usize, i64, i64); INLINE_REGEX_REGISTERS]>;

// ---------------------------------------------------------------------------
// Phase 1: Opcodes and Data Structures
// ---------------------------------------------------------------------------

/// Bytecode opcodes for the compiled regex pattern.
///
/// Translated from `re_opcode_t` enum in regex-emacs.c (lines 202-337).
/// Each opcode may be followed by argument bytes in the bytecode buffer.
/// Bytecode opcodes for the compiled regex pattern.
///
/// **Strict GNU parity**: the numeric values here mirror
/// `enum re_opcode_t` in GNU `src/regex-emacs.c:202-337` exactly.
/// A compiled pattern emitted by our compiler is byte-compatible with
/// the same pattern emitted by GNU's compiler — every opcode occupies
/// the same numeric slot, so bytecode dumps can be compared directly
/// during debugging and future external tools can read either
/// without a translation layer.
///
/// The one-byte form we emit via `<op> as u8` is the same as GNU's
/// `BUF_COMPILED[pc++]` byte. **Do not reorder without updating the
/// GNU reference at the top of this file.**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum RegexOp {
    /// No operation (padding/alignment). GNU `no_op` = 0.
    NoOp = 0,

    /// Succeed immediately — no more backtracking. GNU `succeed` = 1.
    Succeed = 1,

    /// Match N exact bytes.  Followed by one byte N, then N literal
    /// bytes. GNU `exactn` = 2.
    Exactn = 2,

    /// Match any character (except newline in some modes).
    /// GNU `anychar` = 3.
    AnyChar = 3,

    /// Match character in bitmap set. Same byte layout as GNU
    /// `charset` = 4:
    /// - 1 byte: bitmap length (low 7 bits), high bit = has range table
    /// - N bytes: bitmap (bit per character, low-bit-first)
    /// - Optional range table for multibyte characters
    Charset = 4,

    /// Match character NOT in bitmap set.  Same format as `Charset`.
    /// GNU `charset_not` = 5.
    CharsetNot = 5,

    /// Start remembering text for group N.  Followed by 1 byte: group
    /// number. GNU `start_memory` = 6.
    StartMemory = 6,

    /// Stop remembering text for group N.  Followed by 1 byte: group
    /// number. GNU `stop_memory` = 7.
    StopMemory = 7,

    /// Match duplicate of group N (backreference \N).  Followed by
    /// 1 byte: group number. GNU `duplicate` = 8.
    Duplicate = 8,

    /// Fail unless at beginning of line (^). GNU `begline` = 9.
    BegLine = 9,

    /// Fail unless at end of line ($). GNU `endline` = 10.
    EndLine = 10,

    /// Succeed at beginning of buffer/string. `` \` ``.
    /// GNU `begbuf` = 11.
    BegBuf = 11,

    /// Succeed at end of buffer/string. `\'`.
    /// GNU `endbuf` = 12.
    EndBuf = 12,

    /// Unconditional jump.  Followed by 2-byte signed offset.
    /// GNU `jump` = 13.
    Jump = 13,

    /// Push failure point, then continue.  Followed by 2-byte signed
    /// offset. GNU `on_failure_jump` = 14.
    OnFailureJump = 14,

    /// Like `OnFailureJump` but doesn't restore string position on
    /// failure. GNU `on_failure_keep_string_jump` = 15.
    OnFailureKeepStringJump = 15,

    /// Like `OnFailureJump` but detects infinite empty-match loops.
    /// GNU `on_failure_jump_loop` = 16.
    OnFailureJumpLoop = 16,

    /// Like `OnFailureJumpLoop` but for non-greedy operators.
    /// GNU `on_failure_jump_nastyloop` = 17.
    OnFailureJumpNastyloop = 17,

    /// Smart jump for greedy `*` and `+`.  Analyzes loop to optimize.
    /// GNU `on_failure_jump_smart` = 18.
    OnFailureJumpSmart = 18,

    /// Match N times then jump on failure.  Followed by 2-byte offset
    /// + 2-byte count. GNU `succeed_n` = 19.
    SucceedN = 19,

    /// Jump N times then fail.  Followed by 2-byte offset + 2-byte
    /// count. GNU `jump_n` = 20.
    JumpN = 20,

    /// Set counter at offset.  Followed by 2-byte offset + 2-byte
    /// value. GNU `set_number_at` = 21.
    SetNumberAt = 21,

    /// Succeed at word beginning (syntax-table aware).  `\<`.
    /// GNU `wordbeg` = 22.
    WordBeg = 22,

    /// Succeed at word end (syntax-table aware).  `\>`.
    /// GNU `wordend` = 23.
    WordEnd = 23,

    /// Succeed at word boundary (syntax-table aware).  `\b`.
    /// GNU `wordbound` = 24.
    WordBound = 24,

    /// Succeed at non-word boundary (syntax-table aware).  `\B`.
    /// GNU `notwordbound` = 25.
    NotWordBound = 25,

    /// Succeed at symbol beginning (syntax-table aware).  `\_<`.
    /// GNU `symbeg` = 26.
    SymBeg = 26,

    /// Succeed at symbol end (syntax-table aware).  `\_>`.
    /// GNU `symend` = 27.
    SymEnd = 27,

    /// Match character with syntax class C.  Followed by 1 byte:
    /// syntax code.  `\sC`. GNU `syntaxspec` = 28.
    SyntaxSpec = 28,

    /// Match character without syntax class C.  Followed by 1 byte.
    /// `\SC`. GNU `notsyntaxspec` = 29.
    NotSyntaxSpec = 29,

    /// Succeed if at point.  `\=`. GNU `at_dot` = 30.
    AtDot = 30,

    /// Match character with category C.  Followed by 1 byte: category
    /// code.  `\cC`. GNU `categoryspec` = 31.
    CategorySpec = 31,

    /// Match character without category C.  Followed by 1 byte.
    /// `\CC`. GNU `notcategoryspec` = 32.
    NotCategorySpec = 32,
}

impl RegexOp {
    /// Convert a byte to an opcode.  Returns None for invalid bytes.
    fn from_byte(b: u8) -> Option<Self> {
        if b <= 32 {
            // SAFETY: all values 0-32 are valid enum variants
            Some(unsafe { std::mem::transmute(b) })
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Compiled Pattern
// ---------------------------------------------------------------------------

/// A compiled regex pattern — the output of `regex_compile()`.
///
/// Mirrors GNU's `struct re_pattern_buffer` from regex-emacs.h.
#[derive(Clone)]
pub(crate) struct CompiledPattern {
    /// Bytecode buffer.
    pub buffer: Vec<u8>,

    /// Number of subexpressions (groups).
    pub re_nsub: usize,

    /// Fast rejection map: fastmap[c] is true if the pattern can start
    /// with byte c.  Used by `re_search` to skip non-matching positions.
    pub fastmap: [bool; 256],

    /// Whether the fastmap is valid (needs recomputation after compile).
    pub fastmap_accurate: bool,

    /// True if the pattern was compiled for POSIX backtracking.
    pub posix: bool,

    /// True if the source regexp string was multibyte.
    pub multibyte: bool,

    /// True if the current search target is multibyte.
    pub target_multibyte: bool,

    /// True if the pattern can match the empty string.
    pub can_be_null: bool,

    /// True if matching this pattern depends on the active syntax table.
    ///
    /// GNU `src/search.c:compile_pattern` keeps syntax-table-sensitive
    /// regexps in cache entries keyed by `BVAR (current_buffer, syntax_table)`.
    /// Neomacs currently keeps compiled regexp bytecode independent of the
    /// syntax table and passes the active table to the matcher, so fastmap
    /// skipping must be disabled for these patterns unless the fastmap was
    /// built for the same table.
    pub uses_syntax: bool,

    /// Character translation table for case-folding.
    ///
    /// GNU stores the active case-canon char-table in `re_pattern_buffer.translate`.
    /// Keep the same shape at the matcher boundary: literal compilation and input
    /// matching both call through this translator, with a 256-entry byte fast path
    /// only for fastmap/bitmap operations.
    pub translate: Option<CaseTranslation>,

    /// Multibyte (non-ASCII) character ranges for Charset/CharsetNot opcodes.
    /// Key = bytecode position of the Charset/CharsetNot opcode.
    /// Value = list of inclusive (start_char, end_char) character ranges.
    pub multibyte_charsets: HashMap<usize, Vec<(char, char)>>,

    /// Per-charset POSIX character class flags.
    ///
    /// GNU stores POSIX classes in the charset range-table bits
    /// (`regex-emacs.c:re_wctype_to_bit`) and checks `re_iswctype`
    /// while executing the charset.  The ASCII bitmap remains the fast path,
    /// but these bits preserve the runtime predicate for multibyte characters
    /// and for syntax-table-sensitive classes such as `word` and `space`.
    pub charset_class_bits: HashMap<usize, u32>,
}

#[derive(Clone, Debug)]
pub struct CaseTranslation {
    byte: [u32; 256],
    table: Option<crate::emacs_core::value::Value>,
}

impl CaseTranslation {
    pub(crate) fn standard() -> Self {
        // The canonicalization of bytes 0..256 is a constant
        // (`downcase_char_code_emacs_compat`), but a case-insensitive regex is
        // recompiled on every `re-search-forward`, and recomputing this table
        // via Unicode case folding each time was ~15% of a search.  GNU keeps
        // its case-canon table precomputed; build it once per thread and copy.
        thread_local! {
            static STANDARD_BYTE: [u32; 256] = {
                let mut byte = [0u32; 256];
                for i in 0..=255u32 {
                    byte[i as usize] = CaseTranslation::canonicalize_char(i);
                }
                byte
            };
        }
        let byte = STANDARD_BYTE.with(|b| *b);
        Self { byte, table: None }
    }

    pub(crate) fn from_char_table(table: crate::emacs_core::value::Value) -> Self {
        let mut byte = [0u32; 256];
        for i in 0..=255i64 {
            byte[i as usize] = crate::emacs_core::chartable::translate_char(&table, i) as u32;
        }
        Self {
            byte,
            table: Some(table),
        }
    }

    pub(crate) fn cache_key(&self) -> usize {
        self.table.map_or(0, |table| table.bits())
    }

    fn translate(&self, c: u32) -> u32 {
        if let Some(translated) = self.byte.get(c as usize).copied() {
            return translated;
        }
        if let Some(table) = self.table {
            return crate::emacs_core::chartable::translate_char(&table, c as i64) as u32;
        }
        Self::canonicalize_char(c)
    }

    fn translate_byte(&self, c: u8) -> u8 {
        self.byte[c as usize] as u8
    }

    fn canonicalize_char(c: u32) -> u32 {
        crate::emacs_core::builtins::downcase_char_code_emacs_compat(c as i64) as u32
    }
}

const CHARSET_CLASS_BIT_ALNUM: u32 = 1 << 0;
const CHARSET_CLASS_BIT_ALPHA: u32 = 1 << 1;
const CHARSET_CLASS_BIT_BLANK: u32 = 1 << 2;
const CHARSET_CLASS_BIT_CNTRL: u32 = 1 << 3;
const CHARSET_CLASS_BIT_DIGIT: u32 = 1 << 4;
const CHARSET_CLASS_BIT_GRAPH: u32 = 1 << 5;
const CHARSET_CLASS_BIT_LOWER: u32 = 1 << 6;
const CHARSET_CLASS_BIT_PRINT: u32 = 1 << 7;
const CHARSET_CLASS_BIT_PUNCT: u32 = 1 << 8;
const CHARSET_CLASS_BIT_SPACE: u32 = 1 << 9;
const CHARSET_CLASS_BIT_UPPER: u32 = 1 << 10;
const CHARSET_CLASS_BIT_XDIGIT: u32 = 1 << 11;
const CHARSET_CLASS_BIT_ASCII: u32 = 1 << 12;
const CHARSET_CLASS_BIT_WORD: u32 = 1 << 13;
const CHARSET_CLASS_BIT_NONASCII: u32 = 1 << 14;
const CHARSET_CLASS_BIT_UNIBYTE: u32 = 1 << 15;
const CHARSET_CLASS_BIT_MULTIBYTE: u32 = 1 << 16;

impl CompiledPattern {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(256),
            re_nsub: 0,
            fastmap: [false; 256],
            fastmap_accurate: false,
            posix: false,
            multibyte: true,
            target_multibyte: true,
            can_be_null: false,
            uses_syntax: false,
            translate: None,
            multibyte_charsets: HashMap::new(),
            charset_class_bits: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Match Registers
// ---------------------------------------------------------------------------

/// Match result — stores group start/end positions.
///
/// Mirrors GNU's `struct re_registers` from regex-emacs.h.
#[derive(Clone, Debug)]
pub(crate) struct MatchRegisters {
    /// Start positions for each group (group 0 = full match).
    /// -1 means group did not participate in match.
    pub start: Vec<i64>,

    /// End positions for each group.
    pub end: Vec<i64>,
}

impl MatchRegisters {
    pub fn new(num_groups: usize) -> Self {
        Self {
            start: vec![-1; num_groups],
            end: vec![-1; num_groups],
        }
    }

    pub fn num_regs(&self) -> usize {
        self.start.len()
    }
}

// ---------------------------------------------------------------------------
// Failure Stack (for backtracking)
// ---------------------------------------------------------------------------

/// A single failure point on the backtracking stack.
///
/// When the matcher hits a choice point (OnFailureJump), it pushes the
/// current state so it can backtrack if the primary path fails.
#[derive(Clone, Debug)]
struct FailurePoint {
    /// Position in the bytecode to resume at.
    pattern_pos: usize,

    /// Position in the input text to resume at.
    /// None means "keep current string position" (OnFailureKeepStringJump).
    string_pos: Option<usize>,

    /// Saved group register values at this point.
    saved_registers: SavedRegisters, // (group_idx, start, end)

    /// Saved interval-counter overrides at this point.
    /// Keyed by bytecode position of the 2-byte counter field.
    saved_counters: HashMap<usize, i16>,
}

// SyntaxClass is imported from crate::emacs_core::syntax.

// ---------------------------------------------------------------------------
// Bytecode helpers
// ---------------------------------------------------------------------------

/// Store a 2-byte signed offset at position in bytecode buffer.
fn store_number(buf: &mut [u8], pos: usize, number: i16) {
    let bytes = number.to_le_bytes();
    buf[pos] = bytes[0];
    buf[pos + 1] = bytes[1];
}

/// Read a 2-byte signed offset from bytecode buffer.
fn extract_number(buf: &[u8], pos: usize) -> i16 {
    i16::from_le_bytes([buf[pos], buf[pos + 1]])
}

/// Read a counter value from the counter table, falling back to the bytecode
/// if no override has been stored yet.  Used by `succeed_n`, `jump_n`, and
/// `set_number_at` to emulate GNU's in-place bytecode mutation on immutable
/// bytecode.
fn get_counter(counters: &HashMap<usize, i16>, bytecode: &[u8], pos: usize) -> i16 {
    counters
        .get(&pos)
        .copied()
        .unwrap_or_else(|| extract_number(bytecode, pos))
}

/// Store a counter value in the mutable counter table (keyed by bytecode
/// position).
fn set_counter(counters: &mut HashMap<usize, i16>, pos: usize, val: i16) {
    counters.insert(pos, val);
}

// ---------------------------------------------------------------------------
// Phase 2: Compiler (regex_compile)
//
// Translates GNU Emacs regex-emacs.c:1710-3400 (regex_compile function).
// Compiles an Emacs regex pattern string into bytecode.
// ---------------------------------------------------------------------------

/// Error from regex compilation.
#[derive(Debug, Clone)]
pub(crate) struct RegexCompileError {
    pub message: String,
}

impl std::fmt::Display for RegexCompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// Compile stack entry — tracks open groups during compilation.
/// Mirrors GNU's compile_stack_elt_t.
#[derive(Clone, Debug)]
struct CompileStackEntry {
    /// Bytecode position of the start of the group's alternatives.
    begalt_offset: usize,
    /// Bytecode position of the fixup jump for alternation (or 0).
    fixup_alt_jump: Option<usize>,
    /// Bytecode position of the last expression start (for postfix ops).
    laststart_offset: Option<usize>,
    /// Group number at the time of \( (before incrementing).
    regnum: usize,
    /// The actual group number assigned to this \( (None for shy groups).
    assigned_group: Option<usize>,
    /// Bytecode position of the group's StartMemory (or OnFailureJump
    /// for shy groups). Used by postfix ops like ? * + after \).
    group_bytecode_start: usize,
}

/// Compile an Emacs regex pattern into bytecode.
///
/// This is the main entry point, equivalent to GNU's `regex_compile()`.
///
/// # Arguments
/// * `pattern` - The Emacs regex pattern string
/// * `posix` - If true, use POSIX backtracking semantics
/// * `case_fold` - If true, compile for case-insensitive matching
///
/// # Returns
/// A `CompiledPattern` with bytecode ready for the matcher.
pub(crate) fn regex_compile(
    pattern: &str,
    posix: bool,
    case_fold: bool,
) -> Result<CompiledPattern, RegexCompileError> {
    let pattern = crate::heap_types::LispString::from_utf8(pattern);
    regex_compile_lisp(&pattern, posix, case_fold)
}

pub(crate) fn regex_compile_lisp(
    pattern: &crate::heap_types::LispString,
    posix: bool,
    case_fold: bool,
) -> Result<CompiledPattern, RegexCompileError> {
    let translation = case_fold.then(CaseTranslation::standard);
    regex_compile_lisp_with_translation(pattern, posix, translation)
}

pub(crate) fn regex_compile_lisp_with_translation(
    pattern: &crate::heap_types::LispString,
    posix: bool,
    translation: Option<CaseTranslation>,
) -> Result<CompiledPattern, RegexCompileError> {
    let mut buf = CompiledPattern::new();
    buf.posix = posix;
    buf.multibyte = pattern.is_multibyte();
    buf.target_multibyte = pattern.is_multibyte();
    buf.translate = translation;
    let case_fold = buf.translate.is_some();

    let pattern_bytes = pattern.as_bytes();
    let plen = pattern_bytes.len();
    let mut p = 0; // Current position in pattern

    // Compile stack for tracking open groups
    let mut compile_stack: Vec<CompileStackEntry> = Vec::new();
    let mut regnum: usize = 0; // Current group number

    // Track positions in bytecode for fixup
    let mut begalt_offset: usize = 0; // Start of current alternative
    let mut pending_exact: Option<usize> = None; // Position of current exactn being built
    let mut laststart: Option<usize> = None; // Start of last complete expression (for postfix ops)
    let mut laststart_is_group = false; // True when laststart came from a closed \( ... \).
    let mut fixup_alt_jump: Option<usize> = None; // Jump to fixup at end of alternation

    /// Helper: push a byte to the bytecode buffer
    macro_rules! emit {
        ($byte:expr) => {
            buf.buffer.push($byte);
        };
    }

    /// Helper: push an opcode
    macro_rules! emit_op {
        ($op:expr) => {
            buf.buffer.push($op as u8);
        };
    }

    /// Helper: current bytecode position
    macro_rules! bpos {
        () => {
            buf.buffer.len()
        };
    }

    // Macro to fetch next pattern byte, returning error if at end
    #[allow(unused_macros)]
    macro_rules! pat_fetch {
        () => {{
            if p >= plen {
                return Err(RegexCompileError {
                    message: "premature end of pattern".to_string(),
                });
            }
            let c = pattern_bytes[p];
            p += 1;
            c
        }};
    }

    // Main compilation loop
    while p < plen {
        let c = pattern_bytes[p];
        p += 1;

        match c {
            // ----------------------------------------------------------
            // ^ — beginning of line
            // ----------------------------------------------------------
            b'^' => {
                if !(p == 1 || at_begline_loc_p(pattern_bytes, p)) {
                    goto_normal_char(
                        c as u32,
                        &mut buf,
                        &mut pending_exact,
                        &mut laststart,
                        &mut laststart_is_group,
                    );
                    continue;
                }
                laststart = None;
                laststart_is_group = false;
                pending_exact = None;
                emit_op!(RegexOp::BegLine);
            }

            // ----------------------------------------------------------
            // $ — end of line
            // ----------------------------------------------------------
            b'$' => {
                if !(p == plen || at_endline_loc_p(pattern_bytes, p)) {
                    goto_normal_char(
                        c as u32,
                        &mut buf,
                        &mut pending_exact,
                        &mut laststart,
                        &mut laststart_is_group,
                    );
                    continue;
                }
                laststart = Some(bpos!());
                laststart_is_group = false;
                pending_exact = None;
                emit_op!(RegexOp::EndLine);
            }

            // ----------------------------------------------------------
            // . — any character
            // ----------------------------------------------------------
            b'.' => {
                laststart = Some(bpos!());
                pending_exact = None;
                emit_op!(RegexOp::AnyChar);
            }

            // ----------------------------------------------------------
            // * + ? — repetition operators
            // ----------------------------------------------------------
            b'*' | b'+' | b'?' => {
                let Some(mut last) = laststart else {
                    // No previous expression to repeat — treat as literal
                    goto_normal_char(
                        c as u32,
                        &mut buf,
                        &mut pending_exact,
                        &mut laststart,
                        &mut laststart_is_group,
                    );
                    continue;
                };
                let last_is_group = laststart_is_group;

                // GNU regex_compile: if the preceding expression was part
                // of an exactn with count > 1, split off the last character
                // so that the repetition applies only to that character.
                //
                // Do not split when the postfix applies to a just-closed shy
                // group. GNU clears `pending_exact` on \), so `\(?:ab\)?`
                // repeats the whole "ab" group, not only "b".
                if !last_is_group {
                    last = split_trailing_exactn_atom_if_needed(&mut buf, last);
                }

                // GNU regex-emacs.c: if there is a sequence of repetition
                // chars, collapse it down to just one (the right one).  We
                // track zero_times_ok / many_times_ok / greedy exactly as GNU
                // does so that stacked quantifiers like `a**`, `a*?*`, `a++`,
                // `a???` fold onto the preceding atom instead of being treated
                // as literals.  (Interval operators `\{n,m\}` are NOT folded
                // here, matching GNU.)
                let mut cur = c;
                let mut zero_times_ok = false;
                let mut many_times_ok = false;
                let mut greedy = true;
                loop {
                    if cur == b'?' && (zero_times_ok || many_times_ok) {
                        greedy = false;
                    } else {
                        zero_times_ok |= cur != b'+';
                        many_times_ok |= cur != b'?';
                    }

                    if !(p < plen
                        && (pattern_bytes[p] == b'*'
                            || pattern_bytes[p] == b'+'
                            || pattern_bytes[p] == b'?'))
                    {
                        break;
                    }
                    // Found another repeat character — consume and fold it.
                    cur = pattern_bytes[p];
                    p += 1;
                }

                // Map the collapsed flags back to a single effective postfix
                // operator for `compile_repetition`:
                //   (zero, many) = (T,T) -> `*`, (T,F) -> `?`, (F,T) -> `+`.
                let folded_op = match (zero_times_ok, many_times_ok) {
                    (true, true) => b'*',
                    (true, false) => b'?',
                    (false, true) => b'+',
                    // Unreachable: the first iteration always sets at least one
                    // flag, but fall back to a plain match if it somehow isn't.
                    (false, false) => b'+',
                };

                compile_repetition(folded_op, greedy, posix, last, &mut buf)?;

                laststart = None; // Can't apply another postfix op
                laststart_is_group = false;
                pending_exact = None;
            }

            // ----------------------------------------------------------
            // [ — character class
            // ----------------------------------------------------------
            b'[' => {
                laststart = Some(bpos!());
                pending_exact = None;
                let pattern_multibyte = buf.multibyte;
                compile_charset(
                    pattern_bytes,
                    &mut p,
                    &mut buf,
                    case_fold,
                    pattern_multibyte,
                )?;
            }

            // ----------------------------------------------------------
            // \ — escape sequence
            // ----------------------------------------------------------
            b'\\' => {
                if p >= plen {
                    return Err(RegexCompileError {
                        message: "Trailing backslash".to_string(),
                    });
                }
                let c2 = pattern_bytes[p];
                p += 1;

                match c2 {
                    // \( — start group
                    b'(' => {
                        let mut is_shy = false;
                        let mut explicit_group: Option<usize> = None;
                        if p < plen && pattern_bytes[p] == b'?' {
                            p += 1; // skip ?
                            if p < plen && pattern_bytes[p] == b':' {
                                is_shy = true;
                                p += 1; // skip :
                            } else {
                                let num_start = p;
                                let mut n = 0usize;
                                while p < plen && pattern_bytes[p].is_ascii_digit() {
                                    if p == num_start && pattern_bytes[p] == b'0' {
                                        return Err(RegexCompileError {
                                            message: "Invalid regular expression".to_string(),
                                        });
                                    }
                                    n = n
                                        .checked_mul(10)
                                        .and_then(|value| {
                                            value.checked_add((pattern_bytes[p] - b'0') as usize)
                                        })
                                        .ok_or_else(|| RegexCompileError {
                                            message: "Regular expression too big".to_string(),
                                        })?;
                                    p += 1;
                                }
                                if p == num_start || p >= plen || pattern_bytes[p] != b':' {
                                    return Err(RegexCompileError {
                                        message: "Invalid regular expression".to_string(),
                                    });
                                }
                                explicit_group = Some(n);
                                p += 1; // skip :
                            }
                        }

                        let group_start = bpos!();
                        let assigned = if let Some(n) = explicit_group {
                            Some(n)
                        } else if !is_shy {
                            Some(regnum + 1)
                        } else {
                            None
                        };

                        compile_stack.push(CompileStackEntry {
                            begalt_offset,
                            fixup_alt_jump,
                            laststart_offset: laststart,
                            regnum,
                            assigned_group: assigned,
                            group_bytecode_start: group_start,
                        });

                        if let Some(n) = explicit_group {
                            // Explicit numbered group: assign group number n
                            while buf.re_nsub < n {
                                buf.re_nsub += 1;
                            }
                            regnum = n;
                            emit_op!(RegexOp::StartMemory);
                            emit!(n as u8);
                        } else if !is_shy {
                            regnum += 1;
                            buf.re_nsub += 1;
                            emit_op!(RegexOp::StartMemory);
                            emit!(regnum as u8);
                        }

                        begalt_offset = bpos!();
                        laststart = None;
                        fixup_alt_jump = None;
                        pending_exact = None;
                    }

                    // \) — end group
                    b')' => {
                        let Some(entry) = compile_stack.pop() else {
                            return Err(RegexCompileError {
                                message: "Unmatched ) or \\)".to_string(),
                            });
                        };

                        // Handle pending alternation fixup
                        if let Some(fixup) = fixup_alt_jump {
                            let target = bpos!() as i16 - fixup as i16 - 2;
                            store_number(&mut buf.buffer, fixup, target);
                        }

                        // Emit StopMemory for non-shy groups.
                        if let Some(group_num) = entry.assigned_group {
                            emit_op!(RegexOp::StopMemory);
                            emit!(group_num as u8);
                        }

                        begalt_offset = entry.begalt_offset;
                        fixup_alt_jump = entry.fixup_alt_jump;
                        // After \), laststart points to the group's start
                        // so postfix operators (?, *, +) apply to the group.
                        laststart = Some(entry.group_bytecode_start);
                        laststart_is_group = true;
                        // Do NOT restore regnum — it keeps incrementing
                        // across sibling groups (GNU behavior).
                        pending_exact = None;
                    }

                    // \| — alternation
                    b'|' => {
                        pending_exact = None;

                        // Emit jump past the next alternative
                        emit_op!(RegexOp::Jump);
                        let jump_pos = bpos!();
                        emit!(0);
                        emit!(0); // placeholder offset

                        // Fixup previous alternative's failure jump
                        if let Some(fixup) = fixup_alt_jump {
                            let target = bpos!() as i16 - fixup as i16 - 2;
                            store_number(&mut buf.buffer, fixup, target);
                        }

                        // Insert on_failure_jump at the start of the current alt
                        let alt_start = begalt_offset;
                        // We need to insert 3 bytes at alt_start
                        buf.buffer
                            .splice(alt_start..alt_start, [RegexOp::OnFailureJump as u8, 0, 0]);
                        // The failure jump target is right after the jump we just emitted
                        let target = (bpos!() - alt_start - 3) as i16;
                        store_number(&mut buf.buffer, alt_start + 1, target);

                        // Adjust jump_pos since we inserted 3 bytes
                        fixup_alt_jump = Some(jump_pos + 3);

                        begalt_offset = bpos!();
                        laststart = None;
                    }

                    // \` — beginning of buffer
                    b'`' => {
                        laststart = Some(bpos!());
                        pending_exact = None;
                        emit_op!(RegexOp::BegBuf);
                    }

                    // \' — end of buffer
                    b'\'' => {
                        laststart = Some(bpos!());
                        pending_exact = None;
                        emit_op!(RegexOp::EndBuf);
                    }

                    // \= — at point
                    b'=' => {
                        laststart = Some(bpos!());
                        pending_exact = None;
                        emit_op!(RegexOp::AtDot);
                    }

                    // \b — word boundary
                    b'b' => {
                        laststart = Some(bpos!());
                        pending_exact = None;
                        buf.uses_syntax = true;
                        emit_op!(RegexOp::WordBound);
                    }

                    // \B — not word boundary
                    b'B' => {
                        laststart = Some(bpos!());
                        pending_exact = None;
                        buf.uses_syntax = true;
                        emit_op!(RegexOp::NotWordBound);
                    }

                    // \< — word beginning
                    b'<' => {
                        laststart = Some(bpos!());
                        pending_exact = None;
                        buf.uses_syntax = true;
                        emit_op!(RegexOp::WordBeg);
                    }

                    // \> — word end
                    b'>' => {
                        laststart = Some(bpos!());
                        pending_exact = None;
                        buf.uses_syntax = true;
                        emit_op!(RegexOp::WordEnd);
                    }

                    // \_ — symbol boundary
                    b'_' => {
                        if p >= plen {
                            return Err(RegexCompileError {
                                message: "Premature end of regular expression".to_string(),
                            });
                        }
                        let c3 = pattern_bytes[p];
                        p += 1;
                        match c3 {
                            b'<' => {
                                laststart = Some(bpos!());
                                pending_exact = None;
                                buf.uses_syntax = true;
                                emit_op!(RegexOp::SymBeg);
                            }
                            b'>' => {
                                laststart = Some(bpos!());
                                pending_exact = None;
                                buf.uses_syntax = true;
                                emit_op!(RegexOp::SymEnd);
                            }
                            _ => {
                                return Err(RegexCompileError {
                                    message: "Invalid regular expression".to_string(),
                                });
                            }
                        }
                    }

                    // \w — word constituent (syntax-table aware)
                    b'w' => {
                        laststart = Some(bpos!());
                        pending_exact = None;
                        buf.uses_syntax = true;
                        emit_op!(RegexOp::SyntaxSpec);
                        emit!(u8::from(SyntaxClass::Word));
                    }

                    // \W — not word constituent
                    b'W' => {
                        laststart = Some(bpos!());
                        pending_exact = None;
                        buf.uses_syntax = true;
                        emit_op!(RegexOp::NotSyntaxSpec);
                        emit!(u8::from(SyntaxClass::Word));
                    }

                    // \sC — syntax class C
                    b's' => {
                        if p >= plen {
                            return Err(RegexCompileError {
                                message: "Premature end of regular expression".to_string(),
                            });
                        }
                        let sc = syntax_spec_code(pattern_bytes[p]);
                        p += 1;
                        laststart = Some(bpos!());
                        pending_exact = None;
                        buf.uses_syntax = true;
                        emit_op!(RegexOp::SyntaxSpec);
                        emit!(sc);
                    }

                    // \SC — not syntax class C
                    b'S' => {
                        if p >= plen {
                            return Err(RegexCompileError {
                                message: "Premature end of regular expression".to_string(),
                            });
                        }
                        let sc = syntax_spec_code(pattern_bytes[p]);
                        p += 1;
                        laststart = Some(bpos!());
                        pending_exact = None;
                        buf.uses_syntax = true;
                        emit_op!(RegexOp::NotSyntaxSpec);
                        emit!(sc);
                    }

                    // \cC — category C
                    b'c' => {
                        if p >= plen {
                            return Err(RegexCompileError {
                                message: "\\c requires category character".to_string(),
                            });
                        }
                        let cat = pattern_bytes[p];
                        p += 1;
                        laststart = Some(bpos!());
                        pending_exact = None;
                        emit_op!(RegexOp::CategorySpec);
                        emit!(cat);
                    }

                    // \CC — not category C
                    b'C' => {
                        if p >= plen {
                            return Err(RegexCompileError {
                                message: "\\C requires category character".to_string(),
                            });
                        }
                        let cat = pattern_bytes[p];
                        p += 1;
                        laststart = Some(bpos!());
                        pending_exact = None;
                        emit_op!(RegexOp::NotCategorySpec);
                        emit!(cat);
                    }

                    // \1-\9 — backreference
                    b'1'..=b'9' => {
                        let group = (c2 - b'0') as usize;
                        if group > buf.re_nsub
                            || compile_stack
                                .iter()
                                .any(|entry| entry.assigned_group == Some(group))
                        {
                            return Err(RegexCompileError {
                                message: "Invalid back reference".to_string(),
                            });
                        }
                        laststart = Some(bpos!());
                        pending_exact = None;
                        emit_op!(RegexOp::Duplicate);
                        emit!(group as u8);
                    }

                    // \{ — interval \{n,m\}
                    b'{' => {
                        // Parse interval
                        let interval_start = p;
                        let (min_count, max_count) = parse_interval(pattern_bytes, &mut p)?;

                        let Some(mut last) = laststart else {
                            // GNU regex-emacs.c:2427 `unfetch_interval`: a
                            // syntactically valid interval without a preceding
                            // atom is literal text beginning with `{`.
                            p = interval_start;
                            goto_normal_char(
                                b'{' as u32,
                                &mut buf,
                                &mut pending_exact,
                                &mut laststart,
                                &mut laststart_is_group,
                            );
                            continue;
                        };
                        if !laststart_is_group {
                            last = split_trailing_exactn_atom_if_needed(&mut buf, last);
                        }

                        compile_interval(min_count, max_count, last, &mut buf)?;
                        laststart = Some(last);
                        laststart_is_group = false;
                        pending_exact = None;
                    }

                    // Other escaped characters — treat as literal
                    _ => {
                        if buf.multibyte && c2 >= 0x80 {
                            let char_start = p - 1;
                            let (code, len) = decode_pattern_char(pattern_bytes, char_start, true)
                                .unwrap_or((c2 as u32, 1));
                            p = char_start + len;
                            goto_normal_char(
                                code,
                                &mut buf,
                                &mut pending_exact,
                                &mut laststart,
                                &mut laststart_is_group,
                            );
                        } else {
                            goto_normal_char(
                                c2 as u32,
                                &mut buf,
                                &mut pending_exact,
                                &mut laststart,
                                &mut laststart_is_group,
                            );
                        }
                    }
                }
            }

            // ----------------------------------------------------------
            // Normal character — add to exactn
            // ----------------------------------------------------------
            _ => {
                if buf.multibyte && c >= 0x80 {
                    let char_start = p - 1;
                    let (code, len) = decode_pattern_char(pattern_bytes, char_start, true)
                        .unwrap_or((c as u32, 1));
                    p = char_start + len;
                    goto_normal_char(
                        code,
                        &mut buf,
                        &mut pending_exact,
                        &mut laststart,
                        &mut laststart_is_group,
                    );
                } else {
                    goto_normal_char(
                        c as u32,
                        &mut buf,
                        &mut pending_exact,
                        &mut laststart,
                        &mut laststart_is_group,
                    );
                }
            }
        }
    }

    // Check for unmatched \(
    if !compile_stack.is_empty() {
        return Err(RegexCompileError {
            message: "Unmatched ( or \\(".to_string(),
        });
    }

    // Handle final alternation fixup
    if let Some(fixup) = fixup_alt_jump {
        let target = bpos!() as i16 - fixup as i16 - 2;
        store_number(&mut buf.buffer, fixup, target);
    }

    // Emit final succeed — but only for non-POSIX patterns.
    //
    // GNU regex-emacs.c:2683-2686:
    //
    //     /* If we don't want backtracking, force success
    //        the first time we reach the end of the compiled pattern.  */
    //     if (!posix_backtracking)
    //       BUF_PUSH (succeed);
    //
    // When `posix_backtracking` is true the matcher must see the
    // natural "fell off the end of the bytecode" path so the POSIX
    // longest-match logic at regex-emacs.c:4272-4344 can run. Emitting
    // `succeed` unconditionally (as an earlier version of this file
    // did) made every pattern jump to `succeed_label`, bypassing the
    // longest-match code entirely.
    if !posix {
        emit_op!(RegexOp::Succeed);
    }

    // Populate the fastmap for search-time position skipping.
    compile_fastmap(&mut buf);

    Ok(buf)
}

// ---------------------------------------------------------------------------
// Compiler Helpers
// ---------------------------------------------------------------------------

/// GNU `regex-emacs.c:2765` context check for `^`.
///
/// `p` points just after the `^` byte in `pattern`.  GNU treats `^` as a
/// beginning-of-line assertion only at pattern start, after an alternative
/// (`\|`), or after an opening group (`\(` / `\(?:` / `\(?N:`).  Everywhere
/// else it is a literal character.
fn at_begline_loc_p(pattern: &[u8], p: usize) -> bool {
    if p < 2 {
        return false;
    }

    let mut prev = p - 2;
    match pattern[prev] {
        b'(' | b'|' => {}
        b':' => {
            while prev > 0 && pattern[prev - 1].is_ascii_digit() {
                prev -= 1;
            }
            if !(prev > 1 && pattern[prev - 1] == b'?' && pattern[prev - 2] == b'(') {
                return false;
            }
            prev -= 2;
        }
        _ => return false,
    }

    let slash_end = prev;
    while prev > 0 && pattern[prev - 1] == b'\\' {
        prev -= 1;
    }
    ((slash_end - prev) & 1) != 0
}

/// GNU `regex-emacs.c:2801` context check for `$`.
///
/// `p` points just after the `$` byte in `pattern`.  `$` is an end-of-line
/// assertion only at pattern end, before a closing group (`\)`) or before an
/// alternative (`\|`).
fn at_endline_loc_p(pattern: &[u8], p: usize) -> bool {
    p + 1 < pattern.len() && pattern[p] == b'\\' && matches!(pattern[p + 1], b')' | b'|')
}

fn syntax_spec_code(c: u8) -> u8 {
    SyntaxClass::from_syntax_spec_byte(c)
        .map(u8::from)
        .unwrap_or(0o377)
}

/// Emit a literal character as part of an `exactn` sequence.
///
/// GNU `regex-emacs.c` applies `RE_TRANSLATE (translate, c)` before
/// buffering the char, so a pattern like `"C"` compiled with
/// case-fold on is stored in the bytecode as `'c'`. At match time the
/// buffer char is also `tr()`-translated, so both sides are
/// case-folded to the canonical (lowercase) form. Without the
/// translate-on-compile step here, the pattern byte stays as `'C'`
/// while the matched text byte becomes `'c'` and they fail to compare
/// equal.
fn goto_normal_char(
    c: u32,
    buf: &mut CompiledPattern,
    pending_exact: &mut Option<usize>,
    laststart: &mut Option<usize>,
    laststart_is_group: &mut bool,
) {
    let c = if let Some(table) = buf.translate.as_ref() {
        table.translate(c)
    } else {
        c
    };

    let mut encoded = [0u8; emacs_char::MAX_MULTIBYTE_LENGTH];
    let encoded_len = if buf.multibyte {
        emacs_char::char_string(c, &mut encoded)
    } else {
        encoded[0] = c as u8;
        1
    };

    // If we have a pending exactn and it hasn't reached max length (255),
    // just append to it
    if let Some(exact_pos) = *pending_exact {
        let count = buf.buffer[exact_pos] as usize;
        if count + encoded_len <= 255 {
            buf.buffer[exact_pos] += encoded_len as u8;
            buf.buffer.extend_from_slice(&encoded[..encoded_len]);
            *laststart_is_group = false;
            return;
        }
    }

    // Start a new exactn
    *laststart = Some(buf.buffer.len());
    *laststart_is_group = false;
    buf.buffer.push(RegexOp::Exactn as u8);
    *pending_exact = Some(buf.buffer.len());
    buf.buffer.push(encoded_len as u8);
    buf.buffer.extend_from_slice(&encoded[..encoded_len]);
}

/// Split the final character out of a multi-character `exactn` atom.
///
/// GNU `regex-emacs.c` avoids building one `exactn` across a character
/// followed by a postfix or interval operator (see the `normal_char`
/// check for `*`, `+`, `?`, and `\{`). Since this Rust compiler may
/// already have coalesced adjacent literal characters, split lazily
/// before compiling the repeat so `ab\{0,1\}` repeats only `b`, not
/// the whole `ab`.
fn split_trailing_exactn_atom_if_needed(buf: &mut CompiledPattern, laststart: usize) -> usize {
    if buf.buffer.get(laststart).copied() != Some(RegexOp::Exactn as u8) {
        return laststart;
    }

    let count_pos = laststart + 1;
    let Some(&count_byte) = buf.buffer.get(count_pos) else {
        return laststart;
    };
    let count = count_byte as usize;
    let exact_start = count_pos + 1;
    let exact_end = exact_start + count;
    if exact_end > buf.buffer.len() {
        return laststart;
    }
    let exact_bytes = &buf.buffer[exact_start..exact_end];
    let last_char_start = if buf.multibyte {
        let mut rel = 0;
        let mut previous = 0;
        let mut chars = 0;
        while rel < exact_bytes.len() {
            previous = rel;
            let (_, len) = emacs_char::string_char(&exact_bytes[rel..]);
            rel += len;
            chars += 1;
        }
        if chars > 1 { Some(previous) } else { None }
    } else if count > 1 {
        Some(count - 1)
    } else {
        None
    };

    let Some(split_start) = last_char_start else {
        return laststart;
    };

    let split_bytes = exact_bytes[split_start..].to_vec();
    buf.buffer.truncate(exact_start + split_start);
    buf.buffer[count_pos] = split_start as u8;
    let split_atom = buf.buffer.len();
    buf.buffer.push(RegexOp::Exactn as u8);
    buf.buffer.push(split_bytes.len() as u8);
    buf.buffer.extend_from_slice(&split_bytes);
    split_atom
}

/// Compile a repetition operator (*, +, ?).
///
/// Inserts jump opcodes around the preceding expression to implement
/// the repetition. Mirrors GNU's handling in regex_compile cases '*', '+', '?'.
fn compile_repetition(
    op: u8,
    greedy: bool,
    _posix: bool,
    laststart: usize,
    buf: &mut CompiledPattern,
) -> Result<(), RegexCompileError> {
    // All offsets are relative to the position right after the 2-byte offset
    // field.  This matches GNU's convention: after EXTRACT_NUMBER_AND_INCR,
    // `p` points past the offset, and the target is `p + mcnt`.

    let after_last = buf.buffer.len();

    match op {
        b'*' => {
            // * = zero or more
            if greedy {
                // Layout:
                //   [laststart] OFJL  offset(2)  <expr>  Jump  offset(2)
                //   OFJL fail target → past the Jump instruction
                //   Jump target → back to OFJL opcode

                // Insert OnFailureJumpLoop before the expression
                buf.buffer.splice(
                    laststart..laststart,
                    [RegexOp::OnFailureJumpLoop as u8, 0, 0],
                );
                // After splice, expr occupies [laststart+3 .. laststart+3+expr_len)
                let expr_len = after_last - laststart; // original expr length

                // Add Jump back to the OFJL
                buf.buffer.push(RegexOp::Jump as u8);
                let jpos = buf.buffer.len();
                buf.buffer.push(0);
                buf.buffer.push(0);

                // OFJL fail target: from (laststart+3) → past Jump = (jpos+2)
                // offset = (jpos+2) - (laststart+3) = expr_len + 3
                let ofjl_offset = (expr_len + 3) as i16;
                store_number(&mut buf.buffer, laststart + 1, ofjl_offset);

                // Jump target: from (jpos+2) → OFJL opcode at laststart
                // offset = laststart - (jpos + 2)
                let jump_offset = laststart as i16 - (jpos as i16 + 2);
                store_number(&mut buf.buffer, jpos, jump_offset);
            } else {
                // GNU `regex-emacs.c` compiles non-greedy `*?` as:
                //
                //   jump cond
                // loop:
                //   <expr>
                //   [no-op when expr may match empty]
                // cond:
                //   on_failure_jump[_nastyloop] loop
                //
                // This tries zero iterations first and only falls back
                // into the loop body when a later piece fails.
                let expr_bytes = buf.buffer[laststart..after_last].to_vec();
                let body_may_be_empty = repeated_body_may_match_empty(&expr_bytes);

                buf.buffer.truncate(laststart);

                buf.buffer.push(RegexOp::Jump as u8);
                let jump_pos = buf.buffer.len();
                buf.buffer.push(0);
                buf.buffer.push(0);

                let expr_start = buf.buffer.len();
                buf.buffer.extend_from_slice(&expr_bytes);
                if body_may_be_empty {
                    buf.buffer.push(RegexOp::NoOp as u8);
                }

                let cond_pos = buf.buffer.len();
                buf.buffer.push(if body_may_be_empty {
                    RegexOp::OnFailureJumpNastyloop as u8
                } else {
                    RegexOp::OnFailureJump as u8
                });
                let cond_arg_pos = buf.buffer.len();
                buf.buffer.push(0);
                buf.buffer.push(0);

                // Initial jump skips directly to the conditional branch.
                let jump_offset = cond_pos as i16 - (jump_pos as i16 + 2);
                store_number(&mut buf.buffer, jump_pos, jump_offset);

                // The conditional branch backtracks into the loop body.
                let cond_offset = expr_start as i16 - (cond_arg_pos as i16 + 2);
                store_number(&mut buf.buffer, cond_arg_pos, cond_offset);
            }
        }
        b'+' => {
            // + = one or more
            // Layout: <expr(already emitted)>  OFJL/OFJ  offset(2)  Jump  offset(2)
            if greedy {
                // OFJL fail target → past the Jump instruction (continue)
                buf.buffer.push(RegexOp::OnFailureJumpLoop as u8);
                let ofjl_pos = buf.buffer.len();
                buf.buffer.push(0);
                buf.buffer.push(0);

                buf.buffer.push(RegexOp::Jump as u8);
                let jpos = buf.buffer.len();
                buf.buffer.push(0);
                buf.buffer.push(0);

                // OFJL fail: from (ofjl_pos+2) → (jpos+2)
                store_number(&mut buf.buffer, ofjl_pos, (jpos + 2 - ofjl_pos - 2) as i16);

                // Jump target: from (jpos+2) → laststart (start of expr)
                let jump_offset = laststart as i16 - (jpos as i16 + 2);
                store_number(&mut buf.buffer, jpos, jump_offset);
            } else {
                // GNU `regex-emacs.c:1987-1999`: non-greedy `+?`
                // matches one body copy, then uses the same
                // "repeat until fail" conditional jump as `*?`.
                let expr_bytes = buf.buffer[laststart..after_last].to_vec();
                let body_may_be_empty = repeated_body_may_match_empty(&expr_bytes);

                buf.buffer.truncate(laststart);
                let expr_start = buf.buffer.len();
                buf.buffer.extend_from_slice(&expr_bytes);
                if body_may_be_empty {
                    buf.buffer.push(RegexOp::NoOp as u8);
                }

                buf.buffer.push(if body_may_be_empty {
                    RegexOp::OnFailureJumpNastyloop as u8
                } else {
                    RegexOp::OnFailureJump as u8
                });
                let cond_arg_pos = buf.buffer.len();
                buf.buffer.push(0);
                buf.buffer.push(0);

                let cond_offset = expr_start as i16 - (cond_arg_pos as i16 + 2);
                store_number(&mut buf.buffer, cond_arg_pos, cond_offset);
            }
        }
        b'?' => {
            // ? = zero or one
            if greedy {
                // Layout: [laststart] OFJ  offset(2)  <expr>
                // OFJ fail target → past expr
                buf.buffer
                    .splice(laststart..laststart, [RegexOp::OnFailureJump as u8, 0, 0]);
                let expr_len = after_last - laststart;
                // From (laststart+3) → (laststart+3+expr_len), offset = expr_len
                store_number(&mut buf.buffer, laststart + 1, expr_len as i16);
            } else {
                // Non-greedy ??
                buf.buffer.splice(
                    laststart..laststart,
                    [RegexOp::OnFailureKeepStringJump as u8, 0, 0],
                );
                let expr_len = after_last - laststart;
                store_number(&mut buf.buffer, laststart + 1, expr_len as i16);
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}

/// GNU's non-greedy `*?` / `+?` loops use the "repeat until fail"
/// layout from `src/regex-emacs.c:1980-2006`, not the eager
/// `on_failure_keep_string_jump` prefix form used for some greedy
/// optimizations. We only need to know whether the repeated body
/// could match the empty string in order to pick between
/// `on_failure_jump` and `on_failure_jump_nastyloop`.
///
/// A full `analyze_first` port would be ideal, but a conservative
/// first-opcode check is sufficient here: return `false` only for
/// obviously consuming atoms. Everything else falls back to the
/// nastyloop opcode, which is slower but semantics-safe.
fn repeated_body_may_match_empty(body: &[u8]) -> bool {
    let Some(&op) = body.first() else {
        return true;
    };

    !matches!(
        RegexOp::from_byte(op),
        Some(
            RegexOp::Exactn
                | RegexOp::AnyChar
                | RegexOp::Charset
                | RegexOp::CharsetNot
                | RegexOp::SyntaxSpec
                | RegexOp::NotSyntaxSpec
                | RegexOp::CategorySpec
                | RegexOp::NotCategorySpec
        )
    )
}

/// Decode one Emacs character from a pattern byte slice starting at `pos`.
///
/// Multibyte patterns use Emacs internal encoding; unibyte patterns map each
/// byte to a single character code directly.
fn decode_pattern_char(bytes: &[u8], pos: usize, multibyte: bool) -> Option<(u32, usize)> {
    if pos >= bytes.len() {
        return None;
    }
    if multibyte {
        Some(emacs_char::string_char(&bytes[pos..]))
    } else {
        Some((bytes[pos] as u32, 1))
    }
}

fn emacs_char_to_rust_char(code: u32) -> char {
    if emacs_char::char_byte8_p(code) {
        char::from(emacs_char::char_to_byte8(code))
    } else {
        char::from_u32(code).unwrap_or(char::REPLACEMENT_CHARACTER)
    }
}

/// Compile a character class `[...]` into charset bytecode.
/// POSIX named character class kind, returned by `parse_posix_char_class`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PosixCharClassKind {
    Word,
    Space,
    Upper,
    Lower,
    Alpha,
    Alnum,
    Digit,
    Xdigit,
    Punct,
    Graph,
    Print,
    Blank,
    Cntrl,
    Ascii,
    Unibyte,
    NonAscii,
    Multibyte,
    /// `[:` was found but the name is not a valid class.
    Error,
}

impl PosixCharClassKind {
    fn name(self) -> Option<&'static str> {
        match self {
            Self::Word => Some("word"),
            Self::Space => Some("space"),
            Self::Upper => Some("upper"),
            Self::Lower => Some("lower"),
            Self::Alpha => Some("alpha"),
            Self::Alnum => Some("alnum"),
            Self::Digit => Some("digit"),
            Self::Xdigit => Some("xdigit"),
            Self::Punct => Some("punct"),
            Self::Graph => Some("graph"),
            Self::Print => Some("print"),
            Self::Blank => Some("blank"),
            Self::Cntrl => Some("cntrl"),
            Self::Ascii => Some("ascii"),
            Self::Unibyte => Some("unibyte"),
            Self::NonAscii => Some("nonascii"),
            Self::Multibyte => Some("multibyte"),
            Self::Error => None,
        }
    }
}

struct PosixCharClass {
    kind: PosixCharClassKind,
    byte_len: usize,
}

/// Parse a POSIX named character class `[:name:]` at `pattern[pos..]`.
/// Returns `None` if `pattern[pos..]` doesn't start with `[:`.
fn parse_posix_char_class(pattern: &[u8], pos: usize, plen: usize) -> Option<PosixCharClass> {
    if pos + 4 > plen {
        return None;
    }
    if pattern[pos] != b'[' || pattern[pos + 1] != b':' {
        return None;
    }
    // Find closing ":]"
    let name_start = pos + 2;
    let mut end = name_start;
    while end + 1 < plen {
        if pattern[end] == b':' && pattern[end + 1] == b']' {
            break;
        }
        end += 1;
    }
    if end + 1 >= plen {
        // No closing ":]" found — treat "[:" as literal characters
        return None;
    }
    let name = &pattern[name_start..end];
    let kind = match name {
        b"word" => PosixCharClassKind::Word,
        b"alnum" => PosixCharClassKind::Alnum,
        b"alpha" => PosixCharClassKind::Alpha,
        b"space" => PosixCharClassKind::Space,
        b"digit" => PosixCharClassKind::Digit,
        b"blank" => PosixCharClassKind::Blank,
        b"upper" => PosixCharClassKind::Upper,
        b"lower" => PosixCharClassKind::Lower,
        b"punct" => PosixCharClassKind::Punct,
        b"ascii" => PosixCharClassKind::Ascii,
        b"graph" => PosixCharClassKind::Graph,
        b"print" => PosixCharClassKind::Print,
        b"cntrl" => PosixCharClassKind::Cntrl,
        b"xdigit" => PosixCharClassKind::Xdigit,
        b"unibyte" => PosixCharClassKind::Unibyte,
        b"nonascii" => PosixCharClassKind::NonAscii,
        b"multibyte" => PosixCharClassKind::Multibyte,
        _ => PosixCharClassKind::Error,
    };
    Some(PosixCharClass {
        kind,
        byte_len: end + 2 - pos, // include ":]" closing
    })
}

fn compile_charset(
    pattern: &[u8],
    p: &mut usize,
    buf: &mut CompiledPattern,
    case_fold: bool,
    pattern_multibyte: bool,
) -> Result<(), RegexCompileError> {
    let plen = pattern.len();
    if *p >= plen {
        return Err(RegexCompileError {
            message: "Unmatched [ or [^".to_string(),
        });
    }

    // Check for negation
    let negate = *p < plen && pattern[*p] == b'^';
    if negate {
        *p += 1;
        if *p >= plen {
            return Err(RegexCompileError {
                message: "Unmatched [ or [^".to_string(),
            });
        }
    }

    let op = if negate {
        RegexOp::CharsetNot
    } else {
        RegexOp::Charset
    };

    // Record the bytecode position of this charset opcode for the
    // multibyte_charsets map.
    let charset_opcode_pos = buf.buffer.len();
    buf.buffer.push(op as u8);
    let _bitmap_len_pos = buf.buffer.len();
    buf.buffer.push(32); // 256 bits = 32 bytes bitmap

    // Initialize 32-byte bitmap (256 bits, one per ASCII char)
    let bitmap_start = buf.buffer.len();
    buf.buffer.extend_from_slice(&[0u8; 32]);

    // Collect multibyte (non-ASCII) ranges for this charset.
    let mut mb_ranges: Vec<(char, char)> = Vec::new();

    // Bitmask of class flags for `[[:word:]]` / `[[:space:]]`. The
    // matcher checks these against the buffer syntax table at run
    // time so per-mode word/space definitions take effect.
    let mut class_bits: u32 = 0;

    // Special case: ] at start is literal
    let mut first = true;
    let mut pending_char: Option<char> = None;
    let mut closed = false;

    while *p < plen {
        // GNU `re_wctype_parse`: before reading a character, check if
        // we're at `[:name:]` — a POSIX named character class inside
        // the bracket expression.  The `]` in the closing `:]` must
        // not close the outer `[...]`.
        if let Some(cc) = parse_posix_char_class(pattern, *p, plen) {
            if let Some(c) = pending_char.take() {
                add_charset_char(
                    &mut buf.buffer,
                    bitmap_start,
                    c,
                    case_fold,
                    &mut mb_ranges,
                    buf.translate.as_ref(),
                );
            }
            *p += cc.byte_len;
            let Some(class_name) = cc.kind.name() else {
                return Err(RegexCompileError {
                    message: "Invalid character class name".to_string(),
                });
            };
            apply_posix_class(
                class_name,
                &mut buf.buffer,
                bitmap_start,
                &mut mb_ranges,
                &mut class_bits,
                buf.translate.as_ref(),
            )?;
            if *p >= plen {
                return Err(RegexCompileError {
                    message: "Unmatched [ or [^".to_string(),
                });
            }
            // Mark that we've consumed a character (prevents `]` from
            // being treated as literal at position 0).
            first = false;
            pending_char = None;
            continue;
        }

        let b = pattern[*p];

        // Decode a full Emacs character from the pattern.
        let (c, clen) =
            decode_pattern_char(pattern, *p, pattern_multibyte).unwrap_or((b as u32, 1));
        *p += clen;

        if b == b']' && !first {
            closed = true;
            break;
        }
        first = false;

        if b == b'-' && *p < plen && pattern[*p] != b']' {
            if let Some(range_start) = pending_char.take() {
                // Range: range_start - next_char
                let (range_end, rlen) = decode_pattern_char(pattern, *p, pattern_multibyte)
                    .unwrap_or((pattern[*p] as u32, 1));
                let range_end = emacs_char_to_rust_char(range_end);
                *p += rlen;
                let translate = buf.translate.as_ref();
                if range_start <= range_end {
                    if range_start.is_ascii() && range_end.is_ascii() {
                        // Both ASCII — use the bitmap
                        for ch in (range_start as u8)..=(range_end as u8) {
                            set_bitmap_bit(&mut buf.buffer, bitmap_start, ch, translate);
                        }
                    } else {
                        // At least one endpoint is non-ASCII.
                        // Put the ASCII portion in the bitmap and the rest in
                        // multibyte ranges.
                        let start_u32 = range_start as u32;
                        let end_u32 = range_end as u32;
                        // ASCII portion: codepoints <= 127
                        if start_u32 <= 127 {
                            let ascii_end = end_u32.min(127) as u8;
                            for ch in (start_u32 as u8)..=ascii_end {
                                set_bitmap_bit(&mut buf.buffer, bitmap_start, ch, translate);
                            }
                        }
                        // Multibyte portion: codepoints >= 128
                        let mb_start = if start_u32 >= 128 {
                            range_start
                        } else {
                            '\u{80}'
                        };
                        if end_u32 >= 128 {
                            add_multibyte_range(&mut mb_ranges, mb_start, range_end, case_fold);
                        }
                    }
                }
                continue;
            }
            // '-' at start or after a range → literal '-'
            pending_char = Some('-');
            continue;
        }

        // GNU `regex-emacs.c` treats backslash as a literal character
        // inside a bracket expression: the parser at lines 2055-2140
        // has no escape handling in the `[...]` loop, so `[\w]` is
        // the character class containing `\` and `w`, and `\n` is
        // the class containing `\` and `n`. Users who want a word
        // character class inside a bracket expression must use the
        // POSIX class `[[:word:]]`.
        if b == b'\\' {
            if let Some(c) = pending_char.take() {
                add_charset_char(
                    &mut buf.buffer,
                    bitmap_start,
                    c,
                    case_fold,
                    &mut mb_ranges,
                    buf.translate.as_ref(),
                );
            }
            pending_char = Some('\\');
            continue;
        }

        // POSIX named character classes (`[:alpha:]`, etc.) are now
        // handled by `parse_posix_char_class` at the top of the
        // loop, before the per-character decode.  That function
        // does not advance `p` when the `[:` prefix is a false
        // start, so a literal `[` inside a bracket expression
        // (e.g. `[[`, `[a[:c]`) is treated correctly — it falls
        // through to the character-level processing below.
        //
        // The previous inline handler that used to live here
        // unconditionally advanced `p` past the pattern end when
        // `[:` was not followed by a valid class name, causing
        // spurious "Unmatched [" errors.

        // Regular character
        let c = emacs_char_to_rust_char(c);
        if let Some(prev) = pending_char.take() {
            add_charset_char(
                &mut buf.buffer,
                bitmap_start,
                prev,
                case_fold,
                &mut mb_ranges,
                buf.translate.as_ref(),
            );
        }
        pending_char = Some(c);
    }

    if !closed {
        return Err(RegexCompileError {
            message: "Unmatched [ or [^".to_string(),
        });
    }

    if let Some(c) = pending_char.take() {
        add_charset_char(
            &mut buf.buffer,
            bitmap_start,
            c,
            case_fold,
            &mut mb_ranges,
            buf.translate.as_ref(),
        );
    }

    // Store multibyte ranges if any were collected.
    if !mb_ranges.is_empty() {
        buf.multibyte_charsets.insert(charset_opcode_pos, mb_ranges);
    }

    // Record class flags so the matcher can consult the buffer
    // syntax table at run time for `[[:word:]]` and `[[:space:]]`.
    if class_bits != 0 {
        buf.uses_syntax = true;
        buf.charset_class_bits
            .insert(charset_opcode_pos, class_bits);
    }

    Ok(())
}

fn add_charset_char(
    buffer: &mut Vec<u8>,
    bitmap_start: usize,
    c: char,
    case_fold: bool,
    mb_ranges: &mut Vec<(char, char)>,
    translate: Option<&CaseTranslation>,
) {
    if c.is_ascii() {
        set_bitmap_bit(buffer, bitmap_start, c as u8, translate);
    } else {
        add_multibyte_range(mb_ranges, c, c, case_fold);
    }
}

/// Add a multibyte character range, optionally expanding for case-folding.
fn add_multibyte_range(ranges: &mut Vec<(char, char)>, start: char, end: char, case_fold: bool) {
    ranges.push((start, end));
    if case_fold {
        // For case-folding, also add the upper/lower-case variants.
        // For single-char ranges, just add the case-folded char.
        // For multi-char ranges, this is a conservative approximation:
        // we add the lowercased and uppercased versions of the endpoints.
        if start == end {
            for variant in start.to_lowercase() {
                if variant != start {
                    ranges.push((variant, variant));
                }
            }
            for variant in start.to_uppercase() {
                if variant != start {
                    ranges.push((variant, variant));
                }
            }
        }
        // For multi-char ranges (start != end), the range itself should
        // cover the needed codepoints in most cases. We don't expand
        // further to avoid combinatorial explosion.
    }
}

/// Set a bit in the charset bitmap, translating through TRANSLATE if
/// supplied.
///
/// GNU `regex-emacs.c:SETUP_ASCII_RANGE` (lines 1397-1412) runs
/// `C1 = TRANSLATE(C0)` and then `SET_LIST_BIT(C1)` — it translates
/// each individual character as the range is walked and only stores
/// the translated bit. The matcher at regex-emacs.c:4553 does the
/// same TRANSLATE on the input character before the bitmap lookup,
/// so matches work out for any case-equivalent input.
///
/// Earlier versions of this function instead set the bit for both
/// the raw character and its Rust-derived upper/lower partners,
/// regardless of what translate table the pattern was compiled with.
/// That was audit finding #9 in `drafts/regex-search-audit.md`:
/// "charset case-fold range translation is eager (not lazy)". The
/// practical difference only shows up when Rust's Unicode case
/// mapping disagrees with Emacs's case canon table, but the GNU-
/// parity fix is to consult the same translate table both sides.
fn set_bitmap_bit(
    buffer: &mut Vec<u8>,
    bitmap_start: usize,
    c: u8,
    translate: Option<&CaseTranslation>,
) {
    let target = match translate {
        Some(table) => table.translate_byte(c),
        None => c,
    };
    let byte_idx = bitmap_start + (target as usize / 8);
    let bit_idx = target as usize % 8;
    if byte_idx < buffer.len() {
        buffer[byte_idx] |= 1 << bit_idx;
    }
}

/// Apply a POSIX character class to the bitmap and multibyte range list.
///
/// Mirrors GNU `regex-emacs.c:re_wctype_parse` (lines 1525-1601) and
/// `re_iswctype` (lines 1603-1630). The full set of 17 classes is:
/// `alnum`, `alpha`, `blank`, `cntrl`, `digit`, `graph`, `lower`,
/// `print`, `punct`, `space`, `upper`, `xdigit`, `ascii`, `word`,
/// `nonascii`, `unibyte`, `multibyte`.
///
/// Semantics are taken from GNU's header macros at `regex-emacs.c:98-153`:
///
/// - `IS_REAL_ASCII(c)` is `c < 0x80`.
/// - `ISBLANK(c)` for ASCII is `c == ' ' || c == '\t'` only
///   (space and tab; NOT newline, formfeed, carriage return).
/// - `ISSPACE(c)` is `BUFFER_SYNTAX(c) == Swhitespace`; GNU's default
///   standard syntax table treats space, tab, newline, formfeed, and
///   carriage return as whitespace.
/// - `ISGRAPH(c)` for single-byte is `c > ' '` AND NOT in
///   `[0x7F..=0xA0]`.
/// - `ISPRINT(c)` for single-byte is `c >= ' '` AND NOT in
///   `[0x7F..=0x9F]`.
/// - `ISWORD(c)` is `BUFFER_SYNTAX(c) == Sword`; GNU's default treats
///   ASCII letters and digits as word constituents.
/// - `IS_REAL_ASCII(c)` covers 0x00..=0x7F for `ascii`.
/// - `nonascii` = `!IS_REAL_ASCII(c)` (>= 0x80).
/// - `unibyte` matches any single-byte character (bytes 0x00..=0xFF
///   in the bitmap, plus 8-bit raw byte chars).
/// - `multibyte` = `!ISUNIBYTE(c)`; matches multibyte characters
///   only (non-ASCII range via the multibyte range list).
///
/// Unknown class names mirror GNU's `RECC_ERROR` (regex-emacs.c:1600,
/// consumed as `REG_ECTYPE` at line 2071). We signal the same error
/// rather than silently ignoring the class as before.
///
/// Note: `word` and `space` semantically depend on the buffer's
/// syntax table (see audit finding #8 in
/// `drafts/regex-search-audit.md`). For now we bake in the standard
/// default; threading the per-buffer syntax table through charset
/// compilation is tracked as audit #8.
fn apply_posix_class(
    name: &str,
    buffer: &mut Vec<u8>,
    bitmap_start: usize,
    mb_ranges: &mut Vec<(char, char)>,
    class_bits: &mut u32,
    translate: Option<&CaseTranslation>,
) -> Result<(), RegexCompileError> {
    *class_bits |= posix_class_bit(name)?;
    // --- ASCII bitmap bits ------------------------------------------------
    //
    // GNU `regex_compile` (regex-emacs.c:2081-2092) sets bitmap (list) bits
    // ONLY for ASCII characters `c < 0x80` where `re_iswctype(c, cc)` is true;
    // the non-ASCII / multibyte side is recorded SOLELY as a range-table bit
    // (`re_wctype_to_bit`) and consulted later by `execute_charset` for chars
    // `c >= 256` (our `class_bits` / `mb_ranges`, the multibyte dispatch path).
    //
    // Therefore the bitmap NEVER contains bits for bytes 0x80..=0xFF.  A raw
    // high byte 0x80..=0xFF in a UNIBYTE target hits the bitmap-only branch of
    // `execute_charset` (`unibyte && c < 256`, regex-emacs.c:3773), where these
    // bits are absent, so it matches NO POSIX class — exactly GNU's behavior
    // (e.g. `[[:nonascii:]]`, `[[:print:]]`, even `[[:unibyte:]]` do NOT match
    // a raw high byte in a unibyte string).  Each arm below enumerates only the
    // ASCII bytes for which `re_iswctype` is true.
    let ascii_bytes: Vec<u8> = match name {
        "alpha" => (b'A'..=b'Z').chain(b'a'..=b'z').collect(),
        "digit" => (b'0'..=b'9').collect(),
        "alnum" => (b'A'..=b'Z')
            .chain(b'a'..=b'z')
            .chain(b'0'..=b'9')
            .collect(),
        // GNU ISSPACE uses BUFFER_SYNTAX; default standard-syntax-table
        // whitespace is space, tab, LF, CR, and FF. Vtab (0x0B) is NOT
        // whitespace in GNU's default. See syntax.c standard init.
        "space" => vec![b' ', b'\t', b'\n', b'\r', 0x0C],
        // GNU ISBLANK is strictly ASCII space and tab.
        "blank" => vec![b' ', b'\t'],
        "upper" => (b'A'..=b'Z').collect(),
        "lower" => (b'a'..=b'z').collect(),
        "punct" => b"!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~".to_vec(),
        // GNU ISPRINT for ASCII (c < 0x80): c >= ' ' and not 0x7F, i.e.
        // 0x20..=0x7E.  The high-byte printable range (0xA0..=0xFF) is NOT a
        // bitmap bit; multibyte printable chars match via `class_bits` in the
        // multibyte path of `execute_charset`.
        "print" => (0x20u8..=0x7E).collect(),
        // GNU ISGRAPH for ASCII (c < 0x80): c > ' ' and not 0x7F, i.e.
        // 0x21..=0x7E.  High bytes go through the multibyte path only.
        "graph" => (0x21u8..=0x7E).collect(),
        // GNU `ISCNTRL(c)` is `((c) < ' ')` (regex-emacs.c:108), i.e. only
        // 0x00..=0x1F.  DEL (0x7F) is NOT a control char for Emacs regexp
        // `[[:cntrl:]]`, unlike the C-locale `iscntrl`.  Including 0x7F here
        // made json.el's `(rx (in cntrl))` escape DEL as `` instead of
        // emitting it literally (matching GNU `json-encode-string`).
        "cntrl" => (0x00u8..=0x1F).collect(),
        "xdigit" => (b'0'..=b'9')
            .chain(b'A'..=b'F')
            .chain(b'a'..=b'f')
            .collect(),
        "ascii" => (0x00u8..=0x7F).collect(),
        // GNU ISWORD(c) = BUFFER_SYNTAX(c) == Sword. Default standard
        // syntax table has ASCII letters and digits as word
        // constituents. Per-buffer syntax tables are audit #8.
        "word" => (b'A'..=b'Z')
            .chain(b'a'..=b'z')
            .chain(b'0'..=b'9')
            .collect(),
        // `re_iswctype(c, RECC_NONASCII)` = `!IS_REAL_ASCII(c)` is FALSE for
        // every ASCII byte, so nonascii sets NO bitmap bits; non-ASCII chars
        // match via the `BIT_MULTIBYTE` range-table bit (the multibyte path).
        "nonascii" => Vec::new(),
        // `re_iswctype(c, RECC_UNIBYTE)` = `ISUNIBYTE(c)` is true for all ASCII
        // bytes (c < 0x80) only at compile time (the loop runs c < 0x80); high
        // bytes 0x80..=0xFF are NOT set, so `[[:unibyte:]]` matches an ASCII
        // byte but NOT a raw high byte in a unibyte string (matching GNU).
        // RECC_UNIBYTE has no range-table bit (`re_wctype_to_bit` returns 0).
        "unibyte" => (0x00u8..=0x7F).collect(),
        // !ISUNIBYTE(c): only multibyte (non-ASCII) characters.
        // Nothing in the bitmap; everything in the multibyte range.
        "multibyte" => Vec::new(),
        // GNU `re_wctype_parse` returns RECC_ERROR (regex-emacs.c:1600)
        // for unknown names; the caller at regex-emacs.c:2071 then
        // signals REG_ECTYPE. We raise the equivalent compile error
        // here rather than silently continuing.
        _ => {
            return Err(RegexCompileError {
                message: format!("Invalid character class name: {}", name),
            });
        }
    };

    // GNU regex-emacs.c:2081-2092 sets the bit for the raw class
    // member AND also the bit for its TRANSLATE-mapped partner when
    // a translate table is in effect. Our set_bitmap_bit always
    // applies the translation (so it already sets the translated
    // bit); we additionally set the raw bit here to cover inputs
    // that match the raw form without going through the translate.
    for c in ascii_bytes {
        // Raw bit (no translation).
        set_bitmap_bit(buffer, bitmap_start, c, None);
        // Translated bit, when a translate table is active. This is
        // a no-op when `translate` is `None` or `translate[c] == c`.
        if translate.is_some() {
            set_bitmap_bit(buffer, bitmap_start, c, translate);
        }
    }

    // --- Multibyte coverage ----------------------------------------------
    //
    // GNU does not expand multibyte POSIX classes into concrete ranges at
    // compile time.  It records class bits and calls `re_iswctype` while
    // executing the charset.  `class_bits` above is the Neomacs equivalent.
    // These explicit ranges remain only for the two classes that are pure
    // broad range tests and were represented this way historically.
    match name {
        // Non-ASCII entirely: 0x80..=max Unicode scalar.
        "nonascii" | "multibyte" => {
            mb_ranges.push(('\u{80}', '\u{10FFFF}'));
        }
        _ => {}
    }

    Ok(())
}

fn posix_class_bit(name: &str) -> Result<u32, RegexCompileError> {
    match name {
        "alnum" => Ok(CHARSET_CLASS_BIT_ALNUM),
        "alpha" => Ok(CHARSET_CLASS_BIT_ALPHA),
        "blank" => Ok(CHARSET_CLASS_BIT_BLANK),
        "cntrl" => Ok(CHARSET_CLASS_BIT_CNTRL),
        "digit" => Ok(CHARSET_CLASS_BIT_DIGIT),
        "graph" => Ok(CHARSET_CLASS_BIT_GRAPH),
        "lower" => Ok(CHARSET_CLASS_BIT_LOWER),
        "print" => Ok(CHARSET_CLASS_BIT_PRINT),
        "punct" => Ok(CHARSET_CLASS_BIT_PUNCT),
        "space" => Ok(CHARSET_CLASS_BIT_SPACE),
        "upper" => Ok(CHARSET_CLASS_BIT_UPPER),
        "xdigit" => Ok(CHARSET_CLASS_BIT_XDIGIT),
        "ascii" => Ok(CHARSET_CLASS_BIT_ASCII),
        "word" => Ok(CHARSET_CLASS_BIT_WORD),
        "nonascii" => Ok(CHARSET_CLASS_BIT_NONASCII),
        "unibyte" => Ok(CHARSET_CLASS_BIT_UNIBYTE),
        "multibyte" => Ok(CHARSET_CLASS_BIT_MULTIBYTE),
        _ => Err(RegexCompileError {
            message: format!("Invalid character class name: {}", name),
        }),
    }
}

/// Parse an interval \{n,m\} from the pattern.
/// Returns (min, max) where max=None means unbounded.
fn parse_interval(
    pattern: &[u8],
    p: &mut usize,
) -> Result<(usize, Option<usize>), RegexCompileError> {
    let plen = pattern.len();

    // Parse min
    let mut min = 0usize;
    while *p < plen && pattern[*p].is_ascii_digit() {
        min = min * 10 + (pattern[*p] - b'0') as usize;
        *p += 1;
    }

    let max = if *p < plen && pattern[*p] == b',' {
        *p += 1; // skip comma
        if *p < plen && pattern[*p] == b'\\' && *p + 1 < plen && pattern[*p + 1] == b'}' {
            // \{n,\} — unbounded
            None
        } else {
            let mut m = 0usize;
            while *p < plen && pattern[*p].is_ascii_digit() {
                m = m * 10 + (pattern[*p] - b'0') as usize;
                *p += 1;
            }
            Some(m)
        }
    } else {
        Some(min) // \{n\} — exact count
    };

    // GNU regex-emacs.c:2390 rejects a descending interval where a finite
    // upper bound is smaller than the lower bound (e.g. `a\{2,1\}`), signaling
    // `(invalid-regexp "Invalid content of \\{\\}")`.  An unbounded `\{n,\}`
    // (max == None) is always valid.
    if let Some(m) = max {
        if m < min {
            return Err(RegexCompileError {
                message: "Invalid content of \\{\\}".to_string(),
            });
        }
    }

    // Expect \}
    if *p + 1 < plen && pattern[*p] == b'\\' && pattern[*p + 1] == b'}' {
        *p += 2;
    } else {
        return Err(RegexCompileError {
            message: "Unmatched \\{".to_string(),
        });
    }

    Ok((min, max))
}

/// Compile an interval \{n,m\} into bytecode.
fn checked_i16_offset(offset: isize) -> Result<i16, RegexCompileError> {
    i16::try_from(offset).map_err(|_| RegexCompileError {
        message: "Regular expression too big".to_string(),
    })
}

fn checked_i16_counter(value: usize) -> Result<i16, RegexCompileError> {
    i16::try_from(value).map_err(|_| RegexCompileError {
        message: "Regular expression too big".to_string(),
    })
}

fn store_jump_at(
    buffer: &mut Vec<u8>,
    op_pos: usize,
    op: RegexOp,
    target: usize,
) -> Result<(), RegexCompileError> {
    buffer[op_pos] = op as u8;
    let offset = checked_i16_offset(target as isize - (op_pos + 3) as isize)?;
    store_number(buffer, op_pos + 1, offset);
    Ok(())
}

fn store_jump2_at(
    buffer: &mut Vec<u8>,
    op_pos: usize,
    op: RegexOp,
    target: usize,
    count: usize,
) -> Result<(), RegexCompileError> {
    store_jump_at(buffer, op_pos, op, target)?;
    store_number(buffer, op_pos + 3, checked_i16_counter(count)?);
    Ok(())
}

fn insert_jump(
    buffer: &mut Vec<u8>,
    at: usize,
    op: RegexOp,
    target: usize,
) -> Result<(), RegexCompileError> {
    buffer.splice(at..at, [op as u8, 0, 0]);
    store_jump_at(buffer, at, op, target)
}

fn insert_jump2(
    buffer: &mut Vec<u8>,
    at: usize,
    op: RegexOp,
    target: usize,
    count: usize,
) -> Result<(), RegexCompileError> {
    buffer.splice(at..at, [op as u8, 0, 0, 0, 0]);
    store_jump2_at(buffer, at, op, target, count)
}

fn insert_set_number_at(
    buffer: &mut Vec<u8>,
    at: usize,
    target_counter_offset: usize,
    value: usize,
) -> Result<(), RegexCompileError> {
    buffer.splice(at..at, [RegexOp::SetNumberAt as u8, 0, 0, 0, 0]);
    let offset = checked_i16_offset(target_counter_offset as isize)?;
    store_number(buffer, at + 1, offset);
    store_number(buffer, at + 3, checked_i16_counter(value)?);
    Ok(())
}

/// Compile an interval \{n,m\} into GNU's counted interval bytecode.
///
/// This mirrors `src/regex-emacs.c`'s interval layout:
///
/// ```text
/// set_number_at <jump_n count> <upper>
/// set_number_at <succeed_n count> <lower>
/// succeed_n     <after jump_n>   <lower>
/// <body>
/// jump_n        <succeed_n>      <upper - 1>
/// ```
///
/// GNU uses `on_failure_jump_loop` instead of `succeed_n` for a zero
/// lower bound and omits the upper-bound `jump_n` when no finite upper
/// bound exists.  Keeping this counted shape matters for large intervals
/// such as CC Mode's `[[:alnum:]]\\{,1000\\}`: expanding the body into
/// hundreds of optional copies creates backtracking behavior GNU avoids.
fn compile_interval(
    min: usize,
    max: Option<usize>,
    laststart: usize,
    buf: &mut CompiledPattern,
) -> Result<(), RegexCompileError> {
    if let Some(max_val) = max {
        if max_val == 0 {
            buf.buffer.truncate(laststart);
            return Ok(());
        }
        if min == 1 && max_val == 1 {
            return Ok(());
        }
    }

    let old_end = buf.buffer.len();
    let upper_extra_bytes = match max {
        None => 3,
        Some(max_val) if max_val > 1 => 5,
        Some(_) => 0,
    };
    let mut emitted_end = old_end;
    let mut startoffset = 0usize;

    if min == 0 {
        insert_jump(
            &mut buf.buffer,
            laststart,
            RegexOp::OnFailureJumpLoop,
            old_end + 3 + upper_extra_bytes,
        )?;
        emitted_end += 3;
    } else {
        insert_jump2(
            &mut buf.buffer,
            laststart,
            RegexOp::SucceedN,
            old_end + 5 + upper_extra_bytes,
            min,
        )?;
        emitted_end += 5;
        insert_set_number_at(&mut buf.buffer, laststart, 5, min)?;
        emitted_end += 5;
        startoffset += 5;
    }

    match max {
        None => {
            let op_pos = emitted_end;
            buf.buffer.extend_from_slice(&[RegexOp::Jump as u8, 0, 0]);
            store_jump_at(
                &mut buf.buffer,
                op_pos,
                RegexOp::Jump,
                laststart + startoffset,
            )?;
        }
        Some(max_val) if max_val > 1 => {
            let op_pos = emitted_end;
            buf.buffer
                .extend_from_slice(&[RegexOp::JumpN as u8, 0, 0, 0, 0]);
            store_jump2_at(
                &mut buf.buffer,
                op_pos,
                RegexOp::JumpN,
                laststart + startoffset,
                max_val - 1,
            )?;
            emitted_end += 5;
            insert_set_number_at(
                &mut buf.buffer,
                laststart,
                emitted_end - laststart,
                max_val - 1,
            )?;
        }
        Some(_) => {}
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Phase 3: Matcher (re_match_2_internal)
//
// Translates GNU regex-emacs.c:4072-5340.
// Executes compiled bytecode against input text with backtracking.
// ---------------------------------------------------------------------------

/// Context for syntax-table and category-table queries during matching.
///
/// The matcher queries the syntax table to implement `\w`, `\b`, `\sC`, etc.
/// In GNU Emacs, this is done via the `SYNTAX()` macro which reads from
/// `gl_state.current_syntax_table`.
pub(crate) trait SyntaxLookup {
    /// Return the syntax class of character `c` in the current syntax table.
    fn char_syntax(&self, c: char) -> SyntaxClass;

    /// Return true if character `c` belongs to category `cat`.
    fn char_has_category(&self, c: char, cat: u8) -> bool;
}

/// Default syntax lookup — uses GNU's standard syntax-table definitions.
/// This is used when no buffer-specific syntax table is available
/// (e.g. in unit tests or string-only matching).
pub(crate) struct DefaultSyntaxLookup;

/// Syntax lookup backed by a buffer's actual syntax table.
/// Used when regex searching within a buffer context.
pub(crate) struct BufferSyntaxLookup {
    pub syntax_table: crate::emacs_core::syntax::SyntaxTable,
    pub category_table: Option<crate::emacs_core::value::Value>,
}

impl SyntaxLookup for DefaultSyntaxLookup {
    fn char_syntax(&self, c: char) -> SyntaxClass {
        crate::emacs_core::syntax::standard_syntax_class_for_char(c)
    }

    fn char_has_category(&self, c: char, cat: u8) -> bool {
        default_char_has_category(c, cat)
    }
}

impl SyntaxLookup for BufferSyntaxLookup {
    fn char_syntax(&self, c: char) -> SyntaxClass {
        self.syntax_table.char_syntax(c)
    }

    fn char_has_category(&self, c: char, cat: u8) -> bool {
        self.category_table
            .and_then(|table| {
                crate::emacs_core::category::char_has_category_in_table(table, c, cat).ok()
            })
            .unwrap_or_else(|| default_char_has_category(c, cat))
    }
}

/// Return whether character `c` belongs to the GNU regex category
/// `cat` (`\cX`).
///
/// GNU's category mechanism (`src/category.c`) gives each character
/// a 128-bit set of category memberships, populated at startup
/// time from `lisp/international/characters.el`. We don't ship the
/// full table; instead we hardcode the most common categories using
/// Unicode block ranges. The category mnemonics here are taken
/// directly from `lisp/international/characters.el` (the GNU
/// `(define-category ?X "...")` lines starting at line 37).
///
/// Audit finding #6 in `drafts/regex-search-audit.md` flagged that
/// only `\c|` worked. This implementation covers the categories the
/// CJK font-lock and bidi paths actually use.
fn default_char_has_category(c: char, cat: u8) -> bool {
    let cp = c as u32;
    match cat {
        // |  -- "line breakable". GNU's `characters.el` adds this
        // for most CJK and fullwidth ranges; we use the practical
        // shortcut of "any non-ASCII char" which is what neomacs
        // historically returned.
        b'|' => !c.is_ascii(),

        // a  -- ASCII. GNU `lisp/international/characters.el` assigns
        // category `a` to codepoints 32..127, not ASCII controls.
        b'a' => (0x20..=0x7f).contains(&cp),

        // A  -- 2-byte alnum. GNU populates this from CJK Latin /
        // fullwidth ASCII ranges. The practical shortcut is the
        // fullwidth ASCII alphanumeric block.
        b'A' => matches!(cp, 0xFF10..=0xFF19 | 0xFF21..=0xFF3A | 0xFF41..=0xFF5A),

        // l  -- Latin (a-z, A-Z and Latin-1/Extended letters).
        // r  -- Roman (Japanese context, same effective range).
        b'l' | b'r' => {
            c.is_ascii_alphabetic()
                || matches!(cp, 0x00C0..=0x00FF | 0x0100..=0x024F | 0x1E00..=0x1EFF)
        }

        // g  -- Greek (Greek and Coptic block).
        b'g' => matches!(cp, 0x0370..=0x03FF | 0x1F00..=0x1FFF),

        // G  -- 2-byte Greek (fullwidth Greek). Rare; use the
        // same practical bounds as `g` for now.
        b'G' => matches!(cp, 0x0370..=0x03FF | 0x1F00..=0x1FFF),

        // y  -- Cyrillic.
        b'y' | b'Y' => matches!(cp, 0x0400..=0x04FF | 0x0500..=0x052F),

        // b  -- Arabic.
        b'b' => matches!(cp, 0x0600..=0x06FF | 0x0750..=0x077F | 0xFB50..=0xFDFF),

        // w  -- Hebrew.
        b'w' => matches!(cp, 0x0590..=0x05FF | 0xFB1D..=0xFB4F),

        // t  -- Thai.
        b't' => matches!(cp, 0x0E00..=0x0E7F),

        // o  -- Lao.
        b'o' => matches!(cp, 0x0E80..=0x0EFF),

        // q  -- Tibetan.
        b'q' => matches!(cp, 0x0F00..=0x0FFF),

        // i  -- Indian (Devanagari + related). GNU's actual table
        // covers more scripts; this is the most common one.
        b'i' => matches!(cp, 0x0900..=0x097F),

        // I  -- Indian glyphs (broader Indic blocks).
        b'I' => matches!(cp, 0x0900..=0x0DFF),

        // e  -- Ethiopic (Ge'ez).
        b'e' => matches!(cp, 0x1200..=0x137F),

        // v  -- Vietnamese (Latin Extended Additional).
        b'v' => matches!(cp, 0x1E00..=0x1EFF),

        // h  -- Korean (Hangul Syllables + Jamo).
        // N  -- 2-byte Korean (same range here).
        b'h' | b'N' => {
            matches!(cp, 0x1100..=0x11FF | 0xAC00..=0xD7A3 | 0xA960..=0xA97F | 0xD7B0..=0xD7FF)
        }

        // c  -- Chinese / Han ideographs (broad).
        // C  -- 2-byte han (slightly narrower set).
        b'c' | b'C' => matches!(
            cp,
            0x3400..=0x4DBF
                | 0x4E00..=0x9FFF
                | 0xF900..=0xFAFF
                | 0x20000..=0x2FFFF
                | 0x30000..=0x323AF
        ),

        // H  -- Hiragana (Japanese).
        b'H' => matches!(cp, 0x3040..=0x309F | 0x1B000..=0x1B16F),

        // K  -- Katakana (Japanese).
        b'K' => matches!(
            cp,
            0x3099..=0x309C | 0x30A0..=0x30FF | 0x31F0..=0x31FF | 0x1AFF0..=0x1B16F
        ),

        // k  -- Katakana (lowercase mnemonic, same coverage).
        b'k' => matches!(cp, 0x30A0..=0x30FF | 0x31F0..=0x31FF | 0xFF66..=0xFF9F),

        // j  -- Japanese (Hiragana + Katakana + half-width Katakana
        // + CJK punctuation + fullwidth ASCII).
        b'j' => matches!(
            cp,
            0x3000..=0x303F
                | 0x3040..=0x309F
                | 0x30A0..=0x30FF
                | 0xFF00..=0xFFEF
        ),

        // .  -- Base (Unicode L,N,P,S,Zs).
        b'.' => match c.is_ascii() {
            true => c.is_ascii_graphic() || c == ' ',
            false => {
                !matches!(cp, 0x0300..=0x036F | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x20D0..=0x20FF | 0xFE20..=0xFE2F)
            }
        },

        // ^  -- Combining diacritic / mark (Unicode M).
        b'^' => {
            matches!(cp, 0x0300..=0x036F | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x20D0..=0x20FF | 0xFE20..=0xFE2F)
        }

        // R  -- Strong R2L (right-to-left). Practical heuristic:
        // Hebrew and Arabic ranges.
        b'R' => matches!(cp, 0x0590..=0x05FF | 0x0600..=0x06FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF),

        // L  -- Strong L2R (everything else).
        b'L' => {
            !matches!(cp, 0x0590..=0x05FF | 0x0600..=0x06FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF)
        }

        // 6  -- digit (numeric).
        b'6' => c.is_numeric(),

        // Categories we don't recognize fall through as "no
        // membership" — same as GNU's behavior for an unset bit.
        _ => false,
    }
}

fn unicode_blank_char(c: char) -> bool {
    matches!(
        c,
        '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{202f}' | '\u{205f}' | '\u{3000}'
    )
}

fn unicode_line_or_paragraph_separator(c: char) -> bool {
    matches!(c, '\u{2028}' | '\u{2029}')
}

fn unicode_graphic_char(c: char) -> bool {
    !unicode_blank_char(c) && !unicode_line_or_paragraph_separator(c) && !c.is_control()
}

fn unicode_printable_char(c: char) -> bool {
    !c.is_control()
}

/// Match a compiled pattern against input text.
///
/// This is the core matching function, equivalent to GNU's `re_match_2_internal`.
///
/// # Arguments
/// * `pattern` - Compiled bytecode pattern
/// * `text` - Input text to match against
/// * `pos` - Starting position in text
/// * `stop` - End of matching region
/// * `syntax` - Syntax table for `\w`, `\b`, `\sC` etc.
/// * `point` - Buffer point position (for `\=` / AtDot)
///
/// # Returns
/// * `Some(end_pos)` if matched — end position of the match
/// * `None` if no match
pub(crate) fn re_match(
    pattern: &CompiledPattern,
    text: &[u8],
    pos: usize,
    stop: usize,
    syntax: &dyn SyntaxLookup,
    point: usize,
) -> Option<(usize, MatchRegisters)> {
    let bytecode = &pattern.buffer;
    let num_regs = pattern.re_nsub + 1;
    let mut fail_stack: Vec<FailurePoint> = Vec::new();

    // Mutable counter table for interval repetition (succeed_n / jump_n / set_number_at).
    // GNU modifies bytecode in-place; we use a side table keyed by bytecode position.
    let mut counters: HashMap<usize, i16> = HashMap::new();

    // GNU regex-emacs.c:4188-4204 skips all internal register arrays
    // when the pattern has no subexpressions. Register 0 is handled
    // separately on success, so failed no-group candidate checks should
    // not allocate register scratch space.
    let has_subexpressions = pattern.re_nsub > 0;
    let mut regstart: RegisterScratch = if has_subexpressions {
        register_scratch(num_regs)
    } else {
        RegisterScratch::new()
    };
    let mut regend: RegisterScratch = if has_subexpressions {
        register_scratch(num_regs)
    } else {
        RegisterScratch::new()
    };

    // Best match tracking for POSIX longest-match (audit #2).
    //
    // Mirrors GNU regex-emacs.c:4143-4154 and the main loop handling
    // at lines 4268-4345. When the pattern reaches its end with the
    // matcher positioned before the end of the searchable region
    // (`d < stop`), we save the current register state as the "best
    // so far" and force a backtrack to explore alternative paths.
    // After all backtracks have been exhausted, the best saved match
    // is restored. See GNU regex-emacs.c:5278-5279 for the
    // equivalent "restore after total failure" path.
    let posix_longest = pattern.posix;
    let mut best_regs_set = false;
    let mut best_match_end: usize = pos;
    let mut best_regstart: RegisterScratch = if has_subexpressions {
        register_scratch(num_regs)
    } else {
        RegisterScratch::new()
    };
    let mut best_regend: RegisterScratch = if has_subexpressions {
        register_scratch(num_regs)
    } else {
        RegisterScratch::new()
    };

    let mut pc = 0usize; // Bytecode program counter
    let mut d = pos; // Data position in text

    let translate = &pattern.translate;
    let pattern_multibyte = pattern.multibyte;
    let target_multibyte = pattern.target_multibyte;

    // Helper: translate a character for case-folding
    let tr = |c: u32| -> u32 {
        if let Some(table) = translate {
            table.translate(c)
        } else {
            c
        }
    };

    // Helper: get char at position in text (with bounds check)
    let text_byte = |pos: usize| -> Option<u8> {
        if pos < text.len() {
            Some(text[pos])
        } else {
            None
        }
    };

    let unibyte_to_emacs_char = |byte: u8| -> u32 {
        if byte < 0x80 {
            byte as u32
        } else {
            emacs_char::unibyte_to_char(byte)
        }
    };
    let syntax_char = |code: u32| -> char {
        if emacs_char::char_byte8_p(code) {
            char::from(emacs_char::char_to_byte8(code))
        } else {
            char::from_u32(code).unwrap_or(char::REPLACEMENT_CHARACTER)
        }
    };
    let emacs_char_to_unibyte = |code: u32| -> Option<u8> {
        if code < 0x80 || emacs_char::char_byte8_p(code) {
            Some(emacs_char::char_to_byte8(code))
        } else {
            None
        }
    };
    let posix_class_matches = |code: u32, bits: u32| -> bool {
        let byte = emacs_char_to_unibyte(code);
        let ch = syntax_char(code);
        let is_real_ascii = code < 0x80;
        let ascii_alnum = |b: u8| b.is_ascii_alphabetic() || b.is_ascii_digit();
        let ascii_alpha = |b: u8| b.is_ascii_alphabetic();

        (bits & CHARSET_CLASS_BIT_ALNUM != 0
            && if is_real_ascii {
                ascii_alnum(code as u8)
            } else {
                ch.is_alphanumeric()
            })
            || (bits & CHARSET_CLASS_BIT_ALPHA != 0
                && if is_real_ascii {
                    ascii_alpha(code as u8)
                } else {
                    ch.is_alphabetic()
                })
            || (bits & CHARSET_CLASS_BIT_BLANK != 0
                && if is_real_ascii {
                    matches!(code, 0x20 | 0x09)
                } else {
                    unicode_blank_char(ch)
                })
            || (bits & CHARSET_CLASS_BIT_CNTRL != 0 && code < 0x20)
            || (bits & CHARSET_CLASS_BIT_DIGIT != 0
                && is_real_ascii
                && (code as u8).is_ascii_digit())
            || (bits & CHARSET_CLASS_BIT_GRAPH != 0
                && byte.map_or_else(
                    || unicode_graphic_char(ch),
                    |b| b > b' ' && !(0x7f..=0xa0).contains(&b),
                ))
            || (bits & CHARSET_CLASS_BIT_LOWER != 0 && ch.is_lowercase())
            || (bits & CHARSET_CLASS_BIT_PRINT != 0
                && byte.map_or_else(
                    || unicode_printable_char(ch),
                    |b| b >= b' ' && !(0x7f..=0x9f).contains(&b),
                ))
            || (bits & CHARSET_CLASS_BIT_PUNCT != 0
                && if is_real_ascii {
                    let b = code as u8;
                    b > b' ' && b < 0x7f && !ascii_alnum(b)
                } else {
                    syntax.char_syntax(ch) != SyntaxClass::Word
                })
            || (bits & CHARSET_CLASS_BIT_SPACE != 0
                && syntax.char_syntax(ch) == SyntaxClass::Whitespace)
            || (bits & CHARSET_CLASS_BIT_UPPER != 0 && ch.is_uppercase())
            || (bits & CHARSET_CLASS_BIT_XDIGIT != 0
                && is_real_ascii
                && (code as u8).is_ascii_hexdigit())
            || (bits & CHARSET_CLASS_BIT_ASCII != 0 && is_real_ascii)
            || (bits & CHARSET_CLASS_BIT_WORD != 0 && syntax.char_syntax(ch) == SyntaxClass::Word)
            || (bits & CHARSET_CLASS_BIT_NONASCII != 0 && !is_real_ascii)
            || (bits & CHARSET_CLASS_BIT_UNIBYTE != 0 && byte.is_some())
            || (bits & CHARSET_CLASS_BIT_MULTIBYTE != 0 && byte.is_none())
    };

    // Helper: decode an Emacs character at position.
    let text_char = |pos: usize| -> Option<(u32, usize)> {
        if pos >= text.len() {
            return None;
        }
        if target_multibyte {
            Some(emacs_char::string_char(&text[pos..]))
        } else {
            Some((unibyte_to_emacs_char(text[pos]), 1))
        }
    };

    // Helper: find the start of the character before `pos`.
    let prev_char_start = |pos: usize| -> Option<usize> {
        if pos == 0 {
            return None;
        }
        if !target_multibyte {
            return Some(pos - 1);
        }
        let mut p = pos - 1;
        while p > 0 && (text[p] & 0xC0) == 0x80 {
            p -= 1;
        }
        Some(p)
    };

    // Helper: is position at a word boundary?
    let at_word_boundary = |pos: usize| -> bool {
        let prev_word = if let Some(prev) = prev_char_start(pos) {
            text_char(prev)
                .map(|(c, _)| syntax.char_syntax(syntax_char(c)) == SyntaxClass::Word)
                .unwrap_or(false)
        } else {
            false
        };
        let curr_word = text_char(pos)
            .map(|(c, _)| syntax.char_syntax(syntax_char(c)) == SyntaxClass::Word)
            .unwrap_or(false);
        prev_word != curr_word
    };

    // Helper: is position at a symbol boundary?
    let at_symbol_boundary = |pos: usize| -> bool {
        let is_symbol_char = |c: u32| {
            let syn = syntax.char_syntax(syntax_char(c));
            syn == SyntaxClass::Word || syn == SyntaxClass::Symbol
        };
        let prev_sym = if let Some(prev) = prev_char_start(pos) {
            text_char(prev)
                .map(|(c, _)| is_symbol_char(c))
                .unwrap_or(false)
        } else {
            false
        };
        let curr_sym = text_char(pos)
            .map(|(c, _)| is_symbol_char(c))
            .unwrap_or(false);
        prev_sym != curr_sym
    };

    // `try_fail!()` is the in-function replacement for GNU's
    // `goto fail`: pop the failure stack and resume there. If the
    // stack is empty, every backtracking avenue has been exhausted.
    // GNU's re_match_2_internal checks `best_regs_set` at that point
    // (regex-emacs.c:5278) and restores the saved best match instead
    // of returning -1. We do the same by setting `total_failure` and
    // breaking to the outer finalization block, which consults
    // `best_regs_set` to decide between returning None and restoring
    // the best registers.
    let mut total_failure = false;
    // `macro_rules!` labels are hygienic, so the label has to be
    // passed in explicitly as a `lifetime` metavariable.
    macro_rules! try_fail {
        ($label:lifetime) => {
            if goto_fail(
                &mut pc,
                &mut d,
                &mut fail_stack,
                &mut regstart,
                &mut regend,
                &mut counters,
            )
            .is_none()
            {
                total_failure = true;
                break $label;
            }
        };
    }

    'main_loop: loop {
        // End of pattern = potential match.
        //
        // GNU regex-emacs.c:4272-4345: if we haven't consumed the
        // full match region and POSIX longest-match is requested,
        // save the current registers as the best seen so far and
        // force another backtrack. When no more backtracks remain,
        // restore whichever saved best is better than the final
        // candidate (regex-emacs.c:4323-4344).
        if pc >= bytecode.len() {
            if d > stop {
                try_fail!('main_loop);
                continue 'main_loop;
            }
            if posix_longest && d < stop {
                let better_than_best = !best_regs_set || d > best_match_end;
                if !fail_stack.is_empty() {
                    if better_than_best {
                        best_regs_set = true;
                        best_match_end = d;
                        best_regstart.clone_from_slice(&regstart);
                        best_regend.clone_from_slice(&regend);
                    }
                    // Force a backtrack to explore alternative paths.
                    // The stack is non-empty so goto_fail cannot fail.
                    try_fail!('main_loop);
                    continue 'main_loop;
                } else if best_regs_set && !better_than_best {
                    // No more backtracks; the previously saved best
                    // beats the current finishing position.  Restore
                    // it before finalizing.
                    d = best_match_end;
                    for i in 1..num_regs {
                        regstart[i] = best_regstart[i];
                        regend[i] = best_regend[i];
                    }
                }
            }
            break 'main_loop;
        }

        let op_byte = bytecode[pc];
        let Some(op) = RegexOp::from_byte(op_byte) else {
            // Invalid opcode — treat as match failure
            return None;
        };
        pc += 1;

        match op {
            RegexOp::NoOp => {
                // Skip
            }

            RegexOp::Succeed => {
                // GNU regex-emacs.c:4429-4431 jumps directly to
                // `succeed_label`, bypassing the POSIX longest-match
                // check. For non-POSIX patterns, neomacs's compiler
                // emits a trailing `Succeed` so the matcher exits as
                // soon as the pattern completes (mirroring GNU's
                // `if (!posix_backtracking) BUF_PUSH(succeed)` at
                // regex-emacs.c:2685). In POSIX mode the trailing
                // `Succeed` is NOT emitted, so the matcher instead
                // falls through to the end-of-bytecode check above.
                if d > stop {
                    try_fail!('main_loop);
                    continue 'main_loop;
                }
                break 'main_loop;
            }

            RegexOp::Exactn => {
                let count = bytecode[pc] as usize;
                pc += 1;
                let mut matched = true;
                let literal_start = pc;
                let literal_end = literal_start + count;
                let mut pat_pos = literal_start;
                while pat_pos < literal_end {
                    if d >= stop {
                        matched = false;
                        break;
                    }

                    let Some((buf_ch, buf_len)) = text_char(d) else {
                        matched = false;
                        break;
                    };

                    if target_multibyte {
                        let (pat_ch, pat_len) = if pattern_multibyte {
                            emacs_char::string_char(&bytecode[pat_pos..literal_end])
                        } else {
                            (unibyte_to_emacs_char(bytecode[pat_pos]), 1)
                        };
                        if tr(buf_ch) != pat_ch {
                            matched = false;
                            break;
                        }
                        pat_pos += pat_len;
                        d += buf_len;
                    } else {
                        let pat_byte = if pattern_multibyte {
                            let (pat_ch, pat_len) =
                                emacs_char::string_char(&bytecode[pat_pos..literal_end]);
                            let Some(byte) = emacs_char_to_unibyte(pat_ch) else {
                                matched = false;
                                break;
                            };
                            pat_pos += pat_len;
                            byte
                        } else {
                            let byte = bytecode[pat_pos];
                            pat_pos += 1;
                            byte
                        };
                        let buf_byte = text[d];
                        let mut translated = unibyte_to_emacs_char(buf_byte);
                        if !emacs_char::char_byte8_p(translated) {
                            translated = tr(translated);
                            if let Some(byte) = emacs_char_to_unibyte(translated) {
                                translated = byte as u32;
                            } else {
                                translated = buf_byte as u32;
                            }
                        } else {
                            translated = buf_byte as u32;
                        }
                        if translated as u8 != pat_byte {
                            matched = false;
                            break;
                        }
                        d += 1;
                    }
                }
                pc = literal_end;
                if !matched {
                    try_fail!('main_loop);
                }
            }

            RegexOp::AnyChar => {
                if d >= stop {
                    try_fail!('main_loop);
                    continue;
                }
                // Match any character except newline
                let Some((buf_ch, buf_len)) = text_char(d) else {
                    try_fail!('main_loop);
                    continue;
                };
                if tr(buf_ch) == '\n' as u32 {
                    try_fail!('main_loop);
                    continue;
                }
                d += buf_len;
            }

            RegexOp::Charset | RegexOp::CharsetNot => {
                let negate = op == RegexOp::CharsetNot;
                let charset_op_pos = pc - 1; // bytecode position of the opcode
                let bitmap_len = bytecode[pc] as usize & 0x7F;
                pc += 1;

                if d >= stop {
                    pc += bitmap_len;
                    try_fail!('main_loop);
                    continue;
                }

                let Some((orig_ch, ch_len)) = text_char(d) else {
                    pc += bitmap_len;
                    try_fail!('main_loop);
                    continue;
                };
                let mut ch = orig_ch;
                let mut unibyte_char = false;

                if target_multibyte {
                    ch = tr(ch);
                    if let Some(byte) = emacs_char_to_unibyte(ch) {
                        unibyte_char = true;
                        ch = byte as u32;
                    }
                } else {
                    let mut converted = unibyte_to_emacs_char(text[d]);
                    if !emacs_char::char_byte8_p(converted) {
                        converted = tr(converted);
                        if let Some(byte) = emacs_char_to_unibyte(converted) {
                            unibyte_char = true;
                            ch = byte as u32;
                        }
                    } else {
                        unibyte_char = true;
                        ch = text[d] as u32;
                    }
                }

                // GNU `execute_charset` (regex-emacs.c:3756-3815) has two
                // MUTUALLY EXCLUSIVE branches keyed on `unibyte_char`:
                //   * `unibyte && c < 256`  -> consult the BITMAP ONLY
                //     (regex-emacs.c:3773-3779).  The range-table class bits
                //     (`BIT_MULTIBYTE`, `BIT_ALPHA`, ...) are NOT tested, so a
                //     raw high byte in a unibyte target matches no POSIX class.
                //   * otherwise (a true multibyte char) -> consult the
                //     range-table CLASS BITS and explicit ranges
                //     (regex-emacs.c:3781-3811).  The bitmap is NOT consulted.
                // Replicating this split is what makes `[[:nonascii:]]` etc.
                // match a multibyte char but NOT a unibyte raw byte.
                //
                // Caveat for the bitmap branch: GNU builds the bitmap at
                // COMPILE time with `re_iswctype(c, cc)` over `c < 0x80`, which
                // for the syntax-sensitive classes `[:word:]`/`[:space:]`
                // reflects the buffer's syntax table (regex-emacs.c:2081-2101,
                // `used_syntax`).  Neomacs uses a fixed standard-syntax ASCII
                // bitmap, so it re-derives those syntax-sensitive ASCII bits at
                // match time via `posix_class_matches`.  That union is applied
                // ONLY for ASCII chars (`ch < 0x80`); a raw high byte is never
                // tested against the class bits, preserving GNU's rule that no
                // POSIX class matches a high byte in a unibyte string.
                let in_set = if unibyte_char {
                    let c = ch as usize;
                    let bitmap_hit = if (c / 8) < bitmap_len {
                        let byte = bytecode[pc + c / 8];
                        (byte >> (c % 8)) & 1 != 0
                    } else {
                        false
                    };
                    // Re-derive syntax-sensitive ASCII class membership at
                    // match time (the buffer syntax table may differ from the
                    // hardcoded compile-time bitmap, e.g. `_` made a word
                    // constituent).  Restricted to `ch < 0x80` so high bytes
                    // stay bitmap-only and match no class.
                    bitmap_hit
                        || (ch < 0x80
                            && pattern
                                .charset_class_bits
                                .get(&charset_op_pos)
                                .copied()
                                .map(|bits| posix_class_matches(orig_ch, bits))
                                .unwrap_or(false))
                } else {
                    let range_hit = pattern
                        .multibyte_charsets
                        .get(&charset_op_pos)
                        .map(|ranges| {
                            let ch = syntax_char(ch);
                            ranges.iter().any(|&(lo, hi)| ch >= lo && ch <= hi)
                        })
                        .unwrap_or(false);
                    // Union with POSIX class bits, required for multibyte
                    // `[:alnum:]`/`[:print:]` and syntax-sensitive
                    // `[:word:]`/`[:space:]`.  Only reachable for true
                    // multibyte chars, exactly as in GNU's `else if (rtp)`.
                    range_hit
                        || pattern
                            .charset_class_bits
                            .get(&charset_op_pos)
                            .copied()
                            .map(|bits| posix_class_matches(orig_ch, bits))
                            .unwrap_or(false)
                };

                let matched = if negate { !in_set } else { in_set };
                pc += bitmap_len;

                if !matched {
                    try_fail!('main_loop);
                    continue;
                }
                d += ch_len;
            }

            RegexOp::StartMemory => {
                let group = bytecode[pc] as usize;
                pc += 1;
                if group < num_regs
                    && let Some(start) = regstart.get_mut(group)
                {
                    *start = Some(d);
                }
            }

            RegexOp::StopMemory => {
                let group = bytecode[pc] as usize;
                pc += 1;
                if group < num_regs
                    && let Some(end) = regend.get_mut(group)
                {
                    *end = Some(d);
                }
            }

            RegexOp::Duplicate => {
                let group = bytecode[pc] as usize;
                pc += 1;

                let Some(start) = regstart.get(group).copied().flatten() else {
                    try_fail!('main_loop);
                    continue;
                };
                let Some(end) = regend.get(group).copied().flatten() else {
                    try_fail!('main_loop);
                    continue;
                };

                let ref_len = end - start;
                if d + ref_len > stop {
                    try_fail!('main_loop);
                    continue;
                }

                // Compare the backreference text
                let mut matched = true;
                for i in 0..ref_len {
                    if tr(text[d + i].into()) != tr(text[start + i].into()) {
                        matched = false;
                        break;
                    }
                }
                if !matched {
                    try_fail!('main_loop);
                    continue;
                }
                d += ref_len;
            }

            RegexOp::BegLine => {
                if d == 0 || (d > 0 && text[d - 1] == b'\n') {
                    // At beginning of line — succeed
                } else {
                    try_fail!('main_loop);
                }
            }

            RegexOp::EndLine => {
                if d >= text.len() || text[d] == b'\n' {
                    // At end of line — succeed
                } else {
                    try_fail!('main_loop);
                }
            }

            RegexOp::BegBuf => {
                if d != 0 {
                    try_fail!('main_loop);
                }
            }

            RegexOp::EndBuf => {
                if d != text.len() {
                    try_fail!('main_loop);
                }
            }

            RegexOp::AtDot => {
                if d != point {
                    try_fail!('main_loop);
                }
            }

            RegexOp::WordBound => {
                if !at_word_boundary(d) {
                    try_fail!('main_loop);
                }
            }

            RegexOp::NotWordBound => {
                if at_word_boundary(d) {
                    try_fail!('main_loop);
                }
            }

            RegexOp::WordBeg => {
                // Word beginning: not in word before, in word after
                let prev_word = prev_char_start(d)
                    .and_then(|p| text_char(p))
                    .map(|(c, _)| syntax.char_syntax(syntax_char(c)) == SyntaxClass::Word)
                    .unwrap_or(false);
                let curr_word = text_char(d)
                    .map(|(c, _)| syntax.char_syntax(syntax_char(c)) == SyntaxClass::Word)
                    .unwrap_or(false);
                if prev_word || !curr_word {
                    try_fail!('main_loop);
                }
            }

            RegexOp::WordEnd => {
                let prev_word = prev_char_start(d)
                    .and_then(|p| text_char(p))
                    .map(|(c, _)| syntax.char_syntax(syntax_char(c)) == SyntaxClass::Word)
                    .unwrap_or(false);
                let curr_word = text_char(d)
                    .map(|(c, _)| syntax.char_syntax(syntax_char(c)) == SyntaxClass::Word)
                    .unwrap_or(false);
                if !prev_word || curr_word {
                    try_fail!('main_loop);
                }
            }

            RegexOp::SymBeg => {
                let is_sym = |c: u32| {
                    let s = syntax.char_syntax(syntax_char(c));
                    s == SyntaxClass::Word || s == SyntaxClass::Symbol
                };
                let prev_sym = prev_char_start(d)
                    .and_then(|p| text_char(p))
                    .map(|(c, _)| is_sym(c))
                    .unwrap_or(false);
                let curr_sym = text_char(d).map(|(c, _)| is_sym(c)).unwrap_or(false);
                if prev_sym || !curr_sym {
                    try_fail!('main_loop);
                }
            }

            RegexOp::SymEnd => {
                let is_sym = |c: u32| {
                    let s = syntax.char_syntax(syntax_char(c));
                    s == SyntaxClass::Word || s == SyntaxClass::Symbol
                };
                let prev_sym = prev_char_start(d)
                    .and_then(|p| text_char(p))
                    .map(|(c, _)| is_sym(c))
                    .unwrap_or(false);
                let curr_sym = text_char(d).map(|(c, _)| is_sym(c)).unwrap_or(false);
                if !prev_sym || curr_sym {
                    try_fail!('main_loop);
                }
            }

            RegexOp::SyntaxSpec => {
                let class_byte = bytecode[pc];
                pc += 1;
                if d >= stop {
                    try_fail!('main_loop);
                    continue;
                }
                let Some((c, len)) = text_char(d) else {
                    try_fail!('main_loop);
                    continue;
                };
                if syntax.char_syntax(syntax_char(c)) as u8 != class_byte {
                    try_fail!('main_loop);
                    continue;
                }
                d += len;
            }

            RegexOp::NotSyntaxSpec => {
                let class_byte = bytecode[pc];
                pc += 1;
                if d >= stop {
                    try_fail!('main_loop);
                    continue;
                }
                let Some((c, len)) = text_char(d) else {
                    try_fail!('main_loop);
                    continue;
                };
                if syntax.char_syntax(syntax_char(c)) as u8 == class_byte {
                    try_fail!('main_loop);
                    continue;
                }
                d += len;
            }

            RegexOp::CategorySpec => {
                let cat = bytecode[pc];
                pc += 1;
                if d >= stop {
                    try_fail!('main_loop);
                    continue;
                }
                let Some((c, len)) = text_char(d) else {
                    try_fail!('main_loop);
                    continue;
                };
                if !syntax.char_has_category(syntax_char(c), cat) {
                    try_fail!('main_loop);
                    continue;
                }
                d += len;
            }

            RegexOp::NotCategorySpec => {
                let cat = bytecode[pc];
                pc += 1;
                if d >= stop {
                    try_fail!('main_loop);
                    continue;
                }
                let Some((c, len)) = text_char(d) else {
                    try_fail!('main_loop);
                    continue;
                };
                if syntax.char_has_category(syntax_char(c), cat) {
                    try_fail!('main_loop);
                    continue;
                }
                d += len;
            }

            RegexOp::Jump => {
                // Mirrors GNU `regex-emacs.c:4901`: poll quit at the
                // unconditional-jump site inside the matcher bytecode
                // dispatch loop. Gives interactive `C-g` a chance to
                // abort a pathological regex that would otherwise run
                // for many seconds on a large input.
                if crate::emacs_core::eval::tls_quit_pending() {
                    return None;
                }
                let offset = extract_number(bytecode, pc);
                pc = ((pc as i64) + 2 + (offset as i64)) as usize;
            }

            RegexOp::OnFailureJump => {
                let offset = extract_number(bytecode, pc);
                pc += 2;
                let fail_pc = ((pc as i64) + (offset as i64)) as usize;
                fail_stack.push(FailurePoint {
                    pattern_pos: fail_pc,
                    string_pos: Some(d),
                    saved_registers: save_registers(&regstart, &regend, num_regs),
                    saved_counters: counters.clone(),
                });
            }

            RegexOp::OnFailureKeepStringJump => {
                let offset = extract_number(bytecode, pc);
                pc += 2;
                let fail_pc = ((pc as i64) + (offset as i64)) as usize;
                fail_stack.push(FailurePoint {
                    pattern_pos: fail_pc,
                    string_pos: None, // Don't restore string position
                    saved_registers: save_registers(&regstart, &regend, num_regs),
                    saved_counters: counters.clone(),
                });
            }

            RegexOp::OnFailureJumpLoop => {
                let offset = extract_number(bytecode, pc);
                pc += 2;
                let fail_pc = ((pc as i64) + (offset as i64)) as usize;
                // Check for infinite loop (empty match detection)
                let already_at_same_pos = fail_stack
                    .last()
                    .is_some_and(|fp| fp.string_pos == Some(d) && fp.pattern_pos == fail_pc);
                if already_at_same_pos {
                    // Would loop forever on empty match — skip the loop
                    pc = fail_pc;
                } else {
                    fail_stack.push(FailurePoint {
                        pattern_pos: fail_pc,
                        string_pos: Some(d),
                        saved_registers: save_registers(&regstart, &regend, num_regs),
                        saved_counters: counters.clone(),
                    });
                }
            }

            RegexOp::OnFailureJumpNastyloop => {
                // Same as OnFailureJumpLoop but for non-greedy
                let offset = extract_number(bytecode, pc);
                pc += 2;
                let fail_pc = ((pc as i64) + (offset as i64)) as usize;
                fail_stack.push(FailurePoint {
                    pattern_pos: fail_pc,
                    string_pos: Some(d),
                    saved_registers: save_registers(&regstart, &regend, num_regs),
                    saved_counters: counters.clone(),
                });
            }

            RegexOp::OnFailureJumpSmart => {
                // Smart greedy optimization — treated same as OnFailureJump
                let offset = extract_number(bytecode, pc);
                pc += 2;
                let fail_pc = ((pc as i64) + (offset as i64)) as usize;
                fail_stack.push(FailurePoint {
                    pattern_pos: fail_pc,
                    string_pos: Some(d),
                    saved_registers: save_registers(&regstart, &regend, num_regs),
                    saved_counters: counters.clone(),
                });
            }

            RegexOp::SucceedN => {
                // GNU: succeed_n  <jump-offset:2> <counter:2>
                // "Have to succeed matching what follows at least n times."
                // Counter lives at pc+2 (2 bytes).  When counter > 0 we
                // decrement and continue (must still succeed more times).
                // When counter == 0 we fall through to on_failure_jump_loop
                // semantics using the jump offset.
                let counter_pos = pc + 2; // bytecode position of the counter
                let count = get_counter(&counters, bytecode, counter_pos);
                if count != 0 {
                    // Still must succeed more times — decrement & continue
                    set_counter(&mut counters, counter_pos, count - 1);
                    pc += 4;
                } else {
                    // Counter exhausted — behave like on_failure_jump_loop.
                    // Read the jump offset and push a failure point.
                    let offset = extract_number(bytecode, pc);
                    pc += 2; // skip the offset field
                    let fail_pc = ((pc as i64) + (offset as i64)) as usize;
                    pc += 2; // skip the counter field
                    // Infinite-loop detection (same as OnFailureJumpLoop)
                    let already_at_same_pos = fail_stack
                        .last()
                        .is_some_and(|fp| fp.string_pos == Some(d) && fp.pattern_pos == fail_pc);
                    if already_at_same_pos {
                        pc = fail_pc;
                    } else {
                        fail_stack.push(FailurePoint {
                            pattern_pos: fail_pc,
                            string_pos: Some(d),
                            saved_registers: save_registers(&regstart, &regend, num_regs),
                            saved_counters: counters.clone(),
                        });
                    }
                }
            }

            RegexOp::JumpN => {
                // GNU: jump_n  <jump-offset:2> <counter:2>
                // "Originally, this is how many times we CAN jump."
                // If counter > 0, decrement and jump.
                // If counter == 0, skip past (don't jump).
                let counter_pos = pc + 2;
                let count = get_counter(&counters, bytecode, counter_pos);
                if count != 0 {
                    // Decrement counter and perform unconditional jump
                    set_counter(&mut counters, counter_pos, count - 1);
                    let offset = extract_number(bytecode, pc);
                    pc = ((pc as i64) + 2 + (offset as i64)) as usize;
                } else {
                    pc += 4; // Skip past offset + counter fields
                }
            }

            RegexOp::SetNumberAt => {
                // GNU: set_number_at  <offset-to-counter:2> <value:2>
                // Sets the counter at the given offset to the given value.
                // Used to reset interval counters at the start of a loop.
                let rel_offset = extract_number(bytecode, pc);
                pc += 2; // advance past the offset field
                let value = extract_number(bytecode, pc);
                pc += 2; // advance past the value field
                // Target counter position: relative to position after
                // the offset field (same convention as GNU).
                let target_pos = ((pc as i64) - 2 + (rel_offset as i64)) as usize;
                set_counter(&mut counters, target_pos, value);
            }
        }
    }

    // GNU regex-emacs.c:5278-5279: when the matcher breaks out of
    // the main loop due to total backtracking exhaustion, if a best
    // match was previously saved for POSIX longest-match, restore it
    // and fall through to the success path; otherwise there is no
    // match at all.
    if total_failure {
        if best_regs_set {
            d = best_match_end;
            for i in 1..num_regs {
                regstart[i] = best_regstart[i];
                regend[i] = best_regend[i];
            }
        } else {
            return None;
        }
    }

    // If we got here, we matched!
    // Fill in registers
    let mut regs = MatchRegisters::new(num_regs);
    regs.start[0] = pos as i64;
    regs.end[0] = d as i64;
    for i in 1..num_regs {
        regs.start[i] = regstart
            .get(i)
            .copied()
            .flatten()
            .map(|v| v as i64)
            .unwrap_or(-1);
        regs.end[i] = regend
            .get(i)
            .copied()
            .flatten()
            .map(|v| v as i64)
            .unwrap_or(-1);
    }

    Some((d, regs))
}

/// Save current register state for backtracking.
fn save_registers(
    regstart: &[Option<usize>],
    regend: &[Option<usize>],
    num_regs: usize,
) -> SavedRegisters {
    let mut saved = SavedRegisters::new();
    for i in 1..num_regs.min(regstart.len()).min(regend.len()) {
        saved.push((
            i,
            regstart[i].map(|v| v as i64).unwrap_or(-1),
            regend[i].map(|v| v as i64).unwrap_or(-1),
        ));
    }
    saved
}

/// Restore register state from a failure point.
fn restore_registers(
    fp: &FailurePoint,
    regstart: &mut [Option<usize>],
    regend: &mut [Option<usize>],
) {
    for &(idx, start, end) in &fp.saved_registers {
        if idx < regstart.len() {
            regstart[idx] = if start >= 0 {
                Some(start as usize)
            } else {
                None
            };
        }
        if idx < regend.len() {
            regend[idx] = if end >= 0 { Some(end as usize) } else { None };
        }
    }
}

/// Handle match failure — pop the failure stack and backtrack.
/// Returns None if the failure stack is empty (complete failure).
fn goto_fail(
    pc: &mut usize,
    d: &mut usize,
    fail_stack: &mut Vec<FailurePoint>,
    regstart: &mut [Option<usize>],
    regend: &mut [Option<usize>],
    counters: &mut HashMap<usize, i16>,
) -> Option<()> {
    // Mirrors GNU `regex-emacs.c:5236`: poll quit at the failure /
    // backtrack site. Backtracking loops are the worst offenders for
    // pathological-regex responsiveness; a `C-g` arriving mid-backtrack
    // aborts the entire search here so the evaluator can surface the
    // quit signal on its next `maybe_quit` poll.
    if crate::emacs_core::eval::tls_quit_pending() {
        return None;
    }
    let fp = fail_stack.pop()?;
    *pc = fp.pattern_pos;
    if let Some(sp) = fp.string_pos {
        *d = sp;
    }
    restore_registers(&fp, regstart, regend);
    // Restore interval counters to the state when this failure point was pushed
    *counters = fp.saved_counters;
    Some(())
}

fn register_scratch(num_regs: usize) -> RegisterScratch {
    let mut scratch = RegisterScratch::new();
    scratch.resize(num_regs, None);
    scratch
}

// ---------------------------------------------------------------------------
// Phase 4: Searcher (re_search_2)
//
// Translates GNU regex-emacs.c:3408-4070.
// Searches for a match in text, using fastmap for optimization.
// ---------------------------------------------------------------------------

/// Analyze compiled bytecode to populate `pattern.fastmap`.
///
/// For each byte value `c` that could possibly appear as the first byte of a
/// match, sets `pattern.fastmap[c] = true`.  The searcher (`re_search`) uses
/// this to skip positions that cannot start a match, giving a significant
/// speed-up for patterns that begin with a restricted set of characters.
///
/// Populate `fastmap` for `\sX` (or `\SX` when `negate` is true) by
/// querying the standard syntax table for every ASCII byte. Mirrors
/// GNU regex-emacs.c:3170-3186 which iterates the same range and
/// consults the buffer's actual syntax table. We don't have a per-
/// buffer syntax table at compile time so we fall back to the
/// standard one — that matches GNU's behavior for all the standard
/// classes (Whitespace, Punctuation, Word, Symbol, Open, Close, ...)
/// for ASCII bytes. Audit finding #16 in
/// `drafts/regex-search-audit.md`.
fn fastmap_for_syntax_class(fastmap: &mut [bool; 256], class_byte: u8, negate: bool) {
    let target = match crate::emacs_core::syntax::SyntaxClass::from_code(class_byte as i64) {
        Some(cls) => cls,
        None => {
            // Unknown class — conservatively allow every byte
            // (matches GNU's "fall through to set all" behavior).
            *fastmap = [true; 256];
            return;
        }
    };
    let table = crate::emacs_core::syntax::SyntaxTable::new_standard();
    for c in 0u8..=127 {
        let in_class = table.char_syntax(c as char) == target;
        if in_class != negate {
            fastmap[c as usize] = true;
        }
    }
    // Conservatively allow every non-ASCII byte. The matcher will do
    // the real per-character syntax lookup at match time.
    for c in 128..256usize {
        fastmap[c] = true;
    }
}

/// Translated from GNU regex-emacs.c `re_compile_fastmap`.
fn compile_fastmap(pattern: &mut CompiledPattern) {
    pattern.fastmap = [false; 256];
    pattern.can_be_null = false;

    let bytecode = &pattern.buffer;
    if bytecode.is_empty() {
        pattern.can_be_null = true;
        pattern.fastmap_accurate = true;
        return;
    }

    let case_fold = pattern.translate.is_some();

    // Worklist of bytecode positions still to process.
    let mut worklist: Vec<usize> = vec![0];
    let mut visited: HashSet<usize> = HashSet::new();

    while let Some(pc) = worklist.pop() {
        let mut pc = pc;

        loop {
            if !visited.insert(pc) {
                // Already processed this position — avoid infinite loops.
                break;
            }

            if pc >= bytecode.len() {
                // Fell off the end of bytecode — pattern can match empty string.
                pattern.can_be_null = true;
                break;
            }

            let Some(op) = RegexOp::from_byte(bytecode[pc]) else {
                break;
            };
            pc += 1;

            match op {
                RegexOp::Succeed => {
                    // Pattern can succeed here (may match empty string).
                    pattern.can_be_null = true;
                    break;
                }

                RegexOp::Exactn => {
                    if pc >= bytecode.len() {
                        break;
                    }
                    let count = bytecode[pc] as usize;
                    pc += 1;
                    if count == 0 || pc >= bytecode.len() {
                        break;
                    }
                    let first = bytecode[pc];
                    pattern.fastmap[first as usize] = true;
                    if case_fold {
                        if first >= 0x80 {
                            // Multibyte character: the case-folded form may have
                            // a different leading byte (e.g. Cyrillic
                            // т = D1 82 vs Т = D0 A2), so byte-level case-folding
                            // of `first` is meaningless and would wrongly exclude
                            // the other case's lead byte. Conservatively allow all
                            // multibyte leading bytes, matching the Charset path.
                            for c in 128..256usize {
                                pattern.fastmap[c] = true;
                            }
                        } else {
                            let upper = (first as char)
                                .to_uppercase()
                                .next()
                                .unwrap_or(first as char)
                                as u8;
                            let lower = (first as char)
                                .to_lowercase()
                                .next()
                                .unwrap_or(first as char)
                                as u8;
                            pattern.fastmap[upper as usize] = true;
                            pattern.fastmap[lower as usize] = true;
                        }
                    }
                    break; // This opcode consumes input — done on this path.
                }

                RegexOp::AnyChar => {
                    // Matches any character except newline.
                    for c in 0..256usize {
                        if c != b'\n' as usize {
                            pattern.fastmap[c] = true;
                        }
                    }
                    break;
                }

                RegexOp::Charset => {
                    if pc >= bytecode.len() {
                        break;
                    }
                    let charset_op_pos = pc - 1;
                    let bitmap_len = bytecode[pc] as usize & 0x7F;
                    pc += 1;
                    for c in 0..256usize {
                        if c / 8 < bitmap_len && pc + c / 8 < bytecode.len() {
                            if (bytecode[pc + c / 8] >> (c % 8)) & 1 != 0 {
                                pattern.fastmap[c] = true;
                            }
                        }
                    }
                    // If this charset has multibyte ranges, conservatively
                    // allow all non-ASCII leading bytes in the fastmap.
                    if pattern.multibyte_charsets.contains_key(&charset_op_pos) {
                        for c in 128..256usize {
                            pattern.fastmap[c] = true;
                        }
                    }
                    break;
                }

                RegexOp::CharsetNot => {
                    if pc >= bytecode.len() {
                        break;
                    }
                    let charset_op_pos = pc - 1;
                    let bitmap_len = bytecode[pc] as usize & 0x7F;
                    pc += 1;
                    for c in 0..256usize {
                        let in_set = if c / 8 < bitmap_len && pc + c / 8 < bytecode.len() {
                            (bytecode[pc + c / 8] >> (c % 8)) & 1 != 0
                        } else {
                            false
                        };
                        if !in_set {
                            pattern.fastmap[c] = true;
                        }
                    }
                    break;
                }

                RegexOp::SyntaxSpec => {
                    if pc >= bytecode.len() {
                        break;
                    }
                    // GNU `re_compile_fastmap` consults the buffer's
                    // syntax table for `\sX` (regex-emacs.c:3170-3186).
                    // We don't have a per-buffer table at compile
                    // time so we use the standard one. The previous
                    // body hardcoded Rust's Unicode `is_whitespace` /
                    // `is_alphanumeric` and silently dropped classes
                    // 4-15, so any pattern using `\s(`, `\s)`, `\s\"`
                    // etc. went down a wrong fastmap path. See audit
                    // finding #16 in `drafts/regex-search-audit.md`.
                    fastmap_for_syntax_class(&mut pattern.fastmap, bytecode[pc], false);
                    break;
                }

                RegexOp::NotSyntaxSpec => {
                    if pc >= bytecode.len() {
                        break;
                    }
                    fastmap_for_syntax_class(&mut pattern.fastmap, bytecode[pc], true);
                    break;
                }

                RegexOp::CategorySpec | RegexOp::NotCategorySpec => {
                    // Categories are too dynamic to predict — allow all bytes.
                    pattern.fastmap = [true; 256];
                    break;
                }

                // Zero-width assertions: they don't consume input, so we
                // continue to the next opcode to find what actually starts
                // the match.
                RegexOp::BegLine
                | RegexOp::EndLine
                | RegexOp::BegBuf
                | RegexOp::EndBuf
                | RegexOp::AtDot
                | RegexOp::WordBound
                | RegexOp::NotWordBound
                | RegexOp::WordBeg
                | RegexOp::WordEnd
                | RegexOp::SymBeg
                | RegexOp::SymEnd => {
                    // Continue to the next opcode.
                }

                RegexOp::StartMemory | RegexOp::StopMemory => {
                    // Skip the group number byte, continue.
                    pc += 1;
                }

                RegexOp::Duplicate => {
                    // Backreferences can match anything — set all.
                    pattern.fastmap = [true; 256];
                    break;
                }

                RegexOp::NoOp => {
                    // Continue to next opcode.
                }

                RegexOp::Jump => {
                    if pc + 1 >= bytecode.len() {
                        break;
                    }
                    let offset = extract_number(bytecode, pc);
                    pc = ((pc as i64) + 2 + (offset as i64)) as usize;
                    // Continue walking from the jump target (don't break).
                }

                RegexOp::OnFailureJump
                | RegexOp::OnFailureKeepStringJump
                | RegexOp::OnFailureJumpLoop
                | RegexOp::OnFailureJumpNastyloop
                | RegexOp::OnFailureJumpSmart => {
                    if pc + 1 >= bytecode.len() {
                        break;
                    }
                    let offset = extract_number(bytecode, pc);
                    pc += 2;
                    // Both the fallthrough (next opcode) and the jump target
                    // can start a match.  Push the jump target onto the
                    // worklist and continue with the fallthrough.
                    let target = ((pc as i64) + (offset as i64)) as usize;
                    worklist.push(target);
                    // Continue with the next opcode (fallthrough path).
                }

                RegexOp::SucceedN => {
                    // succeed_n <offset:2> <counter:2>
                    // When counter > 0, acts like a mandatory match of what follows.
                    // When counter == 0, acts like on_failure_jump.
                    // For fastmap purposes, both paths are possible.
                    if pc + 3 >= bytecode.len() {
                        break;
                    }
                    let offset = extract_number(bytecode, pc);
                    let target = ((pc as i64) + 2 + (offset as i64)) as usize;
                    worklist.push(target);
                    pc += 4; // skip offset + counter, continue with fallthrough
                }

                RegexOp::JumpN => {
                    // jump_n <offset:2> <counter:2>
                    // If counter > 0, jumps; if counter == 0, falls through.
                    // For fastmap, both paths are possible.
                    if pc + 3 >= bytecode.len() {
                        break;
                    }
                    let offset = extract_number(bytecode, pc);
                    let target = ((pc as i64) + 2 + (offset as i64)) as usize;
                    worklist.push(target);
                    pc += 4; // fallthrough
                }

                RegexOp::SetNumberAt => {
                    // set_number_at <offset:2> <value:2> — no input consumed.
                    pc += 4;
                }
            }
        }
    }

    pattern.fastmap_accurate = true;
}

/// Search for a match of the compiled pattern in text.
///
/// Equivalent to GNU's `re_search_2()` operating on a single
/// contiguous string. GNU also exposes the two-string variant
/// `re_match_2(pattern, string1, size1, string2, size2, ...)` which
/// walks the buffer text across the gap boundary
/// (`BEG..GPT` and `GPT..ZV`) without copying — for a 100MB buffer
/// that saves a 100MB allocation per search. Audit finding #17 in
/// `drafts/regex-search-audit.md` flags this as missing in neomacs.
///
/// We currently allocate the full buffer text via
/// `Buffer::buffer_substring_range(Buffer::accessible_emacs_byte_range())` at
/// the call site in `regex.rs::re_search_forward_with_posix` and friends, which is
/// correctness-equivalent to GNU's `re_match_2_internal` running
/// over a single string but is O(buffer-size) per search instead of
/// O(match-length). Porting the gap-aware path is a separate
/// optimization (audit Phase D Task 4.1, ~1 day).
///
/// # Arguments
/// * `pattern` - Compiled pattern
/// * `text` - Input text
/// * `start` - Starting search position
/// * `range` - How far to search (positive = forward, negative = backward)
/// * `syntax` - Syntax table lookup
/// * `point` - Buffer point (for `\=`)
///
/// # Returns
/// * `Some((match_start, registers))` if found
/// * `None` if no match
pub(crate) fn re_search(
    pattern: &CompiledPattern,
    text: &[u8],
    start: usize,
    range: isize,
    syntax: &dyn SyntaxLookup,
    point: usize,
) -> Option<(usize, MatchRegisters)> {
    let text_len = text.len();
    let use_fastmap = pattern.fastmap_accurate && !pattern.can_be_null && !pattern.uses_syntax;
    let translate = pattern.translate.as_ref();

    if range >= 0 {
        // Forward search
        let end = (start + range as usize).min(text_len);
        let mut pos = start;
        if use_fastmap {
            if let Some(table) = translate {
                while pos <= end {
                    if pos > text_len {
                        break;
                    }
                    // Skip UTF-8 continuation bytes — only try match at character
                    // boundaries to avoid matching in the middle of a multibyte char.
                    if pattern.target_multibyte && pos < text_len && (text[pos] & 0xC0) == 0x80 {
                        pos += 1;
                        continue;
                    }
                    // GNU disables fastmap skipping for nullable patterns so zero-width
                    // matches like `\\(?:...\\)\\=` are still considered at every point.
                    //
                    // GNU regex-emacs.c:3568 applies TRANSLATE to the input
                    // byte before indexing the fastmap. Under case-fold that
                    // is what lets a fastmap built for a bitmap of lowercase
                    // characters still catch uppercase input (audit #9).
                    if pos < text_len {
                        let idx = table.translate_byte(text[pos]) as usize;
                        if !pattern.fastmap[idx] {
                            pos += 1;
                            continue;
                        }
                    }
                    if let Some(result) = re_match(pattern, text, pos, end, syntax, point) {
                        return Some((pos, result.1));
                    }
                    pos += 1;
                }
            } else {
                while pos <= end {
                    if pos > text_len {
                        break;
                    }
                    if pattern.target_multibyte && pos < text_len && (text[pos] & 0xC0) == 0x80 {
                        pos += 1;
                        continue;
                    }
                    if pos < text_len && !pattern.fastmap[text[pos] as usize] {
                        pos += 1;
                        continue;
                    }
                    if let Some(result) = re_match(pattern, text, pos, end, syntax, point) {
                        return Some((pos, result.1));
                    }
                    pos += 1;
                }
            };
        } else {
            while pos <= end {
                if pos > text_len {
                    break;
                }
                if pattern.target_multibyte && pos < text_len && (text[pos] & 0xC0) == 0x80 {
                    pos += 1;
                    continue;
                }
                if let Some(result) = re_match(pattern, text, pos, end, syntax, point) {
                    return Some((pos, result.1));
                }
                pos += 1;
            }
        }
    } else {
        // Backward search
        let end = start.saturating_sub((-range) as usize);
        if use_fastmap {
            if let Some(table) = translate {
                for pos in (end..=start).rev() {
                    // Skip UTF-8 continuation bytes — only try at character boundaries.
                    if pattern.target_multibyte && pos < text_len && (text[pos] & 0xC0) == 0x80 {
                        continue;
                    }
                    // GNU disables fastmap skipping for nullable patterns so zero-width
                    // matches like `\\(?:...\\)\\=` are still considered at every point.
                    if pos < text_len {
                        let idx = table.translate_byte(text[pos]) as usize;
                        if !pattern.fastmap[idx] {
                            continue;
                        }
                    }
                    // GNU `search.c:1195-1201` calls `re_search_2` for backward
                    // searches with STOP set to the point where the search began.
                    // That means a candidate may start before `start`, but it may
                    // not extend past it.  This prevents a repeated backward search
                    // from re-matching the same non-empty match that begins at
                    // point but ends after it.
                    if let Some(result) = re_match(pattern, text, pos, start, syntax, point) {
                        return Some((pos, result.1));
                    }
                }
            } else {
                for pos in (end..=start).rev() {
                    if pattern.target_multibyte && pos < text_len && (text[pos] & 0xC0) == 0x80 {
                        continue;
                    }
                    if pos < text_len && !pattern.fastmap[text[pos] as usize] {
                        continue;
                    }
                    if let Some(result) = re_match(pattern, text, pos, start, syntax, point) {
                        return Some((pos, result.1));
                    }
                }
            }
        } else {
            for pos in (end..=start).rev() {
                // Skip UTF-8 continuation bytes — only try at character boundaries.
                if pattern.target_multibyte && pos < text_len && (text[pos] & 0xC0) == 0x80 {
                    continue;
                }
                // GNU `search.c:1195-1201` calls `re_search_2` for backward
                // searches with STOP set to the point where the search began.
                // That means a candidate may start before `start`, but it may
                // not extend past it.  This prevents a repeated backward search
                // from re-matching the same non-empty match that begins at
                // point but ends after it.
                if let Some(result) = re_match(pattern, text, pos, start, syntax, point) {
                    return Some((pos, result.1));
                }
            }
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Convenience: compile + search in one call
// ---------------------------------------------------------------------------

/// Compile a pattern and search for it in text.
pub(crate) fn search_pattern(
    pattern_str: &str,
    text: &str,
    start: usize,
    case_fold: bool,
    syntax: &dyn SyntaxLookup,
    point: usize,
) -> Result<Option<(usize, MatchRegisters)>, RegexCompileError> {
    let compiled = regex_compile(pattern_str, false, case_fold)?;
    Ok(re_search(
        &compiled,
        text.as_bytes(),
        start,
        (text.len() - start) as isize,
        syntax,
        point,
    ))
}

/// Compile a pattern and match at a specific position.
pub(crate) fn match_pattern(
    pattern_str: &str,
    text: &str,
    pos: usize,
    case_fold: bool,
    syntax: &dyn SyntaxLookup,
    point: usize,
) -> Result<Option<(usize, MatchRegisters)>, RegexCompileError> {
    let compiled = regex_compile(pattern_str, false, case_fold)?;
    Ok(re_match(
        &compiled,
        text.as_bytes(),
        pos,
        text.len(),
        syntax,
        point,
    ))
}

#[cfg(test)]
#[path = "regex_emacs_test.rs"]
mod tests;

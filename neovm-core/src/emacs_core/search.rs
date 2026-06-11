//! Search and regex builtins for the Elisp interpreter.
//!
//! Pure builtins:
//! - `string-match`, `string-match-p`, `regexp-quote`
//! - `match-beginning`, `match-end`, `match-data`, `set-match-data`
//! - `looking-at`, `looking-at-p`, `replace-regexp-in-string`
//!
//! Eval-dependent builtins:
//! - `search-forward`, `search-backward`
//! - `re-search-forward`, `re-search-backward`
//! - `posix-search-forward`, `posix-search-backward`
//! - `replace-match`
//! - `word-search-forward`, `word-search-backward`

use super::error::{EvalResult, Flow, signal};
use super::intern::intern;
use super::regex::MatchGroup;
use super::value::*;
use crate::buffer::{CharLen, CharPos0, EmacsBytePos};
use crate::emacs_core::value::ValueKind;

// ---------------------------------------------------------------------------
// Argument helpers
// ---------------------------------------------------------------------------

fn expect_args(name: &str, args: &[Value], n: usize) -> Result<(), Flow> {
    if args.len() != n {
        Err(signal(
            "wrong-number-of-arguments",
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

fn expect_min_args(name: &str, args: &[Value], min: usize) -> Result<(), Flow> {
    if args.len() < min {
        Err(signal(
            "wrong-number-of-arguments",
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

#[inline]
fn buffer_match_char_pos_to_byte_pos(
    buf: &crate::buffer::Buffer,
    lisp_char_pos: usize,
) -> EmacsBytePos {
    buf.char_pos_to_emacs_byte_pos_clamped(
        CharPos0::new(lisp_char_pos).saturating_sub_len(CharLen::new(1)),
    )
}

fn expect_range_args(name: &str, args: &[Value], min: usize, max: usize) -> Result<(), Flow> {
    if args.len() < min || args.len() > max {
        Err(signal(
            "wrong-number-of-arguments",
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

fn expect_int(val: &Value) -> Result<i64, Flow> {
    match val.kind() {
        ValueKind::Fixnum(n) => Ok(n),
        other => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("integerp"), *val],
        )),
    }
}

fn expect_fixnum(val: &Value) -> Result<i64, Flow> {
    match val.kind() {
        ValueKind::Fixnum(n) => Ok(n),
        other => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("fixnump"), *val],
        )),
    }
}

fn expect_integer_or_marker(val: &Value) -> Result<i64, Flow> {
    match val.kind() {
        ValueKind::Fixnum(n) => Ok(n),
        other => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("integer-or-marker-p"), *val],
        )),
    }
}

fn expect_string(val: &Value) -> Result<String, Flow> {
    match val.kind() {
        ValueKind::String => Ok(val
            .as_runtime_string_owned()
            .expect("ValueKind::String must carry LispString payload")),
        other => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("stringp"), *val],
        )),
    }
}

fn expect_lisp_string(val: &Value) -> Result<&'static crate::heap_types::LispString, Flow> {
    val.as_lisp_string()
        .ok_or_else(|| signal("wrong-type-argument", vec![Value::symbol("stringp"), *val]))
}

fn cloned_lisp_string_value(string: &crate::heap_types::LispString) -> Value {
    Value::heap_string(string.clone())
}

fn regexp_quote_lisp_string(
    input: &crate::heap_types::LispString,
) -> crate::heap_types::LispString {
    let mut out = Vec::with_capacity(input.as_bytes().len() + 8);
    for &byte in input.as_bytes() {
        match byte {
            b'.' | b'*' | b'+' | b'?' | b'[' | b'^' | b'$' | b'\\' => {
                out.push(b'\\');
                out.push(byte);
            }
            _ => out.push(byte),
        }
    }

    if input.is_multibyte() {
        crate::heap_types::LispString::from_emacs_bytes(out)
    } else {
        crate::heap_types::LispString::from_unibyte(out)
    }
}

fn normalize_string_start_arg(string: &str, start: Option<&Value>) -> Result<usize, Flow> {
    let Some(start_val) = start else {
        return Ok(0);
    };
    if start_val.is_nil() {
        return Ok(0);
    }

    let raw_start = expect_fixnum(start_val)?;
    let string_bytes = string.as_bytes();
    let len = crate::emacs_core::emacs_char::chars_in_multibyte(string_bytes) as i64;
    let normalized = if raw_start < 0 {
        len.checked_add(raw_start)
    } else {
        Some(raw_start)
    };

    let Some(start_idx) = normalized else {
        return Err(signal(
            "args-out-of-range",
            vec![Value::string(string), Value::fixnum(raw_start)],
        ));
    };

    if !(0..=len).contains(&start_idx) {
        return Err(signal(
            "args-out-of-range",
            vec![Value::string(string), Value::fixnum(raw_start)],
        ));
    }

    let start_char_idx = start_idx as usize;
    if start_char_idx == len as usize {
        return Ok(string.len());
    }

    Ok(crate::emacs_core::emacs_char::char_to_byte_pos(
        string_bytes,
        start_char_idx,
    ))
}

pub(crate) fn normalize_lisp_string_start_arg(
    string: &crate::heap_types::LispString,
    start: Option<&Value>,
) -> Result<usize, Flow> {
    let Some(start_val) = start else {
        return Ok(0);
    };
    if start_val.is_nil() {
        return Ok(0);
    }

    let raw_start = expect_fixnum(start_val)?;
    if !string.is_multibyte() {
        let len = string.byte_len() as i64;
        let normalized = if raw_start < 0 {
            len.checked_add(raw_start)
        } else {
            Some(raw_start)
        };
        let Some(start_idx) = normalized else {
            return Err(signal(
                "args-out-of-range",
                vec![cloned_lisp_string_value(string), Value::fixnum(raw_start)],
            ));
        };
        if !(0..=len).contains(&start_idx) {
            return Err(signal(
                "args-out-of-range",
                vec![cloned_lisp_string_value(string), Value::fixnum(raw_start)],
            ));
        }
        return Ok(start_idx as usize);
    }

    let len = string.schars() as i64;
    let normalized = if raw_start < 0 {
        len.checked_add(raw_start)
    } else {
        Some(raw_start)
    };
    let Some(start_idx) = normalized else {
        return Err(signal(
            "args-out-of-range",
            vec![cloned_lisp_string_value(string), Value::fixnum(raw_start)],
        ));
    };
    if !(0..=len).contains(&start_idx) {
        return Err(signal(
            "args-out-of-range",
            vec![cloned_lisp_string_value(string), Value::fixnum(raw_start)],
        ));
    }
    let start_char_idx = start_idx as usize;
    if start_char_idx == len as usize {
        return Ok(string.byte_len());
    }
    Ok(crate::emacs_core::emacs_char::char_to_byte_pos(
        string.as_bytes(),
        start_char_idx,
    ))
}

fn flatten_match_data(md: &super::regex::MatchData) -> Value {
    let mut trailing = md.groups.len();
    while trailing > 0 && md.groups[trailing - 1].is_none() {
        trailing -= 1;
    }

    let mut flat: Vec<Value> = Vec::with_capacity(trailing * 2);
    for grp in md.groups.iter().take(trailing) {
        match grp {
            Some(group) => {
                // For string searches, positions are already character positions.
                // For buffer searches, positions are byte positions (returned as-is).
                flat.push(Value::fixnum(group.start() as i64));
                flat.push(Value::fixnum(group.end() as i64));
            }
            None => {
                flat.push(Value::NIL);
                flat.push(Value::NIL);
            }
        }
    }
    Value::list(flat)
}

// ---------------------------------------------------------------------------
// Pure builtins
// ---------------------------------------------------------------------------

/// `(regexp-quote STRING)` -- return a regexp that matches STRING literally,
/// quoting all special regex characters.
pub(crate) fn builtin_regexp_quote(args: Vec<Value>) -> EvalResult {
    crate::emacs_core::perf_trace::time_op(
        crate::emacs_core::perf_trace::HotpathOp::RegexpQuote,
        || {
            expect_args("regexp-quote", &args, 1)?;
            let string = expect_lisp_string(&args[0])?;
            Ok(Value::heap_string(regexp_quote_lisp_string(string)))
        },
    )
}

fn parse_replace_regexp_subexp_start_lisp(
    args: &[Value],
    string: &crate::heap_types::LispString,
) -> Result<ReplaceRegexpSubexpStart, Flow> {
    let subexp = match args.get(5) {
        Some(v) if v.is_nil() => 0i64,
        None => 0i64,
        Some(value) => expect_int(value)?,
    };
    if subexp < 0 {
        return Err(signal(
            "args-out-of-range",
            vec![
                Value::fixnum(subexp),
                Value::fixnum(0),
                Value::fixnum(string.schars() as i64),
            ],
        ));
    }
    let start = normalize_lisp_string_start_arg(string, args.get(6))?;
    Ok(ReplaceRegexpSubexpStart {
        subexp: subexp as usize,
        start,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ReplaceRegexpSubexpStart {
    subexp: usize,
    start: usize,
}

pub(crate) fn storage_string_from_lisp_string(string: &crate::heap_types::LispString) -> String {
    crate::emacs_core::string_escape::emacs_bytes_to_storage_string(
        string.as_bytes(),
        string.is_multibyte(),
    )
}

fn storage_string_to_lisp_string(text: &str, multibyte: bool) -> crate::heap_types::LispString {
    let bytes = crate::emacs_core::string_escape::storage_string_to_buffer_bytes(text, multibyte);
    if multibyte {
        crate::heap_types::LispString::from_emacs_bytes(bytes)
    } else {
        crate::heap_types::LispString::from_unibyte(bytes)
    }
}

fn translate_match_data_to_substring(
    match_data: &super::regex::MatchData,
    delta: i64,
    searched_string: super::regex::SearchedString,
) -> super::regex::MatchData {
    let mut translated = match_data.clone();
    for group in translated.groups.iter_mut() {
        if let Some(group) = group {
            *group = group.translate_saturating(delta);
        }
    }
    translated.set_string_source(Some(searched_string));
    translated
}

fn replace_match_on_substring(
    source: &crate::heap_types::LispString,
    replacement: &crate::heap_types::LispString,
    fixedcase: bool,
    literal: bool,
    subexp: usize,
    match_data: &Option<super::regex::MatchData>,
) -> Result<crate::heap_types::LispString, Flow> {
    if let Some(md) = match_data
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
    replace_match_lisp_string_with_syntax(
        source,
        replacement,
        fixedcase,
        literal,
        subexp,
        match_data,
    )
    .map_err(|msg| signal("error", vec![Value::string(msg)]))
}

fn concat_lisp_string_pieces(
    pieces: Vec<crate::heap_types::LispString>,
) -> crate::heap_types::LispString {
    let mut iter = pieces.into_iter();
    let Some(mut acc) = iter.next() else {
        return crate::heap_types::LispString::from_unibyte(Vec::new());
    };
    for piece in iter {
        acc = acc.concat(&piece);
    }
    acc
}

fn empty_lisp_string(multibyte: bool) -> crate::heap_types::LispString {
    if multibyte {
        crate::heap_types::LispString::from_emacs_bytes(Vec::new())
    } else {
        crate::heap_types::LispString::from_unibyte(Vec::new())
    }
}

fn lisp_char_at_byte(
    string: &crate::heap_types::LispString,
    byte_pos: usize,
) -> Option<(u32, usize)> {
    if byte_pos >= string.byte_len() {
        return None;
    }
    if string.is_multibyte() {
        Some(crate::emacs_core::emacs_char::string_char(
            &string.as_bytes()[byte_pos..],
        ))
    } else {
        Some((string.as_bytes()[byte_pos] as u32, 1))
    }
}

fn match_group_to_byte_range(
    source: &crate::heap_types::LispString,
    md: &super::regex::MatchData,
    group: usize,
) -> Option<MatchGroup> {
    let range = md.groups.get(group).and_then(|range| *range)?;
    if md.is_string_match() {
        Some(MatchGroup::new(
            super::regex::char_pos_to_byte_lisp_string(source, range.start()),
            super::regex::char_pos_to_byte_lisp_string(source, range.end()),
        ))
    } else {
        Some(MatchGroup::new(
            range.start().min(source.byte_len()),
            range.end().min(source.byte_len()),
        ))
    }
}

fn build_replacement_lisp_string(
    source: &crate::heap_types::LispString,
    newtext: &crate::heap_types::LispString,
    literal: bool,
    md: &super::regex::MatchData,
    preserve_substitution_properties: bool,
) -> Result<crate::heap_types::LispString, String> {
    const INVALID_BACKSLASH_MSG: &str = "Invalid use of `\\' in replacement text";

    if literal || !newtext.as_bytes().contains(&b'\\') {
        return Ok(newtext.clone());
    }

    let mut pieces = Vec::new();
    let mut pos = 0usize;
    let mut last = 0usize;
    let len = newtext.byte_len();

    while pos < len {
        let Some((ch, ch_len)) = lisp_char_at_byte(newtext, pos) else {
            break;
        };
        let ch_start = pos;
        pos += ch_len;

        if ch != b'\\' as u32 {
            continue;
        }

        let Some((next, next_len)) = lisp_char_at_byte(newtext, pos) else {
            continue;
        };
        let next_start = pos;
        pos += next_len;

        match next {
            c if c == b'&' as u32 => {
                if ch_start != last {
                    pieces.push(
                        lisp_string_slice_for_replace_match(
                            newtext,
                            last,
                            ch_start,
                            preserve_substitution_properties,
                        )
                        .expect("validated replacement literal slice"),
                    );
                }
                if let Some(range) = match_group_to_byte_range(source, md, 0) {
                    pieces.push(
                        lisp_string_slice_for_replace_match(
                            source,
                            range.start(),
                            range.end(),
                            preserve_substitution_properties,
                        )
                        .expect("validated whole-match replacement slice"),
                    );
                }
                last = pos;
            }
            c if (b'1' as u32..=b'9' as u32).contains(&c) => {
                if ch_start != last {
                    pieces.push(
                        lisp_string_slice_for_replace_match(
                            newtext,
                            last,
                            ch_start,
                            preserve_substitution_properties,
                        )
                        .expect("validated replacement literal slice"),
                    );
                }
                let group = (c as u8 - b'0') as usize;
                if let Some(range) = match_group_to_byte_range(source, md, group) {
                    pieces.push(
                        lisp_string_slice_for_replace_match(
                            source,
                            range.start(),
                            range.end(),
                            preserve_substitution_properties,
                        )
                        .expect("validated submatch replacement slice"),
                    );
                }
                last = pos;
            }
            c if c == b'\\' as u32 => {
                pieces.push(
                    lisp_string_slice_for_replace_match(
                        newtext,
                        last,
                        next_start,
                        preserve_substitution_properties,
                    )
                    .expect("validated escaped-backslash replacement slice"),
                );
                last = pos;
            }
            c if c == b'?' as u32 => {
                // GNU leaves `\?' in the literal run for query-replace-regexp
                // compatibility.
            }
            _ => return Err(INVALID_BACKSLASH_MSG.to_string()),
        }
    }

    if last < len {
        pieces.push(
            lisp_string_slice_for_replace_match(
                newtext,
                last,
                len,
                preserve_substitution_properties,
            )
            .expect("validated trailing replacement slice"),
        );
    }

    if pieces.is_empty() {
        Ok(empty_lisp_string(
            source.is_multibyte() || newtext.is_multibyte(),
        ))
    } else {
        Ok(concat_lisp_string_pieces(pieces))
    }
}

fn lisp_string_slice_for_replace_match(
    string: &crate::heap_types::LispString,
    start: usize,
    end: usize,
    preserve_properties: bool,
) -> Option<crate::heap_types::LispString> {
    if preserve_properties {
        string.slice(start, end)
    } else {
        string.slice_no_properties(start, end)
    }
}

pub(crate) fn replace_match_lisp_string_with_syntax(
    source: &crate::heap_types::LispString,
    newtext: &crate::heap_types::LispString,
    fixedcase: bool,
    literal: bool,
    subexp: usize,
    match_data: &Option<super::regex::MatchData>,
) -> Result<crate::heap_types::LispString, String> {
    replace_match_lisp_string_with_syntax_and_properties(
        source, newtext, fixedcase, literal, subexp, match_data, true,
    )
}

fn replace_match_lisp_string_with_syntax_and_properties(
    source: &crate::heap_types::LispString,
    newtext: &crate::heap_types::LispString,
    fixedcase: bool,
    literal: bool,
    subexp: usize,
    match_data: &Option<super::regex::MatchData>,
    preserve_substitution_properties: bool,
) -> Result<crate::heap_types::LispString, String> {
    let md = match match_data {
        Some(md) => md,
        None => return Err(super::regex::REPLACE_MATCH_SUBEXP_MISSING.to_string()),
    };
    let Some(byte_range) = match_group_to_byte_range(source, md, subexp) else {
        return Err(super::regex::REPLACE_MATCH_SUBEXP_MISSING.to_string());
    };
    let byte_start = byte_range.start();
    let byte_end = byte_range.end();
    if byte_end > source.byte_len() || byte_start > byte_end {
        return Err(super::regex::REPLACE_MATCH_SUBEXP_MISSING.to_string());
    }

    let before = source
        .slice(0, byte_start)
        .expect("validated replace-match prefix slice");
    let after = source
        .slice(byte_end, source.byte_len())
        .expect("validated replace-match suffix slice");
    let mut replacement = build_replacement_lisp_string(
        source,
        newtext,
        literal,
        md,
        preserve_substitution_properties,
    )?;

    if !fixedcase {
        let matched = source
            .slice(byte_start, byte_end)
            .expect("validated replace-match matched slice");
        let cased = crate::emacs_core::casefiddle::apply_replace_match_case(
            &storage_string_from_lisp_string(&replacement),
            &storage_string_from_lisp_string(&matched),
        );
        let mut cased = storage_string_to_lisp_string(
            &cased,
            source.is_multibyte() || replacement.is_multibyte(),
        );
        if cased.schars() == replacement.schars() {
            *cased.intervals_mut() = replacement.intervals().clone();
        }
        replacement = cased;
    }

    Ok(concat_lisp_string_pieces(vec![before, replacement, after]))
}

pub(crate) fn compute_buffer_replacement_lisp_string(
    buf: &crate::buffer::Buffer,
    newtext: &crate::heap_types::LispString,
    fixedcase: bool,
    literal: bool,
    subexp: usize,
    match_data: &Option<super::regex::MatchData>,
) -> Result<(usize, usize, crate::heap_types::LispString), String> {
    let md = match match_data {
        Some(md) => md,
        None => return Err(super::regex::REPLACE_MATCH_SUBEXP_MISSING.to_string()),
    };
    let Some(match_group) = md.groups.get(subexp).and_then(|range| *range) else {
        return Err(super::regex::REPLACE_MATCH_SUBEXP_MISSING.to_string());
    };
    let match_start = match_group.start();
    let match_end = match_group.end();

    let (buffer_start, buffer_end) = if md.is_string_match() {
        (
            buf.char_pos_to_emacs_byte_pos_clamped(CharPos0::new(match_start))
                .get(),
            buf.char_pos_to_emacs_byte_pos_clamped(CharPos0::new(match_end))
                .get(),
        )
    } else if md.uses_buffer_lisp_char_positions() {
        (
            buffer_match_char_pos_to_byte_pos(buf, match_start).get(),
            buffer_match_char_pos_to_byte_pos(buf, match_end).get(),
        )
    } else {
        (match_start, match_end)
    };

    // GNU `replace_match` reads only the matched text from the buffer; it never
    // copies the whole buffer.  Materialize just group 0 -- which spans the
    // whole match and therefore every subgroup -- and re-base the group indices
    // onto it, so this is O(match) rather than O(buffer).  The whole-buffer copy
    // made a replace loop (query-replace / replace-string) O(n^2).
    let mut replacement_match_data = md.clone();
    let source = if replacement_match_data.uses_buffer_lisp_char_positions() {
        let group0 = md
            .groups
            .first()
            .and_then(|range| *range)
            .ok_or_else(|| super::regex::REPLACE_MATCH_SUBEXP_MISSING.to_string())?;
        let group0_start = group0.start();
        let group0_byte_start = buffer_match_char_pos_to_byte_pos(buf, group0_start);
        let group0_byte_end = buffer_match_char_pos_to_byte_pos(buf, group0.end());
        let source = buf.buffer_substring_lisp_string_range(crate::buffer::EmacsByteRange::new(
            group0_byte_start,
            group0_byte_end,
        ));
        // Group positions are 1-based buffer chars; re-base them to 0-based
        // offsets within `source` (whose char 0 is the first char of group 0).
        for group in &mut replacement_match_data.groups {
            if let Some(group) = group {
                *group = group.saturating_sub(group0_start);
            }
        }
        replacement_match_data
            .set_string_source(Some(super::regex::SearchedString::Owned(source.clone())));
        source
    } else if replacement_match_data.searched_buffer_id().is_some() {
        // Same bounding, but the match was reported in BYTE positions
        // (`buffer_positions_are_bytes`).  Group 0's byte range bounds `source`;
        // convert each group's buffer-byte offset to a char position within the
        // small group-0 `source` so the shared string path below applies.
        let group0 = md
            .groups
            .first()
            .and_then(|range| *range)
            .ok_or_else(|| super::regex::REPLACE_MATCH_SUBEXP_MISSING.to_string())?;
        let group0_byte_start = group0.start();
        let source = buf.buffer_substring_lisp_string_range(crate::buffer::EmacsByteRange::new(
            crate::buffer::EmacsBytePos::new(group0_byte_start),
            crate::buffer::EmacsBytePos::new(group0.end()),
        ));
        let byte_to_char = |byte: usize| -> usize {
            let rel = byte
                .saturating_sub(group0_byte_start)
                .min(source.byte_len());
            source.slice(0, rel).map(|s| s.schars()).unwrap_or(0)
        };
        for group in &mut replacement_match_data.groups {
            if let Some(group) = group {
                *group = super::regex::MatchGroup::new(
                    byte_to_char(group.start()),
                    byte_to_char(group.end()),
                );
            }
        }
        replacement_match_data
            .set_string_source(Some(super::regex::SearchedString::Owned(source.clone())));
        source
    } else {
        buf.buffer_substring_lisp_string_range(buf.full_emacs_byte_range())
    };

    let replacement_match_option = Some(replacement_match_data.clone());
    let replacement = replace_match_lisp_string_with_syntax_and_properties(
        &source,
        newtext,
        fixedcase,
        literal,
        subexp,
        &replacement_match_option,
        false,
    )?;
    let replace_start = if replacement_match_data.is_string_match() {
        super::regex::char_pos_to_byte_lisp_string(
            &source,
            replacement_match_data
                .groups
                .get(subexp)
                .and_then(|range| range.map(|group| group.start()))
                .unwrap_or(0),
        )
    } else {
        buffer_start
    };
    let replace_end = if replacement_match_data.is_string_match() {
        super::regex::char_pos_to_byte_lisp_string(
            &source,
            replacement_match_data
                .groups
                .get(subexp)
                .and_then(|range| range.map(|group| group.end()))
                .unwrap_or(0),
        )
    } else {
        buffer_end
    };
    let replacement_only = replacement
        .slice(
            replace_start,
            replacement
                .byte_len()
                .saturating_sub(source.byte_len().saturating_sub(replace_end)),
        )
        .expect("computed replacement slice is within replacement string");

    Ok((buffer_start, buffer_end, replacement_only))
}

fn replace_regexp_in_string_lisp<F>(
    args: &[Value],
    case_fold: bool,
    mut replacement_for_match: F,
) -> EvalResult
where
    F: FnMut(
        &crate::heap_types::LispString,
        &Option<super::regex::MatchData>,
    ) -> Result<crate::heap_types::LispString, Flow>,
{
    let pattern = expect_lisp_string(&args[0])?;
    let source = expect_lisp_string(&args[2])?;
    let parsed = parse_replace_regexp_subexp_start_lisp(args, source)?;
    let start = parsed.start;
    let mut cursor = start;
    let mut search_at = start;
    let mut pieces = Vec::new();
    let mut match_data = None;
    let total_chars = source.schars();

    // GNU `replace-regexp-in-string` searches the original Lisp string,
    // translates match data onto the matched substring, then runs
    // `replace-match` semantics on that substring.
    while search_at < source.byte_len() {
        let found = super::regex::string_match_full_with_case_fold_source_lisp_pattern_posix(
            pattern,
            source,
            super::regex::SearchedString::Heap(args[2]),
            search_at,
            case_fold,
            false,
            &mut match_data,
        )
        .map_err(|msg| signal("invalid-regexp", vec![Value::string(msg)]))?;
        if found.is_none() {
            break;
        }

        let Some(current_md) = match_data.clone() else {
            break;
        };
        let Some(full_group) = current_md.groups.first().and_then(|g| *g) else {
            break;
        };
        let full_start_char = full_group.start();
        let full_end_char = full_group.end();

        let match_span_end_char = if full_start_char == full_end_char {
            (full_start_char + 1).min(total_chars)
        } else {
            full_end_char
        };
        let full_start_byte = super::regex::char_pos_to_byte_lisp_string(source, full_start_char);
        let match_span_end_byte =
            super::regex::char_pos_to_byte_lisp_string(source, match_span_end_char);

        pieces.push(
            source
                .slice(cursor, full_start_byte)
                .expect("validated match prefix must slice"),
        );

        let match_span = source
            .slice(full_start_byte, match_span_end_byte)
            .expect("validated match span must slice");
        let translated_md = Some(translate_match_data_to_substring(
            &current_md,
            -(full_start_char as i64),
            super::regex::SearchedString::Owned(match_span.clone()),
        ));
        pieces.push(replacement_for_match(&match_span, &translated_md)?);
        cursor = match_span_end_byte;
        search_at = match_span_end_byte;
    }

    pieces.push(
        source
            .slice(cursor, source.byte_len())
            .expect("validated match tail must slice"),
    );
    Ok(Value::heap_string(concat_lisp_string_pieces(pieces)))
}

/// Route symbol-value reads through the full GNU lookup path so
/// LOCALIZED BLV / FORWARDED slot / specpdl let-binding state is
/// observed. Mirrors `find_symbol_value` at GNU `src/data.c:1584-1609`.
/// See the extended comment on the identical helper in
/// `builtins/misc_eval.rs` (audit finding #3 in
/// `drafts/regex-search-audit.md`).
fn dynamic_or_global_symbol_value(eval: &super::eval::Context, name: &str) -> Option<Value> {
    let id = crate::emacs_core::intern::intern(name);
    eval.eval_symbol_by_id(id).ok()
}

pub(crate) fn builtin_replace_regexp_in_string(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_range_args("replace-regexp-in-string", &args, 3, 7)?;
    let case_fold = dynamic_or_global_symbol_value(eval, "case-fold-search")
        .map(|value| !value.is_nil())
        .unwrap_or(false);

    let fixedcase = args.get(3).is_some_and(|v| v.is_truthy());
    let literal = args.get(4).is_some_and(|v| v.is_truthy());
    let parsed = parse_replace_regexp_subexp_start_lisp(&args, expect_lisp_string(&args[2])?)?;
    let subexp = parsed.subexp;

    if args[1].is_string() {
        let replacement = expect_lisp_string(&args[1])?.clone();
        return replace_regexp_in_string_lisp(&args, case_fold, |match_span, translated_md| {
            replace_match_on_substring(
                match_span,
                &replacement,
                fixedcase,
                literal,
                subexp,
                translated_md,
            )
        });
    }

    let func = args[1];
    let gc_roots = eval.save_specpdl_roots();
    eval.push_specpdl_root(func);
    let saved_match_data = eval.match_data.clone();

    let result = (|| -> EvalResult {
        replace_regexp_in_string_lisp(&args, case_fold, |match_span, translated_md| {
            // GNU wraps the whole function in `save-match-data`, but each REP
            // callback observes the translated substring-local match data.
            eval.match_data = translated_md.clone();
            let Some(match_group) = translated_md
                .as_ref()
                .and_then(|md| md.groups.first().and_then(|group| *group))
            else {
                return Err(signal(
                    "error",
                    vec![
                        Value::string("replace-match subexpression does not exist"),
                        Value::fixnum(subexp as i64),
                    ],
                ));
            };
            let match_start = match_group.start();
            let match_end = match_group.end();
            let match_start_byte =
                super::regex::char_pos_to_byte_lisp_string(match_span, match_start);
            let match_end_byte = super::regex::char_pos_to_byte_lisp_string(match_span, match_end);
            let matched = match_span
                .slice(match_start_byte, match_end_byte)
                .expect("translated match bounds must slice");
            let func_result = eval.apply(func, vec![Value::heap_string(matched)])?;
            let replacement = func_result.as_lisp_string().ok_or_else(|| {
                signal(
                    "wrong-type-argument",
                    vec![Value::symbol("stringp"), func_result],
                )
            })?;
            replace_match_on_substring(
                match_span,
                replacement,
                fixedcase,
                literal,
                subexp,
                translated_md,
            )
        })
    })();

    eval.match_data = saved_match_data;
    eval.restore_specpdl_roots(gc_roots);
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
#[path = "search_test.rs"]
mod tests;

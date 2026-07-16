//! Built-in primitive functions.
//!
//! All functions here take pre-evaluated `Vec<Value>` arguments and return `EvalResult`.
//! The evaluator dispatches here after evaluating the argument expressions.

use malachite::base::num::conversion::traits::RoundingFrom;
use malachite::base::rounding_modes::RoundingMode;
use std::sync::{
    Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};

/// Debug flag: when true, log every dispatch_builtin call name.
/// Activated after window-setup-hook completes during startup.
static TRACE_ALL_BUILTINS: AtomicBool = AtomicBool::new(false);

pub(super) use super::error::{EvalResult, Flow, LispCondition, signal};
pub(super) use super::intern::{SymId, intern, resolve_sym};
pub(super) use super::keyboard::pure::{
    KEY_CHAR_CODE_MASK, KEY_CHAR_META, convert_lucid_event_list, describe_single_key_value,
    key_sequence_values,
};
pub(super) use super::value::*;
pub(super) use ::regex::Regex;
pub(crate) use buffers::lisp_string_from_buffer_bytes;
pub(super) use std::cell::RefCell;
pub(super) use std::collections::{HashMap, HashSet};
pub(crate) use strings::downcase_char_code_emacs_compat;
pub(crate) use strings::upcase_char_code_emacs_compat;

// ---------------------------------------------------------------------------
// Transitional string character iteration
// ---------------------------------------------------------------------------

/// Iterate Emacs character codes from a `LispString`.
///
/// For **multibyte** strings each character is decoded straight from the
/// Emacs-internal bytes via `string_char_unchecked`: standard UTF-8 code points
/// (including real Private-Use-Area glyphs such as nerd-font icons) and the
/// extended `0x3FFF00+byte` sequences for eight-bit raw bytes. There is no
/// in-Unicode "sentinel" remapping — that conflated real PUA characters with
/// raw bytes and corrupted them (issue #131). For **unibyte** strings each byte
/// maps to its value directly (0..255).
pub(crate) fn lisp_string_char_codes(string: &crate::heap_types::LispString) -> Vec<u32> {
    let bytes = string.as_bytes();
    if !string.is_multibyte() {
        return bytes.iter().map(|&b| b as u32).collect();
    }
    let mut out = Vec::with_capacity(string.schars());
    let mut pos = 0;
    while pos < bytes.len() {
        let byte = bytes[pos];
        if byte < 0x80 {
            out.push(byte as u32);
            pos += 1;
            continue;
        }
        let (cp, len) = crate::emacs_core::emacs_char::string_char_unchecked(&bytes[pos..]);
        out.push(cp);
        pos += len;
    }
    out
}

/// Return the character code at character index `idx` in `string`, or
/// `None` if `idx` is out of range. Unlike `lisp_string_char_codes`, this
/// does not allocate a `Vec<u32>` — it walks bytes only as far as needed.
/// Mirrors the byte-level access pattern used by GNU's `Faref` on strings
/// (fns.c:3108-3123).
pub(crate) fn lisp_string_char_at(
    string: &crate::heap_types::LispString,
    idx: usize,
) -> Option<u32> {
    let bytes = string.as_bytes();
    if !string.is_multibyte() {
        return bytes.get(idx).map(|&b| b as u32);
    }
    if idx >= string.schars() {
        return None;
    }
    let byte_pos = crate::emacs_core::emacs_char::char_to_byte_pos(bytes, idx);
    let (cp, _) = crate::emacs_core::emacs_char::string_char_unchecked(&bytes[byte_pos..]);
    Some(cp)
}

/// Iterate character codes via a closure (avoids allocation when possible).
pub(crate) fn for_each_lisp_string_char(
    string: &crate::heap_types::LispString,
    mut f: impl FnMut(u32),
) {
    let bytes = string.as_bytes();
    if !string.is_multibyte() {
        for &b in bytes {
            f(b as u32);
        }
        return;
    }
    let mut pos = 0;
    while pos < bytes.len() {
        let (cp, len) = crate::emacs_core::emacs_char::string_char_unchecked(&bytes[pos..]);
        f(cp);
        pos += len;
    }
}

/// Reset all thread-local state in builtins (called from Context::new).
pub(crate) fn reset_builtins_thread_locals() {
    collections::reset_collections_thread_locals();
    stubs::reset_stubs_thread_locals();
    hooks::reset_hooks_thread_locals();
    symbols::reset_symbols_thread_locals();
}

pub use stubs::{NeomacsMonitorInfo, neomacs_monitor_info_snapshot, set_neomacs_monitor_info};

/// Expect exactly N arguments.
pub(super) fn expect_args(name: &str, args: &[Value], n: usize) -> Result<(), Flow> {
    if args.len() != n {
        Err(signal(
            LispCondition::WrongNumberOfArguments,
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

/// Expect at least N arguments.
pub(super) fn expect_min_args(name: &str, args: &[Value], min: usize) -> Result<(), Flow> {
    if args.len() < min {
        Err(signal(
            LispCondition::WrongNumberOfArguments,
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

/// Expect at most N arguments.
pub(super) fn expect_max_args(name: &str, args: &[Value], max: usize) -> Result<(), Flow> {
    if args.len() > max {
        Err(signal(
            LispCondition::WrongNumberOfArguments,
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

pub(super) fn expect_range_args(
    name: &str,
    args: &[Value],
    min: usize,
    max: usize,
) -> Result<(), Flow> {
    if args.len() < min || args.len() > max {
        Err(signal(
            LispCondition::WrongNumberOfArguments,
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

/// Extract an integer, signaling wrong-type-argument if not.
pub(super) fn expect_int(value: &Value) -> Result<i64, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) => Ok(n),
        _other => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("integerp"), *value],
        )),
    }
}

pub(super) fn expect_fixnum(value: &Value) -> Result<i64, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) => Ok(n),
        _other => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("fixnump"), *value],
        )),
    }
}

pub(super) fn expect_char_table_index(value: &Value) -> Result<i64, Flow> {
    let idx = expect_fixnum(value)?;
    if !(0..=0x3F_FFFF).contains(&idx) {
        maybe_trace_characterp_nil(value, "expect_char_table_index");
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("characterp"), *value],
        ));
    }
    Ok(idx)
}

pub(super) fn expect_char_equal_code(value: &Value) -> Result<i64, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) if (0..=KEY_CHAR_CODE_MASK).contains(&n) => Ok(n),
        _other => {
            maybe_trace_characterp_nil(value, "expect_char_equal_code");
            Err(signal(
                LispCondition::WrongTypeArgument,
                vec![Value::symbol("characterp"), *value],
            ))
        }
    }
}

pub(super) fn expect_character_code(value: &Value) -> Result<i64, Flow> {
    match value.kind() {
        ValueKind::Fixnum(c) if (0..=0x3F_FFFF).contains(&c) => Ok(c as i64),
        _other => {
            maybe_trace_characterp_nil(value, "expect_character_code");
            Err(signal(
                LispCondition::WrongTypeArgument,
                vec![Value::symbol("characterp"), *value],
            ))
        }
    }
}

pub(crate) fn character_code_to_rust_char(code: i64) -> Option<char> {
    let code = code as u32;
    char::from_u32(code).or_else(|| {
        crate::emacs_core::emacs_char::char_byte8_p(code).then(|| {
            char::from_u32(crate::emacs_core::emacs_char::char_to_byte8(code) as u32)
                .expect("raw byte values must be valid Unicode scalars")
        })
    })
}

fn maybe_trace_characterp_nil(value: &Value, source: &str) {
    if !value.is_nil() {
        return;
    }
    if std::env::var("NEOVM_TRACE_CHARACTERP_NIL").unwrap_or_default() != "1" {
        return;
    }
    eprintln!(
        "NEOVM_TRACE_CHARACTERP_NIL source={source}\n{}",
        std::backtrace::Backtrace::force_capture()
    );
}

pub(super) fn char_equal_folded(code: i64) -> Option<String> {
    char::from_u32(code as u32).map(|ch| ch.to_lowercase().collect())
}

/// Extract an integer/marker-ish position value.
///
/// GNU Emacs accepts marker designators anywhere `integer-or-marker-p`
/// is allowed, using the marker's current position.
pub(super) fn expect_integer_or_marker(value: &Value) -> Result<i64, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) => Ok(n),
        _ if super::marker::is_marker(value) => super::marker::marker_position_as_int(value),
        _ => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("integer-or-marker-p"), *value],
        )),
    }
}

pub(super) fn expect_integer_or_marker_eval(
    eval: &super::eval::Context,
    value: &Value,
) -> Result<i64, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) => Ok(n),
        _ if super::marker::is_marker(value) => {
            super::marker::marker_position_as_int_eval(eval, value)
        }
        _ => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("integer-or-marker-p"), *value],
        )),
    }
}

/// Extract a non-negative integer, signaling `wholenump` on failure.
pub(super) fn expect_wholenump(value: &Value) -> Result<i64, Flow> {
    let n = match value.kind() {
        ValueKind::Fixnum(n) => n,
        _ => {
            return Err(signal(
                LispCondition::WrongTypeArgument,
                vec![Value::symbol("wholenump"), *value],
            ));
        }
    };
    if n < 0 {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("wholenump"), *value],
        ));
    }
    Ok(n)
}

#[derive(Clone, Copy, Debug)]
pub(super) enum NumberOrMarker {
    Int(i64),
    Float(f64),
}

pub(super) fn expect_number_or_marker(value: &Value) -> Result<NumberOrMarker, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) => Ok(NumberOrMarker::Int(n)),
        ValueKind::Float => Ok(NumberOrMarker::Float(value.xfloat())),
        // Bignums lower into f64 for the comparison/numeric path,
        // matching GNU's XFLOATINT behaviour. Callers that need
        // exact arithmetic dispatch on the Value::kind() directly.
        ValueKind::Veclike(VecLikeType::Bignum) => Ok(NumberOrMarker::Float(
            f64::rounding_from(value.as_bignum().unwrap(), RoundingMode::Nearest).0,
        )),
        _ if super::marker::is_marker(value) => Ok(NumberOrMarker::Int(
            super::marker::marker_position_as_int(value)?,
        )),
        _ => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("number-or-marker-p"), *value],
        )),
    }
}

pub(super) fn expect_number_or_marker_eval(
    eval: &super::eval::Context,
    value: &Value,
) -> Result<NumberOrMarker, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) => Ok(NumberOrMarker::Int(n)),
        ValueKind::Float => Ok(NumberOrMarker::Float(value.xfloat())),
        ValueKind::Veclike(VecLikeType::Bignum) => Ok(NumberOrMarker::Float(
            f64::rounding_from(value.as_bignum().unwrap(), RoundingMode::Nearest).0,
        )),
        _ if super::marker::is_marker(value) => Ok(NumberOrMarker::Int(
            super::marker::marker_position_as_int_eval(eval, value)?,
        )),
        _ => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("number-or-marker-p"), *value],
        )),
    }
}

/// Extract a number as f64.
pub(super) fn expect_number(value: &Value) -> Result<f64, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) => Ok(n as f64),
        ValueKind::Float => Ok(value.xfloat()),
        ValueKind::Veclike(VecLikeType::Bignum) => {
            Ok(f64::rounding_from(value.as_bignum().unwrap(), RoundingMode::Nearest).0)
        }
        _ => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("numberp"), *value],
        )),
    }
}

pub(super) fn expect_number_or_marker_f64(value: &Value) -> Result<f64, Flow> {
    match expect_number_or_marker(value)? {
        NumberOrMarker::Int(n) => Ok(n as f64),
        NumberOrMarker::Float(f) => Ok(f),
    }
}

pub(super) fn expect_number_or_marker_f64_eval(
    eval: &super::eval::Context,
    value: &Value,
) -> Result<f64, Flow> {
    match expect_number_or_marker_eval(eval, value)? {
        NumberOrMarker::Int(n) => Ok(n as f64),
        NumberOrMarker::Float(f) => Ok(f),
    }
}

pub(super) fn expect_integer_or_marker_after_number_check(value: &Value) -> Result<i64, Flow> {
    match expect_number_or_marker(value)? {
        NumberOrMarker::Int(n) => Ok(n),
        NumberOrMarker::Float(_) => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("integer-or-marker-p"), *value],
        )),
    }
}

pub(super) fn expect_integer_or_marker_after_number_check_eval(
    eval: &super::eval::Context,
    value: &Value,
) -> Result<i64, Flow> {
    match expect_number_or_marker_eval(eval, value)? {
        NumberOrMarker::Int(n) => Ok(n),
        NumberOrMarker::Float(_) => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("integer-or-marker-p"), *value],
        )),
    }
}

/// True if any arg is a float (triggers float arithmetic).
pub(super) fn has_float(args: &[Value]) -> bool {
    args.iter().any(|v| v.is_float())
}

pub(super) fn normalize_string_start_arg(
    string: &str,
    start: Option<&Value>,
) -> Result<usize, Flow> {
    let Some(start_val) = start else {
        return Ok(0);
    };
    if start_val.is_nil() {
        return Ok(0);
    }

    let raw_start = expect_fixnum(start_val)?;
    let len = string.chars().count() as i64;
    let normalized = if raw_start < 0 {
        len.checked_add(raw_start)
    } else {
        Some(raw_start)
    };

    let Some(start_idx) = normalized else {
        return Err(signal(
            LispCondition::ArgsOutOfRange,
            vec![Value::string(string), Value::fixnum(raw_start)],
        ));
    };

    if !(0..=len).contains(&start_idx) {
        return Err(signal(
            LispCondition::ArgsOutOfRange,
            vec![Value::string(string), Value::fixnum(raw_start)],
        ));
    }

    let start_char_idx = start_idx as usize;
    if start_char_idx == len as usize {
        return Ok(string.len());
    }

    Ok(string
        .char_indices()
        .nth(start_char_idx)
        .map(|(byte_idx, _)| byte_idx)
        .unwrap_or(string.len()))
}

// Re-export sibling modules so submodules can use `super::eval`, `super::marker`, etc.
pub(super) use super::autoload;
pub(super) use super::builtins_extra;
pub(super) use super::ccl;
pub(super) use super::charset;
pub(super) use super::chartable;
pub(super) use super::editfns;
pub(super) use super::error;
pub(super) use super::eval;
pub(super) use super::fileio;
pub(super) use super::kbd;
pub(super) use super::keymap;
pub(super) use super::load;
pub(super) use super::marker;
pub(super) use super::navigation;
pub(super) use super::print;
pub(super) use super::regex;
pub(super) use super::subr_info;
pub(super) use super::terminal;
pub(super) use super::textprop;
pub(super) use super::value;
pub(super) use super::window_cmds;

// --- Submodules ---
pub(crate) mod arithmetic;
pub(crate) mod buffer_text_backend;
pub(crate) mod collections;
pub(crate) mod cons_list;
pub(crate) mod from_value;
pub(crate) mod misc_pure;
pub(crate) mod strings;
pub(crate) mod types;

pub(crate) use arithmetic::*;
pub(crate) use buffer_text_backend::*;
pub(crate) use collections::*;
pub use cons_list::lambda_params_to_value;
pub use cons_list::lambda_to_closure_vector;
pub use cons_list::parse_lambda_params_from_value;
pub(crate) use cons_list::*;
pub(crate) use from_value::*;
pub(crate) use misc_pure::*;
pub(crate) use strings::*;
pub(crate) use types::*;

// `pub(crate)` so the R2 JIT Tier-A CallBuiltinSym read shim
// (`jit::compile::neovm_jit_cbsym_read`) can DELEGATE to the GC-free buffer
// primitive bodies (`builtin_point_0`, `builtin_char_after`, ...) by name
// instead of reimplementing them (matches the sibling `navigation`/`editfns`/
// `search` modules, already crate-visible).
pub(crate) mod buffers;
pub(crate) mod file_notify;
pub(crate) mod fringe_bitmap;
pub(crate) mod fringe_standard_bitmaps;
pub(crate) mod gnutls;
pub(crate) mod higher_order;
pub(crate) mod hooks;
pub(crate) mod keymaps;
pub(crate) mod lcms;
pub(crate) mod misc_eval;
pub(crate) mod search;
pub(crate) mod stubs;
pub(crate) mod symbols;
pub(crate) mod treesit;

pub(crate) use buffers::*;
pub(crate) use file_notify::*;
pub(crate) use higher_order::*;
pub(crate) use hooks::*;
pub(crate) use keymaps::*;
pub(crate) use misc_eval::*;
pub(crate) use search::*;
pub(crate) use stubs::*;
pub(crate) use symbols::*;
pub(crate) use treesit::*;

// ===========================================================================
// Helpers
// ===========================================================================

pub(super) fn expect_lisp_string(
    value: &Value,
) -> Result<&'static crate::heap_types::LispString, Flow> {
    value.as_lisp_string().ok_or_else(|| {
        signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("stringp"), *value],
        )
    })
}

/// Validate a string argument and decode it to a Rust `String` for text-only
/// processing (display strings, names, identifiers). Valid Unicode (including
/// real Private-Use glyphs) is preserved exactly; raw eight-bit bytes become
/// U+FFFD. Callers that must preserve raw bytes use `expect_lisp_string`.
pub(super) fn expect_string_lossy(value: &Value) -> Result<String, Flow> {
    expect_lisp_string(value).map(|ls| crate::emacs_core::emacs_char::to_utf8_lossy(ls.as_bytes()))
}

pub(super) fn expect_string_comparison_operand(
    value: &Value,
) -> Result<crate::heap_types::LispString, Flow> {
    match value.kind() {
        ValueKind::String => Ok(value
            .as_lisp_string()
            .expect("ValueKind::String must carry LispString payload")
            .clone()),
        _ => value
            .as_symbol_name()
            .map(crate::heap_types::LispString::from_utf8)
            .ok_or_else(|| {
                signal(
                    LispCondition::WrongTypeArgument,
                    vec![Value::symbol("stringp"), *value],
                )
            }),
    }
}

/// Build a `LispString` from a plain (sentinel-free) Rust `&str`, preserving the
/// caller's multibyteness choice.
///
/// Every Lisp-visible string built from a Rust `&str` (doc strings, parsed file
/// data, filenames, pdump payloads, printer output) goes through here: the str
/// carries no storage-String sentinels, so its bytes are already in Emacs
/// internal form and become the `LispString` directly. The legacy
/// storage-decode round-trip that this replaced has been retired (issue #131);
/// the storage codec now survives only inside the buffer-text/runtime-reader
/// layer (`storage_string_to_buffer_bytes`), which is unrelated to the
/// Lisp-string path.
pub(crate) fn plain_str_to_lisp_string(
    text: &str,
    multibyte: bool,
) -> crate::heap_types::LispString {
    if multibyte {
        crate::heap_types::LispString::from_emacs_bytes(text.as_bytes().to_vec())
    } else {
        crate::heap_types::LispString::from_unibyte(text.as_bytes().to_vec())
    }
}

/// Test-only convenience: decode a string Value to a lossy `String` (valid
/// Unicode preserved, raw eight-bit -> U+FFFD). No longer produces a storage
/// string; production code uses `as_lisp_string` for byte-faithful access.
/// `#[cfg(test)]`-gated so this lossy helper can never re-enter a production
/// path (issue #131).
#[cfg(test)]
pub(crate) fn lisp_string_to_runtime_string(value: Value) -> String {
    value
        .as_lisp_string()
        .map(|ls| crate::emacs_core::emacs_char::to_utf8_lossy(ls.as_bytes()))
        .expect("ValueKind::String must carry LispString payload")
}

// Search / regex builtins are defined at the end of this file.

/// Try to dispatch a builtin function by name. Returns None if not a known builtin.
pub(crate) fn dispatch_builtin(
    eval: &mut super::eval::Context,
    name: &str,
    args: Vec<Value>,
) -> Option<EvalResult> {
    dispatch_builtin_by_id(eval, intern(name), args)
}

/// Try to dispatch a builtin function by its canonical symbol id.
pub(crate) fn dispatch_builtin_by_id(
    eval: &mut super::eval::Context,
    sym_id: SymId,
    args: Vec<Value>,
) -> Option<EvalResult> {
    eval.dispatch_subr_value(Value::subr_from_sym_id(sym_id), args)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BuiltinNoEvalPlaceholder {
    Nil,
    FixnumZero,
    WindowLineHeight,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BuiltinNoEvalPolicy {
    Native,
    RequiresEvalState,
    Placeholder(BuiltinNoEvalPlaceholder),
}

static BUILTIN_NO_EVAL_POLICIES: OnceLock<Mutex<Vec<Option<BuiltinNoEvalPolicy>>>> =
    OnceLock::new();

pub(crate) fn builtin_no_eval_policies() -> &'static Mutex<Vec<Option<BuiltinNoEvalPolicy>>> {
    BUILTIN_NO_EVAL_POLICIES.get_or_init(|| Mutex::new(Vec::new()))
}

pub(crate) fn record_builtin_no_eval_policy(name: &str, policy: BuiltinNoEvalPolicy) {
    let sym_id = intern(name);
    let mut policies = builtin_no_eval_policies()
        .lock()
        .expect("builtin no-eval policy registry poisoned");
    let index = sym_id.0 as usize;
    if policies.len() <= index {
        policies.resize(index + 1, None);
    }
    policies[index] = Some(policy);
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn builtin_no_eval_policy(sym_id: SymId) -> BuiltinNoEvalPolicy {
    builtin_no_eval_policies()
        .lock()
        .expect("builtin no-eval policy registry poisoned")
        .get(sym_id.0 as usize)
        .copied()
        .flatten()
        .unwrap_or(BuiltinNoEvalPolicy::Native)
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn dispatch_builtin_stateless_placeholder(
    policy: BuiltinNoEvalPolicy,
    args: &[Value],
) -> Option<EvalResult> {
    let value = match policy {
        BuiltinNoEvalPolicy::Placeholder(BuiltinNoEvalPlaceholder::Nil) => Value::NIL,
        BuiltinNoEvalPolicy::Placeholder(BuiltinNoEvalPlaceholder::FixnumZero) => Value::fixnum(0),
        BuiltinNoEvalPolicy::Placeholder(BuiltinNoEvalPlaceholder::WindowLineHeight) => {
            if args.len() == 2 && args[1].as_symbol_name() == Some("window") {
                Value::NIL
            } else {
                return None;
            }
        }
        BuiltinNoEvalPolicy::Native | BuiltinNoEvalPolicy::RequiresEvalState => return None,
    };
    Some(Ok(value))
}

#[cfg(test)]
pub(crate) fn dispatch_builtin_without_eval_state(
    name: &str,
    args: Vec<Value>,
) -> Option<EvalResult> {
    use crate::emacs_core::eval::Context;

    thread_local! {
        static CTX: std::cell::RefCell<Context> = std::cell::RefCell::new(Context::new());
    }

    CTX.with(|cell| {
        let ctx = &mut *cell.borrow_mut();
        let sym_id = intern(name);
        let policy = builtin_no_eval_policy(sym_id);
        if let Some(result) = dispatch_builtin_stateless_placeholder(policy, &args) {
            return Some(result);
        }
        if policy == BuiltinNoEvalPolicy::RequiresEvalState {
            return None;
        }
        dispatch_builtin_by_id(ctx, sym_id, args)
    })
}

#[cfg(test)]
pub(crate) mod tests;

#[cfg(test)]
pub(crate) mod replace_region_contents_test;

// -----------------------------------------------------------------------
// Wrapper functions for builtins that need tracing or non-standard access
// -----------------------------------------------------------------------

pub(crate) fn defsubr_run_hooks(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    let hook_names: Vec<String> = args
        .iter()
        .filter_map(|a| a.as_symbol_name().map(|s| s.to_string()))
        .collect();
    let dominated_by_noise = hook_names
        .iter()
        .all(|h| h == "custom-define-hook" || h == "change-major-mode-hook");
    tracing::debug!(hooks = ?hook_names, noisy = dominated_by_noise, "run-hooks called");
    let result = builtin_run_hooks(eval, args);
    tracing::debug!(hooks = ?hook_names, noisy = dominated_by_noise, "run-hooks returned");
    if hook_names.iter().any(|h| h == "window-setup-hook") {
        tracing::debug!("Enabling post-startup builtin tracing");
        TRACE_ALL_BUILTINS.store(true, Ordering::Relaxed);
    }
    result
}

pub(crate) fn defsubr_load(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    let file_name = args.first().map(|a| format!("{}", a)).unwrap_or_default();
    tracing::debug!(file = %file_name, "load called");
    let result = builtin_load(eval, args);
    tracing::debug!(file = %file_name, ok = result.is_ok(), "load returned");
    result
}

pub(crate) fn defsubr_message(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    let msg_preview: String = args
        .first()
        .map(|a| {
            let s = format!("{}", a);
            if s.len() > 120 {
                format!("{}...", &s[..120])
            } else {
                s
            }
        })
        .unwrap_or_default();
    tracing::debug!(msg = %msg_preview, "message");
    builtin_message(eval, args)
}

pub(crate) fn defsubr_coding_system_aliases(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::coding::builtin_coding_system_aliases(&eval.coding_systems, args)
}
pub(crate) fn defsubr_coding_system_plist(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::coding::builtin_coding_system_plist(&eval.coding_systems, args)
}
pub(crate) fn defsubr_coding_system_put(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::coding::builtin_coding_system_put(&mut eval.coding_systems, args)
}
pub(crate) fn defsubr_coding_system_base(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::coding::builtin_coding_system_base(&eval.coding_systems, args)
}
pub(crate) fn defsubr_coding_system_eol_type(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::coding::builtin_coding_system_eol_type(&eval.coding_systems, args)
}
pub(crate) fn defsubr_detect_coding_string(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::coding::builtin_detect_coding_string(&eval.coding_systems, args)
}
pub(crate) fn defsubr_detect_coding_region(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::coding::builtin_detect_coding_region(&eval.coding_systems, &eval.buffers, args)
}
pub(crate) fn defsubr_keyboard_coding_system(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::coding::builtin_keyboard_coding_system(&eval.coding_systems, args)
}
pub(crate) fn defsubr_terminal_coding_system(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::coding::builtin_terminal_coding_system(&eval.coding_systems, args)
}
pub(crate) fn defsubr_coding_system_priority_list(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::coding::builtin_coding_system_priority_list(&eval.coding_systems, args)
}

pub(crate) fn defsubr_coding_system_p(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::coding::builtin_coding_system_p(&eval.coding_systems, args)
}
pub(crate) fn defsubr_check_coding_system(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::coding::builtin_check_coding_system(&eval.coding_systems, args)
}
pub(crate) fn defsubr_check_coding_systems_region(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::coding::builtin_check_coding_systems_region(eval, args)
}
pub(crate) fn defsubr_define_coding_system_internal(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let result = super::coding::builtin_define_coding_system_internal(
        &mut eval.coding_systems,
        args.clone(),
    )?;
    super::coding::record_lisp_define_coding_system_internal(&mut eval.obarray, &args);
    Ok(result)
}
pub(crate) fn defsubr_define_coding_system_alias(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let result =
        super::coding::builtin_define_coding_system_alias(&mut eval.coding_systems, args.clone())?;
    super::coding::record_lisp_define_coding_system_alias(&mut eval.obarray, &args);
    Ok(result)
}
pub(crate) fn defsubr_set_coding_system_priority(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let result = super::coding::builtin_set_coding_system_priority(&mut eval.coding_systems, args)?;
    // GNU `Fset_coding_system_priority` also rebuilds the `coding-category-list`
    // variable (coding.c) from the reordered category priorities.
    let categories = super::coding::coding_category_priority_list(&eval.coding_systems);
    let list = Value::list(categories.into_iter().map(Value::symbol).collect());
    eval.set_variable("coding-category-list", list);
    Ok(result)
}
pub(crate) fn defsubr_set_keyboard_coding_system_internal(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::coding::builtin_set_keyboard_coding_system_internal(&mut eval.coding_systems, args)
}
pub(crate) fn defsubr_set_safe_terminal_coding_system_internal(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::coding::builtin_set_safe_terminal_coding_system_internal(&mut eval.coding_systems, args)
}
pub(crate) fn defsubr_set_terminal_coding_system_internal(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::coding::builtin_set_terminal_coding_system_internal(&mut eval.coding_systems, args)
}

type BuiltinFn = fn(&mut super::eval::Context, Vec<Value>) -> EvalResult;

#[derive(Clone, Copy)]
pub(crate) struct BuiltinRegistration {
    name: &'static str,
    func: BuiltinFn,
    min_args: u16,
    max_args: Option<u16>,
    no_eval_policy: BuiltinNoEvalPolicy,
}

impl BuiltinRegistration {
    pub(crate) const fn requires_eval_state(
        name: &'static str,
        func: BuiltinFn,
        min_args: u16,
        max_args: Option<u16>,
    ) -> Self {
        Self {
            name,
            func,
            min_args,
            max_args,
            no_eval_policy: BuiltinNoEvalPolicy::RequiresEvalState,
        }
    }

    pub(crate) const fn placeholder(
        name: &'static str,
        func: BuiltinFn,
        min_args: u16,
        max_args: Option<u16>,
        placeholder: BuiltinNoEvalPlaceholder,
    ) -> Self {
        Self {
            name,
            func,
            min_args,
            max_args,
            no_eval_policy: BuiltinNoEvalPolicy::Placeholder(placeholder),
        }
    }
}

fn register_cursor_effect_subrs(ctx: &mut super::eval::Context) {
    macro_rules! reg {
        ($name:literal, $func:ident, $max:expr) => {
            ctx.defsubr($name, $func, 1, Some($max));
        };
    }

    reg!(
        "neomacs-set-cursor-glow",
        builtin_neomacs_set_cursor_glow,
        3
    );
    reg!(
        "neomacs-set-cursor-pulse",
        builtin_neomacs_set_cursor_pulse,
        2
    );
    reg!(
        "neomacs-set-cursor-color-cycle",
        builtin_neomacs_set_cursor_color_cycle,
        4
    );
    reg!(
        "neomacs-set-cursor-shadow",
        builtin_neomacs_set_cursor_shadow,
        4
    );
    reg!(
        "neomacs-set-cursor-wake",
        builtin_neomacs_set_cursor_wake,
        3
    );
    reg!(
        "neomacs-set-cursor-error-pulse",
        builtin_neomacs_set_cursor_error_pulse,
        3
    );
    reg!(
        "neomacs-set-cursor-crosshair",
        builtin_neomacs_set_cursor_crosshair,
        3
    );
    reg!(
        "neomacs-set-cursor-magnetism",
        builtin_neomacs_set_cursor_magnetism,
        5
    );
    reg!(
        "neomacs-set-cursor-comet",
        builtin_neomacs_set_cursor_comet,
        5
    );
    reg!(
        "neomacs-set-cursor-spotlight",
        builtin_neomacs_set_cursor_spotlight,
        4
    );
    reg!(
        "neomacs-set-cursor-particles",
        builtin_neomacs_set_cursor_particles,
        5
    );
    reg!(
        "neomacs-set-cursor-trail-fade",
        builtin_neomacs_set_cursor_trail_fade,
        3
    );
    reg!(
        "neomacs-set-cursor-size-transition",
        builtin_neomacs_set_cursor_size_transition,
        2
    );
    reg!(
        "neomacs-set-cursor-elastic-snap",
        builtin_neomacs_set_cursor_elastic_snap,
        3
    );
    reg!(
        "neomacs-set-cursor-ghost",
        builtin_neomacs_set_cursor_ghost,
        4
    );
    reg!(
        "neomacs-set-cursor-ripple-wave",
        builtin_neomacs_set_cursor_ripple_wave,
        5
    );
    reg!(
        "neomacs-set-cursor-lighthouse",
        builtin_neomacs_set_cursor_lighthouse,
        5
    );
    reg!(
        "neomacs-set-cursor-sonar-ping",
        builtin_neomacs_set_cursor_sonar_ping,
        5
    );
    reg!(
        "neomacs-set-cursor-orbit-particles",
        builtin_neomacs_set_cursor_orbit_particles,
        6
    );
    reg!(
        "neomacs-set-cursor-heartbeat",
        builtin_neomacs_set_cursor_heartbeat,
        5
    );
    reg!(
        "neomacs-set-cursor-metronome",
        builtin_neomacs_set_cursor_metronome,
        5
    );
    reg!(
        "neomacs-set-cursor-radar",
        builtin_neomacs_set_cursor_radar,
        5
    );
    reg!(
        "neomacs-set-cursor-ripple-ring",
        builtin_neomacs_set_cursor_ripple_ring,
        6
    );
    reg!(
        "neomacs-set-cursor-scope",
        builtin_neomacs_set_cursor_scope,
        5
    );
    reg!(
        "neomacs-set-cursor-shockwave",
        builtin_neomacs_set_cursor_shockwave,
        5
    );
    reg!(
        "neomacs-set-cursor-gravity-well",
        builtin_neomacs_set_cursor_gravity_well,
        5
    );
    reg!(
        "neomacs-set-cursor-water-drop",
        builtin_neomacs_set_cursor_water_drop,
        5
    );
    reg!(
        "neomacs-set-cursor-pixel-dust",
        builtin_neomacs_set_cursor_pixel_dust,
        5
    );
    reg!(
        "neomacs-set-cursor-candle-flame",
        builtin_neomacs_set_cursor_candle_flame,
        5
    );
    reg!(
        "neomacs-set-cursor-moth-flame",
        builtin_neomacs_set_cursor_moth_flame,
        5
    );
    reg!(
        "neomacs-set-cursor-sparkler",
        builtin_neomacs_set_cursor_sparkler,
        5
    );
    reg!(
        "neomacs-set-cursor-plasma-ball",
        builtin_neomacs_set_cursor_plasma_ball,
        5
    );
    reg!(
        "neomacs-set-cursor-quill-pen",
        builtin_neomacs_set_cursor_quill_pen,
        5
    );
    reg!(
        "neomacs-set-cursor-aurora-borealis",
        builtin_neomacs_set_cursor_aurora_borealis,
        5
    );
    reg!(
        "neomacs-set-cursor-feather",
        builtin_neomacs_set_cursor_feather,
        5
    );
    reg!(
        "neomacs-set-cursor-stardust",
        builtin_neomacs_set_cursor_stardust,
        5
    );
    reg!(
        "neomacs-set-cursor-compass-needle",
        builtin_neomacs_set_cursor_compass_needle,
        5
    );
    reg!(
        "neomacs-set-cursor-galaxy",
        builtin_neomacs_set_cursor_galaxy,
        5
    );
    reg!(
        "neomacs-set-cursor-prism",
        builtin_neomacs_set_cursor_prism,
        5
    );
    reg!(
        "neomacs-set-cursor-moth",
        builtin_neomacs_set_cursor_moth,
        5
    );
    reg!(
        "neomacs-set-cursor-flame",
        builtin_neomacs_set_cursor_flame,
        5
    );
    reg!(
        "neomacs-set-cursor-crystal",
        builtin_neomacs_set_cursor_crystal,
        5
    );
    reg!(
        "neomacs-set-cursor-lightning",
        builtin_neomacs_set_cursor_lightning,
        5
    );
    reg!(
        "neomacs-set-cursor-snowflake",
        builtin_neomacs_set_cursor_snowflake,
        5
    );
    reg!(
        "neomacs-set-cursor-firework",
        builtin_neomacs_set_cursor_firework,
        5
    );
    reg!(
        "neomacs-set-cursor-tornado",
        builtin_neomacs_set_cursor_tornado,
        5
    );
    reg!(
        "neomacs-set-cursor-portal",
        builtin_neomacs_set_cursor_portal,
        5
    );
    reg!(
        "neomacs-set-cursor-bubble",
        builtin_neomacs_set_cursor_bubble,
        5
    );
    reg!(
        "neomacs-set-cursor-sparkle-burst",
        builtin_neomacs_set_cursor_sparkle_burst,
        5
    );
    reg!(
        "neomacs-set-cursor-compass",
        builtin_neomacs_set_cursor_compass,
        5
    );
    reg!(
        "neomacs-set-cursor-dna-helix",
        builtin_neomacs_set_cursor_dna_helix,
        6
    );
    reg!(
        "neomacs-set-cursor-pendulum",
        builtin_neomacs_set_cursor_pendulum,
        5
    );
}

/// Diagnostics-only (feature `vm-profile`): clear the VM profiler histograms
/// (OP-MIX + SUBR-MIX + the Op::Call/CallBuiltinSym entry split). Call before a
/// measured batch editing session so loadup/startup traffic is excluded.
#[cfg(feature = "vm-profile")]
pub(crate) fn defsubr_vm_profile_reset(
    _eval: &mut super::eval::Context,
    _args: Vec<Value>,
) -> EvalResult {
    crate::emacs_core::bytecode::vm::vm_profile::reset();
    Ok(Value::NIL)
}

/// Diagnostics-only (feature `vm-profile`): dump the VM profiler histograms to
/// stderr with an optional LABEL (string). Returns nil. Pairs with
/// `neovm--vm-profile-reset` for a reset → workload → dump batch session.
#[cfg(feature = "vm-profile")]
pub(crate) fn defsubr_vm_profile_dump(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let label = args
        .first()
        .map(|v| format!("{v}").trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "batch".to_string());
    crate::emacs_core::bytecode::vm::vm_profile::dump(&label);
    Ok(Value::NIL)
}

/// Internal test hook: panic with the optional MESSAGE argument. Exists so
/// panic-containment tests (the module ABI today, JIT shims next) can
/// originate a HOST-code panic from Lisp: a foreign Rust module's own panic
/// cannot cross its statically linked std into our `catch_unwind`, and no
/// legitimate Lisp input panics the evaluator on demand.
pub(crate) fn defsubr_neovm_internal_panic(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let message = args
        .first()
        .and_then(|v| v.as_lisp_string())
        .map(|ls| String::from_utf8_lossy(ls.as_bytes()).into_owned())
        .unwrap_or_else(|| "neovm--internal-panic".to_string());
    panic!("{message}");
}

pub(crate) fn register_builtin(ctx: &mut super::eval::Context, builtin: BuiltinRegistration) {
    if builtin.no_eval_policy != BuiltinNoEvalPolicy::Native {
        record_builtin_no_eval_policy(builtin.name, builtin.no_eval_policy);
    }
    ctx.defsubr(
        builtin.name,
        builtin.func,
        builtin.min_args,
        builtin.max_args,
    );
}

/// Register all builtins via defsubr — function pointer dispatch.
///
/// This replaces the giant match-by-name block in dispatch_builtin.
/// Each registered builtin is called via a direct function pointer,
/// matching GNU Emacs's defsubr/funcall_subr architecture.
pub(crate) fn init_builtins(ctx: &mut super::eval::Context) {
    // Per-module registrars (GNU syms_of_* pattern); order is not
    // observable — the subr table is SymId-keyed with independent entries.
    super::alloc::syms_of_alloc(ctx);
    super::buffer::syms_of_buffer(ctx);
    super::bytecode::syms_of_bytecode(ctx);
    super::callproc::syms_of_callproc(ctx);
    super::casefiddle::syms_of_casefiddle(ctx);
    super::casetab::syms_of_casetab(ctx);
    super::category::syms_of_category(ctx);
    super::emacs_char::syms_of_character(ctx);
    super::charset::syms_of_charset(ctx);
    super::chartable::syms_of_chartab(ctx);
    super::coding::syms_of_coding(ctx);
    super::comp::syms_of_comp(ctx);
    super::composite::syms_of_composite(ctx);
    super::data::syms_of_data(ctx);
    super::dbus::syms_of_dbusbind(ctx);
    super::zlib::syms_of_decompress(ctx);
    super::dired::syms_of_dired(ctx);
    super::dispnew::syms_of_dispnew(ctx);
    super::doc::syms_of_doc(ctx);
    super::editfns::syms_of_editfns(ctx);
    super::dynamic_module::syms_of_module(ctx);
    super::eval::syms_of_eval(ctx);
    super::fileio::syms_of_fileio(ctx);
    super::floatfns::syms_of_floatfns(ctx);
    super::fns::syms_of_fns(ctx);
    super::font::syms_of_font(ctx);
    super::fontset::syms_of_fontset(ctx);
    gnutls::syms_of_gnutls(ctx);
    super::image::syms_of_image(ctx);
    super::indent::syms_of_indent(ctx);
    file_notify::syms_of_inotify(ctx);
    super::json::syms_of_json(ctx);
    super::keyboard::syms_of_keyboard(ctx);
    super::keymap::syms_of_keymap(ctx);
    super::lread::syms_of_lread(ctx);
    super::kmacro::syms_of_macros(ctx);
    super::marker::syms_of_marker(ctx);
    super::minibuffer::syms_of_minibuf(ctx);
    super::print::syms_of_print(ctx);
    super::process::syms_of_process(ctx);
    super::profiler::syms_of_profiler(ctx);
    super::search::syms_of_search(ctx);
    super::sound::syms_of_sound(ctx);
    super::sqlite::syms_of_sqlite(ctx);
    super::syntax::syms_of_syntax(ctx);
    super::terminal::syms_of_terminal(ctx);
    super::textprop::syms_of_textprop(ctx);
    super::threads::syms_of_threads(ctx);
    super::timefns::syms_of_timefns(ctx);
    super::treesit::syms_of_treesit(ctx);
    super::undo::syms_of_undo(ctx);
    super::window_cmds::syms_of_window(ctx);
    super::xdisp::syms_of_xdisp(ctx);
    super::xfaces::syms_of_xfaces(ctx);
    super::xml::syms_of_xml(ctx);
    use super::value::*;
    #[cfg(windows)]
    super::windows::register_builtin_subrs(ctx);
    lcms::register_builtin_subrs(ctx);
    // Diagnostics-only VM-profiler control subrs (feature `vm-profile`).
    #[cfg(feature = "vm-profile")]
    {
        ctx.defsubr(
            "neovm--vm-profile-reset",
            defsubr_vm_profile_reset,
            0,
            Some(0),
        );
        ctx.defsubr(
            "neovm--vm-profile-dump",
            defsubr_vm_profile_dump,
            0,
            Some(1),
        );
    }
    ctx.defsubr(
        "neovm--internal-panic",
        defsubr_neovm_internal_panic,
        0,
        Some(1),
    );
    ctx.defsubr_slice(
        "funcall-interactively",
        builtin_funcall_interactively_slice,
        0,
        None,
    );
    ctx.defsubr(
        "make-xwidget",
        super::xwidget::builtin_make_xwidget,
        4,
        Some(7),
    );
    ctx.defsubr("xwidgetp", super::xwidget::builtin_xwidgetp, 1, Some(1));
    ctx.defsubr(
        "xwidget-view-p",
        super::xwidget::builtin_xwidget_view_p,
        1,
        Some(1),
    );
    ctx.defsubr(
        "xwidget-live-p",
        super::xwidget::builtin_xwidget_live_p,
        1,
        Some(1),
    );
    ctx.defsubr(
        "xwidget-info",
        super::xwidget::builtin_xwidget_info,
        1,
        Some(1),
    );
    ctx.defsubr(
        "xwidget-view-info",
        super::xwidget::builtin_xwidget_view_info,
        1,
        Some(1),
    );
    ctx.defsubr(
        "xwidget-view-model",
        super::xwidget::builtin_xwidget_view_model,
        1,
        Some(1),
    );
    ctx.defsubr(
        "xwidget-view-window",
        super::xwidget::builtin_xwidget_view_window,
        1,
        Some(1),
    );
    ctx.defsubr(
        "xwidget-view-lookup",
        super::xwidget::builtin_xwidget_view_lookup,
        2,
        Some(2),
    );
    ctx.defsubr(
        "delete-xwidget-view",
        super::xwidget::builtin_delete_xwidget_view,
        1,
        Some(1),
    );
    ctx.defsubr(
        "xwidget-plist",
        super::xwidget::builtin_xwidget_plist,
        1,
        Some(1),
    );
    ctx.defsubr(
        "set-xwidget-plist",
        super::xwidget::builtin_set_xwidget_plist,
        2,
        Some(2),
    );
    ctx.defsubr(
        "xwidget-buffer",
        super::xwidget::builtin_xwidget_buffer,
        1,
        Some(1),
    );
    ctx.defsubr(
        "set-xwidget-buffer",
        super::xwidget::builtin_set_xwidget_buffer,
        2,
        Some(2),
    );
    ctx.defsubr(
        "xwidget-query-on-exit-flag",
        super::xwidget::builtin_xwidget_query_on_exit_flag,
        1,
        Some(1),
    );
    ctx.defsubr(
        "set-xwidget-query-on-exit-flag",
        super::xwidget::builtin_set_xwidget_query_on_exit_flag,
        2,
        Some(2),
    );
    ctx.defsubr(
        "get-buffer-xwidgets",
        super::xwidget::builtin_get_buffer_xwidgets,
        1,
        Some(1),
    );
    ctx.defsubr(
        "kill-xwidget",
        super::xwidget::builtin_kill_xwidget,
        1,
        Some(1),
    );
    ctx.defsubr(
        "xwidget-resize",
        super::xwidget::builtin_xwidget_resize,
        3,
        Some(3),
    );
    ctx.defsubr(
        "xwidget-size-request",
        super::xwidget::builtin_xwidget_size_request,
        1,
        Some(1),
    );
    ctx.defsubr(
        "xwidget-webkit-uri",
        super::xwidget::builtin_xwidget_webkit_uri,
        1,
        Some(1),
    );
    ctx.defsubr(
        "xwidget-webkit-title",
        super::xwidget::builtin_xwidget_webkit_title,
        1,
        Some(1),
    );
    ctx.defsubr(
        "xwidget-webkit-goto-uri",
        super::xwidget::builtin_xwidget_webkit_goto_uri,
        2,
        Some(2),
    );
    ctx.defsubr("string-match-p", builtin_string_match_p, 0, None);
    ctx.defsubr("global-set-key", builtin_global_set_key, 0, None);
    ctx.defsubr("local-set-key", builtin_local_set_key, 0, None);
    ctx.defsubr(
        "neomacs-open-tls-stream",
        super::process::builtin_neomacs_open_tls_stream,
        4,
        Some(4),
    );
    ctx.defsubr(
        "open-tls-stream",
        super::process::builtin_neomacs_open_tls_stream,
        4,
        Some(4),
    );
    ctx.defsubr(
        "neomacs-tls-available-p",
        |_ctx, args| super::tls::builtin_neomacs_tls_available_p(args),
        0,
        Some(0),
    );
    ctx.defsubr(
        "start-process",
        super::process::builtin_start_process,
        3,
        None,
    );
    ctx.defsubr(
        "start-file-process",
        super::process::builtin_start_file_process,
        3,
        None,
    );
    ctx.defsubr(
        "start-process-shell-command",
        super::process::builtin_start_process_shell_command,
        3,
        Some(3),
    );
    ctx.defsubr(
        "start-file-process-shell-command",
        super::process::builtin_start_file_process_shell_command,
        3,
        Some(3),
    );
    ctx.defsubr(
        "defining-kbd-macro",
        super::kmacro::builtin_defining_kbd_macro,
        1,
        Some(2),
    );
    ctx.defsubr(
        "defining-kbd-macro-p",
        super::kmacro::builtin_defining_kbd_macro_p,
        0,
        Some(0),
    );
    ctx.defsubr(
        "executing-kbd-macro-p",
        super::kmacro::builtin_executing_kbd_macro_p,
        0,
        Some(0),
    );
    ctx.defsubr(
        "kmacro-set-counter",
        super::kmacro::builtin_kmacro_set_counter,
        1,
        Some(1),
    );
    ctx.defsubr(
        "kmacro-add-counter",
        super::kmacro::builtin_kmacro_add_counter,
        1,
        Some(1),
    );
    ctx.defsubr(
        "kmacro-set-format",
        super::kmacro::builtin_kmacro_set_format,
        1,
        Some(1),
    );
    ctx.defsubr(
        "forward-line",
        super::navigation::builtin_forward_line,
        0,
        Some(1),
    );
    ctx.defsubr(
        "beginning-of-line",
        super::navigation::builtin_beginning_of_line,
        0,
        Some(1),
    );
    ctx.defsubr(
        "end-of-line",
        super::navigation::builtin_end_of_line,
        0,
        Some(1),
    );
    ctx.defsubr(
        "forward-char",
        super::navigation::builtin_forward_char,
        0,
        Some(1),
    );
    ctx.defsubr(
        "backward-char",
        super::navigation::builtin_backward_char,
        0,
        Some(1),
    );
    ctx.defsubr(
        "transient-mark-mode",
        super::navigation::builtin_transient_mark_mode,
        0,
        None,
    );
    ctx.defsubr("symbol-file", super::autoload::builtin_symbol_file, 0, None);
    ctx.defsubr(
        "window-edges",
        super::window_cmds::builtin_window_edges,
        0,
        Some(4),
    );
    ctx.defsubr(
        "window-pixel-edges",
        super::window_cmds::builtin_window_pixel_edges,
        0,
        Some(1),
    );
    ctx.defsubr(
        "window-absolute-pixel-edges",
        super::window_cmds::builtin_window_absolute_pixel_edges,
        0,
        Some(1),
    );
    ctx.defsubr(
        "delete-window",
        super::window_cmds::builtin_delete_window,
        0,
        None,
    );
    ctx.defsubr(
        "delete-other-windows",
        super::window_cmds::builtin_delete_other_windows,
        0,
        None,
    );
    ctx.defsubr(
        "fit-window-to-buffer",
        super::window_cmds::builtin_fit_window_to_buffer,
        0,
        Some(6),
    );
    register_builtin(
        ctx,
        BuiltinRegistration::placeholder(
            "old-selected-frame",
            builtin_old_selected_frame,
            0,
            Some(0),
            BuiltinNoEvalPlaceholder::Nil,
        ),
    );
    ctx.defsubr(
        "selected-frame",
        super::window_cmds::builtin_selected_frame,
        0,
        Some(0),
    );
    ctx.defsubr(
        "mouse-pixel-position",
        builtin_mouse_pixel_position,
        0,
        Some(0),
    );
    ctx.defsubr("mouse-position", builtin_mouse_position, 0, Some(0));
    register_builtin(
        ctx,
        BuiltinRegistration::placeholder(
            "next-frame",
            builtin_next_frame,
            0,
            Some(2),
            BuiltinNoEvalPlaceholder::Nil,
        ),
    );
    register_builtin(
        ctx,
        BuiltinRegistration::placeholder(
            "previous-frame",
            builtin_previous_frame,
            0,
            Some(2),
            BuiltinNoEvalPlaceholder::Nil,
        ),
    );
    ctx.defsubr(
        "select-frame",
        super::window_cmds::builtin_select_frame,
        1,
        Some(2),
    );
    ctx.defsubr(
        "last-nonminibuffer-frame",
        super::window_cmds::builtin_selected_frame,
        0,
        None,
    );
    ctx.defsubr(
        "visible-frame-list",
        super::window_cmds::builtin_visible_frame_list,
        0,
        None,
    );
    ctx.defsubr(
        "frame-list",
        super::window_cmds::builtin_frame_list,
        0,
        None,
    );
    ctx.defsubr(
        "x-create-frame",
        super::window_cmds::builtin_x_create_frame,
        1,
        Some(1),
    );
    ctx.defsubr(
        "make-frame-visible",
        super::window_cmds::builtin_make_frame_visible,
        0,
        Some(1),
    );
    ctx.defsubr(
        "make-frame",
        super::window_cmds::builtin_make_frame,
        0,
        None,
    );
    ctx.defsubr(
        "iconify-frame",
        super::window_cmds::builtin_iconify_frame,
        0,
        Some(1),
    );
    ctx.defsubr(
        "delete-frame",
        super::window_cmds::builtin_delete_frame,
        0,
        Some(2),
    );
    ctx.defsubr(
        "frame-char-height",
        super::window_cmds::builtin_frame_char_height,
        0,
        Some(1),
    );
    ctx.defsubr(
        "frame-char-width",
        super::window_cmds::builtin_frame_char_width,
        0,
        Some(1),
    );
    ctx.defsubr(
        "frame-native-height",
        super::window_cmds::builtin_frame_native_height,
        0,
        None,
    );
    ctx.defsubr(
        "frame-native-width",
        super::window_cmds::builtin_frame_native_width,
        0,
        None,
    );
    ctx.defsubr(
        "frame-text-cols",
        super::window_cmds::builtin_frame_text_cols,
        0,
        Some(1),
    );
    ctx.defsubr(
        "frame-text-height",
        super::window_cmds::builtin_frame_text_height,
        0,
        Some(1),
    );
    ctx.defsubr(
        "frame-text-lines",
        super::window_cmds::builtin_frame_text_lines,
        0,
        Some(1),
    );
    ctx.defsubr(
        "frame-text-width",
        super::window_cmds::builtin_frame_text_width,
        0,
        Some(1),
    );
    ctx.defsubr(
        "frame-total-cols",
        super::window_cmds::builtin_frame_total_cols,
        0,
        Some(1),
    );
    ctx.defsubr(
        "frame-total-lines",
        super::window_cmds::builtin_frame_total_lines,
        0,
        Some(1),
    );
    ctx.defsubr(
        "frame-position",
        super::window_cmds::builtin_frame_position,
        0,
        None,
    );
    ctx.defsubr(
        "frame-parameters",
        super::window_cmds::builtin_frame_parameters,
        0,
        Some(1),
    );
    ctx.defsubr(
        "set-frame-height",
        super::window_cmds::builtin_set_frame_height,
        2,
        Some(4),
    );
    ctx.defsubr(
        "set-frame-width",
        super::window_cmds::builtin_set_frame_width,
        2,
        Some(4),
    );
    ctx.defsubr(
        "set-frame-size",
        super::window_cmds::builtin_set_frame_size,
        3,
        Some(4),
    );
    ctx.defsubr(
        "set-frame-position",
        super::window_cmds::builtin_set_frame_position,
        3,
        Some(3),
    );
    ctx.defsubr(
        "frame-visible-p",
        super::window_cmds::builtin_frame_visible_p,
        0,
        None,
    );
    ctx.defsubr(
        "frame-live-p",
        super::window_cmds::builtin_frame_live_p,
        1,
        Some(1),
    );
    ctx.defsubr("framep", super::window_cmds::builtin_framep, 1, Some(1));
    ctx.defsubr("frame-id", builtin_frame_id, 0, Some(1));
    ctx.defsubr("frame-root-frame", builtin_frame_root_frame, 0, None);
    ctx.defsubr(
        "x-open-connection",
        super::display::builtin_x_open_connection,
        1,
        Some(3),
    );
    ctx.defsubr(
        "x-get-resource",
        super::display::builtin_x_get_resource,
        2,
        Some(4),
    );
    register_builtin(
        ctx,
        BuiltinRegistration::requires_eval_state(
            "window-system",
            super::display::builtin_window_system,
            0,
            Some(1),
        ),
    );
    ctx.defsubr(
        "x-server-version",
        super::display::builtin_x_server_version,
        0,
        Some(1),
    );
    ctx.defsubr(
        "x-server-input-extension-version",
        super::display::builtin_x_server_input_extension_version,
        0,
        None,
    );
    ctx.defsubr(
        "x-server-vendor",
        super::display::builtin_x_server_vendor,
        0,
        Some(1),
    );
    ctx.defsubr(
        "display-color-cells",
        super::display::builtin_display_color_cells,
        0,
        None,
    );
    ctx.defsubr(
        "x-display-mm-height",
        super::display::builtin_x_display_mm_height,
        0,
        Some(1),
    );
    ctx.defsubr(
        "x-display-mm-width",
        super::display::builtin_x_display_mm_width,
        0,
        Some(1),
    );
    ctx.defsubr(
        "x-display-planes",
        super::display::builtin_x_display_planes,
        0,
        Some(1),
    );
    ctx.defsubr(
        "x-display-screens",
        super::display::builtin_x_display_screens,
        0,
        Some(1),
    );
    ctx.defsubr(
        "x-close-connection",
        super::display::builtin_x_close_connection,
        1,
        Some(1),
    );
    ctx.defsubr(
        "call-interactively",
        super::interactive::builtin_call_interactively,
        1,
        Some(3),
    );
    ctx.defsubr(
        "self-insert-command",
        super::interactive::builtin_self_insert_command,
        1,
        Some(2),
    );
    ctx.defsubr(
        "read-number",
        super::reader::builtin_read_number,
        1,
        Some(2),
    );
    register_builtin(
        ctx,
        BuiltinRegistration::requires_eval_state("kill-emacs", builtin_kill_emacs, 0, Some(2)),
    );
    ctx.defsubr(
        "primitive-undo",
        super::undo::builtin_primitive_undo,
        2,
        Some(2),
    );
    ctx.defsubr("undo", super::undo::builtin_undo, 0, Some(1));
    ctx.defsubr(
        "buffer-disable-undo",
        builtin_buffer_disable_undo,
        0,
        Some(1),
    );
    ctx.defsubr(
        "move-marker",
        super::marker::builtin_move_marker,
        2,
        Some(3),
    );
    ctx.defsubr(
        "delete-char",
        super::editfns::builtin_delete_char,
        1,
        Some(2),
    );
    ctx.defsubr(
        "file-locked-p",
        super::filelock::builtin_file_locked_p,
        1,
        Some(1),
    );
    ctx.defsubr(
        "file-system-info",
        super::fileio::builtin_file_system_info,
        1,
        Some(1),
    );
    ctx.defsubr("macrop", super::builtins::symbols::builtin_macrop, 0, None);
    ctx.defsubr(
        "frame-parameter",
        super::window_cmds::builtin_frame_parameter,
        2,
        Some(2),
    );
    ctx.defsubr(
        "tty-type",
        super::terminal::pure::builtin_tty_type,
        0,
        Some(1),
    );
    ctx.defsubr(
        "tty-top-frame",
        super::terminal::pure::builtin_tty_top_frame,
        0,
        Some(1),
    );
    ctx.defsubr(
        "tty-display-color-p",
        super::terminal::pure::builtin_tty_display_color_p,
        0,
        None,
    );
    ctx.defsubr(
        "tty-display-color-cells",
        super::terminal::pure::builtin_tty_display_color_cells,
        0,
        None,
    );
    ctx.defsubr(
        "tty-no-underline",
        super::terminal::pure::builtin_tty_no_underline,
        0,
        Some(1),
    );
    ctx.defsubr(
        "controlling-tty-p",
        super::terminal::pure::builtin_controlling_tty_p,
        0,
        Some(1),
    );
    ctx.defsubr(
        "suspend-tty",
        super::terminal::pure::builtin_suspend_tty,
        0,
        Some(1),
    );
    ctx.defsubr(
        "resume-tty",
        super::terminal::pure::builtin_resume_tty,
        0,
        Some(1),
    );
    ctx.defsubr(
        "x-display-monitor-attributes-list",
        super::display::builtin_x_display_monitor_attributes_list,
        0,
        None,
    );
    ctx.defsubr(
        "prefix-numeric-value",
        |_ctx, args| builtin_prefix_numeric_value(args),
        0,
        None,
    );
    ctx.defsubr(
        "get-internal-run-time",
        |_ctx, args| builtin_get_internal_run_time(args),
        0,
        Some(0),
    );
    ctx.defsubr("daemonp", |_ctx, args| builtin_daemonp(args), 0, Some(0));
    ctx.defsubr(
        "daemon-initialized",
        |_ctx, args| builtin_daemon_initialized(args),
        0,
        Some(0),
    );
    ctx.defsubr(
        "invocation-directory",
        builtin_invocation_directory,
        0,
        Some(0),
    );
    ctx.defsubr("invocation-name", builtin_invocation_name, 0, Some(0));
    ctx.defsubr(
        "set-frame-size-and-position-pixelwise",
        super::window_cmds::builtin_set_frame_size_and_position_pixelwise,
        0,
        None,
    );
    ctx.defsubr(
        "mouse-position-in-root-frame",
        |_ctx, args| builtin_mouse_position_in_root_frame(args),
        0,
        None,
    );
    ctx.defsubr(
        "define-fringe-bitmap",
        |ctx, args| builtin_define_fringe_bitmap(ctx, args),
        2,
        Some(5),
    );
    ctx.defsubr(
        "destroy-fringe-bitmap",
        |ctx, args| builtin_destroy_fringe_bitmap(ctx, args),
        1,
        Some(1),
    );
    ctx.defsubr(
        "make-auto-save-file-name",
        super::fileio::builtin_make_auto_save_file_name,
        0,
        Some(0),
    );
    ctx.defsubr(
        "font-get-system-font",
        |_ctx, args| builtin_font_get_system_font(args),
        0,
        None,
    );
    ctx.defsubr(
        "font-get-system-normal-font",
        |_ctx, args| builtin_font_get_system_normal_font(args),
        0,
        None,
    );
    ctx.defsubr(
        "frame--set-was-invisible",
        |_ctx, args| builtin_frame_set_was_invisible(args),
        0,
        None,
    );
    ctx.defsubr(
        "frame-after-make-frame",
        |_ctx, args| builtin_frame_after_make_frame(args),
        0,
        None,
    );
    ctx.defsubr(
        "frame-ancestor-p",
        super::window_cmds::builtin_frame_ancestor_p,
        0,
        None,
    );
    ctx.defsubr(
        "frame-bottom-divider-width",
        super::window_cmds::builtin_frame_bottom_divider_width,
        0,
        Some(1),
    );
    ctx.defsubr(
        "frame-child-frame-border-width",
        super::window_cmds::builtin_frame_child_frame_border_width,
        0,
        Some(1),
    );
    ctx.defsubr(
        "frame-focus",
        super::window_cmds::builtin_frame_focus,
        0,
        Some(1),
    );
    ctx.defsubr(
        "frame-fringe-width",
        |_ctx, args| builtin_frame_fringe_width(args),
        0,
        Some(1),
    );
    ctx.defsubr(
        "frame-internal-border-width",
        super::window_cmds::builtin_frame_internal_border_width,
        0,
        Some(1),
    );
    ctx.defsubr(
        "frame-parent",
        super::window_cmds::builtin_frame_parent,
        0,
        None,
    );
    ctx.defsubr(
        "frame-pointer-visible-p",
        |_ctx, args| builtin_frame_pointer_visible_p(args),
        0,
        None,
    );
    ctx.defsubr(
        "frame-right-divider-width",
        super::window_cmds::builtin_frame_right_divider_width,
        0,
        Some(1),
    );
    ctx.defsubr(
        "frame-scale-factor",
        super::window_cmds::builtin_frame_scale_factor,
        0,
        Some(1),
    );
    ctx.defsubr(
        "frame-scroll-bar-height",
        |_ctx, args| builtin_frame_scroll_bar_height(args),
        0,
        Some(1),
    );
    ctx.defsubr(
        "frame-scroll-bar-width",
        |_ctx, args| builtin_frame_scroll_bar_width(args),
        0,
        Some(1),
    );
    ctx.defsubr(
        "frame-window-state-change",
        super::window_cmds::builtin_frame_window_state_change,
        0,
        None,
    );
    ctx.defsubr(
        "fringe-bitmaps-at-pos",
        |_ctx, args| builtin_fringe_bitmaps_at_pos(args),
        0,
        Some(2),
    );
    ctx.defsubr(
        "gpm-mouse-start",
        |_ctx, args| builtin_gpm_mouse_start(args),
        0,
        None,
    );
    ctx.defsubr(
        "gpm-mouse-stop",
        |_ctx, args| builtin_gpm_mouse_stop(args),
        0,
        None,
    );
    ctx.defsubr(
        "handle-save-session",
        |_ctx, args| builtin_handle_save_session(args),
        1,
        Some(1),
    );
    ctx.defsubr(
        "handle-switch-frame",
        |_ctx, args| builtin_handle_switch_frame(args),
        1,
        Some(1),
    );
    // byte-code: mirrors GNU Emacs Fbyte_code (src/bytecode.c).
    // Receives pre-evaluated args (bytestr, vector, maxdepth), decodes
    // the GNU bytecodes, and executes them via the bytecode VM.
    ctx.defsubr(
        "byte-code",
        |ctx, args| {
            crate::emacs_core::builtins::expect_args("byte-code", &args, 3)?;
            let bytestr = args[0];
            let constants_vec = args[1];
            let maxdepth = args[2];

            use crate::emacs_core::bytecode::ByteCodeFunction;
            use crate::emacs_core::bytecode::decode::decode_gnu_bytecode_with_offset_map;
            use crate::emacs_core::value::LambdaParams;

            // Bytecode strings are unibyte and may contain non-UTF-8 bytes.
            let raw_bytes = if let Some(ls) = bytestr.as_lisp_string() {
                ls.as_bytes().to_vec()
            } else {
                return Err(super::error::signal(
                    LispCondition::WrongTypeArgument,
                    vec![Value::symbol("stringp"), bytestr],
                ));
            };

            let mut constants: Vec<Value> = match constants_vec.kind() {
                ValueKind::Veclike(VecLikeType::Vector) => {
                    constants_vec.as_vector_data().unwrap().clone()
                }
                _ => {
                    return Err(super::error::signal(
                        LispCondition::WrongTypeArgument,
                        vec![Value::symbol("vectorp"), constants_vec],
                    ));
                }
            };

            for i in 0..constants.len() {
                constants[i] = super::builtins::try_convert_nested_compiled_literal(constants[i]);
            }

            let (ops, gnu_byte_offset_map) =
                decode_gnu_bytecode_with_offset_map(&raw_bytes, &mut constants).map_err(|e| {
                    super::error::signal(
                        "error",
                        vec![Value::string(format!("bytecode decode error: {}", e))],
                    )
                })?;

            let max_stack = match maxdepth.kind() {
                ValueKind::Fixnum(n) => n as u16,
                _ => 16,
            };

            let bc = ByteCodeFunction {
                source_id: super::bytecode::fresh_bytecode_source_id(),
                ops,
                constants,
                max_stack,
                params: LambdaParams::simple(vec![]),
                arglist: Value::NIL,
                lexical: false,
                env: None,
                gnu_byte_offset_map: Some(gnu_byte_offset_map),
                gnu_bytecode_bytes: None,
                docstring: None,
                doc_form: None,
                interactive: None,
                closure_slot_count: 4,
                extra_slots: Vec::new(),
                #[cfg(feature = "jit")]
                runtime: crate::emacs_core::jit::Runtime::new(),
            };

            ctx.refresh_features_from_variable();
            let mut vm = super::bytecode::Vm::from_context(ctx);
            let result = vm.execute(&bc, vec![]);
            ctx.sync_features_variable();
            result
        },
        0,
        None,
    );
    ctx.defsubr(
        "dump-emacs-portable",
        builtin_dump_emacs_portable,
        1,
        Some(2),
    );
    ctx.defsubr(
        "dump-emacs-portable--sort-predicate",
        |_ctx, args| builtin_dump_emacs_portable_sort_predicate(args),
        2,
        Some(2),
    );
    ctx.defsubr(
        "dump-emacs-portable--sort-predicate-copied",
        |_ctx, args| builtin_dump_emacs_portable_sort_predicate_copied(args),
        2,
        Some(2),
    );
    ctx.defsubr(
        "emacs-repository-get-version",
        |_ctx, args| builtin_emacs_repository_get_version(args),
        0,
        Some(0),
    );
    ctx.defsubr(
        "emacs-repository-get-branch",
        |_ctx, args| builtin_emacs_repository_get_branch(args),
        0,
        Some(0),
    );
    ctx.defsubr(
        "lower-frame",
        |_ctx, args| builtin_lower_frame(args),
        0,
        Some(1),
    );
    ctx.defsubr(
        "memory-limit",
        |_ctx, args| builtin_memory_limit(args),
        0,
        Some(0),
    );
    register_builtin(
        ctx,
        BuiltinRegistration::requires_eval_state(
            "make-frame-invisible",
            super::window_cmds::builtin_make_frame_invisible,
            0,
            Some(2),
        ),
    );
    ctx.defsubr(
        "menu-bar-menu-at-x-y",
        |ctx, args| builtin_menu_bar_menu_at_x_y(ctx, args),
        2,
        Some(3),
    );
    ctx.defsubr(
        "menu-or-popup-active-p",
        |_ctx, args| builtin_menu_or_popup_active_p(args),
        0,
        Some(0),
    );
    ctx.defsubr(
        "pdumper-stats",
        |_ctx, args| builtin_pdumper_stats(args),
        0,
        Some(0),
    );
    register_builtin(
        ctx,
        BuiltinRegistration::placeholder(
            "raise-frame",
            |_ctx, args| builtin_raise_frame(args),
            0,
            Some(1),
            BuiltinNoEvalPlaceholder::Nil,
        ),
    );
    ctx.defsubr(
        "neomacs--frame-snapshot",
        super::xdisp::builtin_neomacs_frame_snapshot,
        0,
        Some(2),
    );
    ctx.defsubr(
        "neomacs--write-frame-snapshot",
        super::xdisp::builtin_neomacs_write_frame_snapshot,
        1,
        Some(3),
    );
    ctx.defsubr(
        "reconsider-frame-fonts",
        builtin_reconsider_frame_fonts,
        1,
        Some(1),
    );
    ctx.defsubr(
        "redirect-frame-focus",
        super::window_cmds::builtin_redirect_frame_focus,
        1,
        Some(2),
    );
    ctx.defsubr(
        "set-frame-window-state-change",
        super::window_cmds::builtin_set_frame_window_state_change,
        0,
        Some(2),
    );
    ctx.defsubr(
        "set-fringe-bitmap-face",
        |ctx, args| builtin_set_fringe_bitmap_face(ctx, args),
        1,
        Some(2),
    );
    ctx.defsubr(
        "set-mouse-pixel-position",
        |ctx, args| builtin_set_mouse_pixel_position(ctx, args),
        3,
        Some(3),
    );
    ctx.defsubr(
        "set-mouse-position",
        |ctx, args| builtin_set_mouse_position(ctx, args),
        3,
        Some(3),
    );
    ctx.defsubr(
        "tool-bar-get-system-style",
        |_ctx, args| builtin_tool_bar_get_system_style(args),
        0,
        Some(0),
    );
    ctx.defsubr(
        "tool-bar-pixel-width",
        |_ctx, args| builtin_tool_bar_pixel_width(args),
        0,
        Some(1),
    );
    ctx.defsubr(
        "tty--output-buffer-size",
        |_ctx, args| builtin_tty_output_buffer_size(args),
        0,
        Some(1),
    );
    ctx.defsubr(
        "tty--set-output-buffer-size",
        |_ctx, args| builtin_tty_set_output_buffer_size(args),
        1,
        Some(2),
    );
    ctx.defsubr(
        "tty-display-pixel-height",
        builtin_tty_display_pixel_height,
        0,
        Some(1),
    );
    ctx.defsubr(
        "tty-display-pixel-width",
        builtin_tty_display_pixel_width,
        0,
        Some(1),
    );
    ctx.defsubr(
        "tty-frame-at",
        super::window_cmds::builtin_tty_frame_at,
        2,
        Some(2),
    );
    ctx.defsubr(
        "tty-frame-edges",
        super::window_cmds::builtin_tty_frame_edges,
        0,
        Some(2),
    );
    ctx.defsubr(
        "tty-frame-geometry",
        super::window_cmds::builtin_tty_frame_geometry,
        0,
        Some(1),
    );
    ctx.defsubr(
        "tty-frame-list-z-order",
        super::window_cmds::builtin_tty_frame_list_z_order,
        0,
        None,
    );
    ctx.defsubr(
        "tty-frame-restack",
        |_ctx, args| builtin_tty_frame_restack(args),
        0,
        None,
    );
    ctx.defsubr(
        "x-begin-drag",
        |_ctx, args| builtin_x_begin_drag(args),
        1,
        Some(6),
    );
    ctx.defsubr(
        "x-double-buffered-p",
        |_ctx, args| builtin_x_double_buffered_p(args),
        0,
        Some(1),
    );
    ctx.defsubr(
        "x-menu-bar-open-internal",
        |_ctx, args| builtin_x_menu_bar_open_internal(args),
        0,
        Some(1),
    );
    register_builtin(
        ctx,
        BuiltinRegistration::placeholder(
            "xw-color-defined-p",
            |ctx, args| super::font::builtin_xw_color_defined_p_ctx(ctx, args),
            1,
            Some(2),
            BuiltinNoEvalPlaceholder::Nil,
        ),
    );
    ctx.defsubr(
        "color-defined-p",
        |ctx, args| super::font::builtin_xw_color_defined_p_ctx(ctx, args),
        0,
        None,
    );
    register_builtin(
        ctx,
        BuiltinRegistration::placeholder(
            "xw-color-values",
            |ctx, args| super::font::builtin_xw_color_values_ctx(ctx, args),
            1,
            Some(2),
            BuiltinNoEvalPlaceholder::Nil,
        ),
    );
    ctx.defsubr(
        "color-values",
        |ctx, args| super::font::builtin_xw_color_values_ctx(ctx, args),
        0,
        None,
    );
    register_builtin(
        ctx,
        BuiltinRegistration::placeholder(
            "xw-display-color-p",
            |ctx, args| builtin_xw_display_color_p_ctx(ctx, args),
            0,
            Some(1),
            BuiltinNoEvalPlaceholder::Nil,
        ),
    );
    if INOTIFY_FEATURE_AVAILABLE {
        let _ = ctx.provide_value(Value::symbol("inotify"), None);
    }
    ctx.defsubr("lock-buffer", super::filelock::builtin_lock_buffer, 0, None);
    ctx.defsubr("lock-file", super::filelock::builtin_lock_file, 1, Some(1));
    ctx.defsubr(
        "unlock-buffer",
        super::filelock::builtin_unlock_buffer,
        0,
        Some(0),
    );
    ctx.defsubr(
        "unlock-file",
        super::filelock::builtin_unlock_file,
        1,
        Some(1),
    );
    ctx.defsubr(
        "treesit-language-version",
        |ctx, args| builtin_treesit_language_version(ctx, args),
        0,
        Some(1),
    );
    ctx.defsubr(
        "treesit-parser-changed-ranges",
        |ctx, args| builtin_treesit_parser_changed_ranges(ctx, args),
        1,
        Some(1),
    );
    if super::sqlite::SQLITE3_LISP_API_AVAILABLE {
        ctx.defsubr(
            "sqlite-close",
            |_ctx, args| super::sqlite::builtin_sqlite_close(args),
            1,
            Some(1),
        );
        ctx.defsubr(
            "sqlite-columns",
            |_ctx, args| super::sqlite::builtin_sqlite_columns(args),
            1,
            Some(1),
        );
        ctx.defsubr(
            "sqlite-commit",
            |_ctx, args| super::sqlite::builtin_sqlite_commit(args),
            1,
            Some(1),
        );
        ctx.defsubr(
            "sqlite-execute",
            |_ctx, args| super::sqlite::builtin_sqlite_execute(args),
            2,
            Some(3),
        );
        ctx.defsubr(
            "sqlite-execute-batch",
            |ctx, args| super::sqlite::builtin_sqlite_execute_batch(ctx, args),
            2,
            Some(2),
        );
        ctx.defsubr(
            "sqlite-finalize",
            |_ctx, args| super::sqlite::builtin_sqlite_finalize(args),
            1,
            Some(1),
        );
        ctx.defsubr(
            "sqlite-load-extension",
            |ctx, args| super::sqlite::builtin_sqlite_load_extension(ctx, args),
            2,
            Some(2),
        );
        ctx.defsubr(
            "sqlite-more-p",
            |_ctx, args| super::sqlite::builtin_sqlite_more_p(args),
            1,
            Some(1),
        );
        ctx.defsubr(
            "sqlite-next",
            |_ctx, args| super::sqlite::builtin_sqlite_next(args),
            1,
            Some(1),
        );
        ctx.defsubr(
            "sqlite-open",
            |_ctx, args| super::sqlite::builtin_sqlite_open(args),
            0,
            Some(3),
        );
        ctx.defsubr(
            "sqlite-pragma",
            |_ctx, args| super::sqlite::builtin_sqlite_pragma(args),
            2,
            Some(2),
        );
        ctx.defsubr(
            "sqlite-rollback",
            |_ctx, args| super::sqlite::builtin_sqlite_rollback(args),
            1,
            Some(1),
        );
        ctx.defsubr(
            "sqlite-select",
            |_ctx, args| super::sqlite::builtin_sqlite_select(args),
            2,
            Some(4),
        );
        ctx.defsubr(
            "sqlite-transaction",
            |_ctx, args| super::sqlite::builtin_sqlite_transaction(args),
            1,
            Some(1),
        );
        ctx.defsubr(
            "sqlite-version",
            |_ctx, args| super::sqlite::builtin_sqlite_version(args),
            0,
            Some(0),
        );
    }
    ctx.defsubr(
        "neomacs-frame-geometry",
        |_ctx, args| builtin_neomacs_frame_geometry(args),
        0,
        None,
    );
    ctx.defsubr(
        "neomacs-frame-edges",
        super::window_cmds::builtin_neomacs_frame_edges,
        0,
        Some(2),
    );
    ctx.defsubr(
        "neomacs-mouse-absolute-pixel-position",
        |_ctx, args| builtin_neomacs_mouse_absolute_pixel_position(args),
        0,
        None,
    );
    ctx.defsubr(
        "neomacs-set-mouse-absolute-pixel-position",
        |_ctx, args| builtin_neomacs_set_mouse_absolute_pixel_position(args),
        0,
        None,
    );
    ctx.defsubr(
        "neomacs-set-cursor-blink",
        builtin_neomacs_set_cursor_blink,
        1,
        Some(2),
    );
    ctx.defsubr(
        "neomacs-set-cursor-animation",
        builtin_neomacs_set_cursor_animation,
        1,
        Some(2),
    );
    register_cursor_effect_subrs(ctx);
    ctx.defsubr(
        "neomacs-display-monitor-attributes-list",
        builtin_neomacs_display_monitor_attributes_list,
        0,
        None,
    );
    ctx.defsubr(
        "x-scroll-bar-foreground",
        |_ctx, args| builtin_x_scroll_bar_foreground(args),
        0,
        None,
    );
    ctx.defsubr(
        "x-scroll-bar-background",
        |_ctx, args| builtin_x_scroll_bar_background(args),
        0,
        None,
    );
    ctx.defsubr(
        "neomacs-clipboard-set",
        builtin_neomacs_clipboard_set,
        0,
        None,
    );
    ctx.defsubr(
        "neomacs-clipboard-get",
        builtin_neomacs_clipboard_get,
        0,
        None,
    );
    ctx.defsubr(
        "neomacs-primary-selection-set",
        builtin_neomacs_primary_selection_set,
        0,
        None,
    );
    ctx.defsubr(
        "neomacs-primary-selection-get",
        builtin_neomacs_primary_selection_get,
        0,
        None,
    );
    ctx.defsubr(
        "neomacs-core-backend",
        |_ctx, args| builtin_neomacs_core_backend(args),
        0,
        None,
    );
    ctx.defsubr(
        "neomacs-buffer-text-backend",
        builtin_neomacs_buffer_text_backend,
        0,
        None,
    );
    ctx.defsubr(
        "neomacs-default-buffer-text-backend",
        builtin_neomacs_default_buffer_text_backend,
        0,
        None,
    );
    ctx.defsubr(
        "neomacs-set-default-buffer-text-backend",
        builtin_neomacs_set_default_buffer_text_backend,
        0,
        None,
    );
    ctx.defsubr(
        "neomacs-set-buffer-text-backend",
        builtin_neomacs_set_buffer_text_backend,
        0,
        None,
    );
    ctx.defsubr(
        "frame-windows-min-size",
        |_ctx, args| builtin_frame_windows_min_size(args),
        0,
        None,
    );
    ctx.defsubr(
        "select-frame-set-input-focus",
        super::window_cmds::builtin_select_frame_set_input_focus,
        0,
        None,
    );
    ctx.defsubr(
        "modify-frame-parameters",
        super::window_cmds::builtin_modify_frame_parameters,
        2,
        Some(2),
    );
    ctx.defsubr(
        "x-display-pixel-width",
        super::display::builtin_x_display_pixel_width,
        0,
        Some(1),
    );
    ctx.defsubr(
        "x-display-pixel-height",
        super::display::builtin_x_display_pixel_height,
        0,
        Some(1),
    );
    ctx.defsubr(
        "x-server-max-request-size",
        super::display::builtin_x_server_max_request_size,
        0,
        Some(1),
    );
    ctx.defsubr(
        "x-display-grayscale-p",
        super::display::builtin_x_display_grayscale_p,
        0,
        Some(1),
    );
    ctx.defsubr(
        "x-display-backing-store",
        super::display::builtin_x_display_backing_store,
        0,
        Some(1),
    );
    ctx.defsubr(
        "x-display-color-cells",
        super::display::builtin_x_display_color_cells,
        0,
        Some(1),
    );
    ctx.defsubr(
        "x-display-save-under",
        super::display::builtin_x_display_save_under,
        0,
        Some(1),
    );
    ctx.defsubr(
        "x-display-set-last-user-time",
        super::display::builtin_x_display_set_last_user_time,
        0,
        None,
    );
    ctx.defsubr(
        "x-display-visual-class",
        super::display::builtin_x_display_visual_class,
        0,
        Some(1),
    );

    // Pure builtins from builtins_extra (previously in old match dispatch).
    // These don't need &mut Context, so we wrap them.
    macro_rules! defsubr_pure {
        ($ctx:expr, $name:expr, $func:expr) => {
            $ctx.defsubr($name, |_eval, args| $func(args), 0, None);
        };
    }
    defsubr_pure!(ctx, "take", super::builtins_extra::builtin_take);
    defsubr_pure!(
        ctx,
        "assoc-string",
        super::builtins_extra::builtin_assoc_string
    );
    defsubr_pure!(
        ctx,
        "string-search",
        super::builtins_extra::builtin_string_search
    );
    defsubr_pure!(
        ctx,
        "bare-symbol-p",
        super::builtins_extra::builtin_bare_symbol_p
    );
    defsubr_pure!(ctx, "byteorder", super::builtins_extra::builtin_byteorder);
    defsubr_pure!(
        ctx,
        "car-less-than-car",
        super::builtins_extra::builtin_car_less_than_car
    );
    defsubr_pure!(
        ctx,
        "proper-list-p",
        super::builtins_extra::builtin_proper_list_p
    );
    defsubr_pure!(ctx, "subrp", super::builtins_extra::builtin_subrp);
    defsubr_pure!(
        ctx,
        "byte-code-function-p",
        super::builtins_extra::builtin_byte_code_function_p
    );
    defsubr_pure!(ctx, "natnump", super::builtins_extra::builtin_natnump);
    defsubr_pure!(ctx, "emacs-pid", super::builtins_extra::builtin_emacs_pid);
    defsubr_pure!(
        ctx,
        "memory-use-counts",
        super::builtins_extra::builtin_memory_use_counts
    );
    ctx.defsubr_1("not", builtin_not_1, 1);
    ctx.defsubr(
        "list-of-strings-p",
        |_ctx, args| builtin_list_of_strings_p(args),
        0,
        None,
    );
    ctx.defsubr_1("booleanp", builtin_booleanp_1, 1);
    ctx.defsubr(
        "integer-or-null-p",
        |_ctx, args| builtin_integer_or_null_p(args),
        0,
        None,
    );
    ctx.defsubr(
        "string-or-null-p",
        |_ctx, args| builtin_string_or_null_p(args),
        0,
        None,
    );
    ctx.defsubr(
        "char-uppercase-p",
        |_ctx, args| builtin_char_uppercase_p(args),
        0,
        None,
    );
    ctx.defsubr("ignore", |_ctx, args| builtin_ignore(args), 0, None);
    ctx.defsubr("string=", |_ctx, args| builtin_string_equal(args), 0, None);
    ctx.defsubr("string<", |_ctx, args| builtin_string_lessp(args), 0, None);
    ctx.defsubr(
        "string-greaterp",
        |_ctx, args| builtin_string_greaterp(args),
        0,
        None,
    );
    ctx.defsubr(
        "string>",
        |_ctx, args| builtin_string_greaterp(args),
        0,
        None,
    );
    ctx.defsubr(
        "combine-after-change-execute",
        |ctx, args| builtin_combine_after_change_execute(ctx, args),
        0,
        Some(0),
    );
    ctx.defsubr(
        "set-text-conversion-style",
        |_ctx, args| super::coding::builtin_set_text_conversion_style(args),
        0,
        None,
    );
    ctx.defsubr(
        "set-buffer-file-coding-system",
        super::coding::builtin_set_buffer_file_coding_system,
        1,
        Some(3),
    );

    // -- CCL (eval-dependent) --
    ctx.defsubr("ccl-program-p", builtin_ccl_program_p, 1, Some(1));
    ctx.defsubr("ccl-execute", builtin_ccl_execute, 2, Some(2));
    ctx.defsubr(
        "ccl-execute-on-string",
        builtin_ccl_execute_on_string,
        3,
        Some(5),
    );
    ctx.defsubr(
        "register-ccl-program",
        builtin_register_ccl_program,
        0,
        None,
    );
    ctx.defsubr(
        "register-code-conversion-map",
        builtin_register_code_conversion_map,
        0,
        None,
    );

    // -- Display/terminal --
    ctx.defsubr(
        "x-export-frames",
        |_ctx, args| super::display::builtin_x_export_frames(args),
        0,
        Some(2),
    );
    ctx.defsubr(
        "x-backspace-delete-keys-p",
        |_ctx, args| super::display::builtin_x_backspace_delete_keys_p(args),
        0,
        None,
    );
    ctx.defsubr(
        "x-change-window-property",
        |_ctx, args| super::display::builtin_x_change_window_property(args),
        2,
        Some(7),
    );
    ctx.defsubr(
        "x-focus-frame",
        |ctx, args| super::display::builtin_x_focus_frame(ctx, args),
        1,
        Some(2),
    );
    ctx.defsubr(
        "x-get-local-selection",
        |_ctx, args| super::display::builtin_x_get_local_selection(args),
        0,
        Some(2),
    );
    ctx.defsubr(
        "x-get-modifier-masks",
        |_ctx, args| super::display::builtin_x_get_modifier_masks(args),
        0,
        None,
    );
    ctx.defsubr(
        "x-get-selection-internal",
        |_ctx, args| super::display::builtin_x_get_selection_internal(args),
        2,
        Some(4),
    );
    ctx.defsubr(
        "x-display-list",
        |_ctx, args| super::display::builtin_x_display_list(args),
        0,
        Some(0),
    );
    ctx.defsubr(
        "x-disown-selection-internal",
        |_ctx, args| super::display::builtin_x_disown_selection_internal(args),
        1,
        Some(3),
    );
    ctx.defsubr(
        "x-delete-window-property",
        |_ctx, args| super::display::builtin_x_delete_window_property(args),
        1,
        Some(3),
    );
    ctx.defsubr(
        "x-frame-edges",
        |_ctx, args| super::display::builtin_x_frame_edges(args),
        0,
        Some(2),
    );
    ctx.defsubr(
        "x-frame-geometry",
        |_ctx, args| super::display::builtin_x_frame_geometry(args),
        0,
        Some(1),
    );
    ctx.defsubr(
        "x-frame-list-z-order",
        |_ctx, args| super::display::builtin_x_frame_list_z_order(args),
        0,
        None,
    );
    ctx.defsubr(
        "x-frame-restack",
        |_ctx, args| super::display::builtin_x_frame_restack(args),
        2,
        Some(3),
    );
    ctx.defsubr(
        "x-get-atom-name",
        |_ctx, args| super::display::builtin_x_get_atom_name(args),
        0,
        None,
    );
    ctx.defsubr(
        "x-mouse-absolute-pixel-position",
        |_ctx, args| super::display::builtin_x_mouse_absolute_pixel_position(args),
        0,
        None,
    );
    ctx.defsubr(
        "x-own-selection-internal",
        |_ctx, args| super::display::builtin_x_own_selection_internal(args),
        2,
        Some(3),
    );
    ctx.defsubr(
        "x-parse-geometry",
        |_ctx, args| super::display::builtin_x_parse_geometry(args),
        1,
        Some(1),
    );
    ctx.defsubr(
        "x-popup-dialog",
        |ctx, args| super::display::builtin_x_popup_dialog(ctx, args),
        2,
        Some(3),
    );
    ctx.defsubr(
        "x-popup-menu",
        |ctx, args| super::display::builtin_x_popup_menu(ctx, args),
        2,
        Some(2),
    );
    ctx.defsubr(
        "x-register-dnd-atom",
        |_ctx, args| super::display::builtin_x_register_dnd_atom(args),
        0,
        None,
    );
    ctx.defsubr(
        "x-selection-exists-p",
        |_ctx, args| super::display::builtin_x_selection_exists_p(args),
        0,
        None,
    );
    ctx.defsubr(
        "x-selection-owner-p",
        |_ctx, args| super::display::builtin_x_selection_owner_p(args),
        0,
        None,
    );
    ctx.defsubr(
        "x-hide-tip",
        |_ctx, args| super::display::builtin_x_hide_tip(args),
        0,
        Some(0),
    );
    ctx.defsubr(
        "x-internal-focus-input-context",
        |_ctx, args| super::display::builtin_x_internal_focus_input_context(args),
        0,
        None,
    );
    ctx.defsubr(
        "x-send-client-message",
        |_ctx, args| super::display::builtin_x_send_client_message(args),
        6,
        Some(6),
    );
    ctx.defsubr(
        "x-show-tip",
        |_ctx, args| super::display::builtin_x_show_tip(args),
        1,
        Some(6),
    );
    ctx.defsubr(
        "x-set-mouse-absolute-pixel-position",
        |_ctx, args| super::display::builtin_x_set_mouse_absolute_pixel_position(args),
        2,
        Some(2),
    );
    ctx.defsubr(
        "x-synchronize",
        |_ctx, args| super::display::builtin_x_synchronize(args),
        1,
        Some(2),
    );
    ctx.defsubr(
        "x-translate-coordinates",
        |_ctx, args| super::display::builtin_x_translate_coordinates(args),
        0,
        None,
    );
    ctx.defsubr(
        "x-uses-old-gtk-dialog",
        |_ctx, args| super::display::builtin_x_uses_old_gtk_dialog(args),
        0,
        None,
    );
    ctx.defsubr(
        "x-window-property",
        |_ctx, args| super::display::builtin_x_window_property(args),
        1,
        Some(6),
    );
    ctx.defsubr(
        "x-window-property-attributes",
        |_ctx, args| super::display::builtin_x_window_property_attributes(args),
        0,
        None,
    );
    ctx.defsubr(
        "x-wm-set-size-hint",
        |_ctx, args| super::display::builtin_x_wm_set_size_hint(args),
        0,
        None,
    );
    register_builtin(
        ctx,
        BuiltinRegistration::requires_eval_state(
            "make-terminal-frame",
            super::window_cmds::builtin_make_terminal_frame,
            1,
            Some(1),
        ),
    );

    // -- Window builtins: display-buffer, switch-to-buffer, pop-to-buffer --
    ctx.defsubr(
        "switch-to-buffer",
        super::window_cmds::builtin_switch_to_buffer,
        1,
        Some(3),
    );
    ctx.defsubr(
        "display-buffer",
        super::window_cmds::builtin_display_buffer,
        1,
        Some(3),
    );
    ctx.defsubr(
        "pop-to-buffer",
        super::window_cmds::builtin_pop_to_buffer,
        1,
        Some(3),
    );

    // -- Window tree / resize builtins --
    ctx.defsubr(
        "balance-windows",
        super::window_cmds::builtin_balance_windows,
        0,
        Some(1),
    );
    ctx.defsubr(
        "enlarge-window",
        super::window_cmds::builtin_enlarge_window,
        1,
        Some(2),
    );
    ctx.defsubr(
        "shrink-window",
        super::window_cmds::builtin_shrink_window,
        1,
        Some(2),
    );
    ctx.defsubr(
        "window-tree",
        super::window_cmds::builtin_window_tree,
        0,
        Some(1),
    );

    // GNU exposes public evaluator-owned entries like `if` and `throw` as
    // real subrs in the function cell even though they are dispatched by the
    // evaluator rather than the ordinary builtin function table.
    symbols::init_event_symbol_properties(&mut ctx.obarray);
    ctx.materialize_public_evaluator_function_cells();
}

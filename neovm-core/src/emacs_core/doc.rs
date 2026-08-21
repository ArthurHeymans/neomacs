//! Documentation and help support builtins.
//!
//! Provides:
//! - `documentation` — retrieve docstring from a function
//! - `documentation-property` — retrieve documentation property
//! - `Snarf-documentation` — internal DOC file loader compatibility shim

use super::error::{EvalResult, Flow, signal};
use super::intern::{intern, resolve_sym};
use super::value::*;
use crate::emacs_core::error::LispCondition;
use crate::emacs_core::error::expect_args;
use std::fs::File;
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Argument helpers
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Eval-dependent builtins
// ---------------------------------------------------------------------------

/// `(documentation FUNCTION &optional RAW)` -- return the docstring of FUNCTION.
///
/// Looks up FUNCTION in the obarray's function cell. If the function cell
/// holds a `Lambda` (or `Macro`) with a docstring, returns it as a string.
/// Otherwise returns nil.  Unless RAW is non-nil, string results are passed
/// through `substitute-command-keys`, matching GNU Emacs.
pub(crate) fn builtin_documentation(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let raw = args.get(1).is_some_and(|v| v.is_truthy());
    let (plan, lisp_directory) = documentation_plan(eval, args)?;
    finish_documentation_result(
        execute_documentation_plan(
            plan,
            |execution| match execution {
                DocumentationExecution::Eval(value) => eval.eval_value(&value),
                DocumentationExecution::FunctionDoc(function) => {
                    eval.apply(Value::symbol("function-documentation"), vec![function])
                }
            },
            lisp_directory.as_deref(),
        )?,
        raw,
        |value| maybe_substitute_command_keys(eval, value),
    )
}

enum DocumentationPlan {
    Final(Value),
    Eval(Value),
    FunctionDoc(Value),
}

enum DocumentationExecution {
    Eval(Value),
    FunctionDoc(Value),
}

fn execute_documentation_plan(
    plan: DocumentationPlan,
    mut execute: impl FnMut(DocumentationExecution) -> EvalResult,
    lisp_directory: Option<&str>,
) -> EvalResult {
    match plan {
        DocumentationPlan::Final(value) => Ok(value),
        DocumentationPlan::Eval(value) => execute(DocumentationExecution::Eval(value)),
        DocumentationPlan::FunctionDoc(function) => {
            let doc = execute(DocumentationExecution::FunctionDoc(function))?;
            documentation_result_from_raw_doc(lisp_directory, doc)
        }
    }
}

fn finish_documentation_result(
    value: Value,
    raw: bool,
    mut substitute_command_keys: impl FnMut(Value) -> EvalResult,
) -> EvalResult {
    if raw || !value.is_string() {
        Ok(value)
    } else {
        substitute_command_keys(value)
    }
}

fn maybe_substitute_command_keys(eval: &mut super::eval::Context, value: Value) -> EvalResult {
    if eval
        .obarray()
        .symbol_function_id(intern("substitute-command-keys"))
        .is_none()
    {
        return Ok(value);
    }

    eval.eval_value(&Value::list(vec![
        Value::symbol("substitute-command-keys"),
        value,
    ]))
}

fn documentation_plan(
    eval: &super::eval::Context,
    args: Vec<Value>,
) -> Result<(DocumentationPlan, Option<String>), Flow> {
    expect_min_max_args("documentation", &args, 1, 2)?;
    let obarray = eval.obarray();
    let lisp_directory = obarray.symbol_value("lisp-directory").and_then(|v| {
        v.as_lisp_string()
            .map(|ls| crate::emacs_core::emacs_char::to_utf8_lossy(ls.as_bytes()))
    });

    // GNU doc.c:Fdocumentation calls Fget on the original symbol before
    // looking at the function cell.  Keep that exact object identity so
    // uninterned symbols and symbols-with-pos use the same path as `get`.
    if super::builtins::symbols::symbol_id_checked(&args[0], eval.symbols_with_pos_enabled)
        .is_some()
    {
        let prop_key = Value::symbol("function-documentation");
        if let Some(prop) =
            super::builtins::symbols::symbol_property_get(eval, args[0], prop_key)?.1
            && !prop.is_nil()
        {
            let plan = documentation_plan_from_property_value(lisp_directory.as_deref(), prop)?;
            return Ok((plan, lisp_directory));
        }
    }

    let function =
        resolve_documentation_function_value(obarray, args[0], eval.symbols_with_pos_enabled)?;
    let plan = if obarray
        .symbol_function_id(intern("function-documentation"))
        .is_some()
    {
        DocumentationPlan::FunctionDoc(function)
    } else {
        DocumentationPlan::Final(function_doc_or_error(function)?)
    };
    Ok((plan, lisp_directory))
}

fn documentation_result_from_raw_doc(lisp_directory: Option<&str>, value: Value) -> EvalResult {
    if value == Value::fixnum(0) {
        return Ok(Value::NIL);
    }

    if let Some((file, position)) = compiled_doc_ref(&value) {
        return load_compiled_doc_string(lisp_directory, &file, position);
    }

    Ok(value)
}

fn resolve_documentation_function_value(
    obarray: &super::symbol::Obarray,
    function: Value,
    symbols_with_pos_enabled: bool,
) -> EvalResult {
    let mut resolved =
        if super::builtins::symbols::symbol_id_checked(&function, symbols_with_pos_enabled)
            .is_some()
        {
            let func = super::builtins::symbols::symbol_function_impl_1_checked(
                obarray,
                function,
                symbols_with_pos_enabled,
            )?;
            if func.is_nil() {
                return Err(signal(LispCondition::VoidFunction, vec![function]));
            }
            func
        } else {
            function
        };

    if let Some(alias_symbol) =
        super::builtins::symbols::symbol_id_checked(&resolved, symbols_with_pos_enabled)
        && let Some(indirect) =
            super::builtins::symbols::resolve_indirect_symbol_by_id_in_obarray_checked(
                obarray,
                alias_symbol,
                symbols_with_pos_enabled,
            )
            .map(|(_, value)| value)
    {
        resolved = indirect;
    }

    Ok(resolved)
}

fn function_doc_or_error(func_val: Value) -> EvalResult {
    if let Some(result) = quoted_lambda_documentation(&func_val) {
        return result;
    }
    if let Some(result) = quoted_macro_invalid_designator(&func_val) {
        return result;
    }

    match func_val.kind() {
        ValueKind::Veclike(VecLikeType::Lambda) | ValueKind::Veclike(VecLikeType::Macro) => {
            Ok(func_val
                .closure_docstring()
                .flatten()
                .map_or(Value::NIL, |doc| Value::heap_string(doc.clone())))
        }
        ValueKind::Subr(_) | ValueKind::Veclike(VecLikeType::Subr) => {
            // `SnarfedSubr::of` is `Fsnarf_documentation`'s `Ffboundp` clause
            // (`src/doc.c:617-621`) carried by the type: it is the only key
            // `subr_docs::lookup` accepts, and only a subr `Value` can make
            // one.  Both subr representations reach here, and both are subrs.
            let subr =
                super::subr_docs::SnarfedSubr::of(func_val).expect("matched on a subr Value");
            let doc = super::subr_docs::lookup(&subr).unwrap_or("Built-in function.");
            Ok(Value::string(doc))
        }
        ValueKind::String | ValueKind::Veclike(VecLikeType::Vector) => {
            Ok(Value::string("Keyboard macro."))
        }
        ValueKind::Veclike(VecLikeType::ByteCode) => {
            let bc = func_val.get_bytecode_data().unwrap();
            Ok(bc
                .docstring
                .as_ref()
                .map_or(Value::NIL, |doc| Value::heap_string(doc.clone())))
        }
        _other => Err(signal(LispCondition::InvalidFunction, vec![func_val])),
    }
}

fn quoted_lambda_documentation(function: &Value) -> Option<EvalResult> {
    if !function.is_cons() {
        return None;
    };

    let pair_car = function.cons_car();
    let pair_cdr = function.cons_cdr();
    if pair_car.as_symbol_name() != Some("lambda") {
        return None;
    }

    let mut tail = pair_cdr;

    if !tail.is_cons() {
        return Some(Err(signal(LispCondition::InvalidFunction, vec![*function])));
    };
    let _params_and_body_car = tail.cons_car();
    let params_and_body_cdr = tail.cons_cdr();
    tail = params_and_body_cdr;

    match tail.kind() {
        ValueKind::Nil => Some(Ok(Value::NIL)),
        ValueKind::Cons => {
            let body_car = tail.cons_car();
            let _body_cdr = tail.cons_cdr();
            if body_car.is_string() {
                Some(Ok(body_car))
            } else {
                Some(Ok(Value::NIL))
            }
        }
        _other => Some(Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("listp"), tail],
        ))),
    }
}

fn quoted_macro_invalid_designator(function: &Value) -> Option<EvalResult> {
    if !function.is_cons() {
        return None;
    };

    let pair_car = function.cons_car();
    let pair_cdr = function.cons_cdr();
    if pair_car.as_symbol_name() != Some("macro") {
        return None;
    }

    let payload = pair_cdr;
    if payload.is_nil() {
        return Some(Err(signal(LispCondition::VoidFunction, vec![Value::NIL])));
    }

    // GNU extracts the docstring from the function part of (macro . fn),
    // rather than signaling invalid-function.
    Some(function_doc_or_error(payload))
}

fn documentation_plan_from_property_value(
    lisp_directory: Option<&str>,
    value: Value,
) -> Result<DocumentationPlan, Flow> {
    if value.is_string() {
        return Ok(DocumentationPlan::Final(value));
    }

    if let Some((file, position)) = compiled_doc_ref(&value) {
        return load_compiled_doc_string(lisp_directory, &file, position)
            .map(DocumentationPlan::Final);
    }

    // Integer doc offsets require DOC-file lookup; return nil when unresolved.
    if value.is_fixnum() {
        return Ok(DocumentationPlan::Final(Value::NIL));
    }

    Ok(DocumentationPlan::Eval(value))
}

fn compiled_doc_ref(value: &Value) -> Option<(String, i64)> {
    if !value.is_cons() {
        return None;
    };
    let pair_car = value.cons_car();
    let pair_cdr = value.cons_cdr();
    Some((
        pair_car
            .as_lisp_string()
            .map(|ls| crate::emacs_core::emacs_char::to_utf8_lossy(ls.as_bytes()))?,
        pair_cdr.as_int()?,
    ))
}

fn resolve_compiled_doc_path(lisp_directory: Option<&str>, file: &str) -> PathBuf {
    let path = Path::new(file);
    if path.is_absolute() {
        return path.to_path_buf();
    }

    if let Some(dir) = lisp_directory {
        return Path::new(dir).join(path);
    }

    path.to_path_buf()
}

fn compiled_doc_prefix_is_valid(prefix: &[u8]) -> bool {
    if prefix.is_empty() {
        return false;
    }

    let mut test = 1_usize;
    if prefix[prefix.len() - test] == 0x1f {
        return true;
    }
    if prefix[prefix.len() - test] != b' ' {
        return false;
    }
    test += 1;
    while prefix.len() >= test && prefix[prefix.len() - test].is_ascii_digit() {
        test += 1;
    }
    if prefix.len() < test || prefix[prefix.len() - test] != b'@' {
        return false;
    }
    test += 1;
    prefix.len() >= test && prefix[prefix.len() - test] == b'#'
}

fn decode_compiled_doc_bytes(bytes: &[u8]) -> EvalResult {
    let mut out = Vec::with_capacity(bytes.len());
    let mut pos = 0_usize;
    while pos < bytes.len() {
        if bytes[pos] != 0x01 {
            out.push(bytes[pos]);
            pos += 1;
            continue;
        }

        pos += 1;
        let Some(&escaped) = bytes.get(pos) else {
            return Err(signal(
                "error",
                vec![Value::string(
                    "Invalid data in documentation file -- dangling ^A escape",
                )],
            ));
        };
        match escaped {
            0x01 => out.push(0x01),
            b'0' => out.push(0x00),
            b'_' => out.push(0x1f),
            other => {
                return Err(signal(
                    "error",
                    vec![Value::string(format!(
                        "Invalid data in documentation file -- ^A followed by code {:03o}",
                        other
                    ))],
                ));
            }
        }
        pos += 1;
    }

    Ok(Value::string(super::load::decode_emacs_utf8(&out)))
}

fn load_compiled_doc_string(lisp_directory: Option<&str>, file: &str, position: i64) -> EvalResult {
    let position = position.unsigned_abs();
    let resolved = resolve_compiled_doc_path(lisp_directory, file);
    let mut handle = match File::open(&resolved) {
        Ok(file_handle) => file_handle,
        Err(err) if matches!(err.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory) => {
            return Ok(Value::string(format!(
                "Cannot open doc string file \"{file}\"\n"
            )));
        }
        Err(err) => {
            return Err(signal(
                LispCondition::FileError,
                vec![
                    Value::string("Read error on documentation file"),
                    Value::string(format!("{}: {}", resolved.display(), err)),
                ],
            ));
        }
    };

    let prefix_len = usize::try_from(position.min(1024)).unwrap_or(1024);
    let start = position.saturating_sub(prefix_len as u64);
    handle.seek(SeekFrom::Start(start)).map_err(|_| {
        signal(
            "error",
            vec![Value::string(format!(
                "Position {position} out of range in doc string file \"{file}\""
            ))],
        )
    })?;

    let offset = prefix_len;
    let mut buffer = Vec::with_capacity(prefix_len + 8192);
    let mut chunk = [0_u8; 8192];
    let end_index = loop {
        let read = handle.read(&mut chunk).map_err(|err| {
            signal(
                LispCondition::FileError,
                vec![
                    Value::string("Read error on documentation file"),
                    Value::string(format!("{}: {}", resolved.display(), err)),
                ],
            )
        })?;
        if read == 0 {
            break None;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.len() > offset
            && let Some(pos) = buffer[offset..].iter().position(|&byte| byte == 0x1f)
        {
            break Some(offset + pos);
        }
    };

    let Some(end_index) = end_index else {
        return Ok(Value::NIL);
    };

    if offset == 0 || buffer.len() < offset || !compiled_doc_prefix_is_valid(&buffer[..offset]) {
        return Ok(Value::NIL);
    }

    decode_compiled_doc_bytes(&buffer[offset..end_index])
}

fn startup_doc_quote_style_display(doc: &str) -> String {
    let mut out = String::with_capacity(doc.len());
    let mut backtick_open = false;
    let mut escaped_backtick_open = false;
    let mut chars = doc.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.peek().copied() {
                Some('`') => {
                    chars.next();
                    escaped_backtick_open = true;
                    backtick_open = false;
                    continue;
                }
                Some('\'') if escaped_backtick_open => {
                    chars.next();
                    escaped_backtick_open = false;
                    continue;
                }
                _ => {
                    out.push(ch);
                    continue;
                }
            }
        }

        if escaped_backtick_open {
            if ch == '\'' {
                escaped_backtick_open = false;
            } else {
                out.push(ch);
            }
            continue;
        }

        match ch {
            '`' => {
                if backtick_open {
                    out.push('\u{2019}');
                    backtick_open = false;
                } else {
                    out.push('\u{2018}');
                    backtick_open = true;
                }
            }
            '\'' => {
                out.push('\u{2019}');
                if backtick_open {
                    backtick_open = false;
                }
            }
            _ => out.push(ch),
        }
    }

    out
}

fn startup_doc_quote_style_raw(doc: &str) -> String {
    doc.chars()
        .map(|ch| match ch {
            '\u{2018}' => '`',
            '\u{2019}' => '\'',
            _ => ch,
        })
        .collect()
}

/// `(documentation-property SYMBOL PROP &optional RAW)` -- return the
/// documentation property PROP of SYMBOL.
///
/// Context-aware implementation:
/// - validates SYMBOL as a symbol designator (`symbolp`)
/// - returns nil when PROP is not a symbol (matching Emacs `get`-like behavior)
/// - unresolved integer doc offsets return nil
/// - non-integer values are evaluated as Lisp and returned
/// - unless RAW is non-nil, string results are passed through
///   `substitute-command-keys`
pub(crate) fn builtin_documentation_property(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let raw = args.get(2).is_some_and(|v| v.is_truthy());
    let plan = documentation_property_plan(eval, args)?;
    finish_documentation_result(
        execute_documentation_plan(
            plan,
            |execution| match execution {
                DocumentationExecution::Eval(value) => eval.eval_value(&value),
                DocumentationExecution::FunctionDoc(_) => unreachable!(),
            },
            None,
        )?,
        raw,
        |value| maybe_substitute_command_keys(eval, value),
    )
}

fn documentation_property_plan(
    eval: &super::eval::Context,
    args: Vec<Value>,
) -> Result<DocumentationPlan, Flow> {
    expect_min_max_args("documentation-property", &args, 2, 3)?;
    let obarray = eval.obarray();
    let lisp_directory = obarray.symbol_value("lisp-directory").and_then(|v| {
        v.as_lisp_string()
            .map(|ls| crate::emacs_core::emacs_char::to_utf8_lossy(ls.as_bytes()))
    });

    let prop = args[1];
    let (symbol_id, mut property_value) =
        super::builtins::symbols::symbol_property_get(eval, args[0], prop)?;
    let mut documentation_symbol_id = symbol_id;
    let prop_is_variable_documentation = eq_value_swp(
        &prop,
        &Value::symbol("variable-documentation"),
        eval.symbols_with_pos_enabled,
    );

    // GNU doc.c:Fdocumentation_property retries variable aliases only for
    // `variable-documentation' when the direct property lookup returned nil.
    if prop_is_variable_documentation
        && property_value.as_ref().is_none_or(|value| value.is_nil())
        && let Some(indirect) = obarray.indirect_variable_id(symbol_id)
        && indirect != symbol_id
    {
        let plist = obarray.symbol_plist_id(indirect);
        property_value =
            crate::emacs_core::plist::plist_get_swp(plist, &prop, eval.symbols_with_pos_enabled);
        documentation_symbol_id = indirect;
    }

    let sym = resolve_sym(documentation_symbol_id);
    let raw = args.get(2).is_some_and(|v| v.is_truthy());

    // `Fsnarf_documentation`'s `Fboundp` gate (`src/doc.c:606-613`), asked once
    // here and passed to `var_docs::lookup` as the only key it accepts.  GNU
    // asks it at dump time and stores the answer as a plist property; Neomacs
    // has no DOC file to snarf, so the generated table is consulted lazily and
    // the gate has to travel with the query.
    let snarfed =
        super::var_docs::SnarfedVariable::if_bound_in(obarray, documentation_symbol_id, sym);

    // GNU's two sources, and there is no third (ledger 178).
    //
    // A plist entry exists only because Lisp `defvar'/`defvaralias' put one
    // there (`src/eval.c:723`, `src/eval.c:911`); everything else GNU knows
    // came from `Fsnarf_documentation' reading `etc/DOC' behind the gate
    // above.  Neomacs has no DOC file, so `var_docs::gnu_table' stands in for
    // it and is read lazily -- but only through [`var_docs::SnarfedDoc`],
    // which cannot be constructed without the gate having said yes.
    match property_value {
        Some(value) => documentation_plan_from_property_value(lisp_directory.as_deref(), value),
        // No plist entry: this is where `Fsnarf_documentation' would have put
        // one, so ask its question and read `etc/DOC' if the answer is yes.
        //
        // The grave/curly conversion is applied here because a caller may be
        // in a context where `substitute-command-keys' (lisp/help.el) is not
        // reachable.
        //
        // Other property names fall through to nil: only
        // `variable-documentation' has a central source.
        _ if prop_is_variable_documentation => {
            let Some(doc) = snarfed.and_then(super::var_docs::lookup) else {
                return Ok(DocumentationPlan::Final(Value::NIL));
            };
            let text = doc.text();
            let doc = if raw {
                startup_doc_quote_style_raw(text)
            } else {
                startup_doc_quote_style_display(text)
            };
            Ok(DocumentationPlan::Final(Value::string(doc)))
        }
        _ => Ok(DocumentationPlan::Final(Value::NIL)),
    }
}

// ---------------------------------------------------------------------------
// Pure builtins
// ---------------------------------------------------------------------------

/// `(Snarf-documentation FILENAME)` -- load documentation strings from
/// the internal DOC file.
///
/// Compatibility implementation: accepts the canonical `"DOC"` token and
/// preserves observed GNU Emacs error classes for invalid and missing paths.
/// It does not load or parse an on-disk DOC table yet.
fn snarf_doc_path_invalid(filename: &str) -> bool {
    if filename.is_empty() {
        return true;
    }

    let mut segments = filename
        .split('/')
        .filter(|segment| !segment.is_empty())
        .peekable();
    if segments.peek().is_none() {
        return true;
    }

    segments.all(|segment| segment == "." || segment == "..")
}

pub(crate) fn builtin_snarf_documentation(args: Vec<Value>) -> EvalResult {
    expect_args("Snarf-documentation", &args, 1)?;
    let filename = match args[0].as_utf8_str() {
        Some(name) => name,
        None => {
            return Err(signal(
                LispCondition::WrongTypeArgument,
                vec![Value::symbol("stringp"), args[0]],
            ));
        }
    };

    // In batch compatibility mode, allow the canonical DOC token while
    // preserving observable error classes for invalid/missing names.
    if filename == "DOC" {
        return Ok(Value::NIL);
    }

    if filename.starts_with("DOC/") {
        return Err(signal(
            LispCondition::FileError,
            vec![
                Value::string("Read error"),
                Value::string(format!("/usr/share/emacs/etc/{filename}")),
            ],
        ));
    }

    if snarf_doc_path_invalid(filename) {
        return Err(signal(
            "error",
            vec![Value::string("DOC file invalid at position 0")],
        ));
    }

    Err(signal(
        LispCondition::FileMissing,
        vec![
            Value::string("Opening doc string file"),
            Value::string("No such file or directory"),
            Value::string(format!("/usr/share/emacs/etc/{filename}")),
        ],
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
#[path = "doc_test.rs"]
mod tests;

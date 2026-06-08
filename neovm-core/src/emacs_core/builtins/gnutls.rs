use super::{
    EvalResult, Flow, Value, ValueKind, expect_args, expect_range_args, expect_strict_string,
    signal,
};

pub(crate) fn builtin_gnutls_available_p(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-available-p", &args, 0)?;
    Ok(Value::list(
        crate::emacs_core::tls::gnutls_available_capabilities()
            .iter()
            .map(|capability| Value::symbol(*capability))
            .collect(),
    ))
}

pub(crate) fn builtin_gnutls_ciphers(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-ciphers", &args, 0)?;
    ensure_gnutls_available()?;
    Ok(Value::list(vec![
        gnutls_cipher_entry("AES-256-GCM", 10, true, 16, 1, 32, 12),
        gnutls_cipher_entry("AES-256-CBC", 5, false, 0, 16, 32, 16),
    ]))
}

fn gnutls_cipher_entry(
    name: &str,
    cipher_id: i64,
    aead_capable: bool,
    tag_size: i64,
    block_size: i64,
    key_size: i64,
    iv_size: i64,
) -> Value {
    Value::list(vec![
        Value::symbol(name),
        Value::keyword(":cipher-id"),
        Value::fixnum(cipher_id),
        Value::keyword(":type"),
        Value::symbol("gnutls-symmetric-cipher"),
        Value::keyword(":cipher-aead-capable"),
        Value::bool_val(aead_capable),
        Value::keyword(":cipher-tagsize"),
        Value::fixnum(tag_size),
        Value::keyword(":cipher-blocksize"),
        Value::fixnum(block_size),
        Value::keyword(":cipher-keysize"),
        Value::fixnum(key_size),
        Value::keyword(":cipher-ivsize"),
        Value::fixnum(iv_size),
    ])
}

pub(crate) fn builtin_gnutls_digests(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-digests", &args, 0)?;
    ensure_gnutls_available()?;
    Ok(Value::list(vec![Value::symbol("SHA256")]))
}

pub(crate) fn builtin_gnutls_macs(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-macs", &args, 0)?;
    ensure_gnutls_available()?;
    Ok(Value::list(vec![Value::symbol("AEAD")]))
}

pub(crate) fn builtin_gnutls_errorp(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-errorp", &args, 1)?;
    if args[0] == Value::T || args[0].is_symbol_named("gnutls-e-again") {
        Ok(Value::NIL)
    } else {
        Ok(Value::T)
    }
}

pub(crate) fn builtin_gnutls_error_string(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-error-string", &args, 1)?;
    let message = if args[0] == Value::T {
        "Not an error"
    } else if args[0].is_symbol_named("gnutls-e-again") {
        "Resource temporarily unavailable, try again."
    } else if args[0].is_symbol_named("gnutls-e-interrupted") {
        "Function was interrupted."
    } else {
        match args[0].kind() {
            ValueKind::Fixnum(0) => "Success.",
            ValueKind::Nil => "Symbol has no numeric gnutls-code property",
            _ => "(unknown error code)",
        }
    };
    Ok(Value::string(message))
}

pub(crate) fn builtin_gnutls_error_fatalp(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-error-fatalp", &args, 1)?;
    if args[0].is_nil() {
        return Err(signal(
            "error",
            vec![Value::string("Symbol has no numeric gnutls-code property")],
        ));
    }
    Ok(Value::NIL)
}

fn expect_processp(value: &Value) -> Result<(), Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) if n >= 0 => Ok(()),
        _ => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), *value],
        )),
    }
}

pub(crate) fn builtin_gnutls_peer_status_warning_describe(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-peer-status-warning-describe", &args, 1)?;
    if args[0].is_nil() {
        return Ok(Value::NIL);
    }
    if args[0].as_symbol_name().is_none() {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("symbolp"), args[0]],
        ));
    }
    Ok(Value::NIL)
}

pub(crate) fn builtin_gnutls_format_certificate(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-format-certificate", &args, 1)?;
    let _ = expect_strict_string(&args[0])?;
    Ok(Value::string("Certificate"))
}

pub(crate) fn builtin_gnutls_hash_digest(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-hash-digest", &args, 2)?;
    ensure_gnutls_available()?;
    if args[0].is_nil() {
        return Err(signal(
            "error",
            vec![
                Value::string("GnuTLS digest-method is invalid or not found"),
                Value::NIL,
            ],
        ));
    }
    if args[0].as_symbol_name().is_none() {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("symbolp"), args[0]],
        ));
    }
    let _ = expect_strict_string(&args[1])?;
    Ok(Value::string("digest"))
}

pub(crate) fn builtin_gnutls_hash_mac(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-hash-mac", &args, 3)?;
    ensure_gnutls_available()?;
    if args[0].is_nil() {
        return Err(signal(
            "error",
            vec![
                Value::string("GnuTLS MAC-method is invalid or not found"),
                Value::NIL,
            ],
        ));
    }
    if args[0].as_symbol_name().is_none() {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("symbolp"), args[0]],
        ));
    }
    let _ = expect_strict_string(&args[1])?;
    let _ = expect_strict_string(&args[2])?;
    Ok(Value::string("mac"))
}

pub(crate) fn builtin_gnutls_symmetric_decrypt(args: Vec<Value>) -> EvalResult {
    expect_range_args("gnutls-symmetric-decrypt", &args, 4, 5)?;
    ensure_gnutls_available()?;
    gnutls_symmetric_result(&args[2], &args[3])
}

pub(crate) fn builtin_gnutls_symmetric_encrypt(args: Vec<Value>) -> EvalResult {
    expect_range_args("gnutls-symmetric-encrypt", &args, 4, 5)?;
    ensure_gnutls_available()?;
    gnutls_symmetric_result(&args[2], &args[3])
}

fn ensure_gnutls_available() -> Result<(), Flow> {
    Err(signal(
        "error",
        vec![Value::string("GnuTLS crypto capability is not available")],
    ))
}

fn gnutls_symmetric_result(iv: &Value, input: &Value) -> EvalResult {
    let data = extract_gnutls_data(input, "input")?;
    let actual_iv = extract_gnutls_iv(iv)?;
    Ok(Value::list(vec![data, actual_iv]))
}

fn extract_gnutls_iv(value: &Value) -> Result<Value, Flow> {
    if let Some(items) = crate::emacs_core::value::list_to_vec(value) {
        if items.len() == 2 && items[0] == Value::symbol("iv-auto") {
            let size = match items[1].kind() {
                ValueKind::Fixnum(n) if n >= 0 => n as usize,
                _ => {
                    return Err(signal(
                        "wrong-type-argument",
                        vec![Value::symbol("natnump"), items[1]],
                    ));
                }
            };
            return Ok(Value::heap_string(
                crate::heap_types::LispString::from_emacs_bytes(vec![0; size]),
            ));
        }
    }
    extract_gnutls_data(value, "IV")
}

fn extract_gnutls_data(value: &Value, label: &str) -> Result<Value, Flow> {
    if let Some(string) = value.as_lisp_string() {
        return Ok(Value::heap_string(string.clone()));
    }
    if let Some(items) = crate::emacs_core::value::list_to_vec(value)
        && items.len() == 1
        && let Some(string) = items[0].as_lisp_string()
    {
        return Ok(Value::heap_string(string.clone()));
    }
    Err(signal(
        "wrong-type-argument",
        vec![Value::symbol("stringp"), *value, Value::string(label)],
    ))
}

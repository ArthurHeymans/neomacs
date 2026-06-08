use super::{
    EvalResult, Value, ValueKind, expect_args, expect_range_args, expect_strict_string, signal,
};
use crate::emacs_core::tls::format_x509_certificate_pem;

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
    gnutls_crypto_unavailable()
}

pub(crate) fn builtin_gnutls_digests(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-digests", &args, 0)?;
    gnutls_crypto_unavailable()
}

pub(crate) fn builtin_gnutls_macs(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-macs", &args, 0)?;
    gnutls_crypto_unavailable()
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
    let cert = expect_strict_string(&args[0])?;
    let formatted = format_x509_certificate_pem(cert.as_bytes()).map_err(|err| {
        signal(
            "error",
            vec![Value::string(format!(
                "gnutls-format-certificate error: {err}"
            ))],
        )
    })?;
    Ok(Value::string(formatted))
}

pub(crate) fn builtin_gnutls_hash_digest(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-hash-digest", &args, 2)?;
    gnutls_crypto_unavailable()
}

pub(crate) fn builtin_gnutls_hash_mac(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-hash-mac", &args, 3)?;
    gnutls_crypto_unavailable()
}

pub(crate) fn builtin_gnutls_symmetric_decrypt(args: Vec<Value>) -> EvalResult {
    expect_range_args("gnutls-symmetric-decrypt", &args, 4, 5)?;
    gnutls_crypto_unavailable()
}

pub(crate) fn builtin_gnutls_symmetric_encrypt(args: Vec<Value>) -> EvalResult {
    expect_range_args("gnutls-symmetric-encrypt", &args, 4, 5)?;
    gnutls_crypto_unavailable()
}

fn gnutls_crypto_unavailable() -> EvalResult {
    Err(signal(
        "error",
        vec![Value::string("GnuTLS crypto capability is not available")],
    ))
}

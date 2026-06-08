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
    let message = match gnutls_error_code(args[0]) {
        GnutlsErrorCode::SuccessSentinel => "Not an error",
        GnutlsErrorCode::Code(code) => gnutls_error_code_string(code),
        GnutlsErrorCode::SymbolWithoutCode => "Symbol has no numeric gnutls-code property",
        GnutlsErrorCode::InvalidObject => "Not an error symbol or code",
    };
    Ok(Value::string(message))
}

pub(crate) fn builtin_gnutls_error_fatalp(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-error-fatalp", &args, 1)?;
    match gnutls_error_code(args[0]) {
        GnutlsErrorCode::SuccessSentinel | GnutlsErrorCode::Code(0) => Ok(Value::NIL),
        GnutlsErrorCode::Code(-28 | -52) => Ok(Value::NIL),
        GnutlsErrorCode::Code(code) if code < 0 => Ok(Value::T),
        GnutlsErrorCode::Code(_) => Ok(Value::NIL),
        GnutlsErrorCode::SymbolWithoutCode => Err(signal(
            "error",
            vec![Value::string("Symbol has no numeric gnutls-code property")],
        )),
        GnutlsErrorCode::InvalidObject => Err(signal(
            "error",
            vec![Value::string("Not an error symbol or code")],
        )),
    }
}

enum GnutlsErrorCode {
    SuccessSentinel,
    Code(i64),
    SymbolWithoutCode,
    InvalidObject,
}

fn gnutls_error_code(value: Value) -> GnutlsErrorCode {
    if value == Value::T {
        return GnutlsErrorCode::SuccessSentinel;
    }
    match value.kind() {
        ValueKind::Fixnum(code) => GnutlsErrorCode::Code(code),
        ValueKind::Nil => GnutlsErrorCode::SymbolWithoutCode,
        ValueKind::Symbol(_) => match value.as_symbol_name() {
            Some("gnutls-e-again") => GnutlsErrorCode::Code(-28),
            Some("gnutls-e-interrupted") => GnutlsErrorCode::Code(-52),
            Some("gnutls-e-invalid-session") => GnutlsErrorCode::Code(-10),
            Some("gnutls-e-not-ready-for-handshake") => GnutlsErrorCode::Code(-65500),
            _ => GnutlsErrorCode::SymbolWithoutCode,
        },
        _ => GnutlsErrorCode::InvalidObject,
    }
}

fn gnutls_error_code_string(code: i64) -> &'static str {
    match code {
        0 => "Success.",
        -28 => "Resource temporarily unavailable, try again.",
        -52 => "Function was interrupted.",
        -10 => "The specified session has been invalidated for some reason.",
        _ => "(unknown error code)",
    }
}

pub(crate) fn builtin_gnutls_peer_status_warning_describe(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-peer-status-warning-describe", &args, 1)?;
    if args[0].is_nil() {
        return Ok(Value::NIL);
    }
    let Some(symbol) = args[0].as_symbol_name() else {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("symbolp"), args[0]],
        ));
    };
    let Some(description) = gnutls_peer_status_warning_description(symbol) else {
        return Ok(Value::NIL);
    };
    Ok(Value::string(description))
}

fn gnutls_peer_status_warning_description(symbol: &str) -> Option<&'static str> {
    match symbol {
        ":invalid" => Some("certificate could not be verified"),
        ":revoked" => Some("certificate was revoked (CRL)"),
        ":self-signed" => Some("certificate signer was not found (self-signed)"),
        ":unknown-ca" => {
            Some("the certificate was signed by an unknown and therefore untrusted authority")
        }
        ":not-ca" => Some("certificate signer is not a CA"),
        ":insecure" => Some("certificate was signed with an insecure algorithm"),
        ":not-activated" => Some("certificate is not yet activated"),
        ":expired" => Some("certificate has expired"),
        ":no-host-match" => Some("certificate host does not match hostname"),
        ":signature-failure" => Some("certificate signature could not be verified"),
        ":revocation-data-superseded" => {
            Some("certificate revocation data are old and have been superseded")
        }
        ":revocation-data-issued-in-future" => {
            Some("certificate revocation data have a future issue date")
        }
        ":signer-constraints-failure" => Some("certificate signer constraints were violated"),
        ":purpose-mismatch" => Some("certificate does not match the intended purpose"),
        ":missing-ocsp-status" => Some(
            "certificate requires the server to send a OCSP certificate status, but no status was received",
        ),
        ":invalid-ocsp-status" => Some("the received OCSP certificate status is invalid"),
        _ => None,
    }
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

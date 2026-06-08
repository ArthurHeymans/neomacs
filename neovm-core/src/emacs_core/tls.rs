//! TLS transport and GNU-compatible TLS facade support.
//!
//! Rustls is the default transport backend, but it is deliberately kept behind
//! this module so process management and Elisp builtins do not depend on
//! rustls-specific types.

use super::builtins::{EvalResult, expect_args};
use super::value::Value;
use base64::Engine;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use x509_parser::prelude::{FromDer, X509Certificate, parse_x509_pem};

pub(crate) fn gnutls_available_capabilities() -> &'static [&'static str] {
    &["gnutls3", "gnutls"]
}

pub(crate) fn builtin_neomacs_tls_available_p(args: Vec<Value>) -> EvalResult {
    expect_args("neomacs-tls-available-p", &args, 0)?;
    Ok(Value::T)
}

#[derive(Debug)]
pub(crate) struct CertificateFormatError {
    message: String,
}

impl CertificateFormatError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for CertificateFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

pub(crate) fn format_x509_certificate_pem(
    pem_bytes: &[u8],
) -> Result<String, CertificateFormatError> {
    let (_, pem) = parse_x509_pem(pem_bytes)
        .map_err(|err| CertificateFormatError::new(format!("cannot import X.509 PEM: {err}")))?;
    if pem.label != "CERTIFICATE" {
        return Err(CertificateFormatError::new(format!(
            "expected CERTIFICATE PEM block, got {}",
            pem.label
        )));
    }

    let (remaining, cert) = X509Certificate::from_der(&pem.contents)
        .map_err(|err| CertificateFormatError::new(format!("cannot parse X.509 DER: {err}")))?;
    if !remaining.is_empty() {
        return Err(CertificateFormatError::new(
            "trailing data after X.509 certificate",
        ));
    }

    Ok(format_x509_certificate(&cert))
}

fn format_x509_certificate(cert: &X509Certificate<'_>) -> String {
    let validity = cert.validity();
    let mut out = String::new();
    out.push_str("X.509 Certificate\n");
    out.push_str(&format!("Version: {}\n", cert.version().0 + 1));
    out.push_str(&format!("Serial Number: {}\n", cert.raw_serial_as_string()));
    out.push_str(&format!("Issuer: {}\n", cert.issuer()));
    out.push_str(&format!("Subject: {}\n", cert.subject()));
    out.push_str(&format!("Not Before: {}\n", validity.not_before));
    out.push_str(&format!("Not After: {}\n", validity.not_after));
    out.push_str(&format!(
        "Public Key Algorithm: {}\n",
        cert.public_key().algorithm.algorithm
    ));
    out.push_str(&format!(
        "Signature Algorithm: {}\n",
        cert.signature_algorithm.algorithm
    ));
    if !cert.extensions().is_empty() {
        out.push_str("Extensions:\n");
        for extension in cert.extensions() {
            out.push_str(&format!(
                "  {}{}\n",
                extension.oid,
                if extension.critical {
                    " (critical)"
                } else {
                    ""
                }
            ));
        }
    }
    out
}

type RustlsClientStream = rustls::StreamOwned<rustls::ClientConnection, TcpStream>;

pub(crate) struct RustlsTlsStream {
    inner: RustlsClientStream,
    peer_certificates_pem: Vec<String>,
}

/// Backend-neutral TLS stream owned by a Neomacs process.
pub(crate) enum TlsStream {
    Rustls(RustlsTlsStream),
}

impl TlsStream {
    fn rustls(inner: RustlsClientStream, peer_certificates_pem: Vec<String>) -> Self {
        Self::Rustls(RustlsTlsStream {
            inner,
            peer_certificates_pem,
        })
    }

    pub(crate) fn set_nonblocking(&self, nonblocking: bool) -> std::io::Result<()> {
        match self {
            Self::Rustls(stream) => stream.inner.sock.set_nonblocking(nonblocking),
        }
    }

    pub(crate) fn peer_certificates_pem(&self) -> &[String] {
        match self {
            Self::Rustls(stream) => &stream.peer_certificates_pem,
        }
    }
}

impl Read for TlsStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Rustls(stream) => stream.inner.read(buf),
        }
    }
}

impl Write for TlsStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Rustls(stream) => stream.inner.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Rustls(stream) => stream.inner.flush(),
        }
    }
}

/// Error produced by a TLS backend before conversion to GNU-shaped Lisp errors.
#[derive(Debug)]
pub(crate) enum TlsBackendError {
    InvalidHostname(String),
    Connect(String),
    UnexpectedEof,
    Io(std::io::Error),
}

impl std::fmt::Display for TlsBackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidHostname(host) => write!(f, "Invalid hostname for TLS: {host}"),
            Self::Connect(err) => write!(f, "TLS handshake failed: {err}"),
            Self::UnexpectedEof => f.write_str("TLS handshake: unexpected EOF"),
            Self::Io(err) => write!(f, "TLS handshake: {err}"),
        }
    }
}

/// TLS transport backend boundary.
///
/// The process layer owns backend-neutral `TlsStream` values, while each
/// backend handles its own handshake, certificate roots, and error conversion.
pub(crate) trait TlsClientBackend {
    fn connect_client(tcp_stream: TcpStream, hostname: &str) -> Result<TlsStream, TlsBackendError>;
}

/// Rustls-backed TLS transport implementation.
pub(crate) struct RustlsBackend;

impl TlsClientBackend for RustlsBackend {
    fn connect_client(tcp_stream: TcpStream, hostname: &str) -> Result<TlsStream, TlsBackendError> {
        let root_store =
            rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let server_name: rustls_pki_types::ServerName<'_> = hostname
            .to_owned()
            .try_into()
            .map_err(|_| TlsBackendError::InvalidHostname(hostname.to_owned()))?;

        let tls_conn = rustls::ClientConnection::new(Arc::new(config), server_name)
            .map_err(|err| TlsBackendError::Connect(err.to_string()))?;

        tcp_stream
            .set_nonblocking(false)
            .map_err(TlsBackendError::Io)?;
        let mut tls_stream = rustls::StreamOwned::new(tls_conn, tcp_stream);

        let mut dummy = [0u8; 0];
        match tls_stream.read(&mut dummy) {
            Ok(_) => {}
            Err(ref err) if err.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(ref err) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(TlsBackendError::UnexpectedEof);
            }
            Err(err) => return Err(TlsBackendError::Io(err)),
        }

        let peer_certificates_pem = tls_stream
            .conn
            .peer_certificates()
            .map(|certs| {
                certs
                    .iter()
                    .map(|cert| der_certificate_to_pem(cert.as_ref()))
                    .collect()
            })
            .unwrap_or_default();
        let stream = TlsStream::rustls(tls_stream, peer_certificates_pem);
        stream.set_nonblocking(true).ok();
        Ok(stream)
    }
}

pub(crate) fn der_certificate_to_pem(der: &[u8]) -> String {
    let encoded = base64::engine::general_purpose::STANDARD.encode(der);
    let mut pem = String::from("-----BEGIN CERTIFICATE-----\n");
    for chunk in encoded.as_bytes().chunks(64) {
        pem.push_str(std::str::from_utf8(chunk).expect("base64 output is ASCII"));
        pem.push('\n');
    }
    pem.push_str("-----END CERTIFICATE-----\n");
    pem
}

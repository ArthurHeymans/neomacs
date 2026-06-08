//! TLS transport and GNU-compatible TLS facade support.
//!
//! Rustls is the default transport backend, but it is deliberately kept behind
//! this module so process management and Elisp builtins do not depend on
//! rustls-specific types.

use super::builtins::{EvalResult, expect_args};
use super::value::Value;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;

pub(crate) fn gnutls_available_capabilities() -> &'static [&'static str] {
    &["gnutls3", "gnutls"]
}

pub(crate) fn builtin_neomacs_tls_available_p(args: Vec<Value>) -> EvalResult {
    expect_args("neomacs-tls-available-p", &args, 0)?;
    Ok(Value::T)
}

/// Backend-neutral TLS stream owned by a Neomacs process.
pub struct TlsStream {
    inner: RustlsClientStream,
}

type RustlsClientStream = rustls::StreamOwned<rustls::ClientConnection, TcpStream>;

impl TlsStream {
    fn new(inner: RustlsClientStream) -> Self {
        Self { inner }
    }

    pub(crate) fn set_nonblocking(&self, nonblocking: bool) -> std::io::Result<()> {
        self.inner.sock.set_nonblocking(nonblocking)
    }
}

impl Read for TlsStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Write for TlsStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
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

/// Rustls-backed TLS transport implementation.
pub(crate) struct RustlsBackend;

impl RustlsBackend {
    pub(crate) fn connect_client(
        tcp_stream: TcpStream,
        hostname: &str,
    ) -> Result<TlsStream, TlsBackendError> {
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

        let stream = TlsStream::new(tls_stream);
        stream.set_nonblocking(true).ok();
        Ok(stream)
    }
}

#[cfg(test)]
mod tests {
    use super::{TlsBackendError, gnutls_available_capabilities};

    #[test]
    fn backend_errors_render_boundary_messages() {
        assert_eq!(
            TlsBackendError::InvalidHostname("bad host".to_owned()).to_string(),
            "Invalid hostname for TLS: bad host"
        );
        assert_eq!(
            TlsBackendError::Connect("bad cert".to_owned()).to_string(),
            "TLS handshake failed: bad cert"
        );
        assert_eq!(
            TlsBackendError::UnexpectedEof.to_string(),
            "TLS handshake: unexpected EOF"
        );
    }

    #[test]
    fn rustls_backend_advertises_conservative_gnutls_compatibility() {
        assert_eq!(gnutls_available_capabilities(), &["gnutls3", "gnutls"]);
    }
}

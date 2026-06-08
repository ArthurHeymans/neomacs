use super::tls::{
    TlsBackendError, der_certificate_to_pem, format_x509_certificate_pem,
    gnutls_available_capabilities,
};

const TEST_CERTIFICATE_PEM: &str = concat!(
    "-----BEGIN CERTIFICATE-----\n",
    "MIIFWzCCBEOgAwIBAgISAyBIAwu7NBD5CTxX8suDCMgFMA0GCSqGSIb3DQEBCwUA\n",
    "MEoxCzAJBgNVBAYTAlVTMRYwFAYDVQQKEw1MZXQncyBFbmNyeXB0MSMwIQYDVQQD\n",
    "ExpMZXQncyBFbmNyeXB0IEF1dGhvcml0eSBYMzAeFw0xOTA3MTIxMTEyMzBaFw0x\n",
    "OTEwMTAxMTEyMzBaMB0xGzAZBgNVBAMTEmxpc3RzLmZvci1vdXIuaW5mbzCCASIw\n",
    "DQYJKoZIhvcNAQEBBQADggEPADCCAQoCggEBAMVoti34X46DaI2nX24C+aZ2Ofkm\n",
    "hKbidiXiRTon1MLSMGl1oNW9MyRyYYCzP4j6DNKChJnr8ZnVShh2oZD+yHWP9lpn\n",
    "XMGkbsUxejRMU9hnaAB50pXRIDAzavkVFCguFlJ8nKkv/Y1Avlw7tc2aZOd3lOZB\n",
    "Er8gJ8mRDGqqsNU+Z12I6slEstzGMpsq6AewCVw4lMjdWWgugzUrxQTRAsG87on6\n",
    "gOiQH2cMODN3L7Fq4KOLQIjb3/luQhAQhpdKmEGFLin3c+f5or3thCDuwwDtOU1l\n",
    "Zf+8t9S8pZPLrZrIs6H2xjXqCRuUY7iRNbO18Ukc6rlDYhBj9LT+cpmBbHECAwEA\n",
    "AaOCAmYwggJiMA4GA1UdDwEB/wQEAwIFoDAdBgNVHSUEFjAUBggrBgEFBQcDAQYI\n",
    "KwYBBQUHAwIwDAYDVR0TAQH/BAIwADAdBgNVHQ4EFgQUJj2pvRtl3GloH3He6FX1\n",
    "ds3X0VEwHwYDVR0jBBgwFoAUqEpqYwR93brm0Tm3pkVl7/Oo7KEwbwYIKwYBBQUH\n",
    "AQEEYzBhMC4GCCsGAQUFBzABhiJodHRwOi8vb2NzcC5pbnQteDMubGV0c2VuY3J5\n",
    "cHQub3JnMC8GCCsGAQUFBzAChiNodHRwOi8vY2VydC5pbnQteDMubGV0c2VuY3J5\n",
    "cHQub3JnLzAdBgNVHREEFjAUghJsaXN0cy5mb3Itb3VyLmluZm8wTAYDVR0gBEUw\n",
    "QzAIBgZngQwBAgEwNwYLKwYBBAGC3xMBAQEwKDAmBggrBgEFBQcCARYaaHR0cDov\n",
    "L2Nwcy5sZXRzZW5jcnlwdC5vcmcwggEDBgorBgEEAdZ5AgQCBIH0BIHxAO8AdgAp\n",
    "PFGWVMg5ZbqqUPxYB9S3b79Yeily3KTDDPTlRUf0eAAAAWvmGV7yAAAEAwBHMEUC\n",
    "ICQL2Sm14aCMLxX9a9RbySgyBfichMRdbu6QA2Mbrl4eAiEA1vgJ7snqUWCgoqEE\n",
    "3SEfK3ioMopzWBsPvG6LdCuCMRAAdQBvU3asMfAxGdiZAKRRFf93FRwR2QLBACkG\n",
    "jbIImjfZEwAAAWvmGV9oAAAEAwBGMEQCIExGqw3Lo0nSCyUuTRf92FgGASwWYji5\n",
    "UGnXuYnpJrAvAiBw8AWVag8fzZ4ogAhY9EFRNdLrUcBjStipL888vyuxKzANBgkq\n",
    "hkiG9w0BAQsFAAOCAQEAF8BBLDvSWZg57B6aDtzfUTSGetCYs3k0vJqCJlL+Pz7/\n",
    "UruCSsojQzp5R6jvvgYQ83MaIdwe2mgt+OCQB5v7ylctyBzBmYIw9nPnxEC7HlcJ\n",
    "L2K/k5ZjJFRnv4kV1Si8+TIpEAV0ksf39KGKemG8kGi4GXV1v03zSv0p8aCarpuo\n",
    "SKBJ4qlB0CvmS2MqV4KnzO0O2h0c/ZQ4jg7l53eiN7VPdRMMO1DRw+MaW6I/hEZp\n",
    "+oZQ7hhKXgKUBvF4IGwyrfyIZ8AeWKG4IP98COgyRbz7qtrAVevRKCM0ZC2t04A2\n",
    "Fcix40FKEeiE093Aj3cweMYxNLPgwgQP8Xu3kA5QEw==\n",
    "-----END CERTIFICATE-----\n",
);

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

#[test]
fn format_x509_certificate_rejects_invalid_pem() {
    assert!(format_x509_certificate_pem(b"x").is_err());
}

#[test]
fn format_x509_certificate_extracts_parsed_fields() {
    let formatted =
        format_x509_certificate_pem(TEST_CERTIFICATE_PEM.as_bytes()).expect("valid cert");
    assert!(formatted.contains("X.509 Certificate"));
    assert!(formatted.contains("Subject: CN=lists.for-our.info"));
    assert!(formatted.contains("Issuer: C=US, O=Let's Encrypt, CN=Let's Encrypt Authority X3"));
    assert!(formatted.contains("Signature Algorithm: 1.2.840.113549.1.1.11"));
}

#[test]
fn der_certificates_are_formatted_as_pem_blocks() {
    assert_eq!(
        der_certificate_to_pem(&[1, 2, 3]),
        "-----BEGIN CERTIFICATE-----\nAQID\n-----END CERTIFICATE-----\n"
    );
}

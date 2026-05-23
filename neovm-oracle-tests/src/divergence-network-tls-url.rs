//! Divergence tests: network, socket, TLS, URL deep.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_network_options() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'network-interface-list)
  (fboundp 'network-interface-info)
  (fboundp 'format-network-address)) "#,
    );
}

#[test]
fn divergence_socket_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'open-network-stream)
  (fboundp 'gnutls-available-p)
  (fboundp 'open-gnutls-stream)
  (featurep 'gnutls)) "#,
    );
}

#[test]
fn divergence_tls_vars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (boundp 'gnutls-trustfiles)
  (listp gnutls-trustfiles)
  (boundp 'gnutls-verify-error)
  (boundp 'gnutls-min-prime-bits)
  (integerp gnutls-min-prime-bits)) "#,
    );
}

#[test]
fn divergence_url_vars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (boundp 'url-configuration-directory)
  (boundp 'url-cookie-file)
  (boundp 'url-history-file)
  (fboundp 'url-insert-file-contents)) "#,
    );
}

#[test]
fn divergence_dns_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'dns-query)
  (fboundp 'dns-lookup-host)
  (fboundp 'network-lookup-address-info)
  (fboundp 'lookup-host)))) "#,
    );
}

#[test]
fn divergence_http_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'url-http-file-exists-p)
  (fboundp 'url-file-exists-p)
  (fboundp 'url-file-directory-p)
  (fboundp 'url-expand-file-name)) "#,
    );
}

#[test]
fn divergence_ldap_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'ldap-open)
  (fboundp 'ldap-close)
  (fboundp 'ldap-search)
  (featurep 'ldap)) "#,
    );
}

#[test]
fn divergence_mime_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'mime-edit)
  (fboundp 'mailcap-parse-mailcaps)
  (fboundp 'mailcap-mime-info)
  (featurep 'mailcap)) "#,
    );
}

#[test]
fn divergence_mail_utils() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'mail-strip-quoted-names)
  (fboundp 'rfc822-addresses)
  (fboundp 'mail-header-parse-address)
  (featurep 'mail-utils)) "#,
    );
}

#[test]
fn divergence_news_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'gnus)
  (fboundp 'gnus-group-read-news)
  (fboundp 'gnus-msg-mail)
  (featurep 'gnus)) "#,
    );
}

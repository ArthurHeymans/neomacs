//! Divergence tests: auth-source, gnus, message, mail deep stubs.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_auth_source() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'auth-source-search)
  (fboundp 'auth-source-forget)
  (featurep 'auth-source))"#,
    );
}

#[test]
fn divergence_gnus_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'gnus)
  (fboundp 'gnus-group-list)
  (featurep 'gnus)
  (featurep 'gnus-group))"#,
    );
}

#[test]
fn divergence_message_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'message-mail)
  (fboundp 'message-reply)
  (featurep 'message))"#,
    );
}

#[test]
fn divergence_sendmail() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'sendmail-send-it)
  (boundp 'send-mail-function)
  (featurep 'sendmail))"#,
    );
}

#[test]
fn divergence_smtpmail() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'smtpmail-send-it)
  (featurep 'smtpmail))"#,
    );
}

#[test]
fn divergence_epa_gpg() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'epa-encrypt-file)
  (fboundp 'epa-decrypt-file)
  (fboundp 'epa-sign-file)
  (featurep 'epa)
  (featurep 'epg))"#,
    );
}

#[test]
fn divergence_erc() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'erc)
  (fboundp 'erc-select)
  (featurep 'erc))"#,
    );
}

#[test]
fn divergence_rcirc() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'rcirc)
  (featurep 'rcirc))"#,
    );
}

#[test]
fn divergence_eww_shr() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'eww)
  (fboundp 'eww-open-file)
  (featurep 'eww)
  (featurep 'shr))"#,
    );
}

#[test]
fn divergence_url_library() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'url-retrieve)
  (fboundp 'url-retrieve-synchronously)
  (featurep 'url))"#,
    );
}

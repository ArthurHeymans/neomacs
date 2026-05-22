//! Divergence tests: shr, eww, url, xml-rpc, network stubs.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_shr_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'shr-render-region)
  (featurep 'shr)
  (fboundp 'libxml-parse-html-region))"#,
    );
}

#[test]
fn divergence_eww_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'eww)
  (fboundp 'eww-browse-url)
  (featurep 'eww))"#,
    );
}

#[test]
fn divergence_url_encode() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(require 'url-util)
(list
  (url-hexify-string "hello world")
  (url-hexify-string "a=b&c=d")
  (string= (url-unhex-string "hello%20world") "hello world"))"#,
    );
}

#[test]
fn divergence_url_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(require 'url-parse)
(let ((u (url-generic-parse-url "https://example.com/path?q=1#frag")))
  (list (url-type u)
        (url-host u)
        (url-filename u)
        (url-target u)))"#,
    );
}

#[test]
fn divergence_dom_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(require 'dom)
(let ((tree '(html nil (body nil (p nil "hello")))))
  (list (dom-tag tree)
        (dom-children tree)
        (dom-text tree)
        (dom-by-tag tree 'body)))"#,
    );
}

#[test]
fn divergence_svg_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'svg-create)
  (fboundp 'svg-rectangle)
  (fboundp 'svg-circle)
  (featurep 'svg))"#,
    );
}

#[test]
fn divergence_mail_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'mail-parse)
  (fboundp 'rfc822-addresses)
  (fboundp 'mail-header-parse-address)
  (featurep 'mail-parse))"#,
    );
}

#[test]
fn divergence_message_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'message-mode)
  (featurep 'message)
  (fboundp 'message-make-from))"#,
    );
}

#[test]
fn divergence_sendmail() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'sendmail-send-it)
  (fboundp 'mail-send)
  (boundp 'send-mail-function))"#,
    );
}

#[test]
fn divergence_mml_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'mml-generate-mime)
  (featurep 'mml)
  (fboundp 'mml-insert-multipart))"#,
    );
}

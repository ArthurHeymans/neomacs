//! Strict combo oracle probes, batch 71: face remapping (add/remove-relative),
//! text-scale-set, add-to-history, mailcap (extension->MIME mapping), and
//! syntax-pp (syntax cache lookup).
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::assert_oracle_parity_with_load;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_o5_face_remap_add_remove_relative() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "hello")
  (let ((cookie (face-remap-add-relative 'default :height 2.0)))
    (list (consp cookie)
          (face-remap-remove-relative cookie))))
"##,
    );
}

#[test]
fn div_o5_text_scale_set() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (text-scale-set 2)
  (list text-scale-mode-amount
        (boundp 'text-scale-mode)
        (integerp text-scale-mode-amount)))
"##,
    );
}

#[test]
fn div_o5_add_to_history() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((probe-history nil))
  (add-to-history 'probe-history "foo")
  (add-to-history 'probe-history "bar")
  (add-to-history 'probe-history "foo")
  (add-to-history 'probe-history "baz")
  probe-history)
"##,
    );
}

#[test]
fn div_o5_mailcap_extension_to_mime() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(list (mailcap-extension-to-mime "txt")
      (mailcap-extension-to-mime "html")
      (mailcap-extension-to-mime "json"))
"##,
        &["net/mailcap.el"],
    );
}

#[test]
fn div_o5_syntax_pp_cache() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(defun foo ()\n  body)\n")
  (goto-char 1)
  (list (nth 0 (syntax-pp 1))
        (nth 0 (syntax-pp 10))
        (nth 0 (syntax-pp 15))))
"##,
    );
}

//! Divergence tests: mode-line, header-line, mode hooks deep.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_mode_line_vars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'mode-line-format)
  (listp mode-line-format)
  (boundp 'mode-line-modified)
  (listp mode-line-modified)
  (boundp 'mode-line-buffer-identification)
  (listp mode-line-buffer-identification)) "#,
    );
}

#[test]
fn divergence_header_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'header-line-format)
  (boundp 'mode-line-front-space)
  (boundp 'mode-line-mule-info)
  (boundp 'mode-line-client)
  (boundp 'mode-line-remote)
  (boundp 'mode-line-frame-identification)) "#,
    );
}

#[test]
fn divergence_mode_line_position() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'mode-line-position)
  (listp mode-line-position)
  (fboundp 'line-number-at-pos)
  (integerp (line-number-at-pos))) "#,
    );
}

#[test]
fn divergence_mode_line_modes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'mode-line-modes)
  (listp mode-line-modes)
  (boundp 'mode-name)
  (stringp mode-name)
  (boundp 'major-mode)
  (symbolp major-mode)) "#,
    );
}

#[test]
fn divergence_mode_hooks() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'after-change-major-mode-hook)
  (boundp 'change-major-mode-hook)
  (listp after-change-major-mode-hook)
  (listp change-major-mode-hook)
  (fboundp 'run-mode-hooks)) "#,
    );
}

#[test]
fn divergence_delayed_mode_hooks() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'delay-mode-hooks)
  (booleanp delay-mode-hooks)
  (fboundp 'delay-mode-hooks-update)) "#,
    );
}

#[test]
fn divergence_global_mode_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'global-mode-string)
  (listp global-mode-string)
  (fboundp 'format-mode-line)
  (stringp (format-mode-line mode-line-format))) "#,
    );
}

#[test]
fn divergence_minor_mode_alist() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'minor-mode-alist)
  (listp minor-mode-alist)
  (boundp 'minor-mode-overriding-map-alist)
  (listp minor-mode-overriding-map-alist)) "#,
    );
}

#[test]
fn divergence_special_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'special-mode)
  (fboundp 'fundamental-mode)
  (fboundp 'text-mode)
  (fboundp 'prog-mode)
  (featurep 'prog-mode)) "#,
    );
}

#[test]
fn derivation_mode_derive() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'derived-mode-p)
  (fboundp 'provided-mode-derived-p)
  (fboundp 'set-buffer-major-mode)) "#,
    );
}

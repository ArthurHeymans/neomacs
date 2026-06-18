/// Batch 467: final deep edge probes - fill-column, auto-fill, dabbrev, hippie.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx467_fill_column_auto_fill() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (text-mode)
  (setq fill-column 30)
  (auto-fill-mode 1)
  (insert "This is a long line that should auto-fill at column 30 for testing")
  (buffer-string))"##,
    );
}

#[test]
fn div_cx467_dabbrev_expand() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'dabbrev)
  (list (boundp 'dabbrev-case-fold-search)
        (fboundp 'dabbrev-expand)))"##,
    );
}

#[test]
fn div_cx467_hippie_expand() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'hippie-exp)
  (list (boundp 'hippie-expand-try-functions-list)
        (fboundp 'he-substitute-string)))"##,
    );
}

#[test]
fn div_cx467_isearch_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'isearch)
  (list (boundp 'isearch-mode-map)
        (fboundp 'isearch-forward)
        (fboundp 'isearch-backward)))"##,
    );
}

#[test]
fn div_cx467_query_replace_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'replace)
  (list (boundp 'query-replace-map)
        (fboundp 'query-replace)
        (fboundp 'query-replace-regexp)))"##,
    );
}

#[test]
fn div_cx467_shell_completion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'shell)
  (list (boundp 'shell-completion-fignore)
        (fboundp 'shell-dynamic-complete-command)))"##,
    );
}

#[test]
fn div_cx467_completion_at_point() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'completion)
  (list (boundp 'completion-at-point-functions)
        (fboundp 'completion-at-point)))"##,
    );
}

#[test]
fn div_cx467_minibuffer_history() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (boundp 'minibuffer-history)
      (boundp 'minibuffer-history-variable)
      (listp minibuffer-history))"##,
    );
}

#[test]
fn div_cx467_face_spec_custom() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (face-spec-set 'bold '((t (:weight bold))) nil)
      (face-attribute 'bold :weight nil 'default))"##,
    );
}

#[test]
fn div_cx467_read_buffer_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(fboundp 'read-buffer)"##);
}

#[test]
fn div_cx467_image_type_avail() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (image-type-available-p 'png)
      (image-type-available-p 'jpeg)
      (image-type-available-p 'xpm))"##,
    );
}

#[test]
fn div_cx467_doc_view_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'doc-view)
  (list (fboundp 'doc-view-mode)
        (fboundp 'doc-view-toggle-display)))"##,
    );
}

#[test]
fn div_cx467_print_help_message() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'help-mode)
  (condition-case e
      (with-temp-buffer
        (help-mode)
        (print-help-return-message))
    (error (car e))))"##,
    );
}

#[test]
fn div_cx467_display_battery_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'battery)
  (list (boundp 'display-battery-mode)
        (fboundp 'battery-status-function)))"##,
    );
}

#[test]
fn div_cx467_abbrev_inverse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'abbrev)
  (list (fboundp 'inverse-add-mode-abbrev)
        (fboundp 'inverse-add-global-abbrev)))"##,
    );
}

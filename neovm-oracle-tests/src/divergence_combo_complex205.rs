//! Complex combo batch 205 — `describe-*` / `finder-*` / `info-*` /
//! `man-*` / `help-mode` availability and metadata queries.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx205_describe_functions_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (fboundp 'describe-function)
      (fboundp 'describe-variable)
      (fboundp 'describe-symbol)
      (fboundp 'describe-key)
      (fboundp 'describe-key-briefly)
      (fboundp 'describe-bindings)
      (fboundp 'describe-mode)
      (fboundp 'describe-face)
      (fboundp 'describe-char)
      (fboundp 'describe-package)
      (fboundp 'describe-syntax)
      (fboundp 'describe-category))
"##,
    );
}

#[test]
fn div_cx205_finder_inf_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'finder-inf)
      (list (boundp 'package--builtins)
            (consp package--builtins)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx205_help_mode_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'help-mode)
      (list (fboundp 'help-mode)
            (boundp 'help-xref-stack)
            (boundp 'help-xref-stack-item)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx205_info_node_navigation_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'info)
      (list (fboundp 'info)
            (fboundp 'Info-forward-node)
            (fboundp 'Info-backward-node)
            (fboundp 'Info-index)
            (fboundp 'Info-search)
            (boundp 'Info-directory-list)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx205_man_mode_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'man)
      (list (fboundp 'man)
            (fboundp 'Man-getpage-in-background)
            (boundp 'Man-notify-method)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx205_help_buffer_predicate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((buf (help-buffer)))
      (list (or (null buf) (bufferp buf))
            (or (null buf) (buffer-live-p buf))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx205_apropos_internal_search() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((results (apropos-internal "buffer")))
      (list (consp results)
            (> (length results) 0)
            (memq 'buffer results)
            (memq 'buffer-name results)
            (memq 'get-buffer results)
            (memq 'set-buffer results)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx205_help_symbol_completion_via_obarray() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((completions (all-completions "with-" obarray)))
  (list (consp completions)
        (> (length completions) 0)
        (member "with-current-buffer" completions)
        (member "with-temp-buffer" completions)
        (member "with-output-to-temp-buffer" completions)))
"##,
    );
}

#[test]
fn div_cx205_describe_syntax_table_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (fboundp 'describe-syntax)
      (boundp 'syntax-code-table)
      (boundp 'syntax-mnemonic-table))
"##,
    );
}

#[test]
fn div_cx205_help_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((apropos-result (apropos-internal "overlay")))
  (with-temp-buffer
    (buffer-enable-undo)
    (insert (format "Found %d overlay symbols" (length apropos-result)))
    (put-text-property 1 6 'face 'bold)
    (let ((m (set-marker (make-marker) 10))
          (ov (make-overlay 4 18)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 2 25)
      (let ((state (list (length apropos-result)
                         (memq 'make-overlay apropos-result)
                         (memq 'overlay-start apropos-result)
                         (memq 'delete-overlay apropos-result)
                         (buffer-string)
                         (marker-position m)
                         (overlay-start ov) (overlay-end ov)
                         (text-properties-at 1))))
        (undo)
        (widen)
        (list state (buffer-string) (marker-position m)
              (overlay-start ov) (overlay-end ov)
              (text-properties-at 1))))))
"##,
    );
}

//! subword-mode forward-word (find-word-boundary-function-table), output
//! functions (with-output-to-string, princ/prin1/terpri/print to a buffer,
//! prin1 to a function stream, pp-to-string), and format-message quoting
//! (grave/curve); plus the batch current-message divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn sw_find_word_boundary_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "helloWorld test")
  (goto-char (point-min))
  (list (progn (forward-word) (point)) (boundp 'find-word-boundary-function-table)))"##,
    );
}

#[test]
fn sw_format_message_curve() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((text-quoting-style 'curve))
  (format-message "type `C-x C-c' to quit"))"##,
    );
}

#[test]
fn sw_format_message_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((text-quoting-style 'grave))
  (list (format-message "use `foo'") (substitute-command-keys "\\`a\\'")))"##,
    );
}

#[test]
fn sw_output_to_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (with-output-to-string (princ "hello") (princ " ") (princ 42))
        (with-output-to-string (prin1 '(1 2 3))))"##,
    );
}

#[test]
fn sw_pp_to_string_forms() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (string-trim (pp-to-string '(a (b c) d)))
        (string-trim (pp-to-string [1 2 3]))
        (string-trim (pp-to-string 42)))"##,
    );
}

#[test]
fn sw_prin1_to_string_stream() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((acc nil))
  (prin1 'symbol (lambda (c) (push c acc)))
  (concat (nreverse acc)))"##,
    );
}

#[test]
fn sw_princ_prin1_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (princ "abc" (current-buffer))
  (prin1 '(x y) (current-buffer))
  (terpri (current-buffer))
  (buffer-string))"##,
    );
}

#[test]
fn sw_print_to_buffer_point() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "XY")
  (goto-char 2)
  (print "inserted" (current-buffer))
  (buffer-string))"##,
    );
}

#[test]
fn sw_subword_forward() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(condition-case e (progn (require 'subword)
  (with-temp-buffer
    (insert "fooBarBaz quxQux")
    (subword-mode 1)
    (goto-char (point-min))
    (list (progn (forward-word) (point)) (progn (forward-word) (point)) (progn (forward-word) (point))))) (error (cons (quote ERR) (car e))))"##,
    );
}

#[test]
#[ignore = "DIVERGENCE: in --batch mode (current-message) returns nil in GNU (the echo area is not maintained in batch) but neomacs returns the text of the last `message`."]
fn divergence_current_message_batch() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (message "hello %d" 42)
       (list (current-message) (booleanp (current-message))))"##,
    );
}

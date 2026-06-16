//! Complex combo batch 151 — `print-circle` / `print-gensym` /
//! `print-continuous-numbering` / `float-format` and other print engine
//! vars with edge cases.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx151_print_circle_with_complex_structure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((a (list 1 2 3))
      (b (vector :v1 :v2)))
  (let ((data (list a b a b (cons a b))))
    (list (let ((print-circle t)) (prin1-to-string data))
          (let ((print-circle nil))
            (condition-case e (prin1-to-string data) (error (car e)))))))
"##,
    );
}

#[test]
fn div_cx151_print_gensym_round_trip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((gs1 (gensym "G-"))
      (gs2 (gensym "G-")))
  (list (symbol-name gs1)
        (symbol-name gs2)
        (eq gs1 gs2)
        (let ((print-gensym t)) (prin1-to-string gs1))
        (let ((print-gensym t)) (prin1-to-string gs2))))
"##,
    );
}

#[test]
fn div_cx151_print_escape_control_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((s (string ?\a ?\b ?\t ?\n ?\v ?\f ?\r ?\e ?\x7f)))
  (list (prin1-to-string s)
        (princ-to-string s)
        (let ((print-escape-control-characters t)) (prin1-to-string s))))
"##,
    );
}

#[test]
fn div_cx151_float_format_variations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((float-pi 3.141592653589793))
  (list (let ((float-output-format nil)) (prin1-to-string float-pi))
        (let ((float-output-format "%f")) (prin1-to-string float-pi))
        (let ((float-output-format "%.3f")) (prin1-to-string float-pi))
        (let ((float-output-format "%e")) (prin1-to-string float-pi))
        (let ((float-output-format "%g")) (prin1-to-string float-pi))))
"##,
    );
}

#[test]
fn div_cx151_print_length_level_combined() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((deep '(((("deep")))))
      (long (number-sequence 1 100)))
  (list (let ((print-length 3) (print-level 2))
          (prin1-to-string (list deep long)))
        (let ((print-length 0) (print-level 0))
          (prin1-to-string (list deep long)))))
"##,
    );
}

#[test]
fn div_cx151_print_quoted_emits_quote_form() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((data '(alpha (beta (gamma delta)))))
  (list (let ((print-quoted t)) (prin1-to-string data))
        (let ((print-quoted nil)) (prin1-to-string data))))
"##,
    );
}

#[test]
fn div_cx151_prin1_to_string_vs_princ_to_string_with_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((s "with \"quotes\" and \\ backslash"))
  (list (prin1-to-string s)
        (princ-to-string s)
        (length (prin1-to-string s))
        (length (princ-to-string s))))
"##,
    );
}

#[test]
fn div_cx151_pp_to_string_indentation_with_deep_struct() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((data '(:config
              (:option-a "value"
               :option-b ((:nested-key . nested-val)
                          (:other-key . other-val)))
              :list-of-things (item1 item2 (nested1 nested2)
                                item3))))
  (let ((pp-str (pp-to-string data))
        (p1-str (prin1-to-string data)))
    (list (> (length pp-str) (length p1-str))
          (> (length (split-string pp-str "\n")) 3)
          (car (split-string pp-str "\n")))))
"##,
    );
}

#[test]
fn div_cx151_print_charset_qualified_interned() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((syms '(alpha β γ δ ε 中文 日本語)))
  (list (mapcar #'prin1-to-string syms)
        (mapcar #'princ-to-string syms)
        (length (mapcar #'prin1-to-string syms))))
"##,
    );
}

#[test]
fn div_cx151_print_continuous_numbering() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((print-circle t)
          (print-continuous-numbering t)
          (print-number-table nil))
      (let ((shared (list 1 2 3)))
        (let ((printed1 (prin1-to-string (list shared shared)))
              (printed2 (prin1-to-string (list shared shared))))
          (list printed1 printed2))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx151_print_with_text_properties_via_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((s (propertize "hello" 'face 'bold)))
  (list (format "%S" s)
        (format "%s" s)
        (text-properties-at 0 (format "%S" s))
        (text-properties-at 0 s)))
"##,
    );
}

#[test]
fn div_cx151_print_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((shared (list 1 2 3))
       (data (list shared shared (list :a :b)))
       (printed (let ((print-circle t)) (prin1-to-string data))))
  (with-temp-buffer
    (buffer-enable-undo)
    (insert printed)
    (put-text-property 1 6 'face 'bold)
    (let ((m (set-marker (make-marker) 8))
          (ov (make-overlay 4 14)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 2 18)
      (let ((state (list printed
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

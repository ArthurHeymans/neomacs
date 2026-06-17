//! Complex combo batch 325 — `print`/`read` engine ultimate: print-circle,
//! print-gensym, print-length, print-level, print-quoted, print-escape
//! variants with deeply shared/circular structures and uninterned symbols.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx325_print_circle_deeply_shared() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((inner (list 1 2 3))
       (data (list inner inner inner)))
  (list (let ((print-circle t)) (prin1-to-string data))
        (let ((print-circle nil))
          (condition-case e (prin1-to-string data) (error (car e))))))
"##,
    )
}

#[test]
fn div_cx325_print_circle_circular_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((circular (list 1 2 3)))
  (setcdr (cddr circular) circular)
  (list (let ((print-circle t)) (prin1-to-string circular))
        (let ((print-circle nil))
          (condition-case e (prin1-to-string circular) (error (car e))))))
"##,
    )
}

#[test]
fn div_cx325_print_gensym_uninterned_in_shared() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((gs (gensym "G-")))
  (list (symbol-name gs)
        (let ((print-gensym t)) (prin1-to-string gs))
        (let ((print-gensym nil)) (prin1-to-string gs))
        (let ((print-gensym t) (print-circle t))
          (prin1-to-string (list gs gs)))))
"##,
    )
}

#[test]
fn div_cx325_print_length_and_level_combined() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((deep '(((("deep")))))
      (long (number-sequence 1 50)))
  (list (let ((print-length 3) (print-level 2))
          (prin1-to-string (list deep long)))
        (let ((print-length 0) (print-level 0))
          (prin1-to-string (list deep long)))
        (let ((print-length nil) (print-level nil))
          (prin1-to-string (list deep long)))))
"##,
    )
}

#[test]
fn div_cx325_print_quoted_emits_quote_form() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((data '(alpha (beta (gamma delta)))))
  (list (let ((print-quoted t)) (prin1-to-string data))
        (let ((print-quoted nil)) (prin1-to-string data))))
"##,
    )
}

#[test]
fn div_cx325_print_escape_nonascii_and_control() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((s (decode-coding-string (unibyte-string #xff #xfe) 'utf-8-unix t)))
  (list (prin1-to-string s)
        (let ((print-escape-nonascii t)) (prin1-to-string s))
        (let ((print-escape-multibyte t)) (prin1-to-string s))))
"##,
    )
}

#[test]
fn div_cx325_read_circle_shared_round_trip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((shared (list 1 2 3))
       (data (list shared shared))
       (printed (let ((print-circle t)) (prin1-to-string data)))
       (read-with (let ((read-circle t)) (read-from-string printed)))
       (read-without (let ((read-circle nil))
                       (condition-case e (read-from-string printed)
                         (error (cons :err (car e)))))))
  (list printed
        (car read-with)
        (eq (car (car read-with)) (cadr (car read-with)))
        read-without))
"##,
    )
}

#[test]
fn div_cx325_prin1_vs_princ_with_strings_and_structures() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((s "with \"quotes\" and \\ backslash")
      (lst '(1 "two" (3 4))))
  (list (prin1-to-string s)
        (princ-to-string s)
        (prin1-to-string lst)
        (princ-to-string lst)
        (length (prin1-to-string s))
        (length (princ-to-string s))))
"##,
    )
}

#[test]
fn div_cx325_pp_to_string_with_deep_indent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((data '(:config
              (:option-a "value"
               :option-b (:nested-a 1
                          :nested-b 2))
              :option-c (1 2 3))))
  (let ((pp-str (pp-to-string data))
        (p1-str (prin1-to-string data)))
    (list (> (length pp-str) (length p1-str))
          (> (length (split-string pp-str "\n")) 3)
          (car (split-string pp-str "\n")))))
"##,
    )
}

#[test]
fn div_cx325_print_read_with_marker_overlay_undo_narrow_mega() {
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
                         (eq (car (car data)) (cadr data))
                         (buffer-string)
                         (marker-position m)
                         (overlay-start ov) (overlay-end ov)
                         (text-properties-at 1))))
        (undo)
        (widen()
        (list state (buffer-string) (marker-position m)
              (overlay-start ov) (overlay-end ov)
              (text-properties-at 1)))))))
"##,
    )
}

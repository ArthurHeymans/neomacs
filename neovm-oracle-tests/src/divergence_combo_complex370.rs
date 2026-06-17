//! Complex combo batch 370 — `read`/`print` engine ultimate: reader macros
//! (#. eval, #_ skip, #s record, #N= #N# shared/circular, #[...] bytecode),
//! print-circle/print-gensym/print-length/print-level combinations.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx370_read_reader_macros_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (car (read-from-string "#.(+ 1 2)")) (error (cons :err (car e))))
      (condition-case e (car (read-from-string "#.(* 6 7)")) (error (cons :err (car e))))
      (condition-case e (car (read-from-string "#_skipped actual-value")) (error (cons :err (car e))))
      (condition-case e (car (read-from-string "#1=(a . b) #1#")) (error (cons :err (car e)))))
"##,
    )
}

#[test]
fn div_cx370_print_circle_deeply_shared_and_circular() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((inner (list 1 2 3))
       (shared (list inner inner inner)))
  (list (let ((print-circle t)) (prin1-to-string shared))
        (let ((print-circle nil))
          (condition-case e (prin1-to-string shared) (error (car e))))))
"##,
    )
}

#[test]
fn div_cx370_print_circle_circular_list_print_round_trip() {
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
fn div_cx370_print_gensym_uninterned_in_shared() {
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
fn div_cx370_print_length_and_level_combined() {
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
fn div_cx370_read_circle_shared_round_trip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((shared (list 1 2 3))
       (data (list shared shared))
       (printed (let ((print-circle t)) (prin1-to-string data)))
       (read-with (let ((read-circle t)) (read-from-string printed))))
  (list printed
        (car read-with)
        (eq (car (car read-with)) (cadr (car read-with)))))
"##,
    )
}

#[test]
fn div_cx370_prin1_vs_princ_with_strings_and_structures() {
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
fn div_cx370_pp_to_string_with_deep_indent() {
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
fn div_cx370_read_special_syntaxes_matrix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (s)
          (condition-case e
              (let ((v (car (read-from-string s))))
                (list s v (type-of v)))
            (error (list s :err (car e)))))
        '("[1 2 3]"
          "#(1 2 3)"
          "#s(record a b c)"
          "?A"
          "#x10"
          "#o17"
          "#b1010"
          "1.5"
          "1/2"
          "1000000000000000000000"))
"##,
    )
}

#[test]
fn div_cx370_print_read_with_marker_overlay_undo_narrow_mega() {
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

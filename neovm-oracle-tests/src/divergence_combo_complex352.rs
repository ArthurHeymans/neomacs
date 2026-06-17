//! Complex combo batch 352 — `ring` data structure, `system-process-
//! attributes`, `process-attributes`, `float-arithmetic` edge cases.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx352_ring_create_insert_remove_ref() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((r (make-ring 5)))
  (ring-insert r :a)
  (ring-insert r :b)
  (ring-insert r :c)
  (list (ring-length r) (ring-ref r 0) (ring-ref r 1) (ring-ref r 2)
        (ring-remove r) (ring-length r)))
"##,
    )
}

#[test]
fn div_cx352_ring_insert_at_remove_at() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((r (make-ring 5)))
  (ring-insert r :a)
  (ring-insert r :b)
  (ring-insert r :c)
  (ring-insert-at r :x 1)
  (list (ring-length r) (ring-ref r 0) (ring-ref r 1) (ring-ref r 2) (ring-ref r 3)
        (ring-remove-at r 1) (ring-ref r 1)))
"##,
    )
}

#[test]
fn div_cx352_ring_overflow_drops_oldest() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((r (make-ring 3)))
  (ring-insert r :a)
  (ring-insert r :b)
  (ring-insert r :c)
  (ring-insert r :d)
  (list (ring-length r) (ring-ref r 0) (ring-ref r 1) (ring-ref r 2)))
"##,
    )
}

#[test]
fn div_cx352_ring_copy_and_equal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((r1 (make-ring 5)))
  (ring-insert r1 :a)
  (ring-insert r1 :b)
  (let ((r2 (copy-ring r1)))
    (ring-insert r2 :c)
    (list (ring-length r1) (ring-length r2)
          (ring-ref r1 1) (ring-ref r2 1)
          (ring-ref r2 2))))
"##,
    )
}

#[test]
fn div_cx352_system_process_attributes_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (fboundp 'system-process-attributes)
      (fboundp 'process-attributes)
      (fboundp 'list-system-processes)
      (boundp 'system-uses-terminfo))
"##,
    )
}

#[test]
fn div_cx352_list_system_processes_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((procs (list-system-processes)))
      (list (consp procs)
            (> (length procs) 0)
            (integerp (car procs))))
  (error (list :errored (car e))))
"##,
    )
}

#[test]
fn div_cx352_float_denormal_arithmetic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (* 1.0 1e-308)
      (* 1.0 1e-300)
      (* 1.0 5e-324)
      (+ 0.0 0.0)
      (- 0.0 0.0)
      (* 0.0 -1.0)
      (+ 0.5 -0.5)
      (* 1e308 10.0))
"##,
    )
}

#[test]
fn div_cx352_float_formatting_precision_extreme() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (format "%.20f" 3.14159265358979323846)
      (format "%.0f" 0.5)
      (format "%.0f" 1.5)
      (format "%.0f" 2.5)
      (format "%.15e" 1.0e-300)
      (format "%.15g" 1.0e10)
      (format "%.15g" 0.0000001))
"##,
    )
}

#[test]
fn div_cx352_ring_elements_to_list_and_back() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((r (make-ring 10)))
  (dolist (x '(:a :b :c :d :e))
    (ring-insert r x))
  (let ((as-list (ring-elements r)))
    (list as-list (ring-length r)
          (ring-p r) (ring-size r))))
"##,
    )
}

#[test]
fn div_cx352_ring_system_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((r (make-ring 5)))
  (ring-insert r :a)
  (ring-insert r :b)
  (ring-insert r :c)
  (with-temp-buffer
    (buffer-enable-undo)
    (insert (format "Ring/system mega: %S" (ring-elements r)))
    (put-text-property 1 6 'face 'bold)
    (let ((m (set-marker (make-marker) 10))
          (ov (make-overlay 4 18)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 2 25)
      (let ((state (list (ring-elements r) (ring-length r)
                         (fboundp 'list-system-processes)
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

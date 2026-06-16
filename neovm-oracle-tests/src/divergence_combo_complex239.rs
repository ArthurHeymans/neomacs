//! Complex combo batch 239 — `garbage-collect` / `memory-use-counts` /
//! `memory-limit` / `purecopy` / `float-pairs` / `gc-cons-threshold`
//! interaction with weak-hash and buffer-local state.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx239_garbage_collect_return_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((gc-result (garbage-collect)))
  (list (consp gc-result)
        (> (length gc-result) 0)
        (assq 'conses gc-result)
        (assq 'symbols gc-result)
        (assq 'strings gc-result)))
"##,
    );
}

#[test]
fn div_cx239_memory_use_counts() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((counts (memory-use-counts)))
  (list (consp counts)
        (> (length counts) 5)
        (integerp (nth 0 counts))
        (integerp (nth 1 counts))))
"##,
    );
}

#[test]
fn div_cx239_memory_limit_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (fboundp 'memory-limit)
      (integerp (memory-limit))
      (> (memory-limit) 0))
"##,
    );
}

#[test]
fn div_cx239_gc_cons_threshold_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (boundp 'gc-cons-threshold)
      (integerp gc-cons-threshold)
      (> gc-cons-threshold 0)
      (boundp 'gc-cons-percentage)
      (floatp gc-cons-percentage))
"##,
    );
}

#[test]
fn div_cx239_weak_hash_after_gc_consistency() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ht (make-hash-table :weakness 'key :test 'eq)))
  (dotimes (i 10) (puthash (cons i nil) (* i 10) ht))
  (let ((before (hash-table-count ht)))
    (garbage-collect)
    (let ((after (hash-table-count ht)))
      (list before after (<= after before)))))
"##,
    );
}

#[test]
fn div_cx239_purecopy_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (fboundp 'purecopy)
      (boundp 'purify-flag))
"##,
    );
}

#[test]
fn div_cx239_gc_elapsed_time_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (boundp 'gc-elapsed)
      (floatp gc-elapsed)
      (boundp 'gcs-done)
      (integerp gcs-done))
"##,
    );
}

#[test]
fn div_cx239_buffer_resources_after_create_kill() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((before (garbage-collect)))
  (dotimes (i 20)
    (let ((buf (get-buffer-create (format " *neo-cx239-tmp-%d*" i))))
      (with-current-buffer buf
        (insert (make-string 1000 ?x)))))
  (garbage-collect)
  (let ((after-create (garbage-collect)))
    (dolist (i (number-sequence 0 19))
      (kill-buffer (get-buffer (format " *neo-cx239-tmp-%d*" i))))
    (garbage-collect)
    (let ((after-kill (garbage-collect)))
      (list (consp before)
            (consp after-create)
            (consp after-kill)))))
"##,
    );
}

#[test]
fn div_cx239_float_pairs_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (fboundp 'make-float-pairs)
      (fboundp 'float-pairs-p)
      (boundp 'float-pairs))
"##,
    );
}

#[test]
fn div_cx239_gc_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((gc-before (garbage-collect))
      (weak-ht (make-hash-table :weakness 'key :test 'eq)))
  (puthash (cons 1 nil) :v weak-ht)
  (garbage-collect)
  (with-temp-buffer
    (buffer-enable-undo)
    (insert "GC mega test buffer content")
    (put-text-property 1 5 'face 'bold)
    (let ((m (set-marker (make-marker) 8))
          (ov (make-overlay 4 14)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 2 18)
      (let ((gc-after (garbage-collect)))
        (let ((state (list (consp gc-before)
                           (consp gc-after)
                           (hash-table-count weak-ht)
                           (memory-limit)
                           (buffer-string)
                           (marker-position m)
                           (overlay-start ov) (overlay-end ov)
                           (text-properties-at 1))))
          (undo)
          (widen)
          (list state (buffer-string) (marker-position m)
                (overlay-start ov) (overlay-end ov)
                (text-properties-at 1)))))))
"##,
    );
}

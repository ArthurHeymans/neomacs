//! Divergence tests: stress tests - many operations combined, edge case combos.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_stress_many_inserts_deletes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (setq buffer-undo-list nil)
  (dotimes (i 100)
    (goto-char (point-max))
    (insert (format "line%d\n" i)))
  (let ((len (point-max)))
    (goto-char 1)
    (dotimes (_ 50)
      (forward-line 1)
      (kill-line))
    (list len (point-max) (buffer-size))))"#,
    );
}

#[test]
fn divergence_stress_nested_save_excursion() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "ABCDEFGHIJ")
  (save-excursion
    (goto-char 5)
    (save-excursion
      (goto-char 8)
      (save-excursion
        (goto-char 3))))
    (list (point)))
  (list (point)))"#,
    );
}

#[test]
fn divergence_stress_many_overlays() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert (make-string 100 ?X))
  (let ((ovs nil))
    (dotimes (i 10)
      (push (make-overlay (1+ (* i 10)) (+ 5 (* i 10))) ovs))
    (list (length ovs)
          (length (overlays-in 1 100))
          (overlayp (car ovs)))))"#,
    );
}

#[test]
fn divergence_stress_many_text_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert (make-string 100 ?X))
  (dotimes (i 10)
    (put-text-property (1+ (* i 10)) (+ 5 (* i 10))
                       'face (list 'bold 'italic 'underline)))
  (list (get-text-property 1 'face)
        (get-text-property 11 'face)
        (get-text-property 6 'face)))"#,
    );
}

#[test]
fn divergence_stress_many_narrows() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "ABCDEFGHIJ")
  (save-restriction
    (narrow-to-region 2 9)
    (save-restriction
      (narrow-to-region 3 8)
      (list (point-min) (point-max) (buffer-string)))
    (list (point-min) (point-max) (buffer-string)))
  (list (point-min) (point-max) (buffer-string)))"#,
    );
}

#[test]
fn divergence_stress_many_markers() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "ABCDEFGHIJ")
  (let ((markers (mapcar (lambda (i) (set-marker (make-marker) i))
                         (number-sequence 1 10))))
    (goto-char 3)
    (insert "XXX")
    (mapcar #'marker-position markers)))"#,
    );
}

#[test]
fn divergence_stress_condition_case_loop() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((result nil))
  (dotimes (i 5)
    (condition-case err
        (if (= i 3) (error "boom at %d" i) (push i result))
      (error (push (cons 'caught err) result))))
  result)"#,
    );
}

#[test]
fn divergence_stress_undo_redo_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (setq buffer-undo-list nil)
  (insert "one")
  (undo-boundary)
  (insert "two")
  (undo-boundary)
  (insert "three")
  (let ((full (buffer-string)))
    (undo)
    (undo)
    (let ((after2 (buffer-string)))
      (undo)
      (list full after2 (buffer-string)))))"#,
    );
}

#[test]
fn divergence_stress_hash_many_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((ht (make-hash-table :test 'eql)))
  (dotimes (i 100)
    (puthash i (* i i) ht))
  (list (hash-table-count ht)
        (gethash 10 ht)
        (gethash 50 ht)
        (gethash 99 ht)
        (gethash 100 ht 'missing)))"#,
    );
}

#[test]
fn divergence_stress_list_manipulation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((lst (number-sequence 1 50)))
  (setq lst (mapcar #'1+ lst))
  (setq lst (delq 2 (remq 51 lst)))
  (list (length lst)
        (car lst)
        (car (last lst))
        (nth 10 lst)
        (member 30 lst)))"#,
    );
}

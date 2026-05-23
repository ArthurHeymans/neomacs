//! Divergence tests: property-based combinatorial - random-ish operations.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_combo_insert_narrow_undo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (setq buffer-undo-list nil)
  (insert "ABCDEFGHIJ")
  (narrow-to-region 3 8)
  (goto-char 4)
  (insert "XXX")
  (undo-boundary)
  (delete-region 5 7)
  (list (buffer-string) (point-min) (point-max))
  (undo)
  (list (buffer-string))
  (widen)
  (buffer-string))"#,
    );
}

#[test]
fn divergence_combo_overlay_prop_narrow() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "ABCDEFGHIJ")
  (let ((ov (make-overlay 3 7)))
    (overlay-put ov 'face 'bold)
    (put-text-property 5 9 'face 'italic)
    (narrow-to-region 2 9)
    (list (get-char-property 4 'face)
          (get-char-property 6 'face)
          (get-char-property 8 'face))
    (widen)
    (list (get-char-property 4 'face)
          (get-char-property 6 'face)
          (get-char-property 8 'face))))"#,
    );
}

#[test]
fn divergence_combo_marker_insert_delete_narrow() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "ABCDEFGHIJ")
  (let ((m (set-marker (make-marker) 5)))
    (narrow-to-region 2 9)
    (goto-char 3)
    (insert "XX")
    (delete-region 6 8)
    (list (marker-position m)
          (point-min)
          (point-max)
          (buffer-string))
    (widen)
    (list (marker-position m) (buffer-string))))"#,
    );
}

#[test]
fn divergence_combo_save_excursion_kill_yank() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "Hello World Foo Bar")
  (save-excursion
    (goto-char 7)
    (kill-region 7 12))
  (list (buffer-string))
  (goto-char 7)
  (yank)
  (list (buffer-string) (point)))"#,
    );
}

#[test]
fn divergence_combo_hash_symbol_prop() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (setplist 'my-combo-sym '(a 1 b 2))
  (let ((ht (make-hash-table)))
    (puthash 'x 'my-combo-sym ht)
    (list (get 'my-combo-sym 'a)
          (get (gethash 'x ht) 'b)
          (put 'my-combo-sym 'c 3)
          (symbol-plist 'my-combo-sym))))"#,
    );
}

#[test]
fn divergence_combo_catch_throw_unwind_narrow() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "ABCDEFGHIJ")
  (narrow-to-region 3 8)
  (let ((result
         (catch 'done
           (unwind-protect
               (throw 'done (buffer-string))
             (widen)))))
    (list result
          (buffer-narrowed-p)
          (point-min)
          (point-max))))"#,
    );
}

#[test]
fn divergence_combo_read_print_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((data '(a (b . c) [1 2 3] "hello \"world\"" ?\n)))
  (list data
        (read (prin1-to-string data))
        (equal data (read (prin1-to-string data)))))"#,
    );
}

#[test]
fn divergence_combo_condition_case_save_restriction() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "ABCDEFGHIJ")
  (narrow-to-region 3 8)
  (let ((result
         (condition-case err
             (save-restriction
               (widen)
               (error "test"))
           (error (list (point-min) (point-max) err)))))
    (list result
          (point-min)
          (point-max)
          (buffer-narrowed-p))))"#,
    );
}

#[test]
fn divergence_combo_multiple_buffers() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((buf1 (generate-new-buffer " *combo1*"))
        (buf2 (generate-new-buffer " *combo2*")))
  (unwind-protect
      (progn
        (with-current-buffer buf1
          (insert "AAA")
          (setq buffer-undo-list nil))
        (with-current-buffer buf2
          (insert "BBB")
          (setq buffer-undo-list nil))
        (list (with-current-buffer buf1 (buffer-string))
              (with-current-buffer buf2 (buffer-string))
              (current-buffer)))
    (kill-buffer buf1)
    (kill-buffer buf2)))"#,
    );
}

#[test]
fn divergence_combo_keymap_closure() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((map (make-sparse-keymap))
        (counter 0)
        (increment (lambda () (interactive) (setq counter (1+ counter)))))
  (define-key map "a" increment)
  (list (lookup-key map "a")
        (commandp increment)
        counter))"#,
    );
}

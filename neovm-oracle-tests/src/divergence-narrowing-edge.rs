//! Divergence tests: narrowing, widening, restriction edge cases.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_narrow_to_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "Hello World Foo Bar")
  (narrow-to-region 7 11)
  (list (point-min) (point-max)
        (buffer-string))) "#,
    );
}

#[test]
fn divergence_widen() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "Hello World Foo Bar")
  (narrow-to-region 7 11)
  (let ((narrowed (buffer-string)))
    (widen)
    (list narrowed (point-min) (point-max)
          (buffer-string)))) "#,
    );
}

#[test]
fn divergence_narrow_and_restriction() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "ABCDEFGHIJ")
  (narrow-to-region 3 7)
  (list (buffer-narrowed-p)
        (point-min) (point-max)
        (region-beginning) (region-end))) "#,
    );
}

#[test]
fn divergence_buffer_narrowed_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (buffer-narrowed-p)
  (progn (insert "Hello") (buffer-narrowed-p))
  (progn (narrow-to-region 1 3) (buffer-narrowed-p))
  (progn (widen) (buffer-narrowed-p))) "#,
    );
}

#[test]
fn divergence_save_restriction() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "Hello World")
  (narrow-to-region 1 5)
  (let ((inner (list (point-min) (point-max) (buffer-string))))
    (save-restriction
      (widen)
      (list inner
            (point-min) (point-max) (buffer-string)
            (progn
              (save-restriction)
              (list (point-min) (point-max))))))) "#,
    );
}

#[test]
fn divergence_save_excursion_narrow() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "Hello World")
  (narrow-to-region 1 5)
  (let ((pos (point)))
    (save-excursion
      (widen)
      (goto-char 10))
    (list (point) (point-min) (point-max)))) "#,
    );
}

#[test]
fn divergence_narrow_with_overlay() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "Hello World")
  (make-overlay 1 6)
  (narrow-to-region 3 8)
  (list (overlays-in (point-min) (point-max))
        (length (overlays-in 1 12))
        (buffer-string))) "#,
    );
}

#[test]
fn divergence_narrow_with_text_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "Hello World")
  (put-text-property 1 6 'face 'bold)
  (narrow-to-region 3 8)
  (list (get-text-property (point-min) 'face)
        (get-text-property (1+ (point-min)) 'face)
        (buffer-string))) "#,
    );
}

#[test]
fn divergence_narrow_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "ABCDEFGHIJ")
  (narrow-to-region 3 7)
  (goto-char (point-min))
  (insert "X")
  (list (buffer-string) (point) (point-min) (point-max))
  (widen)
  (list (buffer-string))) "#,
    );
}

#[test]
fn divergence_narrow_delete() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "ABCDEFGHIJ")
  (narrow-to-region 3 7)
  (delete-region (point-min) (+ (point-min) 2))
  (list (buffer-string) (point-min) (point-max))
  (widen)
  (list (buffer-string))) "#,
    );
}

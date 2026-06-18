//! Text-property stickiness/inheritance (insert-and-inherit, rear-nonsticky,
//! get-char-property, propertize, field ops) and time edge cases (pre-epoch,
//! year boundary, far future) parity.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn insert_inherit() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert (propertize "AB" 'face 'bold 'rear-nonsticky nil))
  (goto-char (point-max))
  (insert-and-inherit "CD")
  (list (get-text-property 3 'face) (get-text-property 1 'face)))"##,
    );
}

#[test]
fn prop_field_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "aaabbbccc")
  (put-text-property 1 4 'field 'f1)
  (put-text-property 4 7 'field 'f2)
  (goto-char 2)
  (list (field-beginning) (field-end) (field-string)))"##,
    );
}

#[test]
fn propertize_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((s (propertize "hello" 'face 'bold 'help-echo "hi" 'x 1)))
  (list (get-text-property 0 'face s) (get-text-property 0 'x s)
        (text-properties-at 0 s)))"##,
    );
}

#[test]
fn stickiness_get() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "abc")
  (put-text-property 1 2 'face 'bold)
  (put-text-property 1 2 'rear-nonsticky t)
  (list (get-text-property 1 'face) (get-char-property 1 'face)))"##,
    );
}

#[test]
fn time_far_future() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (format-time-string "%Y-%m-%d" 4102444800 t)
        (format-time-string "%j" 4102444800 t)
        (nth 5 (decode-time 4102444800 t)))"##,
    );
}

#[test]
fn time_pre_epoch() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (format-time-string "%Y-%m-%d %H:%M:%S" '(-1 0) t)
        (format-time-string "%Y" '(-100 0) t)
        (format-time-string "%Y-%m-%d" -86400 t))"##,
    );
}

#[test]
fn time_year_boundary() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (format-time-string "%Y-%m-%d %H:%M:%S" '(1 4084) t)
        (format-time-string "%Y-%m-%d" 0 t)
        (decode-time 0 t))"##,
    );
}

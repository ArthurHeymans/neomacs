//! Divergence tests: miscellaneous Emacs Lisp builtins not yet covered.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn deficiency_yes_or_no_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'yes-or-no-p)
  (fboundp 'y-or-n-p)
  (fboundp 'read-char-choice))"#,
    );
}

#[test]
fn divergence_random_numbers() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (integerp (random))
  (>= (random 100) 0)
  (< (random 100) 100)
  (integerp (random 1000)))"#,
    );
}

#[test]
fn divergence_copy_alist() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let* ((orig '((a . 1) (b . 2) (c . 3)))
         (copy (copy-alist orig)))
  (setcdr (assoc 'b copy) 99)
  (list orig copy))"#,
    );
}

#[test]
fn divergence_copy_sequence() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let* ((orig '(1 2 3))
         (copy (copy-sequence orig)))
  (list (equal orig copy)
        (not (eq orig copy))
        (= (length copy) 3)))"#,
    );
}

#[test]
fn divergence_copy_tree() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let* ((orig '(a (b (c d)) e))
         (copy (copy-tree orig)))
  (list (equal orig copy)
        (not (eq (cadr orig) (cadr copy)))))"#,
    );
}

#[test]
fn deficiency_equal_including_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (equal-including-properties "abc" "abc")
  (equal-including-properties
    (propertize "abc" 'face 'bold)
    "abc")
  (equal-including-properties
    (propertize "abc" 'face 'bold)
    (propertize "abc" 'face 'bold)))"#,
    );
}

#[test]
fn divergence_plist_member() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((pl '(a 1 b 2 c 3)))
  (list (plist-member pl 'b)
        (plist-member pl 'z)
        (not (plist-member pl 'z))))"#,
    );
}

#[test]
fn divergence_format_mode_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'format-mode-line)
  (stringp (format-mode-line mode-line-format))
  (> (length (format-mode-line mode-line-format)) 0))"#,
    );
}

#[test]
fn divergence_accessible_keymaps() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'accessible-keymaps)
  (fboundp 'where-is-internal)
  (fboundp 'describe-bindings))"#,
    );
}

#[test]
fn divergence_local_key_bindings() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (keymapp (current-local-map))
  (keymapp (current-global-map))
  (or (null (current-local-map))
      (keymapp (current-local-map))))"#,
    );
}

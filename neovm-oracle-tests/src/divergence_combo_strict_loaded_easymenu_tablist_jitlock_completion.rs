//! Strict combo oracle probes, batch 61: more untested deterministic areas —
//! easy-menu (menu keymap construction), tabulated-list (entry printing),
//! jit-lock (fontification registration), and completion-metadata/boundaries.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::assert_oracle_parity_with_load;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_n1_easy_menu_define() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(let ((map (make-sparse-keymap)))
  (easy-menu-define probe-menu map "Probe menu"
    '("Probe"
      ["Item 1" (ignore) t]
      ["Item 2" (ignore) t]))
  (list (keymapp (lookup-key map [menu-bar probe]))
        (keymapp (lookup-key map [menu-bar probe]))))
"##,
        &["emacs-lisp/easymenu.el"],
    );
}

#[test]
fn div_n1_tabulated_list_print() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(with-temp-buffer
  (tabulated-list-mode)
  (setq tabulated-list-format [("A" 10 nil) ("B" 10 nil)])
  (setq tabulated-list-entries '(("a" ["1" "one"]) ("b" ["2" "two"])))
  (tabulated-list-init-header)
  (tabulated-list-print t)
  (buffer-string))
"##,
        &["emacs-lisp/tabulated-list.el"],
    );
}

#[test]
fn div_n1_jit_lock_fontify_now() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(with-temp-buffer
  (insert "abc def ghi")
  (jit-lock-register (lambda (start end) (put-text-property start end 'face 'bold)))
  (jit-lock-fontify-now)
  (list (get-text-property 1 'face)
        (get-text-property 5 'face)
        (get-text-property 9 'face)))
"##,
        &["jit-lock.el"],
    );
}

#[test]
fn div_n1_completion_metadata_and_boundaries() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((coll '("abc" "abd" "abe" "xyz")))
  (list (try-completion "a" coll)
        (all-completions "a" coll)
        (completion-boundaries "ab" coll nil "")))
"##,
    );
}

#[test]
fn div_n1_completion_table_substring() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(let ((coll (completion-table-substring '("bar baz" "foo bar"))))
  (list (try-completion "ba" coll)
        (all-completions "bar" coll)))
"##,
        &["minibuffer.el"],
    );
}

//! Completion deep coverage (transformers, metadata, styles).
//!
//! Beyond the known divergences (completion-ignore-case prefix case, extra
//! calendar-month category), completion is faithful. This batch covers the
//! table transformers (in-turn, merge), metadata, boundaries, try-completion,
//! and the style variants to pin that parity.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cdt_try_completion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(completion-try-completion "ap" '("apple" "apricot" "banana") nil 2)"##);
}

#[test]
fn div_cdt_all_completions_metadata_tail() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ac (completion-all-completions "ap" '("apple" "apricot") nil 2)))
  (list ac (and (consp ac) (consp (last ac)))))
"##,
    );
}

#[test]
fn div_cdt_completion_boundaries() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(completion-boundaries "ap" '("apple" "apricot") nil "x")"##);
}

#[test]
fn div_cdt_table_in_turn() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((tbl (completion-table-in-turn '("abc" "abd") '("xyz"))))
  (list (all-completions "" tbl) (try-completion "a" tbl)))
"##,
    );
}

#[test]
fn div_cdt_table_merge() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((tbl (completion-table-merge '("a" "b") '("c"))))
  (all-completions "" tbl))
"##,
    );
}

#[test]
fn div_cdt_metadata() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((md (completion-metadata "ap" '("apple") nil)))
  (list (consp md) (completion-metadata-get md 'category)))
"##,
    );
}

#[test]
fn div_cdt_flex_style() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((completion-styles '(flex)))
  (list (all-completions "abc" '("axbycz" "aXbYc" "axbyc"))
        (try-completion "abc" '("axbycz"))))
"##,
    );
}

#[test]
fn div_cdt_initials_style() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((completion-styles '(initials)))
  (all-completions "abc" '("a-big-cat" "another-bigger-cat" "axbyc")))
"##,
    );
}

#[test]
fn div_cdt_partial_style() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((completion-styles '(partial)))
  (all-completions "bc" '("abcd" "xabcd" "xbcd")))
"##,
    );
}

#[test]
fn div_cdt_ignored_extensions() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((completion-ignored-extensions '(".o" ".elc")))
  (all-completions "f" '("f.o" "f.elc" "f.c")))
"##,
    );
}

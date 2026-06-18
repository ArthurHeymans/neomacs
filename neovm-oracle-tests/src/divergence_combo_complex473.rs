/// Batch 473: edmacro, expand, ffap, find-func, find-dired, flow-fill, forms, gametree.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx473_edmacro_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'edmacro)
  (list (fboundp 'edmacro-format-keys) (fboundp 'edmacro-parse-keys)))
"##,
    );
}

#[test]
fn div_cx473_expand_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'expand)
  (list (boundp 'expand-list) (fboundp 'expand-abbrev-hook)))
"##,
    );
}

#[test]
fn div_cx473_ffap_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'ffap)
  (list (fboundp 'ffap) (fboundp 'ffap-menu)))
"##,
    );
}

#[test]
fn div_cx473_find_func_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'find-func)
  (list (fboundp 'find-function) (fboundp 'find-variable)))
"##,
    );
}

#[test]
fn div_cx473_find_dired_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'find-dired)
  (list (fboundp 'find-dired) (boundp 'find-ls-option)))
"##,
    );
}

#[test]
fn div_cx473_flow_fill_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'flow-fill)
  (list (fboundp 'fill-flowed) (fboundp 'fill-flowed-encode)))
"##,
    );
}

#[test]
fn div_cx473_forms_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'forms)
  (list (fboundp 'forms-mode) (boundp 'forms-version)))
"##,
    );
}

#[test]
fn div_cx473_gametree_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'gametree)
  (list (fboundp 'gametree-layout-mode) (boundp 'gametree-version)))
"##,
    );
}

#[test]
fn div_cx473_hide_show_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'hideshow)
  (list (fboundp 'hs-minor-mode) (boundp 'hs-special-modes-alist)))
"##,
    );
}

#[test]
fn div_cx473_ibuffer_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'ibuffer)
  (list (fboundp 'ibuffer) (boundp 'ibuffer-formats)))
"##,
    );
}

#[test]
fn div_cx473_jka_compr_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'jka-compr)
  (list (boundp 'jka-compr-compression-info-list) (fboundp 'jka-compr-install)))
"##,
    );
}

#[test]
fn div_cx473_kermit_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'kermit)
  (list (fboundp 'kermit) (boundp 'kermit-version)))
"##,
    );
}

#[test]
fn div_cx473_latin_lang() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'language/latin-1)
  (list (boundp 'latin-1-language-environment)))
"##,
    );
}

#[test]
fn div_cx473_language_env() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (boundp 'language-info-alist) (assoc "English" language-info-alist))
"##,
    );
}

#[test]
fn div_cx473_ledit_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'ledit)
  (list (fboundp 'ledit-mode) (boundp 'ledit-version)))
"##,
    );
}

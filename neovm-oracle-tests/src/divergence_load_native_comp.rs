//! Divergence tests: dynamic modules, native-comp, dynamic loading deep.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_module_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'module-load)
  (fboundp 'load)
  (featurep 'dynamic-modules)
  (fboundp 'list-dynamic-modules))"#,
    );
}

#[test]
fn divergence_native_comp() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'native-compile)
  (fboundp 'native-comp-available-p)
  (featurep 'native-compile))"#,
    );
}

#[test]
fn divergence_comp_el() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'comp-deferred-compilation)
  (boundp 'native-comp-deferred-compilation-deny-list)
  (boundp 'native-comp-async-query-onexit-jobs-number)
  (featurep 'comp))"#,
    );
}

#[test]
fn divergence_load_path_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (listp load-path)
  (listp load-suffixes)
  (fboundp 'locate-file)
  (fboundp 'locate-library)
  (member ".el" load-suffixes)
  (member ".elc" load-suffixes)) "#,
    );
}

#[test]
fn divergence_load_history() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'load-history)
  (listp load-history)
  (fboundp 'loadhist-unload-feature))"#,
    );
}

#[test]
fn divergence_autoload_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'autoload)
  (fboundp 'autoload-do-load)
  (fboundp 'update-file-autoloads)
  (fboundp 'update-directory-autoloads))"#,
    );
}

#[test]
fn divergence_feature_provide() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'provide)
  (fboundp 'require)
  (fboundp 'featurep)
  (listp features)
  (featurep 'emacs))"#,
    );
}

#[test]
fn divergence_after_load() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'eval-after-load)
  (fboundp 'after-load-functions)
  (boundp 'after-load-alist)
  (listp after-load-alist)) "#,
    );
}

#[test]
fn divergence_obsolete_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'make-obsolete)
  (fboundp 'define-obsolete-function-alias)
  (fboundp 'make-obsolete-variable)
  (fboundp 'define-obsolete-variable-alias))"#,
    );
}

#[test]
fn divergence_defalias_fset() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (defalias 'test-div-alias-xxx 'car)
  (list (fboundp 'test-div-alias-xxx)
        (eq (symbol-function 'test-div-alias-xxx) 'car)
        (funcall 'test-div-alias-xxx '(1 2 3)))) "#,
    );
}

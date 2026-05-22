//! Divergence tests: defvar/defconst, load, provide/require, autoload.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_defvar_only_sets_when_void() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (defvar my-dv-test-1 100)
  (defvar my-dv-test-1 999)
  my-dv-test-1)"#,
    );
}

#[test]
fn divergence_defconst_always_sets() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (defconst my-dc-test-1 100)
  (defconst my-dc-test-1 999)
  my-dc-test-1)"#,
    );
}

#[test]
fn divergence_defvar_inside_let_shadow() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (defvar my-dv-shadow 0)
  (let ((my-dv-shadow 42))
    (defvar my-dv-shadow 99)
    (list my-dv-shadow
          (eval 'my-dv-shadow)))
  my-dv-shadow)"#,
    );
}

#[test]
fn divergence_special_variable_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (defvar my-spec-var 0)
  (list (special-variable-p 'my-spec-var)
        (special-variable-p 'my-nonspec-var)
        (special-variable-p 'load-file-name)
        (special-variable-p 'buffer-read-only)))"#,
    );
}

#[test]
fn divergence_defvar_local_bare_declare() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ()
  (defvar my-bare-dv)
  (list (special-variable-p 'my-bare-dv)
        (boundp 'my-bare-dv)))"#,
    );
}

#[test]
fn divergence_featurep_after_provide() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (provide 'my-test-feature)
  (list (featurep 'my-test-feature)
        (member 'my-test-feature features)))"#,
    );
}

#[test]
fn divergence_provide_subfeature() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (provide '(my-test-sub . 42))
  (list (featurep 'my-test-sub)
        (featurep 'my-test-sub 42)))"#,
    );
}

#[test]
fn divergence_autoload_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (autoload 'my-autoload-fn "nonexistent-file" "doc" t)
  (list (fboundp 'my-autoload-fn)
        (autoloadp (symbol-function 'my-autoload-fn))
        (documentation 'my-autoload-fn)))"#,
    );
}

#[test]
fn divergence_load_path_search() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list (member (expand-file-name "emacs-lisp" (car load-path)) load-path)
              (consp load-path)
              (> (length load-path) 0))"#,
    );
}

#[test]
fn divergence_load_file_name_during_eval() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list load-file-name
              load-in-progress
              (booleanp load-in-progress))"#,
    );
}

#[test]
fn divergence_variable_alias() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (defvar my-alias-source 42)
  (defvaralias 'my-alias-target 'my-alias-source)
  (list my-alias-target
        (symbol-value 'my-alias-target)
        (variable-binding-alias 'my-alias-target)
        (setq my-alias-target 99)
        my-alias-source))"#,
    );
}

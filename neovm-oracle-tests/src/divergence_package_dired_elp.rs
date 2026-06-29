//! Divergence tests: package system, dired, and ELP (Emacs Lisp Profiler).

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_package_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'package-initialize)
  (fboundp 'package-install)
  (fboundp 'package-delete)
  (fboundp 'package-list-packages)
  (fboundp 'package-installed-p)
  (boundp 'package-alist)
  (listp package-alist))"#,
        expect_test::expect![[r#""ERR (void-variable package-alist)""#]],
    );
}

#[test]
fn divergence_package_archives() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (boundp 'package-archives)
  (listp package-archives)
  (consp (car package-archives)))"#,
        expect_test::expect![[r#""ERR (void-variable package-archives)""#]],
    );
}

#[test]
fn divergence_package_desc() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'package-desc-create)
  (fboundp 'package-desc-name)
  (fboundp 'package-desc-version))"#,
        expect_test::expect![[r#""OK (nil nil nil)""#]],
    );
}

#[test]
fn divergence_use_package() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'use-package)
  (featurep 'use-package)
  (fboundp 'require))"#,
        expect_test::expect![[r#""OK (t nil t)""#]],
    );
}

#[test]
fn divergence_dired_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'dired)
  (fboundp 'dired-other-window)
  (fboundp 'dired-get-filename)
  (fboundp 'dired-mark)
  (fboundp 'dired-unmark))"#,
        expect_test::expect![[r#""OK (t t nil nil nil)""#]],
    );
}

#[test]
fn divergence_dired_mode_vars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (boundp 'dired-listing-switches)
  (stringp dired-listing-switches)
  (boundp 'dired-recursive-deletes)
  (boundp 'dired-recursive-copies))"#,
        expect_test::expect![[r#""OK (t t nil nil)""#]],
    );
}

#[test]
fn divergence_elp_profiler() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'elp-instrument-function)
  (fboundp 'elp-instrument-package)
  (fboundp 'elp-results)
  (fboundp 'elp-reset-all))"#,
        expect_test::expect![[r#""OK (t t t nil)""#]],
    );
}

#[test]
fn divergence_elisp_benchmark() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'benchmark-run)
  (fboundp 'benchmark-run-compiled)
  (fboundp 'benchmark-elapse))"#,
        expect_test::expect![[r#""OK (t t nil)""#]],
    );
}

#[test]
fn divergence_info_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'info)
  (fboundp 'info lookup-symbol)
  (featurep 'info))"#,
        expect_test::expect![[r#""ERR (wrong-number-of-arguments fboundp 2)""#]],
    );
}

#[test]
fn divergence_eshell_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'eshell)
  (featurep 'eshell)
  (fboundp 'eshell-command))"#,
        expect_test::expect![[r#""OK (t nil t)""#]],
    );
}

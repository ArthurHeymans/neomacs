/// Batch 474: vc-deep, smerge, diff-mode, ediff, log-view, log-edit, add-log.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx474_vc_register() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'vc)
  (list (fboundp 'vc-register) (fboundp 'vc-next-action)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx474_vc_dir() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'vc-dir)
  (list (fboundp 'vc-dir) (boundp 'vc-dir-mode-map)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx474_vc_annotate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'vc-annotate)
  (list (fboundp 'vc-annotate) (boundp 'vc-annotate-mode-map)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx474_smerge_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'smerge-mode)
  (list (fboundp 'smerge-mode) (boundp 'smerge-basic-map)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx474_diff_mode_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'diff-mode)
  (list major-mode (derived-mode-p 'fundamental-mode)))
"##,
        expect_test::expect![[r#""OK (fundamental-mode fundamental-mode)""#]],
    );
}

#[test]
fn div_cx474_ediff_buffers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'ediff)
  (list (fboundp 'ediff-buffers) (fboundp 'ediff-files)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx474_ediff_merge() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'ediff-merge)
  (list (fboundp 'ediff-merge-files) (fboundp 'ediff-merge-buffers)))
"##,
        expect_test::expect![[
            r#""ERR (file-missing \"Cannot open load file\" \"No such file or directory\" \"ediff-merge\")""#
        ]],
    );
}

#[test]
fn div_cx474_log_view_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'log-view)
  (list (fboundp 'log-view-mode) (boundp 'log-view-mode-map)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx474_log_edit_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'log-edit)
  (list (fboundp 'log-edit) (boundp 'log-edit-mode-map)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx474_add_log_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'add-log)
  (list (fboundp 'add-log) (boundp 'change-log-default-name)))
"##,
        expect_test::expect![[r#""OK (nil t)""#]],
    );
}

#[test]
fn div_cx474_pcvs_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'pcvs)
  (list (fboundp 'cvs-examine) (fboundp 'cvs-update)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx474_eshell_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'esh-mode)
  (list (boundp 'eshell-mode-map) (fboundp 'eshell)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx474_shell_mode_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'shell)
  (list (boundp 'shell-mode-map) (fboundp 'shell)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx474_term_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'term)
  (list (boundp 'term-mode-map) (fboundp 'term)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx474_comint_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'comint)
  (list (boundp 'comint-mode-map) (fboundp 'comint-send-string)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

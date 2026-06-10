//! Divergence tests: ediff, diff, smerge, merge operations.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_ediff_controls() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'ediff-next-difference)
  (fboundp 'ediff-previous-difference)
  (fboundp 'ediff-jump-to-difference)
  (fboundp 'ediff-toggle-wide-display))"#,
    );
}

#[test]
fn divergence_ediff_merge() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'ediff-merge-files)
  (fboundp 'ediff-merge-files-with-ancestor)
  (fboundp 'ediff-merge-buffers)
  (fboundp 'ediff-merge-revisions))"#,
    );
}

#[test]
fn diff_tool_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'diff-file-local-copy)
  (fboundp 'diff-goto-source)
  (fboundp 'diff-hunk-status-msg-type)
  (featurep 'diff-mode)) "#,
    );
}

#[test]
fn divergence_smerge() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'smerge-mode)
  (fboundp 'smerge-find-conflict)
  (fboundp 'smerge-resolve)
  (featurep 'smerge-mode)) "#,
    );
}

#[test]
fn divergence_compare() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'compare-strings)
  (fboundp 'string-collate-lessp)
  (fboundp 'string-collate-equalp)
  (fboundp 'string-version-lessp)) "#,
    );
}

#[test]
fn divergence_diff_compare_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (compare-strings "hello" 0 5 "hello" 0 5)
  (compare-strings "hello" 0 3 "HELLO" 0 3)
  (compare-strings "hello" 0 3 "HELLO" 0 3 t)
  (string-version-lessp "1.2" "1.10")) "#,
    );
}

#[test]
fn divergence_vcs_log() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'vc-print-log)
  (fboundp 'vc-print-root-log)
  (fboundp 'vc-diff)
  (fboundp 'vc-revert)
  (fboundp 'vc-revision-other-window)
  (featurep 'vc)) "#,
    );
}

#[test]
fn divergence_vcs_annotate() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'vc-annotate)
  (fboundp 'vc-annotate-show-log-revision-at-line)
  (fboundp 'vc-next-action)) "#,
    );
}

#[test]
fn divergence_magit() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'magit-status)
  (fboundp 'magit-log-current)
  (featurep 'magit)
  (fboundp 'magit-dispatch)) "#,
    );
}

#[test]
fn divergence_project_vcs() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'project-vc-merge-subnets-p)
  (fboundp 'project-ignores)
  (fboundp 'project-files)
  (fboundp 'project-buffers)) "#,
    );
}

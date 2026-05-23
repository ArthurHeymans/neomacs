//! Divergence tests: completions, corfu, vertico, ido deep.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_vertico() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'vertico-mode)
  (featurep 'vertico)
  (fboundp 'vertico-next)
  (fboundp 'vertico-previous)) "#,
    );
}

#[test]
fn divergence_consult() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'consult-buffer)
  (fboundp 'consult-line)
  (fboundp 'consult-ripgrep)
  (featurep 'consult)) "#,
    );
}

#[test]
fn divergence_marginalia() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'marginalia-mode)
  (featurep 'marginalia)) "#,
    );
}

#[test]
fn divergence_orderless() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'orderless-filter)
  (featurep 'orderless)) "#,
    );
}

#[test]
fn divergence_embark() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'embark-act)
  (fboundp 'embark-dwim)
  (featurep 'embark)) "#,
    );
}

#[test]
fn divergence_ido() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'ido-mode)
  (fboundp 'ido-find-file)
  (fboundp 'ido-switch-buffer)
  (featurep 'ido)) "#,
    );
}

#[test]
fn divergence_ivy() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'ivy-mode)
  (fboundp 'counsel-M-x)
  (featurep 'ivy)
  (featurep 'counsel)) "#,
    );
}

#[test]
fn divergence_helm() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'helm-M-x)
  (fboundp 'helm-find-files)
  (fboundp 'helm-buffers-list)
  (featurep 'helm)) "#,
    );
}

#[test]
fn divergence_perspective() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'persp-mode)
  (featurep 'perspective)
  (fboundp 'eyebrowse-mode)
  (featurep 'eyebrowse)) "#,
    );
}

#[test]
fn divergence_magit_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'magit-stage-file)
  (fboundp 'magit-unstage-file)
  (fboundp 'magit-commit-create)
  (fboundp 'magit-push-current)
  (fboundp 'magit-pull-from-upstream)) "#,
    );
}

//! Divergence tests: comint, shell-mode, term, eshell stubs.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_comint_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'comint-run)
  (fboundp 'comint-send-input)
  (fboundp 'comint-send-string)
  (fboundp 'comint-interrupt-subjob)
  (featurep 'comint))"#,
    );
}

#[test]
fn divergence_shell_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'shell)
  (fboundp 'shell-command)
  (featurep 'shell))"#,
    );
}

#[test]
fn divergence_term_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'term)
  (fboundp 'ansi-term)
  (featurep 'term))"#,
    );
}

#[test]
fn divergence_eshell_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'eshell)
  (featurep 'eshell)
  (featurep 'esh-mode))"#,
    );
}

#[test]
fn divergence_ansi_color() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'ansi-color-apply)
  (fboundp 'ansi-color-filter-apply)
  (featurep 'ansi-color))"#,
    );
}

#[test]
fn divergence_exec_path_from_shell() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'exec-path-from-shell-getenv)
  (fboundp 'exec-path-from-shell-copy-env)
  (featurep 'exec-path-from-shell))"#,
    );
}

#[test]
fn divergence_tramp_connection() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'tramp-connect-with-su)
  (fboundp 'tramp-connect-with-sudo)
  (fboundp 'tramp-connect-with-ssh)
  (featurep 'tramp))"#,
    );
}

#[test]
fn divergence_subr_misc() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'posn-at-point)
  (fboundp 'posn-at-x-y)
  (fboundp 'window-absolute-pixel-edges)
  (fboundp 'frame-edge-positions))"#,
    );
}

#[test]
fn divergence_display_pixel_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'display-pixel-width)
  (fboundp 'display-pixel-height)
  (fboundp 'display-mm-width)
  (fboundp 'display-mm-height))"#,
    );
}

#[test]
fn divergence_frame_pixel_size() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'frame-pixel-width)
  (fboundp 'frame-pixel-height)
  (fboundp 'frame-char-width)
  (fboundp 'frame-char-height))"#,
    );
}

//! Divergence tests: buffer local variables, frame parameters deep.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_buffer_local_set() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn
  (set (make-local-variable 'test-blocal-xxx) 42)
  (list test-blocal-xxx
        (local-variable-p 'test-blocal-xxx)
        (buffer-local-value 'test-blocal-xxx (current-buffer)))) ",
    );
}

#[test]
fn divergence_buffer_local_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn
  (setq test-blocal-default-xxx 10)
  (set (make-local-variable 'test-blocal-default-xxx) 20)
  (list test-blocal-default-xxx
        (default-value 'test-blocal-default-xxx)
        (local-variable-p 'test-blocal-default-xxx))) ",
    );
}

#[test]
fn divergence_buffer_local_kill() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn
  (setq test-blocal-kill-xxx 100)
  (set (make-local-variable 'test-blocal-kill-xxx) 200)
  (kill-local-variable 'test-blocal-kill-xxx)
  (list test-blocal-kill-xxx
        (local-variable-p 'test-blocal-kill-xxx))) ",
    );
}

#[test]
fn divergence_buffer_locals_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(list
  (fboundp 'buffer-local-variables)
  (listp (buffer-local-variables))
  (fboundp 'buffer-local-value)
  (fboundp 'buffer-bound-p)
  (fboundp 'default-boundp)) ",
    );
}

#[test]
fn divergence_make_variable_buffer_local() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(list
  (fboundp 'make-variable-buffer-local)
  (fboundp 'make-local-variable)
  (fboundp 'kill-local-variable)
  (fboundp 'local-variable-p)
  (fboundp 'default-value)
  (fboundp 'set-default)) ",
    );
}

#[test]
fn divergence_frame_params_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(let ((params (frame-parameters)))
  (list (listp params)
        (assq 'name params)
        (assq 'width params)
        (assq 'height params)
        (assq 'fullscreen params))) ",
    );
}

#[test]
fn divergence_frame_terminal() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(list
  (fboundp 'frame-terminal)
  (fboundp 'terminal-name)
  (fboundp 'terminal-list)
  (fboundp 'terminal-live-p)) ",
    );
}

#[test]
fn divergence_frame_focus() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(list
  (fboundp 'select-frame)
  (fboundp 'selected-frame)
  (fboundp 'redirect-frame-focus)
  (fboundp 'frame-focus)) ",
    );
}

#[test]
fn divergence_frame_management() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(list
  (fboundp 'make-frame)
  (fboundp 'make-frame-on-display)
  (fboundp 'delete-frame)
  (fboundp 'frame-list)
  (fboundp 'next-frame)) ",
    );
}

#[test]
fn divergence_frame_parameters_modify() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(list
  (fboundp 'modify-frame-parameters)
  (fboundp 'set-frame-parameter)
  (fboundp 'frame-parameter)
  (fboundp 'frame-parameters)) ",
    );
}

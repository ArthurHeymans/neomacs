//! Interactive `kill-buffer` divergence probes.
//!
//! User-visible symptom: `C-x k` should run `kill-buffer` interactively, which
//! reads a buffer name (and therefore honors completion/read-buffer config).
//! Neomacs currently treats the built-in `kill-buffer` as a no-argument
//! interactive command, so invoking it through `call-interactively` kills the
//! current buffer immediately.

use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_kill_buffer_c_x_k_is_bound_to_kill_buffer_not_kill_current_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[r#""OK (kill-buffer kill-buffer t t)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"
(list (key-binding (kbd "C-x k"))
      (lookup-key (current-global-map) (kbd "C-x k"))
      (commandp 'kill-buffer)
      (commandp 'kill-current-buffer))
"##,
        expect,
    );
}

#[test]
fn div_kill_buffer_interactive_form_is_missing_the_read_buffer_spec() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Root cause of the whole divergence, isolated to a single, fully
    // deterministic value (no stdin / EOF / prompt-text dependence):
    //
    //   GNU:    (interactive-form 'kill-buffer) => (interactive "bKill buffer: ")
    //   Neomacs:(interactive-form 'kill-buffer) => nil
    //
    // Because Neomacs' built-in `kill-buffer` carries no interactive form,
    // `call-interactively` (and therefore the `C-x k` command loop) supplies
    // no buffer argument and kills the current buffer outright, instead of
    // reading a buffer name with completion the way GNU's "b" spec does.
    //
    // The inline snapshot records the *current* Neomacs value so this test is
    // green in the default snapshot mode and documents today's behavior; the
    // divergence surfaces under `NEOVM_ORACLE_MODE=verify`/`live`, where the
    // GNU oracle returns the `(interactive "bKill buffer: ")` spec instead.
    let expect = expect_test::expect![[r#""OK (nil (interactive nil) t)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"
(list (interactive-form 'kill-buffer)
      (interactive-form 'kill-current-buffer)
      (commandp 'kill-buffer))
"##,
        expect,
    );
}

#[test]
fn div_kill_buffer_interactive_uses_read_buffer_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity(
        r##"
(let* ((current (get-buffer-create "kb-rbf-current"))
       (chosen (get-buffer-create "kb-rbf-chosen"))
       seen)
  (set-buffer current)
  (let ((read-buffer-function
         (lambda (&rest args)
           (setq seen args)
           "kb-rbf-chosen")))
    (let ((result (condition-case err
                      (list 'ok (call-interactively 'kill-buffer))
                    (error (list 'err (car err) (cadr err))))))
      (list result
            (and seen
                 (mapcar (lambda (x)
                           (cond ((bufferp x) (buffer-name x))
                                 (t x)))
                         seen))
            (buffer-live-p current)
            (buffer-live-p chosen)
            (and (current-buffer) (buffer-name (current-buffer)))))))
"##,
    );
}

#[test]
fn div_kill_buffer_interactive_eof_preserves_current_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity(
        r##"
(let* ((current (get-buffer-create "kb-eof-current"))
       (other (get-buffer-create "kb-eof-other")))
  (set-buffer current)
  (let ((result (condition-case err
                    (list 'ok (call-interactively 'kill-buffer))
                  (error (list 'err (car err) (cadr err))))))
    (list result
          (buffer-live-p current)
          (and (buffer-live-p current) (buffer-name current))
          (buffer-live-p other)
          (and (current-buffer) (buffer-name (current-buffer))))))
"##,
    );
}

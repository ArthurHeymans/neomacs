use expect_test::expect;

use super::assert_alda_mode_parity;

#[test]
fn alda_input_sender_forwards_payload_and_required_newline_to_comint_process() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'comint-send-string)
                    (lambda (process string)
                      (push (list process string) calls)
                      (length calls))))
           (list
            (alda-input-sender 'alda-process "piano: c d e")
            (nreverse calls))))"##;
    let expect = expect![[r#"OK (2 ((alda-process "piano: c d e") (alda-process "\n")))"#]];
    assert_alda_mode_parity(elisp_form, expect);
}

#[test]
fn alda_interpreter_checks_named_comint_buffer_and_starts_only_when_absent() {
    let elisp_form = r##"(let (running calls)
         (cl-letf (((symbol-function 'comint-check-proc)
                    (lambda (buffer)
                      (push (list 'check buffer) calls)
                      running))
                   ((symbol-function 'alda-run-alda)
                    (lambda ()
                      (push '(run) calls)
                      'started)))
           (let ((first (alda-check-or-start-interpreter)))
             (setq running t)
             (list first
                   (alda-check-or-start-interpreter)
                   (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (started nil ((check "*inferior-alda*") (run) (check "*inferior-alda*")))"#
    ]];
    assert_alda_mode_parity(elisp_form, expect);
}

#[test]
fn alda_run_alda_constructs_comint_process_enables_mode_and_displays_buffer() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function 'alda-repl)
               (lambda () "/opt/alda repl --port 27713"))
              ((symbol-function 'alda-interpreter-running-p-1)
               (lambda () nil))
              ((symbol-function 'make-comint)
               (lambda (name program startfile &rest switches)
                 (push (list 'make name program startfile switches)
                       calls)
                 (get-buffer-create alda-inf-buffer-name)))
              ((symbol-function 'alda-mode-inf)
               (lambda () (push '(mode) calls)))
              ((symbol-function 'pop-to-buffer)
               (lambda (buffer)
                 (push (list 'pop buffer) calls)
                 'displayed)))
           (unwind-protect
               (list (alda-run-alda) (nreverse calls))
             (when (get-buffer alda-inf-buffer-name)
               (kill-buffer alda-inf-buffer-name)))))"##;
    let expect = expect![[
        r#"OK (displayed ((make "inferior-alda" "/opt/alda" nil ("repl" "--port" "27713")) (mode) (pop "*inferior-alda*")))"#
    ]];
    assert_alda_mode_parity(elisp_form, expect);
}

#[test]
fn alda_switch_to_interpreter_ensures_process_then_changes_other_window() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function 'alda-check-or-start-interpreter)
               (lambda () (push '(ensure) calls) 'ready))
              ((symbol-function 'switch-to-buffer-other-window)
               (lambda (buffer)
                 (push (list 'switch buffer) calls)
                 'switched)))
           (list
            (alda-switch-to-interpreter)
            (nreverse calls))))"##;
    let expect = expect![[r#"OK (switched ((ensure) (switch "*inferior-alda*")))"#]];
    assert_alda_mode_parity(elisp_form, expect);
}

#[test]
fn alda_inferior_region_sends_exact_slice_then_newline_after_startup() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function 'alda-check-or-start-interpreter)
               (lambda () (push '(ensure) calls)))
              ((symbol-function 'comint-send-region)
               (lambda (buffer start end)
                 (push (list 'region buffer start end
                             (buffer-substring-no-properties
                              start end))
                       calls)))
              ((symbol-function 'comint-send-string)
               (lambda (buffer string)
                 (push (list 'string buffer string) calls))))
           (with-temp-buffer
             (insert "piano: c d e")
             (list
              (alda-inf-eval-region 1 7)
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (#1=((string "*inferior-alda*" "\n")) ((ensure) (region "*inferior-alda*" 1 7 "piano:") . #1#))"#
    ]];
    assert_alda_mode_parity(elisp_form, expect);
}

#[test]
fn alda_down_runs_binary_command_then_deletes_output_process() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'alda-location)
                    (lambda () "/opt/alda"))
                   ((symbol-function 'shell-command)
                    (lambda (command)
                      (push (list 'shell command) calls)
                      0))
                   ((symbol-function 'delete-process)
                    (lambda (process)
                      (push (list 'delete process) calls)
                      'deleted)))
           (list
            (alda-down)
            (nreverse calls))))"##;
    let expect = expect![[r#"OK (deleted ((shell "/opt/alda down") (delete "*alda-output*")))"#]];
    assert_alda_mode_parity(elisp_form, expect);
}

use expect_test::expect;

use super::{assert_abs_mode_parity, assert_abs_mode_signal_parity};

#[test]
fn abs_mode_flymake_mode_on_enables_existing_files_or_defers_until_save() {
    let elisp_form = r##"(let ((answers '(t nil))
                    events)
               (with-temp-buffer
                 (setq buffer-file-name "/workspace/model.abs")
                 (cl-letf
                     (((symbol-function 'file-exists-p)
                       (lambda (path)
                         (push (list 'exists path) events)
                         (pop answers)))
                      ((symbol-function 'flymake-mode)
                       (lambda ()
                         (push '(flymake) events)
                         'enabled))
                      ((symbol-function 'add-hook)
                       (lambda (&rest arguments)
                         (push (cons 'add-hook arguments) events)
                         'added))
                      ((symbol-function 'remove-hook)
                       (lambda (&rest arguments)
                         (push
                          (cons 'remove-hook arguments)
                          events)
                         'removed)))
                   (list
                    (abs-flymake-mode-on)
                    (abs-flymake-mode-on)
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (removed added ((exists "/workspace/model.abs") (flymake) (remove-hook after-save-hook abs-flymake-mode-on t) (exists "/workspace/model.abs") (add-hook after-save-hook abs-flymake-mode-on nil t)))"#
    ]];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_run_model_maude_loads_generated_file_then_sends_start_command() {
    let elisp_form = r##"(let ((abs-backend 'maude)
                    (abs-maude-output-file "generated.maude")
                    events
                    (buffer
                     (generate-new-buffer
                      " *abs-maude-parity*")))
               (unwind-protect
                   (progn
                     (setq inferior-maude-buffer buffer)
                     (with-temp-buffer
                       (setq buffer-file-name
                             "/workspace/main.abs")
                       (cl-letf
                           (((symbol-function 'run-maude)
                             (lambda ()
                               (push '(run-maude) events)))
                            ((symbol-function
                              'comint-send-string)
                             (lambda (target text)
                               (push
                                (list
                                 'send-string
                                 (eq target buffer)
                                 text)
                                events)))
                            ((symbol-function 'sit-for)
                             (lambda (seconds)
                               (push
                                (list 'sit seconds)
                                events)))
                            ((symbol-function
                              'comint-send-input)
                             (lambda ()
                               (push
                                (list
                                 'send-input
                                 (buffer-string))
                                events)
                               'sent)))
                         (list
                          (abs--run-model)
                          (with-current-buffer
                              buffer
                            (buffer-string))
                          (nreverse events)))))
                 (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK (sent "frew start ." ((run-maude) (send-string t "in \"/workspace/generated.maude\"\n") (sit 1) (send-input "frew start .")))"#
    ]];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_run_model_erlang_routes_unix_and_windows_commands_with_optional_limits_ports() {
    let elisp_form = r##"(with-temp-buffer
               (let ((abs-backend 'erlang)
                     (abs-output-directory "/workspace/gen")
                     (abs-clock-limit 12)
                     (abs-local-port 8080)
                     events)
                 (cl-letf
                     (((symbol-function 'get-buffer)
                       (lambda (name)
                         (push (list 'get name) events)
                         nil))
                      ((symbol-function 'inferior-erlang)
                       (lambda (command)
                         (push
                          (list 'inferior-erlang command)
                          events)
                         'unix-result))
                      ((symbol-function 'get-buffer-create)
                       (lambda (name)
                         (push
                          (list 'create name)
                          events)
                         (current-buffer)))
                      ((symbol-function 'pop-to-buffer)
                       (lambda (buffer)
                         (push
                          (list
                           'pop
                           (eq buffer (current-buffer)))
                          events)))
                      ((symbol-function 'shell-command)
                       (lambda (command buffer)
                         (push
                          (list
                           'shell
                           command
                           (eq buffer (current-buffer)))
                          events)
                         'windows-result)))
                   (list
                    (let ((window-system nil))
                      (abs--run-model))
                    (let ((window-system 'w32))
                      (abs--run-model))
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (unix-result windows-result ((get "*erlang*") (inferior-erlang "/workspace/gen/run -l 12  -p 8080 ") (get "*erlang*") (create "*erlang*") (pop t) (shell "/workspace/gen/run.bat -l 12  -p 8080 " t)))"#
    ]];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_run_model_java_replaces_named_buffer_and_builds_exact_classpath_command() {
    let elisp_form = r##"(with-temp-buffer
               (let ((abs-backend 'java)
                     (abs-java-classpath
                      "/workspace/abs frontend.jar")
                     (abs-clock-limit 5)
                     (abs-local-port 9000)
                     events)
                 (cl-letf
                     (((symbol-function 'abs--guess-module)
                       (lambda () "Demo.Main"))
                      ((symbol-function 'get-buffer)
                       (lambda (name)
                         (push (list 'get name) events)
                         'old-buffer))
                      ((symbol-function 'kill-buffer)
                       (lambda (buffer)
                         (push (list 'kill buffer) events)
                         t))
                      ((symbol-function 'get-buffer-create)
                       (lambda (name)
                         (push (list 'create name) events)
                         (current-buffer)))
                      ((symbol-function 'pop-to-buffer)
                       (lambda (buffer)
                         (push
                          (list
                           'pop
                           (eq buffer (current-buffer)))
                          events)))
                      ((symbol-function 'shell-command)
                       (lambda (command buffer)
                         (push
                          (list
                           'shell
                           command
                           (eq buffer (current-buffer)))
                          events)
                         'run-result)))
                   (list
                    (abs--run-model)
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (run-result ((get "*abs java Demo.Main*") (get "*abs java Demo.Main*") (kill old-buffer) (create "*abs java Demo.Main*") (pop t) (shell "java -cp gen:/workspace/abs frontend.jar Demo.Main.Main -l 5  -p 9000  &" t)))"#
    ]];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_run_model_rejects_the_non_runnable_prolog_backend() {
    let elisp_form = r##"(let ((abs-backend 'prolog))
               (abs--run-model))"##;
    let expect = expect![[r#"ERR (error "Don’t know how to run with target prolog")"#]];

    assert_abs_mode_signal_parity(elisp_form, expect);
}

#[test]
fn abs_mode_next_action_selects_backend_then_compiles_or_runs_without_leaking_binding() {
    let elisp_form = r##"(let ((abs-backend 'erlang)
                    (needs '(t nil nil))
                    events)
               (cl-letf
                   (((symbol-function 'abs--read-backend)
                     (lambda ()
                       (push '(read-backend) events)
                       'java))
                    ((symbol-function 'abs--needs-compilation)
                     (lambda ()
                       (push
                        (list 'needs abs-backend)
                        events)
                       (pop needs)))
                    ((symbol-function 'abs--compile-model)
                     (lambda ()
                       (push
                        (list 'compile abs-backend)
                        events)
                       'compiled))
                    ((symbol-function 'abs--run-model)
                     (lambda ()
                       (push
                        (list 'run abs-backend)
                        events)
                       'ran)))
                 (list
                  (abs-next-action 1)
                  abs-backend
                  (abs-next-action 4)
                  abs-backend
                  (progn
                    (cl-letf
                        (((symbol-function
                           'abs--read-backend)
                          (lambda () nil)))
                      (abs-next-action 4)))
                  abs-backend
                  (nreverse events))))"##;
    let expect = expect![
        "OK (compiled erlang ran erlang ran erlang ((needs erlang) (compile erlang) (read-backend) (needs java) (run java) (needs erlang) (run erlang)))"
    ];

    assert_abs_mode_parity(elisp_form, expect);
}

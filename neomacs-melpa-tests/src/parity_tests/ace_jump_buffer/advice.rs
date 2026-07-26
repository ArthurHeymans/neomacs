use expect_test::expect;

use super::assert_ace_jump_buffer_parity;

#[test]
fn ace_jump_buffer_header_advice_calls_original_only_outside_the_menu() {
    let elisp_form = r##"(let (calls)
               (list
                (let ((ajb/showing nil))
                  (ajb/bs--show-header--around
                   (lambda ()
                     (push 'original calls)
                     'header-result)))
                (let ((ajb/showing t))
                  (ajb/bs--show-header--around
                   (lambda ()
                     (push 'unexpected calls)
                     'unexpected-result)))
                (nreverse calls)))"##;
    let expect = expect!["OK (header-result nil (original))"];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_header_advice_propagates_original_errors_outside_the_menu() {
    let elisp_form = r##"(let ((ajb/showing nil))
               (ajb/bs--show-header--around
                (lambda ()
                  (error "synthetic header failure"))))"##;
    let expect = expect![[r#"ERR (error "synthetic header failure")"#]];
    super::assert_ace_jump_buffer_signal_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_real_header_dispatch_inserts_normally_and_suppresses_while_showing() {
    let elisp_form = r##"(with-temp-buffer
               (let ((bs-attributes-list
                      '(("Head" 4 4 left "value"))))
                 (let ((normal-result
                        (let ((ajb/showing nil))
                          (bs--show-header)))
                       (normal-text
                        (buffer-string)))
                   (erase-buffer)
                   (let ((showing-result
                          (let ((ajb/showing t))
                            (bs--show-header))))
                     (list
                      normal-result
                      normal-text
                      showing-result
                      (buffer-string))))))"##;
    let expect = expect![[r#"OK (nil "Head\n----\n" nil "")"#]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_configuration_advice_sets_live_sort_function_only_while_showing() {
    let elisp_form = r##"(let ((bs-buffer-sort-function 'outer)
                   (ajb-sort-function 'selected))
               (list
                (let ((ajb/showing nil))
                  (list
                   (ajb/bs-set-configuration--after "ignored")
                   bs-buffer-sort-function))
                (let ((ajb/showing t))
                  (list
                   (ajb/bs-set-configuration--after "also-ignored")
                   bs-buffer-sort-function))
                bs-buffer-sort-function))"##;
    let expect = expect!["OK ((nil outer) (selected selected) selected)"];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_configuration_advice_accepts_nil_sort_function() {
    let elisp_form = r##"(let ((bs-buffer-sort-function 'outer)
                   (ajb-sort-function nil)
                   (ajb/showing t))
               (list
                (ajb/bs-set-configuration--after nil)
                bs-buffer-sort-function))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_real_configuration_dispatch_runs_the_attached_after_advice() {
    let elisp_form = r##"(let ((bs-configurations
                    '(("chosen" nil nil nil nil config-sort)))
                   (bs-buffer-sort-function 'outer)
                   (bs-current-configuration "outer")
                   (ajb-sort-function 'ajb-sort)
                   (ajb/showing t))
               (list
                (bs-set-configuration "chosen")
                bs-current-configuration
                bs-buffer-sort-function))"##;
    let expect = expect![[r#"OK (config-sort "chosen" ajb-sort)"#]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_recentf_sort_orders_only_two_known_buffer_files() {
    let elisp_form = r##"(let ((first
                    (generate-new-buffer "ajb-first"))
                   (second
                    (generate-new-buffer "ajb-second"))
                   (missing
                    (generate-new-buffer "ajb-missing")))
               (unwind-protect
                   (progn
                     (with-current-buffer first
                       (setq buffer-file-name
                             "/workspace/first.el"))
                     (with-current-buffer second
                       (setq buffer-file-name
                             "/workspace/second.el"))
                     (with-current-buffer missing
                       (setq buffer-file-name
                             "/workspace/missing.el"))
                     (let ((recentf-list
                            '("/workspace/second.el"
                              "/workspace/first.el")))
                       (list
                        (bs--sort-by-recentf second first)
                        (bs--sort-by-recentf first second)
                        (bs--sort-by-recentf first missing)
                        (bs--sort-by-recentf missing first)
                        (bs--sort-by-recentf first first))))
                 (kill-buffer first)
                 (kill-buffer second)
                 (kill-buffer missing)))"##;
    let expect = expect!["OK (t nil nil nil nil)"];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_recentf_sort_treats_nil_file_names_as_members_when_nil_is_listed() {
    let elisp_form = r##"(let ((first
                    (generate-new-buffer "ajb-nil-first"))
                   (second
                    (generate-new-buffer "ajb-nil-second")))
               (unwind-protect
                   (progn
                     (with-current-buffer second
                       (setq buffer-file-name
                             "/workspace/second.el"))
                     (let ((recentf-list
                            '(nil "/workspace/second.el")))
                       (list
                        (bs--sort-by-recentf first second)
                        (bs--sort-by-recentf second first))))
                 (kill-buffer first)
                 (kill-buffer second)))"##;
    let expect = expect!["OK (t nil)"];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

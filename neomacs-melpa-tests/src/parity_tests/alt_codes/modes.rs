use expect_test::expect;

use super::assert_alt_codes_parity;

#[test]
fn alt_codes_local_mode_adds_and_removes_only_its_buffer_local_hook() {
    let elisp_form = r##"(let ((global-before
                        (default-value
                         'pre-command-hook)))
         (with-temp-buffer
           (let ((before pre-command-hook))
             (alt-codes-mode 1)
             (let ((enabled
                    (list
                     alt-codes-mode
                     (local-variable-p
                      'pre-command-hook)
                     (memq
                      #'alt-codes--pre-command-hook
                      pre-command-hook))))
               (alt-codes-mode -1)
               (list
                before
                enabled
                (list
                 alt-codes-mode
                 (memq
                  #'alt-codes--pre-command-hook
                  pre-command-hook))
                (equal
                 global-before
                 (default-value
                  'pre-command-hook)))))))"##;
    let expect = expect!["OK ((tooltip-hide) (t t (alt-codes--pre-command-hook t)) (nil nil) t)"];
    assert_alt_codes_parity(elisp_form, expect);
}

#[test]
fn alt_codes_enable_disable_helpers_are_idempotent() {
    let elisp_form = r##"(with-temp-buffer
         (alt-codes--enable)
         (alt-codes--enable)
         (let ((enabled
                (list
                 (local-variable-p 'pre-command-hook)
                 (cl-count
                  #'alt-codes--pre-command-hook
                  pre-command-hook))))
           (alt-codes--disable)
           (alt-codes--disable)
           (list
            enabled
            (cl-count
             #'alt-codes--pre-command-hook
             pre-command-hook))))"##;
    let expect = expect!["OK ((t 1) 0)"];
    assert_alt_codes_parity(elisp_form, expect);
}

#[test]
fn alt_codes_turn_on_helper_enables_mode_through_public_command() {
    let elisp_form = r##"(with-temp-buffer
         (let (arguments)
           (cl-letf
               (((symbol-function 'alt-codes-mode)
                 (lambda (&optional argument)
                   (push argument arguments)
                   'enabled)))
             (list
              (alt-codes-turn-on-alt-codes-mode)
              (nreverse arguments)))))"##;
    let expect = expect!["OK (enabled (1))"];
    assert_alt_codes_parity(elisp_form, expect);
}

#[test]
fn alt_codes_global_mode_enables_existing_eligible_buffers_and_cleans_up() {
    let elisp_form = r##"(let ((first
                (generate-new-buffer "alt-global-one"))
               (second
                (generate-new-buffer " alt-global-hidden")))
         (unwind-protect
             (progn
               (with-current-buffer first
                 (fundamental-mode))
               (with-current-buffer second
                 (fundamental-mode))
               (global-alt-codes-mode 1)
               (let ((enabled
                      (mapcar
                       (lambda (buffer)
                         (with-current-buffer buffer
                           (list
                            (buffer-name)
                            alt-codes-mode
                            (memq
                             #'alt-codes--pre-command-hook
                             pre-command-hook))))
                       (list first second))))
                 (global-alt-codes-mode -1)
                 (list
                  enabled
                  (mapcar
                   (lambda (buffer)
                     (with-current-buffer buffer
                       (list
                        (buffer-name)
                        alt-codes-mode
                        (memq
                         #'alt-codes--pre-command-hook
                         pre-command-hook))))
                   (list first second)))))
           (global-alt-codes-mode -1)
           (kill-buffer first)
           (kill-buffer second)))"##;
    let expect = expect![[
        r#"OK ((("alt-global-one" t (alt-codes--pre-command-hook eldoc-pre-command-refresh-echo-area t)) (" alt-global-hidden" t (alt-codes--pre-command-hook eldoc-pre-command-refresh-echo-area t))) (("alt-global-one" nil nil) (" alt-global-hidden" nil nil)))"#
    ]];
    assert_alt_codes_parity(elisp_form, expect);
}

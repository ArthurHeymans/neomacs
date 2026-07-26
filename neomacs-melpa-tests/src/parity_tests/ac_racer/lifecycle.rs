use expect_test::expect;

use super::{assert_ac_racer_autoload_parity, assert_ac_racer_parity};

#[test]
fn ac_racer_setup_enables_auto_complete_adds_source_once_and_preserves_existing_order() {
    let elisp_form = r##"(let ((ac-sources
                    '(first-source))
                   calls)
               (cl-letf
                   (((symbol-function
                      'auto-complete-mode)
                     (lambda (argument)
                       (push
                        (list
                         'mode argument)
                        calls)
                       'enabled)))
                 (let ((first
                        (ac-racer-setup)))
                   (let ((second
                          (ac-racer-setup)))
                     (list
                      first
                      second
                      ac-sources
                      (nreverse calls)
                      (commandp
                       'ac-racer-setup)
                      (interactive-form
                       'ac-racer-setup))))))"##;
    let expect = expect![
        "OK (#1=(ac-source-racer first-source) #1# #1# ((mode 1) (mode 1)) t (interactive nil))"
    ];

    assert_ac_racer_parity(elisp_form, expect);
}

#[test]
fn ac_racer_setup_mutates_buffer_local_sources_without_changing_default_sources() {
    let elisp_form = r##"(let ((default-before
                    (default-value
                     'ac-sources)))
               (cl-letf
                   (((symbol-function
                      'auto-complete-mode)
                     (lambda (_argument)
                       (setq-local
                        ac-sources
                        (copy-sequence
                         ac-sources))
                       'enabled)))
                 (with-temp-buffer
                   (setq-local
                    ac-sources
                    '(buffer-source))
                   (let ((return
                          (call-interactively
                           'ac-racer-setup)))
                     (list
                      return
                      ac-sources
                      (equal
                       default-before
                       (default-value
                        'ac-sources))
                      (local-variable-p
                       'ac-sources))))))"##;
    let expect = expect!["OK (#1=(ac-source-racer buffer-source) #1# t t)"];

    assert_ac_racer_parity(elisp_form, expect);
}

#[test]
fn ac_racer_fresh_autoload_registers_only_the_interactive_setup_entrypoint() {
    let elisp_form = r##"(list
               (featurep
                'ac-racer)
               (featurep
                'ac-racer-autoloads)
               (fboundp
                'ac-racer-setup)
               (autoloadp
                (symbol-function
                 'ac-racer-setup))
               (commandp
                'ac-racer-setup)
               (symbol-function
                'ac-racer-setup)
               (mapcar
                #'fboundp
                '(ac-racer--collect-candidates
                  ac-racer--prefix
                  ac-racer--candidates))
               (get
                'ac-racer
                'custom-loads))"##;
    let expect = expect![[r#"OK (nil t t t t (autoload "ac-racer" nil t nil) (nil nil nil) nil)"#]];

    assert_ac_racer_autoload_parity(elisp_form, expect);
}

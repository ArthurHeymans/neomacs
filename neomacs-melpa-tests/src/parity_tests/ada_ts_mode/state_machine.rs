use expect_test::expect;

use super::assert_ada_ts_mode_parity;

#[test]
fn ada_ts_mode_grammar_install_state_machine_covers_unavailable_auto_prompt_decline_and_failure() {
    let elisp_form = r##"(cl-labels
         ((run
           (treesit-available
            grammar-available
            install-mode
            prompt-answer
            install-error)
           (let ((installed
                  grammar-available)
                 events)
             (cl-letf
                 (((symbol-function
                    'treesit-available-p)
                   (lambda ()
                     (push
                      'treesit-available
                      events)
                     treesit-available))
                  ((symbol-function
                    'treesit-language-available-p)
                   (lambda (language)
                     (push
                      (list
                       'language-available
                       language
                       installed)
                      events)
                     installed))
                  ((symbol-function
                    'treesit-ready-p)
                   (lambda (language &rest _)
                     (push
                      (list
                       'ready
                       language
                       installed)
                      events)
                     installed))
                  ((symbol-function
                    'treesit-install-language-grammar)
                   (lambda (language)
                     (push
                      (list
                       'install
                       language)
                      events)
                     (if install-error
                         (error
                          "fixture install failure")
                       (setq
                        installed
                        t)
                       'installed)))
                  ((symbol-function
                    'y-or-n-p)
                   (lambda (prompt)
                     (push
                      (list
                       'prompt
                       prompt)
                      events)
                     prompt-answer))
                  ((symbol-function
                    'treesit-parser-create)
                   (lambda (language)
                     (push
                      (list
                       'parser
                       language)
                      events)
                     'fixture-parser))
                  ((symbol-function
                    'treesit-major-mode-setup)
                   (lambda ()
                     (push
                      'major-mode-setup
                      events)
                     'setup)))
               (let ((outcome
                      (condition-case error-data
                          (with-temp-buffer
                            (let ((ada-ts-mode-grammar-install
                                   install-mode))
                              (ada-ts-mode))
                            (list
                             'ok
                             major-mode
                             mode-name))
                        (error
                         (list
                          'error
                          (car
                           error-data)
                          (error-message-string
                           error-data))))))
                 (list
                  outcome
                  installed
                  (nreverse
                   events)))))))
         (list
          (run
           nil
           nil
           nil
           nil
           nil)
          (run
           t
           nil
           nil
           nil
           nil)
          (run
           t
           nil
           'auto
           nil
           nil)
          (run
           t
           nil
           'prompt
           t
           nil)
          (run
           t
           nil
           'prompt
           nil
           nil)
          (run
           t
           nil
           'auto
           nil
           t)
          (run
           t
           t
           'prompt
           nil
           nil)))"##;
    let expect = expect![[
        r#"OK (((error error "Tree-sitter for Ada isn’t available") nil (treesit-available (ready ada nil))) ((error error "Tree-sitter for Ada isn’t available") nil (treesit-available (language-available ada nil) (ready ada nil))) ((ok ada-ts-mode "Ada") t (treesit-available (language-available ada nil) (install ada) (ready ada t) (parser ada) major-mode-setup)) ((ok ada-ts-mode "Ada") t (treesit-available (language-available ada nil) (prompt "Tree-sitter grammar for Ada is missing.  Install it from https://github.com/briot/tree-sitter-ada? ") (install ada) (ready ada t) (parser ada) major-mode-setup)) ((error error "Tree-sitter for Ada isn’t available") nil (treesit-available (language-available ada nil) (prompt "Tree-sitter grammar for Ada is missing.  Install it from https://github.com/briot/tree-sitter-ada? ") (ready ada nil))) ((error error "fixture install failure") nil (treesit-available (language-available ada nil) (install ada))) ((ok ada-ts-mode "Ada") t (treesit-available (language-available ada t) (ready ada t) (parser ada) major-mode-setup)))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

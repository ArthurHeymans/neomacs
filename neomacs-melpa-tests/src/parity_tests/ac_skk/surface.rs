use expect_test::expect;

use super::assert_ac_skk_parity;

#[test]
fn ac_skk_internal_defaults_hook_and_all_active_advice_registrations_match() {
    let elisp_form = r##"(list
               ac-skk-selected-candidate
               ac-skk-ac-trigger-commands-orig
               ac-skk-enable
               ac-skk-ac-sources-orig
               ac-skk-save-variable
               (cl-count
                'ac-skk-setup
                skk-mode-hook)
               (not
                (null
                 (advice-member-p
                  #'ad-Advice-skk-mode-exit
                  'skk-mode-exit)))
               (not
                (null
                 (advice-member-p
                  #'ad-Advice-skk-j-mode-on
                  'skk-j-mode-on)))
               (not
                (null
                 (advice-member-p
                  #'ad-Advice-skk-latin-mode
                  'skk-latin-mode)))
               (not
                (null
                 (advice-member-p
                  #'ad-Advice-ac-trigger-command-p
                  'ac-trigger-command-p)))
               (not
                (null
                 (advice-member-p
                  #'ad-Advice-ac-expand-string
                  'ac-expand-string)))
               (help-function-arglist
                'ad-Advice-skk-mode-exit
                t)
               (help-function-arglist
                'ad-Advice-skk-j-mode-on
                t)
               (help-function-arglist
                'ad-Advice-skk-latin-mode
                t)
               (help-function-arglist
                'ad-Advice-ac-trigger-command-p
                t)
               (help-function-arglist
                'ad-Advice-ac-expand-string
                t))"##;
    let expect = expect![
        "OK (nil nil nil nil (ac-trigger-commands skk-dcomp-activate skk-dcomp-multiple-activate) 1 t t t t t (ad--addoit-function) (ad--addoit-function &optional katakana) (ad--addoit-function arg) (ad--addoit-function command) (ad--addoit-function string &optional remove-undo-boundary))"
    ];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_packaged_source_descriptor_autoload_and_readme_assets_have_exact_hashes() {
    let elisp_form = r##"(let ((root
                    (file-name-directory
                     (symbol-file
                      'ac-skk-prefix
                      'defun))))
               (mapcar
                (lambda (file)
                  (with-temp-buffer
                    (insert-file-contents-literally
                     (expand-file-name
                      file
                      root))
                    (list
                     file
                     (buffer-size)
                     (secure-hash
                      'sha256
                      (current-buffer)))))
                '("ac-skk.el"
                  "ac-skk-pkg.el"
                  "ac-skk-autoloads.el"
                  "README-elpa")))"##;
    let expect = expect![[
        r#"OK (("ac-skk.el" 8644 "092dd61bcd0a8466b6fff62d01c08afde7e435a43dc694f9a044be0d5b1afcdd") ("ac-skk-pkg.el" 495 "6ea326206c584aa5d6e713dad1246a199bdda5e38d7aac39f402063754d20773") ("ac-skk-autoloads.el" 792 "8ddea8bd18d302246c1d72c3f8a2ff520f3590906ae99445ade86dba89d32ebb") ("README-elpa" 359 "14fead7b521986c03fe4492814982b767b8577a9e88de5d80a37899d076c0f6a"))"#
    ]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_source_callbacks_honor_runtime_rebinding_while_match_callbacks_return_candidates() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'ac-skk-prefix)
                     (lambda ()
                       (push
                        'prefix
                        calls)
                       8))
                    ((symbol-function
                      'ac-skk-candidates)
                     (lambda ()
                       (push
                        'candidates
                        calls)
                       '(candidate-a)))
                    ((symbol-function
                      'ac-skk-prefix-hiracomp)
                     (lambda ()
                       (push
                        'hira-prefix
                        calls)
                       4))
                    ((symbol-function
                      'ac-skk-hiracomp-candidates)
                     (lambda ()
                       (push
                        'hira-candidates
                        calls)
                       '(candidate-b))))
                 (list
                  (funcall
                   (cdr
                    (assq
                     'prefix
                     ac-source-skk)))
                  (funcall
                   (cdr
                    (assq
                     'candidates
                     ac-source-skk)))
                  (funcall
                   (cdr
                    (assq
                     'match
                     ac-source-skk))
                   "pre"
                   '(one two))
                  (funcall
                   (cdr
                    (assq
                     'prefix
                     ac-source-skk-hiracomp)))
                  (funcall
                   (cdr
                    (assq
                     'candidates
                     ac-source-skk-hiracomp)))
                  (funcall
                   (cdr
                    (assq
                     'match
                     ac-source-skk-hiracomp))
                   "pre"
                   '(three four))
                  (nreverse
                   calls))))"##;
    let expect = expect![
        "OK (8 (candidate-a) (one two) 4 (candidate-b) (three four) (prefix candidates hira-prefix hira-candidates))"
    ];

    assert_ac_skk_parity(elisp_form, expect);
}

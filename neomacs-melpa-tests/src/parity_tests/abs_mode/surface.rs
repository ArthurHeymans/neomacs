use expect_test::expect;

use super::{assert_abs_mode_parity, assert_abs_mode_signal_parity};

#[test]
fn abs_mode_exact_pinned_source_loads_with_its_dependency_closure() {
    let elisp_form = r##"(list
               (featurep 'abs-mode)
               (fboundp 'abs-mode)
               (fboundp 'abs-next-action)
               (featurep 'yasnippet)
               (featurep 'flymake-proc)
               (featurep 'cc-mode))"##;
    let expect = expect!["OK (t t t t t t)"];
    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_public_surface_and_command_classification_match_the_pin() {
    let elisp_form = r##"(list
               (mapcar
                #'fboundp
                '(abs-maybe-yasnippet-mode-on
                  abs--read-backend
                  abs--current-buffer-referenced-modules
                  abs--file-imports
                  abs--current-buffer-module-definitions
                  abs--file-module-definitions
                  abs--module-file-alist
                  abs--calculate-input-files
                  abs-flymake-mode-on
                  abs-flymake-init
                  abs--file-date-<
                  abs--input-files
                  abs--maude-filename
                  abs--absolutify-filename
                  abs--real-output-directory
                  abs--guess-module
                  abs--calculate-compile-command
                  abs--needs-compilation
                  abs--compile-model
                  abs--compile-model-no-prompt
                  abs--run-model
                  abs-next-action
                  abs--inside-string-or-comment-p
                  abs-beginning-of-definition
                  abs-end-of-definition
                  abs-mode
                  abs-check-installation
                  abs-download-compiler))
               (mapcar
                #'commandp
                '(abs-maybe-yasnippet-mode-on
                  abs--read-backend
                  abs--compile-model
                  abs--compile-model-no-prompt
                  abs--run-model
                  abs-next-action
                  abs-beginning-of-definition
                  abs-end-of-definition
                  abs-mode
                  abs-check-installation
                  abs-download-compiler)))"##;
    let expect = expect![
        "OK ((t t t t t t t t t t t t t t t t t t t t t t t t t t t t) (nil nil nil nil nil t t t t t t))"
    ];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_defaults_custom_metadata_obsolete_aliases_and_registrations_match_the_pin() {
    let elisp_form = r##"(list
               abs--backends
               (mapcar
                #'symbol-value
                '(abs-backend
                  abs-compiler-program
                  abs-output-directory
                  abs-java-classpath
                  abs-use-timed-interpreter
                  abs-clock-limit
                  abs-local-port
                  abs-compile-with-coverage-info
                  abs-default-resourcecost
                  abs-link-source-path
                  abs-directory
                  abs-product-name
                  abs-maude-output-file
                  abs-java-output-jar-file
                  abs-input-files
                  abs-modelapi-index-file
                  abs-modelapi-static-dir
                  abs-compile-command))
               abs-mode-hook
               (mapcar
                (lambda (variable)
                  (list
                   variable
                   (get variable 'custom-group)
                   (get variable 'custom-type)))
                '(abs-backend
                  abs-compiler-program
                  abs-output-directory
                  abs-java-classpath
                  abs-use-timed-interpreter
                  abs-clock-limit
                  abs-local-port
                  abs-compile-with-coverage-info
                  abs-default-resourcecost
                  abs-link-source-path
                  abs-directory))
               (indirect-variable 'abs-target-language)
               (indirect-variable 'abs-indent)
               (get 'abs-mode 'c-mode-prefix)
               (assq 'abs-mode auto-mode-alist)
               (assoc "\\.abs\\'"
                      flymake-proc-allowed-file-name-masks))"##;
    let expect = expect![[
        r#"OK ((java erlang maude prolog) (erlang "absc" nil "absfrontend.jar" nil nil nil nil 0 nil "~/.emacs.d/abs-mode" nil nil nil nil nil nil nil) (imenu-add-menubar-index abs-flymake-mode-on abs-maybe-yasnippet-mode-on) ((abs-backend nil (radio (const java) (const erlang) (const maude) (const prolog))) (abs-compiler-program nil string) (abs-output-directory nil directory) (abs-java-classpath nil string) (abs-use-timed-interpreter nil boolean) (abs-clock-limit nil (choice integer (const :tag "No limit" nil))) (abs-local-port nil integer) (abs-compile-with-coverage-info nil boolean) (abs-default-resourcecost nil integer) (abs-link-source-path nil (choice (const :tag "Do not link" nil) directory)) (abs-directory nil directory)) abs-backend c-basic-offset "abs-" nil ("\\.abs\\'" abs-flymake-init flymake-proc-simple-cleanup flymake-proc-get-real-file-name))"#
    ]];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_safe_and_risky_local_variable_policies_accept_exact_boundaries() {
    let elisp_form = r##"(let ((default-directory "/workspace/project/"))
               (list
                (mapcar
                 (lambda (value)
                   (funcall
                    (get 'abs-backend 'safe-local-variable)
                    value))
                 '(java erlang maude prolog rust nil))
                (mapcar
                 (lambda (value)
                   (funcall
                    (get
                     'abs-output-directory
                     'safe-local-variable)
                    value))
                 '("gen" "/workspace/project/out"
                   "/workspace/other"))
                (mapcar
                 (lambda (value)
                   (funcall
                    (get 'abs-clock-limit 'safe-local-variable)
                    value))
                 '(nil 0 12 "12"))
                (mapcar
                 (lambda (value)
                   (funcall
                    (get 'abs-input-files 'safe-local-variable)
                    value))
                 '(nil ("a.abs" "b.abs") ("a.abs" 1) "a.abs"))
                (mapcar
                 (lambda (value)
                   (funcall
                    (get
                     'abs-maude-output-file
                     'safe-local-variable)
                    value))
                 '("out.maude"
                   "/workspace/project/out.maude"
                   "/workspace/other/out.maude"))
                (mapcar
                 (lambda (value)
                   (funcall
                    (get
                     'abs-link-source-path
                     'safe-local-variable)
                    value))
                 '(nil "runtime" 7))
                (mapcar
                 (lambda (variable)
                   (get variable 'risky-local-variable))
                 '(abs-compiler-program
                   abs-java-classpath
                   abs-directory))))"##;
    let expect = expect![
        "OK (((java . #1=(erlang . #2=(maude . #3=(prolog)))) #1# #2# #3# nil nil) (t t nil) (t t t nil) (t t nil nil) (t t nil) (t t nil) (t t t))"
    ];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_language_tables_faces_regexps_and_initial_keymap_match_the_pin() {
    let elisp_form = r##"(list
               (mapcar
                #'facep
                '(abs-keyword-face
                  abs-constant-face
                  abs-function-name-face
                  abs-type-face
                  abs-variable-name-face))
               (mapcar
                (lambda (text)
                  (list
                   text
                   (string-match-p abs-keywords text)
                   (string-match-p abs-constants text)
                   (string-match-p abs--cid-regexp text)
                   (string-match-p abs--id-regexp text)))
                '("module" "True" "Thing" "value_name'"
                  "MODULE" "nullary"))
               (length abs-font-lock-keywords)
               (eq abs-font-lock-keywords-1
                   abs-font-lock-keywords)
               (eq abs-font-lock-keywords-2
                   abs-font-lock-keywords)
               (eq abs-font-lock-keywords-3
                   abs-font-lock-keywords)
               abs-imenu-syntax-alist
               (mapcar #'car abs-imenu-generic-expression)
               abs--outline-regexp
               (lookup-key abs-mode-map (kbd "C-c C-c"))
               (abbrev-expansion "else" abs-mode-abbrev-table))"##;
    let expect = expect![[
        r#"OK (([face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified]) (("module" 0 nil 0 0) ("True" nil 0 0 0) ("Thing" nil nil 0 0) ("value_name'" nil nil 0 0) ("MODULE" 0 nil 0 0) ("nullary" nil nil 0 0)) 6 t t t (("." . "_")) ("Deltas" "Functions" "Datatypes" "Exceptions" "Classes" "Interfaces" "Modules") "^\\(?:class\\|d\\(?:ata\\|e\\(?:f\\|lta\\)\\)\\|exception\\|\\(?:interfac\\|modul\\|typ\\)e\\)" comment-region "else")"#
    ]];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_read_backend_forwards_the_exact_completion_contract_and_interns_result() {
    let elisp_form = r##"(let ((abs-backend 'maude)
                    events)
               (cl-letf
                   (((symbol-function 'completing-read)
                     (lambda (&rest arguments)
                       (push arguments events)
                       "java")))
                 (list
                  (abs--read-backend)
                  (nreverse events))))"##;
    let expect =
        expect![[r#"OK (java (("Backend: " (java erlang maude prolog) nil t nil nil maude)))"#]];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_read_backend_returns_nil_for_a_noninterned_completion_result() {
    let elisp_form = r##"(cl-letf
               (((symbol-function 'completing-read)
                 (lambda (&rest _) "not-an-existing-symbol-43811")))
               (abs--read-backend))"##;
    let expect = expect!["OK nil"];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_yasnippet_helper_calls_only_when_the_feature_is_loaded() {
    let elisp_form = r##"(let ((answers '(nil t))
                    events)
               (cl-letf
                   (((symbol-function 'featurep)
                     (lambda (feature)
                       (push (list 'feature feature) events)
                       (pop answers)))
                    ((symbol-function 'yas-minor-mode-on)
                     (lambda ()
                       (push '(yas-on) events)
                       'enabled)))
                 (list
                  (abs-maybe-yasnippet-mode-on)
                  (abs-maybe-yasnippet-mode-on)
                  (nreverse events))))"##;
    let expect = expect!["OK (nil enabled ((feature yasnippet) (feature yasnippet) (yas-on)))"];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_missing_definition_navigation_signal_is_preserved() {
    let elisp_form = r##"(with-temp-buffer
               (set-syntax-table abs-mode-syntax-table)
               (insert "module Empty;\n")
               (goto-char (point-max))
               (abs-end-of-definition))"##;
    let expect = expect!["ERR (end-of-buffer)"];

    assert_abs_mode_signal_parity(elisp_form, expect);
}

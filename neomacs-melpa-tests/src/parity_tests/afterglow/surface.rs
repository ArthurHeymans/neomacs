use expect_test::expect;

use super::assert_afterglow_parity;

#[test]
fn afterglow_exact_pin_metadata_and_feature_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'afterglow
                      package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (package-desc-summary descriptor)
                (copy-tree
                 (package-desc-extras descriptor))
                (featurep 'afterglow)))"##;
    let expect = expect![[
        r#"OK (afterglow "20240312.953" ((emacs (26 1))) "Temporary Highlighting after Function Calls." ((:maintainers ("Ernest M. van der Linden" . "hello@ernestoz.com")) (:authors ("Ernest M. van der Linden" . "hello@ernestoz.com")) (:keywords "highlight" "line" "convenience" "evil") (:revdesc . "d90fcf4e5c8a") (:commit . "d90fcf4e5c8ac6f5bae2eb01dea32558b2b18fba") (:url . "https://github.com/ernstvanderlinden/emacs-afterglow")) t)"#
    ]];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_complete_callable_surface_has_expected_signatures_and_commands() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (help-function-arglist symbol t)
            (interactive-form symbol)
            (documentation symbol)))
         '(afterglow--add-trigger
           afterglow-add-trigger
           afterglow-add-triggers
           afterglow--remove-trigger
           afterglow-remove-trigger
           afterglow-remove-triggers
           afterglow--trigger-functions
           afterglow--advice-fn-symbol
           afterglow--advice-add
           afterglow--advice-remove
           afterglow--advice-remove-all
           afterglow--advices-remove-unused
           afterglow--advices-remove-all
           afterglow--reset
           afterglow--enable
           afterglow--disable
           afterglow--remove-overlays
           afterglow--apply-overlay
           afterglow--current-line-empty-p
           afterglow-mode))"##;
    let expect = expect![[
        r#"OK ((afterglow--add-trigger (fn args) nil "Set up a trigger function FN with properties specified in ARGS.") (afterglow-add-trigger (fn &rest args) nil "Add a trigger function FN to be advised with properties.\n\nExample:\n(afterglow-add-trigger =’evil-previous-visual-line\n                       :thing =’line :width 5 :duration 0.2)\nOptional argument ARGS adsf.") (afterglow-add-triggers (triggers) nil "Add multiple triggers at once.\n\nTRIGGERS is a list where each element is a list containing the\nfunction symbol followed by keyword arguments for additional\nproperties.\n\nExample 1:\n\n(afterglow-add-triggers\n =’((evil-previous-visual-line :thing line :width 5 :duration 0.2)\n   (evil-next-visual-line :thing line :width 5 :duration 0.2)\n   (previous-line :thing line :duration 0.2)\n   (next-line :thing line :duration 0.2)\n   (eval-buffer :thing window :duration 0.2)\n   (eval-defun :thing defun :duration 0.2)\n   (eval-expression :thing sexp :duration 1)\n   (eval-last-sexp :thing sexp :duration 1)\n   (my-function :thing =’my-region-function :duration 0.5\n                :face =’highlight)))\n\n;; Example 2: use let binding instead\n(let ((width 5)\n      (duration 0.3))\n  (afterglow-add-triggers\n   ‘((evil-previous-visual-line :thing line :width ,width\n                                :duration ,duration)\n     (evil-next-visual-line :thing line :width ,width\n                            :duration ,duration)\n     (previous-line :thing line :duration ,duration)\n     (next-line :thing line :duration ,duration)\n     (eval-buffer :thing window :duration ,duration)\n     (eval-defun :thing defun :duration ,duration)\n     (eval-region :thing region :duration ,duration\n                  :face (:background \"green\"))\n     (eval-last-sexp :thing sexp :duration ,duration))))") (afterglow--remove-trigger (fn) nil "Remove a single trigger and its associated advice.\nArgument FN .") (afterglow-remove-trigger (fn) nil "Remove a single trigger and its associated advice.\n\nExample:\n(afterglow-remove-trigger =’evil-previous-visual-line)\nArgument FN .") (afterglow-remove-triggers (fn-list) nil "Remove multiple triggers and their associated advice.\n  \nExample:\n\n(afterglow-add-triggers\n=’(evil-previous-visual-line\n    evil-previous-visual-line\n    evil-previous-line\n    evil-next-visual-line))\nArgument FN-LIST .") (afterglow--trigger-functions nil nil "Return a list of functions that have been added as triggers.") (afterglow--advice-fn-symbol (fn) nil "Generate an advice function name symbol for FN.") (afterglow--advice-add (fn advice-fn-symbol) nil "Add advice to FN with ADVICE-FN-SYMBOL and track it.") (afterglow--advice-remove (fn advice-fn-symbol) nil "Remove advice from FN identified by ADVICE-FN-SYMBOL and untrack.") (afterglow--advice-remove-all nil nil "Remove all advices added by Afterglow.") (afterglow--advices-remove-unused nil nil "Cleanup unused trigger functions and their advices.") (afterglow--advices-remove-all (unbind-functions-p) nil "Cleanup all advices added by Afterglow, optionally unbinding the functions.\nUNBIND-FUNCTIONS-P, when non-nil, also unbinds the advised functions.") (afterglow--reset nil nil "Disable and enable afterglow.") (afterglow--enable nil nil "Enable advising functions for highlighting.") (afterglow--disable nil nil "Disable advising functions and remove highlight.\n(afterglow-cleanup-advices nil) ; Remove advice, don’t unbind triggers\n(afterglow-cleanup-advices t) ; Remove advice and unbind triggers") (afterglow--remove-overlays nil nil "Remove all afterglow overlays from the current buffer.") (afterglow--apply-overlay (properties) nil "Apply an overlay based on PROPERTIES.") (afterglow--current-line-empty-p nil nil "True if the current line is empyty.") (afterglow-mode (&optional arg) (interactive (list (if current-prefix-arg (prefix-numeric-value current-prefix-arg) 'toggle))) "Toggle Afterglow mode.\n\nThis is a minor mode.  If called interactively, toggle the ‘afterglow\nmode’ mode.  If the prefix argument is positive, enable the mode, and if\nit is zero or negative, disable the mode.\n\nIf called from Lisp, toggle the mode if ARG is ‘toggle’.  Enable the\nmode if ARG is nil, omitted, or is a positive number.  Disable the mode\nif ARG is a negative number.\n\nTo check whether the minor mode is enabled in the current buffer,\nevaluate the variable ‘afterglow-mode’.\n\nThe mode’s hook is called both when the mode is enabled and when it is\ndisabled."))"#
    ]];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_variable_defaults_customization_and_initial_mutable_state_match() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (symbol-value symbol)
             (get symbol 'standard-value)
             (get symbol 'custom-type)
             (get symbol 'custom-group)
             (local-variable-p symbol)))
          '(afterglow-default-duration
            afterglow-default-face
            afterglow-mode))
         (list
          afterglow--temp-overlay
          (hash-table-p afterglow--triggers)
          (hash-table-test afterglow--triggers)
          (hash-table-count afterglow--triggers)
          afterglow--advised-functions
          afterglow-mode-hook)
         (assq 'afterglow-mode minor-mode-alist)
         (assq 'afterglow-mode minor-mode-map-alist)
         (get 'afterglow 'custom-group))"##;
    let expect = expect![[
        r#"OK (((afterglow-default-duration 1 (1) number nil nil) (afterglow-default-face hl-line ('hl-line) face nil nil) (afterglow-mode nil nil nil nil nil)) (nil t equal 0 nil nil) (afterglow-mode " afterglow") nil ((afterglow-default-duration custom-variable) (afterglow-default-face custom-variable)))"#
    ]];
    assert_afterglow_parity(elisp_form, expect);
}

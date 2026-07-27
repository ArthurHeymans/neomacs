use expect_test::expect;

use super::assert_aio_parity;

#[test]
fn aio_exact_pin_feature_errors_and_public_contract_match() {
    let elisp_form = r##"(list
                      (featurep 'aio)
                      (get 'aio-cancel 'error-conditions)
                      (get 'aio-cancel 'error-message)
                      (get 'aio-timeout 'error-conditions)
                      (get 'aio-timeout 'error-message)
                      (function-get 'aio-describe-function 'aio-defun-p)
                      (member #'aio-describe-function
                              help-fns-describe-function-functions)
                      (cl-find-if
                       (lambda (entry)
                         (and (listp entry)
                              (stringp (cadr entry))
                              (string-match-p "aio-defun" (cadr entry))))
                       lisp-imenu-generic-expression))"##;
    let expect = expect![[
        r#"OK (t (aio-cancel error) "Promise was canceled" (aio-timeout error) "Timeout was reached" nil (aio-describe-function cl--generic-describe) (nil "^\\s-*(aio-defun\\s-+\\(\\(?:\\w\\|\\s_\\|\\\\.\\)+\\)" 1))"#
    ]];
    assert_aio_parity(elisp_form, expect);
}

#[test]
fn aio_complete_callable_surface_arglists_macros_and_commands_match() {
    let elisp_form = r##"(let ((source
                           (file-truename (locate-library "aio")))
                          rows)
                      (mapatoms
                       (lambda (symbol)
                         (when (and
                                (string-prefix-p "aio" (symbol-name symbol))
                                (fboundp symbol)
                                (when-let ((file (symbol-file symbol 'defun)))
                                  (string=
                                   source
                                   (file-truename file))))
                           (push
                            (list symbol
                                  (condition-case nil
                                      (copy-tree
                                       (help-function-arglist symbol t))
                                    (error :unavailable))
                                  (macrop symbol)
                                  (commandp symbol))
                            rows))))
                      (sort rows
                            (lambda (left right)
                              (string-lessp
                               (symbol-name (car left))
                               (symbol-name (car right))))))"##;
    let expect = expect![
        "OK ((aio--make-select (&rest --cl-rest--) nil nil) (aio--make-select--cmacro (cl-whole &rest --cl-rest--) nil nil) (aio--make-sem (&rest --cl-rest--) nil nil) (aio--make-sem--cmacro (cl-whole &rest --cl-rest--) nil nil) (aio--queue-empty-p (queue) nil nil) (aio--queue-get (queue) nil nil) (aio--queue-put (queue element) nil nil) (aio--step (iter promise yield-result) nil nil) (aio-all (promises) t nil) (aio-await (expr) t nil) (aio-cancel (promise &optional reason) nil nil) (aio-catch (promise) nil nil) (aio-chain (expr) t nil) (aio-defun (name arglist &rest body) t nil) (aio-describe-function (function) nil nil) (aio-idle (seconds &optional result) nil nil) (aio-lambda (arglist &rest body) t nil) (aio-listen (promise callback) nil nil) (aio-make-callback (&rest --cl-rest--) nil nil) (aio-make-select (&optional promises) nil nil) (aio-promise (&rest --cl-rest--) nil nil) (aio-promise--cmacro (cl-whole &rest --cl-rest--) nil nil) (aio-promise-callbacks (x) nil nil) (aio-promise-callbacks--inliner (inline--form x) nil nil) (aio-promise-p (x) nil nil) (aio-promise-p--inliner (inline--form x) nil nil) (aio-promise-result (x) nil nil) (aio-promise-result--inliner (inline--form x) nil nil) (aio-resolve (promise value-function) nil nil) (aio-result (promise) nil nil) (aio-select (select) nil nil) (aio-select-add (select promise) nil nil) (aio-select-callback (x) nil nil) (aio-select-callback--inliner (inline--form x) nil nil) (aio-select-members (x) nil nil) (aio-select-members--inliner (inline--form x) nil nil) (aio-select-p (x) nil nil) (aio-select-p--inliner (inline--form x) nil nil) (aio-select-promises (select) nil nil) (aio-select-queue (x) nil nil) (aio-select-queue--inliner (inline--form x) nil nil) (aio-select-remove (select promise) nil nil) (aio-select-seen (x) nil nil) (aio-select-seen--inliner (inline--form x) nil nil) (aio-sem (init) nil nil) (aio-sem-p (x) nil nil) (aio-sem-p--inliner (inline--form x) nil nil) (aio-sem-post (sem) nil nil) (aio-sem-queue (x) nil nil) (aio-sem-queue--inliner (inline--form x) nil nil) (aio-sem-value (x) nil nil) (aio-sem-value--inliner (inline--form x) nil nil) (aio-sem-wait (sem) nil nil) (aio-sleep (seconds &optional result) nil nil) (aio-timeout (seconds) nil nil) (aio-url-retrieve (url &optional silent inhibit-cookies) nil nil) (aio-wait-for (promise) nil nil) (aio-with-async (&rest body) t nil) (aio-with-promise (promise &rest body) t nil))"
    ];
    assert_aio_parity(elisp_form, expect);
}

#[test]
fn aio_struct_defaults_and_async_definition_metadata_match_without_bytecode_identity() {
    let elisp_form = r##"(progn
                      (aio-defun aio-parity-command (foo &optional bar)
                        "A practical async command."
                        (declare (obsolete nil nil))
                        (interactive "sFoo: ")
                        (list foo bar))
                      (let ((promise (aio-promise))
                            (select (aio-make-select))
                            (sem (aio-sem 2)))
                        (list
                         (mapcar
                          (lambda (value)
                            (list
                             (type-of value)
                             (cond
                              ((aio-promise-p value)
                               (list (aio-result value)
                                     (aio-promise-callbacks value)))
                              ((aio-select-p value)
                               (list
                                (hash-table-count
                                 (aio-select-members value))
                                (hash-table-count
                                 (aio-select-seen value))
                                (aio-select-queue value)
                                (aio-select-callback value)))
                              ((aio-sem-p value)
                               (list
                                (aio-sem-value value)
                                (aio-sem-queue value))))))
                          (list promise select sem))
                         (commandp 'aio-parity-command)
                         (interactive-form 'aio-parity-command)
                         (documentation 'aio-parity-command)
                         (function-get
                          'aio-parity-command 'aio-defun-p)
                         (help-function-arglist
                          'aio-parity-command t))))"##;
    let expect = expect![[
        r#"OK (((aio-promise (nil nil)) (aio-select (0 0 (nil) nil)) (aio-sem (2 (nil)))) t (interactive "sFoo: ") "A practical async command." t (&rest args))"#
    ]];
    assert_aio_parity(elisp_form, expect);
}

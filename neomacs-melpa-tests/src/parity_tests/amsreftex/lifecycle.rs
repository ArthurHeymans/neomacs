use expect_test::expect;

use super::assert_amsreftex_parity;

#[test]
fn amsreftex_turn_on_installs_the_complete_advice_and_font_lock_lifecycle() {
    let elisp_form = r##"(unwind-protect
         (progn
           (amsreftex-turn-on)
           (list
            amsreftex-p
            (mapcar
             (lambda (pair)
               (list
                (car pair)
                (and
                 (advice-member-p
                  (cdr pair)
                  (car pair))
                 t)))
             '((reftex-locate-bibliography-files
                . amsreftex-advise-reftex-locate-bibliography-files)
               (reftex-parse-bibtex-entry
                . amsreftex-advise-reftex-parse-bibtex-entry)
               (reftex-get-crossref-alist
                . amsreftex-advise-reftex-get-crossref-alist)
               (reftex-extract-bib-entries
                . amsreftex-advise-reftex-extract-bib-entries)
               (reftex-extract-bib-entries-from-thebibliography
                . amsreftex-advise-reftex-extract-bib-entries-from-thebibliography)
               (reftex-pop-to-bibtex-entry
                . amsreftex-advise-reftex-pop-to-bibtex-entry)
               (reftex-echo-cite
                . amsreftex-set-last-arg-to-nil)
               (reftex-parse-from-file
                . amsreftex-parse-from-file)
               (reftex-bibtex-selection-callback
                . amsreftex-database-selection-callback)
               (reftex-end-of-bib-entry
                . amsreftex-end-of-bib-entry)))
            (member
             amsreftex-font-lock-keywords
             (cdr
              (assq
               'latex-mode
               font-lock-keywords-alist)))
            (member
             amsreftex-font-lock-keywords
             (cdr
              (assq
               'LaTeX-mode
               font-lock-keywords-alist)))))
       (when amsreftex-p
         (amsreftex-turn-off))
       (advice-remove
        'reftex-end-of-bib-entry
        #'amsreftex-end-of-bib-entry))"##;
    let expect = expect![
        "OK (t ((reftex-locate-bibliography-files t) (reftex-parse-bibtex-entry t) (reftex-get-crossref-alist t) (reftex-extract-bib-entries t) (reftex-extract-bib-entries-from-thebibliography t) (reftex-pop-to-bibtex-entry t) (reftex-echo-cite t) (reftex-parse-from-file t) (reftex-bibtex-selection-callback t) (reftex-end-of-bib-entry t)) nil nil)"
    ];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_repeated_turn_on_keeps_each_advice_function_single_and_effective() {
    let elisp_form = r##"(unwind-protect
         (progn
           (amsreftex-turn-on)
           (amsreftex-turn-on)
           (list
            amsreftex-p
            (mapcar
             (lambda (function)
               (let ((count 0))
                 (advice-mapc
                  (lambda (advice _properties)
                    (when
                        (memq
                         advice
                         '(amsreftex-advise-reftex-locate-bibliography-files
                           amsreftex-set-last-arg-to-nil
                           amsreftex-parse-from-file
                           amsreftex-database-selection-callback
                           amsreftex-end-of-bib-entry))
                      (setq count
                            (1+ count))))
                  function)
                 (list function count)))
             '(reftex-locate-bibliography-files
               reftex-echo-cite
               reftex-parse-from-file
               reftex-bibtex-selection-callback
               reftex-end-of-bib-entry))))
       (when amsreftex-p
         (amsreftex-turn-off))
       (advice-remove
        'reftex-end-of-bib-entry
        #'amsreftex-end-of-bib-entry))"##;
    let expect = expect![
        "OK (t ((reftex-locate-bibliography-files 1) (reftex-echo-cite 1) (reftex-parse-from-file 1) (reftex-bibtex-selection-callback 1) (reftex-end-of-bib-entry 1)))"
    ];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_turn_off_removes_declared_hooks_and_exposes_the_upstream_end_entry_trace() {
    let elisp_form = r##"(unwind-protect
         (progn
           (amsreftex-turn-on)
           (amsreftex-turn-off)
           (list
            amsreftex-p
            (mapcar
             (lambda (pair)
               (list
                (car pair)
                (and
                 (advice-member-p
                  (cdr pair)
                  (car pair))
                 t)))
             '((reftex-locate-bibliography-files
                . amsreftex-advise-reftex-locate-bibliography-files)
               (reftex-parse-bibtex-entry
                . amsreftex-advise-reftex-parse-bibtex-entry)
               (reftex-echo-cite
                . amsreftex-set-last-arg-to-nil)
               (reftex-parse-from-file
                . amsreftex-parse-from-file)
               (reftex-bibtex-selection-callback
                . amsreftex-database-selection-callback)
               (reftex-end-of-bib-entry
                . amsreftex-end-of-bib-entry)))
            (member
             amsreftex-font-lock-keywords
             (cdr
              (assq
               'latex-mode
               font-lock-keywords-alist)))))
       (advice-remove
        'reftex-end-of-bib-entry
        #'amsreftex-end-of-bib-entry))"##;
    let expect = expect![
        "OK (nil ((reftex-locate-bibliography-files nil) (reftex-parse-bibtex-entry nil) (reftex-echo-cite nil) (reftex-parse-from-file nil) (reftex-bibtex-selection-callback nil) (reftex-end-of-bib-entry t)) nil)"
    ];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_turn_off_when_inactive_signals_the_exact_user_error() {
    let elisp_form = r##"(condition-case error
         (amsreftex-turn-off)
       (error
        (list
         (car error)
         (cadr error)
         amsreftex-p)))"##;
    let expect = expect![[r#"OK (user-error "Amsreftex is not turned on!" nil)"#]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_generated_around_advice_dispatches_by_document_database_marker() {
    let elisp_form = r##"(let ((reftex-docstruct-symbol
                        'amsreftex-test-docstruct))
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'amsreftex-locate-bibliography-files)
                   (lambda (&rest arguments)
                     (cons
                      'amsrefs arguments))))
               (list
                (progn
                  (set
                   reftex-docstruct-symbol
                   '((database .
                      "amsrefs")))
                  (amsreftex-advise-reftex-locate-bibliography-files
                   (lambda (&rest arguments)
                     (cons
                      'vanilla arguments))
                   "/project"
                   '("alpha")))
                (progn
                  (set
                   reftex-docstruct-symbol
                   '((bib .
                      ("plain.bib"))))
                  (amsreftex-advise-reftex-locate-bibliography-files
                   (lambda (&rest arguments)
                     (cons
                      'vanilla arguments))
                   "/project"
                   '("alpha")))))
           (makunbound
            reftex-docstruct-symbol)))"##;
    let expect = expect![[r#"OK ((amsrefs "/project" ("alpha")) (vanilla "/project" ("alpha")))"#]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_macro_expansions_generate_exact_advice_definitions_and_operations() {
    let elisp_form = r##"(list
         (macroexpand-1
          '(amsreftex-advise-fn
            old-operation
            new-operation))
         (macroexpand-1
          '(amsreftex-add-advice
            old-operation))
         (macroexpand-1
          '(amsreftex-remove-advice
            old-operation)))"##;
    let expect = expect![[
        r#"OK ((defun amsreftex-advise-old-operation (old-fn &rest args) "If amsrefs databases are in use, replace OLD-FN with amsreftex equivalent.\n\nThe amseftex equivalent is:\nnew-operation\n\nIntended to advise:\nold-operation." (if (assq 'database (symbol-value reftex-docstruct-symbol)) (apply #'new-operation args) (apply old-fn args))) (advice-add 'old-operation :around #'amsreftex-advise-old-operation) (advice-remove 'old-operation #'amsreftex-advise-old-operation))"#
    ]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_font_lock_marks_real_bib_records_and_key_value_fields() {
    let elisp_form = r##"(unwind-protect
         (progn
           (amsreftex-turn-on)
           (with-temp-buffer
             (latex-mode)
             (insert
              "\\bib{lovelace}{article}{\n"
              " author={Lovelace, Ada}\n"
              " title={Notes}\n"
              "}\n")
             (font-lock-ensure)
             (mapcar
              (lambda (needle)
                (goto-char
                 (point-min))
                (search-forward needle)
                (list
                 needle
                 (get-text-property
                  (1-
                   (point))
                  'face)
                 (get-text-property
                  (match-beginning 0)
                  'face)))
              '("\\bib"
                "lovelace"
                "article"
                "author"
                "title"))))
       (when amsreftex-p
         (amsreftex-turn-off))
       (advice-remove
        'reftex-end-of-bib-entry
        #'amsreftex-end-of-bib-entry))"##;
    let expect = expect![[
        r#"OK (("\\bib" font-lock-keyword-face font-lock-keyword-face) ("lovelace" font-lock-type-face font-lock-type-face) ("article" font-lock-function-name-face font-lock-function-name-face) ("author" font-lock-variable-name-face font-lock-variable-name-face) ("title" font-lock-variable-name-face font-lock-variable-name-face))"#
    ]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_missing_database_entry_reports_key_and_preserves_origin_buffer() {
    let elisp_form = r##"(let ((database
                        (get-buffer-create
                         " *amsreftex-missing*"))
                       (origin
                        (current-buffer)))
         (unwind-protect
             (progn
               (with-current-buffer database
                 (erase-buffer)
                 (insert
                  "\\bib{present}{book}{title={Here}}")
                 (goto-char 4))
               (cl-letf
                   (((symbol-function
                      'reftex-get-file-buffer-force)
                     (lambda (&rest _)
                       database)))
                 (condition-case error
                     (amsreftex-pop-to-database-entry
                      "absent"
                      '("database.ltb")
                      nil nil nil t)
                   (error
                    (list
                     (car error)
                     (cadr error)
                     (eq
                      (current-buffer)
                      origin)
                     (with-current-buffer
                         database
                       (point)))))))
           (kill-buffer database)))"##;
    let expect = expect![[r#"OK (error "No amsrefs entry with citation key absent" t 4)"#]];
    assert_amsreftex_parity(elisp_form, expect);
}

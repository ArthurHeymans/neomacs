use expect_test::expect;

use super::{assert_amsreftex_autoload_parity, assert_amsreftex_parity};

#[test]
fn amsreftex_package_descriptor_records_exact_pin_dependency_summary_and_payload() {
    let elisp_form = r##"(let* ((description
                          (cadr
                           (assq 'amsreftex
                                 package-alist)))
               (directory
                (package-desc-dir description)))
         (list
          (package-version-join
           (package-desc-version description))
          (package-desc-reqs description)
          (package-desc-summary description)
          (package-desc-kind description)
          (sort
           (mapcar
            #'file-name-nondirectory
            (directory-files
             directory t
             "\\.el\\'"))
           #'string<)))"##;
    let expect = expect![[
        r#"OK ("20240512.1746" ((emacs (25 1))) "Add amsrefs bibliography support for reftex." nil ("amsreftex-autoloads.el" "amsreftex-pkg.el" "amsreftex.el"))"#
    ]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_autoloads_expose_only_the_two_interactive_entry_points() {
    let elisp_form = r##"(list
         (featurep 'amsreftex)
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (fboundp symbol)
             (autoloadp
              (and
               (fboundp symbol)
               (symbol-function symbol)))
             (commandp symbol)
             (help-function-arglist symbol t)))
          '(amsreftex-sort-bibliography
            amsreftex-turn-on
            amsreftex-turn-off
            amsreftex-parse-entry)))"##;
    let expect = expect![[
        r#"OK (nil ((amsreftex-sort-bibliography t t t "[Arg list not available until function definition is loaded.]") (amsreftex-turn-on t t t "[Arg list not available until function definition is loaded.]") (amsreftex-turn-off nil nil nil t) (amsreftex-parse-entry nil nil nil t)))"#
    ]];
    assert_amsreftex_autoload_parity(elisp_form, expect);
}

#[test]
fn amsreftex_source_registers_feature_commands_aliases_and_exact_arglists() {
    let elisp_form = r##"(list
         (featurep 'amsreftex)
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (fboundp symbol)
             (commandp symbol)
             (help-function-arglist symbol t)))
          '(amsreftex-locate-bibliography-files
            amsreftex-extract-fields
            amsreftex-parse-entry
            amsreftex-get-bib-field
            amsreftex-get-crossref-alist
            amsreftex--extract-entries
            amsreftex-extract-entries
            amsreftex-using-amsrefs-p
            amsreftex-parse-from-file
            amsreftex-database-selection-callback
            amsreftex-pop-to-database-entry
            amsreftex-end-of-bib-entry
            amsreftex-strip-LaTeX
            amsreftex-get-name-parts
            amsreftex-get-bib-name-list
            amsreftex-compare-by-field
            amsreftex-compare-lists
            amsreftex-compare-author
            amsreftex-compare-year
            amsreftex-sort-nextrecfn
            amsreftex-sort-endrecfn
            amsreftex-sort-startkeyfn
            amsreftex-sort-buffer-by
            amsreftex-sort-bibliography
            amsreftex-set-last-arg-to-nil
            amsreftex-turn-on
            amsreftex-turn-off
            turn-on-amsreftex
            turn-off-amsreftex)))"##;
    let expect = expect![
        "OK (t ((amsreftex-locate-bibliography-files t nil (master-dir &optional files)) (amsreftex-extract-fields t nil (blob &optional prefix)) (amsreftex-parse-entry t nil (entry &optional from to)) (amsreftex-get-bib-field t nil (field entry)) (amsreftex-get-crossref-alist t nil (entry)) (amsreftex--extract-entries t nil (re-list buffer)) (amsreftex-extract-entries t nil (buffers)) (amsreftex-using-amsrefs-p t nil nil) (amsreftex-parse-from-file t nil (file docstruct master-dir)) (amsreftex-database-selection-callback t nil (data _ignore no-revisit)) (amsreftex-pop-to-database-entry t nil (key file-list &optional mark-to-kill highlight _ return)) (amsreftex-end-of-bib-entry t nil (item)) (amsreftex-strip-LaTeX t nil (str)) (amsreftex-get-name-parts t nil (name)) (amsreftex-get-bib-name-list t nil (entry)) (amsreftex-compare-by-field t nil (e1 e2 field)) (amsreftex-compare-lists t nil (l1 l2 pred)) (amsreftex-compare-author t nil (e1 e2)) (amsreftex-compare-year t nil (e1 e2)) (amsreftex-sort-nextrecfn t nil nil) (amsreftex-sort-endrecfn t nil nil) (amsreftex-sort-startkeyfn t nil nil) (amsreftex-sort-buffer-by t nil (pred)) (amsreftex-sort-bibliography t t nil) (amsreftex-set-last-arg-to-nil t nil (args)) (amsreftex-turn-on t t nil) (amsreftex-turn-off t t nil) (turn-on-amsreftex t t nil) (turn-off-amsreftex t t nil)))"
    ];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_obsolete_aliases_retain_targets_versions_and_interactivity() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (indirect-function symbol)
            (get symbol 'byte-obsolete-info)
            (commandp symbol)))
         '(turn-on-amsreftex
           turn-off-amsreftex))"##;
    let expect = expect![[
        r#"OK ((turn-on-amsreftex #[nil ((advice-add 'reftex-locate-bibliography-files :around #'amsreftex-advise-reftex-locate-bibliography-files) (advice-add 'reftex-parse-bibtex-entry :around #'amsreftex-advise-reftex-parse-bibtex-entry) (advice-add 'reftex-get-crossref-alist :around #'amsreftex-advise-reftex-get-crossref-alist) (advice-add 'reftex-extract-bib-entries :around #'amsreftex-advise-reftex-extract-bib-entries) (advice-add 'reftex-extract-bib-entries-from-thebibliography :around #'amsreftex-advise-reftex-extract-bib-entries-from-thebibliography) (advice-add 'reftex-pop-to-bibtex-entry :around #'amsreftex-advise-reftex-pop-to-bibtex-entry) (advice-add 'reftex-echo-cite :filter-args #'amsreftex-set-last-arg-to-nil) (advice-add 'reftex-parse-from-file :override #'amsreftex-parse-from-file) (advice-add 'reftex-bibtex-selection-callback :override #'amsreftex-database-selection-callback) (advice-add 'reftex-end-of-bib-entry :override #'amsreftex-end-of-bib-entry) (font-lock-add-keywords 'latex-mode amsreftex-font-lock-keywords) (font-lock-add-keywords 'LaTeX-mode amsreftex-font-lock-keywords) (setq amsreftex-p t)) #1=(reftex--index-tags t) nil "Turn on amsreftex.\n\nThis advises several reftex functions to make them work with\namsrefs databases and installs some font-locking for \\bib\nmacros." nil] (amsreftex-turn-on nil "0.2") t) (turn-off-amsreftex #[nil ((if (not amsreftex-p) (user-error "Amsreftex is not turned on!") (advice-remove 'reftex-locate-bibliography-files #'amsreftex-advise-reftex-locate-bibliography-files) (advice-remove 'reftex-parse-bibtex-entry #'amsreftex-advise-reftex-parse-bibtex-entry) (advice-remove 'reftex-get-crossref-alist #'amsreftex-advise-reftex-get-crossref-alist) (advice-remove 'reftex-extract-bib-entries #'amsreftex-advise-reftex-extract-bib-entries) (advice-remove 'reftex-extract-bib-entries-from-thebibliography #'amsreftex-advise-reftex-extract-bib-entries-from-thebibliography) (advice-remove 'reftex-pop-to-bibtex-entry #'amsreftex-advise-reftex-pop-to-bibtex-entry) (advice-remove 'reftex-echo-cite #'amsreftex-set-last-arg-to-nil) (advice-remove 'reftex-end-of-bib-entry #'amsreftex-set-last-arg-to-nil) (advice-remove 'reftex-parse-from-file #'amsreftex-parse-from-file) (advice-remove 'reftex-bibtex-selection-callback #'amsreftex-database-selection-callback) (font-lock-remove-keywords 'latex-mode amsreftex-font-lock-keywords) (font-lock-remove-keywords 'LaTeX-mode amsreftex-font-lock-keywords) (setq amsreftex-p nil))) #1# nil "Turn off amsreftex, leaving almost no trace behind.\n\n We remove all advice added by `turn-on-amsrefs' and any font-locking installed." nil] (amsreftex-turn-off nil "0.2") t))"#
    ]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_load_registers_ltb_discovery_and_mode_associations_once() {
    let elisp_form = r##"(let ((source
                        (getenv
                         "NEOMACS_PACKAGE_SOURCE")))
         (load source nil t t)
         (list
          (cl-count
           '("\\.ltb\\'" . latex-mode)
           auto-mode-alist
           :test #'equal)
          (cl-count
           '("ltb" ".ltb")
           reftex-file-extensions
           :test #'equal)
          (cl-count
           '("ltb" . "kpsewhich %f.ltb")
           reftex-external-file-finders
           :test #'equal)
          reftex-ltbpath-environment-variables
          (mapcar
           (lambda (property)
             (get 'reftex-ltb-path property))
           '(status
             master-dir
             recursive-path
             rec-type))))"##;
    let expect = expect![[r#"OK (1 1 1 ("TEXINPUTS") (nil nil nil nil))"#]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_configuration_defaults_and_recognition_regexps_are_exact() {
    let elisp_form = r##"(list
         amsreftex-sort-fields
         amsreftex-sort-name-parts
         amsreftex-p
         amsreftex-bib-start-re
         amsreftex-kv-start-re
         amsreftex-biblist-start-re
         amsreftex-biblist-end-re
         amsreftex-font-lock-keywords
         (secure-hash
          'sha256
          (prin1-to-string
           amsreftex-font-lock-keywords)))"##;
    let expect = expect![[
        r#"OK (("author" "year") (last initial) nil "^[ \11]*\\(\\\\bib[*]?\\){\\(\\(?:\\w\\|\\s_\\)+\\)}{\\(\\w+\\)}{" "^[ \11]*\\(\\(?:\\w\\|-\\)+\\)[ \11\n\15]*=[ \11\n\15]*{" "^[^%\n\15]*\\\\begin{biblist}" "^[^%\n\15]*\\\\end{biblist}" (("^[ \11]*\\(\\\\bib[*]?\\){\\(\\(?:\\w\\|\\s_\\)+\\)}{\\(\\w+\\)}{" (1 font-lock-keyword-face) (2 font-lock-type-face) (3 font-lock-function-name-face)) ("^[ \11]*\\(\\(?:\\w\\|-\\)+\\)[ \11\n\15]*=[ \11\n\15]*{" (1 font-lock-variable-name-face))) "e82e7ad9b67c521da8f671918aed2c6b94039a622ab2d4d96e914e4d9f1b7a0d")"#
    ]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_generated_advice_functions_have_stable_callable_contracts() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (fboundp symbol)
            (help-function-arglist symbol t)
            (documentation symbol)))
         '(amsreftex-advise-reftex-locate-bibliography-files
           amsreftex-advise-reftex-parse-bibtex-entry
           amsreftex-advise-reftex-get-crossref-alist
           amsreftex-advise-reftex-extract-bib-entries
           amsreftex-advise-reftex-extract-bib-entries-from-thebibliography
           amsreftex-advise-reftex-pop-to-bibtex-entry))"##;
    let expect = expect![[
        r#"OK ((amsreftex-advise-reftex-locate-bibliography-files t #1=(old-fn &rest args) "If amsrefs databases are in use, replace OLD-FN with amsreftex equivalent.\n\nThe amseftex equivalent is:\namsreftex-locate-bibliography-files\n\nIntended to advise:\nreftex-locate-bibliography-files.") (amsreftex-advise-reftex-parse-bibtex-entry t #1# "If amsrefs databases are in use, replace OLD-FN with amsreftex equivalent.\n\nThe amseftex equivalent is:\namsreftex-parse-entry\n\nIntended to advise:\nreftex-parse-bibtex-entry.") (amsreftex-advise-reftex-get-crossref-alist t #1# "If amsrefs databases are in use, replace OLD-FN with amsreftex equivalent.\n\nThe amseftex equivalent is:\namsreftex-get-crossref-alist\n\nIntended to advise:\nreftex-get-crossref-alist.") (amsreftex-advise-reftex-extract-bib-entries t #1# "If amsrefs databases are in use, replace OLD-FN with amsreftex equivalent.\n\nThe amseftex equivalent is:\namsreftex-extract-entries\n\nIntended to advise:\nreftex-extract-bib-entries.") (amsreftex-advise-reftex-extract-bib-entries-from-thebibliography t #1# "If amsrefs databases are in use, replace OLD-FN with amsreftex equivalent.\n\nThe amseftex equivalent is:\namsreftex-extract-entries\n\nIntended to advise:\nreftex-extract-bib-entries-from-thebibliography.") (amsreftex-advise-reftex-pop-to-bibtex-entry t #1# "If amsrefs databases are in use, replace OLD-FN with amsreftex equivalent.\n\nThe amseftex equivalent is:\namsreftex-pop-to-database-entry\n\nIntended to advise:\nreftex-pop-to-bibtex-entry."))"#
    ]];
    assert_amsreftex_parity(elisp_form, expect);
}

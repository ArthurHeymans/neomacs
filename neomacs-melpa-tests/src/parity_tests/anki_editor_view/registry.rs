use expect_test::expect;

use super::{assert_anki_editor_view_autoload_parity, assert_anki_editor_view_parity};

#[test]
fn anki_editor_view_registers_exact_feature_group_custom_and_function_surface() {
    let elisp_form = r##"(list
         (featurep 'anki-editor-view)
         (get 'anki-editor-view
              'custom-group)
         (get 'anki-editor-view
              'group-documentation)
         (custom-variable-p
          'anki-editor-view-files)
         (get 'anki-editor-view-files
              'custom-type)
         (get 'anki-editor-view-files
              'standard-value)
         anki-editor-view-files
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (fboundp symbol)
             (help-function-arglist
              symbol t)
             (documentation symbol)))
          '(anki-editor-view--ripgrep-find-locations
            anki-editor-view--open-anki-note)))"##;
    let expect = expect![[
        r#"OK (t ((anki-editor-view-files custom-variable)) "Open anki notes in Emacs from Anki." #1=((funcall #'#[nil ((list org-directory)) (t)])) (repeat :tag "List of files and directories" file) #1# ("~/org") ((anki-editor-view--ripgrep-find-locations t (search-string directories) "Search for all locations of SEARCH-STRING in DIRECTORIES with ripgrep.\n\nReturns a list of alists in the form ‘((file . \"…\") (line . …))’") (anki-editor-view--open-anki-note t (info) "Open the Anki note with the id specified in the plist INFO.")))"#
    ]];
    assert_anki_editor_view_parity(elisp_form, expect);
}

#[test]
fn anki_editor_view_registers_exact_org_protocol_handler_once() {
    let elisp_form = r##"(let ((entries
                        (seq-filter
                         (lambda (entry)
                           (equal
                            (plist-get
                             (cdr entry)
                             :protocol)
                            "anki-editor-view"))
                         org-protocol-protocol-alist)))
         (list
          (length entries)
          entries
          (mapcar
           (lambda (entry)
             (list
              (car entry)
              (plist-get
               (cdr entry)
               :protocol)
              (plist-get
               (cdr entry)
               :function)
              (functionp
               (plist-get
                (cdr entry)
                :function))))
           entries)))"##;
    let expect = expect![[
        r#"OK (1 (("anki-editor-view" :protocol "anki-editor-view" :function anki-editor-view--open-anki-note)) (("anki-editor-view" "anki-editor-view" anki-editor-view--open-anki-note t)))"#
    ]];
    assert_anki_editor_view_parity(elisp_form, expect);
}

#[test]
fn anki_editor_view_descriptor_records_exact_pin_dependencies_and_payload() {
    let elisp_form = r##"(let* ((description
                          (cadr
                           (assq
                            'anki-editor-view
                            package-alist)))
               (directory
                (package-desc-dir description)))
         (list
          (package-desc-name description)
          (package-version-join
           (package-desc-version description))
          (package-desc-kind description)
          (package-desc-summary description)
          (package-desc-reqs description)
          (sort
           (mapcar #'file-name-nondirectory
                   (directory-files
                    directory t
                    "\\`[^.]"))
           #'string<)))"##;
    let expect = expect![[
        r#"OK (anki-editor-view "20230807.806" nil "Open anki-editor notes from Anki." ((emacs (29 1))) ("README-elpa" "anki-editor-view-autoloads.el" "anki-editor-view-pkg.el" "anki-editor-view.el" "anki-editor-view.elc"))"#
    ]];
    assert_anki_editor_view_parity(elisp_form, expect);
}

#[test]
fn anki_editor_view_autoload_only_registers_package_directory() {
    let elisp_form = r##"(let* ((source
                          (getenv
                           "NEOMACS_PACKAGE_SOURCE"))
               (directory
                (file-name-directory source)))
         (list
          (featurep 'anki-editor-view)
          (fboundp
           'anki-editor-view--ripgrep-find-locations)
          (fboundp
           'anki-editor-view--open-anki-note)
          (member directory load-path)
          (cl-count
           directory load-path
           :test #'equal)))"##;
    let expect = expect!["OK (nil nil nil nil 0)"];
    assert_anki_editor_view_autoload_parity(elisp_form, expect);
}

#[test]
fn anki_editor_view_source_reload_deduplicates_protocol_entry() {
    let elisp_form = r##"(let ((source
                        (getenv
                         "NEOMACS_PACKAGE_SOURCE")))
         (load source nil t t)
         (load source nil t t)
         (let ((entries
                (seq-filter
                 (lambda (entry)
                   (equal
                    (plist-get
                     (cdr entry)
                     :protocol)
                    "anki-editor-view"))
                 org-protocol-protocol-alist)))
           (list
            (length entries)
            entries
            (featurep
             'anki-editor-view)
            (help-function-arglist
             'anki-editor-view--open-anki-note
             t))))"##;
    let expect = expect![[
        r#"OK (1 (("anki-editor-view" :protocol "anki-editor-view" :function anki-editor-view--open-anki-note)) t (info))"#
    ]];
    assert_anki_editor_view_parity(elisp_form, expect);
}

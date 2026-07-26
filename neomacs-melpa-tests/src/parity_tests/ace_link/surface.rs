use super::assert_ace_link_parity;
use expect_test::expect;

#[test]
fn ace_link_public_callable_metadata_matches() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (help-function-arglist symbol t)
            (commandp symbol)
            (interactive-form symbol)
            (documentation symbol)
            (file-name-nondirectory
             (symbol-file symbol 'defun))))
         '(ace-link
           ace-link-info
           ace-link-help
           ace-link-commit
           ace-link-man
           ace-link-woman
           ace-link-eww
           ace-link-w3m
           ace-link-compilation
           ace-link-gnus
           ace-link-mu4e
           ace-link-notmuch-plain
           ace-link-notmuch-html
           ace-link-notmuch
           ace-link-widget
           ace-link-org
           ace-link-org-agenda
           ace-link-xref
           ace-link-custom
           ace-link-addr
           ace-link-sldb
           ace-link-slime-xref
           ace-link-slime-inspector
           ace-link-indium-inspector
           ace-link-indium-debugger-frames
           ace-link-cider-inspector
           ace-link-setup-default))"##;
    let expect = expect![[
        r#"OK ((ace-link nil t (interactive nil) "Call the ace link function for the current ‘major-mode’" "ace-link.el") (ace-link-info nil t (interactive nil) "Open a visible link in an ‘Info-mode’ buffer." "ace-link.el") (ace-link-help nil t (interactive nil) "Open a visible link in a ‘help-mode’ buffer." "ace-link.el") (ace-link-commit nil t (interactive nil) "Open an issue link in the browser." "ace-link.el") (ace-link-man nil t (interactive nil) "Open a visible link in a ‘man’ buffer." "ace-link.el") (ace-link-woman nil t (interactive nil) "Open a visible link in a ‘woman-mode’ buffer." "ace-link.el") (ace-link-eww (&optional external) t (interactive "P") "Open a visible link in an ‘eww-mode’ buffer.\nIf EXTERNAL is single prefix, browse the URL using\n‘browse-url-secondary-browser-function’.\n\nIf EXTERNAL is double prefix, browse in new buffer." "ace-link.el") (ace-link-w3m nil t (interactive nil) "Open a visible link in an ‘w3m-mode’ buffer." "ace-link.el") (ace-link-compilation nil t (interactive nil) "Open a visible link in a ‘compilation-mode’ buffer." "ace-link.el") (ace-link-gnus nil t (interactive nil) "Open a visible link in a ‘gnus-article-mode’ buffer." "ace-link.el") (ace-link-mu4e nil t (interactive nil) "Open a visible link in an ‘mu4e-view-mode’ buffer." "ace-link.el") (ace-link-notmuch-plain nil t (interactive nil) "Open a visible link in a ‘notmuch-show’ buffer.\nOnly consider the ’text/plain’ portion of the buffer." "ace-link.el") (ace-link-notmuch-html nil t (interactive nil) "Open a visible link in a ‘notmuch-show’ buffer.\nOnly consider the ’text/html’ portion of the buffer." "ace-link.el") (ace-link-notmuch nil t (interactive nil) "Open a visible link in ‘notmuch-show’ buffer.\nConsider both the links in ’text/plain’ and ’text/html’." "ace-link.el") (ace-link-widget nil t (interactive nil) "Open or go to a visible widget." "ace-link.el") (ace-link-org nil t (interactive nil) "Open a visible link in an ‘org-mode’ buffer." "ace-link.el") (ace-link-org-agenda nil t (interactive nil) "Open a visible link in an ‘org-mode-agenda’ buffer." "ace-link.el") (ace-link-xref nil t (interactive nil) "Open a visible link in an ‘xref--xref-buffer-mode’ buffer." "ace-link.el") (ace-link-custom nil t (interactive nil) "Open a visible link in an ‘Custom-mode’ buffer." "ace-link.el") (ace-link-addr nil t (interactive nil) "Open a visible link in a goto-address buffer." "ace-link.el") (ace-link-sldb nil t (interactive nil) "Interact with a frame or local variable in a sldb buffer." "ace-link.el") (ace-link-slime-xref nil t (interactive nil) "Open a visible link in an ‘slime-xref-mode’ buffer." "ace-link.el") (ace-link-slime-inspector nil t (interactive nil) "Interact with a value, an action or a range button in a\n‘slime-inspector-mode’ buffer." "ace-link.el") (ace-link-indium-inspector nil t (interactive nil) "Interact with a value, an action or a range button in a\n‘indium-inspector-mode’ buffer." "ace-link.el") (ace-link-indium-debugger-frames nil t (interactive nil) "Interact with a value, an action or a range button in a\n‘indium-debugger-frames-mode’ buffer." "ace-link.el") (ace-link-cider-inspector nil t (interactive nil) "Open a visible link in a ‘cider-inspector-mode’ buffer." "ace-link.el") (ace-link-setup-default (&optional key) nil nil "Bind KEY to appropriate functions in appropriate keymaps." "ace-link.el"))"#
    ]];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_internal_callable_arities_interactivity_and_source_ownership_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (help-function-arglist symbol t)
            (commandp symbol)
            (interactive-form symbol)
            (file-name-nondirectory
             (symbol-file symbol 'defun))))
         '(ace-link--info-action
           ace-link--info-current
           ace-link--info-collect
           ace-link--help-action
           ace-link--help-collect
           ace-link--man-action
           ace-link--man-collect
           ace-link--woman-action
           ace-link--woman-collect
           ace-link--eww-action
           ace-link--eww-collect
           ace-link--w3m-action
           ace-link--w3m-collect
           ace-link--compilation-action
           ace-link--gnus-action
           ace-link--gnus-collect
           ace-link--email-view-plain-collect
           ace-link--email-view-next-link
           ace-link--email-view-end-of-link
           ace-link--email-view-html-collect
           ace-link--mu4e-action
           ace-link--notmuch-plain-action
           ace-link--notmuch-html-action
           ace-link--notmuch-collect
           ace-link--widget-action
           ace-link--widget-collect
           ace-link--org-action
           ace-link--org-collect
           ace-link--org-agenda-action
           ace-link--org-agenda-collect
           ace-link--xref-action
           ace-link--xref-collect
           ace-link--custom-action
           ace-link--custom-collect
           ace-link--addr-action
           ace-link--addr-collect
           ace-link--sldb-action
           ace-link--sldb-collect
           ace-link--slime-xref-action
           ace-link--slime-xref-collect
           ace-link--slime-inspector-action
           ace-link--slime-inspector-collect
           ace-link--indium-inspector-action
           ace-link--indium-inspector-collect
           ace-link--indium-debugger-frames-action
           ace-link--indium-debugger-frames-collect
           ace-link--cider-inspector-collect
           ace-link--cider-inspector-action))"##;
    let expect = expect![[
        r#"OK ((ace-link--info-action (pt) nil nil "ace-link.el") (ace-link--info-current nil nil nil "ace-link.el") (ace-link--info-collect nil nil nil "ace-link.el") (ace-link--help-action (pt) nil nil "ace-link.el") (ace-link--help-collect nil nil nil "ace-link.el") (ace-link--man-action (pt) nil nil "ace-link.el") (ace-link--man-collect nil nil nil "ace-link.el") (ace-link--woman-action (pt) nil nil "ace-link.el") (ace-link--woman-collect nil nil nil "ace-link.el") (ace-link--eww-action (pt external) nil nil "ace-link.el") (ace-link--eww-collect (&optional property) nil nil "ace-link.el") (ace-link--w3m-action (pt) nil nil "ace-link.el") (ace-link--w3m-collect nil nil nil "ace-link.el") (ace-link--compilation-action (pt) nil nil "ace-link.el") (ace-link--gnus-action (pt) nil nil "ace-link.el") (ace-link--gnus-collect nil nil nil "ace-link.el") (ace-link--email-view-plain-collect nil nil nil "ace-link.el") (ace-link--email-view-next-link (pos &optional mu4e) nil nil "ace-link.el") (ace-link--email-view-end-of-link (link) nil nil "ace-link.el") (ace-link--email-view-html-collect (&optional mu4e) nil nil "ace-link.el") (ace-link--mu4e-action (pt) nil nil "ace-link.el") (ace-link--notmuch-plain-action (pt) nil nil "ace-link.el") (ace-link--notmuch-html-action (pt) nil nil "ace-link.el") (ace-link--notmuch-collect nil nil nil "ace-link.el") (ace-link--widget-action (pt) nil nil "ace-link.el") (ace-link--widget-collect nil nil nil "ace-link.el") (ace-link--org-action (pt) nil nil "ace-link.el") (ace-link--org-collect nil nil nil "ace-link.el") (ace-link--org-agenda-action (pt) nil nil "ace-link.el") (ace-link--org-agenda-collect nil nil nil "ace-link.el") (ace-link--xref-action (pt) nil nil "ace-link.el") (ace-link--xref-collect nil nil nil "ace-link.el") (ace-link--custom-action (pt) nil nil "ace-link.el") (ace-link--custom-collect nil nil nil "ace-link.el") (ace-link--addr-action (pt) nil nil "ace-link.el") (ace-link--addr-collect nil nil nil "ace-link.el") (ace-link--sldb-action (pt) nil nil "ace-link.el") (ace-link--sldb-collect nil nil nil "ace-link.el") (ace-link--slime-xref-action (pt) nil nil "ace-link.el") (ace-link--slime-xref-collect nil nil nil "ace-link.el") (ace-link--slime-inspector-action (pt) nil nil "ace-link.el") (ace-link--slime-inspector-collect nil nil nil "ace-link.el") (ace-link--indium-inspector-action (pt) nil nil "ace-link.el") (ace-link--indium-inspector-collect nil nil nil "ace-link.el") (ace-link--indium-debugger-frames-action (pt) nil nil "ace-link.el") (ace-link--indium-debugger-frames-collect nil nil nil "ace-link.el") (ace-link--cider-inspector-collect nil nil nil "ace-link.el") (ace-link--cider-inspector-action (pt) nil nil "ace-link.el"))"#
    ]];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_variable_defaults_and_full_mode_dispatch_tables_match() {
    let elisp_form = r##"(list
         ace-link-fallback-function
         ace-link-minor-mode-actions
         ace-link-major-mode-actions
         ace-link--sldb-action-fn)"##;
    let expect = expect![
        "OK (nil ((ace-link-compilation compilation-shell-minor-mode)) ((ace-link-org org-mode erc-mode elfeed-show-mode term-mode vterm-mode eshell-mode telega-chat-mode org-roam-mode) (ace-link-org-agenda org-agenda-mode) (ace-link-info Info-mode) (ace-link-help help-mode package-menu-mode geiser-doc-mode elbank-report-mode elbank-overview-mode slime-trace-dialog-mode helpful-mode) (ace-link-man Man-mode) (ace-link-woman woman-mode) (ace-link-eww eww-mode) (ace-link-w3m w3m-mode) (ace-link-compilation compilation-mode grep-mode) (ace-link-gnus gnus-article-mode gnus-summary-mode) (ace-link-mu4e mu4e-view-mode) (ace-link-notmuch notmuch-show-mode) (ace-link-custom Custom-mode) (ace-link-sldb sldb-mode) (ace-link-slime-xref slime-xref-mode) (ace-link-slime-inspector slime-inspector-mode) (ace-link-indium-inspector indium-inspector-mode) (ace-link-indium-debugger-frames indium-debugger-frames-mode) (ace-link-commit magit-commit-mode) (ace-link-cider-inspector cider-inspector-mode)) sldb-default-action)"
    ];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_variable_metadata_and_source_ownership_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (special-variable-p symbol)
            (get symbol 'standard-value)
            (documentation-property
             symbol
             'variable-documentation
             t)
            (file-name-nondirectory
             (symbol-file symbol 'defvar))))
         '(ace-link-fallback-function
           ace-link-minor-mode-actions
           ace-link-major-mode-actions
           ace-link--sldb-action-fn))"##;
    let expect = expect![[
        r#"OK ((ace-link-fallback-function t nil "When non-nil, called by `ace-link' when `major-mode' isn't recognized." "ace-link.el") (ace-link-minor-mode-actions t nil "Mapping of minor modes to ace-link actions." "ace-link.el") (ace-link-major-mode-actions t nil "Reverse mapping of `major-mode' to ace-link actions." "ace-link.el") (ace-link--sldb-action-fn t nil "Function to call after jump." "ace-link.el"))"#
    ]];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_source_reload_preserves_all_prebound_package_variables() {
    let elisp_form = r##"(let ((path
              (symbol-file
               'ace-link
               'defun)))
         (setq ace-link-fallback-function
               'prebound-fallback)
         (setq ace-link-minor-mode-actions
               'prebound-minor)
         (setq ace-link-major-mode-actions
               'prebound-major)
         (setq ace-link--sldb-action-fn
               'prebound-sldb)
         (load path nil t)
         (list
          ace-link-fallback-function
          ace-link-minor-mode-actions
          ace-link-major-mode-actions
          ace-link--sldb-action-fn
          (featurep 'ace-link)))"##;
    let expect = expect!["OK (prebound-fallback prebound-minor prebound-major prebound-sldb t)"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_packaged_source_descriptor_autoload_and_readme_assets_match() {
    let elisp_form = r##"(let* ((descriptor
                      (cadr
                       (assq
                        'ace-link
                        package-alist)))
                     (directory
                      (package-desc-dir descriptor)))
               (mapcar
                (lambda (name)
                  (let ((path
                         (expand-file-name
                          name
                          directory)))
                    (with-temp-buffer
                      (set-buffer-multibyte nil)
                      (insert-file-contents-literally path)
                      (list
                       name
                       (buffer-size)
                       (secure-hash
                        'sha256
                        (current-buffer))))))
                '("ace-link.el"
                  "ace-link-pkg.el"
                  "ace-link-autoloads.el"
                  "README-elpa")))"##;
    let expect = expect![[
        r#"OK (("ace-link.el" 37820 "6d7b37842e4c74a2b7c8858516391880da1fd3fc8f1256820d3d8e486c598d1a") ("ace-link-pkg.el" 420 "32c3632cb9805f6e93e76a36137fbe5f936ca5440339e6430d8c41356d85cf6f") ("ace-link-autoloads.el" 3592 "75c604383a274c0213b232d2595b2ee005b287aaedb4deaf2858d3e7dabada57") ("README-elpa" 511 "7790e04b2319df10a3b8bbf7ec097972d03e9d5e8d597e8ce37d9818309652b6"))"#
    ]];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_installation_produces_local_byte_compilation_artifact() {
    let elisp_form = r##"(let* ((descriptor
                      (cadr
                       (assq
                        'ace-link
                        package-alist)))
                     (directory
                      (package-desc-dir descriptor))
                     (path
                      (expand-file-name
                       "ace-link.elc"
                       directory)))
               (list
                (file-exists-p path)
                (file-regular-p path)
                (> (file-attribute-size
                    (file-attributes path))
                   0)))"##;
    let expect = expect!["OK (t t t)"];
    assert_ace_link_parity(elisp_form, expect);
}

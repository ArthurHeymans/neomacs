use super::assert_ace_link_autoload_parity;
use expect_test::expect;

#[test]
fn ace_link_autoload_file_provides_only_its_autoload_feature_before_dispatch() {
    let elisp_form = r##"(list
         (featurep 'ace-link-autoloads)
         (featurep 'ace-link)
         (featurep 'avy)
         (boundp 'ace-link-major-mode-actions))"##;
    let expect = expect!["OK (t nil nil nil)"];
    assert_ace_link_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_link_command_autoload_descriptors_match_exact_files_docs_and_interactivity() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (let ((definition
                  (symbol-function symbol)))
             (list
              symbol
              (autoloadp definition)
              (nth 1 definition)
              (nth 2 definition)
              (nth 3 definition)
              (nth 4 definition)
              (commandp symbol))))
         '(ace-link
           ace-link-info
           ace-link-help
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
           ace-link-cider-inspector))"##;
    let expect = expect![[
        r#"OK ((ace-link t "ace-link" "Call the ace link function for the current `major-mode'" t nil t) (ace-link-info t "ace-link" "Open a visible link in an `Info-mode' buffer." t nil t) (ace-link-help t "ace-link" "Open a visible link in a `help-mode' buffer." t nil t) (ace-link-man t "ace-link" "Open a visible link in a `man' buffer." t nil t) (ace-link-woman t "ace-link" "Open a visible link in a `woman-mode' buffer." t nil t) (ace-link-eww t "ace-link" "Open a visible link in an `eww-mode' buffer.\nIf EXTERNAL is single prefix, browse the URL using\n`browse-url-secondary-browser-function'.\n\nIf EXTERNAL is double prefix, browse in new buffer.\n\n(fn &optional EXTERNAL)" t nil t) (ace-link-w3m t "ace-link" "Open a visible link in an `w3m-mode' buffer." t nil t) (ace-link-compilation t "ace-link" "Open a visible link in a `compilation-mode' buffer." t nil t) (ace-link-gnus t "ace-link" "Open a visible link in a `gnus-article-mode' buffer." t nil t) (ace-link-mu4e t "ace-link" "Open a visible link in an `mu4e-view-mode' buffer." t nil t) (ace-link-notmuch-plain t "ace-link" "Open a visible link in a `notmuch-show' buffer.\nOnly consider the 'text/plain' portion of the buffer." t nil t) (ace-link-notmuch-html t "ace-link" "Open a visible link in a `notmuch-show' buffer.\nOnly consider the 'text/html' portion of the buffer." t nil t) (ace-link-notmuch t "ace-link" "Open a visible link in `notmuch-show' buffer.\nConsider both the links in 'text/plain' and 'text/html'." t nil t) (ace-link-widget t "ace-link" "Open or go to a visible widget." t nil t) (ace-link-org t "ace-link" "Open a visible link in an `org-mode' buffer." t nil t) (ace-link-org-agenda t "ace-link" "Open a visible link in an `org-mode-agenda' buffer." t nil t) (ace-link-xref t "ace-link" "Open a visible link in an `xref--xref-buffer-mode' buffer." t nil t) (ace-link-custom t "ace-link" "Open a visible link in an `Custom-mode' buffer." t nil t) (ace-link-addr t "ace-link" "Open a visible link in a goto-address buffer." t nil t) (ace-link-sldb t "ace-link" "Interact with a frame or local variable in a sldb buffer." t nil t) (ace-link-slime-xref t "ace-link" "Open a visible link in an `slime-xref-mode' buffer." t nil t) (ace-link-slime-inspector t "ace-link" "Interact with a value, an action or a range button in a\n`slime-inspector-mode' buffer." t nil t) (ace-link-indium-inspector t "ace-link" "Interact with a value, an action or a range button in a\n`indium-inspector-mode' buffer." t nil t) (ace-link-indium-debugger-frames t "ace-link" "Interact with a value, an action or a range button in a\n`indium-debugger-frames-mode' buffer." t nil t) (ace-link-cider-inspector t "ace-link" "Open a visible link in a `cider-inspector-mode' buffer." t nil t))"#
    ]];
    assert_ace_link_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_link_setup_default_is_a_noninteractive_autoload_with_optional_key() {
    let elisp_form = r##"(let ((definition
              (symbol-function
               'ace-link-setup-default)))
         (list
          (autoloadp definition)
          (nth 1 definition)
          (nth 2 definition)
          (nth 3 definition)
          (commandp 'ace-link-setup-default)))"##;
    let expect = expect![[
        r#"OK (t "ace-link" "Bind KEY to appropriate functions in appropriate keymaps.\n\n(fn &optional KEY)" nil nil)"#
    ]];
    assert_ace_link_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_link_autoload_dispatch_loads_package_and_preserves_prebound_fallback() {
    let elisp_form = r##"(progn
         (setq ace-link-fallback-function
               (lambda ()
                 'fallback-result))
         (let ((major-mode
                'ace-link-fixture-mode))
         (list
          (ace-link)
          (featurep 'ace-link)
          (featurep 'avy)
          (functionp
           ace-link-fallback-function))))"##;
    let expect = expect!["OK (fallback-result t t t)"];
    assert_ace_link_autoload_parity(elisp_form, expect);
}

use expect_test::expect;

use super::assert_ac_html_csswatcher_parity;

#[test]
fn ac_html_csswatcher_exact_pin_dependencies_defaults_and_locality_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ac-html-csswatcher
                      package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (mapcar
                 #'featurep
                 '(ac-html-csswatcher
                   web-completion-data))
                ac-html-csswatcher-source-dir
                ac-html-csswatcher-command
                ac-html-csswatcher-command-args
                ac-html-csswatcher-debug
                ac-html-csswatcher-log-buf-name
                (get
                 'ac-html-csswatcher-command
                 'custom-type)
                (get
                 'ac-html-csswatcher-command-args
                 'custom-type)
                (local-variable-if-set-p
                 'ac-html-csswatcher-source-dir)))"##;
    let expect = expect![[
        r#"OK (ac-html-csswatcher "20151208.2113" ((web-completion-data (0 1))) (t t) nil "csswatcher" nil nil "*ac-html-csswatcher debug*" string list t)"#
    ]];

    assert_ac_html_csswatcher_parity(elisp_form, expect);
}

#[test]
fn ac_html_csswatcher_function_surface_aliases_arities_interactivity_and_docs_match() {
    let elisp_form = r##"(mapcar
               (lambda (function)
                 (list
                  function
                  (functionp function)
                  (help-function-arglist
                   function t)
                  (interactive-form function)
                  (documentation function t)
                  (let ((definition
                         (symbol-function
                          function)))
                    (cond
                     ((symbolp definition)
                      definition)
                     ((byte-code-function-p
                       definition)
                      'byte-code)
                     (t 'interpreted)))))
               '(ac-html-csswatcher-log-buf
                 AC-HTML-CSSWATCHER-LOG
                 ac-html-csswatcher-setup-html-stuff-async
                 ac-html-csswatcher-refresh
                 company-web-csswatcher-refresh
                 ac-html-csswatcher+
                 company-web-csswatcher+
                 ac-html-csswatcher-setup
                 company-web-csswatcher-setup))"##;
    let expect = expect![[
        r#"OK ((ac-html-csswatcher-log-buf t nil nil nil interpreted) (AC-HTML-CSSWATCHER-LOG t (&rest messages) nil nil interpreted) (ac-html-csswatcher-setup-html-stuff-async t nil nil "Asynchronous call \"csswatcher\".\nSet `ac-html-csswatcher-source-dir' with returned by csswatcher value after \"ACSOURCE: \"" interpreted) (ac-html-csswatcher-refresh t nil (interactive nil) "Interactive version of `ac-html-csswatcher-setup-html-stuff-async' with nice name.\n\nRefresh csswatcher." interpreted) (company-web-csswatcher-refresh t nil (interactive nil) "Interactive version of `ac-html-csswatcher-setup-html-stuff-async' with nice name.\n\nRefresh csswatcher." ac-html-csswatcher-refresh) (ac-html-csswatcher+ t nil (interactive nil) "Enable csswatcher for this buffer, csswatcher called after each current buffer save.\n`ac-html-csswatcher+' automatically added to mode hook when you `ac-html-csswatcher-setup'." interpreted) (company-web-csswatcher+ t nil (interactive nil) "Enable csswatcher for this buffer, csswatcher called after each current buffer save.\n`ac-html-csswatcher+' automatically added to mode hook when you `ac-html-csswatcher-setup'." ac-html-csswatcher+) (ac-html-csswatcher-setup t nil nil "1. Enable for web, html, haml etc hooks `ac-html-csswatcher+'\n\n2. Setup `after-save-hook' for CSS modes.\nCurrently we suport only `css-mode' and `less-mode', but later style, sass, scsc etc will be included\nwhen `csswatcher' support them." interpreted) (company-web-csswatcher-setup t nil nil "1. Enable for web, html, haml etc hooks `ac-html-csswatcher+'\n\n2. Setup `after-save-hook' for CSS modes.\nCurrently we suport only `css-mode' and `less-mode', but later style, sass, scsc etc will be included\nwhen `csswatcher' support them." ac-html-csswatcher-setup))"#
    ]];

    assert_ac_html_csswatcher_parity(elisp_form, expect);
}

use expect_test::expect;

use super::{assert_apache_mode_autoload_parity, assert_apache_mode_parity};

#[test]
fn apache_mode_exact_pin_descriptor_origin_and_dependency_contract_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'apache-mode
                      package-alist))))
         (list
          (package-desc-name descriptor)
          (package-version-join
           (package-desc-version descriptor))
          (package-desc-reqs descriptor)
          (package-desc-summary descriptor)
          (copy-tree
           (package-desc-extras descriptor))
          (featurep 'apache-mode)))"##;
    let expect = expect![[
        r#"OK (apache-mode "20210519.1931" nil "Major mode for editing Apache httpd configuration files." ((:maintainers ("USAMI Kenta" . "tadsan@zonu.me")) (:authors ("Karl Chen" . "quarl@nospam.quarl.org")) (:keywords "languages" "faces") (:revdesc . "f2c11aac2f5f") (:commit . "f2c11aac2f5fc598123e04f4604bea248689a117") (:url . "https://github.com/emacs-php/apache-mode")) t)"#
    ]];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_installed_payload_inventory_and_source_digests_match_exactly() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'apache-mode
                    package-alist)))
                 (directory
                  (package-desc-dir descriptor))
                 (files
                  (sort
                   (mapcar
                    #'file-name-nondirectory
                    (directory-files
                     directory
                     t
                     "^[^.].*"))
                   #'string<)))
         (list
          files
          (mapcar
           (lambda (name)
             (let ((path
                    (expand-file-name
                     name
                     directory)))
               (list
                name
                (nth
                 7
                 (file-attributes path))
                (secure-hash
                 'sha256
                 path))))
           '("apache-mode-pkg.el"
             "apache-mode.el"))))"##;
    let expect = expect![[
        r#"OK (("README-elpa" "apache-mode-autoloads.el" "apache-mode-pkg.el" "apache-mode.el" "apache-mode.elc") (("apache-mode-pkg.el" 437 "3dc8a6645ec532ff58e1aadef049479397df68b107716a5694773efe3da46f2e") ("apache-mode.el" 42216 "198af58ce36570935bb08a6f1c68cf44972992de6e4e99de899aa957e0c37dc6")))"#
    ]];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_public_surface_arglists_commands_variables_and_docs_match() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (and
              (fboundp symbol)
              t)
             (commandp symbol)
             (help-function-arglist symbol t)
             (documentation symbol)))
          '(apache-mode
            apache-indent-line
            apache-previous-indentation
            apache-calculate-indentation))
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (boundp symbol)
             (cond
              ((eq
                symbol
                'apache-indent-level)
               apache-indent-level)
              ((eq
                symbol
                'apache-font-lock-keywords)
               (list
                (length
                 apache-font-lock-keywords)
                (mapcar
                 (lambda (entry)
                   (secure-hash
                    'sha256
                    (car entry)))
                 apache-font-lock-keywords)))
              ((eq
                symbol
                'apache-mode-syntax-table)
               (list
                (syntax-table-p
                 apache-mode-syntax-table)
                (with-syntax-table
                    apache-mode-syntax-table
                  (mapcar
                   #'char-syntax
                   '(?_ ?- ?< ?> ?\" ?# ?\n))))))
             (documentation-property
              symbol
              'variable-documentation)))
          '(apache-indent-level
            apache-font-lock-keywords
            apache-mode-syntax-table)))"##;
    let expect = expect![[
        r#"OK (((apache-mode t t nil "Major mode for editing Apache configuration files.\n\nThis mode runs the hook ‘apache-mode-hook’, as the final or\npenultimate step during initialization.\n\n") (apache-indent-line t t nil "Indent current line of Apache code.") (apache-previous-indentation t nil nil "Return the previous (non-empty/comment) indentation.  Doesn’t save position.") (apache-calculate-indentation t nil nil "Return the amount the current line should be indented.")) ((apache-indent-level t 4 "*Number of spaces to indent per level.") (apache-font-lock-keywords t (3 ("061bafd942841181cba42b2767819afa69048a18bb1ebb3487643288c423599f" "84d482ca2ab444196c943a44cb6983145e6089b545747fbcaea18dd35e5ac8e7" "afb2d7a80e9d49197930fbbd91d9ceedf17bfa74a25e4700858e561eea0cdde7")) "Expressions to highlight in Apache config buffers.") (apache-mode-syntax-table t (t (95 95 40 41 34 60 62)) "Syntax table for ‘apache-mode’.")))"#
    ]];

    assert_apache_mode_parity(elisp_form, expect);
}

#[test]
fn apache_mode_autoloads_register_the_exact_file_name_patterns_and_entry_point() {
    let elisp_form = r##"(list
         (featurep 'apache-mode)
         (autoloadp
          (symbol-function
           'apache-mode))
         (commandp 'apache-mode)
         (cl-loop
          for entry in auto-mode-alist
          when
          (eq
           (cdr entry)
           'apache-mode)
          collect
          (car entry)))"##;
    let expect = expect![[
        r#"OK (nil t t ("/apache2/sites-\\(?:available\\|enabled\\)/" "/httpd/conf/.+\\.conf\\'" "/apache2/.+\\.conf\\'" "/\\(?:access\\|httpd\\|srm\\)\\.conf\\'" "/\\.htaccess\\'"))"#
    ]];

    assert_apache_mode_autoload_parity(elisp_form, expect);
}

#[test]
fn apache_mode_autoload_dispatch_loads_the_pinned_source_and_activates_the_mode() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "<VirtualHost *:443>\n"
          "ServerName example.test\n"
          "</VirtualHost>\n")
         (apache-mode)
         (list
          major-mode
          mode-name
          (featurep 'apache-mode)
          (eq
           indent-line-function
           #'apache-indent-line)
          (current-indentation)))"##;
    let expect = expect![[r#"OK (apache-mode "Apache" t t 0)"#]];

    assert_apache_mode_autoload_parity(elisp_form, expect);
}

#[test]
fn apache_mode_font_lock_tables_have_exact_entry_shape_capture_contract_and_digest() {
    let elisp_form = r##"(list
         (length apache-font-lock-keywords)
         (mapcar
          (lambda (entry)
            (list
             (length (car entry))
             (secure-hash
              'sha256
              (car entry))
             (cdr entry)))
          apache-font-lock-keywords)
         (secure-hash
          'sha256
          (mapconcat
           (lambda (entry)
             (format
              "%s\0%S"
              (car entry)
              (cdr entry)))
           apache-font-lock-keywords
           "\0")))"##;
    let expect = expect![[
        r#"OK (3 ((270 "061bafd942841181cba42b2767819afa69048a18bb1ebb3487643288c423599f" (1 'font-lock-function-name-face)) (10205 "84d482ca2ab444196c943a44cb6983145e6089b545747fbcaea18dd35e5ac8e7" (1 'font-lock-keyword-face)) (1931 "afb2d7a80e9d49197930fbbd91d9ceedf17bfa74a25e4700858e561eea0cdde7" (1 'font-lock-type-face))) "386d0029b018947841b0cb6dce43e2e117ae10d03c66b5ed1019974ccef113ae")"#
    ]];

    assert_apache_mode_parity(elisp_form, expect);
}

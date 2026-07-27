use expect_test::expect;

use super::{assert_astute_autoload_parity, assert_astute_parity};

#[test]
fn astute_exact_package_descriptor_dependency_origin_and_feature_contract_match() {
    let elisp_form = r##"(let ((descriptor
                (cadr
                 (assq 'astute package-alist))))
         (list
          (package-desc-name descriptor)
          (package-version-join
           (package-desc-version descriptor))
          (package-desc-summary descriptor)
          (package-desc-kind descriptor)
          (package-desc-reqs descriptor)
          (package-desc-extras descriptor)
          (featurep 'astute)
          (package-installed-p
           'astute
           '(20241015 444))
          (file-name-nondirectory
           (locate-library "astute"))))"##;
    let expect = expect![[
        r#"OK (astute "20241015.444" "A minor mode to redisplay `smart' typography." nil ((emacs (25 1))) ((:maintainers ("Paul W. Rankin" . "rnkn@rnkn.xyz")) (:authors ("Paul W. Rankin" . "rnkn@rnkn.xyz")) (:keywords "faces" "wp") (:revdesc . "69d413c95277") (:commit . "69d413c952771c0d06cda161fb25fe495fb895b0") (:url . "https://github.com/rnkn/astute")) t t "astute.el")"#
    ]];

    assert_astute_parity(elisp_form, expect);
}

#[test]
fn astute_installed_payload_inventory_sizes_and_content_digests_match() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq 'astute package-alist)))
                 (directory
                  (package-desc-dir descriptor)))
         (mapcar
          (lambda (file)
            (let ((path
                   (expand-file-name file directory)))
              (list
               file
               (file-attribute-size
                (file-attributes path))
               (secure-hash 'sha256 path))))
          (sort
           (seq-filter
            (lambda (file)
              (file-regular-p
               (expand-file-name file directory)))
            (directory-files directory nil "\\`[^.]"))
           #'string<)))"##;
    let expect = expect![[
        r#"OK (("README-elpa" 382 "58a702d7fcada39d00c0b9079e2fee0182d770234217f178795701010835020b") ("astute-autoloads.el" 1298 "ca018ee727960b71c68bf048e042b3579c080a533c4d3273f45254257351bd22") ("astute-pkg.el" 416 "c37692eed1993d5cb0b52366a6eba38a39b98475f7cc695aea777e3e3a24f45e") ("astute.el" 6226 "67efc7a4a469e95aac61138817f09cf96336b9d98e7919e58657253a82f67fd9") ("astute.elc" 5749 "0762a218fad28e781b9a9bacfec5d2b984ac3c06cbb8a1db6f097b686984ce39"))"#
    ]];

    assert_astute_parity(elisp_form, expect);
}

#[test]
fn astute_complete_callable_command_arglist_and_source_surface_matches() {
    let elisp_form = r##"(let (symbols)
         (mapatoms
          (lambda (symbol)
            (when
                (and
                 (string-prefix-p
                  "astute"
                  (symbol-name symbol))
                 (not
                  (string-suffix-p
                   "--inliner"
                   (symbol-name symbol)))
                 (not
                  (string-suffix-p
                   "--cmacro"
                   (symbol-name symbol)))
                 (fboundp symbol)
                 (let ((file
                        (symbol-file symbol 'defun)))
                   (and file
                        (string=
                         (file-name-nondirectory file)
                         "astute.el"))))
              (push symbol symbols))))
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (commandp symbol)
             (interactive-form symbol)
             (prin1-to-string
              (help-function-arglist symbol t))
             (file-name-nondirectory
              (symbol-file symbol 'defun))))
          (sort symbols
                (lambda (left right)
                  (string<
                   (symbol-name left)
                   (symbol-name right))))))"##;
    let expect = expect![[
        r#"OK ((astute-case-insensitize nil nil "(string)" "astute.el") (astute-init-font-lock nil nil "nil" "astute.el") (astute-mode t (interactive (list (if current-prefix-arg (prefix-numeric-value current-prefix-arg) 'toggle))) "(&optional arg)" "astute.el"))"#
    ]];

    assert_astute_parity(elisp_form, expect);
}

#[test]
fn astute_complete_declared_variable_defaults_scope_custom_and_source_surface_matches() {
    let elisp_form = r##"(cl-labels
        ((stable
          (value)
          (cond
           ((and
             (functionp value)
             (not
              (symbolp value)))
            :function)
           ((consp value)
            (cons
             (stable
              (car value))
             (stable
              (cdr value))))
           ((vectorp value)
            (cons
             :vector
             (mapcar
              #'stable
              (append value nil))))
           (t value))))
       (let (symbols)
         (mapatoms
          (lambda (symbol)
            (when
                (and
                 (string-prefix-p
                  "astute"
                  (symbol-name symbol))
                 (boundp symbol)
                 (let ((file
                        (symbol-file symbol 'defvar)))
                   (and file
                        (string=
                         (file-name-nondirectory file)
                         "astute.el"))))
              (push symbol symbols))))
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (prin1-to-string
              (stable
               (default-value symbol)))
             (special-variable-p symbol)
             (local-variable-if-set-p symbol)
             (custom-variable-p symbol)
             (prin1-to-string
              (stable
               (get symbol 'custom-type)))
             (prin1-to-string
              (stable
               (get symbol 'custom-group)))
             (prin1-to-string
              (stable
               (get symbol 'safe-local-variable)))
             (file-name-nondirectory
              (symbol-file symbol 'defvar))))
          (sort symbols
                (lambda (left right)
                  (string<
                   (symbol-name left)
                   (symbol-name right)))))))"##;
    let expect = expect![[
        r#"OK ((astute--keywords "nil" t t nil "nil" "nil" "nil" "astute.el") (astute-double-quote-close-regexp "\"[[:alnum:][:punct:]]\\\\(\\\"\\\\)\"" t nil nil "nil" "nil" "nil" "astute.el") (astute-double-quote-open-regexp "\"\\\\(\\\"\\\\)[[:alnum:][:punct:]]\"" t nil nil "nil" "nil" "nil" "astute.el") (astute-em-dash-regexp "\"[^-]\\\\(---\\\\)[^-]\"" t nil nil "nil" "nil" "nil" "astute.el") (astute-en-dash-regexp "\"[^-]\\\\(--\\\\)[^-]\"" t nil nil "nil" "nil" "nil" "astute.el") (astute-lighter "\" “As”\"" t nil ((funcall #'#[nil ((format " %sAs%s" (char-to-string 8220) (char-to-string 8221))) #1=(t)])) "string" "nil" "stringp" "astute.el") (astute-mode "nil" t t nil "nil" "nil" "nil" "astute.el") (astute-mode-hook "nil" t nil (nil) "hook" "nil" "nil" "astute.el") (astute-prefix-single-quote-exceptions "(\"bout\" \"em\" \"n'\" \"cause\" \"round\" \"twas\" \"tis\")" t nil ((funcall #'#[nil ('("bout" "em" "n'" "cause" "round" "twas" "tis")) #1#])) "(repeat string)" "nil" "nil" "astute.el") (astute-single-quote-close-regexp "\"[[:alnum:][:punct:]]\\\\('\\\\)\"" t nil nil "nil" "nil" "nil" "astute.el") (astute-single-quote-inner-regexp "\"[:alnum:]\\\\('\\\\)[:alnum:]\"" t nil nil "nil" "nil" "nil" "astute.el") (astute-single-quote-open-regexp "\"\\\\('\\\\)[[:alnum:][:punct:]]\"" t nil nil "nil" "nil" "nil" "astute.el") (astute-transform-list "(single-quote double-quote en-dash em-dash)" t nil ((funcall #'#[nil ('(single-quote double-quote en-dash em-dash)) #1#])) "(set (const :tag \"Single Quotes\" single-quote) (const :tag \"Double Quotes\" double-quote) (const :tag \"En Dashes\" en-dash) (const :tag \"Em Dashes\" em-dash))" "nil" "listp" "astute.el"))"#
    ]];

    assert_astute_parity(elisp_form, expect);
}

#[test]
fn astute_custom_group_members_types_safety_and_documentation_contract_match() {
    let elisp_form = r##"(list
         (get 'astute 'custom-group)
         (get 'astute 'group-documentation)
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (get symbol 'custom-type)
             (get symbol 'custom-group)
             (get symbol 'safe-local-variable)
             (documentation-property
              symbol
              'variable-documentation)))
          '(astute-lighter
            astute-transform-list
            astute-prefix-single-quote-exceptions))
         (documentation 'astute-mode)
         (documentation 'astute-case-insensitize)
         (documentation 'astute-init-font-lock))"##;
    let expect = expect![[
        r#"OK (((astute-lighter custom-variable) (astute-transform-list custom-variable) (astute-prefix-single-quote-exceptions custom-variable)) "A minor mode to redisplay ``smart'' typography." ((astute-lighter string nil stringp "Mode-line indicator for ‘astute-mode’.") (astute-transform-list (set (const :tag "Single Quotes" single-quote) (const :tag "Double Quotes" double-quote) (const :tag "En Dashes" en-dash) (const :tag "Em Dashes" em-dash)) nil listp "List of characters to typographically transform.") (astute-prefix-single-quote-exceptions (repeat string) nil nil "List of regular expressions that should be prefixed by a closing quote.")) "Redisplay ‘smart’ typography.\n\nThis is a minor mode.  If called interactively, toggle the ‘Astute mode’\nmode.  If the prefix argument is positive, enable the mode, and if it is\nzero or negative, disable the mode.\n\nIf called from Lisp, toggle the mode if ARG is ‘toggle’.  Enable the\nmode if ARG is nil, omitted, or is a positive number.  Disable the mode\nif ARG is a negative number.\n\nTo check whether the minor mode is enabled in the current buffer,\nevaluate the variable ‘astute-mode’.\n\nThe mode’s hook is called both when the mode is enabled and when it is\ndisabled." "Return a case-insensitive regular expression for STRING." "Return a new list of ‘font-lock-keywords’.")"#
    ]];

    assert_astute_parity(elisp_form, expect);
}

#[test]
fn astute_generated_autoload_registers_mode_without_eagerly_loading_package() {
    let elisp_form = r##"(list
         (featurep 'astute)
         (fboundp 'astute-mode)
         (autoloadp
          (symbol-function 'astute-mode))
         (symbol-file 'astute-mode 'defun)
         (fboundp 'astute-case-insensitize)
         (fboundp 'astute-init-font-lock)
         (boundp 'astute-transform-list)
         (assoc 'astute-mode minor-mode-alist))"##;
    let expect = expect![[
        r#"OK (nil t t "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/astute/20241015.444/home/.emacs.d/elpa/astute-20241015.444/astute.el" nil nil nil nil)"#
    ]];

    assert_astute_autoload_parity(elisp_form, expect);
}

use expect_test::expect;

use super::assert_apheleia_parity;

#[test]
fn apheleia_get_formatters_ports_the_complete_upstream_mode_filename_and_predicate_matrix() {
    let elisp_form = r##"(let ((apheleia-mode-alist
                '(("\\.foobar\\'" . fmt-foobar)
                  (sgml-mode . fmt-sgml)
                  (html-mode . (fmt-html fmt-html-again))
                  (text-mode . fmt-text)
                  (blah-mode . fmt-blah)
                  ("\\.foobaz\\'" . fmt-foobaz)))
               (apheleia-mode-predicates
                '((lambda ()
                    (when
                        (and
                         buffer-file-name
                         (string-match-p
                          "\\.blah"
                          buffer-file-name))
                      'blah-mode)))))
         (mapcar
          (lambda (spec)
            (with-temp-buffer
              (setq major-mode
                    (car spec))
              (setq-local
               buffer-file-name
               (cadr spec))
              (list
               (car spec)
               (cadr spec)
               (apheleia--get-formatters))))
          '((fundamental-mode nil)
            (fundamental-mode "ok.foobar")
            (text-mode nil)
            (sgml-mode nil)
            (html-mode nil)
            (html-mode "ok.foobar")
            (html-mode "ok.foobaz")
            (html-mode "ok.blah")
            (html-mode "ok.blah.foobar")
            (html-mode "ok.blah.foobaz"))))"##;
    let expect = expect![[
        r#"OK ((fundamental-mode nil nil) (fundamental-mode "ok.foobar" (fmt-foobar)) (text-mode nil (fmt-text)) (sgml-mode nil (fmt-sgml)) (html-mode nil #1=(fmt-html fmt-html-again)) (html-mode "ok.foobar" (fmt-foobar)) (html-mode "ok.foobaz" #1#) (html-mode "ok.blah" (fmt-blah)) (html-mode "ok.blah.foobar" (fmt-foobar)) (html-mode "ok.blah.foobaz" (fmt-blah)))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_get_formatters_returns_nil_for_the_upstream_empty_configuration_matrix() {
    let elisp_form = r##"(let ((apheleia-mode-alist nil)
               (apheleia-mode-predicates nil))
         (mapcar
          (lambda (spec)
            (with-temp-buffer
              (setq major-mode
                    (car spec))
              (setq-local
               buffer-file-name
               (cadr spec))
              (apheleia--get-formatters)))
          '((text-mode "foo.txt")
            (fundamental-mode nil)
            (cc-mode "foo.c")
            (mhtml-mode "foo.html"))))"##;
    let expect = expect!["OK (nil nil nil nil)"];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_formatter_override_accepts_single_and_chained_values_and_safe_local_validation() {
    let elisp_form = r##"(list
         (mapcar
          #'apheleia--formatter-safe-p
          '(nil
            black
            (isort black)
            ()
            ("black")
            (black "isort")
            42
            [black]))
         (mapcar
          #'apheleia--ensure-list
          '(black
            (isort black)
            nil
            "formatter"))
         (with-temp-buffer
           (setq-local
            apheleia-formatter
            'explicit)
           (setq-local
            apheleia-mode-alist
            '((fundamental-mode . fallback)))
           (apheleia--get-formatters))
         (with-temp-buffer
           (setq-local
            apheleia-formatter
            '(first second))
           (apheleia--get-formatters)))"##;
    let expect = expect![[
        r#"OK ((t t t t nil nil nil nil) ((black) (isort black) nil ("formatter")) (explicit) (first second))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_mode_selection_prefers_the_most_specific_derived_mode_then_regex_order() {
    let elisp_form = r##"(progn
         (define-derived-mode
           apheleia-test-parent-mode
           fundamental-mode
           "AParent")
         (define-derived-mode
           apheleia-test-child-mode
           apheleia-test-parent-mode
           "AChild")
         (let ((apheleia-mode-predicates nil))
           (mapcar
            (lambda (alist)
              (with-temp-buffer
                (apheleia-test-child-mode)
                (setq-local
                 buffer-file-name
                 "/project/demo.special")
                (let ((apheleia-mode-alist
                       alist))
                  (apheleia--get-formatters))))
            '(((fundamental-mode . root)
               (apheleia-test-parent-mode . parent)
               (apheleia-test-child-mode . child))
              ((apheleia-test-child-mode . child)
               (apheleia-test-parent-mode . parent)
               (fundamental-mode . root))
              (("\\.special\\'" . by-file)
               (apheleia-test-child-mode . child))
              ((apheleia-test-parent-mode . parent)
               ("\\.special\\'" . by-file)
               (apheleia-test-child-mode . child))))))"##;
    let expect = expect!["OK ((child) (child) (by-file) (child))"];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_get_mode_chain_reports_real_and_custom_parentage_in_specificity_order() {
    let elisp_form = r##"(progn
         (define-derived-mode
           apheleia-test-grandparent-mode
           fundamental-mode
           "AGrand")
         (define-derived-mode
           apheleia-test-parent-mode
           apheleia-test-grandparent-mode
           "AParent")
         (define-derived-mode
           apheleia-test-child-mode
           apheleia-test-parent-mode
           "AChild")
         (list
          (with-temp-buffer
            (apheleia-test-child-mode)
            (apheleia--get-mode-chain))
          (with-temp-buffer
            (emacs-lisp-mode)
            (apheleia--get-mode-chain))
          (with-temp-buffer
            (fundamental-mode)
            (apheleia--get-mode-chain))))"##;
    let expect = expect![
        "OK ((apheleia-test-child-mode apheleia-test-parent-mode apheleia-test-grandparent-mode) (emacs-lisp-mode lisp-data-mode prog-mode) (fundamental-mode))"
    ];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_interactive_selection_prompts_only_when_requested_or_unconfigured() {
    let elisp_form = r##"(let ((apheleia-formatters
                '((alpha . ("alpha"))
                  (beta . ("beta"))))
               (apheleia-mode-alist
                '((fundamental-mode . alpha)))
               prompts)
         (cl-letf
             (((symbol-function
                'completing-read)
               (lambda
                   (prompt collection
                    &optional predicate require-match
                    initial-input history default inherit)
                 (ignore
                  predicate initial-input history default inherit)
                 (setq prompts
                       (append
                        prompts
                        (list
                         (list
                          prompt
                          collection
                          require-match))))
                 "beta")))
           (with-temp-buffer
             (fundamental-mode)
             (list
              (apheleia--get-formatters)
              (apheleia--get-formatters
               'interactive)
              (apheleia--get-formatters
               'prompt)
              (let ((apheleia-mode-alist nil))
                (apheleia--get-formatters
                 'interactive))
              prompts))))"##;
    let expect = expect![[
        r#"OK ((alpha) (alpha) (beta) (beta) (("Formatter: " (alpha beta) require-match) ("Formatter: " (alpha beta) require-match)))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_mhtml_predicate_uses_submode_properties_at_point_and_end_of_buffer() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "html css javascript")
         (put-text-property
          6
          9
          'mhtml-submode
          'css-mode)
         (put-text-property
          10
          (point-max)
          'mhtml-submode
          'javascript-mode)
         (list
          (progn
            (goto-char 1)
            (apheleia-mhtml-mode-predicate))
          (progn
            (goto-char 7)
            (apheleia-mhtml-mode-predicate))
          (progn
            (goto-char
             (point-max))
            (apheleia-mhtml-mode-predicate))
          (progn
            (erase-buffer)
            (apheleia-mhtml-mode-predicate))))"##;
    let expect = expect!["OK (nil mhtml-mode mhtml-mode nil)"];

    assert_apheleia_parity(elisp_form, expect);
}

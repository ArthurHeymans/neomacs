use expect_test::expect;

use super::assert_ac_haskell_process_parity;

#[test]
fn ac_haskell_process_symbol_start_covers_words_strings_punctuation_and_missing_symbols() {
    let elisp_form = r##"(mapcar
               (lambda (fixture)
                 (with-temp-buffer
                   (modify-syntax-entry
                    ?_ "w")
                   (insert
                    (car fixture))
                   (goto-char
                    (cadr fixture))
                   (cl-letf
                       (((symbol-function
                          'in-string-p)
                         (lambda ()
                           (nth
                            2 fixture))))
                     (list
                      fixture
                      (symbol-at-point)
                      (ac-haskell-process-symbol-start-pos)
                      (point)))))
               '(("alpha_beta" 7 nil)
                 ("λvalue" 3 nil)
                 ("plain" 3 t)
                 ("   " 2 nil)
                 ("" 1 nil)))"##;
    let expect = expect![[
        r#"OK ((("alpha_beta" 7 nil) alpha_beta 1 7) (("λvalue" 3 nil) λvalue 1 3) (("plain" 3 t) plain nil 3) (("   " 2 nil) nil nil 2) (("" 1 nil) nil nil 1))"#
    ]];

    assert_ac_haskell_process_parity(elisp_form, expect);
}

#[test]
fn ac_haskell_process_symbol_start_does_not_query_string_state_without_symbol() {
    let elisp_form = r##"(with-temp-buffer
               (insert "   ")
               (goto-char 2)
               (let (calls)
                 (cl-letf
                     (((symbol-function
                        'symbol-at-point)
                       (lambda ()
                         (push 'symbol calls)
                         nil))
                      ((symbol-function
                        'in-string-p)
                       (lambda ()
                         (push 'in-string calls)
                         t)))
                   (list
                    (ac-haskell-process-symbol-start-pos)
                    calls))))"##;
    let expect = expect!["OK (nil (symbol))"];

    assert_ac_haskell_process_parity(elisp_form, expect);
}

#[test]
fn ac_haskell_process_popup_doc_forwards_symbol_documentation_and_exact_popup_options() {
    let elisp_form = r##"(with-temp-buffer
               (insert "map")
               (goto-char 2)
               (let (calls)
                 (cl-letf
                     (((symbol-function
                        'ac-haskell-process-doc)
                       (lambda (symbol)
                         (push
                          (list 'doc symbol)
                          calls)
                         "map docs"))
                      ((symbol-function
                        'ac-haskell-process-symbol-start-pos)
                       (lambda ()
                         (push '(start) calls)
                         17))
                      ((symbol-function
                        'popup-tip)
                       (lambda
                           (doc &rest options)
                         (push
                          (list
                           'popup doc options)
                          calls)
                         'popup-result)))
                   (list
                    (ac-haskell-process-popup-doc)
                    (nreverse calls)
                    (point)
                    (interactive-form
                     #'ac-haskell-process-popup-doc)))))"##;
    let expect = expect![[
        r#"OK (popup-result ((doc "map") (start) (popup "map docs" (:point 17 :around t :scroll-bar t :margin t))) 2 (interactive nil))"#
    ]];

    assert_ac_haskell_process_parity(elisp_form, expect);
}

#[test]
fn ac_haskell_process_popup_doc_skips_popup_and_start_position_for_nil_documentation() {
    let elisp_form = r##"(with-temp-buffer
               (insert "map")
               (goto-char 2)
               (let (calls)
                 (cl-letf
                     (((symbol-function
                        'ac-haskell-process-doc)
                       (lambda (symbol)
                         (push
                          (list 'doc symbol)
                          calls)
                         nil))
                      ((symbol-function
                        'ac-haskell-process-symbol-start-pos)
                       (lambda ()
                         (push '(start) calls)
                         1))
                      ((symbol-function
                        'popup-tip)
                       (lambda (&rest arguments)
                         (push
                          (cons
                           'popup arguments)
                          calls)
                         'unexpected)))
                   (list
                    (ac-haskell-process-popup-doc)
                    (nreverse calls)))))"##;
    let expect = expect![[r#"OK (nil ((doc "map")))"#]];

    assert_ac_haskell_process_parity(elisp_form, expect);
}

#[test]
fn ac_haskell_process_popup_doc_without_symbol_looks_up_the_literal_nil_name() {
    let elisp_form = r##"(with-temp-buffer
               (insert "   ")
               (goto-char 2)
               (let (calls)
                 (cl-letf
                     (((symbol-function
                        'symbol-at-point)
                       (lambda ()
                         (push '(symbol) calls)
                         nil))
                      ((symbol-function
                        'ac-haskell-process-doc)
                       (lambda (symbol)
                         (push
                          (list 'doc symbol)
                          calls)
                         nil))
                      ((symbol-function
                        'popup-tip)
                       (lambda (&rest arguments)
                         (push
                          (cons
                           'popup arguments)
                          calls)
                         'unexpected)))
                   (list
                    (ac-haskell-process-popup-doc)
                    (nreverse calls)))))"##;
    let expect = expect![[r#"OK (nil ((symbol) (doc "nil")))"#]];

    assert_ac_haskell_process_parity(elisp_form, expect);
}

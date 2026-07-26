use expect_test::expect;

use super::{assert_magit_parity, assert_magit_signal_parity};

#[test]
fn magit_ellipsis_respects_display_capability_and_customization() {
    let elisp_form = r##"(list
              (cl-letf (((symbol-function 'char-displayable-p)
                         (lambda (_) t)))
                (list (magit--ellipsis 'margin)
                      (magit--ellipsis)))
              (cl-letf (((symbol-function 'char-displayable-p)
                         (lambda (_) nil)))
                (list (magit--ellipsis 'margin)
                      (magit--ellipsis)))
              (let ((magit-ellipsis
                     '((margin (?· . "!"))
                       (t (?. . ">")))))
                (list
                 (cl-letf (((symbol-function 'char-displayable-p)
                            (lambda (_) t)))
                   (list (magit--ellipsis 'margin)
                         (magit--ellipsis)))
                 (cl-letf (((symbol-function 'char-displayable-p)
                            (lambda (_) nil)))
                   (list (magit--ellipsis 'margin)
                         (magit--ellipsis))))))"##;
    let expect = expect![[r#"OK (("…" "…") (">" "...") (("·" ".") ("!" ">")))"#]];

    assert_magit_parity(elisp_form, expect);
}

#[test]
fn magit_text_face_composition_preserves_existing_properties_and_order() {
    let elisp_form = r##"(let ((text
                    (concat
                     (propertize "ab" 'font-lock-face 'highlight)
                     (propertize "cd" 'face 'italic)
                     "ef")))
               (magit--add-face-text-property
                2 4 'bold nil text t)
               (magit--add-face-text-property
                0 2 'bold nil text)
               (magit--add-face-text-property
                2 6 'underline t text)
               (list
                (get-text-property 0 'font-lock-face text)
                (get-text-property 2 'font-lock-face text)
                (get-text-property 4 'font-lock-face text)
                (get-text-property 2 'face text)
                (substring-no-properties text)))"##;
    let expect =
        expect![[r#"OK ((bold highlight) (bold italic underline) (underline) nil "abcdef")"#]];

    assert_magit_parity(elisp_form, expect);
}

#[test]
fn magit_face_helpers_apply_and_query_complete_string_properties() {
    let elisp_form = r##"(let ((bold
                    (magit--propertize-face "abc" 'bold))
                   (mixed
                    (concat
                     (magit--propertize-face "ab" 'bold)
                     (magit--propertize-face "c" 'italic))))
               (list
                (get-text-property 0 'face bold)
                (get-text-property 0 'font-lock-face bold)
                (magit-face-property-all 'bold bold)
                (magit-face-property-all 'bold mixed)
                (substring-no-properties bold)))"##;
    let expect = expect![[r#"OK (bold bold t nil "abc")"#]];

    assert_magit_parity(elisp_form, expect);
}

#[test]
fn magit_malformed_ellipsis_customization_signals_user_error() {
    let elisp_form = r##"(let ((magit-ellipsis
                    '((margin (?· . "!")))))
               (magit--ellipsis))"##;
    let expect = expect![[r#"ERR (user-error "Variable magit-ellipsis is invalid")"#]];

    assert_magit_signal_parity(elisp_form, expect);
}

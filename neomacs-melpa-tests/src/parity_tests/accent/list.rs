use expect_test::expect;

use super::assert_accent_parity;

#[test]
fn accent_list_without_custom_entries_copies_only_the_outer_spine() {
    let elisp_form = r##"(let* ((accent-custom nil)
                    (first
                     (accent-lst))
                    (second
                     (accent-lst)))
               (list
                (equal
                 first
                 accent-diacritics)
                (eq
                 first
                 accent-diacritics)
                (eq
                 first
                 second)
                (cl-every
                 #'identity
                 (cl-mapcar
                  #'eq
                  first
                  accent-diacritics))
                (cl-every
                 #'identity
                 (cl-mapcar
                  (lambda (left right)
                    (eq
                     (cadr
                      left)
                     (cadr
                      right)))
                  first
                  accent-diacritics))
                (length
                 first)))"##;
    let expect = expect!["OK (t nil nil t t 22)"];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_list_appends_matching_custom_candidates_and_ignores_unknown_letters() {
    let elisp_form = r##"(let* ((accent-custom
                     '((a
                        (ă
                         ą))
                       (o
                        (ŏ))
                       (q
                        (ꝗ))))
                    (original-a
                     (assq
                      'a
                      accent-diacritics))
                    (original-c
                     (assq
                      'c
                      accent-diacritics))
                    (result
                     (accent-lst))
                    (merged-a
                     (assq
                      'a
                      result))
                    (merged-o
                     (assq
                      'o
                      result))
                    (unchanged-c
                     (assq
                      'c
                      result)))
               (list
                merged-a
                merged-o
                (assq
                 'q
                 result)
                (eq
                 merged-a
                 original-a)
                (eq
                 unchanged-c
                 original-c)
                original-a
                (assq
                 'a
                 accent-diacritics)
                (length
                 result)))"##;
    let expect = expect![
        "OK ((a (à á â ä æ ã å ā ă ą)) (o (ô ö ò ó œ ø ō õ ŏ)) nil nil t #1=(a (à á â ä æ ã å ā)) #1# 22)"
    ];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_list_uses_only_the_first_duplicate_custom_entry() {
    let elisp_form = r##"(let ((accent-custom
                    '((a
                       (first))
                      (a
                       (second))
                      (A
                       (uppercase)))))
               (list
                (assq
                 'a
                 (accent-lst))
                (assq
                 'A
                 (accent-lst))))"##;
    let expect = expect!["OK ((a (à á â ä æ ã å ā first)) (A (À Á Â Ä Æ Ã Å Ā uppercase)))"];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_list_treats_lowercase_and_uppercase_custom_keys_independently() {
    let elisp_form = r##"(let ((accent-custom
                    '((a
                       (lower))
                      (A
                       (upper)))))
               (list
                (car
                 (last
                  (cadr
                   (assq
                    'a
                    (accent-lst)))))
                (car
                 (last
                  (cadr
                   (assq
                    'A
                    (accent-lst)))))
                (assq
                 'c
                 (accent-lst))))"##;
    let expect = expect!["OK (lower upper (c (ç ć č)))"];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_list_accepts_a_matching_custom_entry_without_a_candidate_list() {
    let elisp_form = r##"(let ((accent-custom
                    '((a))))
               (let ((result
                      (accent-lst)))
                 (list
                  (assq
                   'a
                   result)
                  (equal
                   (assq
                    'a
                    result)
                   (assq
                    'a
                    accent-diacritics))
                  (eq
                   (assq
                    'a
                    result)
                   (assq
                    'a
                    accent-diacritics)))))"##;
    let expect = expect!["OK ((a (à á â ä æ ã å ā)) t nil)"];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_diacritics_variable_metadata_and_live_default_identity_match() {
    let elisp_form = r##"(list
               (get
                'accent-diacritics
                'variable-documentation)
               (default-boundp
                'accent-diacritics)
               (eq
                accent-diacritics
                (default-value
                 'accent-diacritics))
               (local-variable-if-set-p
                'accent-diacritics)
               (length
                accent-diacritics)
               (apply
                #'+
                (mapcar
                 (lambda (entry)
                   (length
                    (cadr
                     entry)))
                 accent-diacritics)))"##;
    let expect = expect![[
        r#"OK ("List of diacritics available.\nFor each character, includes a list\nof available options to be displayed in the popup." t t nil 22 93)"#
    ]];

    assert_accent_parity(elisp_form, expect);
}

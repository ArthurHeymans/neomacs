use expect_test::expect;

use super::assert_astute_parity;

#[test]
fn astute_default_keyword_builder_emits_all_eight_rules_in_precedence_order() {
    let elisp_form = r##"(let ((keywords
                (astute-init-font-lock)))
         (list
          (length keywords)
          keywords
          astute-transform-list
          astute-prefix-single-quote-exceptions))"##;
    let expect = expect![[
        r#"OK (8 (("\\('\\)[[:alnum:][:punct:]]" (1 '(face nil display "‘"))) ("[:alnum:]\\('\\)[:alnum:]" (1 '(face nil display "’"))) ("[[:alnum:][:punct:]]\\('\\)" (1 '(face nil display "’"))) ("\\(?1:'\\)[0-9][0-9]s?\\|\\(?1:'\\)[Bb][Oo][Uu][Tt]\\|\\(?1:'\\)[Ee][Mm]\\|\\(?1:'\\)[Nn]'\\|\\(?1:'\\)[Cc][Aa][Uu][Ss][Ee]\\|\\(?1:'\\)[Rr][Oo][Uu][Nn][Dd]\\|\\(?1:'\\)[Tt][Ww][Aa][Ss]\\|\\(?1:'\\)[Tt][Ii][Ss]" (1 '(face nil display "’"))) ("\\(\"\\)[[:alnum:][:punct:]]" (1 '(face nil display "“"))) ("[[:alnum:][:punct:]]\\(\"\\)" (1 '(face nil display "”"))) ("[^-]\\(--\\)[^-]" (1 '(face nil display "–"))) ("[^-]\\(---\\)[^-]" (1 '(face nil display "—")))) (single-quote double-quote en-dash em-dash) ("bout" "em" "n'" "cause" "round" "twas" "tis"))"#
    ]];

    assert_astute_parity(elisp_form, expect);
}

#[test]
fn astute_single_quote_keyword_set_contains_open_inner_close_and_prefix_rules() {
    let elisp_form = r##"(let ((astute-transform-list
                '(single-quote)))
         (astute-init-font-lock))"##;
    let expect = expect![[
        r#"OK (("\\('\\)[[:alnum:][:punct:]]" (1 '(face nil display "‘"))) ("[:alnum:]\\('\\)[:alnum:]" (1 '(face nil display "’"))) ("[[:alnum:][:punct:]]\\('\\)" (1 '(face nil display "’"))) ("\\(?1:'\\)[0-9][0-9]s?\\|\\(?1:'\\)[Bb][Oo][Uu][Tt]\\|\\(?1:'\\)[Ee][Mm]\\|\\(?1:'\\)[Nn]'\\|\\(?1:'\\)[Cc][Aa][Uu][Ss][Ee]\\|\\(?1:'\\)[Rr][Oo][Uu][Nn][Dd]\\|\\(?1:'\\)[Tt][Ww][Aa][Ss]\\|\\(?1:'\\)[Tt][Ii][Ss]" (1 '(face nil display "’"))))"#
    ]];

    assert_astute_parity(elisp_form, expect);
}

#[test]
fn astute_each_non_single_transform_selects_only_its_owned_typography_rules() {
    let elisp_form = r##"(mapcar
         (lambda (transforms)
           (let ((astute-transform-list
                  transforms))
             (list
              transforms
              (astute-init-font-lock))))
         '((double-quote)
           (en-dash)
           (em-dash)
           (double-quote em-dash)
           (en-dash double-quote)))"##;
    let expect = expect![[
        r#"OK (((double-quote) (("\\(\"\\)[[:alnum:][:punct:]]" (1 '(face nil display "“"))) ("[[:alnum:][:punct:]]\\(\"\\)" (1 '(face nil display "”"))))) ((en-dash) (("[^-]\\(--\\)[^-]" (1 '(face nil display "–"))))) ((em-dash) (("[^-]\\(---\\)[^-]" (1 '(face nil display "—"))))) ((double-quote em-dash) (("\\(\"\\)[[:alnum:][:punct:]]" (1 '(face nil display "“"))) ("[[:alnum:][:punct:]]\\(\"\\)" (1 '(face nil display "”"))) ("[^-]\\(---\\)[^-]" (1 '(face nil display "—"))))) ((en-dash double-quote) (("\\(\"\\)[[:alnum:][:punct:]]" (1 '(face nil display "“"))) ("[[:alnum:][:punct:]]\\(\"\\)" (1 '(face nil display "”"))) ("[^-]\\(--\\)[^-]" (1 '(face nil display "–"))))))"#
    ]];

    assert_astute_parity(elisp_form, expect);
}

#[test]
fn astute_empty_unknown_and_duplicate_transform_entries_have_set_semantics() {
    let elisp_form = r##"(mapcar
         (lambda (transforms)
           (let ((astute-transform-list
                  transforms))
             (list
              transforms
              (length
               (astute-init-font-lock))
              (astute-init-font-lock))))
         '(nil
           (unknown)
           (single-quote single-quote)
           (em-dash unknown em-dash)
           (em-dash single-quote double-quote en-dash)
           (en-dash double-quote single-quote em-dash)))"##;
    let expect = expect![[
        r#"OK ((nil 0 nil) ((unknown) 0 nil) ((single-quote single-quote) 4 (("\\('\\)[[:alnum:][:punct:]]" (1 '(face nil display "‘"))) ("[:alnum:]\\('\\)[:alnum:]" (1 '(face nil display "’"))) ("[[:alnum:][:punct:]]\\('\\)" (1 '(face nil display "’"))) ("\\(?1:'\\)[0-9][0-9]s?\\|\\(?1:'\\)[Bb][Oo][Uu][Tt]\\|\\(?1:'\\)[Ee][Mm]\\|\\(?1:'\\)[Nn]'\\|\\(?1:'\\)[Cc][Aa][Uu][Ss][Ee]\\|\\(?1:'\\)[Rr][Oo][Uu][Nn][Dd]\\|\\(?1:'\\)[Tt][Ww][Aa][Ss]\\|\\(?1:'\\)[Tt][Ii][Ss]" (1 '(face nil display "’"))))) ((em-dash unknown em-dash) 1 (("[^-]\\(---\\)[^-]" (1 '(face nil display "—"))))) ((em-dash single-quote double-quote en-dash) 8 (("\\('\\)[[:alnum:][:punct:]]" (1 '(face nil display "‘"))) ("[:alnum:]\\('\\)[:alnum:]" (1 '(face nil display "’"))) ("[[:alnum:][:punct:]]\\('\\)" (1 '(face nil display "’"))) ("\\(?1:'\\)[0-9][0-9]s?\\|\\(?1:'\\)[Bb][Oo][Uu][Tt]\\|\\(?1:'\\)[Ee][Mm]\\|\\(?1:'\\)[Nn]'\\|\\(?1:'\\)[Cc][Aa][Uu][Ss][Ee]\\|\\(?1:'\\)[Rr][Oo][Uu][Nn][Dd]\\|\\(?1:'\\)[Tt][Ww][Aa][Ss]\\|\\(?1:'\\)[Tt][Ii][Ss]" (1 '(face nil display "’"))) ("\\(\"\\)[[:alnum:][:punct:]]" (1 '(face nil display "“"))) ("[[:alnum:][:punct:]]\\(\"\\)" (1 '(face nil display "”"))) ("[^-]\\(--\\)[^-]" (1 '(face nil display "–"))) ("[^-]\\(---\\)[^-]" (1 '(face nil display "—"))))) ((en-dash double-quote single-quote em-dash) 8 (("\\('\\)[[:alnum:][:punct:]]" (1 '(face nil display "‘"))) ("[:alnum:]\\('\\)[:alnum:]" (1 '(face nil display "’"))) ("[[:alnum:][:punct:]]\\('\\)" (1 '(face nil display "’"))) ("\\(?1:'\\)[0-9][0-9]s?\\|\\(?1:'\\)[Bb][Oo][Uu][Tt]\\|\\(?1:'\\)[Ee][Mm]\\|\\(?1:'\\)[Nn]'\\|\\(?1:'\\)[Cc][Aa][Uu][Ss][Ee]\\|\\(?1:'\\)[Rr][Oo][Uu][Nn][Dd]\\|\\(?1:'\\)[Tt][Ww][Aa][Ss]\\|\\(?1:'\\)[Tt][Ii][Ss]" (1 '(face nil display "’"))) ("\\(\"\\)[[:alnum:][:punct:]]" (1 '(face nil display "“"))) ("[[:alnum:][:punct:]]\\(\"\\)" (1 '(face nil display "”"))) ("[^-]\\(--\\)[^-]" (1 '(face nil display "–"))) ("[^-]\\(---\\)[^-]" (1 '(face nil display "—"))))))"#
    ]];

    assert_astute_parity(elisp_form, expect);
}

#[test]
fn astute_keyword_builder_uses_current_custom_exception_values_on_every_call() {
    let elisp_form = r##"(let ((first
                (let ((astute-prefix-single-quote-exceptions
                       '("alpha")))
                  (astute-init-font-lock)))
               (second
                (let ((astute-prefix-single-quote-exceptions
                       '("beta"
                         "gamma")))
                  (astute-init-font-lock))))
         (list
          (car
           (nth 3 first))
          (car
           (nth 3 second))
          (equal first second)
          (eq first second)
          (eq
           (nth 0 first)
           (nth 0 second))))"##;
    let expect = expect![[
        r#"OK ("\\(?1:'\\)[0-9][0-9]s?\\|\\(?1:'\\)[Aa][Ll][Pp][Hh][Aa]" "\\(?1:'\\)[0-9][0-9]s?\\|\\(?1:'\\)[Bb][Ee][Tt][Aa]\\|\\(?1:'\\)[Gg][Aa][Mm][Mm][Aa]" nil nil nil)"#
    ]];

    assert_astute_parity(elisp_form, expect);
}

#[test]
fn astute_keyword_builder_returns_fresh_mutable_lists_without_altering_custom_defaults() {
    let elisp_form = r##"(let* ((default-transforms
                  (copy-tree
                   astute-transform-list))
                 (default-exceptions
                  (copy-tree
                   astute-prefix-single-quote-exceptions))
                 (first
                  (astute-init-font-lock))
                 (second
                  (astute-init-font-lock)))
         (setcar first 'mutated)
         (setcar
          astute-prefix-single-quote-exceptions
          "temporarily-mutated")
         (prog1
             (list
              (car first)
              (car second)
              (eq first second)
              (equal
               astute-transform-list
               default-transforms)
              astute-prefix-single-quote-exceptions
              default-exceptions)
           (setq
            astute-prefix-single-quote-exceptions
            default-exceptions)))"##;
    let expect = expect![[
        r#"OK (mutated ("\\('\\)[[:alnum:][:punct:]]" (1 '(face nil display "‘"))) nil t ("temporarily-mutated" "em" "n'" "cause" "round" "twas" "tis") ("bout" "em" "n'" "cause" "round" "twas" "tis"))"#
    ]];

    assert_astute_parity(elisp_form, expect);
}

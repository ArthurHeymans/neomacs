use expect_test::expect;

use super::assert_amsreftex_parity;

#[test]
fn amsreftex_sort_record_helpers_find_exact_boundaries_and_parse_keys() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "preamble\n"
          "\\bib{first}{article}{\n"
          " author={Zulu, Zoe}\n"
          " date={2024}\n"
          "}\n"
          "between\n"
          "\\bib{second}{book}{\n"
          " title={Nested {Book}}\n"
          " date={2020}\n"
          "}\n"
          "tail")
         (goto-char (point-min))
         (amsreftex-sort-nextrecfn)
         (let ((first-start (point))
               (first-key
                (amsreftex-sort-startkeyfn)))
           (amsreftex-sort-endrecfn)
           (let ((first-end (point)))
             (amsreftex-sort-nextrecfn)
             (let ((second-start (point))
                   (second-key
                    (amsreftex-sort-startkeyfn)))
               (amsreftex-sort-endrecfn)
               (list
                first-start
                first-end
                (buffer-substring-no-properties
                 first-start first-end)
                (mapcar
                 (lambda (field)
                   (assoc field first-key))
                 '("&key" "&type" "author" "year"))
                second-start
                (point)
                (mapcar
                 (lambda (field)
                   (assoc field second-key))
                 '("&key" "&type" "title" "year")))))))"##;
    let expect = expect![[
        r#"OK (10 66 "\\bib{first}{article}{\n author={Zulu, Zoe}\n date={2024}\n}" (("&key" . "first") ("&type" . "article") ("author" . "Zulu, Zoe") ("year" . "2024")) 75 132 (("&key" . "second") ("&type" . "book") ("title" . "Nested {Book}") ("year" . "2020")))"#
    ]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_sort_bibliography_orders_a_real_biblist_by_author_then_year() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "before\n"
          "\\begin{biblist}\n"
          "\\bib{late}{article}{\n author={Zulu, Zoe}\n date={2020}\n title={Late}\n}\n"
          "\\bib{ada-new}{article}{\n author={Lovelace, Ada}\n date={1850}\n title={New}\n}\n"
          "\\bib{ada-old}{article}{\n author={Lovelace, Ada}\n date={1843}\n title={Old}\n}\n"
          "\\bib{grace}{article}{\n author={Hopper, Grace}\n date={1952}\n title={Compiler}\n}\n"
          "\\end{biblist}\n"
          "after\n")
         (goto-char (point-min))
         (search-forward "ada-new")
         (amsreftex-sort-bibliography)
         (list
          (buffer-string)
          (mapcar
           (lambda (key)
             (save-excursion
               (goto-char (point-min))
               (search-forward
                (format "\\bib{%s}" key))
               (line-number-at-pos)))
           '(grace ada-old ada-new late))))"##;
    let expect = expect![[
        r#"OK ("before\n\\begin{biblist}\n\\bib{grace}{article}{\n author={Hopper, Grace}\n date={1952}\n title={Compiler}\n}\n\\bib{ada-old}{article}{\n author={Lovelace, Ada}\n date={1843}\n title={Old}\n}\n\\bib{ada-new}{article}{\n author={Lovelace, Ada}\n date={1850}\n title={New}\n}\n\\bib{late}{article}{\n author={Zulu, Zoe}\n date={2020}\n title={Late}\n}\n\\end{biblist}\nafter\n" (3 8 13 18))"#
    ]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_sort_bibliography_honors_custom_title_then_year_fields() {
    let elisp_form = r##"(let ((amsreftex-sort-fields
                        '("title" "year")))
         (with-temp-buffer
           (insert
            "\\begin{biblist}\n"
            "\\bib{z-old}{book}{\n title={Zeta}\n date={1990}\n}\n"
            "\\bib{a-new}{book}{\n title={Alpha}\n date={2020}\n}\n"
            "\\bib{a-old}{book}{\n title={Alpha}\n date={2000}\n}\n"
            "\\end{biblist}\n")
           (goto-char (point-min))
           (forward-line 2)
           (amsreftex-sort-bibliography)
           (buffer-string)))"##;
    let expect = expect![[
        r#"OK "\\begin{biblist}\n\\bib{a-old}{book}{\n title={Alpha}\n date={2000}\n}\n\\bib{a-new}{book}{\n title={Alpha}\n date={2020}\n}\n\\bib{z-old}{book}{\n title={Zeta}\n date={1990}\n}\n\\end{biblist}\n""#
    ]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_sort_bibliography_can_sort_all_records_while_preserving_surrounding_text() {
    let elisp_form = r##"(let (question)
         (cl-letf
             (((symbol-function
                'y-or-n-p)
               (lambda (prompt)
                 (setq question prompt)
                 t)))
           (with-temp-buffer
             (insert
              "HEADER\n"
              "\\bib{z}{article}{\n author={Zulu, Zoe}\n date={2020}\n}\n"
              "INTERSTITIAL\n"
              "\\bib{a}{article}{\n author={Alpha, Alice}\n date={2021}\n}\n"
              "FOOTER\n")
             (goto-char (point-min))
             (amsreftex-sort-bibliography)
             (list
              question
              (buffer-string)))))"##;
    let expect = expect![[
        r#"OK ("No biblist env found around point: sort whole buffer? " "HEADER\n\\bib{a}{article}{\n author={Alpha, Alice}\n date={2021}\n}\nINTERSTITIAL\n\\bib{z}{article}{\n author={Zulu, Zoe}\n date={2020}\n}\nFOOTER\n")"#
    ]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_sort_bibliography_decline_leaves_whole_buffer_byte_for_byte_unchanged() {
    let elisp_form = r##"(let ((contents
                        "prefix\n\\bib{b}{book}{title={B}}\n\\bib{a}{book}{title={A}}\nsuffix\n")
                       question)
         (cl-letf
             (((symbol-function
                'y-or-n-p)
               (lambda (prompt)
                 (setq question prompt)
                 nil)))
           (with-temp-buffer
             (insert contents)
             (goto-char (point-min))
             (amsreftex-sort-bibliography)
             (list
              question
              (equal
               contents
               (buffer-string))
              (buffer-string)))))"##;
    let expect = expect![[
        r#"OK ("No biblist env found around point: sort whole buffer? " t "prefix\n\\bib{b}{book}{title={B}}\n\\bib{a}{book}{title={A}}\nsuffix\n")"#
    ]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_sort_end_record_reports_malformed_entries_and_restores_point() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "prefix\n\\bib{broken}{article}{title={unterminated}\n")
         (goto-char (point-min))
         (search-forward "\\bib")
         (beginning-of-line)
         (let ((before (point)))
           (condition-case error
               (progn
                 (amsreftex-sort-endrecfn)
                 (list
                  'returned
                  before
                  (point)))
             (error
              (list
               (car error)
               (cadr error)
               before
               (point))))))"##;
    let expect = expect![[r#"OK (error "Malformed \\bib entry near position 8" 8 8)"#]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_sort_buffer_orders_mixed_case_titles_before_later_keys() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "\\bib{upper}{book}{\n"
          " title={alpha}\n date={2000}\n}\n"
          "\\bib{lower}{book}{\n title={Alpha}\n date={2000}\n}\n"
          "\\bib{later}{book}{\n title={beta}\n date={2000}\n}\n")
         (goto-char (point-min))
         (amsreftex-sort-nextrecfn)
         (amsreftex-sort-buffer-by
          (lambda (left right)
            (amsreftex-compare-by-field
             left right "title")))
         (buffer-string))"##;
    let expect = expect![[
        r#"OK "\\bib{lower}{book}{\n title={Alpha}\n date={2000}\n}\n\\bib{upper}{book}{\n title={alpha}\n date={2000}\n}\n\\bib{later}{book}{\n title={beta}\n date={2000}\n}\n""#
    ]];
    assert_amsreftex_parity(elisp_form, expect);
}

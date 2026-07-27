use expect_test::expect;

use super::assert_amsreftex_parity;

#[test]
fn amsreftex_extract_fields_normalizes_real_nested_amsrefs_metadata() {
    let elisp_form = r##"(amsreftex-extract-fields
         "author = {Doe, Jane}
author = {Roe, Richard}
editor = {Curie, Marie}
date = {2024-07-19}
pages = {10\\ndash 27}
doi = { 10.1000/example }
book = {
 title = {Collected Works}
 editor = {Noether, Emmy}
 date = {1999-12}
 publisher = {Math Press}
}")"##;
    let expect = expect![[
        r#"OK (("year" . "2024") ("month" . "07") ("pages" . "10-27") ("doi" . " 10.1000/example ") ("booktitle" . "Collected Works") ("author" . "Doe, Jane and Roe, Richard") ("editor" . "Curie, Marie"))"#
    ]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_extract_fields_prefixes_nested_keys_dates_and_name_aggregates() {
    let elisp_form = r##"(amsreftex-extract-fields
         "author={Alpha, Ada}
author={Beta, Bob}
editor={Gamma, Grace}
title={Proceedings}
date={2001-03}
pages={1\\ndash 9}"
         "series")"##;
    let expect = expect![[
        r#"OK (("booktitle" . "Proceedings") ("series-year" . "2001") ("series-month" . "03") ("series-pages" . "1-9") ("series-author" . "Alpha, Ada and Beta, Bob") ("series-editor" . "Gamma, Grace"))"#
    ]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_parse_entry_converts_book_articles_into_bibtex_style_collections() {
    let elisp_form = r##"(amsreftex-parse-entry
         "\\bib{chapter-key}{article}{
 author={Lovelace, Ada}
 title={Notes on the Engine}
 date={1843-09}
 pages={101\\ndash 125}
 book={
   title={Scientific Memoirs}
   editor={Taylor, Richard}
   publisher={Richard and John E. Taylor}
 }
}")"##;
    let expect = expect![[
        r#"OK (("&type" . "incollection") ("&key" . "chapter-key") ("title" . "Notes on the Engine") ("year" . "1843") ("month" . "09") ("pages" . "101-125") ("booktitle" . "Scientific Memoirs") ("author" . "Lovelace, Ada"))"#
    ]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_parse_entry_classifies_master_and_phd_theses_and_copies_school() {
    let elisp_form = r##"(mapcar
         #'amsreftex-parse-entry
         '("\\bib{masters}{thesis}{
 type={M.Sc. thesis}
 author={Example, Alice}
 organization={University One}
 date={2020}
}"
           "\\bib{doctoral}{thesis}{
 type={Doctoral dissertation}
 author={Example, Bob}
 organization={University Two}
 date={2021}
}"))"##;
    let expect = expect![[
        r#"OK ((("school" . "University One") ("&type" . "mastersthesis") ("&key" . "masters") ("type" . "M.Sc. thesis") ("organization" . "University One") ("year" . "2020") ("author" . "Example, Alice")) (("school" . "University Two") ("&type" . "phdthesis") ("&key" . "doctoral") ("type" . "Doctoral dissertation") ("organization" . "University Two") ("year" . "2021") ("author" . "Example, Bob")))"#
    ]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_parse_entry_respects_an_explicit_narrowed_region() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "prefix ignored\n"
          "\\bib{inside}{book}{title={Inside} date={2022}}\n"
          "\\bib{outside}{book}{title={Outside} date={2023}}\n")
         (let ((from
                (progn
                  (goto-char (point-min))
                  (search-forward "\\bib{inside}")
                  (line-beginning-position)))
               (to
                (progn
                  (forward-line 1)
                  (point))))
           (amsreftex-parse-entry nil from to)))"##;
    let expect = expect![[r#"OK (("&type" . "book") ("&key" . "inside"))"#]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_get_bib_field_distinguishes_absent_empty_and_present_values() {
    let elisp_form = r##"(let ((entry
                        '(("title" . "A title")
                          ("empty" . "")
                          ("nil-value"))))
         (mapcar
          (lambda (field)
            (list
             field
             (amsreftex-get-bib-field
              field entry)
             (assoc field entry)))
          '("title" "empty" "nil-value" "missing")))"##;
    let expect = expect![[
        r#"OK (("title" "A title" ("title" . "A title")) ("empty" "" ("empty" . "")) ("nil-value" nil ("nil-value")) ("missing" nil nil))"#
    ]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_crossref_expands_starred_parent_book_fields_with_prefixes() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "\\bib*{parent}{book}{
 title={Collected Papers}
 editor={Noether, Emmy}
 date={1935-04}
 publisher={Springer}
}
\\bib{child}{article}{
 title={A Chapter}
 xref={parent}
}")
         (goto-char (point-min))
         (amsreftex-get-crossref-alist
          '(("xref" . "parent"))))"##;
    let expect = expect![[
        r#"OK (("booktitle" . "Collected Papers") ("book-year" . "1935") ("book-month" . "04") ("book-publisher" . "Springer") ("book-editor" . "Noether, Emmy"))"#
    ]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_document_detection_ignores_comments_but_finds_bibselect_and_bib() {
    let elisp_form = r##"(mapcar
         (lambda (contents)
           (with-temp-buffer
             (insert contents)
             (let ((match
                    (amsreftex-using-amsrefs-p)))
               (and match
                    (list
                     match
                     (match-string-no-properties
                      0))))))
         '("% \\bibselect{ignored}\nordinary text"
           "Text before \\bibselect{alpha,beta}\n"
           "% \\bib{ignored}{book}{}\n  \\bib{real}{article}{title={T}}"
           "\\bibliography{plain}"))"##;
    let expect =
        expect![[r#"OK (nil (23 "Text before \\bibselect") (47 "  \\bib{real}{article}{") nil)"#]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_locate_bibliography_files_splits_scanned_and_explicit_databases() {
    let elisp_form = r##"(cl-letf
         (((symbol-function
            'reftex-locate-file)
           (lambda (name extension master)
             (unless
                 (equal name "missing")
               (format
                "%s/%s.%s"
                master name extension)))))
         (with-temp-buffer
           (insert
            "\\bibselect[labels={A}]{alpha, beta,\n gamma}\n")
           (list
            (amsreftex-locate-bibliography-files
             "/project")
            (amsreftex-locate-bibliography-files
             "/explicit"
             '("one" "missing" "two")))))"##;
    let expect = expect![[
        r#"OK (("/project/alpha.ltb" "/project/beta.ltb" "/project/gamma.ltb") ("/explicit/one.ltb" "/explicit/two.ltb"))"#
    ]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_extract_entries_filters_multiple_fields_and_resolves_crossrefs() {
    let elisp_form = r##"(cl-letf
         (((symbol-function
            'reftex-format-bib-entry)
           (lambda (entry)
             (format
              "%s|%s|%s"
              (cdr
               (assoc "&key" entry))
              (cdr
               (assoc "author" entry))
              (cdr
               (assoc "title" entry))))))
         (with-temp-buffer
           (insert
            "\\bib{alpha}{article}{author={Doe, Jane} title={Algebra} date={2020}}\n"
            "\\bib{beta}{article}{author={Roe, Richard} title={Geometry} date={2021} xref={parent}}\n"
            "\\bib*{parent}{book}{title={Collected Geometry} editor={Noether, Emmy} date={1999}}\n")
           (mapcar
            (lambda (entry)
              (list
               (car entry)
               (cdr
                (assoc "&type" entry))
               (cdr
                (assoc "title" entry))
               (cdr
                (assoc "booktitle" entry))
               (cdr
                (assoc "&formatted" entry))))
            (amsreftex--extract-entries
             '("Geometry" "Roe")
             (current-buffer)))))"##;
    let expect = expect![[r#"OK (("beta" "article" nil nil "beta|nil|nil"))"#]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_end_of_entry_handles_amsrefs_bibitems_plain_lists_and_malformed_input() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (with-temp-buffer
             (insert (car case))
             (goto-char (point-min))
             (let ((end
                    (amsreftex-end-of-bib-entry
                     (cdr case))))
               (list
                end
                (buffer-substring-no-properties
                 (point-min)
                 end)))))
         '(("\\bib{k}{article}{title={Nested {Title}} date={2024}}TAIL")
           ("\\bibitem{k} text line\ncontinued\n\\bibitem{next} more" . t)
           ("{one {two} three}TAIL")
           ("\\bib{broken}{article}{unterminated" . nil)))"##;
    let expect = expect![[
        r#"OK ((53 "\\bib{k}{article}{title={Nested {Title}} date={2024}}") (32 "\\bibitem{k} text line\ncontinued") (18 "{one {two} three}") (35 "\\bib{broken}{article}{unterminated"))"#
    ]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_strip_latex_normalizes_accents_commands_and_grouping_for_sorting() {
    let elisp_form = r##"(mapcar
         #'amsreftex-strip-LaTeX
         '("\\\"{O}rsted"
           "\\v Cech"
           "{\\AA}ngstrom"
           "Garc\\'ia"
           "de~la~Vall\\'ee Poussin"
           "plain"))"##;
    let expect =
        expect![[r#"OK ("Orsted" "Cech" "AAngstrom" "Garcia" "de~la~Vallee Poussin" "plain")"#]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_name_parts_honor_order_missing_names_and_initials() {
    let elisp_form = r##"(list
         (let ((amsreftex-sort-name-parts
                '(last initial first)))
           (mapcar
            #'amsreftex-get-name-parts
            '("Lovelace, Ada"
              "Noether, Emmy"
              "Mononym"
              "")))
         (let ((amsreftex-sort-name-parts nil))
           (amsreftex-get-name-parts
            "Hopper, Grace Murray")))"##;
    let expect = expect![[
        r#"OK ((("Lovelace" "A" "Ada") ("Noether" "E" "Emmy") ("Mononym" "" "") ("" "" "")) ("Hopper" "Grace Murray"))"#
    ]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_bib_name_list_prefers_authors_then_editors_and_normalizes_each_name() {
    let elisp_form = r##"(let ((amsreftex-sort-name-parts
                        '(last initial)))
         (mapcar
          #'amsreftex-get-bib-name-list
          '((("author" .
              "Lovelace, Ada and Hopp\\'er, Grace"))
            (("editor" .
              "Noether, Emmy and Curie, Marie"))
            (("title" . "Anonymous Work"))
            (("author" . "")
             ("editor" . "Fallback, Editor")))))"##;
    let expect = expect![[
        r#"OK ((("lovelace" "a") ("hopper" "g")) (("noether" "e") ("curie" "m")) ("") (("" "")))"#
    ]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_compare_lists_implements_lexicographic_prefix_and_equality_rules() {
    let elisp_form = r##"(mapcar
         (lambda (pair)
           (list
            (amsreftex-compare-lists
             (car pair)
             (cadr pair)
             #'<)
            (amsreftex-compare-lists
             (cadr pair)
             (car pair)
             #'<)))
         '(((1 2 3) (1 2 4))
           ((1 2) (1 2 0))
           ((1 2) (1 2))
           (() (1))
           ((3) (2))))"##;
    let expect = expect!["OK ((t nil) (t nil) (nil nil) (t nil) (nil t))"];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_comparators_order_authors_years_and_fallback_fields_practically() {
    let elisp_form = r##"(let* ((amsreftex-sort-name-parts
                         '(last initial))
               (ada
                '(("author" .
                   "Lovelace, Ada")
                  ("year" . "1843")
                  ("title" . "Notes")))
               (grace
                '(("author" .
                   "Hopper, Grace")
                  ("year" . "1952")
                  ("title" . "Compiler")))
               (later
                '(("editor" .
                   "Lovelace, Ada")
                  ("year" . "2000")
                  ("title" . "Zeta"))))
         (list
          (amsreftex-compare-author
           grace ada)
          (amsreftex-compare-author
           ada grace)
          (amsreftex-compare-author
           ada later)
          (amsreftex-compare-year
           ada grace)
          (amsreftex-compare-year
           later grace)
          (amsreftex-compare-by-field
           grace later "title")
          (amsreftex-compare-by-field
           later grace "title")))"##;
    let expect = expect!["OK (t nil nil t nil t nil)"];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_filter_args_only_clears_the_last_argument_for_amsrefs_documents() {
    let elisp_form = r##"(let ((reftex-docstruct-symbol
                        'amsreftex-test-docstruct))
         (unwind-protect
             (list
              (progn
                (set
                 reftex-docstruct-symbol
                 '((database . "amsrefs")))
                (amsreftex-set-last-arg-to-nil
                 (list 'entry 'format t)))
              (progn
                (set
                 reftex-docstruct-symbol
                 '((bib . ("plain.bib"))))
                (amsreftex-set-last-arg-to-nil
                 (list 'entry 'format t))))
           (makunbound
            reftex-docstruct-symbol)))"##;
    let expect = expect!["OK ((entry format nil) (entry format t))"];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_pop_to_database_entry_returns_exact_record_and_restores_context() {
    let elisp_form = r##"(let ((database
                        (get-buffer-create
                         " *amsreftex-database*"))
                       (origin
                        (current-buffer)))
         (unwind-protect
             (progn
               (with-current-buffer database
                 (erase-buffer)
                 (insert
                  "prefix\n"
                  "\\bib{target}{article}{title={Nested {Result}} date={2024}}\n"
                  "\\bib{other}{book}{title={Other}}\n")
                 (goto-char 3))
               (cl-letf
                   (((symbol-function
                      'reftex-get-file-buffer-force)
                     (lambda (&rest _)
                       database)))
                 (let ((result
                        (amsreftex-pop-to-database-entry
                         "target"
                         '("database.ltb")
                         nil nil nil t)))
                   (list
                    result
                    (eq
                     (current-buffer)
                     origin)
                    (with-current-buffer database
                      (point))))))
           (kill-buffer database)))"##;
    let expect = expect![[r#"OK ("\\bib{target}" t 3)"#]];
    assert_amsreftex_parity(elisp_form, expect);
}

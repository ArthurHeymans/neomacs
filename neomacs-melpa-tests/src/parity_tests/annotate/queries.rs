use expect_test::expect;

use super::assert_annotate_parity;

#[test]
fn annotate_summary_lexer_tokenizes_complex_query_and_consumes_input() {
    let elisp_form = r##"(let ((annotate-summary-query
               "src/.* and (TODO or not \"needs review\")"))
         (let (tokens states token)
           (while
               (not
                (eq
                 (setq token (annotate-summary-lexer))
                 :no-more-tokens))
             (push token tokens)
             (push annotate-summary-query states))
           (list (nreverse tokens)
                 (nreverse states)
                 annotate-summary-query
                 token)))"##;
    let expect = expect![[
        r#"OK (((re "src/.*" 0 6) (and "and" 1 4) (open-par "(" 1 2) (re "TODO" 0 4) (or "or" 1 3) (not "not" 1 4) (escaped-re "\"needs review\"" 1 15) (close-par ")" 0 1)) (" and (TODO or not \"needs review\")" " (TODO or not \"needs review\")" "TODO or not \"needs review\")" " or not \"needs review\")" " not \"needs review\")" " \"needs review\")" ")" "") "" :no-more-tokens)"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_summary_lexer_lookahead_does_not_consume_query() {
    let elisp_form = r##"(let ((annotate-summary-query "file and note"))
         (let ((first (annotate-summary-lexer t))
               (after-first annotate-summary-query)
               (second (annotate-summary-lexer))
               (after-second annotate-summary-query))
           (list first after-first second after-second)))"##;
    let expect = expect![[r#"OK ((re "file" 0 4) "file and note" (re "file" 0 4) " and note")"#]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_summary_filter_matches_file_and_nested_note_expression() {
    let elisp_form = r##"(let ((db
               '(("/src/main.rs"
                  ((1 4 "TODO optimize parser" "let" 0 :by-length "a" nil)
                   (8 12 "important safety review" "unsafe" 1 :new-line "b" nil))
                  "sum-a")
                 ("/src/lib.el"
                  ((2 6 "TODO document API" "defun" 0 :by-length "c" nil))
                  "sum-b")
                 ("/docs/readme.org"
                  ((3 9 "important release note" "heading" 2 :by-length "d" nil))
                  "sum-c"))))
         (list
          (annotate-summary-filter-db db ".*\\.rs and TODO or important" nil)
          (annotate-summary-filter-db db ".*\\.el and TODO" nil)
          (annotate-summary-filter-db db "docs/ and not TODO" nil)))"##;
    let expect = expect![[
        r#"OK ((("/src/main.rs" ((1 4 "TODO optimize parser" "let" 0 :by-length "a" nil) (8 12 "important safety review" "unsafe" 1 :new-line "b" nil)) "sum-a")) (("/src/lib.el" ((2 6 "TODO document API" "defun" 0 :by-length "c" nil)) "sum-b")) (("/docs/readme.org" ((3 9 "important release note" "heading" 2 :by-length "d" nil)) "sum-c")))"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_summary_filter_supports_quoted_regex_with_spaces() {
    let elisp_form = r##"(let ((db
               '(("/work notes/demo.txt"
                  ((1 4 "needs careful review" "abc" 0 :by-length "a" nil)
                   (8 12 "ship it" "def" 0 :by-length "b" nil))
                  "sum"))))
         (list
          (annotate-summary-filter-db db "\"work notes\" and \"careful review\"" nil)
          (annotate-summary-filter-db db "\"missing path\" or \"ship it\"" nil)))"##;
    let expect = expect!["OK (nil nil)"];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_summary_filter_cutoff_removes_annotations_ending_before_point() {
    let elisp_form = r##"(let ((db
               '(("/a"
                  ((20 25 "third" "cc" 0 :by-length "c" nil)
                   (1 5 "first" "aa" 0 :by-length "a" nil)
                   (10 15 "second" "bb" 0 :by-length "b" nil))
                  "sum"))))
         (list
          (annotate-summary-filter-db db ".*" nil)
          (annotate-summary-filter-db db ".*" 14)
          (annotate-summary-filter-db db ".*" 26)))"##;
    let expect = expect![[
        r#"OK ((("/a" ((1 5 "first" "aa" 0 :by-length "a" nil) . #1=((10 15 "second" "bb" 0 :by-length "b" nil) (20 25 "third" "cc" 0 :by-length "c" nil))) "sum")) (("/a" #1# "sum")) (("/a" nil "sum")))"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_summary_parser_reports_exact_invalid_query_errors() {
    let elisp_form = r##"(let ((db
               '(("/a" ((1 2 "note" "a" 0 :by-length "id" nil)) "sum"))))
         (mapcar
          (lambda (query)
            (condition-case err
                (annotate-summary-filter-db db query nil)
              (error (list query (car err) (cdr err)))))
          '(".* and" ".* or" ".* xor note"
            ".* and not" ".* and (note" ".* and note extra")))"##;
    let expect = expect![[
        r#"OK ((".* and" annotate-query-parsing-error ("No more input after 'and'")) (("/a" ((1 2 "note" "a" 0 :by-length "id" nil)) "sum")) (".* xor note" annotate-query-parsing-error ("Unknown operator: xor is not in '(and, or)")) (".* and not" annotate-query-parsing-error ("No more input after 'not'")) (".* and (note" annotate-query-parsing-error ("Unmatched parens")) (".* and note extra" annotate-query-parsing-error ("Expecting for operator ('and' or 'or') or \")\". found \"extra\" instead")))"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_summary_note_parser_obeys_and_or_not_evaluation() {
    let elisp_form = r##"(let ((annotation '(1 4 "TODO important review" "abc"))
               (filter (lambda (regex item)
                         (and (string-match-p
                               regex
                               (annotate-annotation-string item))
                              item))))
         (mapcar
          (lambda (query)
            (let ((annotate-summary-query query))
              (condition-case err
                  (list query
                        (not
                         (null
                          (annotate-summary-query-parse-note
                           filter annotation))))
                (error (list query (car err) (cdr err))))))
          '("TODO" "missing" "TODO and important"
            "TODO and not missing" "missing or review"
            "(TODO or missing) and important"
            "not TODO")))"##;
    let expect = expect![[
        r#"OK (("TODO" t) ("missing" nil) ("TODO and important" t) ("TODO and not missing" t) ("missing or review" t) ("(TODO or missing) and important" annotate-query-parsing-error ("Unmatched parens")) ("not TODO" nil))"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_database_empty_predicate_distinguishes_empty_records_from_notes() {
    let elisp_form = r##"(mapcar
         #'annotate--db-empty-p
         '(nil
           (("/a" nil "sum"))
           (("/a" (nil nil) "sum"))
           (("/a" ((1 2 "note" "x")) "sum"))
           (("/a" nil "sum") ("/b" ((1 2 "note" "x")) "sum-b"))))"##;
    let expect = expect!["OK (t t t nil nil)"];
    assert_annotate_parity(elisp_form, expect);
}

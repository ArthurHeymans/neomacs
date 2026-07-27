use expect_test::expect;

use super::assert_adoc_mode_parity;

#[test]
fn adoc_mode_anchor_attribute_and_section_collectors_cover_all_supported_forms() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          ":project-name: Demo\n"
          ":project-name-version: 2\n"
          ":empty:\n"
          "[[classic]]\n"
          "[[classic-label,Label]]\n"
          "anchor:macro[Label]\n"
          "[#short]\n"
          "[role#styled]\n"
          "= Document\n\n"
          "== First Section\n\n"
          "[source,ruby]\n----\n== Fake Section\n----\n"
          "=== Crème & API_v2\n")
         (adoc-mode)
         (list
          (adoc--collect-anchor-ids)
          (seq-filter
           (lambda (name)
             (or (string-prefix-p "project" name)
                 (equal name "empty")))
           (adoc--collect-attribute-names))
          (mapcar
           (lambda (section)
             (list (nth 0 section)
                   (substring-no-properties (nth 1 section))
                   (nth 2 section)))
           (adoc--collect-sections))
          (adoc--collect-section-ids)))"##;
    let expect = expect![[
        r#"OK (("classic-label" "classic" "styled" "short") ("empty" "project-name-version" "project-name") (("_first_section" "First Section" 145) ("_crème_api_v2" "Crème & API_v2" 203)) ("_first_section" "_crème_api_v2"))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_completion_context_precedence_and_bounds_cover_xref_attribute_source_include_and_prose()
 {
    let elisp_form = r##"(cl-labels
         ((probe
           (text)
           (with-temp-buffer
             (insert
              ":project: Demo\n"
              "[[alpha]]\n"
              "anchor:beta[]\n"
              text)
             (adoc-mode)
             (goto-char (point-max))
             (let ((capf (adoc-completion-at-point)))
               (when capf
                 (let ((start (nth 0 capf))
                       (end (nth 1 capf))
                       (table (nth 2 capf)))
                   (list
                    (buffer-substring-no-properties start end)
                    (all-completions
                     (buffer-substring-no-properties start end)
                     table))))))))
       (mapcar
        #'probe
        '("See <<a"
          "See xref:b"
          "See <<alpha>>"
          "Value {pro"
          "See <<alpha,{pro"
          "[source,ru"
          "[source,{pro"
          "include::cha"
          "include::chapter.adoc["
          "ordinary prose")))"##;
    let expect = expect![[
        r#"OK (("a" ("alpha")) ("b" nil) nil ("pro" ("project")) ("pro" ("project")) ("ru" ("ruby" "rust")) ("pro" ("project")) ("cha" nil) nil nil)"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_language_resolution_and_completion_cover_mappings_candidates_fallbacks_and_deduplication()
 {
    let elisp_form = r##"(cl-letf
         (((symbol-function 'adoc-test-primary-mode) (lambda ()))
          ((symbol-function 'adoc-test-fallback-mode) (lambda ()))
          ((symbol-function 'adoc-test-direct-mode) (lambda ())))
       (let ((adoc-code-lang-modes
              '(("primary" . adoc-test-primary-mode)
                ("fallback"
                 . (adoc-test-absent-mode adoc-test-fallback-mode))
                ("direct" . adoc-test-direct-mode)
                ("dupe"
                 . (adoc-test-primary-mode adoc-test-primary-mode))
                ("missing" . adoc-test-absent-mode))))
         (list
          (mapcar
           #'adoc-get-lang-mode
           '("primary" "fallback" "direct" "missing" "unknown"))
          (adoc--completion-langs))))"##;
    let expect = expect![[
        r#"OK ((adoc-test-primary-mode adoc-test-fallback-mode adoc-test-direct-mode nil nil) ("primary" "fallback" "direct" "dupe" "missing" "c" "clojure" "cpp" "csharp" "css" "diff" "elixir" "emacs-lisp" "erlang" "go" "groovy" "haskell" "html" "java" "javascript" "json" "kotlin" "lua" "ocaml" "perl" "php" "python" "ruby" "rust" "scala" "sh" "shell" "sql" "swift" "toml" "typescript" "xml" "yaml"))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

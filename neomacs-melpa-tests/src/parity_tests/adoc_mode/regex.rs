use expect_test::expect;

use super::assert_adoc_mode_parity;

#[test]
fn adoc_mode_title_builders_and_matchers_cover_levels_styles_and_boundaries() {
    let elisp_form = r##"(let ((cases
                '("= Document"
                  "== Section ="
                  "====== Deep ======"
                  "Title\n====="
                  "Title\n-----"
                  "not = a title"
                  "======= too deep")))
         (list
          (mapcar
           (lambda (level)
             (list level
                   (adoc-make-one-line-title 1 level "Heading")
                   (adoc-make-one-line-title 2 level "Heading")))
           '(0 1 2 3 4 5))
          (mapcar
           (lambda (level)
             (list level
                   (adoc-make-two-line-title level "Heading")
                   (adoc-make-two-line-title-underline level 9)))
           '(0 1 2 3 4))
          (mapcar
           (lambda (text)
             (with-temp-buffer
               (insert text)
               (goto-char (point-min))
               (list
                (and (re-search-forward
                      (adoc-re-one-line-title nil) nil t)
                     (match-string-no-properties 0))
                (let (match)
                  (dolist (delimiter adoc-two-line-title-del)
                    (goto-char (point-min))
                    (when (re-search-forward
                           (adoc-re-two-line-title delimiter) nil t)
                      (setq match (match-string-no-properties 0))))
                  match))))
           cases)))"##;
    let expect = expect![[
        r#"OK (((0 "= Heading" "= Heading =") (1 "== Heading" "== Heading ==") (2 "=== Heading" "=== Heading ===") (3 "==== Heading" "==== Heading ====") (4 "===== Heading" "===== Heading =====") (5 "====== Heading" "====== Heading ======")) ((0 "Heading\n=======" "=========") (1 "Heading\n-------" "---------") (2 "Heading\n~~~~~~~" "~~~~~~~~~") (3 "Heading\n^^^^^^^" "^^^^^^^^^") (4 "Heading\n+++++++" "+++++++++")) (("= Document" nil) ("== Section =" nil) ("====== Deep ======" nil) (nil "Title\n=====") (nil "Title\n-----") (nil nil) (nil nil)))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_list_regexes_and_marker_builders_cover_all_supported_forms() {
    let elisp_form = r##"(let ((lines
                '("- bullet" "* bullet" "***** deep"
                  ". implicit" "... implicit"
                  "1. arabic" "a. alpha" "Z) alpha"
                  "iv) roman" "label:: text" "label;; text"
                  "[x] checked" "plain")))
         (list
          (mapcar
           (lambda (line)
             (with-temp-buffer
               (insert line)
               (goto-char (point-min))
               (mapcar
                (lambda (spec)
                  (let ((regexp (apply #'adoc-re-oulisti spec)))
                    (and (looking-at regexp)
                         (list (match-string-no-properties 0)
                               (match-data)))))
                '((adoc-unordered nil nil)
                  (adoc-explicitly-numbered nil nil)
                  (adoc-implicitly-numbered nil nil)
                  (adoc-callout nil nil)))))
           lines)
          (mapcar
           (lambda (level)
             (list level
                   (adoc-make-uolisti level t)
                   (adoc-make-uolisti level nil)))
           '(0 1 2 3 4 5 6))
          (mapcar
           (lambda (spec)
             (condition-case error
                 (apply #'adoc-re-llisti spec)
               (error (list (car error) (cdr error)))))
           '((adoc-labeled-normal 0)
             (adoc-labeled-normal 1)
             (adoc-labeled-normal 2)
             (adoc-labeled-normal 3)
             (adoc-labeled-qanda 0)
             (adoc-labeled-glossary 0)))))"##;
    let expect = expect![[
        r#"OK (((("- " ((:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil))) nil nil nil) (("* " ((:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil))) nil nil nil) (("***** " ((:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil))) nil nil nil) (nil nil (". " ((:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil))) nil) (nil nil ("... " ((:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil))) nil) (nil ("1. " ((:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil))) nil nil) (nil ("a. " ((:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil))) nil nil) (nil nil nil nil) (nil ("iv) " ((:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil))) nil nil) (nil nil nil nil) (nil nil nil nil) (nil nil nil nil) (nil nil nil nil)) ((0 "- " "  ") (1 "* " "  ") (2 "\11** " "   ") (3 "\11*** " "    ") (4 "\11\11**** " "     ") (5 "\11\11***** " "      ") (6 "\11\11\11****** " "       ")) ("^\\([ \11]*\\)\\(.*[^:\n]\\)\\(\\(::\\)\\(?:[ \11]+\\|$\\)\\)" "^\\([ \11]*\\)\\(.*[^;\n]\\)\\(\\(;;\\)\\(?:[ \11]+\\|$\\)\\)" "^\\([ \11]*\\)\\(.*[^:\n]\\)\\(\\(:::\\)\\(?:[ \11]+\\|$\\)\\)" "^\\([ \11]*\\)\\(.*[^:\n]\\)\\(\\(::::\\)\\(?:[ \11]+\\|$\\)\\)" "^\\([ \11]*\\)\\(.*[^ \11\n]\\)\\(\\(\\?\\?\\)\\)$" "^\\(\\)\\(.*[^ \11\n]\\)\\(\\(:-\\)\\)$"))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_block_anchor_xref_and_macro_regexes_cover_valid_and_invalid_syntax() {
    let elisp_form = r##"(let ((samples
                '("[[id]]" "[[id,label]]" "anchor:id[label]"
                  "[#short]" "[style#short]"
                  "<<id>>" "<<id,label>>" "xref:id[label]"
                  "image::path.png[Alt]" "include::part.adoc[]"
                  "video::clip.mp4[]" "plain")))
         (mapcar
          (lambda (sample)
            (with-temp-buffer
              (insert sample)
              (goto-char (point-min))
              (mapcar
               (lambda (regexp)
                 (and (re-search-forward regexp nil t)
                      (match-string-no-properties 0)))
               (list (adoc-re-anchor)
                     (adoc-re-xref)
                     (adoc-re-block-macro)
                     (adoc-re-block-macro "include")))))
          samples))"##;
    let expect = expect![[
        r#"OK (("[[id]]" nil nil nil) ("[[id,label]]" nil nil nil) ("anchor:id[label]" nil nil nil) ("[#short]" nil nil nil) ("[style#short]" nil nil nil) (nil "<<id>>" nil nil) (nil "<<id,label>>" nil nil) (nil "xref:id[label]" nil nil) (nil nil "image::path.png[Alt]" nil) (nil nil "include::part.adoc[]" nil) (nil nil "video::clip.mp4[]" nil) (nil nil nil nil))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_section_id_algorithms_cover_asciidoctor_antora_and_document_attributes() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (spec)
            (apply #'adoc--section-id spec))
          '(("Hello, World!" "_" "_")
            ("C++ & Rust: A/B" "_" "_")
            ("Dots... dashes--- spaces" "_" "_")
            ("Hello, World!" "" "")
            ("Hello, World!" "" "-")
            ("Crème brûlée & API_v2" "" "-")
            ("Multiple___separators" "_" "_")))
         (mapcar
          (lambda (style)
            (with-temp-buffer
              (insert ":idprefix: zz_\n:idseparator: .\n\n= Doc\n")
              (let ((adoc-section-id-style style)
                    (buffer-file-name nil))
                (adoc--section-id-params))))
          '(asciidoctor antora auto)))"##;
    let expect = expect![[
        r#"OK (("_hello_world" "_c_rust_ab" "_dots_dashes_spaces" "helloworld" "hello-world" "crème-brûlée-api_v2" "_multiple_separators") (("_" . "_") ("" . "-") ("_" . "_")))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

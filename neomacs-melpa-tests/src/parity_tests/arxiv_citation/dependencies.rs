use expect_test::expect;

use super::assert_arxiv_citation_parity;

#[test]
fn exact_dash_and_s_dependencies_are_installed_loaded_and_own_every_used_primitive() {
    let elisp_form = r##"(list
 (mapcar
  (lambda (name)
    (let ((descriptor
           (cadr (assq name package-alist))))
      (list
       name
       (featurep name)
       (package-installed-p name)
       (package-version-join
        (package-desc-version descriptor))
       (package-desc-reqs descriptor)
       (package-desc-summary descriptor))))
  '(dash s))
 (mapcar
  (lambda (symbol)
    (list
     symbol
     (fboundp symbol)
     (macrop symbol)
     (file-name-base
      (symbol-file symbol 'defun))))
  '(->>
    -compose
    -take
    --filter
    -map
    --map
    -last-item
    -butlast
    s-match
    s-chop-suffix
    s-contains?
    s-replace
    s-replace-regexp
    s-replace-all
    s-split
    s-trim
    s-blank?
    s-prefix?)))"##;
    let expect = expect![[
        r#"OK (((dash t t "20260221.1346" ((emacs (24))) "A modern list library for Emacs.") (s t t "20220902.1511" nil "The long lost Emacs string manipulation library.")) ((->> t t "dash") (-compose t nil "dash") (-take t nil "dash") (--filter t t "dash") (-map t nil "dash") (--map t t "dash") (-last-item t nil "dash") (-butlast t nil "dash") (s-match t nil "s") (s-chop-suffix t nil "s") (s-contains? t nil "s") (s-replace t nil "s") (s-replace-regexp t nil "s") (s-replace-all t nil "s") (s-split t nil "s") (s-trim t nil "s") (s-blank? t nil "s") (s-prefix? t nil "s")))"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}

#[test]
fn real_dash_threading_anaphoric_macros_and_s_transformations_form_a_practical_pipeline() {
    let elisp_form = r##"(let ((raw
        '("  Ada Lovelace  "
          ""
          "  ALAN TURING"
          " Grace_Hopper "
          "   ")))
  (list
   (->> raw
        (--map (s-trim it))
        (--filter (not (s-blank? it)))
        (--map (s-replace "_" " " it))
        (-map #'downcase))
   (-take 2 raw)
   (s-replace-all
    '((" " . "-")
      ("_" . "-"))
    "category theory_foundations")
   (s-match
    "\\`\\([[:alpha:]]+\\)-\\([[:alpha:]]+\\)\\'"
    "arxiv-citation")))"##;
    let expect = expect![[
        r#"OK (("ada lovelace" "alan turing" "grace hopper") ("  Ada Lovelace  " "") "category-theory-foundations" ("arxiv-citation" "arxiv" "citation"))"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}

#[test]
fn package_pdf_naming_exercises_dash_composition_take_and_s_cleanup_together() {
    let elisp_form = r##"(let ((arxiv-citation-library
        "/research/library")
       (info
        '(:authors
          ("Lovelace, Ada"
           "van Rossum, Guido"
           "O'Neil, Shaquille")
          :title
          "Structured_Proofs: $(Co)Algebras, {Types}; Appendix")))
  (list
   (let ((arxiv-citation-max-authors nil))
     (arxiv-citation-pdf-name info))
   (let ((arxiv-citation-max-authors 2))
     (arxiv-citation-pdf-name info))
   (let ((arxiv-citation-max-authors 0))
     (arxiv-citation-pdf-name info))))"##;
    let expect = expect![[
        r#"OK ("/research/library/lovelace-van rossum-o'neil_structured-proofs.pdf" "/research/library/lovelace-van rossum_structured-proofs.pdf" "/research/library/structured-proofs.pdf")"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}

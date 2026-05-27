use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_include_keyword_expands_file_content_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let* ((root (make-temp-file "org-include" t))
         (inc (expand-file-name "inc.org" root)))
    (unwind-protect
        (progn
          (with-temp-file inc
            (insert "#+MACRO: incmacro Included $1\n")
            (insert "* Included\n")
            (insert "Body {{{incmacro(value)}}}\n"))
          (with-temp-buffer
            (org-mode)
            (insert "#+TITLE: Main\n")
            (insert "#+INCLUDE: \"" inc "\"\n")
            (insert "* Local\nBody\n")
            (goto-char (point-min))
            (org-export-expand-include-keyword nil root nil nil nil)
            (buffer-substring-no-properties (point-min) (point-max))))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_macro_escape_extract_replace_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-macro)
  (with-temp-buffer
    (org-mode)
    (insert "#+MACRO: count (eval (number-to-string (1+ (string-to-number $1))))\n")
    (insert "#+MACRO: wrap [$1|$2]\n")
    (insert "Value {{{count(4)}}}; {{{wrap(a,b)}}}; escaped {{{wrap(x\\,y,z)}}}.\n")
    (let ((templates (org-macro--collect-macros)))
      (list (org-macro-escape-arguments "x,y" "z")
            (org-macro-extract-arguments "x\\,y,z")
            (org-macro-expand "wrap(a,b)" templates)
            (progn
              (org-macro-replace-all templates)
              (buffer-substring-no-properties
               (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_macro_html_export_markup_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox-html)
  (with-temp-buffer
    (org-mode)
    (insert "#+TITLE: X\n")
    (insert "#+MACRO: emph /$1/\n")
    (insert "* H\n{{{emph(text)}}}\n")
    (let* ((org-export-with-toc nil)
           (html (org-export-as 'html nil nil t nil)))
      (list (not (null (string-match-p "<i>text</i>" html)))
            (replace-regexp-in-string
             "org[[:alnum:]]+"
             "org-id"
             html)))))"##,
    );
}

#[test]
fn org_include_location_only_contents_footnotes_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let* ((root (make-temp-file "org-include-location" t))
         (inc (expand-file-name "chapters.org" root)))
    (unwind-protect
        (progn
          (with-temp-file inc
            (insert "#+TITLE: Included\n")
            (insert "* Prelude\nSkip me.\n")
            (insert "* Target\n")
            (insert "SCHEDULED: <2026-05-27 Wed>\n")
            (insert ":PROPERTIES:\n:CUSTOM_ID: target\n:END:\n")
            (insert "First body [fn:local].\n")
            (insert "** Child\nChild body.\n")
            (insert "[fn:local] Included footnote.\n"))
          (with-temp-buffer
            (org-mode)
            (insert "#+TITLE: Main\n")
            (insert "* Parent\n")
            (insert "#+INCLUDE: \"" inc "::* Target\" :only-contents t :minlevel 3\n")
            (insert "* After\n")
            (goto-char (point-min))
            (org-export-expand-include-keyword nil root nil nil nil)
            (let ((tree (org-element-parse-buffer)))
              (list (org-element-map tree 'headline
                      (lambda (h)
                        (list (org-element-property :level h)
                              (org-element-property :raw-value h))))
                    (org-element-map tree 'footnote-definition
                      (lambda (f) (org-element-property :label f)))
                    (buffer-substring-no-properties
                     (point-min) (point-max)))))))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_include_literal_blocks_lines_parse_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let* ((root (make-temp-file "org-include-literal" t))
         (src (expand-file-name "snippet.el" root))
         (txt (expand-file-name "notes.txt" root)))
    (unwind-protect
        (progn
          (with-temp-file src
            (insert ";; one\n(message \"two\")\n(message \"three\")\n;; four\n"))
          (with-temp-file txt
            (insert "alpha\nbeta <tag>\ngamma\n"))
          (with-temp-buffer
            (org-mode)
            (insert "#+INCLUDE: \"" src "\" src emacs-lisp :lines \"2-3\" -n\n")
            (insert "#+INCLUDE: \"" txt "\" example :lines \"1-2\"\n")
            (goto-char (point-min))
            (let ((parsed-src (org-export-parse-include-value
                               (concat "\"" src "\" src emacs-lisp :lines \"2-3\" -n")
                               root))
                  (parsed-example (org-export-parse-include-value
                                   (concat "\"" txt "\" example :lines \"1-2\"")
                                   root)))
              (org-export-expand-include-keyword nil root nil nil nil)
              (list parsed-src
                    parsed-example
                    (buffer-substring-no-properties
                     (point-min) (point-max)))))))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_macro_counter_nested_replacement_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-macro)
  (with-temp-buffer
    (org-mode)
    (insert "#+MACRO: wrap <<$1>>\n")
    (insert "#+MACRO: pair $1={{{$2}}}\n")
    (insert "A {{{counter(seq)}}}; B {{{counter(seq,+3)}}}; ")
    (insert "C {{{counter(seq)}}}; D {{{counter(seq,-1)}}}; ")
    (insert "E {{{wrap(text)}}}; F {{{pair(label,wrap(value))}}}.\n")
    (let ((templates (org-macro--collect-macros)))
      (list (mapcar #'car templates)
            (org-macro-expand "wrap(text)" templates)
            (org-macro-expand "pair(label,wrap(value))" templates)
            (progn
              (org-macro-replace-all templates)
              (buffer-substring-no-properties
               (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_include_nested_macro_footnote_export_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox-html)
  (require 'org-macro)
  (let* ((root (make-temp-file "org-include-nested" t))
         (sub (expand-file-name "sub" root))
         (inner (expand-file-name "inner.org" sub))
         (outer (expand-file-name "outer.org" root)))
    (unwind-protect
        (progn
          (make-directory sub)
          (with-temp-file inner
            (insert "#+MACRO: inner /Inner $1/\n")
            (insert "* Inner Head\n")
            (insert "Inner body {{{inner(value)}}} [fn:inner].\n")
            (insert "[fn:inner] Inner footnote.\n"))
          (with-temp-file outer
            (insert "#+MACRO: outer *Outer $1*\n")
            (insert "* Outer Head\n")
            (insert "Outer body {{{outer(value)}}}.\n")
            (insert "#+INCLUDE: \"sub/inner.org\" :minlevel 2\n")
            (insert "[fn:outer] Outer footnote.\n"))
          (with-temp-buffer
            (org-mode)
            (insert "#+TITLE: Main\n")
            (insert "#+MACRO: main =Main $1=\n")
            (insert "* Main Head\n")
            (insert "Main body {{{main(value)}}} [fn:outer].\n")
            (insert "#+INCLUDE: \"" outer "\" :minlevel 2\n")
            (goto-char (point-min))
            (org-export-expand-include-keyword nil root nil nil nil)
            (let* ((expanded (buffer-substring-no-properties
                              (point-min) (point-max)))
                   (templates (org-macro--collect-macros))
                   (macro-output (progn
                                   (org-macro-replace-all templates)
                                   (buffer-substring-no-properties
                                    (point-min) (point-max))))
                   (tree (org-element-parse-buffer))
                   (html (replace-regexp-in-string
                          "org[[:alnum:]]+"
                          "org-id"
                          (org-export-as 'html nil nil t nil))))
              (list (mapcar #'car templates)
                    (org-element-map tree 'headline
                      (lambda (h)
                        (list (org-element-property :level h)
                              (org-element-property :raw-value h))))
                    (org-element-map tree 'footnote-definition
                      (lambda (f) (org-element-property :label f)))
                    expanded
                    macro-output
                    (not (null (string-match-p "<b>Outer value</b>" html)))
                    (not (null (string-match-p "<i>Inner value</i>" html)))
                    (not (null (string-match-p "footnotes" html)))
                    html)))))
      (delete-directory root t))))"##,
    );
}

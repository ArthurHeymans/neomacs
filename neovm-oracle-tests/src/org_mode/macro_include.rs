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

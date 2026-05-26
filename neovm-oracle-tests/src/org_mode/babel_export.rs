use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_babel_src_info_expand_execute_results_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (insert "#+NAME: calc\n")
    (insert "#+begin_src emacs-lisp :var x=5 y=7 :results value replace\n")
    (insert "(+ x y)\n")
    (insert "#+end_src\n")
    (goto-char (point-min))
    (search-forward "begin_src")
    (let ((org-confirm-babel-evaluate nil))
      (let ((info (org-babel-get-src-block-info))
            (expanded (org-babel-expand-src-block))
            (result (org-babel-execute-src-block)))
        (list (nth 0 info)
              (nth 1 info)
              (assq :var (nth 2 info))
              (cdr (assq :result-type (nth 2 info)))
              (nth 4 info)
              expanded
              result
              (buffer-substring-no-properties (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_babel_ref_parse_split_resolve_table_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-ref)
  (with-temp-buffer
    (org-mode)
    (insert "#+NAME: data\n")
    (insert "| a | b |\n| 1 | 2 |\n| 3 | 4 |\n")
    (goto-char (point-min))
    (list (org-babel-ref-parse "x=data[1,2]")
          (org-babel-ref-split-args "a=1, b=two, c=\"three,four\"")
          (org-babel-ref-resolve "data"))))"##,
    );
}

#[test]
fn org_export_environment_and_string_html_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox-html)
  (with-temp-buffer
    (org-mode)
    (insert "#+TITLE: Export Env\n")
    (insert "#+OPTIONS: toc:nil num:nil\n")
    (insert "* One\n")
    (insert "Paragraph [fn:1].\n")
    (insert "#+CAPTION: Table Cap\n")
    (insert "| A | B |\n| 1 | 2 |\n")
    (insert "[fn:1] Foot.\n")
    (let* ((info (org-export-get-environment 'html nil nil))
           (tree (org-element-parse-buffer))
           (heads
            (org-element-map tree 'headline
              (lambda (headline)
                (org-element-property :raw-value headline))))
           (foots
            (org-element-map tree 'footnote-definition
              (lambda (footnote)
                (org-element-property :label footnote))))
           (html (org-export-string-as
                  "* H\nText" 'html t '(:with-toc nil))))
      (list (mapcar #'substring-no-properties (plist-get info :title))
            (plist-get info :with-toc)
            heads
            foots
            (not (null (string-match-p "<h2" html)))
            (replace-regexp-in-string
             "org[[:alnum:]]+"
             "org-id"
             html)))))"##,
    );
}

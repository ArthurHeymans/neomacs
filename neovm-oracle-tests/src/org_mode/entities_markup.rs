use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_entities_user_table_export_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-entities)
  (require 'ox-html)
  (require 'ox-ascii)
  (require 'ox-latex)
  (let ((org-entities-user
         '(("myarrow" "\\Rightarrow" t "&rArr;" "=>" "=>" "⇒")
           ("brand" "\\textsc{Neo}" nil "<span>Neo</span>" "Neo" "Neo" "Neo"))))
    (with-temp-buffer
      (org-mode)
      (insert "#+TITLE: Entities\n")
      (insert "* H\n")
      (insert "A \\myarrow B and \\brand{} plus builtins \\alpha \\ndash.\n")
      (let* ((table (org-entities-create-table))
             (picked (mapcar (lambda (name)
                               (cdr (assoc name table)))
                             '("myarrow" "brand" "alpha" "ndash")))
             (tree (org-element-parse-buffer))
             (entities (org-element-map tree 'entity
                         (lambda (e)
                           (list (org-element-property :name e)
                                 (org-element-property :latex e)
                                 (org-element-property :latex-math-p e)
                                 (org-element-property :html e)
                                 (org-element-property :ascii e)
                                 (org-element-property :utf-8 e)))))
             (html (org-export-as 'html nil nil t '(:with-toc nil)))
             (ascii (let ((org-ascii-charset 'ascii))
                      (org-export-as 'ascii nil nil t '(:with-toc nil))))
             (utf8 (let ((org-ascii-charset 'utf-8))
                     (org-export-as 'ascii nil nil t '(:with-toc nil))))
             (latex (org-export-as 'latex nil nil t '(:with-toc nil))))
        (list picked
              entities
              (mapcar (lambda (needle)
                        (not (null (string-match-p needle html))))
                      '("&rArr;" "<span>Neo</span>" "&alpha;" "&ndash;"))
              (mapcar (lambda (needle)
                        (not (null (string-match-p needle ascii))))
                      '("=>" "Neo" "alpha" "-"))
              (mapcar (lambda (needle)
                        (not (null (string-match-p needle utf8))))
                      '("⇒" "Neo" "α" "–"))
              (mapcar (lambda (needle)
                        (not (null (string-match-p needle latex))))
                      '("\\\\Rightarrow" "\\\\textsc{Neo}" "\\\\alpha" "--"))))))"##,
    );
}

#[test]
fn org_subsuperscript_parse_export_modes_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ox-html)
  (require 'ox-ascii)
  (require 'ox-latex)
  (let ((sample "* H\nH_2O x^{2+y} a_b c_{d e} raw_underscore email_a@b.\n"))
    (mapcar
     (lambda (mode)
       (with-temp-buffer
         (let ((org-use-sub-superscripts mode))
           (org-mode)
           (insert sample)
           (let* ((tree (org-element-parse-buffer))
                  (objects (org-element-map tree '(subscript superscript)
                             (lambda (o)
                               (list (org-element-type o)
                                     (org-element-property :use-brackets-p o)
                                     (buffer-substring-no-properties
                                      (org-element-property :begin o)
                                      (org-element-property :end o)))))))
             (list mode
                   objects
                   (replace-regexp-in-string
                    "org[[:alnum:]]+"
                    "org-id"
                    (org-export-as 'html nil nil t '(:with-toc nil)))
                   (let ((org-ascii-charset 'utf-8))
                     (org-export-as 'ascii nil nil t '(:with-toc nil)))
                   (replace-regexp-in-string
                    "sec:org[[:alnum:]]+"
                    "sec:org-id"
                    (org-export-as 'latex nil nil t '(:with-toc nil))))))))
     '(t {} nil))))"##,
    );
}

#[test]
fn org_pretty_entities_fontify_display_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-entities)
  (with-temp-buffer
    (let ((org-pretty-entities t)
          (org-entities-user
           '(("myheart" "\\heartsuit" t "&hearts;" "<3" "<3" "♥"))))
      (org-mode)
      (insert "Alpha \\alpha, arrow \\rightarrow, custom \\myheart{}, escaped \\\\alpha.\n")
      (font-lock-ensure (point-min) (point-max))
      (let (before after)
        (goto-char (point-min))
        (while (re-search-forward "\\\\\\([A-Za-z]+\\)\\({}\\)?" nil t)
          (push (list (match-string 0)
                      (get-text-property (match-beginning 0) 'display)
                      (get-text-property (match-beginning 0) 'composition)
                      (get-text-property (match-beginning 0) 'face))
                before))
        (org-toggle-pretty-entities)
        (font-lock-ensure (point-min) (point-max))
        (goto-char (point-min))
        (while (re-search-forward "\\\\\\([A-Za-z]+\\)\\({}\\)?" nil t)
          (push (list (match-string 0)
                      (get-text-property (match-beginning 0) 'display)
                      (get-text-property (match-beginning 0) 'composition)
                      (get-text-property (match-beginning 0) 'face))
                after))
        (list (nreverse before)
              (nreverse after)
              org-pretty-entities
              (buffer-substring-no-properties
               (point-min) (point-max))))))"##,
    );
}

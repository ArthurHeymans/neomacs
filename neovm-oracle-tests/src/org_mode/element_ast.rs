use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_element_ast_adopt_extract_set_interpret_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-element)
  (let* ((doc (org-element-create 'org-data nil))
         (h1 (org-element-create
              'headline
              '(:level 1 :raw-value "Alpha" :title ("Alpha") :todo-keyword "TODO")
              (org-element-create
               'section nil
               (org-element-create
                'paragraph nil
                "First paragraph with "
                (org-element-create 'bold nil "bold")
                ".\n"))))
         (h2 (org-element-create
              'headline
              '(:level 1 :raw-value "Beta" :title ("Beta"))
              (org-element-create
               'section nil
               (org-element-create 'paragraph nil "Second paragraph.\n"))))
         (before nil)
         (after-extract nil))
    (org-element-adopt doc h1 h2)
    (setq before (org-element-interpret-data doc))
    (let* ((section (car (org-element-contents h1)))
           (paragraph (car (org-element-contents section)))
           (bold (car (org-element-map paragraph 'bold #'identity))))
      (org-element-extract bold)
      (setq after-extract (org-element-interpret-data doc))
      (org-element-set
       paragraph
       (org-element-create
        'paragraph nil
        "Replacement with "
        (org-element-create 'italic nil "italic")
        " and =literal= text.\n")))
    (list before
          after-extract
          (org-element-property :parent h1)
          (mapcar (lambda (headline)
                    (list (org-element-property :raw-value headline)
                          (org-element-property :level headline)
                          (mapcar #'org-element-type
                                  (org-element-contents headline))))
                  (org-element-map doc 'headline #'identity))
          (org-element-interpret-data doc)))"##,
    );
}

#[test]
fn org_element_parse_lineage_skip_affiliated_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-element)
  (with-temp-buffer
    (org-mode)
    (insert "#+CAPTION: A *caption* with [[https://example.org][link]]\n")
    (insert "#+NAME: tbl\n")
    (insert "| A | B |\n| 1 | 2 |\n\n")
    (insert "* Parent\n")
    (insert "** Child\n")
    (insert "Paragraph with /italic/ and [[#tbl][table link]].\n")
    (let* ((tree (org-element-parse-buffer))
           (no-table-objects
            (org-element-map tree t
              (lambda (node)
                (when (memq (org-element-type node) '(table link italic))
                  (org-element-type node)))
              nil nil 'table))
           (with-affiliated
            (org-element-map tree 'link
              (lambda (link)
                (list (org-element-property :type link)
                      (org-element-property :path link)
                      (mapcar #'org-element-type
                              (org-element-lineage link nil t))))
              nil nil nil t))
           (first-child
            (org-element-map tree 'headline
              (lambda (headline)
                (and (= 2 (org-element-property :level headline))
                     (throw :org-element-skip
                            (org-element-property :raw-value headline))))
              nil t)))
      (list no-table-objects
            with-affiliated
            first-child
            (substring-no-properties
             (org-element-interpret-data tree))))))"##,
    );
}

#[test]
fn org_element_buffer_context_swap_refresh_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-element)
  (with-temp-buffer
    (org-mode)
    (insert "* Alpha\n")
    (insert "First paragraph with *bold*.\n\n")
    (insert "#+BEGIN_QUOTE\nquoted\n#+END_QUOTE\n\n")
    (insert "* Beta\n")
    (insert "Second paragraph with /italic/.\n")
    (goto-char (point-min))
    (search-forward "First")
    (let* ((context-before (org-element-context))
           (lineage-before
            (mapcar #'org-element-type
                    (org-element-lineage context-before nil t)))
           (para-a (org-element-at-point))
           (quote (progn
                    (search-forward "quoted")
                    (org-element-at-point))))
      (org-element-swap-A-B para-a quote)
      (let ((after-swap (buffer-substring-no-properties
                         (point-min) (point-max))))
        (goto-char (point-min))
        (search-forward "Second")
        (let ((context-after (org-element-context)))
          (delete-region (line-beginning-position) (line-end-position))
          (insert "Second paragraph now has [[https://gnu.org][GNU]].")
          (org-element-cache-refresh (line-beginning-position))
          (list (org-element-type context-before)
                lineage-before
                after-swap
                (org-element-type context-after)
                (org-element-type (org-element-context))
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))"##,
    );
}

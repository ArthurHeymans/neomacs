use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_insert_structure_template_region_src_export_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((transient-mark-mode t))
      (org-mode)
      (insert "* Heading\n")
      (insert "(message \"*not a headline*\")\n")
      (insert ",#+already escaped\n")
      (goto-char (point-min))
      (forward-line 1)
      (push-mark (point) nil t)
      (goto-char (point-max))
      (org-insert-structure-template "src emacs-lisp")
      (let ((after-src (buffer-substring-no-properties
                        (point-min) (point-max))))
        (goto-char (point-max))
        (insert "Raw HTML\n")
        (push-mark (line-beginning-position) nil t)
        (goto-char (point-max))
        (org-insert-structure-template "EXPORT html")
        (list after-src
              (point)
              (buffer-substring-no-properties
               (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_tempo_custom_blocks_keywords_include_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-tempo)
  (let* ((root (make-temp-file "org-tempo" t))
         (include-file (expand-file-name "snippet.org" root))
         (default-directory root)
         (org-structure-template-alist
          '(("s" . "src")
            ("Q" . "QUOTE")
            ("el" . "src emacs-lisp")
            ("v" . "verse")))
         (org-tempo-keywords-alist
          '(("L" . "latex")
            ("c" . "caption")
            ("o" . "options"))))
    (unwind-protect
        (progn
          (with-temp-file include-file (insert "* Included\n"))
          (with-temp-buffer
            (org-mode)
            (org-tempo-setup)
            (insert "<el")
            (org-tempo-complete-tag)
            (insert "(+ 1 2)")
            (goto-char (point-max))
            (insert "\n<Q")
            (org-tempo-complete-tag)
            (insert "Quoted\n")
            (goto-char (point-max))
            (insert "\n<c")
            (org-tempo-complete-tag)
            (insert "A caption")
            (goto-char (point-max))
            (insert "\n<I")
            (cl-letf (((symbol-function 'read-file-name)
                       (lambda (&rest _) include-file)))
              (org-tempo-complete-tag))
            (insert ":lines \"1-1\"")
            (list (sort (org-tempo--keys) #'string<)
                  org-tempo-tags
                  (buffer-substring-no-properties
                   (point-min) (point-max)))))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_table_convert_transpose_move_copy_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-table)
  (with-temp-buffer
    (org-mode)
    (insert "Name,Jan,Feb\nAlpha,1,2\nBeta,3,4\n")
    (org-table-convert-region (point-min) (point-max) ",")
    (org-table-align)
    (let ((after-convert
           (buffer-substring-no-properties (point-min) (point-max))))
      (goto-char (point-min))
      (search-forward "Jan")
      (org-table-insert-column)
      (org-table-blank-field)
      (insert "Q1")
      (goto-char (point-min))
      (search-forward "Alpha")
      (org-table-copy-down 1)
      (org-table-move-row-down)
      (goto-char (point-min))
      (search-forward "Feb")
      (org-table-move-column-left)
      (let ((after-mutations
             (buffer-substring-no-properties (point-min) (point-max))))
        (goto-char (point-min))
        (org-table-transpose-table-at-point)
        (list after-convert
              after-mutations
              (buffer-substring-no-properties
               (point-min) (point-max))))))"##,
    );
}

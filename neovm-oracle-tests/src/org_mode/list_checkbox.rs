use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_checkbox_statistics_nested_ctrl_c_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* Project [0/3] [0%]\n")
    (insert "- [ ] One\n")
    (insert "- [-] Two [1/2]\n")
    (insert "  - [X] Two A\n")
    (insert "  - [ ] Two B\n")
    (insert "- [ ] Three\n")
    (goto-char (point-min))
    (search-forward "One")
    (org-ctrl-c-ctrl-c)
    (search-forward "Two B")
    (org-ctrl-c-ctrl-c)
    (goto-char (point-min))
    (org-update-checkbox-count t)
    (list
     (buffer-substring-no-properties (point-min) (point-max))
     (org-element-map (org-element-parse-buffer) 'item
       (lambda (item)
         (list (org-element-property :checkbox item)
               (buffer-substring-no-properties
                (org-element-property :contents-begin item)
                (save-excursion
                  (goto-char (org-element-property :contents-begin item))
                  (line-end-position)))))))))"##,
    );
}

#[test]
fn org_list_move_sort_cycle_bullet_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-list)
  (with-temp-buffer
    (org-mode)
    (insert "- zebra\n")
    (insert "- apple\n")
    (insert "  - child b\n")
    (insert "  - child a\n")
    (insert "- mango\n")
    (goto-char (point-min))
    (search-forward "apple")
    (beginning-of-line)
    (org-move-item-up)
    (let ((after-move
           (buffer-substring-no-properties (point-min) (point-max))))
      (goto-char (point-min))
      (org-sort-list nil ?a)
      (let ((after-sort
             (buffer-substring-no-properties (point-min) (point-max))))
        (goto-char (point-min))
        (org-cycle-list-bullet ?+)
        (list after-move
              after-sort
              (buffer-substring-no-properties (point-min) (point-max))
              (org-list-to-lisp))))))"##,
    );
}

#[test]
fn org_list_to_generic_html_org_delete_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-list)
  (with-temp-buffer
    (org-mode)
    (insert "1. [X] Alpha :: definition line\n")
    (insert "   continuation\n")
    (insert "2. [ ] Beta\n")
    (insert "   1. nested one\n")
    (insert "   2. nested two\n")
    (goto-char (point-min))
    (let* ((as-lisp (org-list-to-lisp))
           (html (org-list-to-html as-lisp))
           (org (org-list-to-org as-lisp))
           (texinfo (org-list-to-texinfo as-lisp)))
      (org-list-to-lisp t)
      (list as-lisp
            (not (null (string-match-p "<ol" html)))
            (not (null (string-match-p "definition line" html)))
            org
            (not (null (string-match-p "@enumerate" texinfo)))
            (buffer-substring-no-properties
             (point-min) (point-max))))))"##,
    );
}

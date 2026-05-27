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

#[test]
fn org_list_indent_outdent_checkbox_repair_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-list)
  (with-temp-buffer
    (org-mode)
    (insert "- [ ] Parent [0/3]\n")
    (insert "- [X] A\n")
    (insert "- [ ] B\n")
    (insert "  - [ ] B child\n")
    (insert "- [ ] C\n")
    (goto-char (point-min))
    (search-forward "B")
    (beginning-of-line)
    (org-indent-item-tree)
    (let ((after-indent
           (buffer-substring-no-properties (point-min) (point-max))))
      (org-toggle-checkbox)
      (org-outdent-item-tree)
      (goto-char (point-min))
      (org-update-checkbox-count t)
      (org-list-repair)
      (let* ((struct (org-list-struct))
             (parents (org-list-parents-alist struct))
             (prevs (org-list-prevs-alist struct))
             (items (mapcar
                     (lambda (item)
                       (list (- item (point-min))
                             (org-list-get-parent item struct parents)
                             (org-list-get-item-number item struct prevs parents)
                             (org-list-get-children item struct parents)
                             (org-list-get-item-end item struct)))
                     (org-list-get-all-items (point-min) struct prevs))))
        (list after-indent
              items
              (org-list-to-lisp)
              (buffer-substring-no-properties
               (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_insert_delete_move_description_items_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-list)
  (with-temp-buffer
    (org-mode)
    (insert "- term A :: first\n")
    (insert "  continuation A\n")
    (insert "- term B :: second\n")
    (insert "- term C :: third\n")
    (goto-char (point-min))
    (search-forward "term B")
    (beginning-of-line)
    (org-insert-item)
    (insert "term inserted :: new")
    (let ((after-insert
           (buffer-substring-no-properties (point-min) (point-max))))
      (org-move-item-down)
      (let ((after-move
             (buffer-substring-no-properties (point-min) (point-max))))
        (goto-char (point-min))
        (search-forward "term C")
        (beginning-of-line)
        (let* ((struct (org-list-struct))
               (item (point)))
          (org-list-delete-item item struct))
        (list after-insert
              after-move
              (org-list-to-lisp)
              (buffer-substring-no-properties
               (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_ordered_alpha_list_sort_renumber_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-list)
  (with-temp-buffer
    (org-mode)
    (insert "a. Gamma\n")
    (insert "b. Alpha\n")
    (insert "   a. child two\n")
    (insert "   b. child one\n")
    (insert "c. Beta\n")
    (goto-char (point-min))
    (search-forward "Alpha")
    (beginning-of-line)
    (org-sort-list nil ?a)
    (let ((after-sort
           (buffer-substring-no-properties (point-min) (point-max))))
      (goto-char (point-min))
      (search-forward "child one")
      (beginning-of-line)
      (org-move-item-up)
      (let* ((struct (org-list-struct))
             (prevs (org-list-prevs-alist struct))
             (parents (org-list-parents-alist struct))
             (summary
              (mapcar
               (lambda (item)
                 (list (buffer-substring-no-properties
                        item (line-end-position))
                       (org-list-get-item-number item struct prevs parents)
                       (org-list-get-list-type item struct prevs)))
               (org-list-get-all-items (point-min) struct prevs))))
        (list after-sort
              summary
              (org-list-to-lisp)
              (buffer-substring-no-properties
               (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_list_descriptive_generic_roundtrip_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-list)
  (with-temp-buffer
    (org-mode)
    (insert "- [X] Term *A* :: First line\n")
    (insert "  continuation with =code=\n")
    (insert "  1. [@3] child three\n")
    (insert "  2. [ ] child off\n")
    (insert "- [-] Term B :: second line\n")
    (goto-char (point-min))
    (let* ((parsed (org-list-to-lisp))
           (generic
            (org-list-to-generic
             parsed
             (list :backend 'org
                   :raw t
                   :dstart (lambda (depth) (format "<dl depth=%d>" depth))
                   :dend "</dl>"
                   :ostart (lambda (depth) (format "<ol depth=%d>" depth))
                   :oend "</ol>"
                   :dtstart "<dt>"
                   :dtend "</dt>"
                   :ddstart "<dd>"
                   :ddend "</dd>"
                   :istart (lambda (type depth)
                             (format "<item type=%S depth=%d>" type depth))
                   :icount (lambda (type depth count)
                             (format "<item type=%S depth=%d count=%d>"
                                     type depth count))
                   :iend "</item>"
                   :isep "|"
                   :cbon "{X}"
                   :cboff "{ }"
                   :cbtrans "{-}"
                   :ifmt (lambda (type contents)
                           (format "[%S]%s" type contents)))))
           (html (org-list-to-html parsed '(:raw t)))
           (org (org-list-to-org parsed))
           (subtree (org-list-to-subtree parsed 2)))
      (org-list-to-lisp t)
      (insert org)
      (list parsed
            generic
            (list (string-match-p "<dl" html)
                  (string-match-p "Term" html)
                  (string-match-p "child three" html))
            org
            subtree
            (buffer-substring-no-properties
             (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_list_nested_counter_checkbox_repair_cycle_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-list)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-include-plain-lists 'integrate))
      (org-mode)
      (insert "* Tasks [0/3]\n")
      (insert "1. [ ] Alpha [1/2]\n")
      (insert "   - [X] Alpha done\n")
      (insert "   - [ ] Alpha todo\n")
      (insert "2. [ ] Beta\n")
      (insert "   a. Beta child b\n")
      (insert "   b. Beta child a\n")
      (insert "3. [X] Gamma\n")
      (let ((snapshot
             (lambda (label)
               (let* ((struct (org-list-struct))
                      (prevs (org-list-prevs-alist struct))
                      (parents (org-list-parents-alist struct))
                      (items (org-list-get-all-items (point-min) struct prevs)))
                 (list label
                       (mapcar
                        (lambda (item)
                          (save-excursion
                            (goto-char item)
                            (list (- item (point-min))
                                  (buffer-substring-no-properties
                                   item (line-end-position))
                                  (org-list-get-parent item struct parents)
                                  (org-list-get-children item struct parents)
                                  (org-list-get-item-number
                                   item struct prevs parents)
                                  (org-list-get-list-type item struct prevs)
                                  (invisible-p item))))
                        items)
                       (org-list-to-lisp)
                       (buffer-substring-no-properties
                        (point-min) (point-max))))))
            states)
        (goto-char (point-min))
        (search-forward "Alpha todo")
        (org-toggle-checkbox)
        (goto-char (point-min))
        (org-update-checkbox-count t)
        (push (funcall snapshot 'after-checkbox) states)
        (goto-char (point-min))
        (search-forward "Beta child a")
        (beginning-of-line)
        (org-move-item-up)
        (push (funcall snapshot 'after-child-move) states)
        (goto-char (point-min))
        (search-forward "Gamma")
        (beginning-of-line)
        (org-indent-item-tree)
        (push (funcall snapshot 'after-indent-gamma) states)
        (org-outdent-item-tree)
        (org-list-repair)
        (push (funcall snapshot 'after-outdent-repair) states)
        (goto-char (point-min))
        (search-forward "Alpha")
        (beginning-of-line)
        (org-cycle)
        (push (funcall snapshot 'after-list-cycle) states)
        (org-fold-show-all)
        (goto-char (point-min))
        (org-cycle-list-bullet ?+)
        (push (funcall snapshot 'after-bullet-cycle) states)
        (list (nreverse states)
              (org-list-to-generic
               (org-list-to-lisp)
               (list :backend 'org
                     :raw t
                     :ostart "<ordered>"
                     :oend "</ordered>"
                     :ulstart "<unordered>"
                     :ulend "</unordered>"
                     :istart "<item>"
                     :iend "</item>"
                     :isep "|"
                     :cbon "[on]"
                     :cboff "[off]"
                     :cbtrans "[mixed]"
                     :ifmt (lambda (_type contents) contents)))
              (buffer-substring-no-properties
               (point-min) (point-max))))))"##,
    );
}

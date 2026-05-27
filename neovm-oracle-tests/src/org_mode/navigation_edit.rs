use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_element_navigation_positions_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* A\n")
    (insert "Paragraph one.\n\n")
    (insert "- item one\n- item two\n\n")
    (insert "#+begin_quote\nquoted\n#+end_quote\n")
    (insert "** B\nBody B\n")
    (insert "* C\nBody C\n")
    (let ((snap (lambda (label)
                  (let ((e (org-element-at-point)))
                    (list label
                          (point)
                          (org-element-type e)
                          (org-element-property :begin e)
                          (org-element-property :end e)
                          (thing-at-point 'line t)))))
          states)
      (goto-char (point-min))
      (push (funcall snap 'start) states)
      (org-forward-element)
      (push (funcall snap 'forward-headline) states)
      (search-forward "Paragraph")
      (push (funcall snap 'paragraph) states)
      (org-forward-element)
      (push (funcall snap 'forward-list) states)
      (org-down-element)
      (push (funcall snap 'down-item) states)
      (org-up-element)
      (push (funcall snap 'up-list) states)
      (org-forward-element)
      (push (funcall snap 'forward-quote) states)
      (org-backward-element)
      (push (funcall snap 'backward-list) states)
      (goto-char (point-min))
      (search-forward "** B")
      (beginning-of-line)
      (push (list 'end-subtree
                  (org-end-of-subtree t nil)
                  (save-excursion
                    (org-end-of-subtree t t)
                    (point))
                  (line-number-at-pos)))
            states)
      (nreverse states))))"##,
    );
}

#[test]
fn org_drag_transpose_element_buffer_integrity_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* H\n")
    (insert "First paragraph.\n\n")
    (insert "#+begin_quote\nQuote block.\n#+end_quote\n\n")
    (insert "- item one\n- item two\n\n")
    (insert "Final paragraph.\n")
    (goto-char (point-min))
    (search-forward "begin_quote")
    (beginning-of-line)
    (org-drag-element-forward)
    (let ((after-forward
           (buffer-substring-no-properties (point-min) (point-max))))
      (search-forward "Final paragraph")
      (beginning-of-line)
      (org-drag-element-backward)
      (let ((after-backward
             (buffer-substring-no-properties (point-min) (point-max))))
        (search-forward "item two")
        (beginning-of-line)
        (org-transpose-element)
        (list after-forward
              after-backward
              (buffer-substring-no-properties (point-min) (point-max))
              (org-element-map (org-element-parse-buffer)
                  '(paragraph quote-block plain-list item)
                (lambda (e)
                  (list (org-element-type e)
                        (org-element-property :begin e)
                        (org-element-property :end e))))))))"##,
    );
}

#[test]
fn org_mark_narrow_unindent_navigation_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "  * A\n")
    (insert "    Body A\n")
    (insert "    ** B\n")
    (insert "      Body B\n")
    (insert "      - item\n")
    (insert "  * C\n")
    (insert "    Body C\n")
    (org-unindent-buffer)
    (let ((after-unindent
           (buffer-substring-no-properties (point-min) (point-max))))
      (goto-char (point-min))
      (search-forward "** B")
      (beginning-of-line)
      (let ((sibling-prev (save-excursion
                            (condition-case err
                                (progn (org-goto-sibling 'previous)
                                       (thing-at-point 'line t))
                              (error (cons (car err) (cdr err))))))
            (first-child (save-excursion
                           (org-goto-first-child)
                           (thing-at-point 'line t))))
        (search-forward "item")
        (org-mark-element)
        (let ((mark-span (list (point) (mark))))
          (org-narrow-to-element)
          (let ((narrow-text
                 (buffer-substring-no-properties (point-min) (point-max)))
                (narrow-limits (list (point-min) (point-max))))
            (widen)
            (list after-unindent
                  sibling-prev
                  first-child
                  mark-span
                  narrow-limits
                  narrow-text
                  (buffer-substring-no-properties
                   (point-min) (point-max))))))))"##,
    );
}

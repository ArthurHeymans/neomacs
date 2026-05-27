use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_context_mixed_positions_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-clock)
  (with-temp-buffer
    (org-mode)
    (insert "* TODO [#A] Heading :tag:\n")
    (insert "SCHEDULED: <2026-05-27 Wed> DEADLINE: <2026-05-28 Thu>\n")
    (insert "- [ ] Item with [[https://example.org][link]]\n")
    (insert "| A | B |\n|---+---|\n| 1 | 2 |\n")
    (insert "#+BEGIN: clocktable :scope file\n#+END:\n")
    (insert "#+begin_src emacs-lisp\n(+ 1 2)\n#+end_src\n")
    (insert "$x+y$ and trailing text.\n")
    (font-lock-ensure (point-min) (point-max))
    (mapcar
     (lambda (probe)
       (goto-char (point-min))
       (search-forward (car probe))
       (when (cdr probe) (beginning-of-line))
       (let ((ctx (org-context)))
         (list (car probe)
               (mapcar #'car ctx)
               (org-at-heading-p)
               (org-at-planning-p)
               (org-at-item-p)
               (org-at-table-p)
               (not (null (org-in-clocktable-p)))
               (org-in-src-block-p)
               (org-in-src-block-p t)
               (org-in-block-p '("src" "quote" "clocktable"))
               (org-element-type (org-element-at-point)))))
     '(("TODO" . nil)
       ("SCHEDULED" . t)
       ("[ ]" . nil)
       ("link" . nil)
       ("| 1" . t)
       ("clocktable" . t)
       ("(+ 1 2)" . nil)
       ("$x+y$" . nil)))))"##,
    );
}

#[test]
fn org_block_map_next_previous_restricted_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* Blocks\n")
    (insert "#+begin_quote\nquote\n#+end_quote\n\n")
    (insert "#+begin_src emacs-lisp\n(+ 1 2)\n#+end_src\n\n")
    (insert "#+begin_example\nexample\n#+end_example\n\n")
    (insert "** Child\n")
    (insert "#+begin_verse\nverse\n#+end_verse\n")
    (let (mapped moves)
      (org-block-map
       (lambda ()
         (let ((e (org-element-at-point)))
           (push (list (org-element-type e)
                       (line-number-at-pos)
                       (buffer-substring-no-properties
                        (org-element-property :post-affiliated e)
                        (line-end-position)))
                 mapped))))
      (goto-char (point-min))
      (org-next-block 1)
      (push (list 'next1 (line-number-at-pos)
                  (org-element-type (org-element-at-point)))
            moves)
      (org-next-block 2)
      (push (list 'next2 (line-number-at-pos)
                  (org-element-type (org-element-at-point)))
            moves)
      (org-previous-block 1)
      (push (list 'prev1 (line-number-at-pos)
                  (org-element-type (org-element-at-point)))
            moves)
      (let ((restricted nil))
        (goto-char (point-min))
        (search-forward "** Child")
        (let ((start (line-beginning-position))
              (end (point-max)))
          (org-block-map
           (lambda ()
             (push (list (line-number-at-pos)
                         (org-element-type (org-element-at-point)))
                   restricted))
           start end))
        (list (nreverse mapped)
              (nreverse moves)
              (nreverse restricted)
              (buffer-substring-no-properties
               (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_between_regexps_nested_blocks_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* H\n")
    (insert "#+begin_special\n")
    (insert "before\n")
    (insert "#+begin_quote\n")
    (insert "inside quote\n")
    (insert "#+end_quote\n")
    (insert "after\n")
    (insert "#+end_special\n")
    (insert "* Next\n")
    (mapcar
     (lambda (needle)
       (goto-char (point-min))
       (search-forward needle)
       (let ((between-special
              (org-between-regexps-p
               "^[ \t]*#\\+begin_special"
               "^[ \t]*#\\+end_special"))
             (between-quote
              (org-between-regexps-p
               "^[ \t]*#\\+begin_quote"
               "^[ \t]*#\\+end_quote")))
         (list needle
               (and between-special
                    (list (- (car between-special) (point-min))
                          (- (cdr between-special) (point-min))))
               (and between-quote
                    (list (- (car between-quote) (point-min))
                          (- (cdr between-quote) (point-min))))
               (org-in-block-p '("quote" "special"))
               (org-element-type (org-element-at-point)))))
     '("before" "inside quote" "after" "Next"))))"##,
    );
}

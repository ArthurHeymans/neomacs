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

#[test]
fn org_context_drawer_timestamp_comment_latex_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-clock)
  (with-temp-buffer
    (let ((org-agenda-include-inactive-timestamps t))
      (org-mode)
      (insert "#+TITLE: Context\n")
      (insert "* TODO Node\n")
      (insert "SCHEDULED: <2026-05-27 Wed 09:30 +1w> DEADLINE: <2026-05-30 Sat>\n")
      (insert ":PROPERTIES:\n:Effort: 0:30\n:CUSTOM_ID: node\n:END:\n")
      (insert ":LOGBOOK:\n")
      (insert "CLOCK: [2026-05-27 Wed 09:00]--[2026-05-27 Wed 09:25] =>  0:25\n")
      (insert ":END:\n")
      (insert "# Comment with [2026-05-27 Wed]\n")
      (insert "#+begin_quote\n")
      (insert "Quote has <2026-05-28 Thu> and $x_1+y$.\n")
      (insert "#+end_quote\n")
      (insert "Paragraph has src_emacs-lisp{(+ 1 2)}, [[#node][self]], and \\alpha.\n")
      (font-lock-ensure (point-min) (point-max))
      (mapcar
       (lambda (probe)
         (goto-char (point-min))
         (search-forward (car probe))
         (when (plist-get (cdr probe) :bol) (beginning-of-line))
         (when (plist-get (cdr probe) :back) (backward-char))
         (let ((ctx (org-context))
               (ts-basic (org-at-timestamp-p))
               (ts-inactive (org-at-timestamp-p 'inactive))
               (ts-agenda (org-at-timestamp-p 'agenda))
               (ts-lax (org-at-timestamp-p 'lax)))
           (list (car probe)
                 (point)
                 (mapcar #'car ctx)
                 (org-element-type (org-element-context))
                 (org-at-keyword-p)
                 (org-at-heading-p)
                 (org-at-planning-p)
                 (org-at-property-drawer-p)
                 (org-at-property-p)
                 (org-at-drawer-p)
                 (org-at-clock-log-p)
                 (org-at-comment-p)
                 (org-at-block-p)
                 (org-in-block-p '("quote" "src"))
                 ts-basic
                 ts-inactive
                 ts-agenda
                 ts-lax
                 (org-inside-LaTeX-fragment-p))))
       '(("TITLE" :bol t)
         ("TODO Node" :bol t)
         ("09:30" :back t)
         ("DEADLINE" :bol t)
         ("Effort" :bol t)
         ("CUSTOM_ID" :bol t)
         ("LOGBOOK" :bol t)
         ("09:25" :back t)
         ("Comment" :bol t)
         ("Quote has" :bol t)
         ("2026-05-28" :back t)
         ("x_1+y")
         ("src_emacs-lisp")
         ("self")
          ("alpha")))))"##,
    );
}

#[test]
fn org_block_drawer_comment_structure_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "#+TITLE: Structure Probe\n\n")
    (insert "* TODO Alpha :work:\n")
    (insert "SCHEDULED: <2026-05-27 Wed>\n")
    (insert ":PROPERTIES:\n:Effort: 2:00\n:CUSTOM_ID: alpha-id\n:Owner: Ada\n:END:\n")
    (insert ":LOGBOOK:\n")
    (insert "CLOCK: [2026-05-27 Wed 09:00]--[2026-05-27 Wed 10:00] =>  1:00\n")
    (insert ":END:\n")
    (insert "# This is a comment\n")
    (insert "Alpha body.\n\n")
    (insert "#+begin_quote\n")
    (insert "Quoted text with *markup*.\n")
    (insert "#+end_quote\n\n")
    (insert "#+begin_src emacs-lisp :results value\n")
    (insert "(+ 1 2)\n")
    (insert "#+end_src\n\n")
    (insert "#+begin_example\n")
    (insert "Example block.\n")
    (insert "#+end_example\n\n")
    (insert "** DONE Beta\n")
    (insert "CLOSED: [2026-05-26 Mon 15:00]\n")
    (insert "Beta body with $x^2$ math.\n\n")
    (insert "#+begin_center\n")
    (insert "Centered content.\n")
    (insert "#+end_center\n")
    ;; Parse full structure
    (let* ((tree (org-element-parse-buffer))
           (element-types
            (org-element-map tree t
              (lambda (el) (org-element-type el))))
           (blocks
            (org-element-map tree '(src-block quote-block example-block center-block)
              (lambda (b)
                (list (org-element-type b)
                      (org-element-property :language b)
                      (org-element-property :value b)
                      (org-element-property :begin b)))))
           (drawers
            (org-element-map tree '(property-drawer drawer)
              (lambda (d)
                (list (org-element-type d)
                      (org-element-property :drawer-name d)
                      (org-element-property :begin d)))))
           (props
            (org-element-map tree 'node-property
              (lambda (np)
                (list (org-element-property :key np)
                      (org-element-property :value np)))))
           (planning
            (org-element-map tree 'planning
              (lambda (p)
                (list (and (org-element-property :scheduled p)
                           (org-element-property :raw-value
                            (org-element-property :scheduled p)))
                      (and (org-element-property :deadline p)
                           (org-element-property :raw-value
                            (org-element-property :deadline p)))
                      (and (org-element-property :closed p)
                           (org-element-property :raw-value
                            (org-element-property :closed p)))))))
           (clocks
            (org-element-map tree 'clock
              (lambda (c)
                (list (org-element-property :duration c)
                      (org-element-property :status c)))))
           (comments
            (org-element-map tree 'comment
              (lambda (c)
                (buffer-substring-no-properties
                 (org-element-property :begin c)
                 (org-element-property :end c))))))
      (list element-types
            blocks
            drawers
            props
            planning
            clocks
            comments
            (buffer-substring-no-properties
             (point-min) (point-max))))))"##,
    );
}

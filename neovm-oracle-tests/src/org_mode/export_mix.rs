use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_html_export_drawer_special_footnote_filter_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox-html)
  (with-temp-buffer
    (org-mode)
    (insert "#+TITLE: Export Mix\n")
    (insert "* Kept\n")
    (insert ":LOGBOOK:\n")
    (insert "Drawer text with \\alpha.\n")
    (insert ":END:\n")
    (insert "#+ATTR_HTML: :class callout :data-x yes\n")
    (insert "#+begin_aside\n")
    (insert "Aside with [fn:a][fn:b] and [[https://example.org][link]].\n")
    (insert "#+end_aside\n")
    (insert "[fn:a] Alpha footnote.\n")
    (insert "[fn:b] Beta footnote.\n")
    (insert "** Hidden :noexport:\n")
    (insert "Should not export.\n")
    (insert "* COMMENT Commented\n")
    (insert "Should not export either.\n")
    (let* ((org-export-with-toc nil)
           (org-export-exclude-tags '("noexport"))
           (org-html-format-drawer-function
            (lambda (name contents)
              (format "<section class=\"drawer\" data-name=\"%s\">%s</section>"
                      name contents)))
           (html (org-export-as 'html nil nil t nil))
           (normalized
            (replace-regexp-in-string
             "org[[:alnum:]]+"
             "org-id"
             html)))
      (list
       (not (null (string-match-p "data-name=\"LOGBOOK\"" html)))
       (not (null (string-match-p "&alpha;" html)))
       (not (null (string-match-p "<aside" html)))
       (not (null (string-match-p "class=\"callout\"" html)))
       (not (null (string-match-p "footnotes" html)))
       (null (string-match-p "Should not export" html))
       normalized))))"##,
    );
}

#[test]
fn org_latex_export_entities_footnotes_special_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox-latex)
  (with-temp-buffer
    (org-mode)
    (insert "#+TITLE: Latex Mix\n")
    (insert "* H\n")
    (insert "Text \\alpha and \\rightarrow with [fn:x][fn:y].\n")
    (insert "#+ATTR_LATEX: :options frametitle={Box}\n")
    (insert "#+begin_tcolorbox\n")
    (insert "Inside *bold* and $x^2$.\n")
    (insert "#+end_tcolorbox\n")
    (insert "[fn:x] First footnote.\n")
    (insert "[fn:y] Second footnote with /italic/.\n")
    (let* ((org-export-with-toc nil)
           (latex (org-export-as 'latex nil nil t nil))
           (normalized
            (replace-regexp-in-string
             "sec:org[[:alnum:]]+"
             "sec:org-id"
             latex)))
      (list
       (not (null (string-match-p "\\\\alpha" latex)))
       (not (null (string-match-p "\\\\rightarrow" latex)))
       (not (null (string-match-p "\\\\footnote" latex)))
       (not (null (string-match-p "\\\\textsuperscript{,}" latex)))
       (not (null (string-match-p "\\\\begin{tcolorbox}" latex)))
       (not (null (string-match-p "frametitle={Box}" latex)))
       (not (null (string-match-p "\\\\textbf{bold}" latex)))
       normalized))))"##,
    );
}

#[test]
fn org_export_data_entities_footnote_numbers_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox-html)
  (with-temp-buffer
    (org-mode)
    (insert "#+TITLE: Data\n")
    (insert "* H\n")
    (insert "A \\beta ref [fn:1] and inline [fn::Inline note].\n")
    (insert "[fn:1] Defined note with [[https://example.org][url]].\n")
    (let* ((info (org-export-get-environment 'html nil nil))
           (tree (org-element-parse-buffer))
           (refs (org-element-map tree 'footnote-reference #'identity))
           (entities (org-element-map tree 'entity
                       (lambda (entity)
                         (list (org-element-property :name entity)
                               (org-element-property :html entity)
                               (org-element-property :latex entity)))))
           (numbers (mapcar
                     (lambda (ref)
                       (list (org-element-property :label ref)
                             (org-export-get-footnote-number ref info)
                             (org-export-footnote-first-reference-p ref info)))
                     refs))
           (rendered (mapcar
                      (lambda (ref)
                        (org-export-data ref info))
                      refs)))
      (list entities numbers rendered))))"##,
    );
}

#[test]
fn org_export_filter_pipeline_order_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox-html)
  (with-temp-buffer
    (org-mode)
    (insert "#+TITLE: Filters\n")
    (insert "* Alpha\n")
    (insert "Plain [[https://example.org][link]] and /italic/ text.\n")
    (insert "#+begin_quote\nquoted text\n#+end_quote\n")
    (let (calls)
      (let ((org-export-filter-plain-text-functions
             (list (lambda (text backend info)
                     (push (list 'plain backend text) calls)
                     (replace-regexp-in-string "Plain" "PLAIN" text))))
            (org-export-filter-link-functions
             (list (lambda (text backend info)
                     (push (list 'link backend text) calls)
                     (concat text "<!--link-filter-->"))))
            (org-export-filter-headline-functions
             (list (lambda (text backend info)
                     (push (list 'headline backend
                                 (plist-get info :title)) calls)
                     text)))
            (org-export-filter-final-output-functions
             (list (lambda (text backend info)
                     (push (list 'final backend (length text)) calls)
                     (concat text "\n<!--final-filter-->")))))
        (let* ((org-export-with-toc nil)
               (html (org-export-as 'html nil nil t nil)))
          (list (mapcar (lambda (call)
                          (pcase call
                            (`(plain ,backend ,text)
                             (list 'plain backend
                                   (not (null (string-match-p "Plain" text)))))
                            (`(link ,backend ,text)
                             (list 'link backend
                                   (not (null (string-match-p "<a href" text)))))
                            (`(headline ,backend ,title)
                             (list 'headline backend title))
                            (`(final ,backend ,len)
                             (list 'final backend (numberp len)))))
                        (nreverse calls))
                (not (null (string-match-p "PLAIN" html)))
                (not (null (string-match-p "link-filter" html)))
                (not (null (string-match-p "final-filter" html)))
                (replace-regexp-in-string
                 "org[[:alnum:]]+"
                 "org-id"
                 html))))))"##,
    );
}

#[test]
fn org_export_collect_options_references_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox-html)
  (with-temp-buffer
    (org-mode)
    (insert "#+TITLE: Collect\n")
    (insert "#+OPTIONS: toc:nil num:2 tags:not-in-toc\n")
    (insert "* One :tag:\n")
    (insert "#+NAME: tbl\n")
    (insert "| A | B |\n|---+---|\n| 1 | 2 |\n")
    (insert "#+CAPTION: Table caption\n")
    (insert "[[tbl][Table link]] and <<target>> target link [[target]].\n")
    (insert "** Two\n")
    (insert "#+begin_src emacs-lisp -n -r\n")
    (insert "(message \"hi\") ;; (ref:msg)\n")
    (insert "#+end_src\n")
    (let* ((info (org-export-get-environment 'html nil '(:with-tags nil)))
           (headlines
            (mapcar (lambda (h)
                      (list (org-element-property :raw-value h)
                            (org-export-get-relative-level h info)
                            (org-export-get-headline-number h info)
                            (org-export-get-tags h info)))
                    (org-export-collect-headlines info)))
           (tables
            (mapcar (lambda (tbl)
                      (list (org-export-get-reference tbl info)
                            (org-export-get-caption tbl)
                            (org-export-get-ordinal tbl info)))
                    (org-export-collect-tables info)))
           (links
            (org-element-map (org-element-parse-buffer) 'link
              (lambda (link)
                (list (org-element-property :raw-link link)
                      (org-export-data link info))))))
      (list (plist-get info :title)
            (plist-get info :with-toc)
            (plist-get info :section-numbers)
            (plist-get info :with-tags)
            headlines
            tables
            links))))"##,
    );
}

#[test]
fn org_export_derived_backend_transcoder_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox-html)
  (org-export-define-derived-backend
      'oracle-html 'html
    :translate-alist
    '((bold . (lambda (bold contents info)
                (format "<strong data-oracle=\"yes\">%s</strong>" contents)))
      (paragraph . (lambda (paragraph contents info)
                     (format "<p class=\"oracle-p\">%s</p>" contents)))))
    :filters-alist
    '((:filter-final-output
       . (lambda (output backend info)
           (concat output "\n<!--oracle-html-->")))))
  (with-temp-buffer
    (org-mode)
    (insert "#+TITLE: Derived\n")
    (insert "* H\n")
    (insert "Text *bold* and [[https://example.org][link]].\n")
    (let* ((org-export-with-toc nil)
           (out (org-export-as 'oracle-html nil nil t nil))
           (backend (org-export-get-backend 'oracle-html)))
      (list (not (null (memq 'oracle-html org-export-registered-backends)))
            (not (null (assq 'bold (org-export-get-all-transcoders backend))))
            (not (null (assq :filter-final-output
                             (org-export-get-all-filters backend))))
            (not (null (string-match-p "data-oracle" out)))
            (not (null (string-match-p "oracle-p" out)))
            (not (null (string-match-p "oracle-html" out)))
            (replace-regexp-in-string
             "org[[:alnum:]]+"
             "org-id"
             out)))))"##,
    );
}

#[test]
fn org_org_export_native_planning_macro_footnote_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox-org)
  (with-temp-buffer
    (org-mode)
    (insert "#+TITLE: Native\n")
    (insert "#+AUTHOR: Ada\n")
    (insert "#+OPTIONS: toc:nil num:nil tags:t prop:t\n")
    (insert "#+MACRO: badge Badge-$1\n")
    (insert "* TODO Keep :work:\n")
    (insert "SCHEDULED: <2026-05-27 Wed 09:00> DEADLINE: <2026-05-28 Thu>\n")
    (insert ":PROPERTIES:\n:Owner: Ada\n:Score: 7\n:END:\n")
    (insert ":LOGBOOK:\n")
    (insert "- State \"TODO\" from \"\" [2026-05-26 Tue]\n")
    (insert ":END:\n")
    (insert "Paragraph {{{badge(ok)}}} with [[https://example.org][link]] ")
    (insert "and footnote[fn:n].\n")
    (insert "#+begin_quote\nQuoted *bold* text.\n#+end_quote\n")
    (insert "[fn:n] Note body with /italic/.\n")
    (insert "** Hidden :noexport:\n")
    (insert "Should not appear.\n")
    (insert "* COMMENT Commented\n")
    (insert "Should not appear either.\n")
    (let* ((org-export-exclude-tags '("noexport"))
           (org-export-with-toc nil)
           (org-export-with-properties t)
           (org-export-with-drawers t)
           (org-export-with-planning t)
           (out (org-export-as 'org nil nil t nil))
           (env (org-export-get-environment 'org nil nil))
           (tree (org-element-parse-buffer)))
      (list
       (plist-get env :title)
       (plist-get env :with-properties)
       (plist-get env :with-drawers)
       (plist-get env :with-planning)
       (mapcar (lambda (h)
                 (list (org-element-property :raw-value h)
                       (org-export-get-relative-level h env)
                       (org-export-get-tags h env)))
               (org-export-collect-headlines env))
       (org-element-map tree '(macro footnote-reference link planning drawer)
         (lambda (el)
           (pcase (org-element-type el)
             ('macro (list 'macro
                           (org-element-property :key el)
                           (org-element-property :args el)))
             ('footnote-reference
              (list 'footnote
                    (org-element-property :label el)))
             ('link (list 'link
                          (org-element-property :raw-link el)
                          (org-element-property :type el)
                          (org-element-property :path el)))
             ('planning
              (list 'planning
                    (and (org-element-property :scheduled el)
                         (org-element-property
                          :raw-value
                          (org-element-property :scheduled el)))
                    (and (org-element-property :deadline el)
                         (org-element-property
                          :raw-value
                          (org-element-property :deadline el)))))
             ('drawer (list 'drawer
                            (org-element-property :drawer-name el))))))
       (not (null (string-match-p "Badge-ok" out)))
       (not (null (string-match-p ":Owner:" out)))
       (not (null (string-match-p "SCHEDULED:" out)))
       (not (null (string-match-p "LOGBOOK" out)))
       (null (string-match-p "Should not appear" out))
       out))))"##,
    );
}

#[test]
fn org_export_hooks_parse_tree_navigation_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox-html)
  (with-temp-buffer
    (org-mode)
    (insert "#+TITLE: Hooked\n")
    (insert "#+MACRO: wrap Before-$1\n")
    (insert "* Keep\n")
    (insert "First paragraph with {{{wrap(macro)}}}.\n")
    (insert "#+begin_comment\n")
    (insert "comment should vanish\n")
    (insert "#+end_comment\n")
    (insert "#+begin_export html\n")
    (insert "<strong>raw html</strong>\n")
    (insert "#+end_export\n")
    (insert "* Drop :noexport:\n")
    (insert "Dropped paragraph.\n")
    (let ((calls nil))
      (let ((org-export-before-processing-functions
             (list (lambda (backend)
                     (push (list 'processing backend
                                 (buffer-substring-no-properties
                                  (point-min) (line-end-position)))
                           calls)
                     (goto-char (point-max))
                     (insert "* Added\n")
                     (insert "Added paragraph with [[https://example.org][link]].\n"))))
            (org-export-before-parsing-functions
             (list (lambda (backend)
                     (push (list 'parsing backend
                                 (buffer-substring-no-properties
                                  (point-min) (line-end-position)))
                           calls)
                     (goto-char (point-min))
                     (search-forward "First paragraph")
                     (end-of-line)
                     (insert "\nSecond paragraph inserted before parsing.\n"))))
            (org-export-filter-options-functions
             (list (lambda (info backend)
                     (push (list 'options backend
                                 (plist-get info :title)
                                 (plist-get info :with-toc))
                           calls)
                     (plist-put info :with-toc nil))))
            (org-export-filter-parse-tree-functions
             (list (lambda (tree backend info)
                     (push
                      (list 'tree backend
                            (mapcar
                             (lambda (h)
                               (org-element-property :raw-value h))
                             (org-element-map tree 'headline #'identity))
                            (length (plist-get info :ignore-list)))
                      calls)
                     tree))))
        (let* ((org-export-exclude-tags '("noexport"))
               (html (org-export-as 'html nil nil t nil))
               (info (org-export-get-environment 'html nil
                                                 '(:with-toc nil)))
               (tree (plist-get info :parse-tree))
               (paragraphs
                (org-element-map tree 'paragraph #'identity))
               (first-p (car paragraphs))
               (second-p (cadr paragraphs))
               (link (car (org-element-map tree 'link #'identity))))
          (list (nreverse calls)
                (mapcar
                 (lambda (p)
                   (list (org-element-type
                          (org-export-get-previous-element p info))
                         (org-element-type
                          (org-export-get-next-element p info))
                         (org-element-type
                          (org-export-get-parent-headline p))))
                 paragraphs)
                (and link
                     (list (org-element-property :raw-link link)
                           (org-element-property
                            :raw-value
                            (org-export-get-parent-headline link))
                           (org-element-type
                            (org-export-get-previous-element link info t))))
                (org-export-get-category first-p info)
                (org-export-get-category second-p info)
                (not (null (string-match-p "Before-macro" html)))
                (not (null (string-match-p "Second paragraph" html)))
                (not (null (string-match-p "Added paragraph" html)))
                (not (null (string-match-p "raw html" html)))
                (null (string-match-p "Dropped paragraph" html))
                (null (string-match-p "comment should vanish" html))
                (replace-regexp-in-string
                 "org[[:alnum:]]+"
                 "org-id"
                 html))))))"##,
    );
}

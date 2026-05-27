//! Xenna-strict combo tests for org-mode extreme edge cases.

use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

// ═══════════════════════════════════════════════════════════════════════
// Xenna: org-element with complex document parsing
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn xenna_complex_document_parsing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'oc)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+TITLE: Complete Document
#+AUTHOR: Test
#+OPTIONS: H:3 num:t toc:t
#+FILETAGS: :test:

* TODO [#A] Chapter 1 :ch1:
SCHEDULED: <2024-01-15 Mon +1w>
DEADLINE: <2024-01-19 Fri -3d>
:PROPERTIES:
:CUSTOM_ID: ch1
:EFFORT: 2h
:END:
:LOGBOOK:
CLOCK: [2024-01-15 Mon 09:00]--[2024-01-15 Mon 10:00] =>  1:00
:END:

Paragraph with *bold*, /italic/, _underline_, =verbatim=, ~code~, +strike+.

Also [[https://orgmode.org][link]], [cite:@key1;@key2], [fn:1], and \\alpha.

| Name | Value |
|------+-------|
| A    |     1 |
| B    |     2 |
#+TBLFM: @3$2=vsum(@1$2..@2$2)

#+BEGIN_QUOTE
Quoted text.
#+END_QUOTE

#+BEGIN_SRC emacs-lisp
(+ 1 2)
#+END_SRC

** DONE Section 1.1 :s11:
CLOSED: [2024-01-16 Wed 10:00]

- [ ] Task 1
- [X] Task 2
  - [ ] Sub-task 2.1
- [-] Task 3

** TODO Section 1.2
<<target>> See [[#ch1][Chapter 1]].

*** Subsection 1.2.1
#+BEGIN_CENTER
Centered text.
#+END_CENTER

* WAIT Chapter 2 :ch2:
#+BEGIN_COMMENT
Under development.
#+END_COMMENT

[fn:1] Footnote with *bold* and [[https://orgmode.org][link]].")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer)))
        (list
         ;; Structure.
         (length (org-element-map tree 'headline #'identity))
         (length (org-element-map tree 'section #'identity))
         (length (org-element-map tree 'paragraph #'identity))
         ;; Inline markup.
         (length (org-element-map tree 'bold #'identity))
         (length (org-element-map tree 'italic #'identity))
         (length (org-element-map tree 'underline #'identity))
         (length (org-element-map tree 'verbatim #'identity))
         (length (org-element-map tree 'code #'identity))
         (length (org-element-map tree 'strike-through #'identity))
         ;; Links and references.
         (length (org-element-map tree 'link #'identity))
         (length (org-element-map tree 'citation #'identity))
         (length (org-element-map tree 'footnote-reference #'identity))
         (length (org-element-map tree 'footnote-definition #'identity))
         ;; Blocks.
         (length (org-element-map tree 'quote-block #'identity))
         (length (org-element-map tree 'src-block #'identity))
         (length (org-element-map tree 'center-block #'identity))
         (length (org-element-map tree 'comment-block #'identity))
         ;; Tables.
         (length (org-element-map tree 'table #'identity))
         (length (org-element-map tree 'table-row #'identity))
         (length (org-element-map tree 'table-cell #'identity))
         ;; Lists.
         (length (org-element-map tree 'plain-list #'identity))
         (length (org-element-map tree 'item #'identity))
         ;; Planning.
         (length (org-element-map tree 'planning #'identity))
         ;; Clock.
         (length (org-element-map tree 'clock #'identity))
         ;; Drawers.
         (length (org-element-map tree 'property-drawer #'identity))
         (length (org-element-map tree 'drawer #'identity))
         ;; Keywords.
         (length (org-element-map tree 'keyword #'identity))
         ;; Entities.
         (length (org-element-map tree 'entity #'identity))
         ;; Targets.
         (length (org-element-map tree 'target #'identity)))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xenna: org-element with complex export round-trip
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn xenna_complex_export_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+TITLE: Round Trip Test
* H1
Paragraph with *bold* and /italic/.
** H2
| a | b |
| c | d |
* H3
- Item 1
- Item 2
#+BEGIN_SRC emacs-lisp
(+ 1 2)
#+END_SRC")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (info (org-combine-plists
                    (org-export--get-export-attributes)
                    (org-export-get-environment)
                    (org-export--collect-tree-properties
                     tree (org-export-get-environment)))))
        (list
         ;; Export data.
         (substring-no-properties (org-export-data tree info))
         ;; Headline numbers.
         (mapcar (lambda (h) (org-export-get-headline-number h info))
                 (org-element-map tree 'headline #'identity))
         ;; Relative levels.
         (mapcar (lambda (h) (org-export-get-relative-level h info))
                 (org-element-map tree 'headline #'identity))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xenna: org-element with complex property inheritance chain
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn xenna_complex_property_inheritance_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (let* ((level4 (org-element-create 'level4 '(:shared 4 :own4 "d")))
         (level3 (org-element-create 'level3 '(:shared 3 :own3 "c") level4))
         (level2 (org-element-create 'level2 '(:shared 2 :own2 "b") level3))
         (level1 (org-element-create 'level1 '(:shared 1 :own1 "a") level2)))
    (list
     ;; At level4: own value wins.
     (org-element-property-inherited :shared level4 'with-self)
     ;; Without self: get parent's.
     (org-element-property-inherited :shared level4)
     ;; Accumulate all.
     (org-element-property-inherited :shared level4 'with-self 'accumulate)
     ;; Only level1 has :own1.
     (org-element-property-inherited :own1 level4 'with-self 'accumulate)
     ;; Only level2 has :own2.
     (org-element-property-inherited :own2 level4 'with-self 'accumulate)
     ;; Only level3 has :own3.
     (org-element-property-inherited :own3 level4 'with-self 'accumulate)
     ;; Only level4 has :own4.
     (org-element-property-inherited :own4 level4 'with-self 'accumulate))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xenna: org-element with complex element operations chain
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn xenna_complex_element_operations_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (let* ((doc (org-element-create 'org-data nil))
         (h1 (org-element-create
              'headline '(:level 1 :raw-value "A" :title ("A"))
              (org-element-create 'section nil (org-element-create 'paragraph nil "P1.\n"))))
         (h2 (org-element-create
              'headline '(:level 1 :raw-value "B" :title ("B"))
              (org-element-create 'section nil (org-element-create 'paragraph nil "P2.\n"))))
         (h3 (org-element-create
              'headline '(:level 1 :raw-value "C" :title ("C"))
              (org-element-create 'section nil (org-element-create 'paragraph nil "P3.\n")))))
    ;; Adopt all three.
    (org-element-adopt doc h1 h2 h3)
    (let ((after-adopt (org-element-interpret-data doc)))
      ;; Extract middle.
      (org-element-extract h2)
      (let ((after-extract (org-element-interpret-data doc)))
        ;; Swap remaining.
        (org-element-swap-A-B h1 h3)
        (let ((after-swap (org-element-interpret-data doc)))
          ;; Set h1's paragraph.
          (let* ((sec (car (org-element-contents h1)))
                 (para (car (org-element-contents sec))))
            (org-element-set para (org-element-create 'paragraph nil "New.\n")))
          (list (substring-no-properties after-adopt)
                (substring-no-properties after-extract)
                (substring-no-properties after-swap)
                (substring-no-properties (org-element-interpret-data doc))
                ;; h2 has no parent after extract.
                (org-element-property :parent h2))))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xenna: org-element with complex deferred chain
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn xenna_complex_deferred_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (list
   ;; Resolve :deferred property.
   (let ((el (org-element-create
              'dummy
              `(:deferred ,(org-element-deferred-create
                            t (lambda (el) (org-element-put-property el :foo 'bar) nil))))))
     (list (org-element-property :foo el) (org-element-property :foo2 el)))
   ;; Deferred value.
   (let ((el (org-element-create
              'dummy `(:foo ,(org-element-deferred-create nil (lambda (_) 'bar))))))
     (org-element-property :foo el))
   ;; Auto-undefer.
   (let ((el (org-element-create
              'dummy `(:foo ,(org-element-deferred-create t (lambda (_) 'bar))))))
     (list (org-element-property :foo el) (org-element-property-raw :foo el)))
   ;; Force undefer.
   (let ((el (org-element-create
              'dummy `(:foo ,(org-element-deferred-create nil (lambda (_) 'bar))))))
     (list (org-element-property :foo el)
           (org-element-property-raw :foo el)
           (org-element-property :foo el nil 'force)
           (org-element-property-raw :foo el)))
   ;; Deferred alias.
   (let ((el (org-element-create
              'dummy `( :foo 1 :bar ,(org-element-deferred-create-alias :foo)))))
     (list (org-element-property :foo el) (org-element-property :bar el)))
   ;; Deferred list.
   (let ((el (org-element-create
              'dummy `(:foo ,(org-element-deferred-create-list
                              (list 1 2 (org-element-deferred-create nil (lambda (_) 3))))))))
     (org-element-property :foo el))
   ;; Deferred with side effects (retry).
   (let ((el (org-element-create
              'dummy `(:foo ,(org-element-deferred-create
                              nil (lambda (el)
                                    (org-element-put-property el :foo 1)
                                    (throw :org-element-deferred-retry nil)))))))
     (org-element-property :foo el))
   ;; Recursive undefer.
   (let ((el (org-element-create
              'dummy `(:foo ,(org-element-deferred-create
                              nil (lambda (el)
                                    (org-element-deferred-create
                                     nil (lambda (_) 1)))))))
     (org-element-property :foo el))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xenna: org-element with complex parse-and-interpret round-trips
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn xenna_complex_parse_and_interpret_roundtrips() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil)
        (org-test-parse-and-interpret
         (lambda (text)
           (with-temp-buffer
             (org-mode) (insert text)
             (org-element-interpret-data (org-element-parse-buffer))))))
    (list
     ;; Bold.
     (funcall org-test-parse-and-interpret "*text*")
     ;; Italic.
     (funcall org-test-parse-and-interpret "/text/")
     ;; Code.
     (funcall org-test-parse-and-interpret "~text~")
     ;; Verbatim.
     (funcall org-test-parse-and-interpret "=text=")
     ;; Underline.
     (funcall org-test-parse-and-interpret "_text_")
     ;; Strike-through.
     (funcall org-test-parse-and-interpret "+target+")
     ;; Subscript.
     (funcall org-test-parse-and-interpret "a_b")
     (funcall org-test-parse-and-interpret "a_{b}")
     ;; Superscript.
     (funcall org-test-parse-and-interpret "a^b")
     (funcall org-test-parse-and-interpret "a^{b}")
     ;; Entity.
     (funcall org-test-parse-and-interpret "\\alpha text")
     (funcall org-test-parse-and-interpret "\\alpha{}text"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xenna: org-element with complex link round-trips
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn xenna_complex_link_roundtrips() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil)
        (org-test-parse-and-interpret
         (lambda (text)
           (with-temp-buffer
             (org-mode) (insert text)
             (org-element-interpret-data (org-element-parse-buffer))))))
    (list
     ;; Link without description.
     (funcall org-test-parse-and-interpret "[[https://orgmode.org]]")
     ;; Link with description.
     (funcall org-test-parse-and-interpret "[[https://orgmode.org][Org mode]]")
     ;; File link.
     (funcall org-test-parse-and-interpret "[[file:todo.org::*task]]")
     ;; Id link.
     (funcall org-test-parse-and-interpret "[[id:aaaa]]")
     ;; Custom-id link.
     (funcall org-test-parse-and-interpret "[[#id]]")
     ;; Plain link.
     (funcall org-test-parse-and-interpret "https://orgmode.org")
     ;; Angular link.
     (funcall org-test-parse-and-interpret "<https://orgmode.org>"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xenna: org-element with complex footnote round-trips
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn xenna_complex_footnote_roundtrips() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil)
        (org-test-parse-and-interpret
         (lambda (text)
           (with-temp-buffer
             (org-mode) (insert text)
             (org-element-interpret-data (org-element-parse-buffer))))))
    (list
     ;; Regular reference.
     (funcall org-test-parse-and-interpret "Text[fn:1]")
     ;; Named reference.
     (funcall org-test-parse-and-interpret "Text[fn:label]")
     ;; Inline reference.
     (funcall org-test-parse-and-interpret "Text[fn:label:def]")
     ;; Anonymous reference.
     (funcall org-test-parse-and-interpret "Text[fn::def]"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xenna: org-element with complex block round-trips
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn xenna_complex_block_roundtrips() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil)
        (org-src-preserve-indentation t)
        (org-test-parse-and-interpret
         (lambda (text)
           (with-temp-buffer
             (org-mode) (insert text)
             (org-element-interpret-data (org-element-parse-buffer))))))
    (list
     ;; Center block.
     (funcall org-test-parse-and-interpret "#+BEGIN_CENTER\nText\n#+END_CENTER")
     ;; Quote block.
     (funcall org-test-parse-and-interpret "#+BEGIN_QUOTE\nText\n#+END_QUOTE")
     ;; Example block.
     (funcall org-test-parse-and-interpret "#+BEGIN_EXAMPLE\nTest\n#+END_EXAMPLE")
     ;; Export block.
     (funcall org-test-parse-and-interpret "#+BEGIN_EXPORT HTML\n<p>Text</p>\n#+END_EXPORT")
     ;; Verse block.
     (funcall org-test-parse-and-interpret "#+BEGIN_VERSE\nTest\n#+END_VERSE"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xenna: org-element with complex inline round-trips
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn xenna_complex_inline_roundtrips() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil)
        (org-test-parse-and-interpret
         (lambda (text)
           (with-temp-buffer
             (org-mode) (insert text)
             (org-element-interpret-data (org-element-parse-buffer))))))
    (list
     ;; Inline babel call.
     (funcall org-test-parse-and-interpret "call_test()")
     (funcall org-test-parse-and-interpret "call_test(x=2)")
     ;; Inline src block.
     (funcall org-test-parse-and-interpret "src_emacs-lisp{(+ 1 1)}")
     ;; Export snippet.
     (funcall org-test-parse-and-interpret "@@backend:contents@@")
     ;; LaTeX fragment.
     (funcall org-test-parse-and-interpret "\\command{}")
     (funcall org-test-parse-and-interpret "$x$")
     (funcall org-test-parse-and-interpret "$$x+y$$")
     (funcall org-test-parse-and-interpret "\\(x+y\\)")
     (funcall org-test-parse-and-interpret "\\[x+y\\]")
     ;; Statistics cookie.
     (funcall org-test-parse-and-interpret "[0/1]")
     (funcall org-test-parse-and-interpret "[66%]")
     ;; Line break.
     (funcall org-test-parse-and-interpret "First line \\\\\nSecond line")
     ;; Target.
     (funcall org-test-parse-and-interpret "<<target>>")
     ;; Radio target.
     (funcall org-test-parse-and-interpret "<<<some text>>>")
     ;; Macro.
     (funcall org-test-parse-and-interpret "{{{test}}}")
     (funcall org-test-parse-and-interpret "{{{test(arg1,arg2)}}}"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xenna: org-element with complex table round-trips
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn xenna_complex_table_roundtrips() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil)
        (org-test-parse-and-interpret
         (lambda (text)
           (with-temp-buffer
             (org-mode) (insert text)
             (org-element-interpret-data (org-element-parse-buffer))))))
    (list
     ;; Simple table.
     (funcall org-test-parse-and-interpret "| a | b |\n| c | d |")
     ;; With horizontal rules.
     (funcall org-test-parse-and-interpret "| a | b |\n|---+---|\n| c | d |")
     ;; With formula.
     (funcall org-test-parse-and-interpret
              "| 2 |\n| 4 |\n| 3 |\n#+TBLFM: @3=vmean(@1..@2)"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xenna: org-element with complex timestamp round-trips
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn xenna_complex_timestamp_roundtrips() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil)
        (org-test-parse-and-interpret
         (lambda (text)
           (with-temp-buffer
             (org-mode) (insert text)
             (org-element-interpret-data (org-element-parse-buffer))))))
    (list
     ;; Active.
     (string-match "<2012-03-29 .* 16:40>"
                   (funcall org-test-parse-and-interpret "<2012-03-29 thu. 16:40>"))
     ;; Inactive.
     (string-match "\\[2012-03-29 .* 16:40\\]"
                   (funcall org-test-parse-and-interpret "[2012-03-29 thu. 16:40]"))
     ;; Active daterange.
     (string-match "<2012-03-29 .* 16:40>--<2012-03-29 .* 16:41>"
                   (funcall org-test-parse-and-interpret
                            "<2012-03-29 thu. 16:40>--<2012-03-29 thu. 16:41>"))
     ;; Active timerange.
     (string-match "<2012-03-29 .* 16:40-16:41>"
                   (funcall org-test-parse-and-interpret
                            "<2012-03-29 thu. 16:40-16:41>"))
     ;; With repeater.
     (string-match "<2012-03-29 .* \\+1y>"
                   (funcall org-test-parse-and-interpret "<2012-03-29 thu. +1y>"))
     ;; Diary.
     (equal "<%%(diary-float t 4 2)>\n"
            (funcall org-test-parse-and-interpret "<%%(diary-float t 4 2)>")))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xenna: org-element with complex keyword/comment round-trips
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn xenna_complex_keyword_comment_roundtrips() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil)
        (org-test-parse-and-interpret
         (lambda (text)
           (with-temp-buffer
             (org-mode) (insert text)
             (org-element-interpret-data (org-element-parse-buffer))))))
    (list
     ;; Keyword.
     (funcall org-test-parse-and-interpret "#+KEYWORD: value")
     ;; Comment.
     (funcall org-test-parse-and-interpret "# Comment")
     ;; Comment block.
     (funcall org-test-parse-and-interpret "#+BEGIN_COMMENT\nTest\n#+END_COMMENT")
     ;; Fixed width.
     (funcall org-test-parse-and-interpret ": Test")
     ;; Horizontal rule.
     (funcall org-test-parse-and-interpret "-------")
     ;; Diary sexp.
     (funcall org-test-parse-and-interpret
              "%%(org-anniversary 1956  5 14)(2) Arthur Dent is %d years old")
     ;; LaTeX environment.
     (funcall org-test-parse-and-interpret
              "\\begin{equation}\n1+1=2\n\\end{equation}"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xenna: org-element with complex citation round-trips
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn xenna_complex_citation_roundtrips() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'oc)
  (let ((org-mode-hook nil)
        (org-test-parse-and-interpret
         (lambda (text)
           (with-temp-buffer
             (org-mode) (insert text)
             (org-element-interpret-data (org-element-parse-buffer))))))
    (list
     (funcall org-test-parse-and-interpret "[cite:@key]")
     (funcall org-test-parse-and-interpret "[cite/style:@key]")
     (funcall org-test-parse-and-interpret "[cite:pre @key]")
     (funcall org-test-parse-and-interpret "[cite:@key post]")
     (funcall org-test-parse-and-interpret "[cite:@a;@b;@c]"))))"##,
    );
}

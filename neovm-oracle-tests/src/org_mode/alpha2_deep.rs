//! Alpha-2 strict combo tests for org-mode extreme edge cases.

use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex document parsing (all element types)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_full_document_all_elements() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'oc)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+TITLE: Full Document
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
  - [X] Sub-task 2.2
- [ ] Task 3

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
         (length (org-element-map tree 'headline #'identity))
         (length (org-element-map tree 'section #'identity))
         (length (org-element-map tree 'paragraph #'identity))
         (length (org-element-map tree 'bold #'identity))
         (length (org-element-map tree 'italic #'identity))
         (length (org-element-map tree 'underline #'identity))
         (length (org-element-map tree 'verbatim #'identity))
         (length (org-element-map tree 'code #'identity))
         (length (org-element-map tree 'strike-through #'identity))
         (length (org-element-map tree 'link #'identity))
         (length (org-element-map tree 'citation #'identity))
         (length (org-element-map tree 'footnote-reference #'identity))
         (length (org-element-map tree 'footnote-definition #'identity))
         (length (org-element-map tree 'quote-block #'identity))
         (length (org-element-map tree 'src-block #'identity))
         (length (org-element-map tree 'center-block #'identity))
         (length (org-element-map tree 'comment-block #'identity))
         (length (org-element-map tree 'table #'identity))
         (length (org-element-map tree 'table-row #'identity))
         (length (org-element-map tree 'table-cell #'identity))
         (length (org-element-map tree 'plain-list #'identity))
         (length (org-element-map tree 'item #'identity))
         (length (org-element-map tree 'planning #'identity))
         (length (org-element-map tree 'clock #'identity))
         (length (org-element-map tree 'property-drawer #'identity))
         (length (org-element-map tree 'drawer #'identity))
         (length (org-element-map tree 'keyword #'identity))
         (length (org-element-map tree 'entity #'identity))
         (length (org-element-map tree 'target #'identity)))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex export round-trip (all features)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_export_roundtrip_all_features() {
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
         (substring-no-properties (org-export-data tree info))
         (mapcar (lambda (h) (org-export-get-headline-number h info))
                 (org-element-map tree 'headline #'identity))
         (mapcar (lambda (h) (org-export-get-relative-level h info))
                 (org-element-map tree 'headline #'identity))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex property inheritance (4 levels)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_property_inheritance_4_levels() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (let* ((level4 (org-element-create 'level4 '(:shared 4 :own4 "d")))
         (level3 (org-element-create 'level3 '(:shared 3 :own3 "c") level4))
         (level2 (org-element-create 'level2 '(:shared 2 :own2 "b") level3))
         (level1 (org-element-create 'level1 '(:shared 1 :own1 "a") level2)))
    (list
     (org-element-property-inherited :shared level4 'with-self)
     (org-element-property-inherited :shared level4)
     (org-element-property-inherited :shared level4 'with-self 'accumulate)
     (org-element-property-inherited :own1 level4 'with-self 'accumulate)
     (org-element-property-inherited :own2 level4 'with-self 'accumulate)
     (org-element-property-inherited :own3 level4 'with-self 'accumulate)
     (org-element-property-inherited :own4 level4 'with-self 'accumulate))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex element operations chain
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_element_operations_chain() {
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
    (org-element-adopt doc h1 h2 h3)
    (let ((after-adopt (org-element-interpret-data doc)))
      (org-element-extract h2)
      (let ((after-extract (org-element-interpret-data doc)))
        (org-element-swap-A-B h1 h3)
        (let ((after-swap (org-element-interpret-data doc)))
          (let* ((sec (car (org-element-contents h1)))
                 (para (car (org-element-contents sec))))
            (org-element-set para (org-element-create 'paragraph nil "New.\n")))
          (list (substring-no-properties after-adopt)
                (substring-no-properties after-extract)
                (substring-no-properties after-swap)
                (substring-no-properties (org-element-interpret-data doc))
                (org-element-property :parent h2))))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex deferred chain
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_deferred_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (list
   (let ((el (org-element-create
              'dummy
              `(:deferred ,(org-element-deferred-create
                            t (lambda (el) (org-element-put-property el :foo 'bar) nil))))))
     (list (org-element-property :foo el) (org-element-property :foo2 el)))
   (let ((el (org-element-create
              'dummy `(:foo ,(org-element-deferred-create nil (lambda (_) 'bar))))))
     (org-element-property :foo el))
   (let ((el (org-element-create
              'dummy `(:foo ,(org-element-deferred-create t (lambda (_) 'bar))))))
     (list (org-element-property :foo el) (org-element-property-raw :foo el)))
   (let ((el (org-element-create
              'dummy `(:foo ,(org-element-deferred-create nil (lambda (_) 'bar))))))
     (list (org-element-property :foo el)
           (org-element-property-raw :foo el)
           (org-element-property :foo el nil 'force)
           (org-element-property-raw :foo el)))
   (let ((el (org-element-create
              'dummy `( :foo 1 :bar ,(org-element-deferred-create-alias :foo)))))
     (list (org-element-property :foo el) (org-element-property :bar el)))
   (let ((el (org-element-create
              'dummy `(:foo ,(org-element-deferred-create-list
                              (list 1 2 (org-element-deferred-create nil (lambda _) 3)))))))
     (org-element-property :foo el))
   (let ((el (org-element-create
              'dummy `(:foo ,(org-element-deferred-create
                              nil (lambda (el)
                                    (org-element-put-property el :foo 1)
                                    (throw :org-element-deferred-retry nil)))))))
     (org-element-property :foo el))
   (let ((el (org-element-create
              'dummy `(:foo ,(org-element-deferred-create
                              nil (lambda (el)
                                    (org-element-deferred-create
                                     nil (lambda (_) 1)))))))
     (org-element-property :foo el))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex parse-and-interpret round-trips
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_parse_interpret_roundtrips() {
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
     (funcall org-test-parse-and-interpret "*text*")
     (funcall org-test-parse-and-interpret "/text/")
     (funcall org-test-parse-and-interpret "~text~")
     (funcall org-test-parse-and-interpret "=text=")
     (funcall org-test-parse-and-interpret "_text_")
     (funcall org-test-parse-and-interpret "+target+")
     (funcall org-test-parse-and-interpret "a_b")
     (funcall org-test-parse-and-interpret "a_{b}")
     (funcall org-test-parse-and-interpret "a^b")
     (funcall org-test-parse-and-interpret "a^{b}")
     (funcall org-test-parse-and-interpret "\\alpha text")
     (funcall org-test-parse-and-interpret "\\alpha{}text"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex link round-trips
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_link_roundtrips() {
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
     (funcall org-test-parse-and-interpret "[[https://orgmode.org]]")
     (funcall org-test-parse-and-interpret "[[https://orgmode.org][Org mode]]")
     (funcall org-test-parse-and-interpret "[[file:todo.org::*task]]")
     (funcall org-test-parse-and-interpret "[[id:aaaa]]")
     (funcall org-test-parse-and-interpret "[[#id]]")
     (funcall org-test-parse-and-interpret "https://orgmode.org")
     (funcall org-test-parse-and-interpret "<https://orgmode.org>"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex footnote round-trips
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_footnote_roundtrips() {
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
     (funcall org-test-parse-and-interpret "Text[fn:1]")
     (funcall org-test-parse-and-interpret "Text[fn:label]")
     (funcall org-test-parse-and-interpret "Text[fn:label:def]")
     (funcall org-test-parse-and-interpret "Text[fn::def]"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex block round-trips
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_block_roundtrips() {
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
     (funcall org-test-parse-and-interpret "#+BEGIN_CENTER\nText\n#+END_CENTER")
     (funcall org-test-parse-and-interpret "#+BEGIN_QUOTE\nText\n#+END_QUOTE")
     (funcall org-test-parse-and-interpret "#+BEGIN_EXAMPLE\nTest\n#+END_EXAMPLE")
     (funcall org-test-parse-and-interpret "#+BEGIN_EXPORT HTML\n<p>Text</p>\n#+END_EXPORT")
     (funcall org-test-parse-and-interpret "#+BEGIN_VERSE\nTest\n#+END_VERSE"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex inline round-trips
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_inline_roundtrips() {
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
     (funcall org-test-parse-and-interpret "call_test()")
     (funcall org-test-parse-and-interpret "call_test(x=2)")
     (funcall org-test-parse-and-interpret "src_emacs-lisp{(+ 1 1)}")
     (funcall org-test-parse-and-interpret "@@backend:contents@@")
     (funcall org-test-parse-and-interpret "\\command{}")
     (funcall org-test-parse-and-interpret "$x$")
     (funcall org-test-parse-and-interpret "$$x+y$$")
     (funcall org-test-parse-and-interpret "\\(x+y\\)")
     (funcall org-test-parse-and-interpret "\\[x+y\\]")
     (funcall org-test-parse-and-interpret "[0/1]")
     (funcall org-test-parse-and-interpret "[66%]")
     (funcall org-test-parse-and-interpret "First line \\\\\nSecond line")
     (funcall org-test-parse-and-interpret "<<target>>")
     (funcall org-test-parse-and-interpret "<<<some text>>>")
     (funcall org-test-parse-and-interpret "{{{test}}}")
     (funcall org-test-parse-and-interpret "{{{test(arg1,arg2)}}}"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex table round-trips
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_table_roundtrips() {
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
     (funcall org-test-parse-and-interpret "| a | b |\n| c | d |")
     (funcall org-test-parse-and-interpret "| a | b |\n|---+---|\n| c | d |")
     (funcall org-test-parse-and-interpret
              "| 2 |\n| 4 |\n| 3 |\n#+TBLFM: @3=vmean(@1..@2)"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex timestamp round-trips
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_timestamp_roundtrips() {
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
     (string-match "<2012-03-29 .* 16:40>"
                   (funcall org-test-parse-and-interpret "<2012-03-29 thu. 16:40>"))
     (string-match "\\[2012-03-29 .* 16:40\\]"
                   (funcall org-test-parse-and-interpret "[2012-03-29 thu. 16:40]"))
     (string-match "<2012-03-29 .* 16:40>--<2012-03-29 .* 16:41>"
                   (funcall org-test-parse-and-interpret
                            "<2012-03-29 thu. 16:40>--<2012-03-29 thu. 16:41>"))
     (string-match "<2012-03-29 .* 16:40-16:41>"
                   (funcall org-test-parse-and-interpret
                            "<2012-03-29 thu. 16:40-16:41>"))
     (string-match "<2012-03-29 .* \\+1y>"
                   (funcall org-test-parse-and-interpret "<2012-03-29 thu. +1y>"))
     (equal "<%%(diary-float t 4 2)>\n"
            (funcall org-test-parse-and-interpret "<%%(diary-float t 4 2)>")))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex keyword/comment round-trips
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_keyword_comment_roundtrips() {
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
     (funcall org-test-parse-and-interpret "#+KEYWORD: value")
     (funcall org-test-parse-and-interpret "# Comment")
     (funcall org-test-parse-and-interpret "#+BEGIN_COMMENT\nTest\n#+END_COMMENT")
     (funcall org-test-parse-and-interpret ": Test")
     (funcall org-test-parse-and-interpret "-------")
     (funcall org-test-parse-and-interpret
              "%%(org-anniversary 1956  5 14)(2) Arthur Dent is %d years old")
     (funcall org-test-parse-and-interpret
              "\\begin{equation}\n1+1=2\n\\end{equation}"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex citation round-trips
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_citation_roundtrips() {
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

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex export options (all 24+)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_export_all_options() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+TITLE: Options Test
#+AUTHOR: Author
#+EMAIL: email@example.org
#+DATE: 2024-01-15
#+DESCRIPTION: Description
#+KEYWORDS: test org
#+LANGUAGE: en
#+OPTIONS: H:3 num:t toc:t \\n:t timestamp:t author:t creator:t d:t email:t \
*:t e:t ::t f:t pri:t -:t ^:t toc:t |:t tags:t tasks:t <:t todo:t \
inline:nil stat:t title:t
#+CATEGORY: test
#+FILETAGS: :test:org:
* H1
Body")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (info (org-combine-plists
                    (org-export--get-export-attributes)
                    (org-export-get-environment)
                    (org-export--collect-tree-properties
                     tree (org-export-get-environment)))))
        (list
         (plist-get info :title)
         (plist-get info :author)
         (plist-get info :email)
         (plist-get info :headline-levels)
         (plist-get info :section-numbers)
         (plist-get info :with-timestamps)
         (plist-get info :with-author)
         (plist-get info :with-email)
         (plist-get info :with-emphasize)
         (plist-get info :with-entities)
         (plist-get info :with-fixed-width)
         (plist-get info :with-footnotes)
         (plist-get info :with-priority)
         (plist-get info :with-special-strings)
         (plist-get info :with-sub-superscript)
         (plist-get info :with-toc)
         (plist-get info :with-tables)
         (plist-get info :with-tags)
         (plist-get info :with-tasks)
         (plist-get info :with-timestamps)
         (plist-get info :with-todo-keywords)
         (plist-get info :with-inlinetasks)
         (plist-get info :with-statistics-cookies)
         (plist-get info :with-title))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex export headline numbers (all levels)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_export_headline_numbers_all_levels() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+OPTIONS: num:t H:3
* Ch1
** S1
*** SS1
*** SS2
** S2
*** SS3
* Ch2
** S3
*** SS4
** S4")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (info (org-combine-plists
                    (org-export--get-export-attributes)
                    (org-export-get-environment)
                    (org-export--collect-tree-properties
                     tree (org-export-get-environment)))))
        (list
         (mapcar (lambda (h) (org-export-get-headline-number h info))
                 (org-element-map tree 'headline #'identity))
         (mapcar (lambda (h) (org-export-get-relative-level h info))
                 (org-element-map tree 'headline #'identity))
         (mapcar (lambda (h) (org-export-numbered-headline-p h info))
                 (org-element-map tree 'headline #'identity))
         (mapcar (lambda (h) (org-export-low-level-p h info))
                 (org-element-map tree 'headline #'identity))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex export footnote numbers (all types)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_export_footnote_numbers_all_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "Text[fn:1] more[fn:2] and[fn:3].
* H1
Body[fn:4].
** H2
Body[fn:5:nested[fn:6]].

[fn:1] Def 1.
[fn:2] Def 2 with *bold*.
[fn:3] Def 3 with [[https://orgmode.org][link]].
[fn:4] Def 4.
[fn:6] Deeply nested.")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (info (org-combine-plists
                    (org-export--get-export-attributes)
                    (org-export-get-environment)
                    (org-export--collect-tree-properties
                     tree (org-export-get-environment)))))
        (list
         (mapcar (lambda (ref) (org-export-get-footnote-number ref info))
                 (org-element-map tree 'footnote-reference #'identity))
         (mapcar (lambda (ref) (org-export-footnote-first-reference-p ref info))
                 (org-element-map tree 'footnote-reference #'identity))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex export tags/categories
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_export_tags_categories() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+CATEGORY: work
* H1 :tag1:
** H2 :tag2:
*** H3 :tag3:
** H2b :tag1:tag2:
* H1b :tag3:
** H2c :tag1:tag3:")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (info (org-combine-plists
                    (org-export--get-export-attributes)
                    (org-export-get-environment)
                    (org-export--collect-tree-properties
                     tree (org-export-get-environment)))))
        (list
         (mapcar (lambda (h) (org-export-get-tags h info))
                 (org-element-map tree 'headline #'identity))
         (mapcar (lambda (h) (org-export-get-category h info))
                 (org-element-map tree 'headline #'identity))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex export first/last sibling
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_export_first_last_sibling() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H1\n** H2\n** H3\n** H4\n* H5")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (headlines (org-element-map tree 'headline #'identity)))
        (list
         (mapcar #'org-export-first-sibling-p headlines)
         (mapcar #'org-export-last-sibling-p headlines)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex export filter apply
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_export_filter_apply() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (list
   (org-export-filter-apply-functions
    (list (lambda (value &rest _) (concat "1" value))
          (lambda (value &rest _) (concat "2" value)))
    "0" nil)
   (org-export-filter-apply-functions
    (list #'ignore (lambda (value &rest _) (concat "2" value)))
    "0" nil)
   (org-export-filter-apply-functions (list #'ignore) "0" nil)
   (org-export-filter-apply-functions
    (list (lambda (_value &rest _) "")
          (lambda (value &rest _) (concat "2" value)))
    "0" nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex export backend chain
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_export_backend_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let (org-export-registered-backends)
    (org-export-define-backend 'parent
      '((headline . (lambda (h c i) (format "PARENT: %s\n%s" (org-element-property :raw-value h) c)))
        (section . (lambda (s c i) c))
        (paragraph . (lambda (p c i) c))
        (plain-text . (lambda (t i) t))))
    (org-export-define-derived-backend 'child 'parent
      :translate-alist
      '((headline . (lambda (h c i) (format "CHILD: %s\n%s" (org-element-property :raw-value h) c)))))
    (list
     (org-export-derived-backend-p 'child 'parent)
     (org-export-derived-backend-p 'child 'child)
     (let ((all (org-export-get-all-transcoders 'child)))
       (list (cdr (assq 'headline all))
             (cdr (assq 'section all)))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex export read-attribute
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_export_read_attribute() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (list
     (org-export-read-attribute
      :attr_html
      (with-temp-buffer (org-mode) (insert "#+ATTR_HTML: :a 1 :b 2\nParagraph")
        (goto-char (point-min)) (org-element-at-point)))
     (org-export-read-attribute
      :attr_html
      (with-temp-buffer (org-mode) (insert "Paragraph")
        (goto-char (point-min)) (org-element-at-point)))
     (org-export-read-attribute
      :attr_html
      (with-temp-buffer (org-mode) (insert "#+ATTR_HTML: :a nil :b nil\nParagraph")
        (goto-char (point-min)) (org-element-at-point))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex export caption
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_export_caption() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode)
       (insert "#+CAPTION: My caption\n| a | b |")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (table (car (org-element-map tree 'table #'identity))))
         (org-export-get-caption table)))
     (with-temp-buffer (org-mode)
       (insert "#+CAPTION[short]: long caption\n| a | b |")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (table (car (org-element-map tree 'table #'identity))))
         (list (org-export-get-caption table)
               (org-export-get-caption table t)))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex export optional title
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_export_optional_title() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+TITLE: Document Title\n* H\nBody")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (info (org-combine-plists
                    (org-export--get-export-attributes)
                    (org-export-get-environment)
                    (org-export--collect-tree-properties
                     tree (org-export-get-environment))))
             (headline (car (org-element-map tree 'headline #'identity))))
        (org-export-get-optional-title headline info)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha-2: org-element with complex export node property
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha2_export_node_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H\n:PROPERTIES:\n:CUSTOM_ID: myid\n:END:")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (headline (car (org-element-map tree 'headline #'identity))))
        (org-export-get-node-property :CUSTOM_ID headline))))"##,
    );
}

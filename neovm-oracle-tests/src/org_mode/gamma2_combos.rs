//! Gamma-2 strict combo tests for org-mode extreme edge cases.

use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

// ═══════════════════════════════════════════════════════════════════════
// Gamma-2: org-element with complex headline + tags + todo + priority combos
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn gamma2_headline_all_features() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* TODO [#A] Project :work:urgent:\n** DONE Design :design:\n** TODO Implementation :dev:\n*** TODO Backend :backend:\n*** TODO Frontend :frontend:\n** TODO Testing :test:")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (headlines (org-element-map tree 'headline #'identity)))
        (list
         (mapcar (lambda (h) (org-element-property :todo-keyword h)) headlines)
         (mapcar (lambda (h) (org-element-property :priority h)) headlines)
         (mapcar (lambda (h) (org-element-property :tags h)) headlines)
         (mapcar (lambda (h) (org-element-property :level h)) headlines))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Gamma-2: org-element with complex planning + clock + property combos
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn gamma2_planning_clock_property_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* TODO Task\nDEADLINE: <2024-01-15 Mon +1w>\nSCHEDULED: <2024-01-14 Sun>\nCLOSED: [2024-01-13 Sat]\n:PROPERTIES:\n:CUSTOM_ID: mytask\n:EFFORT: 2:30\n:CATEGORY: work\n:END:\n:LOGBOOK:\nCLOCK: [2024-01-14 Sun 09:00]--[2024-01-14 Sun 10:30] =>  1:30\nCLOCK: [2024-01-13 Sat 14:00]--[2024-01-13 Sat 16:00] =>  2:00\n:END:\nBody text.")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer)))
        (list
         (org-element-property :todo-keyword (car (org-element-map tree 'headline #'identity)))
         (let ((planning (car (org-element-map tree 'planning #'identity))))
           (list (org-element-property :scheduled planning)
                 (org-element-property :deadline planning)
                 (org-element-property :closed planning)))
         (length (org-element-map tree 'property-drawer #'identity))
         (length (org-element-map tree 'drawer #'identity))
         (length (org-element-map tree 'clock #'identity))
         (mapcar (lambda (c) (org-element-property :status c))
                 (org-element-map tree 'clock #'identity))
         (mapcar (lambda (c) (org-element-property :duration c))
                 (org-element-map tree 'clock #'identity))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Gamma-2: org-element with complex table + formula + export combos
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn gamma2_table_formula_export_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-table)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "| Item | Qty | Price | Total |\n|------+-----+-------+-------|\n| A    |   3 |    10 |       |\n| B    |   2 |    15 |       |\n|------+-----+-------+-------|\n|      |     |       |       |\n#+TBLFM: $4=$2*$3\n#+TBLFM: @>$4=vsum(@I..@-1)")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (table (org-element-map tree 'table #'identity nil t)))
        (list
         (org-element-property :type table)
         (length (org-element-map tree 'table-row #'identity))
         (length (org-element-map
                 (nth 1 (org-element-map tree 'table-row #'identity))
                 'table-cell #'identity))
         (org-element-map tree 'keyword
           (lambda (k) (when (equal (org-element-property :key k) "TBLFM")
                     (org-element-property :value k))))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Gamma-2: org-element with complex list + checkbox + description combos
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn gamma2_list_checkbox_description_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "- [ ] Unchecked\n- [X] Checked\n- [-] Partial\n- tag :: description\n  - [ ] Sub unchecked\n  - [X] Sub checked\n1. Ordered 1\n2. Ordered 2")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer)))
        (list
         (length (org-element-map tree 'item #'identity))
         (length (org-element-map tree 'plain-list #'identity))
         (mapcar (lambda (l) (org-element-property :type l))
                 (org-element-map tree 'plain-list #'identity))
         (mapcar (lambda (i) (org-element-property :checkbox i))
                 (org-element-map tree 'item #'identity))
         (mapcar (lambda (i)
                   (when (org-element-property :tag i)
                     (substring-no-properties
                      (org-element-interpret-data (org-element-property :tag i)))))
                 (org-element-map tree 'item #'identity))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Gamma-2: org-element with complex link + footnote + citation combos
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn gamma2_link_footnote_citation_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'oc)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "See [[https://orgmode.org][Org mode]] and [cite:@key1;@key2].\nAlso [fn:1] and [fn:2:inline footnote].\n\n[fn:1] Definition with *bold*.")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer)))
        (list
         (length (org-element-map tree 'link #'identity))
         (length (org-element-map tree 'citation #'identity))
         (length (org-element-map tree 'citation-reference #'identity))
         (length (org-element-map tree 'footnote-reference #'identity))
         (length (org-element-map tree 'footnote-definition #'identity))
         (org-element-property :type
           (org-element-map tree 'link #'identity nil t))
         (org-element-property :path
           (org-element-map tree 'link #'identity nil t))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Gamma-2: org-element with complex block + drawer + property combos
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn gamma2_block_drawer_property_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H\n:PROPERTIES:\n:KEY: val\n:END:\n:LOGBOOK:\nCLOCK: [2023-10-13 Fri 10:00]--[2023-10-13 Fri 11:00] =>  1:00\n:END:\n#+BEGIN_QUOTE\nQuoted text\n#+END_QUOTE\n#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC\nBody paragraph.")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer)))
        (list
         (delete-dups (mapcar #'org-element-type (org-element-map tree t #'identity)))
         (length (org-element-map tree 'property-drawer #'identity))
         (length (org-element-map tree 'drawer #'identity))
         (length (org-element-map tree '(quote-block src-block) #'identity))
         (length (org-element-map tree 'keyword #'identity))
         (length (org-element-map tree 'paragraph #'identity))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Gamma-2: org-element with complex inline markup combos
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn gamma2_inline_markup_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "Text with *bold*, /italic/, _underline_, =verbatim=, ~code~, +strike+.\nAlso $x^2$, \\alpha, and {{{macro}}}.\nAnd [[https://orgmode.org][link]], [fn:1], and [cite:@key].\n\n[fn:1] Footnote.")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer)))
        (list
         (length (org-element-map tree 'bold #'identity))
         (length (org-element-map tree 'italic #'identity))
         (length (org-element-map tree 'underline #'identity))
         (length (org-element-map tree 'verbatim #'identity))
         (length (org-element-map tree 'code #'identity))
         (length (org-element-map tree 'strike-through #'identity))
         (length (org-element-map tree 'latex-fragment #'identity))
         (length (org-element-map tree 'entity #'identity))
         (length (org-element-map tree 'macro #'identity))
         (length (org-element-map tree 'link #'identity))
         (length (org-element-map tree 'footnote-reference #'identity))
         (length (org-element-map tree 'footnote-definition #'identity))
         (length (org-element-map tree 'citation #'identity))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Gamma-2: org-element with complex timestamp + planning combos
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn gamma2_timestamp_planning_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* Weekly review\nSCHEDULED: <2024-01-15 Mon +1w>\nDEADLINE: <2024-01-19 Fri -3d>\n* Meeting\n<2024-01-20 Sat 14:00-15:30>\n* Deadline only\nDEADLINE: <2024-01-22 Mon>\n* Date range\n<2024-01-23 Tue>--<2024-01-25 Thu>\n* Inactive\n[2024-01-26 Fri 09:00]\n* Diary\n<%%(diary-float t 4 2)>")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (timestamps (org-element-map tree 'timestamp #'identity)))
        (list
         (length timestamps)
         (mapcar (lambda (ts) (org-element-property :type ts)) timestamps)
         (mapcar (lambda (ts) (org-element-property :range-type ts)) timestamps)
         (mapcar (lambda (ts) (list (org-element-property :repeater-type ts)
                              (org-element-property :repeater-value ts)
                              (org-element-property :repeater-unit ts)))
                 timestamps)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Gamma-2: org-element with complex export headline features combo
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn gamma2_export_headline_features_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* TODO [#A] H1 :tag1:tag2:\n* DONE [#B] H2\n* COMMENT H3\n* Normal H4\n* [#C] H5 :important:")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (info (org-combine-plists
                    (org-export--get-export-attributes)
                    (org-export-get-environment)
                    (org-export--collect-tree-properties
                     tree (org-export-get-environment))))
             (headlines (org-element-map tree 'headline #'identity)))
        (list
         (mapcar (lambda (h) (org-element-property :todo-keyword h)) headlines)
         (mapcar (lambda (h) (org-element-property :priority h)) headlines)
         (mapcar (lambda (h) (org-element-property :tags h)) headlines)
         (mapcar (lambda (h) (org-export-get-headline-number h info)) headlines)
         (mapcar (lambda (h) (org-export-numbered-headline-p h info)) headlines)
         (mapcar (lambda (h) (org-export-get-relative-level h info)) headlines))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Gamma-2: org-element with complex export footnote edge cases combo
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn gamma2_export_footnote_edge_cases_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "Text[fn:1] more[fn:2:inline] and[fn::anon].\n\n[fn:1] Standard def.\n")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (info (org-combine-plists
                    (org-export--get-export-attributes)
                    (org-export-get-environment)
                    (org-export--collect-tree-properties
                     tree (org-export-get-environment)))))
        (list
         (mapcar (lambda (ref) (org-element-property :type ref))
                 (org-element-map tree 'footnote-reference #'identity))
         (mapcar (lambda (ref) (org-export-get-footnote-number ref info))
                 (org-element-map tree 'footnote-reference #'identity))
         (mapcar (lambda (ref) (org-export-footnote-first-reference-p ref info))
                 (org-element-map tree 'footnote-reference #'identity))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Gamma-2: org-element with complex export options combo
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn gamma2_export_options_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+TITLE: Test\n#+AUTHOR: Author\n#+EMAIL: email@example.org\n#+DATE: 2024-01-15\n#+OPTIONS: H:3 num:t toc:t\n#+CATEGORY: test\n* H1\nBody")
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
         (plist-get info :with-toc)
         (plist-get info :with-timestamps)
         (plist-get info :with-author)
         (plist-get info :with-email)
         (plist-get info :with-emphasize)
         (plist-get info :with-entities)
         (plist-get info :with-footnotes))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Gamma-2: org-element with complex export backend chain combo
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn gamma2_export_backend_chain_combo() {
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
// Gamma-2: org-element with complex property inheritance chain combo
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn gamma2_property_inheritance_chain_combo() {
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
// Gamma-2: org-element with complex element operations chain combo
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn gamma2_element_operations_chain_combo() {
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
// Gamma-2: org-element with complex deferred chain combo
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn gamma2_deferred_chain_combo() {
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
// Gamma-2: org-element with complex parse-and-interpret round-trip combo
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn gamma2_parse_interpret_roundtrip_combo() {
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
// Gamma-2: org-element with complex link round-trip combo
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn gamma2_link_roundtrip_combo() {
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
// Gamma-2: org-element with complex footnote round-trip combo
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn gamma2_footnote_roundtrip_combo() {
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
// Gamma-2: org-element with complex block round-trip combo
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn gamma2_block_roundtrip_combo() {
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
// Gamma-2: org-element with complex inline round-trip combo
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn gamma2_inline_roundtrip_combo() {
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
// Gamma-2: org-element with complex table round-trip combo
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn gamma2_table_roundtrip_combo() {
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
// Gamma-2: org-element with complex timestamp round-trip combo
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn gamma2_timestamp_roundtrip_combo() {
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
// Gamma-2: org-element with complex keyword/comment round-trip combo
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn gamma2_keyword_comment_roundtrip_combo() {
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
// Gamma-2: org-element with complex citation round-trip combo
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn gamma2_citation_roundtrip_combo() {
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

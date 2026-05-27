//! Zeta-strict combo tests for org-mode export and advanced features.

use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

// ═══════════════════════════════════════════════════════════════════════
// Zeta: org-element with complex export options
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn zeta_export_options_title() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+TITLE: My Title\nBody")
      (goto-char (point-min))
      (let ((info (org-export-get-environment)))
        (plist-get info :title)))))"##,
    );
}

#[test]
fn zeta_export_options_author() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+AUTHOR: Test Author\nBody")
      (goto-char (point-min))
      (let ((info (org-export-get-environment)))
        (plist-get info :author)))))"##,
    );
}

#[test]
fn zeta_export_options_email() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+EMAIL: test@example.org\nBody")
      (goto-char (point-min))
      (let ((info (org-export-get-environment)))
        (plist-get info :email)))))"##,
    );
}

#[test]
fn zeta_export_options_date() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+DATE: 2024-01-15\nBody")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (info (org-combine-plists
                    (org-export--get-export-attributes)
                    (org-export-get-environment)
                    (org-export--collect-tree-properties
                     tree (org-export-get-environment)))))
        (org-export-get-date info)))))"##,
    );
}

#[test]
fn zeta_export_options_description() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+DESCRIPTION: A test document\nBody")
      (goto-char (point-min))
      (let ((info (org-export-get-environment)))
        (plist-get info :description)))))"##,
    );
}

#[test]
fn zeta_export_options_keywords() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+KEYWORDS: test org mode\nBody")
      (goto-char (point-min))
      (let ((info (org-export-get-environment)))
        (plist-get info :keywords)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Zeta: org-element with complex export headline features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn zeta_export_headline_numbers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+OPTIONS: num:t H:3\n* Ch1\n** S1\n*** SS1\n** S2\n* Ch2\n** S3")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (info (org-combine-plists
                    (org-export--get-export-attributes)
                    (org-export-get-environment)
                    (org-export--collect-tree-properties
                     tree (org-export-get-environment)))))
        (mapcar (lambda (h) (org-export-get-headline-number h info))
                (org-element-map tree 'headline #'identity))))))"##,
    );
}

#[test]
fn zeta_export_relative_levels() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H1\n** H2\n*** H3\n* H4")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (info (org-combine-plists
                    (org-export--get-export-attributes)
                    (org-export-get-environment)
                    (org-export--collect-tree-properties
                     tree (org-export-get-environment)))))
        (mapcar (lambda (h) (org-export-get-relative-level h info))
                (org-element-map tree 'headline #'identity))))))"##,
    );
}

#[test]
fn zeta_export_numbered_headline_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H1\n** H2\n*** H3\n* H4")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (info (org-combine-plists
                    (org-export--get-export-attributes)
                    (org-export-get-environment)
                    (org-export--collect-tree-properties
                     tree (org-export-get-environment)))))
        (mapcar (lambda (h) (org-export-numbered-headline-p h info))
                (org-element-map tree 'headline #'identity))))))"##,
    );
}

#[test]
fn zeta_export_low_level_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil)
        (org-export-headline-levels 2))
    (with-temp-buffer (org-mode)
      (insert "* H1\n** H2\n*** H3\n**** H4")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (info (org-combine-plists
                    (org-export--get-export-attributes)
                    (org-export-get-environment)
                    (org-export--collect-tree-properties
                     tree (org-export-get-environment)))))
        (mapcar (lambda (h) (org-export-low-level-p h info))
                (org-element-map tree 'headline #'identity))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Zeta: org-element with complex export tags/categories
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn zeta_export_tags() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H1 :tag1:\n** H2 :tag2:\n* H3")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (info (org-combine-plists
                    (org-export--get-export-attributes)
                    (org-export-get-environment)
                    (org-export--collect-tree-properties
                     tree (org-export-get-environment)))))
        (mapcar (lambda (h) (org-export-get-tags h info))
                (org-element-map tree 'headline #'identity))))))"##,
    );
}

#[test]
fn zeta_export_category() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+CATEGORY: work\n* H1\n* H2")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (info (org-combine-plists
                    (org-export--get-export-attributes)
                    (org-export-get-environment)
                    (org-export--collect-tree-properties
                     tree (org-export-get-environment)))))
        (mapcar (lambda (h) (org-export-get-category h info))
                (org-element-map tree 'headline #'identity))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Zeta: org-element with complex export footnotes
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn zeta_export_footnote_numbers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "Text[fn:1] more[fn:2]\n\n[fn:1] Def 1\n[fn:2] Def 2")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (info (org-combine-plists
                    (org-export--get-export-attributes)
                    (org-export-get-environment)
                    (org-export--collect-tree-properties
                     tree (org-export-get-environment)))))
        (mapcar (lambda (ref) (org-export-get-footnote-number ref info))
                (org-element-map tree 'footnote-reference #'identity))))))"##,
    );
}

#[test]
fn zeta_export_footnote_first_reference() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "Text[fn:1] more[fn:1]\n\n[fn:1] Def")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (info (org-combine-plists
                    (org-export--get-export-attributes)
                    (org-export-get-environment)
                    (org-export--collect-tree-properties
                     tree (org-export-get-environment)))))
        (mapcar (lambda (ref) (org-export-footnote-first-reference-p ref info))
                (org-element-map tree 'footnote-reference #'identity))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Zeta: org-element with complex export captions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn zeta_export_caption() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+CAPTION: My caption\n| a | b |")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (table (car (org-element-map tree 'table #'identity))))
        (org-export-get-caption table)))))"##,
    );
}

#[test]
fn zeta_export_caption_short() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+CAPTION[short]: long caption\n| a | b |")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (table (car (org-element-map tree 'table #'identity))))
        (list (org-export-get-caption table)
              (org-export-get-caption table t)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Zeta: org-element with complex export first/last sibling
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn zeta_export_first_last_sibling() {
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
         (mapcar #'org-export-last-sibling-p headlines))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Zeta: org-element with complex export node property
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn zeta_export_node_property() {
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

// ═══════════════════════════════════════════════════════════════════════
// Zeta: org-element with complex export optional title
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn zeta_export_optional_title() {
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
// Zeta: org-element with complex export filters
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn zeta_export_filter_apply() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (list
   ;; Applied in order.
   (org-export-filter-apply-functions
    (list (lambda (value &rest _) (concat "1" value))
          (lambda (value &rest _) (concat "2" value)))
    "0" nil)
   ;; Nil functions skipped.
   (org-export-filter-apply-functions
    (list #'ignore (lambda (value &rest _) (concat "2" value)))
    "0" nil)
   ;; All skipped: return initial.
   (org-export-filter-apply-functions (list #'ignore) "0" nil)
   ;; Empty string short-circuits.
   (org-export-filter-apply-functions
    (list (lambda (_value &rest _) "")
          (lambda (value &rest _) (concat "2" value)))
    "0" nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Zeta: org-element with complex export backends
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn zeta_export_define_backend() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let (org-export-registered-backends)
    (org-export-define-backend 'test '((headline . my-headline-test)))
    (list
     (org-export-get-all-transcoders 'test)
     (org-export-get-backend 'test))))"##,
    );
}

#[test]
fn zeta_export_derived_backend() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let (org-export-registered-backends)
    (org-export-define-backend 'parent '((:headline . parent)))
    (org-export-define-derived-backend 'test 'parent
      :translate-alist '((:headline . test)))
    (list
     (org-export-derived-backend-p 'test 'parent)
     (org-export-derived-backend-p 'test 'test)
     (let ((all (org-export-get-all-transcoders 'test)))
       (list (cdr (assq :headline all)))))))"##,
    );
}

#[test]
fn zeta_export_get_all_transcoders() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (list
   ;; Nil backend.
   (org-export-get-all-transcoders nil)
   ;; Simple.
   (org-export-get-all-transcoders
    (org-export-create-backend
     :transcoders '((headline . ignore))))
   ;; Inherit.
   (let (org-export-registered-backends)
     (org-export-define-backend 'b1 '((headline . ignore)))
     (org-export-get-all-transcoders
      (org-export-create-backend
       :parent 'b1 :transcoders '((section . ignore)))))))"##,
    );
}

#[test]
fn zeta_export_get_all_options() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (list
   ;; Nil backend.
   (org-export-get-all-options nil)
   ;; Simple.
   (org-export-get-all-options
    (org-export-create-backend
     :options '((:key1 value1))))
   ;; Inherit.
   (let (org-export-registered-backends)
     (org-export-define-backend 'b1 nil :options-alist '((:key1 value1)))
     (org-export-get-all-options
      (org-export-create-backend
       :parent 'b1 :options '((:key2 value2)))))))"##,
    );
}

#[test]
fn zeta_export_get_all_filters() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (list
   ;; Nil backend.
   (org-export-get-all-filters nil)
   ;; Simple.
   (org-export-get-all-filters
    (org-export-create-backend
     :filters '((:filter-headline . ignore))))
   ;; Inherit.
   (let (org-export-registered-backends)
     (org-export-define-backend 'b1
       nil :filters-alist '((:filter-headline . ignore)))
     (org-export-get-all-filters
      (org-export-create-backend
       :parent 'b1 :filters '((:filter-section . ignore)))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Zeta: org-element with complex export blocks/snippets
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn zeta_export_block_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+BEGIN_EXPORT html\n<p>HTML</p>\n#+END_EXPORT\n#+BEGIN_EXPORT latex\n\\textbf{LaTeX}\n#+END_EXPORT")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (blocks (org-element-map tree 'export-block #'identity)))
        (mapcar (lambda (b) (org-element-property :type b)) blocks))))"##,
    );
}

#[test]
fn zeta_export_snippet_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "@@html:<b>@@ @@latex:\\textbf{}@@ @@ascii:text@@")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (snippets (org-element-map tree 'export-snippet #'identity)))
        (mapcar (lambda (s) (org-element-property :back-end s)) snippets))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Zeta: org-element with complex export comments
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn zeta_export_comments() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "# Comment 1\n# Comment 2\n\n* H\nBody")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer)))
        (length (org-element-map tree 'comment #'identity)))))"##,
    );
}

#[test]
fn zeta_export_comment_tree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* COMMENT Hidden\nBody\n* Visible\nBody")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer)))
        (length (org-element-map tree 'headline #'identity)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Zeta: org-element with complex export read-attribute
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn zeta_export_read_attribute() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (list
     ;; Standard.
     (org-export-read-attribute
      :attr_html
      (with-temp-buffer (org-mode) (insert "#+ATTR_HTML: :a 1 :b 2\nParagraph")
        (goto-char (point-min)) (org-element-at-point)))
     ;; Empty.
     (org-export-read-attribute
      :attr_html
      (with-temp-buffer (org-mode) (insert "Paragraph")
        (goto-char (point-min)) (org-element-at-point)))
     ;; nil values.
     (org-export-read-attribute
      :attr_html
      (with-temp-buffer (org-mode) (insert "#+ATTR_HTML: :a nil :b nil\nParagraph")
        (goto-char (point-min)) (org-element-at-point))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Zeta: org-element with complex collect-keywords
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn zeta_collect_keywords() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Basic.
     (with-temp-buffer (org-mode)
       (insert "#+TITLE: My Title\n#+AUTHOR: Me\nBody")
       (goto-char (point-min))
       (org-collect-keywords '("TITLE" "AUTHOR")))
     ;; Not in block.
     (with-temp-buffer (org-mode)
       (insert "#+begin_example\n#+foo: bar\n#+end_example")
       (goto-char (point-min))
       (org-collect-keywords '("FOO")))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Zeta: org-element with complex outline path
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn zeta_get_outline_path() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Top-level.
     (with-temp-buffer (org-mode) (insert "* H") (goto-char (point-min))
       (org-get-outline-path))
     ;; Nested.
     (with-temp-buffer (org-mode) (insert "* H\n** S") (goto-char (point-max))
       (org-get-outline-path))
     ;; From body.
     (with-temp-buffer (org-mode) (insert "* H\n** S\nText") (goto-char (point-max))
       (org-get-outline-path))
     ;; With self.
     (with-temp-buffer (org-mode) (insert "* H") (goto-char (point-min))
       (org-get-outline-path t))
     ;; Empty headlines.
     (with-temp-buffer (org-mode) (insert "* H\n** ") (goto-char (point-max))
       (org-get-outline-path))
     ;; COMMENT removed.
     (with-temp-buffer (org-mode) (insert "* COMMENT This\n** COMMENT is\n*** test")
       (goto-char (point-max))
       (org-get-outline-path)))))"##,
    );
}

#[test]
fn zeta_format_outline_path() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (list
   (org-format-outline-path (list "one" "two" "three"))
   (org-format-outline-path '())
   (org-format-outline-path '(nil))
   (org-format-outline-path '() nil ">>")
   (org-format-outline-path (list "one\t" "tw o " "three  "))
   (org-format-outline-path (list "one" "two" "three") nil ">>" "|")
   (org-format-outline-path (list "one" "two" "three" "four") 10)
   (org-format-outline-path (list "one" "two" "three" "four") 2)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Zeta: org-element with complex end-of-meta-data
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn zeta_end_of_meta_data() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Skip planning.
     (with-temp-buffer (org-mode) (insert "* H\nSCHEDULED: <2014-03-04 tue.>")
       (goto-char (point-min)) (org-end-of-meta-data) (eobp))
     ;; Skip properties.
     (with-temp-buffer (org-mode) (insert "* H\n:PROPERTIES:\n:A: 1\n:END:")
       (goto-char (point-min)) (org-end-of-meta-data) (eobp))
     ;; Nothing to skip.
     (with-temp-buffer (org-mode) (insert "* H\nContents")
       (goto-char (point-min)) (org-end-of-meta-data) (looking-at "Contents"))
     ;; With argument: skip LOGBOOK.
     (with-temp-buffer (org-mode) (insert "* H\n:LOGBOOK:\nlog\n:END:\nContents")
       (goto-char (point-min)) (org-end-of-meta-data t) (looking-at "Contents")))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Zeta: org-element with complex end-of-subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn zeta_end_of_subtree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Simple.
     (with-temp-buffer (org-mode)
       (insert "\n* H\n** S1\n** S2\nasd\n* H2")
       (goto-char (point-min)) (forward-line 1) (org-end-of-subtree)
       (forward-char) (looking-at-p "^\\* H2"))
     ;; TO-HEADING.
     (with-temp-buffer (org-mode)
       (insert "\n* H\n** S1\n** S2\nasd\n* H2")
       (goto-char (point-min)) (forward-line 1) (org-end-of-subtree nil t)
       (looking-at-p "^\\* H2"))
     ;; Before first heading.
     (with-temp-buffer (org-mode)
       (insert "\nText.\n* H\n** S1\n** S2\nasd\n* H2")
       (goto-char (point-min)) (org-end-of-subtree) (eobp)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Zeta: org-element with complex forward/backward element
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn zeta_forward_element() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard.
     (with-temp-buffer (org-mode)
       (insert "First.\n\n\nSecond.")
       (goto-char (point-min)) (org-forward-element) (looking-at "Second."))
     ;; Greater element.
     (with-temp-buffer (org-mode)
       (insert "#+BEGIN_CENTER\nInside.\n#+END_CENTER\n\nOutside.")
       (goto-char (point-min)) (org-forward-element) (looking-at "Outside."))
     ;; Headline.
     (with-temp-buffer (org-mode)
       (insert "\n* H1\n** H1.1\n*** H1.1.1\n** H1.2")
       (goto-line 3) (org-forward-element) (looking-at "** H1.2"))
     ;; List.
     (with-temp-buffer (org-mode)
       (insert "\n- item1\n\n  - sub1\n\n  - sub2\n\n- item2\n\nOutside.")
       (goto-char (point-min)) (forward-line 1) (org-forward-element) (looking-at "Outside.")))))"##,
    );
}

#[test]
fn zeta_backward_element() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard.
     (with-temp-buffer (org-mode)
       (insert "P1.\n\nP2.")
       (goto-char (point-max)) (org-backward-element) (looking-at "P2."))
     ;; Headline.
     (with-temp-buffer (org-mode)
       (insert "\n* H1\n** H1.1\n*** H1.1.1\n** H1.2")
       (goto-line 5) (org-backward-element) (looking-at "** H1.1"))
     ;; Parent.
     (with-temp-buffer (org-mode)
       (insert "\n* H1\n** H1.1\n*** H1.1.1\n** H1.2")
       (goto-line 3) (org-backward-element) (looking-at "* H1"))
     ;; Greater element.
     (with-temp-buffer (org-mode)
       (insert "Before.\n#+BEGIN_CENTER\nInside.\n#+END_CENTER")
       (goto-line 3) (org-backward-element) (looking-at "#+BEGIN_CENTER"))
     ;; List.
     (with-temp-buffer (org-mode)
       (insert "\n- item1\n\n  - sub1\n\n  - sub2\n\n- item2\n\nOutside.")
       (goto-line 8) (org-backward-element) (looking-at "  - sub2")))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Zeta: org-element with complex up/down element
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn zeta_up_element() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Headline.
     (with-temp-buffer (org-mode)
       (insert "* H1\n** S1\n** S2")
       (goto-char (point-min)) (forward-line 2) (org-up-element) (looking-at "\\* H1"))
     ;; Greater element.
     (with-temp-buffer (org-mode)
       (insert "Before.\n#+BEGIN_CENTER\nP1\nP2\n#+END_CENTER")
       (goto-line 3) (org-up-element) (looking-at "#+BEGIN_CENTER"))
     ;; List.
     (with-temp-buffer (org-mode)
       (insert "* Top\n- item1\n\n  - sub1\n\n  - sub2\n\n    P.\n\n- item2")
       (goto-line 8) (org-up-element) (looking-at "  - sub2"))
     ;; Sub-list to parent.
     (with-temp-buffer (org-mode)
       (insert "* Top\n- item1\n\n  - sub1\n\n  - sub2\n\n- item2")
       (goto-line 4) (org-up-element) (looking-at "- item1")))))"##,
    );
}

#[test]
fn zeta_down_element() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; List.
     (with-temp-buffer (org-mode)
       (insert "- Item 1\n  - Item 1.1\n  - Item 2.2")
       (goto-char (point-min)) (forward-line 1) (org-down-element) (looking-at "- Item 1.1"))
     ;; Table.
     (with-temp-buffer (org-mode) (insert "| a | b |")
       (goto-char (point-min)) (org-down-element) (looking-at "a | b |"))
     ;; Greater element.
     (with-temp-buffer (org-mode)
       (insert "#+BEGIN_CENTER\nParagraph.\n#+END_CENTER")
       (goto-char (point-min)) (org-down-element) (looking-at "Paragraph.")))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Zeta: org-element with complex move subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn zeta_move_subtree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Move down.
     (with-temp-buffer (org-mode)
       (insert "* A\nBody A\n* B\nBody B\n* C\nBody C")
       (goto-char (point-min)) (org-move-subtree 1)
       (buffer-substring-no-properties (point-min) (point-max)))
     ;; Move up.
     (with-temp-buffer (org-mode)
       (insert "* A\nBody A\n* B\nBody B\n* C\nBody C")
       (goto-char (point-min)) (forward-line 2) (org-move-subtree -1)
       (buffer-substring-no-properties (point-min) (point-max))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Zeta: org-element with complex promote/demote
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn zeta_promote_demote() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Promote.
     (with-temp-buffer (org-mode) (insert "** H")
       (goto-char (point-min)) (org-promote) (buffer-string))
     ;; Demote.
     (with-temp-buffer (org-mode) (insert "* H")
       (goto-char (point-min)) (org-demote) (buffer-string))
     ;; Promote subtree.
     (with-temp-buffer (org-mode) (insert "** H1\n*** S1\n*** S2")
       (goto-char (point-min)) (org-promote-subtree) (buffer-string))
     ;; Demote subtree.
     (with-temp-buffer (org-mode) (insert "* H1\n** S1\n** S2")
       (goto-char (point-min)) (org-demote-subtree) (buffer-string)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Zeta: org-element with complex next/previous heading
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn zeta_next_visible_heading() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Forward.
     (with-temp-buffer (org-mode)
       (insert "Text\n* H1\n* H2\n* H3")
       (goto-char (point-min)) (org-next-visible-heading 1) (looking-at "\\* H1"))
     ;; Multiple.
     (with-temp-buffer (org-mode)
       (insert "Text\n* H1\n* H2\n* H3")
       (goto-char (point-min)) (org-next-visible-heading 2) (looking-at "\\* H2"))
     ;; Backward.
     (with-temp-buffer (org-mode)
       (insert "* H1\n* H2\n* H3\nText")
       (goto-char (point-max)) (org-previous-visible-heading 1) (looking-at "\\* H3"))
     ;; Multiple backward.
     (with-temp-buffer (org-mode)
       (insert "* H1\n* H2\n* H3\nText")
       (goto-char (point-max)) (org-previous-visible-heading 2) (looking-at "\\* H2")))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Zeta: org-element with complex forward-heading-same-level
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn zeta_forward_heading_same_level() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Forward.
     (with-temp-buffer (org-mode)
       (insert "* H1\n** S1\n** S2\n** S3\n* H2")
       (goto-char (point-min)) (forward-line 1)
       (org-forward-heading-same-level 1) (looking-at "\\*\\* S2"))
     ;; Forward past all.
     (with-temp-buffer (org-mode)
       (insert "* H1\n** S1\n** S2\n* H2")
       (goto-char (point-min)) (forward-line 2)
       (org-forward-heading-same-level 1) (looking-at "\\* H2"))
     ;; Backward.
     (with-temp-buffer (org-mode)
       (insert "* H1\n** S1\n** S2\n** S3\n* H2")
       (goto-char (point-min)) (forward-line 3)
       (org-forward-heading-same-level -1) (looking-at "\\*\\* S2")))))"##,
    );
}

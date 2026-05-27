//! Alpha-strict combo tests for org-mode extreme edge cases.

use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with all org-element-create spec
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_create_spec() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (list
   ;; With plist properties.
   (pcase (org-element-create 'foo '(:a 1 :b 2))
     (`(foo (:standard-properties ,_ :a 1 :b 2)) t))
   ;; Standard property in vector.
   (pcase (org-element-create 'foo '(:begin 10))
     (`(foo (:standard-properties ,vec))
      (= 10 (aref vec (org-element--property-idx :begin)))))
   ;; Strings.
   (equal "foo" (org-element-create "foo"))
   (equal "foo" (org-element-create 'plain-text nil "foo"))
   ;; Text properties on strings.
   (get-text-property 0 :a (org-element-create 'plain-text '(:a 1) "foo"))
   (get-text-property 0 :begin (org-element-create 'plain-text '(:begin 1) "foo"))
   ;; Children.
   (let ((children '("a" "b" (org-element-create 'foo))))
     (equal (cddr (apply #'org-element-create 'bar nil children))
            children))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with all org-element-put-property spec
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_put_property_spec() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard test: put on parsed bold.
     (with-temp-buffer (org-mode) (insert "* Headline\n *a*")
       (goto-char (point-min))
       (let ((tree (org-element-parse-buffer)))
         (org-element-put-property
          (org-element-map tree 'bold 'identity nil t) :test 1)
         (org-element-property
          :test (org-element-map tree 'bold 'identity nil t))))
     ;; Put property on a string.
     (org-element-property :test (org-element-put-property "Paragraph" :test t))
     ;; No properties: put :begin.
     (let ((element (list 'heading nil)) vec)
       (setq vec (make-vector (length org-element--standard-properties) nil))
       (aset vec 0 1)
       (equal (list 'heading (list :standard-properties vec))
              (org-element-put-property element :begin 1)))
     ;; No properties: put :begin1.
     (let ((element (list 'heading nil)))
       (equal (list 'heading (list :begin1 1))
              (org-element-put-property element :begin1 1)))
     ;; Standard property overwrite.
     (let ((element (list 'heading (list :standard-properties
                                         (make-vector (length org-element--standard-properties) 'foo)))))
       (= 1 (org-element-property-raw :begin (org-element-put-property element :begin 1)))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with all org-element-set-contents spec
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_set_contents_spec() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Accept multiple entries.
     (with-temp-buffer (org-mode) (insert "* Headline\n *a*")
       (goto-char (point-min))
       (let ((tree (org-element-parse-buffer)))
         (org-element-set-contents
          (org-element-map tree 'bold 'identity nil t) "b" '(italic nil "a"))
         (org-element-contents
          (org-element-map tree 'bold 'identity nil t))))
     ;; Accept atoms.
     (with-temp-buffer (org-mode) (insert "* Headline\n *a*")
       (goto-char (point-min))
       (let ((tree (org-element-parse-buffer)))
         (org-element-set-contents
          (org-element-map tree 'bold 'identity nil t) "b")
         (org-element-contents
          (org-element-map tree 'bold 'identity nil t))))
     ;; Accept elements.
     (with-temp-buffer (org-mode) (insert "* Headline\n *a*")
       (goto-char (point-min))
       (let ((tree (org-element-parse-buffer)))
         (org-element-set-contents
          (org-element-map tree 'bold 'identity nil t) '(italic nil "b"))
         (org-element-contents
          (org-element-map tree 'bold 'identity nil t))))
     ;; Allow nil contents.
     (with-temp-buffer (org-mode) (insert "* Headline\n *a*")
       (goto-char (point-min))
       (let ((tree (org-element-parse-buffer)))
         (org-element-set-contents (org-element-map tree 'bold 'identity nil t))
         (org-element-contents (org-element-map tree 'bold 'identity nil t)))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with all org-element-adopt-elements spec
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_adopt_elements_spec() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Adopt an element.
     (with-temp-buffer (org-mode) (insert "* Headline\n *a*")
       (goto-char (point-min))
       (let ((tree (org-element-parse-buffer)))
         (org-element-adopt
          (org-element-map tree 'bold 'identity nil t) '(italic nil "a"))
         (mapcar #'org-element-type
                 (org-element-contents
                  (org-element-map tree 'bold 'identity nil t)))))
     ;; Adopt a string.
     (with-temp-buffer (org-mode) (insert "* Headline\n *a*")
       (goto-char (point-min))
       (let ((tree (org-element-parse-buffer)))
         (org-element-adopt
          (org-element-map tree 'bold 'identity nil t) "b")
         (org-element-contents
          (org-element-map tree 'bold 'identity nil t)))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with all org-element-extract spec
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_extract_spec() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Extract a greater element.
     (with-temp-buffer (org-mode) (insert "* Headline")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (element (org-element-map tree 'headline 'identity nil t)))
         (org-element-extract element)
         (org-element-type tree)))
     ;; Extract an element.
     (with-temp-buffer (org-mode) (insert "Paragraph")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (element (org-element-map tree 'paragraph 'identity nil t)))
         (org-element-extract element)
         (org-element-map tree 'paragraph 'identity)))
     ;; Extract an object.
     (with-temp-buffer (org-mode) (insert "*bold*")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (element (org-element-map tree 'bold 'identity nil t)))
         (org-element-extract element)
         (org-element-map tree 'bold 'identity)))
     ;; Extract from secondary string.
     (with-temp-buffer (org-mode) (insert "* Headline *bold*")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (element (org-element-map tree 'bold 'identity nil t)))
         (org-element-extract element)
         (org-element-map tree 'bold 'identity)))
     ;; Return value has no :parent.
     (with-temp-buffer (org-mode) (insert "* Headline\n  Paragraph with *bold* text.")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (element (org-element-map tree 'bold 'identity nil t)))
         (org-element-property :parent (org-element-extract element)))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with all org-element-insert-before spec
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_insert_before_spec() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard test.
     (with-temp-buffer (org-mode) (insert "/some/ *paragraph*")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (_paragraph (org-element-map tree 'paragraph #'identity nil t))
              (bold (org-element-map tree 'bold 'identity nil t)))
         (org-element-insert-before '(entity (:name "\\alpha")) bold)
         (org-element-map tree '(bold entity italic) #'org-element-type nil)))
     ;; Insert in secondary string.
     (with-temp-buffer (org-mode) (insert "* /A/\n  Paragraph.")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (headline (org-element-map tree 'headline 'identity nil t))
              (italic (org-element-map tree 'italic 'identity nil t)))
         (org-element-insert-before '(entity (:name "\\alpha")) italic)
         (org-element-map (org-element-property :title headline) '(entity italic)
                          #'org-element-type))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with all org-element-set spec
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_set_spec() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; New element inserted.
     (with-temp-buffer (org-mode) (insert "* Headline\n*a*")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (bold (org-element-map tree 'bold 'identity nil t)))
         (org-element-set bold '(italic nil "b"))
         (org-element-map tree 'italic 'identity)))
     ;; Old element removed.
     (with-temp-buffer (org-mode) (insert "* Headline\n*a*")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (bold (org-element-map tree 'bold 'identity nil t)))
         (org-element-set bold '(italic nil "b"))
         (org-element-map tree 'bold 'identity)))
     ;; :parent correctly set.
     (with-temp-buffer (org-mode) (insert "* Headline\n*a*")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (bold (org-element-map tree 'bold 'identity nil t)))
         (org-element-set bold '(italic nil "b"))
         (org-element-type
          (org-element-property
           :parent (org-element-map tree 'italic 'identity nil t)))))
     ;; Replace strings with elements.
     (with-temp-buffer (org-mode) (insert "* Headline")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (text (org-element-map tree 'plain-text 'identity nil t)))
         (org-element-set text (list 'bold nil "b"))
         (org-element-map tree 'plain-text 'identity)))
     ;; Replace elements with strings.
     (with-temp-buffer (org-mode) (insert "* =verbatim=")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (verb (org-element-map tree 'verbatim 'identity nil t)))
         (org-element-set verb "a")
         (org-element-map tree 'plain-text 'identity nil t)))
     ;; Replace strings with strings.
     (with-temp-buffer (org-mode) (insert "a")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (text (org-element-map tree 'plain-text 'identity nil t)))
         (org-element-set text "b")
         (org-element-map tree 'plain-text 'identity nil t)))
     ;; KEEP-PROPS.
     (org-element-property
      :foo
      (org-element-set
       (org-element-create 'dummy '(:foo bar))
       (org-element-create 'dummy '(:foo2 bar2))
       '(:foo))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with all org-element-copy spec
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_copy_spec() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Preserve type.
     (with-temp-buffer (org-mode) (insert "*bold*")
       (goto-char (point-min))
       (org-element-type (org-element-copy (org-element-context))))
     ;; Preserve type for plain-text.
     (with-temp-buffer (org-mode) (insert "*bold*")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'plain-text
                         #'org-element-copy nil t)))
     ;; Preserve properties except :parent.
     (with-temp-buffer (org-mode) (insert "*bold*")
       (goto-char (point-min))
       (org-element-property :end (org-element-copy (org-element-context))))
     ;; No :parent on copy.
     (with-temp-buffer (org-mode) (insert "*bold*")
       (goto-char (point-min))
       (org-element-property :parent (org-element-copy (org-element-context))))
     ;; Copying nil returns nil.
     (org-element-copy nil)
     ;; Copy secondary strings.
     (equal '("text") (org-element-copy '("text")))
     ;; Not eq.
     (eq '("text") (org-element-copy '("text")))
     ;; Source not altered.
     (with-temp-buffer (org-mode) (insert "*bold*")
       (goto-char (point-min))
       (let* ((source (org-element-context))
              (copy (org-element-copy source)))
         (list (org-element-parent copy)
               (org-element-parent source)))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with affiliated keywords parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_affiliated_keywords_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Simple keyword.
     (with-temp-buffer (org-mode) (insert "#+NAME: para\nParagraph")
       (goto-char (point-min)) (org-element-property :name (org-element-at-point)))
     ;; Begin position.
     (with-temp-buffer (org-mode) (insert "#+NAME: para\nParagraph")
       (goto-char (point-min)) (org-element-property :begin (org-element-at-point)))
     ;; Multiple keywords.
     (with-temp-buffer (org-mode)
       (insert "#+ATTR_ASCII: line1\n#+ATTR_ASCII: line2\nParagraph")
       (goto-char (point-min)) (org-element-property :attr_ascii (org-element-at-point)))
     ;; Parsed keyword.
     (with-temp-buffer (org-mode) (insert "#+CAPTION: caption\nParagraph")
       (goto-char (point-min))
       (car (org-element-property :caption (org-element-at-point))))
     ;; Dual keyword.
     (with-temp-buffer (org-mode) (insert "#+CAPTION[short]: long\nParagraph")
       (goto-char (point-min)) (org-element-property :caption (org-element-at-point)))
     ;; Multiple captions.
     (with-temp-buffer (org-mode)
       (insert "#+CAPTION[s1]: l1\n#+CAPTION[s2]: l2\nParagraph")
       (goto-char (point-min)) (org-element-property :caption (org-element-at-point)))
     ;; Orphaned keyword: type check.
     (with-temp-buffer (org-mode) (insert "- item\n  #+name: name\nSome paragraph")
       (goto-char (point-min)) (search-forward "name")
       (org-element-type (org-element-at-point)))
     ;; Comments cannot have affiliated keywords.
     (with-temp-buffer (org-mode) (insert "#+name: foo\n# bar")
       (goto-char (point-min)) (search-forward "bar")
       (org-element-property :name (org-element-at-point))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with babel call parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_babel_call_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard.
     (with-temp-buffer (org-mode) (insert "#+CALL: test()")
       (goto-char (point-min)) (org-element-type (org-element-at-point)))
     ;; Ignore case.
     (with-temp-buffer (org-mode) (insert "#+call: test()")
       (goto-char (point-min)) (org-element-type (org-element-at-point)))
     ;; Call name.
     (with-temp-buffer (org-mode) (insert "#+CALL: test()")
       (goto-char (point-min)) (org-element-property :call (org-element-at-point)))
     ;; Inside header.
     (with-temp-buffer (org-mode) (insert "#+CALL: test[:results output]()")
       (goto-char (point-min)) (org-element-property :inside-header (org-element-at-point)))
     ;; Arguments.
     (with-temp-buffer (org-mode) (insert "#+CALL: test(n=4)")
       (goto-char (point-min)) (org-element-property :arguments (org-element-at-point)))
     ;; Nested arguments.
     (with-temp-buffer (org-mode) (insert "#+CALL: test(test())")
       (goto-char (point-min)) (org-element-property :arguments (org-element-at-point)))
     ;; End header.
     (with-temp-buffer (org-mode) (insert "#+CALL: test() :results html")
       (goto-char (point-min)) (org-element-property :end-header (org-element-at-point))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with bold parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_bold_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard bold.
     (with-temp-buffer (org-mode) (insert "*bold*")
       (goto-char (point-min))
       (org-element-map (org-element-parse-buffer) 'bold #'identity nil t))
     ;; Multi-line markup.
     (with-temp-buffer (org-mode) (insert "*first line\nsecond line*")
       (goto-char (point-min))
       (org-element-contents
        (org-element-map (org-element-parse-buffer) 'bold #'identity nil t))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with center block parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_center_block_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard.
     (with-temp-buffer (org-mode) (insert "#+BEGIN_CENTER\nText\n#+END_CENTER")
       (goto-char (point-min))
       (org-element-map (org-element-parse-buffer) 'center-block 'identity))
     ;; Ignore case.
     (with-temp-buffer (org-mode) (insert "#+begin_center\nText\n#+end_center")
       (goto-char (point-min))
       (org-element-map (org-element-parse-buffer) 'center-block 'identity))
     ;; Ignore incomplete block.
     (with-temp-buffer (org-mode) (insert "#+BEGIN_CENTER")
       (goto-char (point-min))
       (org-element-map (org-element-parse-buffer) 'center-block 'identity nil t)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with citation parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_citation_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (require 'oc)
  (let ((org-mode-hook nil))
    (list
     ;; Valid citation.
     (with-temp-buffer (org-mode) (insert "[cite:@key]")
       (goto-char (point-min)) (org-element-type (org-element-context)))
     ;; Invalid: no @.
     (with-temp-buffer (org-mode) (insert "[cite:text]")
       (goto-char (point-min)) (org-element-type (org-element-context)))
     ;; With style.
     (with-temp-buffer (org-mode) (insert "[cite/style:@key]")
       (goto-char (point-min)) (org-element-type (org-element-context)))
     ;; Style value.
     (with-temp-buffer (org-mode) (insert "[cite/style:@key]")
       (goto-char (point-min)) (org-element-property :style (org-element-context)))
     ;; Multi citations.
     (with-temp-buffer (org-mode) (insert "[cite:@a;@b;@c]")
       (goto-char (point-min)) (org-element-type (org-element-context))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with clock parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_clock_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard clock.
     (with-temp-buffer (org-mode)
       (insert "CLOCK: [2023-10-13 Fri 14:40]--[2023-10-13 Fri 14:51] =>  0:11")
       (goto-char (point-min)) (org-element-type (org-element-at-point)))
     ;; Clock value.
     (with-temp-buffer (org-mode)
       (insert "CLOCK: [2023-10-13 Fri 14:40]--[2023-10-13 Fri 14:51] =>  0:11")
       (goto-char (point-min)) (org-element-property :value (org-element-at-point)))
     ;; Duration.
     (with-temp-buffer (org-mode)
       (insert "CLOCK: [2023-10-13 Fri 14:40]--[2023-10-13 Fri 14:51] =>  0:11")
       (goto-char (point-min)) (org-element-property :duration (org-element-at-point))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with comment parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_comment_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard comment.
     (with-temp-buffer (org-mode) (insert "# This is a comment")
       (goto-char (point-min)) (org-element-type (org-element-at-point)))
     ;; Comment block.
     (with-temp-buffer (org-mode) (insert "#+BEGIN_COMMENT\nBlock comment\n#+END_COMMENT")
       (goto-char (point-min)) (org-element-type (org-element-at-point))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with comment block parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_comment_block_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard.
     (with-temp-buffer (org-mode) (insert "#+BEGIN_COMMENT\nSome comment\n#+END_COMMENT")
       (goto-char (point-min))
       (org-element-map (org-element-parse-buffer) 'comment-block 'identity))
     ;; Ignore case.
     (with-temp-buffer (org-mode) (insert "#+begin_comment\nSome comment\n#+end_comment")
       (goto-char (point-min))
       (org-element-map (org-element-parse-buffer) 'comment-block 'identity)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with diary sexp parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_diary_sexp_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "%%(org-anniversary 1956  5 14)(2) Arthur Dent is %d years old")
      (goto-char (point-min))
      (org-element-type (org-element-at-point)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with entity parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_entity_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard entity.
     (with-temp-buffer (org-mode) (insert "\\alpha")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'entity #'identity nil t)))
     ;; Entity name.
     (with-temp-buffer (org-mode) (insert "\\alpha")
       (goto-char (point-min))
       (org-element-property
        :name
        (org-element-map (org-element-parse-buffer) 'entity #'identity nil t))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with example block parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_example_block_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard.
     (with-temp-buffer (org-mode) (insert "#+BEGIN_EXAMPLE\nSome example\n#+END_EXAMPLE")
       (goto-char (point-min)) (org-element-type (org-element-at-point)))
     ;; With switches.
     (with-temp-buffer (org-mode) (insert "#+BEGIN_EXAMPLE -n\nSome example\n#+END_EXAMPLE")
       (goto-char (point-min)) (org-element-property :switches (org-element-at-point))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with export block parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_export_block_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard.
     (with-temp-buffer (org-mode) (insert "#+BEGIN_EXPORT html\n<p>Text</p>\n#+END_EXPORT")
       (goto-char (point-min)) (org-element-type (org-element-at-point)))
     ;; Export type.
     (with-temp-buffer (org-mode) (insert "#+BEGIN_EXPORT html\n<p>Text</p>\n#+END_EXPORT")
       (goto-char (point-min)) (org-element-property :type (org-element-at-point))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with fixed-width parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_fixed_width_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert ": fixed width line")
      (goto-char (point-min)) (org-element-type (org-element-at-point)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with footnote reference parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_footnote_ref_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard footnote ref.
     (with-temp-buffer (org-mode) (insert "Text[fn:1]")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'footnote-reference #'identity nil t)))
     ;; Inline footnote.
     (with-temp-buffer (org-mode) (insert "Text[fn:name:definition]")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'footnote-reference #'identity nil t))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with headline parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_headline_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard headline.
     (with-temp-buffer (org-mode) (insert "* Headline")
       (goto-char (point-min)) (org-element-type (org-element-at-point)))
     ;; Level.
     (with-temp-buffer (org-mode) (insert "*** Deep headline")
       (goto-char (point-min)) (org-element-property :level (org-element-at-point)))
     ;; TODO keyword.
     (with-temp-buffer (org-mode) (insert "* TODO Task")
       (goto-char (point-min)) (org-element-property :todo-keyword (org-element-at-point)))
     ;; Tags.
     (with-temp-buffer (org-mode) (insert "* Headline :tag1:tag2:")
       (goto-char (point-min)) (org-element-property :tags (org-element-at-point)))
     ;; Priority.
     (with-temp-buffer (org-mode) (insert "* [#A] Headline")
       (goto-char (point-min)) (org-element-property :priority (org-element-at-point)))
     ;; Raw value.
     (with-temp-buffer (org-mode) (insert "* TODO [#A] Headline :tag:")
       (goto-char (point-min))
       (substring-no-properties
        (org-element-property :raw-value (org-element-at-point)))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with horizontal rule parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_horizontal_rule_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert "-----")
      (goto-char (point-min)) (org-element-type (org-element-at-point)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with inline src block parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_inline_src_block_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard inline src.
     (with-temp-buffer (org-mode) (insert "src_emacs-lisp{(+ 1 2)}")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'inline-src-block #'identity nil t)))
     ;; Language.
     (with-temp-buffer (org-mode) (insert "src_emacs-lisp{(+ 1 2)}")
       (goto-char (point-min))
       (org-element-property
        :language
        (org-element-map (org-element-parse-buffer) 'inline-src-block #'identity nil t))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with inlinetask parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_inlinetask_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (require 'org-inlinetask)
  (let ((org-mode-hook nil)
        (org-inlinetask-min-level 4))
    (list
     ;; Standard inlinetask.
     (with-temp-buffer (org-mode) (insert "**** Inline task\nBody\n**** END")
       (goto-char (point-min)) (org-element-type (org-element-at-point)))
     ;; Level.
     (with-temp-buffer (org-mode) (insert "**** Inline task\nBody\n**** END")
       (goto-char (point-min)) (org-element-property :level (org-element-at-point))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with item parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_item_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard item.
     (with-temp-buffer (org-mode) (insert "- Item")
       (goto-char (point-min)) (org-element-type (org-element-at-point)))
     ;; Bullet type.
     (with-temp-buffer (org-mode) (insert "- Item")
       (goto-char (point-min)) (org-element-property :bullet (org-element-at-point)))
     ;; Checkbox.
     (with-temp-buffer (org-mode) (insert "- [X] Checked item")
       (goto-char (point-min)) (org-element-property :checkbox (org-element-at-point)))
     ;; Tag (description list).
     (with-temp-buffer (org-mode) (insert "- tag :: description")
       (goto-char (point-min)) (org-element-property :tag (org-element-at-point))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with keyword parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_keyword_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard keyword.
     (with-temp-buffer (org-mode) (insert "#+TITLE: My Title")
       (goto-char (point-min)) (org-element-type (org-element-at-point)))
     ;; Key.
     (with-temp-buffer (org-mode) (insert "#+TITLE: My Title")
       (goto-char (point-min)) (org-element-property :key (org-element-at-point)))
     ;; Value.
     (with-temp-buffer (org-mode) (insert "#+TITLE: My Title")
       (goto-char (point-min)) (org-element-property :value (org-element-at-point))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with latex environment parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_latex_environment_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard LaTeX environment.
     (with-temp-buffer (org-mode)
       (insert "\\begin{equation}\nx^2 + y^2 = z^2\n\\end{equation}")
       (goto-char (point-min)) (org-element-type (org-element-at-point)))
     ;; Environment value.
     (with-temp-buffer (org-mode)
       (insert "\\begin{equation}\nx^2 + y^2 = z^2\n\\end{equation}")
       (goto-char (point-min)) (org-element-property :value (org-element-at-point))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with latex fragment parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_latex_fragment_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Inline latex.
     (with-temp-buffer (org-mode) (insert "$x^2$")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'latex-fragment #'identity nil t)))
     ;; Display latex.
     (with-temp-buffer (org-mode) (insert "$$x^2$$")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'latex-fragment #'identity nil t))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with line break parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_line_break_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert "line1\\\\\nline2")
      (goto-char (point-min))
      (org-element-type
       (org-element-map (org-element-parse-buffer) 'line-break #'identity nil t)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with link parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_link_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard link.
     (with-temp-buffer (org-mode) (insert "https://example.org")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'link #'identity nil t)))
     ;; Explicit link.
     (with-temp-buffer (org-mode) (insert "[[https://example.org][desc]]")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'link #'identity nil t)))
     ;; Link type.
     (with-temp-buffer (org-mode) (insert "[[https://example.org][desc]]")
       (goto-char (point-min))
       (org-element-property
        :type
        (org-element-map (org-element-parse-buffer) 'link #'identity nil t)))
     ;; Link path.
     (with-temp-buffer (org-mode) (insert "[[https://example.org][desc]]")
       (goto-char (point-min))
       (org-element-property
        :path
        (org-element-map (org-element-parse-buffer) 'link #'identity nil t))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with node property parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_node_property_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H\n:PROPERTIES:\n:KEY: val\n:END:")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (np (car (org-element-map tree 'node-property #'identity))))
        (list (org-element-property :key np)
              (org-element-property :value np))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with paragraph parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_paragraph_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert "Simple paragraph.")
      (goto-char (point-min)) (org-element-type (org-element-at-point)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with planning parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_planning_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; DEADLINE.
     (with-temp-buffer (org-mode) (insert "* H\nDEADLINE: <2023-10-13 Fri>")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (planning (car (org-element-map tree 'planning #'identity))))
         (org-element-property :deadline planning)))
     ;; SCHEDULED.
     (with-temp-buffer (org-mode) (insert "* H\nSCHEDULED: <2023-10-13 Fri>")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (planning (car (org-element-map tree 'planning #'identity))))
         (org-element-property :scheduled planning))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with property drawer parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_property_drawer_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H\n:PROPERTIES:\n:KEY: val\n:END:")
      (goto-char (point-min))
      (org-element-type
       (org-element-map (org-element-parse-buffer) 'property-drawer #'identity)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with quote block parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_quote_block_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard.
     (with-temp-buffer (org-mode) (insert "#+BEGIN_QUOTE\nQuoted text\n#+END_QUOTE")
       (goto-char (point-min)) (org-element-type (org-element-at-point)))
     ;; Ignore case.
     (with-temp-buffer (org-mode) (insert "#+begin_quote\nQuoted text\n#+end_quote")
       (goto-char (point-min)) (org-element-type (org-element-at-point))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with section parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_section_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert "* Headline\nBody text.")
      (goto-char (point-min))
      (org-element-type
       (org-element-map (org-element-parse-buffer) 'section #'identity)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with special block parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_special_block_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard special block.
     (with-temp-buffer (org-mode) (insert "#+BEGIN_someblock\nContent\n#+END_someblock")
       (goto-char (point-min)) (org-element-type (org-element-at-point)))
     ;; Block type.
     (with-temp-buffer (org-mode) (insert "#+BEGIN_someblock\nContent\n#+END_someblock")
       (goto-char (point-min)) (org-element-property :type (org-element-at-point))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with src block parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_src_block_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard src block.
     (with-temp-buffer (org-mode) (insert "#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
       (goto-char (point-min)) (org-element-type (org-element-at-point)))
     ;; Language.
     (with-temp-buffer (org-mode) (insert "#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
       (goto-char (point-min)) (org-element-property :language (org-element-at-point)))
     ;; With switches.
     (with-temp-buffer (org-mode) (insert "#+BEGIN_SRC emacs-lisp -n\n(+ 1 2)\n#+END_SRC")
       (goto-char (point-min)) (org-element-property :switches (org-element-at-point))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with table parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_table_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard table.
     (with-temp-buffer (org-mode) (insert "| a | b |\n|---|\n| 1 | 2 |")
       (goto-char (point-min)) (org-element-type (org-element-at-point)))
     ;; Table type.
     (with-temp-buffer (org-mode) (insert "| a | b |\n|---|\n| 1 | 2 |")
       (goto-char (point-min)) (org-element-property :type (org-element-at-point))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with table cell parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_table_cell_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert "| a | b |")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (cells (org-element-map tree 'table-cell #'identity)))
        (mapcar (lambda (c)
                  (substring-no-properties
                   (org-element-interpret-data c)))
                cells)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with table row parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_table_row_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert "| a | b |\n|---|\n| 1 | 2 |")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (rows (org-element-map tree 'table-row #'identity)))
        (mapcar (lambda (r) (org-element-property :type r)) rows)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with timestamp parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_timestamp_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Active timestamp.
     (with-temp-buffer (org-mode) (insert "<2023-10-13 Fri>")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'timestamp #'identity nil t)))
     ;; Inactive timestamp.
     (with-temp-buffer (org-mode) (insert "[2023-10-13 Fri]")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'timestamp #'identity nil t)))
     ;; Timestamp type.
     (with-temp-buffer (org-mode) (insert "<2023-10-13 Fri>")
       (goto-char (point-min))
       (org-element-property
        :type
        (org-element-map (org-element-parse-buffer) 'timestamp #'identity nil t))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with underline parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_underline_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert "_underlined_")
      (goto-char (point-min))
      (org-element-type
       (org-element-map (org-element-parse-buffer) 'underline #'identity nil t)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with verbatim parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_verbatim_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert "=verbatim=")
      (goto-char (point-min))
      (org-element-type
       (org-element-map (org-element-parse-buffer) 'verbatim #'identity nil t)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Alpha: org-element with verse block parser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn alpha_all_verse_block_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+BEGIN_VERSE\nLine one\nLine two\n#+END_VERSE")
      (goto-char (point-min)) (org-element-type (org-element-at-point)))))"##,
    );
}

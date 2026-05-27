//! Ported upstream ERT tests from org-mode's test-org-element.el (9.7.11).
//!
//! Each upstream `ert-deftest` is converted to an `assert_oracle_parity`
//! call where `should` assertions become collected return values.

use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

// ── Getters: org-element-type ────────────────────────────────────────

#[test]
fn upstream_org_element_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (list
   ;; plain-text
   (org-element-type "string")
   ;; nil
   (org-element-type nil)
   ;; number
   (org-element-type 1)
   ;; symbol
   (org-element-type '(dummy))
   ;; with extra args
   (org-element-type '(dummy nil 'foo))
   (org-element-type '(dummy (:a a :b b) 'foo))
   ;; anonymous node
   (org-element-type '((dummy)))
   (org-element-type '((dummy)) t)
   (org-element-type '("string") t)
   (org-element-type '(1 2) t)))"##,
    );
}

#[test]
fn upstream_org_element_type_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (list
   (org-element-type-p '(foo) 'foo)
   (org-element-type-p '(foo) '(foo))
   (org-element-type-p '(foo) '(foo bar))
   (org-element-type-p '(foo) 'bar)
   (org-element-type-p '(foo) '(bar baz))
   (org-element-type-p "string" 'plain-text)
   (org-element-type-p '((foo)) 'anonymous)))"##,
    );
}

#[test]
fn upstream_org_element_class() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (list
   ;; Regular
   (org-element-class '(paragraph nil) nil)
   (org-element-class '(target nil) nil)
   ;; Special types
   (org-element-class '(org-data nil) nil)
   (org-element-class "text" nil)
   (org-element-class '("secondary " "string") nil)
   ;; Pseudo elements
   (org-element-class '(foo nil) nil)
   (org-element-class '(foo nil) '(center-block nil))
   (org-element-class '(foo nil) '(org-data nil))
   ;; Pseudo objects
   (org-element-class '(foo nil) '(bold nil))
   (org-element-class '(foo nil) '(paragraph nil))
   (org-element-class '(foo nil) '("secondary"))
   ;; In title secondary string
   (let* ((datum '(foo nil))
          (headline `(headline (:title (,datum) :secondary (:title)))))
     (org-element-put-property datum :parent headline)
     (org-element-class datum))))"##,
    );
}

// ── Getters: org-element-property-raw ────────────────────────────────

#[test]
fn upstream_org_element_property_raw_no_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (let ((results nil))
    (dolist (element `( nil
                        (headline nil)
                        (headline nil (headline))
                        "string"))
      (push (list (org-element-property-raw :begin element)
                  (org-element-property-raw :begin element 'default)
                  (org-element-property-raw :begin1 element)
                  (org-element-property-raw :begin1 element 'default))
            results))
    (nreverse results)))"##,
    );
}

#[test]
fn upstream_org_element_property_raw_non_standard() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (let ((results nil))
    (dolist (element `((headline (:begin1 1))
                       (headline (:begin1 1) (headline))
                       ,(propertize "string" :begin1 1)))
      (push (list (org-element-property-raw :begin element)
                  (org-element-property-raw :begin element 'default)
                  (org-element-property-raw :begin1 element)
                  (org-element-property-raw :begin1 element 'default))
            results))
    (nreverse results)))"##,
    );
}

#[test]
fn upstream_org_element_property_raw_standard_array() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (let ((results nil))
    (dolist (element `((headline (:standard-properties ,(make-vector 10 'test)))
                       (headline (:standard-properties ,(make-vector 10 'test)) (headline))))
      (push (list (org-element-property-raw :begin element)
                  (org-element-property-raw :begin element 'default)
                  (org-element-property-raw :begin1 element)
                  (org-element-property-raw :begin1 element 'default))
            results))
    (nreverse results)))"##,
    );
}

#[test]
fn upstream_org_element_property_raw_plist() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (let ((results nil))
    (dolist (element `((headline (:begin 1))
                       (headline (:begin 1) (headline))
                       ,(propertize "string" :begin 1)))
      (push (list (org-element-property-raw :begin element)
                  (org-element-property-raw :begin element 'default)
                  (org-element-property-raw :begin1 element)
                  (org-element-property-raw :begin1 element 'default))
            results))
    (nreverse results)))"##,
    );
}

#[test]
fn upstream_org_element_property_raw_mixed() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (let ((results nil))
    (dolist (element `((headline (:standard-properties ,(make-vector 10 'test) :begin 1))
                       (headline (:begin 1 :standard-properties ,(make-vector 10 'test)))
                       (headline (:standard-properties ,(make-vector 10 'test) :begin 1) (headline))))
      (push (list (org-element-property-raw :begin element)
                  (org-element-property-raw :begin element 'default)
                  (org-element-property-raw :begin1 element)
                  (org-element-property-raw :begin1 element 'default))
            results))
    (nreverse results)))"##,
    );
}

#[test]
fn upstream_org_element_property_raw_general() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (let ((results nil))
    (dolist (element `((headline (:standard-properties ,(make-vector 10 'test) :begin1 1))
                       (headline (:begin1 1 :standard-properties ,(make-vector 10 'test)))
                       (headline (:standard-properties ,(make-vector 10 'test) :begin1 1) (headline))))
      (push (list (org-element-property-raw :begin element)
                  (org-element-property-raw :begin element 'default)
                  (org-element-property-raw :begin1 element)
                  (org-element-property-raw :begin1 element 'default))
            results))
    (nreverse results)))"##,
    );
}

// ── Getters: org-element-property (deferred) ─────────────────────────

#[test]
fn upstream_org_element_property_deferred() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (list
   ;; Resolve :deferred property
   (let ((el (org-element-create
              'dummy
              `(:deferred
                ,(org-element-deferred-create
                  t (lambda (el) (org-element-put-property el :foo 'bar) nil))))))
     (list (org-element-property :foo el)
           (org-element-property :foo2 el)))
   ;; Deferred value
   (let ((el (org-element-create
              'dummy
              `(:foo ,(org-element-deferred-create nil (lambda (_) 'bar))))))
     (org-element-property :foo el))
   ;; Auto-undefer
   (let ((el (org-element-create
              'dummy
              `(:foo ,(org-element-deferred-create t (lambda (_) 'bar))))))
     (list (org-element-property :foo el)
           (org-element-property-raw :foo el)))
   ;; Force undefer
   (let ((el (org-element-create
              'dummy
              `(:foo ,(org-element-deferred-create nil (lambda (_) 'bar))))))
     (list (org-element-property :foo el)
           (org-element-property-raw :foo el)
           (org-element-property :foo el nil 'force)
           (org-element-property-raw :foo el)))
   ;; Deferred alias
   (let ((el (org-element-create
              'dummy
              `( :foo 1
                 :bar ,(org-element-deferred-create-alias :foo)))))
     (list (org-element-property :foo el)
           (org-element-property :bar el)))
   ;; Deferred list
   (let ((el (org-element-create
              'dummy
              `(:foo ,(org-element-deferred-create-list
                       (list 1 2 (org-element-deferred-create nil (lambda (_) 3))))))))
     (org-element-property :foo el))
   ;; Deferred with side effects (retry)
   (let ((el (org-element-create
              'dummy
              `(:foo ,(org-element-deferred-create
                       nil (lambda (el)
                             (org-element-put-property el :foo 1)
                             (throw :org-element-deferred-retry nil)))))))
     (org-element-property :foo el))
   ;; Recursive undefer
   (let ((el (org-element-create
              'dummy
              `(:foo ,(org-element-deferred-create
                       nil (lambda (el)
                             (org-element-deferred-create
                              nil (lambda (_) 1)))))))
     (org-element-property :foo el))))"##,
    );
}

#[test]
fn upstream_org_element_property_2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (let ((el (org-element-create 'dummy '(:foo bar))))
    (eq (org-element-property :foo el)
        (org-element-property-2 el :foo))))"##,
    );
}

#[test]
fn upstream_org_element_parent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (let ((el (org-element-create 'dummy '(:parent bar))))
    (eq (org-element-property :parent el)
        (org-element-parent el))))"##,
    );
}

// ── Getters: org-element-properties-resolve ──────────────────────────

#[test]
fn upstream_org_element_properties_resolve() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (let ((el (org-element-create
             'dummy
             `( :foo ,(org-element-deferred-create t (lambda (_) 1))
                :bar ,(org-element-deferred-create nil (lambda (_) 2))
                :deferred ,(org-element-deferred-create
                            t nil (lambda (el)
                                    (org-element-put-property el :baz 3)))))))
    ;; Resolve conditionally.
    (setq el (org-element-properties-resolve el))
    (let ((r1 (list (org-element-property-raw :foo el)
                    (org-element-property-raw :bar el)
                    (org-element-property :bar el)
                    (org-element-property-raw :baz el))))
      ;; Resolve unconditionally.
      (setq el (org-element-properties-resolve el 'force))
      (list r1 (org-element-property-raw :bar el)))))"##,
    );
}

// ── Getters: org-element-secondary-p ─────────────────────────────────

#[test]
fn upstream_org_element_secondary_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (list
   ;; In a secondary string, return property name.
   (let ((org-mode-hook nil))
     (with-temp-buffer
       (org-mode)
       (insert "* Headline *object*")
       (goto-char (point-min))
       (org-element-map (org-element-parse-buffer) 'bold
         (lambda (object) (org-element-secondary-p object))
         nil t)))
   ;; Manual secondary string construction.
   (org-element-secondary-p
    (let* ((el (org-element-create
                'dummy '(:secondary (:foo))))
           (child (org-element-create "string" `(:parent ,el))))
      (org-element-put-property el :foo (list child))
      child))
   ;; Outside a secondary string, return nil.
   (let ((org-mode-hook nil))
     (with-temp-buffer
       (org-mode)
       (insert "Paragraph *object*")
       (goto-char (point-min))
       (org-element-map (org-element-parse-buffer) 'bold
         (lambda (object) (org-element-type (org-element-secondary-p object)))
         nil t)))
   ;; Wrong secondary property.
   (eq :foo
       (org-element-secondary-p
        (let* ((el (org-element-create
                    'dummy '(:secondary (:foo))))
               (child (org-element-create "string" `(:parent ,el))))
          (org-element-put-property el :bar (list child))
          child)))))"##,
    );
}

// ── Map: org-element-map ─────────────────────────────────────────────

#[test]
fn upstream_org_element_map_plain_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "Some text \alpha
#+BEGIN_CENTER
Some other text
#+END_CENTER")
      (goto-char (point-min))
      (let ((count 0))
        (org-element-map
            (org-element-parse-buffer) 'plain-text
          (lambda (s) (when (string-match "text" s) (cl-incf count))))
        count))))"##,
    );
}

#[test]
fn upstream_org_element_map_secondary_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  ;; Applies to secondary strings
  (org-element-map '("some " (bold nil "bold") "text") 'bold 'identity))"##,
    );
}

#[test]
fn upstream_org_element_map_enter_secondary_first() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  ;; Enter secondary strings before entering contents.
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "* Some \\alpha headline\n\\beta entity.")
      (goto-char (point-min))
      (org-element-property
       :name
       (org-element-map (org-element-parse-buffer) 'entity 'identity nil t)))))"##,
    );
}

#[test]
fn upstream_org_element_map_no_recursion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  ;; Apply NO-RECURSION argument.
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "#+BEGIN_CENTER\n\\alpha\n#+END_CENTER")
      (goto-char (point-min))
      (org-element-map
          (org-element-parse-buffer) 'entity 'identity nil nil 'center-block))))"##,
    );
}

#[test]
fn upstream_org_element_map_with_affiliated() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  ;; Use WITH-AFFILIATED argument.
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "#+CAPTION[a]: 1\n#+CAPTION[b]: 2\nParagraph")
      (goto-char (point-min))
      (org-element-map
          (org-element-at-point) 'plain-text 'identity nil nil nil t))))"##,
    );
}

// ── Map: org-element-ast-map ─────────────────────────────────────────

#[test]
fn upstream_org_element_ast_map_types_t() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  ;; TYPES = t
  (org-element-ast-map
      (org-element-create 'anonymous nil "a" "b" (org-element-create 'bold))
      t #'org-element-type))"##,
    );
}

#[test]
fn upstream_org_element_ast_map_ignore() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  ;; IGNORE
  (let ((bold (org-element-create 'bold)))
    (org-element-ast-map
        (org-element-create 'anonymous nil "a" "b" bold)
        t #'org-element-type (list bold))))"##,
    );
}

#[test]
fn upstream_org_element_ast_map_fun_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  ;; FUN as a list form
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "* H1\n* H2")
      (goto-char (point-min))
      (org-element-map
          (org-element-parse-buffer)
          t '(org-element-property :raw-value node)))))"##,
    );
}

#[test]
fn upstream_org_element_ast_map_extra_secondary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (list
   ;; Extra secondary properties.
   (org-element-ast-map
       (org-element-create
        'dummy
        `(:foo ,(org-element-create 'bold))
        (org-element-create 'bold))
       'bold #'org-element-type
       nil nil nil '(:foo))
   ;; Without extra secondary - should differ
   (org-element-ast-map
       (org-element-create
        'dummy
        `(:foo ,(org-element-create 'bold))
        (org-element-create 'bold))
       'bold #'org-element-type)))"##,
    );
}

#[test]
fn upstream_org_element_ast_map_no_secondary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (list
   ;; no-secondary flag
   (org-element-ast-map
       (org-element-create
        'dummy
        `(:secondary (:foo) :foo ,(org-element-create 'bold))
        (org-element-create 'bold))
       'bold #'org-element-type
       nil nil nil nil 'no-secondary)
   ;; Without no-secondary
   (org-element-ast-map
       (org-element-create
        'dummy
        `(:secondary (:foo) :foo ,(org-element-create 'bold))
        (org-element-create 'bold))
       'bold #'org-element-type)))"##,
    );
}

#[test]
fn upstream_org_element_ast_map_deferred() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (list
   ;; no-undefer
   (org-element-ast-map
       (org-element-create
        'dummy
        `(:secondary (:foo) :foo ,(org-element-deferred-create nil (lambda (_) "a")))
        (org-element-create 'bold))
       t #'org-element-type
       nil nil nil nil nil 'no-undefer)
   ;; Default (with undefer)
   (org-element-ast-map
       (org-element-create
        'dummy
        `(:secondary (:foo) :foo ,(org-element-deferred-create nil (lambda (_) "a")))
        (org-element-create 'bold))
       t #'org-element-type)))"##,
    );
}

// ── org-element-properties-mapc ──────────────────────────────────────

#[test]
fn upstream_org_element_properties_mapc() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (let ((el (org-element-create 'dummy '(:a 1 :b 2 :c 3)))
        (acc nil))
    (org-element-properties-mapc
     (lambda (prop val) (push (list prop val) acc))
     el)
    (sort acc (lambda (a b) (string< (symbol-name (car a))
                                      (symbol-name (car b)))))))"##,
    );
}

// ── org-element-put-property ─────────────────────────────────────────

#[test]
fn upstream_org_element_put_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (let ((el (org-element-create 'dummy '(:a 1))))
    (org-element-put-property el :b 2)
    (org-element-put-property el :a 99)
    (list (org-element-property :a el)
          (org-element-property :b el))))"##,
    );
}

// ── org-element-set-contents ─────────────────────────────────────────

#[test]
fn upstream_org_element_set_contents() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (let ((el (org-element-create 'dummy nil "old")))
    (org-element-set-contents el "new1" "new2")
    (org-element-contents el)))"##,
    );
}

// ── org-element-copy ─────────────────────────────────────────────────

#[test]
fn upstream_org_element_copy() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (let* ((original (org-element-create 'headline '(:level 1 :raw-value "Test")))
         (copy (org-element-copy original)))
    (list (eq original copy)
          (equal (org-element-property :level original)
                 (org-element-property :level copy))
          (equal (org-element-property :raw-value original)
                 (org-element-property :raw-value copy))
          ;; Deep copy: modifying copy shouldn't affect original
          (progn (org-element-put-property copy :level 99)
                 (list (org-element-property :level original)
                       (org-element-property :level copy))))))"##,
    );
}

// ── org-element-create ───────────────────────────────────────────────

#[test]
fn upstream_org_element_create_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (list
   ;; Simple element
   (org-element-type (org-element-create 'paragraph))
   ;; With properties
   (org-element-property :level (org-element-create 'headline '(:level 3)))
   ;; With contents
   (org-element-contents (org-element-create 'section nil "body"))
   ;; With properties and contents
   (let ((el (org-element-create 'headline '(:level 1) (org-element-create 'section nil "text"))))
     (list (org-element-property :level el)
           (org-element-type (car (org-element-contents el)))))))"##,
    );
}

// ── org-element-lineage ──────────────────────────────────────────────

#[test]
fn upstream_org_element_lineage_filtered_repeat() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "* H1\nParagraph with *bold* text.")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (bold (car (org-element-map tree 'bold #'identity))))
       (list
        ;; Full lineage
        (mapcar #'org-element-type (org-element-lineage bold))
        ;; With self
        (mapcar #'org-element-type (org-element-lineage bold nil t))
        ;; With types filter
        (mapcar #'org-element-type (org-element-lineage bold 'headline)))))))"##,
    );
}

// ── org-element-interpret-data ───────────────────────────────────────

#[test]
fn upstream_org_element_interpret_data() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "* Title\nParagraph /italic/ and *bold*.\n")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (interpreted (org-element-interpret-data tree)))
        ;; Round-trip: parse then interpret should preserve structure
        (substring-no-properties interpreted 0 30)))))"##,
    );
}

// ── org-element-at-point ─────────────────────────────────────────────

#[test]
fn upstream_org_element_at_point() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "* Heading\nParagraph text.\n* Another\n")
      (goto-char (point-min))
      (list
       ;; On heading
       (org-element-type (org-element-at-point))
       ;; In paragraph
       (progn (forward-line 2)
              (org-element-type (org-element-at-point)))
       ;; On second heading
       (progn (forward-line 2)
              (org-element-type (org-element-at-point)))))))"##,
    );
}

// ── org-element-context ──────────────────────────────────────────────

#[test]
fn upstream_org_element_context() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "Text with *bold* and /italic/ words.")
      (goto-char (point-min))
      (search-forward "bold")
      (list
       ;; On bold
       (org-element-type (org-element-context))
       ;; On italic
       (progn (search-forward "italic")
              (org-element-type (org-element-context)))
       ;; On plain text
       (progn (search-forward "words")
              (org-element-type (org-element-context)))))))"##,
    );
}

// ── org-element-parse-buffer ─────────────────────────────────────────

#[test]
fn upstream_org_element_parse_buffer_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "* H1\nBody 1\n** H2\nBody 2\n")
      (insert "#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC\n")
      (insert "| a | b |\n| 1 | 2 |\n")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (types (org-element-map tree t #'org-element-type)))
        ;; Unique types present
        (delete-dups (copy-sequence types))))))"##,
    );
}

// ── org-element-property-access (combined) ───────────────────────────

#[test]
fn upstream_org_element_property_access_combined() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "* TODO [#A] Headline :tag1:tag2:\nBody text.\n")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (hl (car (org-element-map tree 'headline #'identity))))
        (list
         (org-element-property :level hl)
         (org-element-property :todo-keyword hl)
         (org-element-property :priority hl)
         (org-element-property :tags hl)
         (substring-no-properties (org-element-property :raw-value hl)))))))"##,
    );
}

// ── org-element-adopt / org-element-extract ──────────────────────────

#[test]
fn upstream_org_element_adopt_extract() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (let* ((parent (org-element-create 'section nil))
         (child1 (org-element-create 'paragraph nil "p1"))
         (child2 (org-element-create 'paragraph nil "p2")))
    (org-element-adopt parent child1 child2)
    (let ((after-adopt (mapcar #'org-element-type (org-element-contents parent))))
      (org-element-extract child1)
      (list after-adopt
            (mapcar #'org-element-type (org-element-contents parent))))))"##,
    );
}

// ── org-element-map with first-match ─────────────────────────────────

#[test]
fn upstream_org_element_map_first_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "* H1\n* H2\n* H3\n")
      (goto-char (point-min))
      ;; FIRST-MATCH = t: return only first match
      (org-element-property
       :raw-value
       (org-element-map (org-element-parse-buffer) 'headline #'identity nil t)))))"##,
    );
}

// ── org-element-map accumulate vs first ──────────────────────────────

#[test]
fn upstream_org_element_map_accumulate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "* H1\n* H2\n* H3\n")
      (goto-char (point-min))
      ;; Default: accumulate all matches
      (mapcar (lambda (h) (org-element-property :raw-value h))
              (org-element-map (org-element-parse-buffer) 'headline #'identity)))))"##,
    );
}

// ── Setters: org-element-create (upstream) ───────────────────────────

#[test]
fn upstream_org_element_create_spec() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (list
   ;; With plist properties
   (pcase (org-element-create 'foo '(:a 1 :b 2))
     (`(foo (:standard-properties ,_ :a 1 :b 2)) t))
   ;; Standard property in vector
   (pcase (org-element-create 'foo '(:begin 10))
     (`(foo (:standard-properties ,vec))
      (= 10 (aref vec (org-element--property-idx :begin)))))
   ;; Strings
   (equal "foo" (org-element-create "foo"))
   (equal "foo" (org-element-create 'plain-text nil "foo"))
   ;; Text properties on strings
   (get-text-property 0 :a (org-element-create 'plain-text '(:a 1) "foo"))
   (get-text-property 0 :begin (org-element-create 'plain-text '(:begin 1) "foo"))
   ;; Children
   (let ((children '("a" "b" (org-element-create 'foo))))
     (equal (cddr (apply #'org-element-create 'bar nil children))
            children))))"##,
    );
}

// ── Setters: put-property (upstream) ─────────────────────────────────

#[test]
fn upstream_org_element_put_property_spec() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (list
   ;; Standard test: put on parsed bold
   (let ((org-mode-hook nil))
     (with-temp-buffer
       (org-mode)
       (insert "* Headline\n *a*")
       (goto-char (point-min))
       (let ((tree (org-element-parse-buffer)))
         (org-element-put-property
          (org-element-map tree 'bold 'identity nil t) :test 1)
         (org-element-property
          :test (org-element-map tree 'bold 'identity nil t)))))
   ;; Put property on a string.
   (org-element-property :test (org-element-put-property "Paragraph" :test t))
   ;; No properties: put :begin
   (let ((element (list 'heading nil))
         vec)
     (setq vec (make-vector (length org-element--standard-properties) nil))
     (aset vec 0 1)
     (equal (list 'heading (list :standard-properties vec))
            (org-element-put-property element :begin 1)))
   ;; No properties: put :begin1
   (let ((element (list 'heading nil)))
     (equal (list 'heading (list :begin1 1))
            (org-element-put-property element :begin1 1)))
   ;; Standard property overwrite
   (let ((element (list 'heading (list :standard-properties
                                       (make-vector (length org-element--standard-properties) 'foo)))))
     (= 1 (org-element-property-raw :begin (org-element-put-property element :begin 1))))))"##,
    );
}

// ── Setters: set-contents (upstream) ─────────────────────────────────

#[test]
fn upstream_org_element_set_contents_spec() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Accept multiple entries.
     (with-temp-buffer
       (org-mode)
       (insert "* Headline\n *a*")
       (goto-char (point-min))
       (let ((tree (org-element-parse-buffer)))
         (org-element-set-contents
          (org-element-map tree 'bold 'identity nil t) "b" '(italic nil "a"))
         (org-element-contents
          (org-element-map tree 'bold 'identity nil t))))
     ;; Accept atoms.
     (with-temp-buffer
       (org-mode)
       (insert "* Headline\n *a*")
       (goto-char (point-min))
       (let ((tree (org-element-parse-buffer)))
         (org-element-set-contents
          (org-element-map tree 'bold 'identity nil t) "b")
         (org-element-contents
          (org-element-map tree 'bold 'identity nil t))))
     ;; Accept elements.
     (with-temp-buffer
       (org-mode)
       (insert "* Headline\n *a*")
       (goto-char (point-min))
       (let ((tree (org-element-parse-buffer)))
         (org-element-set-contents
          (org-element-map tree 'bold 'identity nil t) '(italic nil "b"))
         (org-element-contents
          (org-element-map tree 'bold 'identity nil t))))
     ;; Allow nil contents.
     (with-temp-buffer
       (org-mode)
       (insert "* Headline\n *a*")
       (goto-char (point-min))
       (let ((tree (org-element-parse-buffer)))
         (org-element-set-contents (org-element-map tree 'bold 'identity nil t))
         (org-element-contents (org-element-map tree 'bold 'identity nil t)))))))"##,
    );
}

// ── Setters: adopt-elements (upstream) ───────────────────────────────

#[test]
fn upstream_org_element_adopt_spec() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Adopt an element.
     (with-temp-buffer
       (org-mode)
       (insert "* Headline\n *a*")
       (goto-char (point-min))
       (let ((tree (org-element-parse-buffer)))
         (org-element-adopt
          (org-element-map tree 'bold 'identity nil t) '(italic nil "a"))
         (mapcar #'org-element-type
                 (org-element-contents
                  (org-element-map tree 'bold 'identity nil t)))))
     ;; Adopt a string.
     (with-temp-buffer
       (org-mode)
       (insert "* Headline\n *a*")
       (goto-char (point-min))
       (let ((tree (org-element-parse-buffer)))
         (org-element-adopt
          (org-element-map tree 'bold 'identity nil t) "b")
         (org-element-contents
          (org-element-map tree 'bold 'identity nil t)))))))"##,
    );
}

// ── Setters: extract-element (upstream) ──────────────────────────────

#[test]
fn upstream_org_element_extract_spec() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Extract a greater element.
     (with-temp-buffer
       (org-mode)
       (insert "* Headline")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (element (org-element-map tree 'headline 'identity nil t)))
         (org-element-extract element)
         (org-element-type tree)))
     ;; Extract an element.
     (with-temp-buffer
       (org-mode)
       (insert "Paragraph")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (element (org-element-map tree 'paragraph 'identity nil t)))
         (org-element-extract element)
         (org-element-map tree 'paragraph 'identity)))
     ;; Extract an object.
     (with-temp-buffer
       (org-mode)
       (insert "*bold*")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (element (org-element-map tree 'bold 'identity nil t)))
         (org-element-extract element)
         (org-element-map tree 'bold 'identity)))
     ;; Extract from secondary string.
     (with-temp-buffer
       (org-mode)
       (insert "* Headline *bold*")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (element (org-element-map tree 'bold 'identity nil t)))
         (org-element-extract element)
         (org-element-map tree 'bold 'identity)))
     ;; Return value has no :parent.
     (with-temp-buffer
       (org-mode)
       (insert "* Headline\n  Paragraph with *bold* text.")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (element (org-element-map tree 'bold 'identity nil t)))
         (org-element-property :parent (org-element-extract element)))))))"##,
    );
}

// ── Setters: insert-before (upstream) ────────────────────────────────

#[test]
fn upstream_org_element_insert_before_spec() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard test.
     (with-temp-buffer
       (org-mode)
       (insert "/some/ *paragraph*")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (_paragraph (org-element-map tree 'paragraph #'identity nil t))
              (bold (org-element-map tree 'bold 'identity nil t)))
         (org-element-insert-before '(entity (:name "\\alpha")) bold)
         (org-element-map tree '(bold entity italic) #'org-element-type nil)))
     ;; Insert in secondary string.
     (with-temp-buffer
       (org-mode)
       (insert "* /A/\n  Paragraph.")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (headline (org-element-map tree 'headline 'identity nil t))
              (italic (org-element-map tree 'italic 'identity nil t)))
         (org-element-insert-before '(entity (:name "\\alpha")) italic)
         (org-element-map (org-element-property :title headline) '(entity italic)
                          #'org-element-type))))))"##,
    );
}

// ── Setters: set (upstream) ──────────────────────────────────────────

#[test]
fn upstream_org_element_set_spec() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; New element inserted.
     (with-temp-buffer
       (org-mode)
       (insert "* Headline\n*a*")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (bold (org-element-map tree 'bold 'identity nil t)))
         (org-element-set bold '(italic nil "b"))
         (org-element-map tree 'italic 'identity)))
     ;; Old element removed.
     (with-temp-buffer
       (org-mode)
       (insert "* Headline\n*a*")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (bold (org-element-map tree 'bold 'identity nil t)))
         (org-element-set bold '(italic nil "b"))
         (org-element-map tree 'bold 'identity)))
     ;; :parent correctly set.
     (with-temp-buffer
       (org-mode)
       (insert "* Headline\n*a*")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (bold (org-element-map tree 'bold 'identity nil t)))
         (org-element-set bold '(italic nil "b"))
         (org-element-type
          (org-element-property
           :parent (org-element-map tree 'italic 'identity nil t)))))
     ;; Replace strings with elements.
     (with-temp-buffer
       (org-mode)
       (insert "* Headline")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (text (org-element-map tree 'plain-text 'identity nil t)))
         (org-element-set text (list 'bold nil "b"))
         (org-element-map tree 'plain-text 'identity)))
     ;; Replace elements with strings.
     (with-temp-buffer
       (org-mode)
       (insert "* =verbatim=")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (verb (org-element-map tree 'verbatim 'identity nil t)))
         (org-element-set verb "a")
         (org-element-map tree 'plain-text 'identity nil t)))
     ;; Replace strings with strings.
     (with-temp-buffer
       (org-mode)
       (insert "a")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (text (org-element-map tree 'plain-text 'identity nil t)))
         (org-element-set text "b")
         (org-element-map tree 'plain-text 'identity nil t)))
     ;; KEEP-PROPS
     (org-element-property
      :foo
      (org-element-set
       (org-element-create 'dummy '(:foo bar))
       (org-element-create 'dummy '(:foo2 bar2))
       '(:foo))))))"##,
    );
}

// ── Setters: copy (upstream) ─────────────────────────────────────────

#[test]
fn upstream_org_element_copy_spec() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Preserve type.
     (with-temp-buffer
       (org-mode)
       (insert "*bold*")
       (goto-char (point-min))
       (org-element-type (org-element-copy (org-element-context))))
     ;; Preserve type for plain-text.
     (with-temp-buffer
       (org-mode)
       (insert "*bold*")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'plain-text
                         #'org-element-copy nil t)))
     ;; Preserve properties except :parent.
     (with-temp-buffer
       (org-mode)
       (insert "*bold*")
       (goto-char (point-min))
       (org-element-property :end (org-element-copy (org-element-context))))
     ;; No :parent on copy.
     (with-temp-buffer
       (org-mode)
       (insert "*bold*")
       (goto-char (point-min))
       (org-element-property :parent (org-element-copy (org-element-context))))
     ;; Copying nil returns nil.
     (org-element-copy nil)
     ;; Copy secondary strings.
     (equal '("text") (org-element-copy '("text")))
     ;; Not eq.
     (eq '("text") (org-element-copy '("text")))
     ;; Source not altered.
     (with-temp-buffer
       (org-mode)
       (insert "*bold*")
       (goto-char (point-min))
       (let* ((source (org-element-context))
              (copy (org-element-copy source)))
         (list (org-element-parent copy)
               (org-element-parent source)))))))"##,
    );
}

// ── Parsers: affiliated keywords ─────────────────────────────────────

#[test]
fn upstream_org_element_affiliated_keywords() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Simple keyword.
     (with-temp-buffer
       (org-mode)
       (insert "#+NAME: para\nParagraph")
       (goto-char (point-min))
       (org-element-property :name (org-element-at-point)))
     ;; Begin position.
     (with-temp-buffer
       (org-mode)
       (insert "#+NAME: para\nParagraph")
       (goto-char (point-min))
       (org-element-property :begin (org-element-at-point)))
     ;; Multiple keywords.
     (with-temp-buffer
       (org-mode)
       (insert "#+ATTR_ASCII: line1\n#+ATTR_ASCII: line2\nParagraph")
       (goto-char (point-min))
       (org-element-property :attr_ascii (org-element-at-point)))
     ;; Parsed keyword.
     (with-temp-buffer
       (org-mode)
       (insert "#+CAPTION: caption\nParagraph")
       (goto-char (point-min))
       (car (org-element-property :caption (org-element-at-point))))
     ;; Dual keyword.
     (with-temp-buffer
       (org-mode)
       (insert "#+CAPTION[short]: long\nParagraph")
       (goto-char (point-min))
       (org-element-property :caption (org-element-at-point)))
     ;; Multiple captions.
     (with-temp-buffer
       (org-mode)
       (insert "#+CAPTION[s1]: l1\n#+CAPTION[s2]: l2\nParagraph")
       (goto-char (point-min))
       (org-element-property :caption (org-element-at-point)))
     ;; Orphaned keyword: type check.
     (with-temp-buffer
       (org-mode)
       (insert "- item\n  #+name: name\nSome paragraph")
       (goto-char (point-min))
       (search-forward "name")
       (org-element-type (org-element-at-point)))
     ;; Orphaned keyword: no name on paragraph.
     (with-temp-buffer
       (org-mode)
       (insert "- item\n  #+name: name\nSome paragraph")
       (goto-char (point-min))
       (search-forward "Some")
       (org-element-property :name (org-element-at-point)))
     ;; Comments cannot have affiliated keywords.
     (with-temp-buffer
       (org-mode)
       (insert "#+name: foo\n# bar")
       (goto-char (point-min))
       (search-forward "bar")
       (org-element-property :name (org-element-at-point))))))"##,
    );
}

// ── Parsers: babel call ──────────────────────────────────────────────

#[test]
fn upstream_org_element_babel_call() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard.
     (with-temp-buffer
       (org-mode)
       (insert "#+CALL: test()")
       (goto-char (point-min))
       (org-element-type (org-element-at-point)))
     ;; Ignore case.
     (with-temp-buffer
       (org-mode)
       (insert "#+call: test()")
       (goto-char (point-min))
       (org-element-type (org-element-at-point)))
     ;; Call name.
     (with-temp-buffer
       (org-mode)
       (insert "#+CALL: test()")
       (goto-char (point-min))
       (org-element-property :call (org-element-at-point)))
     ;; Inside header.
     (with-temp-buffer
       (org-mode)
       (insert "#+CALL: test[:results output]()")
       (goto-char (point-min))
       (org-element-property :inside-header (org-element-at-point)))
     ;; Arguments.
     (with-temp-buffer
       (org-mode)
       (insert "#+CALL: test(n=4)")
       (goto-char (point-min))
       (org-element-property :arguments (org-element-at-point)))
     ;; Nested arguments.
     (with-temp-buffer
       (org-mode)
       (insert "#+CALL: test(test())")
       (goto-char (point-min))
       (org-element-property :arguments (org-element-at-point)))
     ;; End header.
     (with-temp-buffer
       (org-mode)
       (insert "#+CALL: test() :results html")
       (goto-char (point-min))
       (org-element-property :end-header (org-element-at-point))))))"##,
    );
}

// ── Parsers: bold ────────────────────────────────────────────────────

#[test]
fn upstream_org_element_bold_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard bold.
     (with-temp-buffer
       (org-mode)
       (insert "*bold*")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'bold #'identity nil t)))
     ;; Multi-line markup.
     (with-temp-buffer
       (org-mode)
       (insert "*first line\nsecond line*")
       (goto-char (point-min))
       (org-element-contents
        (org-element-map (org-element-parse-buffer) 'bold #'identity nil t))))))"##,
    );
}

// ── Parsers: center block ────────────────────────────────────────────

#[test]
fn upstream_org_element_center_block_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard.
     (with-temp-buffer
       (org-mode)
       (insert "#+BEGIN_CENTER\nText\n#+END_CENTER")
       (goto-char (point-min))
       (org-element-map (org-element-parse-buffer) 'center-block 'identity))
     ;; Ignore case.
     (with-temp-buffer
       (org-mode)
       (insert "#+begin_center\nText\n#+end_center")
       (goto-char (point-min))
       (org-element-map (org-element-parse-buffer) 'center-block 'identity))
     ;; Ignore incomplete block.
     (with-temp-buffer
       (org-mode)
       (insert "#+BEGIN_CENTER")
       (goto-char (point-min))
       (org-element-map (org-element-parse-buffer) 'center-block 'identity nil t)))))"##,
    );
}

// ── Parsers: citation ────────────────────────────────────────────────

#[test]
fn upstream_org_element_citation_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (require 'oc)
  (let ((org-mode-hook nil))
    (list
     ;; Valid citation.
     (with-temp-buffer
       (org-mode)
       (insert "[cite:@key]")
       (goto-char (point-min))
       (org-element-type (org-element-context)))
     ;; Invalid: no @.
     (with-temp-buffer
       (org-mode)
       (insert "[cite:text]")
       (goto-char (point-min))
       (org-element-type (org-element-context)))
     ;; With style.
     (with-temp-buffer
       (org-mode)
       (insert "[cite/style:@key]")
       (goto-char (point-min))
       (org-element-type (org-element-context)))
     ;; Style value.
     (with-temp-buffer
       (org-mode)
       (insert "[cite/style:@key]")
       (goto-char (point-min))
       (org-element-property :style (org-element-context)))
     ;; Multi citations.
     (with-temp-buffer
       (org-mode)
       (insert "[cite:@a;@b;@c]")
       (goto-char (point-min))
       (org-element-type (org-element-context))))))"##,
    );
}

// ── Parsers: clock ───────────────────────────────────────────────────

#[test]
fn upstream_org_element_clock_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard clock.
     (with-temp-buffer
       (org-mode)
       (insert "CLOCK: [2023-10-13 Fri 14:40]--[2023-10-13 Fri 14:51] =>  0:11")
       (goto-char (point-min))
       (org-element-type (org-element-at-point)))
     ;; Clock value.
     (with-temp-buffer
       (org-mode)
       (insert "CLOCK: [2023-10-13 Fri 14:40]--[2023-10-13 Fri 14:51] =>  0:11")
       (goto-char (point-min))
       (org-element-property :value (org-element-at-point)))
     ;; Duration.
     (with-temp-buffer
       (org-mode)
       (insert "CLOCK: [2023-10-13 Fri 14:40]--[2023-10-13 Fri 14:51] =>  0:11")
       (goto-char (point-min))
       (org-element-property :duration (org-element-at-point))))))"##,
    );
}

// ── Parsers: comment ─────────────────────────────────────────────────

#[test]
fn upstream_org_element_comment_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard comment.
     (with-temp-buffer
       (org-mode)
       (insert "# This is a comment")
       (goto-char (point-min))
       (org-element-type (org-element-at-point)))
     ;; Comment block.
     (with-temp-buffer
       (org-mode)
       (insert "#+BEGIN_COMMENT\nBlock comment\n#+END_COMMENT")
       (goto-char (point-min))
       (org-element-type (org-element-at-point))))))"##,
    );
}

// ── Parsers: comment block ───────────────────────────────────────────

#[test]
fn upstream_org_element_comment_block_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard.
     (with-temp-buffer
       (org-mode)
       (insert "#+BEGIN_COMMENT\nSome comment\n#+END_COMMENT")
       (goto-char (point-min))
       (org-element-map (org-element-parse-buffer) 'comment-block 'identity))
     ;; Ignore case.
     (with-temp-buffer
       (org-mode)
       (insert "#+begin_comment\nSome comment\n#+end_comment")
       (goto-char (point-min))
       (org-element-map (org-element-parse-buffer) 'comment-block 'identity)))))"##,
    );
}

// ── Parsers: diary-sexp ──────────────────────────────────────────────

#[test]
fn upstream_org_element_diary_sexp_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "%%(diary-anniversary 10 31 2023)")
      (goto-char (point-min))
      (org-element-type (org-element-at-point)))))"##,
    );
}

// ── Parsers: entity ──────────────────────────────────────────────────

#[test]
fn upstream_org_element_entity_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard entity.
     (with-temp-buffer
       (org-mode)
       (insert "\\alpha")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'entity #'identity nil t)))
     ;; Entity name.
     (with-temp-buffer
       (org-mode)
       (insert "\\alpha")
       (goto-char (point-min))
       (org-element-property
        :name
        (org-element-map (org-element-parse-buffer) 'entity #'identity nil t))))))"##,
    );
}

// ── Parsers: example block ───────────────────────────────────────────

#[test]
fn upstream_org_element_example_block_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard.
     (with-temp-buffer
       (org-mode)
       (insert "#+BEGIN_EXAMPLE\nSome example\n#+END_EXAMPLE")
       (goto-char (point-min))
       (org-element-type (org-element-at-point)))
     ;; With switches.
     (with-temp-buffer
       (org-mode)
       (insert "#+BEGIN_EXAMPLE -n\nSome example\n#+END_EXAMPLE")
       (goto-char (point-min))
       (org-element-property :switches (org-element-at-point))))))"##,
    );
}

// ── Parsers: export block ────────────────────────────────────────────

#[test]
fn upstream_org_element_export_block_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard.
     (with-temp-buffer
       (org-mode)
       (insert "#+BEGIN_EXPORT html\n<p>Text</p>\n#+END_EXPORT")
       (goto-char (point-min))
       (org-element-type (org-element-at-point)))
     ;; Export type.
     (with-temp-buffer
       (org-mode)
       (insert "#+BEGIN_EXPORT html\n<p>Text</p>\n#+END_EXPORT")
       (goto-char (point-min))
       (org-element-property :type (org-element-at-point))))))"##,
    );
}

// ── Parsers: fixed-width ─────────────────────────────────────────────

#[test]
fn upstream_org_element_fixed_width_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert ": fixed width line")
      (goto-char (point-min))
      (org-element-type (org-element-at-point)))))"##,
    );
}

// ── Parsers: footnote reference ──────────────────────────────────────

#[test]
fn upstream_org_element_footnote_ref_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard footnote ref.
     (with-temp-buffer
       (org-mode)
       (insert "Text[fn:1]")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'footnote-reference
                         #'identity nil t)))
     ;; Inline footnote.
     (with-temp-buffer
       (org-mode)
       (insert "Text[fn:name:definition]")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'footnote-reference
                         #'identity nil t))))))"##,
    );
}

// ── Parsers: headline ────────────────────────────────────────────────

#[test]
fn upstream_org_element_headline_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard headline.
     (with-temp-buffer
       (org-mode)
       (insert "* Headline")
       (goto-char (point-min))
       (org-element-type (org-element-at-point)))
     ;; Level.
     (with-temp-buffer
       (org-mode)
       (insert "*** Deep headline")
       (goto-char (point-min))
       (org-element-property :level (org-element-at-point)))
     ;; TODO keyword.
     (with-temp-buffer
       (org-mode)
       (insert "* TODO Task")
       (goto-char (point-min))
       (org-element-property :todo-keyword (org-element-at-point)))
     ;; Tags.
     (with-temp-buffer
       (org-mode)
       (insert "* Headline :tag1:tag2:")
       (goto-char (point-min))
       (org-element-property :tags (org-element-at-point)))
     ;; Priority.
     (with-temp-buffer
       (org-mode)
       (insert "* [#A] Headline")
       (goto-char (point-min))
       (org-element-property :priority (org-element-at-point)))
     ;; Raw value.
     (with-temp-buffer
       (org-mode)
       (insert "* TODO [#A] Headline :tag:")
       (goto-char (point-min))
       (substring-no-properties
        (org-element-property :raw-value (org-element-at-point)))))))"##,
    );
}

// ── Parsers: horizontal rule ─────────────────────────────────────────

#[test]
fn upstream_org_element_horizontal_rule_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "-----")
      (goto-char (point-min))
      (org-element-type (org-element-at-point)))))"##,
    );
}

// ── Parsers: inline src block ────────────────────────────────────────

#[test]
fn upstream_org_element_inline_src_block_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard inline src.
     (with-temp-buffer
       (org-mode)
       (insert "src_emacs-lisp{(+ 1 2)}")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'inline-src-block
                         #'identity nil t)))
     ;; Language.
     (with-temp-buffer
       (org-mode)
       (insert "src_emacs-lisp{(+ 1 2)}")
       (goto-char (point-min))
       (org-element-property
        :language
        (org-element-map (org-element-parse-buffer) 'inline-src-block
                         #'identity nil t))))))"##,
    );
}

// ── Parsers: inlinetask ──────────────────────────────────────────────

#[test]
fn upstream_org_element_inlinetask_parser() {
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
     (with-temp-buffer
       (org-mode)
       (insert "**** Inline task\nBody\n**** END")
       (goto-char (point-min))
       (org-element-type (org-element-at-point)))
     ;; Level.
     (with-temp-buffer
       (org-mode)
       (insert "**** Inline task\nBody\n**** END")
       (goto-char (point-min))
       (org-element-property :level (org-element-at-point))))))"##,
    );
}

// ── Parsers: item ────────────────────────────────────────────────────

#[test]
fn upstream_org_element_item_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard item.
     (with-temp-buffer
       (org-mode)
       (insert "- Item")
       (goto-char (point-min))
       (org-element-type (org-element-at-point)))
     ;; Bullet type.
     (with-temp-buffer
       (org-mode)
       (insert "- Item")
       (goto-char (point-min))
       (org-element-property :bullet (org-element-at-point)))
     ;; Checkbox.
     (with-temp-buffer
       (org-mode)
       (insert "- [X] Checked item")
       (goto-char (point-min))
       (org-element-property :checkbox (org-element-at-point)))
     ;; Tag (description list).
     (with-temp-buffer
       (org-mode)
       (insert "- tag :: description")
       (goto-char (point-min))
       (org-element-property :tag (org-element-at-point))))))"##,
    );
}

// ── Parsers: keyword ─────────────────────────────────────────────────

#[test]
fn upstream_org_element_keyword_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard keyword.
     (with-temp-buffer
       (org-mode)
       (insert "#+TITLE: My Title")
       (goto-char (point-min))
       (org-element-type (org-element-at-point)))
     ;; Key.
     (with-temp-buffer
       (org-mode)
       (insert "#+TITLE: My Title")
       (goto-char (point-min))
       (org-element-property :key (org-element-at-point)))
     ;; Value.
     (with-temp-buffer
       (org-mode)
       (insert "#+TITLE: My Title")
       (goto-char (point-min))
       (org-element-property :value (org-element-at-point))))))"##,
    );
}

// ── Parsers: latex environment ───────────────────────────────────────

#[test]
fn upstream_org_element_latex_environment_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard LaTeX environment.
     (with-temp-buffer
       (org-mode)
       (insert "\\begin{equation}\nx^2 + y^2 = z^2\n\\end{equation}")
       (goto-char (point-min))
       (org-element-type (org-element-at-point)))
     ;; Environment type.
     (with-temp-buffer
       (org-mode)
       (insert "\\begin{equation}\nx^2 + y^2 = z^2\n\\end{equation}")
       (goto-char (point-min))
       (org-element-property :value (org-element-at-point))))))"##,
    );
}

// ── Parsers: latex fragment ──────────────────────────────────────────

#[test]
fn upstream_org_element_latex_fragment_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Inline latex.
     (with-temp-buffer
       (org-mode)
       (insert "$x^2$")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'latex-fragment
                         #'identity nil t)))
     ;; Display latex.
     (with-temp-buffer
       (org-mode)
       (insert "$$x^2$$")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'latex-fragment
                         #'identity nil t))))))"##,
    );
}

// ── Parsers: line break ──────────────────────────────────────────────

#[test]
fn upstream_org_element_line_break_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "line1\\\\\nline2")
      (goto-char (point-min))
      (org-element-type
       (org-element-map (org-element-parse-buffer) 'line-break
                        #'identity nil t)))))"##,
    );
}

// ── Parsers: link ────────────────────────────────────────────────────

#[test]
fn upstream_org_element_link_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard link.
     (with-temp-buffer
       (org-mode)
       (insert "https://example.org")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'link #'identity nil t)))
     ;; Explicit link.
     (with-temp-buffer
       (org-mode)
       (insert "[[https://example.org][desc]]")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'link #'identity nil t)))
     ;; Link type.
     (with-temp-buffer
       (org-mode)
       (insert "[[https://example.org][desc]]")
       (goto-char (point-min))
       (org-element-property
        :type
        (org-element-map (org-element-parse-buffer) 'link #'identity nil t)))
     ;; Link path.
     (with-temp-buffer
       (org-mode)
       (insert "[[https://example.org][desc]]")
       (goto-char (point-min))
       (org-element-property
        :path
        (org-element-map (org-element-parse-buffer) 'link #'identity nil t))))))"##,
    );
}

// ── Parsers: node property ───────────────────────────────────────────

#[test]
fn upstream_org_element_node_property_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "* H\n:PROPERTIES:\n:KEY: val\n:END:")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (np (car (org-element-map tree 'node-property #'identity))))
        (list (org-element-property :key np)
              (org-element-property :value np))))))"##,
    );
}

// ── Parsers: paragraph ───────────────────────────────────────────────

#[test]
fn upstream_org_element_paragraph_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "Simple paragraph.")
      (goto-char (point-min))
      (org-element-type (org-element-at-point)))))"##,
    );
}

// ── Parsers: planning ────────────────────────────────────────────────

#[test]
fn upstream_org_element_planning_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; DEADLINE.
     (with-temp-buffer
       (org-mode)
       (insert "* H\nDEADLINE: <2023-10-13 Fri>")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (planning (car (org-element-map tree 'planning #'identity))))
         (org-element-property :deadline planning)))
     ;; SCHEDULED.
     (with-temp-buffer
       (org-mode)
       (insert "* H\nSCHEDULED: <2023-10-13 Fri>")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (planning (car (org-element-map tree 'planning #'identity))))
         (org-element-property :scheduled planning))))))"##,
    );
}

// ── Parsers: property drawer ─────────────────────────────────────────

#[test]
fn upstream_org_element_property_drawer_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "* H\n:PROPERTIES:\n:KEY: val\n:END:")
      (goto-char (point-min))
      (org-element-type
       (org-element-map (org-element-parse-buffer) 'property-drawer
                        #'identity)))))"##,
    );
}

// ── Parsers: quote block ─────────────────────────────────────────────

#[test]
fn upstream_org_element_quote_block_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard.
     (with-temp-buffer
       (org-mode)
       (insert "#+BEGIN_QUOTE\nQuoted text\n#+END_QUOTE")
       (goto-char (point-min))
       (org-element-type (org-element-at-point)))
     ;; Ignore case.
     (with-temp-buffer
       (org-mode)
       (insert "#+begin_quote\nQuoted text\n#+end_quote")
       (goto-char (point-min))
       (org-element-type (org-element-at-point))))))"##,
    );
}

// ── Parsers: section ─────────────────────────────────────────────────

#[test]
fn upstream_org_element_section_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "* Headline\nBody text.")
      (goto-char (point-min))
      (org-element-type
       (org-element-map (org-element-parse-buffer) 'section #'identity)))))"##,
    );
}

// ── Parsers: special block ───────────────────────────────────────────

#[test]
fn upstream_org_element_special_block_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard special block.
     (with-temp-buffer
       (org-mode)
       (insert "#+BEGIN_someblock\nContent\n#+END_someblock")
       (goto-char (point-min))
       (org-element-type (org-element-at-point)))
     ;; Block type.
     (with-temp-buffer
       (org-mode)
       (insert "#+BEGIN_someblock\nContent\n#+END_someblock")
       (goto-char (point-min))
       (org-element-property :type (org-element-at-point))))))"##,
    );
}

// ── Parsers: src block ───────────────────────────────────────────────

#[test]
fn upstream_org_element_src_block_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard src block.
     (with-temp-buffer
       (org-mode)
       (insert "#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
       (goto-char (point-min))
       (org-element-type (org-element-at-point)))
     ;; Language.
     (with-temp-buffer
       (org-mode)
       (insert "#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
       (goto-char (point-min))
       (org-element-property :language (org-element-at-point)))
     ;; With switches.
     (with-temp-buffer
       (org-mode)
       (insert "#+BEGIN_SRC emacs-lisp -n\n(+ 1 2)\n#+END_SRC")
       (goto-char (point-min))
       (org-element-property :switches (org-element-at-point))))))"##,
    );
}

// ── Parsers: table ───────────────────────────────────────────────────

#[test]
fn upstream_org_element_table_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard table.
     (with-temp-buffer
       (org-mode)
       (insert "| a | b |\n|---|\n| 1 | 2 |")
       (goto-char (point-min))
       (org-element-type (org-element-at-point)))
     ;; Table type.
     (with-temp-buffer
       (org-mode)
       (insert "| a | b |\n|---|\n| 1 | 2 |")
       (goto-char (point-min))
       (org-element-property :type (org-element-at-point))))))"##,
    );
}

// ── Parsers: table cell ──────────────────────────────────────────────

#[test]
fn upstream_org_element_table_cell_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "| a | b |")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (cells (org-element-map tree 'table-cell #'identity)))
        (mapcar (lambda (c)
                  (substring-no-properties
                   (org-element-interpret-data c)))
                cells)))))"##,
    );
}

// ── Parsers: table row ───────────────────────────────────────────────

#[test]
fn upstream_org_element_table_row_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "| a | b |\n|---|\n| 1 | 2 |")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (rows (org-element-map tree 'table-row #'identity)))
        (mapcar (lambda (r) (org-element-property :type r)) rows)))))"##,
    );
}

// ── Parsers: timestamp ───────────────────────────────────────────────

#[test]
fn upstream_org_element_timestamp_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Active timestamp.
     (with-temp-buffer
       (org-mode)
       (insert "<2023-10-13 Fri>")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'timestamp #'identity nil t)))
     ;; Inactive timestamp.
     (with-temp-buffer
       (org-mode)
       (insert "[2023-10-13 Fri]")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'timestamp #'identity nil t)))
     ;; Timestamp type.
     (with-temp-buffer
       (org-mode)
       (insert "<2023-10-13 Fri>")
       (goto-char (point-min))
       (org-element-property
        :type
        (org-element-map (org-element-parse-buffer) 'timestamp #'identity nil t))))))"##,
    );
}

// ── Parsers: underline ───────────────────────────────────────────────

#[test]
fn upstream_org_element_underline_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "_underlined_")
      (goto-char (point-min))
      (org-element-type
       (org-element-map (org-element-parse-buffer) 'underline #'identity nil t)))))"##,
    );
}

// ── Parsers: verbatim ────────────────────────────────────────────────

#[test]
fn upstream_org_element_verbatim_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "=verbatim=")
      (goto-char (point-min))
      (org-element-type
       (org-element-map (org-element-parse-buffer) 'verbatim #'identity nil t)))))"##,
    );
}

// ── Parsers: verse block ─────────────────────────────────────────────

#[test]
fn upstream_org_element_verse_block_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "#+BEGIN_VERSE\nLine one\nLine two\n#+END_VERSE")
      (goto-char (point-min))
      (org-element-type (org-element-at-point)))))"##,
    );
}

// ── org-element-parse-buffer granularity ─────────────────────────────

#[test]
fn upstream_org_element_parse_buffer_granularity() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "* H1\nParagraph *bold* /italic/.\n* H2\n")
      (goto-char (point-min))
      (list
       ;; Default granularity: full parse
       (length (org-element-map (org-element-parse-buffer) t #'identity))
       ;; Element granularity: no objects
       (length (org-element-map (org-element-parse-buffer 'element) t #'identity))
       ;; Greater element granularity
       (length (org-element-map (org-element-parse-buffer 'greater-element) t #'identity))))))"##,
    );
}

// ── org-element-parse-buffer-as ──────────────────────────────────────

#[test]
fn upstream_org_element_parse_buffer_as() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "* H1\nBody\n* H2\nBody2\n")
      (goto-char (point-min))
      (let ((tree (org-element-parse-buffer)))
        (list
         (org-element-type tree)
         (length (org-element-contents tree)))))))"##,
    );
}

// ── org-element-swap-A-B ─────────────────────────────────────────────

#[test]
fn upstream_org_element_swap_a_b() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "* A\nBody A\n* B\nBody B\n")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (headlines (org-element-map tree 'headline #'identity)))
        (org-element-swap-A-B (nth 0 headlines) (nth 1 headlines))
        (buffer-substring-no-properties (point-min) (point-max))))))"##,
    );
}

// ── org-element-uniq ─────────────────────────────────────────────────

#[test]
fn upstream_org_element_uniq() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (let* ((el1 (org-element-create 'paragraph nil "p1"))
         (el2 (org-element-create 'paragraph nil "p2"))
         (list (list el1 el2 el1 el2 el1)))
    (length (org-element-uniq list))))"##,
    );
}

// ── org-element-property-raw setter ──────────────────────────────────

#[test]
fn upstream_org_element_property_raw_setter() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (let ((el (org-element-create 'dummy '(:foo 1))))
    (setf (org-element-property-raw :foo el) 42)
    (setf (org-element-property-raw :bar el) 'baz)
    (list (org-element-property-raw :foo el)
          (org-element-property-raw :bar el))))"##,
    );
}

// ── org-element-deferred-create ──────────────────────────────────────

#[test]
fn upstream_org_element_deferred_create() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (list
   ;; Deferred-p check
   (org-element-deferred-p (org-element-deferred-create t (lambda (_) 1)))
   (org-element-deferred-p '(dummy))
   ;; Force flag
   (org-element-deferred-force-p (org-element-deferred-create t (lambda (_) 1)))
   (org-element-deferred-force-p (org-element-deferred-create nil (lambda (_) 1)))
   ;; Function
    (functionp (org-element-deferred-get-function
               (org-element-deferred-create nil (lambda (_) 1))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Remaining parser tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn upstream_org_element_citation_reference_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (require 'oc)
  (let ((org-mode-hook nil))
    (list
     ;; Bare key.
     (with-temp-buffer
       (org-mode)
       (insert "[cite:@key]")
       (goto-char (point-min))
       (org-element-type
        (car (org-element-map (org-element-parse-buffer) 'citation-reference #'identity))))
     ;; Key with special chars.
     (with-temp-buffer
       (org-mode)
       (insert "[cite:@a:.#$%&-+?<>~/1]")
       (goto-char (point-min))
       (org-element-property
        :key
        (car (org-element-map (org-element-parse-buffer) 'citation-reference #'identity))))
     ;; Prefix and suffix.
     (with-temp-buffer
       (org-mode)
       (insert "[cite:pre @key post]")
       (goto-char (point-min))
       (let ((ref (car (org-element-map (org-element-parse-buffer) 'citation-reference #'identity))))
         (list (org-element-property :prefix ref)
               (org-element-property :suffix ref)))))))"##,
    );
}

#[test]
fn upstream_org_element_code_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard code.
     (with-temp-buffer
       (org-mode)
       (insert "~code~")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'code #'identity nil t)))
     ;; Multi-line.
     (with-temp-buffer
       (org-mode)
       (insert "~first line\nsecond line~")
       (goto-char (point-min))
       (org-element-property
        :value
        (org-element-map (org-element-parse-buffer) 'code #'identity nil t))))))"##,
    );
}

#[test]
fn upstream_org_element_drawer_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard drawer.
     (with-temp-buffer
       (org-mode)
       (insert ":TEST:\nText\n:END:")
       (goto-char (point-min))
       (org-element-map (org-element-parse-buffer) 'drawer 'identity))
     ;; Ignore incomplete.
     (with-temp-buffer
       (org-mode)
       (insert ":TEST:")
       (goto-char (point-min))
       (org-element-map (org-element-parse-buffer) 'drawer 'identity nil t)))))"##,
    );
}

#[test]
fn upstream_org_element_dynamic_block_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard dynamic block.
     (with-temp-buffer
       (org-mode)
       (insert "#+BEGIN: myblock :param1 val1\nText\n#+END:")
       (goto-char (point-min))
       (org-element-map (org-element-parse-buffer) 'dynamic-block 'identity))
     ;; Ignore case.
     (with-temp-buffer
       (org-mode)
       (insert "#+begin: myblock\nText\n#+end:")
       (goto-char (point-min))
       (org-element-map (org-element-parse-buffer) 'dynamic-block 'identity)))))"##,
    );
}

#[test]
fn upstream_org_element_export_snippet_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer
       (org-mode)
       (insert "@@html:<b>@@")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'export-snippet #'identity nil t)))
     (with-temp-buffer
       (org-mode)
       (insert "@@html:<b>@@")
       (goto-char (point-min))
       (org-element-property
        :back-end
        (org-element-map (org-element-parse-buffer) 'export-snippet #'identity nil t))))))"##,
    );
}

#[test]
fn upstream_org_element_footnote_definition_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard footnote definition.
     (with-temp-buffer
       (org-mode)
       (insert "[fn:1] Definition.")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'footnote-definition #'identity nil t)))
     ;; Label.
     (with-temp-buffer
       (org-mode)
       (insert "[fn:1] Definition.")
       (goto-char (point-min))
       (org-element-property
        :label
        (org-element-map (org-element-parse-buffer) 'footnote-definition #'identity nil t))))))"##,
    );
}

#[test]
fn upstream_org_element_headline_todo_keyword() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "* TODO Task\n* DONE Done\n* Normal\n* WAITING Wait")
      (goto-char (point-min))
      (mapcar (lambda (h) (org-element-property :todo-keyword h))
              (org-element-map (org-element-parse-buffer) 'headline #'identity)))))"##,
    );
}

#[test]
fn upstream_org_element_headline_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "* H\n:PROPERTIES:\n:Custom: val\n:END:")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (hl (car (org-element-map tree 'headline #'identity))))
        (org-element-property :CUSTOM_ID hl)))))"##,
    );
}

#[test]
fn upstream_org_element_inline_babel_call_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer
       (org-mode)
       (insert "call_test()")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'inline-babel-call #'identity nil t)))
     (with-temp-buffer
       (org-mode)
       (insert "call_test(x=2)")
       (goto-char (point-min))
       (org-element-property
        :arguments
        (org-element-map (org-element-parse-buffer) 'inline-babel-call #'identity nil t))))))"##,
    );
}

#[test]
fn upstream_org_element_italic_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer
       (org-mode)
       (insert "/italic/")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'italic #'identity nil t)))
     ;; Multi-line.
     (with-temp-buffer
       (org-mode)
       (insert "/first line\nsecond line/")
       (goto-char (point-min))
       (org-element-contents
        (org-element-map (org-element-parse-buffer) 'italic #'identity nil t))))))"##,
    );
}

#[test]
fn upstream_org_element_macro_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer
       (org-mode)
       (insert "{{{test}}}")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'macro #'identity nil t)))
     (with-temp-buffer
       (org-mode)
       (insert "{{{test(arg1,arg2)}}}")
       (goto-char (point-min))
       (org-element-property
        :value
        (org-element-map (org-element-parse-buffer) 'macro #'identity nil t))))))"##,
    );
}

#[test]
fn upstream_org_element_plain_list_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard list.
     (with-temp-buffer
       (org-mode)
       (insert "- item1\n- item2")
       (goto-char (point-min))
       (org-element-type (org-element-at-point)))
     ;; Ordered list.
     (with-temp-buffer
       (org-mode)
       (insert "1. item1\n2. item2")
       (goto-char (point-min))
       (org-element-property
        :type (org-element-at-point)))
     ;; Description list.
     (with-temp-buffer
       (org-mode)
       (insert "- tag :: desc")
       (goto-char (point-min))
       (org-element-type
        (org-element-map (org-element-parse-buffer) 'item #'identity nil t))))))"##,
    );
}

#[test]
fn upstream_org_element_radio_target_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Standard radio target.
     (with-temp-buffer
       (org-mode)
       (insert "<<<radio>>>")
       (goto-char (point-min))
       (org-element-type (org-element-context)))
     ;; With objects.
     (with-temp-buffer
       (org-mode)
       (insert "<<<radio \\alpha>>>")
       (goto-char (point-min))
       (org-element-type (org-element-context)))
     ;; Cannot begin with whitespace.
     (with-temp-buffer
       (org-mode)
       (insert "<<< radio>>>")
       (goto-char (point-min))
       (org-element-type (org-element-context)))
     ;; Cannot end with whitespace.
     (with-temp-buffer
       (org-mode)
       (insert "<<<radio >>>")
       (goto-char (point-min))
       (org-element-type (org-element-context))))))"##,
    );
}

#[test]
fn upstream_org_element_statistics_cookie_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; With numbers.
     (with-temp-buffer
       (org-mode)
       (insert "[1/2]")
       (goto-char (point-min))
       (org-element-map (org-element-parse-buffer) 'statistics-cookie 'identity))
     ;; With percents.
     (with-temp-buffer
       (org-mode)
       (insert "[33%]")
       (goto-char (point-min))
       (org-element-map (org-element-parse-buffer) 'statistics-cookie 'identity)))))"##,
    );
}

#[test]
fn upstream_org_element_strike_through_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer
       (org-mode)
       (insert "+strike-through+")
       (goto-char (point-min))
       (org-element-map (org-element-parse-buffer) 'strike-through #'identity))
     ;; Multi-line.
     (with-temp-buffer
       (org-mode)
       (insert "+first line\nsecond line+")
       (goto-char (point-min))
       (org-element-contents
        (org-element-map (org-element-parse-buffer) 'strike-through #'identity nil t))))))"##,
    );
}

#[test]
fn upstream_org_element_subscript_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Without braces.
     (with-temp-buffer
       (org-mode)
       (insert "a_b")
       (goto-char (point-min))
       (org-element-map (org-element-parse-buffer) 'subscript 'identity))
     ;; With braces.
     (with-temp-buffer
       (org-mode)
       (insert "a_{b}")
       (goto-char (point-min))
       (org-element-map (org-element-parse-buffer) 'subscript 'identity))
     ;; Multiple.
     (with-temp-buffer
       (org-mode)
       (insert "a_b and c_d")
       (goto-char (point-min))
       (length (org-element-map (org-element-parse-buffer) 'subscript 'identity))))))"##,
    );
}

#[test]
fn upstream_org_element_superscript_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Without braces.
     (with-temp-buffer
       (org-mode)
       (insert "a^b")
       (goto-char (point-min))
       (org-element-map (org-element-parse-buffer) 'superscript 'identity))
     ;; With braces.
     (with-temp-buffer
       (org-mode)
       (insert "a^{b}")
       (goto-char (point-min))
       (org-element-map (org-element-parse-buffer) 'superscript 'identity))
     ;; Multiple.
     (with-temp-buffer
       (org-mode)
       (insert "a^b and c^d")
       (goto-char (point-min))
       (length (org-element-map (org-element-parse-buffer) 'superscript 'identity))))))"##,
    );
}

#[test]
fn upstream_org_element_target_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "<<target>>")
      (goto-char (point-min))
      (org-element-type
       (org-element-map (org-element-parse-buffer) 'target #'identity nil t)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Interpreter round-trip tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn upstream_org_element_interpret_data_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil)
        (org-test-parse-and-interpret
         (lambda (text)
           (with-temp-buffer
             (org-mode)
             (insert text)
             (org-element-interpret-data (org-element-parse-buffer))))))
    (list
     ;; Bold
     (funcall org-test-parse-and-interpret "*text*")
     ;; Italic
     (funcall org-test-parse-and-interpret "/text/")
     ;; Code
     (funcall org-test-parse-and-interpret "~text~")
     ;; Verbatim
     (funcall org-test-parse-and-interpret "=text=")
     ;; Underline
     (funcall org-test-parse-and-interpret "_text_")
     ;; Strike-through
     (funcall org-test-parse-and-interpret "+target+"))))"##,
    );
}

#[test]
fn upstream_org_element_interpret_markup_objects() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil)
        (org-test-parse-and-interpret
         (lambda (text)
           (with-temp-buffer
             (org-mode)
             (insert text)
             (org-element-interpret-data (org-element-parse-buffer))))))
    (list
     ;; Subscript
     (funcall org-test-parse-and-interpret "a_b")
     (funcall org-test-parse-and-interpret "a_{b}")
     ;; Superscript
     (funcall org-test-parse-and-interpret "a^b")
     (funcall org-test-parse-and-interpret "a^{b}")
     ;; Entity
     (funcall org-test-parse-and-interpret "\\alpha text")
     (funcall org-test-parse-and-interpret "\\alpha{}text"))))"##,
    );
}

#[test]
fn upstream_org_element_interpret_links() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil)
        (org-test-parse-and-interpret
         (lambda (text)
           (with-temp-buffer
             (org-mode)
             (insert text)
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

#[test]
fn upstream_org_element_interpret_footnotes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil)
        (org-test-parse-and-interpret
         (lambda (text)
           (with-temp-buffer
             (org-mode)
             (insert text)
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

#[test]
fn upstream_org_element_interpret_blocks() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil)
        (org-src-preserve-indentation t)
        (org-test-parse-and-interpret
         (lambda (text)
           (with-temp-buffer
             (org-mode)
             (insert text)
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

#[test]
fn upstream_org_element_interpret_src_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil)
        (org-edit-src-content-indentation 2)
        (org-src-preserve-indentation nil))
    (let ((org-test-parse-and-interpret
           (lambda (text)
             (with-temp-buffer
               (org-mode)
               (insert text)
               (org-element-interpret-data (org-element-parse-buffer))))))
      (list
       ;; With arguments.
       (funcall org-test-parse-and-interpret
                "#+BEGIN_SRC emacs-lisp :results silent\n(+ 1 1)\n#+END_SRC")
       ;; With switches.
       (funcall org-test-parse-and-interpret
                "#+BEGIN_SRC emacs-lisp -n -k\n(+ 1 1)\n#+END_SRC")))))"##,
    );
}

#[test]
fn upstream_org_element_interpret_inline_objects() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil)
        (org-test-parse-and-interpret
         (lambda (text)
           (with-temp-buffer
             (org-mode)
             (insert text)
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
     (funcall org-test-parse-and-interpret "First line \\\\ \nSecond line")
     ;; Target.
     (funcall org-test-parse-and-interpret "<<target>>")
     ;; Radio target.
     (funcall org-test-parse-and-interpret "<<<some text>>>")
     ;; Macro.
     (funcall org-test-parse-and-interpret "{{{test}}}")
     (funcall org-test-parse-and-interpret "{{{test(arg1,arg2)}}}"))))"##,
    );
}

#[test]
fn upstream_org_element_interpret_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil)
        (org-test-parse-and-interpret
         (lambda (text)
           (with-temp-buffer
             (org-mode)
             (insert text)
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

#[test]
fn upstream_org_element_interpret_timestamp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil)
        (org-test-parse-and-interpret
         (lambda (text)
           (with-temp-buffer
             (org-mode)
             (insert text)
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

#[test]
fn upstream_org_element_interpret_keyword_and_comment() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil)
        (org-test-parse-and-interpret
         (lambda (text)
           (with-temp-buffer
             (org-mode)
             (insert text)
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

#[test]
fn upstream_org_element_interpret_citation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (require 'oc)
  (let ((org-mode-hook nil)
        (org-test-parse-and-interpret
         (lambda (text)
           (with-temp-buffer
             (org-mode)
             (insert text)
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
// Granularity, parent, normalize-contents, at-point extended
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn upstream_org_element_granularity() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "* Head 1\n** Head 2\n#+BEGIN_CENTER\nCentered paragraph.\n#+END_CENTER\nParagraph \\alpha.")
      (goto-char (point-min))
      (list
       ;; headline granularity
       (let ((tree (org-element-parse-buffer 'headline)))
         (list (length (org-element-map tree 'headline 'identity))
               (org-element-map tree 'paragraph 'identity)))
       ;; greater-element granularity
       (let ((tree (org-element-parse-buffer 'greater-element)))
         (list (length (org-element-map tree 'center-block 'identity))
               (length (org-element-map tree 'paragraph 'identity))
               (org-element-map tree 'entity 'identity)))
       ;; element granularity
       (let ((tree (org-element-parse-buffer 'element)))
         (list (length (org-element-map tree 'paragraph 'identity))
               (org-element-map tree 'entity 'identity)))
       ;; object granularity
       (let ((tree (org-element-parse-buffer 'object)))
         (length (org-element-map tree 'entity 'identity)))))))"##,
    );
}

#[test]
fn upstream_org_element_parent_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; Elements.
     (with-temp-buffer
       (org-mode)
       (insert "#+BEGIN_CENTER\nText\n#+END_CENTER")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (parent (org-element-property
                       :parent
                       (org-element-map tree 'paragraph 'identity nil t))))
         (and parent
              (eq (org-element-map tree 'center-block 'identity nil t) parent))))
     ;; Objects.
     (with-temp-buffer
       (org-mode)
       (insert "a_{/b/}")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (parent (org-element-property
                       :parent
                       (org-element-map tree 'italic 'identity nil t))))
         (and parent
              (eq parent (org-element-map tree 'subscript 'identity nil t)))))
     ;; Secondary strings.
     (with-temp-buffer
       (org-mode)
       (insert "* /italic/")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (parent (org-element-property
                       :parent (org-element-map tree 'italic 'identity nil t))))
         (and parent
              (equal parent (org-element-map tree 'headline 'identity nil t))))))))"##,
    );
}

#[test]
fn upstream_org_element_normalize_contents() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (list
   ;; Remove common indentation.
   (org-element-normalize-contents
    '(paragraph nil "  Two spaces\n   Three spaces"))
   ;; No common indentation when first line has none.
   (org-element-normalize-contents
    '(paragraph nil "  Two spaces\nNo space"))
   ;; Ignore blank lines.
   (org-element-normalize-contents
    '(paragraph nil "  Two spaces\n\n \n  Two spaces"))
   ;; With argument: ignore first line indentation.
   (org-element-normalize-contents
    '(paragraph nil "No space\n  Two spaces\n   Three spaces") t)
   ;; Line break corner case.
   (org-element-normalize-contents
    '(paragraph nil " 1 space" (line-break) "  2 spaces"))))"##,
    );
}

#[test]
fn upstream_org_element_at_point_extended() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; At blank line below headline, return that headline.
     (with-temp-buffer
       (org-mode)
       (insert "* H1\n  \n* H2\n")
       (goto-char (point-min))
       (forward-line)
       (org-element-property :title (org-element-at-point)))
     ;; At beginning of table, return table.
     (with-temp-buffer
       (org-mode)
       (insert "| a | b |")
       (goto-char (point-min))
       (org-element-type (org-element-at-point)))
     ;; At beginning of list, return plain-list.
     (with-temp-buffer
       (org-mode)
       (insert "- item")
       (goto-char (point-min))
       (org-element-type (org-element-at-point)))
     ;; At closing line of greater element.
     (with-temp-buffer
       (org-mode)
       (insert "#+BEGIN_CENTER\nParagraph\n#+END_CENTER")
       (goto-char (point-min))
       (forward-line 2)
       (org-element-type (org-element-at-point)))
     ;; At blank line between items.
     (with-temp-buffer
       (org-mode)
       (insert "- Para1\n\n- Para2")
       (goto-char (point-min))
       (forward-line)
       (org-element-type (org-element-at-point))))))"##,
    );
}

#[test]
fn upstream_org_element_lineage() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "* H1\nParagraph with *bold* text.")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (bold (car (org-element-map tree 'bold #'identity))))
        (list
         ;; Full lineage.
         (mapcar #'org-element-type (org-element-lineage bold))
         ;; With self.
         (mapcar #'org-element-type (org-element-lineage bold nil t))
         ;; Filtered.
         (mapcar #'org-element-type (org-element-lineage bold 'headline)))))))"##,
    );
}

#[test]
fn upstream_org_element_secondary_string_parsing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     ;; With headline granularity, title is a string.
     (with-temp-buffer
       (org-mode)
       (insert "* Headline")
       (goto-char (point-min))
       (stringp
        (org-element-property
         :title
         (org-element-map (org-element-parse-buffer 'headline) 'headline
                          #'identity nil t))))
     ;; With default granularity, title is a list.
     (with-temp-buffer
       (org-mode)
       (insert "* Headline")
       (goto-char (point-min))
       (listp
        (org-element-property
         :title
         (org-element-map (org-element-parse-buffer) 'headline
                          #'identity nil t))))
     ;; org-element-at-point never parses secondary strings.
     (with-temp-buffer
       (org-mode)
       (insert "* Headline")
       (goto-char (point-min))
       (listp (org-element-property :title (org-element-at-point)))))))"##,
    );
}

#[test]
fn upstream_org_element_swap_a_b_extended() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer
      (org-mode)
      (insert "* A\nBody A\n* B\nBody B\n* C\nBody C\n")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (headlines (org-element-map tree 'headline #'identity)))
        (org-element-swap-A-B (nth 0 headlines) (nth 1 headlines))
        (mapcar (lambda (h) (org-element-property :raw-value h))
                (org-element-map tree 'headline #'identity))))))"##,
    );
}

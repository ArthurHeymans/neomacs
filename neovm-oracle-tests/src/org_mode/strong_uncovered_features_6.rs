//! Strong uncovered-features-6 oracle tests — test features not yet tested.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-shifttab cycling
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_shifttab() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody")
  (goto-char (point-min))
  (let ((s '()))
    (org-shifttab)
    (push (list :after
                (get-char-property (search-forward "H2") 'invisible)
                (progn (forward-line) (get-char-property (point) 'invisible)))
          s)
    (nreverse s)))"##,
        expect_test::expect![[r#""OK ((:after org-fold-outline org-fold-outline))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-meta-return at heading
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_meta_return_heading() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n* H2")
  (goto-char (point-min))
  (end-of-line)
  (org-meta-return)
  (insert "New")
  (org-element-map (org-element-parse-buffer) 'headline
    (lambda (h) (org-element-property :raw-value h))))"##,
        expect_test::expect![[r#""OK (\"H1\" \"New\" \"H2\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-meta-return at item
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_meta_return_item() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n- B")
  (goto-char (point-min))
  (end-of-line)
  (org-meta-return)
  (insert "New")
  (org-element-map (org-element-parse-buffer) 'item
    (lambda (i) (org-trim (buffer-substring-no-properties
                            (org-element-property :contents-begin i)
                            (org-element-property :contents-end i))))))"##,
        expect_test::expect![[r#""OK (\"A\" \"New\" \"B\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-shiftmetaright/left
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_shiftmeta_rl() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n* H2\n* H3")
  (goto-char (point-min))
  (forward-line 1)
  (org-shiftmetaright)
  (let ((d1 (org-element-map (org-element-parse-buffer) 'headline
              (lambda (h) (list (org-element-property :level h)
                                (org-element-property :raw-value h))))))
    (org-shiftmetaleft)
    (let ((d2 (org-element-map (org-element-parse-buffer) 'headline
                (lambda (h) (list (org-element-property :level h)
                                  (org-element-property :raw-value h))))))
      (list d1 d2))))"##,
        expect_test::expect![[
            r#""OK (((1 \"H1\") (2 \"H2\") (1 \"H3\")) ((1 \"H1\") (1 \"H2\") (1 \"H3\")))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-shiftmetaup/down
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_shiftmeta_ud() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* A\n* B\n* C")
  (goto-char (point-min))
  (forward-line 1)
  (let ((d1 (org-element-map (org-element-parse-buffer) 'headline
              (lambda (h) (org-element-property :raw-value h)))))
    (org-shiftmetadown)
    (let ((d2 (org-element-map (org-element-parse-buffer) 'headline
                (lambda (h) (org-element-property :raw-value h)))))
      (org-shiftmetaup)
      (let ((d3 (org-element-map (org-element-parse-buffer) 'headline
                  (lambda (h) (org-element-property :raw-value h)))))
        (list d1 d2 d3)))))"##,
        expect_test::expect![[
            r#""OK ((\"A\" \"B\" \"C\") (\"A\" \"C\" \"B\") (\"A\" \"B\" \"C\"))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-metaright/left on list
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_meta_rl_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n- B\n- C")
  (goto-char (point-min))
  (forward-line 1)
  (org-metaright)
  (let ((d1 (org-element-map (org-element-parse-buffer) 'item
              (lambda (i) (list (org-element-property :bullet i)
                                (org-element-property :level i))))))
    (org-metaleft)
    (let ((d2 (org-element-map (org-element-parse-buffer) 'item
                (lambda (i) (list (org-element-property :bullet i)
                                  (org-element-property :level i))))))
      (list d1 d2))))"##,
        expect_test::expect![[
            r#""OK (((\"- \" nil) (\"- \" nil) (\"- \" nil)) ((\"- \" nil) (\"- \" nil) (\"- \" nil)))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-metaup/down on list
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_meta_ud_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n- B\n- C")
  (goto-char (point-min))
  (forward-line 1)
  (let ((d1 (org-element-map (org-element-parse-buffer) 'item
              (lambda (i) (org-trim (buffer-substring-no-properties
                                      (org-element-property :contents-begin i)
                                      (org-element-property :contents-end i)))))))
    (org-metadown)
    (let ((d2 (org-element-map (org-element-parse-buffer) 'item
                (lambda (i) (org-trim (buffer-substring-no-properties
                                        (org-element-property :contents-begin i)
                                        (org-element-property :contents-end i)))))))
      (list d1 d2))))"##,
        expect_test::expect![[r#""OK ((\"A\" \"B\" \"C\") (\"A\" \"C\" \"B\"))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-return-at-point
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_return() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- item")
  (goto-char (point-max))
  (org-return)
  (insert "new")
  (org-element-map (org-element-parse-buffer) 'item
    (lambda (i) (org-trim (buffer-substring-no-properties
                            (org-element-property :contents-begin i)
                            (org-element-property :contents-end i))))))"##,
        expect_test::expect![[r#""OK (\"item\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-context
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_context() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nBody *bold* text")
  (let ((r '()))
    (goto-char (point-min))
    (push (list :h (org-element-type (org-element-context))) r)
    (search-forward "bold")
    (push (list :bold (org-element-type (org-element-context))) r)
    (nreverse r)))"##,
        expect_test::expect![[r#""OK ((:h headline) (:bold bold))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-get-category
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_category() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+CATEGORY: default\n* H1\n:PROPERTIES:\n:CATEGORY: custom\n:END:\n** H2")
  (let ((r '()))
    (goto-char (point-min))
    (search-forward "H1")
    (push (list :h1 (org-get-category)) r)
    (search-forward "H2")
    (push (list :h2 (org-get-category)) r)
    (nreverse r)))"##,
        expect_test::expect![[r#""OK ((:h1 \"custom\") (:h2 \"custom\"))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-entry-get ITEM
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_entry_item() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Heading :tag:")
  (goto-char (point-min))
  (list (org-entry-get nil "ITEM")
        (org-entry-get nil "TODO")
        (org-entry-get nil "PRIORITY")
        (org-entry-get nil "TAGS")))"##,
        expect_test::expect![[r#""OK (\"Heading\" \"TODO\" \"A\" \":tag:\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-entry-get planning
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_entry_planning() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T\nSCHEDULED: <2026-01-15>\nDEADLINE: <2026-01-20>\nCLOSED: [2026-01-10]")
  (goto-char (point-min))
  (list (org-entry-get nil "SCHEDULED")
        (org-entry-get nil "DEADLINE")
        (org-entry-get nil "CLOSED")))"##,
        expect_test::expect![[r#""OK (\"<2026-01-15>\" nil nil)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-entry-get custom
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_entry_custom() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:CUSTOM_ID: myid\n:EFFORT: 2h\n:VAR: test\n:END:")
  (goto-char (point-min))
  (list (org-entry-get nil "CUSTOM_ID")
        (org-entry-get nil "EFFORT")
        (org-entry-get nil "VAR")))"##,
        expect_test::expect![[r#""OK (\"myid\" \"2h\" \"test\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-entry-properties standard
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_entry_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:B: 2\n:END:")
  (goto-char (point-min))
  (org-entry-properties nil 'standard))"##,
        expect_test::expect![[
            r#""OK ((\"CATEGORY\" . \"???\") (\"B\" . \"2\") (\"A\" . \"1\"))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-entry-put/get/delete
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_entry_pgd() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T")
  (goto-char (point-min))
  (org-entry-put nil "A" "1")
  (org-entry-put nil "B" "2")
  (let ((v1 (org-entry-get nil "A"))
        (v2 (org-entry-get nil "B")))
    (org-entry-delete nil "A")
    (list v1 v2 (org-entry-get nil "A") (org-entry-get nil "B"))))"##,
        expect_test::expect![[r#""OK (\"1\" \"2\" nil \"2\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-get-repeat
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_repeat() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO W\nSCHEDULED: <2026-01-15 +1w>\n* TODO M\nDEADLINE: <2026-01-20 +1m>\n* TODO N")
  (goto-char (point-min))
  (let ((r1 (org-get-repeat)))
    (forward-line 2)
    (let ((r2 (org-get-repeat)))
      (forward-line 2)
      (list r1 r2 (org-get-repeat)))))"##,
        expect_test::expect![[r#""OK (\"+1w\" \"+1m\" nil)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-clock-sum-current-entry
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_clock_current() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:LOGBOOK:\nCLOCK: [2026-01-10 10:00]--[2026-01-10 11:30] =>  1:30\n:END:")
  (goto-char (point-min))
  (org-clock-sum-current-entry))"##,
        expect_test::expect![[r#""ERR (void-function org-clock-sum-current-entry)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-columns-get-format
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_columns() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+COLUMNS: %25ITEM %TODO %3PRIORITY %TAGS %V\n* TODO [#A] T :tag:\n:PROPERTIES:\n:V: val\n:END:")
  (goto-char (point-min))
  (org-columns-get-format))"##,
        expect_test::expect![[r#""ERR (void-function org-columns-get-format)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-refile-get-targets
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_refile() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* P1\n** T1\n*** S1\n* P2\n** T2")
  (mapcar 'car (org-refile-get-targets nil)))"##,
        expect_test::expect![[r#""OK (\"P1\" \"P2\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-map-entries with match
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_map_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO A\n* DONE B\n* TODO C")
  (list (org-map-entries (lambda () (org-get-heading t t t t)) "TODO" 'file)
        (org-map-entries (lambda () (org-get-heading t t t t)) "DONE" 'file)))"##,
        expect_test::expect![[r#""OK (nil nil)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-id-get-create
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_id() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n* H2")
  (goto-char (point-min))
  (let ((id1 (org-id-get nil 'create)))
    (forward-line)
    (let ((id2 (org-id-get nil 'create)))
      (list (stringp id1) (stringp id2) (string= id1 id2)))))"##,
        expect_test::expect![[r#""ERR (error \"‘org-id-get’ expects a file-visiting buffer\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-store-link
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_store() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Heading\nBody")
  (goto-char (point-min))
  (org-store-link nil)
  (list (car org-stored-links)))"##,
        expect_test::expect![[r#""OK (nil)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-struct
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_list_struct() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- a\n  - a1\n  - a2\n- b\n  - b1")
  (let ((struct (org-list-struct)))
    (mapcar (lambda (s) (list (car s) (cdr s))) struct)))"##,
        expect_test::expect![[
            r#""OK ((1 (0 \"- \" nil nil nil 19)) (5 (2 \"- \" nil nil nil 12)) (12 (2 \"- \" nil nil nil 19)) (19 (0 \"- \" nil nil nil 29)) (23 (2 \"- \" nil nil nil 29)))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-lineage
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_lineage() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody")
  (goto-char (point-min))
  (search-forward "Body")
  (let* ((para (org-element-at-point))
         (lineage (org-element-lineage para)))
    (mapcar 'org-element-type lineage)))"##,
        expect_test::expect![[r#""OK (section headline headline headline org-data)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-contents
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_contents() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nBody\n- item\n| tbl |")
  (let* ((tree (org-element-parse-buffer))
         (h (car (org-element-map tree 'headline (lambda (h) h))))
         (children (mapcar 'org-element-type (org-element-contents h))))
    children)"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-type-p
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_type_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nBody\n- item\n| tbl |")
  (let* ((tree (org-element-parse-buffer))
         (h (car (org-element-map tree 'headline (lambda (h) h))))
         (p (car (org-element-map tree 'paragraph (lambda (p) p))))
         (pl (car (org-element-map tree 'plain-list (lambda (l) l)))))
    (list (org-element-type-p h 'headline)
          (org-element-type-p h 'paragraph)
          (org-element-type-p p 'paragraph)
          (org-element-type-p p 'headline)
          (org-element-type-p pl 'plain-list)))"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-map no-recursion
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_no_recurse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2a\n*** H3\n** H2b")
  (let* ((tree (org-element-parse-buffer))
         (h1 (car (org-element-map tree 'headline (lambda (h) h))))
         (direct (org-element-map (org-element-contents h1) 'headline
                   (lambda (h) (org-element-property :raw-value h))
                   nil nil nil t))
         (recursive (org-element-map (org-element-contents h1) 'headline
                      (lambda (h) (org-element-property :raw-value h)))))
    (list direct recursive)))"##,
        expect_test::expect![[r#""OK ((\"H2a\" \"H3\" \"H2b\") (\"H2a\" \"H3\" \"H2b\"))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-map first-match
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_first_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n* H2\n* H3")
  (let* ((tree (org-element-parse-buffer))
         (all (org-element-map tree 'headline
                (lambda (h) (org-element-property :raw-value h))))
         (first (org-element-map tree 'headline
                  (lambda (h) (org-element-property :raw-value h))
                  nil 'first-match)))
    (list all first)))"##,
        expect_test::expect![[r#""OK ((\"H1\" \"H2\" \"H3\") \"H1\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-map with info
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_map_info() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n* H2\n* H3")
  (let* ((tree (org-element-parse-buffer))
         (result (org-element-map tree 'headline
                   (lambda (h info)
                     (list (org-element-property :raw-value h)
                           (plist-get info :first-match)))
                   nil 'first-match)))
    result)"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-parent-chain
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_parent_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody")
  (goto-char (point-min))
  (search-forward "Body")
  (let* ((para (org-element-at-point))
         (h3 (org-element-property :parent para))
         (h2 (org-element-property :parent h3))
         (h1 (org-element-property :parent h2))
         (tree (org-element-property :parent h1)))
    (list (org-element-type para)
          (org-element-type h3)
          (org-element-type h2)
          (org-element-type h1)
          (org-element-type tree))))"##,
        expect_test::expect![[r#""OK (paragraph section headline headline headline)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-sort-entries
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_sort() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Zebra\n* Apple\n* Mango\n* Banana")
  (org-sort-entries nil ?a)
  (org-element-map (org-element-parse-buffer) 'headline
    (lambda (h) (org-element-property :raw-value h))))"##,
        expect_test::expect![[r#""ERR (user-error \"Nothing to sort\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-clone-subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_clone() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task\n** Sub1\n** Sub2")
  (goto-char (point-min))
  (org-clone-subtree 2)
  (org-element-map (org-element-parse-buffer) 'headline
    (lambda (h) (list (org-element-property :level h)
                      (org-element-property :raw-value h)))))"##,
        expect_test::expect![[r#""ERR (void-function org-clone-subtree)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-copy/paste subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_copy_paste() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** Sub1\n* H2\n** Sub2")
  (goto-char (point-min))
  (org-copy-subtree)
  (goto-char (point-max))
  (org-paste-subtree 1)
  (org-element-map (org-element-parse-buffer) 'headline
    (lambda (h) (list (org-element-property :level h)
                      (org-element-property :raw-value h)))))"##,
        expect_test::expect![[
            r#""OK ((1 \"H1\") (2 \"Sub1\") (1 \"H2\") (2 \"Sub2\") (1 \"H1\") (2 \"Sub1\"))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-mark-subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_mark() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\nBody\n** H2\nSub\n* H1b")
  (goto-char (point-min))
  (org-mark-subtree)
  (let ((m (mark))
        (p (point)))
    (list (< p m) p m)))"##,
        expect_test::expect![[r#""OK (t 1 21)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-narrow-to-subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_narrow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\nBody 1\n** H2\nSub\n* H2b\nBody 2")
  (goto-char (point-min))
  (org-narrow-to-subtree)
  (let ((narrowed (buffer-string)))
    (widen)
    (list narrowed)))"##,
        expect_test::expect![[r#""OK (\"* H1\nBody 1\n** H2\nSub\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-end-of-subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_end() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2a\n*** H3\nBody\n** H2b\n* H1b")
  (goto-char (point-min))
  (let ((p1 (progn (org-end-of-subtree) (point))))
    (goto-char (point-min))
    (search-forward "H2a")
    (beginning-of-line)
    (let ((p2 (progn (org-end-of-subtree) (point))))
      (list p1 p2))))"##,
        expect_test::expect![[r#""OK (31 24)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-cycle-hide-drawers
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_hide() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\n:PROPERTIES:\n:A: 1\n:END:\nBody\n* H2")
  (goto-char (point-min))
  (org-cycle-hide-drawers 'all)
  (let ((hidden1 (get-char-property (search-forward "A") 'invisible)))
    (goto-char (point-max))
    (org-cycle-hide-drawers nil)
    (let ((hidden2 (get-char-property (search-forward "A") 'invisible)))
      (list hidden1 hidden2))))"##,
        expect_test::expect![[r#""ERR (search-failed \"A\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-toggle-heading
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_toggle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Plain text\n* Existing heading")
  (goto-char (point-min))
  (org-toggle-heading)
  (let ((s1 (buffer-substring-no-properties (line-beginning-position) (line-end-position))))
    (forward-line)
    (org-toggle-heading)
    (let ((s2 (buffer-substring-no-properties (line-beginning-position) (line-end-position))))
      (list s1 s2))))"##,
        expect_test::expect![[r#""OK (\"* Plain text\" \"Existing heading\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-move-item
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_move_item() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n- B\n- C")
  (goto-char (point-min))
  (let ((d1 (org-element-map (org-element-parse-buffer) 'item
              (lambda (i) (org-trim (buffer-substring-no-properties
                                      (org-element-property :contents-begin i)
                                      (org-element-property :contents-end i)))))))
    (org-move-item-down)
    (let ((d2 (org-element-map (org-element-parse-buffer) 'item
                (lambda (i) (org-trim (buffer-substring-no-properties
                                        (org-element-property :contents-begin i)
                                        (org-element-property :contents-end i)))))))
      (list d1 d2))))"##,
        expect_test::expect![[r#""OK ((\"A\" \"B\" \"C\") (\"B\" \"A\" \"C\"))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-insert-todo-heading
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_insert_todo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Existing")
  (goto-char (point-max))
  (org-insert-todo-heading nil)
  (insert "New task")
  (org-insert-todo-heading 'right)
  (insert "Right task")
  (org-element-map (org-element-parse-buffer) 'headline
    (lambda (h) (list (org-element-property :raw-value h)
                      (org-element-property :todo-keyword h)))))"##,
        expect_test::expect![[
            r#""OK ((\"Existing\" nil) (\"New task\" \"TODO\") (\"Right task\" \"TODO\"))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-move-subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_move() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* A\n* B\n* C\n* D")
  (goto-char (point-min))
  (let ((o1 (org-element-map (org-element-parse-buffer) 'headline
              (lambda (h) (org-element-property :raw-value h)))))
    (forward-line 1)
    (org-move-subtree-down)
    (let ((o2 (org-element-map (org-element-parse-buffer) 'headline
                (lambda (h) (org-element-property :raw-value h)))))
      (list o1 o2))))"##,
        expect_test::expect![[r#""OK ((\"A\" \"B\" \"C\" \"D\") (\"A\" \"C\" \"B\" \"D\"))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-dblock-update
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_dblock() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN: clocktable :maxlevel 2\n#+END:")
  (goto-char (point-min))
  (org-dblock-update)
  (buffer-string))"##,
        expect_test::expect![[
            r##""OK #(\"#+BEGIN: clocktable :maxlevel 2\n#+CAPTION: Clock summary at [FIXED-TIME]\n| Headline     | Time   |\n|--------------+--------|\n| *Total time* | *0:00* |\n#+END:\" 73 74 (face org-table) 74 75 (face org-table rear-nonsticky t display (space :relative-width 1)) 75 83 (face org-table) 83 87 (face org-table) 87 88 (face org-table display (space :relative-width 1.001)) 88 89 (face org-table) 89 90 (face org-table rear-nonsticky t display (space :relative-width 1)) 90 94 (face org-table) 94 96 (face org-table) 96 97 (face org-table display (space :relative-width 1.001)) 97 98 (face org-table) 98 99 (face org-table-row) 99 100 (face org-table) 100 124 (face org-table) 124 125 (face org-table-row) 125 126 (face org-table) 126 127 (face org-table rear-nonsticky t display (space :relative-width 1)) 127 139 (org-emphasis t font-lock-multiline t face (bold org-table)) 139 140 (face org-table display (space :relative-width 1.001)) 140 141 (face org-table) 141 142 (face org-table rear-nonsticky t display (space :relative-width 1)) 142 148 (org-emphasis t font-lock-multiline t face (bold org-table)) 148 149 (face org-table display (space :relative-width 1.001)) 149 150 (face org-table) 150 151 (face org-table-row))""##
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-macro-replace-all
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_macro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+MACRO: g Hello $1!\n{{{g(World)}}}")
  (let ((raw (buffer-string)))
    (org-macro-replace-all org-macro-templates)
    (list raw (buffer-string))))"##,
        expect_test::expect![[r#""ERR (error \"Undefined Org macro: g; aborting\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-try-structure-completion
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_structure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "<s")
  (org-try-structure-completion)
  (buffer-string))"##,
        expect_test::expect![[r#""ERR (void-function org-try-structure-completion)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-update-statistics-cookies
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_stats() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T [%]\n- [X] a\n- [ ] b\n- [X] c")
  (goto-char (point-min))
  (org-update-statistics-cookies t)
  (buffer-substring-no-properties (line-beginning-position) (line-end-position)))"##,
        expect_test::expect![[r#""OK \"* T [66%]\"""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-match-sparse-tree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf6_sparse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO A\n* DONE B\n* TODO C")
  (goto-char (point-min))
  (org-match-sparse-tree nil "TODO")
  (let ((v '()) (h '()))
    (goto-char (point-min))
    (while (not (eobp))
      (let ((hd (org-get-heading t t t t)))
        (when hd
          (if (get-char-property (point) 'invisible) (push hd h) (push hd v))))
      (forward-line))
    (list (nreverse v) (nreverse h))))"##,
        expect_test::expect![[r#""OK ((\"A\" \"B\" \"C\") nil)""#]],
    );
}

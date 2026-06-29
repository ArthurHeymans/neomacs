//! Strong state-xxxxxxxxxxxxx oracle tests — extreme mutable state capture.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

#[test]
fn sxxxxxxxxxxxxx_headline_edit_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Original :old:\nBody")
  (goto-char (point-min))
  (let ((s1 (list (org-get-heading t t t t) (org-get-todo-state) (org-get-tags nil t))))
    (org-edit-headline "Changed")
    (org-todo 'right)
    (org-set-tags '("new"))
    (list s1 (list (org-get-heading t t t t) (org-get-todo-state) (org-get-tags nil t)))))"##,
        expect_test::expect![[
            r#""OK ((\"Original\" \"TODO\" (\"old\")) (#(\"Changed\" 0 7 (org-todo-head \"TODO\")) #(\"DONE\" 0 4 (org-todo-head \"TODO\")) (\"new\")))""#
        ]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_property_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:END:")
  (goto-char (point-min))
  (org-entry-put nil "A" "2")
  (org-entry-put nil "B" "3")
  (let ((p1 (org-entry-properties nil 'standard)))
    (org-entry-delete nil "A")
    (list p1 (org-entry-properties nil 'standard))))"##,
        expect_test::expect![[
            r#""OK (((\"CATEGORY\" . \"???\") (\"B\" . \"3\") (\"A\" . \"2\")) ((\"CATEGORY\" . \"???\") (\"B\" . \"3\")))""#
        ]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_property_inherit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+PROPERTY: V root\n* L1\n:PROPERTIES:\n:V: l1\n:END:\n** L2\n*** L3")
  (goto-char (point-min))
  (search-forward "L3")
  (list (org-entry-get nil "V" 'inherit) (org-entry-get nil "V" nil)))"##,
        expect_test::expect![[r#""OK (\"l1\" nil)""#]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_table_formula() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| 1 | 2 |\n#+TBLFM: $3=$1+$2")
  (goto-char (point-min))
  (org-table-recalculate 'all)
  (org-table-to-lisp))"##,
        expect_test::expect![[
            r#""OK ((#(\"1\" 0 1 (face org-table)) #(\"2\" 0 1 (face org-table)) #(\"3\" 0 1 (face org-table))))""#
        ]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_checkbox_stats() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T [%]\n- [ ] a\n- [ ] b")
  (goto-char (point-min))
  (org-update-statistics-cookies t)
  (forward-line 1)
  (org-toggle-checkbox)
  (org-update-statistics-cookies t)
  (goto-char (point-min))
  (buffer-substring-no-properties (line-beginning-position) (line-end-position)))"##,
        expect_test::expect![[r#""OK \"* T [50%]\"""#]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_sparse_tree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO A\n* DONE B\n* TODO C")
  (goto-char (point-min))
  (org-match-sparse-tree nil "TODO")
  (let ((vis '()) (hid '()))
    (goto-char (point-min))
    (while (not (eobp))
      (let ((h (org-get-heading t t t t)))
        (when h
          (if (get-char-property (point) 'invisible) (push h hid) (push h vis))))
      (forward-line))
    (list (nreverse vis) (nreverse hid))))"##,
        expect_test::expect![[r#""OK ((\"A\" \"B\" \"C\") nil)""#]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_element_parse_modify() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T :tag:")
  (let* ((tree1 (org-element-parse-buffer))
         (h1 (car (org-element-map tree1 'headline
                    (lambda (h) (list (org-element-property :raw-value h) (org-element-property :tags h)))))))
    (goto-char (point-min))
    (org-edit-headline "New")
    (org-set-tags '("newtag"))
    (let* ((tree2 (org-element-parse-buffer))
           (h2 (car (org-element-map tree2 'headline
                      (lambda (h) (list (org-element-property :raw-value h) (org-element-property :tags h)))))))
      (list h1 h2))))"##,
        expect_test::expect![[r#""OK ((\"T\" (\"tag\")) (\"New\" (\"newtag\")))""#]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_export_env() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: T\n#+OPTIONS: toc:nil\n* H")
  (let ((info (org-export-get-environment nil)))
    (list (plist-get info :title) (plist-get info :with-toc))))"##,
        expect_test::expect![[
            r#""OK ((#(\"T\" 0 1 (:parent (#(\"T\" 0 1 (:parent #4)))))) nil)""#
        ]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_link_attr() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+CAPTION: Cap\n#+ATTR_HTML: :width 300\n[[file:img.png]]")
  (let* ((tree (org-element-parse-buffer))
         (l (car (org-element-map tree 'link (lambda (l) l))))
         (p (org-element-property :parent l)))
    (list (org-element-property :path l) (org-element-property :caption p))))"##,
        expect_test::expect![[
            r#""OK (\"img.png\" (((#(\"Cap\" 0 3 (:parent (#(\"Cap\" 0 3 (:parent #6)))))))))""#
        ]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_planning_repeaters() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO W\nSCHEDULED: <2026-01-15 +1w>")
  (org-element-map (org-element-parse-buffer) 'planning
    (lambda (p) (org-element-property :repeater-type (org-element-property :scheduled p)))))"##,
        expect_test::expect![[r#""OK (cumulate)""#]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_timestamp_repeater() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* M\n<2026-01-25 Wed +1w>")
  (org-element-map (org-element-parse-buffer) 'timestamp
    (lambda (t) (list (org-element-property :type t) (org-element-property :repeater-type t)))))"##,
        expect_test::expect![[r#""OK ((active cumulate))""#]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_drawer_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:END:\nBody")
  (org-element-map (org-element-parse-buffer) 'drawer
    (lambda (d) (org-element-property :drawer-name d))))"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_block_switches() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp -n\n(+ 1)\n#+END_SRC")
  (org-element-map (org-element-parse-buffer) 'src-block
    (lambda (b) (list (org-element-property :language b) (org-element-property :switches b)))))"##,
        expect_test::expect![[r#""OK ((\"emacs-lisp\" \"-n\"))""#]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_footnote_markup() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "T[fn:1]\n\n[fn:1] *b*")
  (org-element-map (org-element-parse-buffer) 'footnote-reference
    (lambda (f) (org-element-property :label f))))"##,
        expect_test::expect![[r#""OK (\"1\")""#]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_inline_task() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (require 'org-inlinetask)
  (insert "B\n*************** TODO I\n*************** END\nM")
  (org-element-map (org-element-parse-buffer) 'headline
    (lambda (h) (when (= (org-element-property :level h) 15) (org-element-property :raw-value h)))))"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_hierarchy_contents() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* L1\n** L2a\n*** L3a\n** L2b")
  (org-element-map (org-element-parse-buffer) 'headline
    (lambda (h) (list (org-element-property :level h) (length (org-element-contents h))))))"##,
        expect_test::expect![[r#""OK ((1 2) (2 1) (3 0) (2 0))""#]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_visibility() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3")
  (goto-char (point-min))
  (org-set-startup-visibility 'overview)
  (get-char-property (search-forward "H2") 'invisible))"##,
        expect_test::expect![[r#""ERR (wrong-number-of-arguments (0 . 0) 1)""#]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_outline_path() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* P\n** T1\n*** S1")
  (goto-char (point-min))
  (search-forward "S1")
  (org-get-outline-path))"##,
        expect_test::expect![[r#""OK (\"P\" \"T1\")""#]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_agenda_entries() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO A\n* DONE B")
  (org-map-entries (lambda () (org-get-todo-state)) nil 'file))"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_colview_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+COLUMNS: %25ITEM %TODO\n* TODO T")
  (goto-char (point-min))
  (org-columns-get-format))"##,
        expect_test::expect![[r#""ERR (void-function org-columns-get-format)""#]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_macro_expansion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+MACRO: g H!\n{{{g}}}")
  (let ((raw (buffer-string)))
    (org-macro-replace-all org-macro-templates)
    (list raw (buffer-string))))"##,
        expect_test::expect![[r#""ERR (error \"Undefined Org macro: g; aborting\")""#]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_dynamic_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN: clocktable\n#+END:")
  (goto-char (point-min))
  (org-dblock-update)
  (buffer-string))"##,
        expect_test::expect![[
            r##""OK #(\"#+BEGIN: clocktable\n#+CAPTION: Clock summary at [FIXED-TIME]\n| Headline     | Time   |\n|--------------+--------|\n| *Total time* | *0:00* |\n#+END:\" 61 62 (face org-table) 62 63 (face org-table rear-nonsticky t display (space :relative-width 1)) 63 71 (face org-table) 71 75 (face org-table) 75 76 (face org-table display (space :relative-width 1.001)) 76 77 (face org-table) 77 78 (face org-table rear-nonsticky t display (space :relative-width 1)) 78 82 (face org-table) 82 84 (face org-table) 84 85 (face org-table display (space :relative-width 1.001)) 85 86 (face org-table) 86 87 (face org-table-row) 87 88 (face org-table) 88 112 (face org-table) 112 113 (face org-table-row) 113 114 (face org-table) 114 115 (face org-table rear-nonsticky t display (space :relative-width 1)) 115 127 (org-emphasis t font-lock-multiline t face (bold org-table)) 127 128 (face org-table display (space :relative-width 1.001)) 128 129 (face org-table) 129 130 (face org-table rear-nonsticky t display (space :relative-width 1)) 130 136 (org-emphasis t font-lock-multiline t face (bold org-table)) 136 137 (face org-table display (space :relative-width 1.001)) 137 138 (face org-table) 138 139 (face org-table-row))""##
        ]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_entity_replacement() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "\\alpha")
  (let ((before (buffer-string)))
    (org-toggle-pretty-entities)
    (list before (buffer-string))))"##,
        expect_test::expect![[r#""OK (\"\\\\alpha\" \"\\\\alpha\")""#]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_radio_targets() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "<<<t>>>\nSee t")
  (org-element-map (org-element-parse-buffer) 'radio-target
    (lambda (rt) (org-element-property :value rt))))"##,
        expect_test::expect![[r#""OK (\"t\")""#]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_structure_template() {
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

#[test]
fn sxxxxxxxxxxxxx_comment_fixed() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "# C\n: F")
  (org-element-map (org-element-parse-buffer) 'comment
    (lambda (c) (org-element-property :value c))))"##,
        expect_test::expect![[r#""OK (\"C\")""#]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_link_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "[[https://x][w]]")
  (org-element-map (org-element-parse-buffer) 'link
    (lambda (l) (org-element-property :type l))))"##,
        expect_test::expect![[r#""OK (\"https\")""#]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_keywords() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: T")
  (org-element-map (org-element-parse-buffer) 'keyword
    (lambda (k) (org-element-property :value k))))"##,
        expect_test::expect![[r#""OK (\"T\")""#]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_refile_targets() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* P1\n** T1\n* P2\n** T2")
  (mapcar 'car (org-refile-get-targets nil)))"##,
        expect_test::expect![[r#""OK (\"P1\" \"P2\")""#]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_pcomplete() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "\\agr")
  (length (all-completions "\\ag" (pcomplete-entries))))"##,
        expect_test::expect![[r#""OK 0""#]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_sparse_dates() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T1\nSCHEDULED: <2026-01-15>\n* T2\nSCHEDULED: <2026-02-01>")
  (goto-char (point-min))
  (org-match-sparse-tree nil "SCHEDULED<=\"<2026-01-31>\"")
  (let ((v '()) (h '()))
    (goto-char (point-min))
    (while (not (eobp))
      (let ((hd (org-get-heading t t t t)))
        (when hd
          (if (get-char-property (point) 'invisible) (push hd h) (push hd v))))
      (forward-line))
    (list (nreverse v) (nreverse h))))"##,
        expect_test::expect![[r#""OK ((\"T1\" \"T2\") (\"T1\" \"T2\"))""#]],
    );
}

#[test]
fn sxxxxxxxxxxxxx_multi_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((r '()))
  (with-temp-buffer
    (org-mode)
    (insert "* A\n** A1")
    (push (org-element-map (org-element-parse-buffer) 'headline
            (lambda (h) (org-element-property :raw-value h))) r))
  (with-temp-buffer
    (org-mode)
    (insert "* B\n** B1")
    (push (org-element-map (org-element-parse-buffer) 'headline
            (lambda (h) (org-element-property :raw-value h))) r))
  (nreverse r))"##,
        expect_test::expect![[r#""OK ((\"A\" \"A1\") (\"B\" \"B1\"))""#]],
    );
}

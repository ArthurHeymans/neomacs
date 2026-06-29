//! Strong uncovered-features-57 oracle tests — org-babel helpers, org-src internal.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-babel-get-src-block-info
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf57_babel_info() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp :results value\n(+ 1)\n#+END_SRC")
  (goto-char (point-min))
  (let ((info (org-babel-get-src-block-info)))
    (list (nth 0 info) (nth 2 info))))"##,
        expect_test::expect![[
            r#""OK (\"emacs-lisp\" ((:colname-names) (:rowname-names) (:result-params \"value\" \"replace\") (:result-type . value) (:results . \"value replace\") (:exports . \"code\") (:lexical . \"no\") (:tangle . \"no\") (:hlines . \"no\") (:noweb . \"no\") (:cache . \"no\") (:session . \"none\")))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-babel-get-src-block-lang
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf57_babel_lang() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1)\n#+END_SRC\n#+BEGIN_SRC python\nprint(1)\n#+END_SRC")
  (goto-char (point-min))
  (list (org-babel-get-src-block-lang)
        (progn (search-forward "python") (org-babel-get-src-block-lang))))"##,
        expect_test::expect![[r#""ERR (void-function org-babel-get-src-block-lang)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-babel-expand-src-block
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf57_babel_expand() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp :var x=1 y=2\n(+ x y)\n#+END_SRC")
  (goto-char (point-min))
  (org-babel-expand-src-block))"##,
        expect_test::expect![[r#""OK \"(let ((x '1)\n      (y '2))\n(+ x y)\n)\"""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-babel-goto-src-block-head
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf57_babel_goto() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1)\n(+ 2)\n#+END_SRC")
  (goto-char (point-min))
  (search-forward "(+ 2)")
  (beginning-of-line)
  (org-babel-goto-src-block-head)
  (buffer-substring-no-properties (line-beginning-position) (line-end-position)))"##,
        expect_test::expect![[r##""OK \"#+BEGIN_SRC emacs-lisp\"""##]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-babel-mark-block
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf57_babel_mark() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1)\n(+ 2)\n#+END_SRC")
  (goto-char (point-min))
  (search-forward "(+ 1)")
  (beginning-of-line)
  (org-babel-mark-block)
  (list (region-beginning) (region-end)))"##,
        expect_test::expect![[r#""OK (24 36)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-babel-demarcate-block
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf57_babel_demarcate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1)\n(+ 2)\n(+ 3)\n#+END_SRC")
  (goto-char (point-min))
  (search-forward "(+ 2)")
  (beginning-of-line)
  (org-babel-demarcate-block)
  (buffer-string))"##,
        expect_test::expect![[
            r##""OK \"#+BEGIN_SRC emacs-lisp\n  (+ 1)\n#+END_SRC\n\n#+BEGIN_SRC emacs-lisp\n  (+ 2)\n  (+ 3)\n#+END_SRC\n\"""##
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-babel-insert-result
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf57_babel_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1)\n#+END_SRC")
  (goto-char (point-min))
  (org-babel-insert-result "42" '("value"))
  (buffer-string))"##,
        expect_test::expect![[
            r##""OK \"#+BEGIN_SRC emacs-lisp\n(+ 1)\n#+END_SRC\n\n#+RESULTS:\n: 42\n\"""##
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-babel-result-to-file
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf57_babel_to_file() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(org-babel-result-to-file "test.png" "desc" '("figure"))"##,
        expect_test::expect![[r#""OK \"[[file:test.png][desc]]\"""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-babel-merge-params
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf57_babel_merge() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(org-babel-merge-params '((:results . "value")) '((:results . "output")))"##,
        expect_test::expect![[r#""OK ((:results . \"output\") (:exports . \"\"))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-babel-variable-assignments
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf57_babel_var() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(org-babel-variable-assignments:emacs-lisp '((:var . "x=1") (:var . "y=2")))"##,
        expect_test::expect![[
            r#""ERR (void-function org-babel-variable-assignments:emacs-lisp)""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-babel-result-params
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf57_babel_result_params() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp :results value output\n(+ 1)\n#+END_SRC")
  (goto-char (point-min))
  (org-babel-result-params))"##,
        expect_test::expect![[r#""ERR (void-function org-babel-result-params)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-babel-params-from-properties
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf57_babel_params_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\n:PROPERTIES:\n:header-args: :results value\n:END:\n#+BEGIN_SRC emacs-lisp\n(+ 1)\n#+END_SRC")
  (goto-char (point-min))
  (org-babel-params-from-properties "emacs-lisp"))"##,
        expect_test::expect![[r#""OK (((:results . \"value\")) nil)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-babel-parse-src-block-match
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf57_babel_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp :results value :var x=1\n(+ x)\n#+END_SRC")
  (goto-char (point-min))
  (org-babel-parse-src-block-match))"##,
        expect_test::expect![[r#""ERR (void-function org-babel-parse-src-block-match)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-babel-execute-buffer
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf57_babel_exec_buf() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1)\n#+END_SRC\n#+BEGIN_SRC emacs-lisp\n(+ 2)\n#+END_SRC")
  (org-babel-execute-buffer)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-babel-execute-subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf57_babel_exec_sub() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\n#+BEGIN_SRC emacs-lisp\n(+ 1)\n#+END_SRC\n#+BEGIN_SRC emacs-lisp\n(+ 2)\n#+END_SRC")
  (goto-char (point-min))
  (org-babel-execute-subtree)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-src-do-at-code-block
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf57_src_at() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
  (goto-char (point-min))
  (condition-case nil
      (org-src-do-at-code-block)
    (error nil)))"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-src-tab-first
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf57_src_tab() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
  (goto-char (point-min))
  (condition-case nil
      (org-src-tab-first)
    (error nil)))"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-map src-block
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf57_map_src() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp :results value\n(+ 1)\n#+END_SRC\n#+BEGIN_SRC python :results output\nprint(1)\n#+END_SRC")
  (org-element-map (org-element-parse-buffer) 'src-block
    (lambda (s) (list (org-element-property :language s)
                      (org-element-property :parameters s)
                      (org-element-property :value s)))))"##,
        expect_test::expect![[
            r#""OK ((\"emacs-lisp\" \":results value\" \"(+ 1)\n\") (\"python\" \":results output\" \"print(1)\n\"))""#
        ]],
    );
}

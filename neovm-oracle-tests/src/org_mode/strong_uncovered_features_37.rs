//! Strong uncovered-features-37 oracle tests — org-table formulas, org-src.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-table-formula-to-user
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf37_formula_user() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-table-formula-to-user "$1+$2")
        (org-table-formula-to-user "@1$1+@2$2")
        (org-table-formula-to-user "remote(name,$1)"))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-formula-to-internal
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf37_formula_internal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-table-formula-to-internal "$1+$2")
        (org-table-formula-to-internal "@1$1+@2$2")
        (org-table-formula-to-internal "remote(name,$1)"))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-eval-formula
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf37_eval_formula() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b | c |\n| 1 | 2 |   |\n| 3 | 4 |   |")
  (goto-char (point-min))
  (forward-line 1)
  (org-table-eval-formula "$3=$1+$2")
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-get-range
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf37_table_range() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b | c |\n| 1 | 2 | 3 |\n| 4 | 5 | 6 |")
  (goto-char (point-min))
  (list (org-table-get-range "1" "2")
        (org-table-get-range "2" "3")))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-get
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf37_table_get() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |")
  (goto-char (point-min))
  (list (org-table-get "1" "2")
        (org-table-get "2" "3")))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-put
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf37_table_put() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |")
  (goto-char (point-min))
  (org-table-put "1" "2" "X")
  (list (org-table-get "1" "2") (buffer-string)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-get-elem
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf37_table_elem() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |")
  (goto-char (point-min))
  (list (org-table-get-elem 1 1)
        (org-table-get-elem 1 2)
        (org-table-get-elem 2 1)
        (org-table-get-elem 2 2)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-current-line
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf37_table_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |\n| 3 | 4 |")
  (goto-char (point-min))
  (forward-line 1)
  (org-table-current-line))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-current-column
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf37_table_col() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |")
  (goto-char (point-min))
  (forward-line 1)
  (org-table-current-column))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-analyze
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf37_table_analyze() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n|---+---|\n| 1 | 2 |\n| 3 | 4 |")
  (goto-char (point-min))
  (let ((a (org-table-analyze)))
    (list (nth 0 a) (nth 1 a))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-maybe-eval-formula
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf37_table_eval() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b | c |\n| 1 | 2 |   |\n| 3 | 4 |   |\n#+TBLFM: $3=$1+$2")
  (goto-char (point-min))
  (forward-line 1)
  (org-table-maybe-eval-formula)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-iterate
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf37_table_iter() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 |   |\n| 2 |   |\n#+TBLFM: $2=$1*2")
  (org-table-iterate)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-src-fontify-block
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf37_src_fontify() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
  (goto-char (point-min))
  (condition-case nil
      (org-src-fontify-block)
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-src-construct-edit-buffer-name
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf37_src_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(org-src-construct-edit-buffer-name "emacs-lisp" "*Org Src*")"##);
}

// ═══════════════════════════════════════════════════════════════════════
// org-src-get-lang-mode
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf37_src_lang() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-src-get-lang-mode "emacs-lisp")
        (org-src-get-lang-mode "python")
        (org-src-get-lang-mode "shell")
        (org-src-get-lang-mode "C"))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-src-do-at-code-block
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf37_src_at() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
  (goto-char (point-min))
  (condition-case nil
      (org-src-do-at-code-block)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-src-edit-buffer-p
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf37_src_edit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (org-src-edit-buffer-p))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-src-in-org-buffer-p
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf37_src_in_org() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (org-src-in-org-buffer-p))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-src-tab-first
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf37_src_tab() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
  (goto-char (point-min))
  (condition-case nil
      (org-src-tab-first)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-src-babel-demarcate-block
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf37_src_demarcate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1)\n(+ 2)\n(+ 3)\n#+END_SRC")
  (goto-char (point-min))
  (search-forward "(+ 2)")
  (beginning-of-line)
  (org-babel-demarcate-block)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-src-navigate-block
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf37_src_navigate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1)\n(+ 2)\n#+END_SRC\nNormal\n#+BEGIN_QUOTE\nQ\n#+END_QUOTE")
  (goto-char (point-min))
  (let ((r '()))
    (condition-case nil
        (progn
          (org-next-block 1)
          (push (buffer-substring-no-properties (line-beginning-position) (line-end-position)) r))
      (error nil))
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-property :language
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf37_src_lang_prop() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1)\n#+END_SRC\n#+BEGIN_SRC python\nprint(1)\n#+END_SRC")
  (org-element-map (org-element-parse-buffer) 'src-block
    (lambda (s) (org-element-property :language s))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-property :parameters
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf37_src_params() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp :results value :var x=1\n(+ x)\n#+END_SRC")
  (org-element-map (org-element-parse-buffer) 'src-block
    (lambda (s) (org-element-property :parameters s))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-property :value (src-block)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf37_src_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n(+ 3 4)\n#+END_SRC")
  (org-element-map (org-element-parse-buffer) 'src-block
    (lambda (s) (org-element-property :value s))))"##,
    );
}

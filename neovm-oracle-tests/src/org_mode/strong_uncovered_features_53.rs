//! Strong uncovered-features-53 oracle tests — org-element-cache, org-indent, org-lint.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-element-cache-status
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf53_cache_status() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nBody")
  (let ((s (org-element-cache-status)))
    (list (plist-get s :size)
          (plist-get s :key))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-cache-reset
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf53_cache_reset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nBody")
  (org-element-cache-reset)
  (let ((s (org-element-cache-status)))
    (list (plist-get s :size)
          (plist-get s :key))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-cache-active-p
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf53_cache_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nBody")
  (org-element-cache-active-p))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-cache-flush
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf53_cache_flush() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nBody")
  (org-element-cache-flush (point-min))
  (let ((s (org-element-cache-status)))
    (list (plist-get s :size))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-cache-sync
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf53_cache_sync() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nBody")
  (org-element-cache-sync)
  (let ((s (org-element-cache-status)))
    (list (plist-get s :size))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-cache after insert
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf53_cache_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nBody")
  (let ((s1 (org-element-cache-status)))
    (insert "\nNew line")
    (let ((s2 (org-element-cache-status)))
      (list (plist-get s1 :size) (plist-get s2 :size)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-cache after level change
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf53_cache_level() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2")
  (goto-char (point-min))
  (org-metaright)
  (list (org-element-map (org-element-parse-buffer) 'headline
          (lambda (h) (org-element-property :level h)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-cache after todo change
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf53_cache_todo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H")
  (goto-char (point-min))
  (org-todo)
  (list (org-element-property :todo-keyword (org-element-at-point))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-indent-mode
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf53_indent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\nBody\n** H2\nSub\n*** H3\nDeep")
  (org-indent-mode 1)
  (let ((r '()))
    (goto-char (point-min))
    (while (not (eobp))
      (let ((indent (get-char-property (point) 'line-prefix)))
        (when indent (push (list (line-number-at-pos) indent) r)))
      (forward-line))
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-indent-indent-buffer
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf53_indent_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\nBody\n** H2\nSub\n*** H3\nDeep")
  (org-indent-indent-buffer)
  (let ((r '()))
    (goto-char (point-min))
    (while (not (eobp))
      (let ((indent (get-char-property (point) 'line-prefix)))
        (when indent (push (list (line-number-at-pos) indent) r)))
      (forward-line))
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-lint
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf53_lint() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nSCHEDULED: <invalid>\nBody [[broken]]")
  (length (org-lint)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-lint-report
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf53_lint_report() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nSCHEDULED: <invalid>\nBody [[broken]]")
  (condition-case nil
      (org-lint-report)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-macro--collect-macros
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf53_macro_collect() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+MACRO: a 1\n#+MACRO: b 2\n{{{a}}} {{{b}}}")
  (org-macro--collect-macros))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-property :language
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf53_src_lang() {
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
fn uf53_src_params() {
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
fn uf53_src_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n(+ 3 4)\n#+END_SRC")
  (org-element-map (org-element-parse-buffer) 'src-block
    (lambda (s) (org-element-property :value s))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-src-get-lang-mode
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf53_src_lang_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-src-get-lang-mode "emacs-lisp")
        (org-src-get-lang-mode "python")
        (org-src-get-lang-mode "shell")
        (org-src-get-lang-mode "C"))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-src-construct-edit-buffer-name
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf53_src_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(org-src-construct-edit-buffer-name "emacs-lisp" "*Org Src*")"##);
}

// ═══════════════════════════════════════════════════════════════════════
// org-src-edit-buffer-p
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf53_src_edit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (org-src-edit-buffer-p))"##,
    );
}

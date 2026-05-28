//! Strong uncovered-features-42 oracle tests — org-property, org-reveal, org-narrow.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-entry-properties
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf42_entry_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:B: 2\n:END:")
  (goto-char (point-min))
  (org-entry-properties nil 'standard))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-entry-put
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf42_entry_put() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T")
  (goto-char (point-min))
  (org-entry-put nil "A" "1")
  (org-entry-put nil "B" "2")
  (list (org-entry-get nil "A") (org-entry-get nil "B")))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-entry-delete
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf42_entry_delete() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:B: 2\n:END:")
  (goto-char (point-min))
  (org-entry-delete nil "A")
  (list (org-entry-get nil "A") (org-entry-get nil "B")))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-entry-get-multivalued-property
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf42_entry_multi() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: v1\n:A+: v2\n:END:")
  (goto-char (point-min))
  (org-entry-get-multivalued-property nil "A"))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-entry-put-multivalued-property
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf42_entry_multi_put() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T")
  (goto-char (point-min))
  (org-entry-put-multivalued-property nil "A" "v1" "v2" "v3")
  (list (org-entry-get nil "A")
        (org-entry-get-multivalued-property nil "A")))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-reveal
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf42_reveal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody\n* H1b")
  (goto-char (point-min))
  (org-overview)
  (search-forward "Body")
  (beginning-of-line)
  (org-reveal)
  (buffer-substring-no-properties (point-min) (point-max)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-show-all
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf42_show_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody\n* H1b")
  (org-overview)
  (org-show-all)
  (buffer-substring-no-properties (point-min) (point-max)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-show-context
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf42_show_context() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody\n* H1b")
  (goto-char (point-min))
  (search-forward "Body")
  (beginning-of-line)
  (org-show-context 'agenda)
  (buffer-substring-no-properties (point-min) (point-max)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-show-set-visibility
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf42_show_set() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody\n* H1b")
  (goto-char (point-min))
  (search-forward "H2")
  (beginning-of-line)
  (org-show-set-visibility 'canonical)
  (buffer-substring-no-properties (point-min) (point-max)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-overview
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf42_overview() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody\n* H1b")
  (org-overview)
  (buffer-substring-no-properties (point-min) (point-max)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-content
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf42_content() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody\n* H1b")
  (org-content)
  (buffer-substring-no-properties (point-min) (point-max)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-narrow-to-subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf42_narrow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\nBody 1\n** H2\nSub\n* H2b\nBody 2")
  (goto-char (point-min))
  (org-narrow-to-subtree)
  (let ((narrowed (buffer-string)))
    (widen)
    (list narrowed)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-end-of-subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf42_end() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
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
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-up-heading
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf42_up() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody")
  (search-forward "Body")
  (beginning-of-line)
  (let ((r '()))
    (org-up-heading)
    (push (org-element-property :raw-value (org-element-at-point)) r)
    (org-up-heading)
    (push (org-element-property :raw-value (org-element-at-point)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-backward-heading-same-level
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf42_back_same() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* A\n** B\n** C\n** D\n* E")
  (goto-char (point-max))
  (let ((r '()))
    (org-backward-heading-same-level 1)
    (push (org-element-property :raw-value (org-element-at-point)) r)
    (org-backward-heading-same-level 1)
    (push (org-element-property :raw-value (org-element-at-point)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-forward-heading-same-level
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf42_fwd_same() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* A\n** B\n** C\n** D\n* E")
  (goto-char (point-min))
  (let ((r '()))
    (org-forward-heading-same-level 1)
    (push (org-element-property :raw-value (org-element-at-point)) r)
    (org-forward-heading-same-level 1)
    (push (org-element-property :raw-value (org-element-at-point)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-next-visible-heading
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf42_next_vis() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* A\n** B\n*** C\n* D")
  (goto-char (point-min))
  (let ((r '()))
    (org-next-visible-heading 1)
    (push (org-element-property :raw-value (org-element-at-point)) r)
    (org-next-visible-heading 1)
    (push (org-element-property :raw-value (org-element-at-point)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-previous-visible-heading
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf42_prev_vis() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* A\n** B\n*** C\n* D")
  (goto-char (point-max))
  (let ((r '()))
    (org-previous-visible-heading 1)
    (push (org-element-property :raw-value (org-element-at-point)) r)
    (org-previous-visible-heading 1)
    (push (org-element-property :raw-value (org-element-at-point)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-next-block
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf42_next_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Before\n#+BEGIN_SRC emacs-lisp\n(+ 1)\n#+END_SRC\nBetween\n#+BEGIN_QUOTE\nQ\n#+END_QUOTE\nAfter")
  (goto-char (point-min))
  (let ((r '()))
    (org-next-block 1)
    (push (buffer-substring-no-properties (line-beginning-position) (line-end-position)) r)
    (org-next-block 1)
    (push (buffer-substring-no-properties (line-beginning-position) (line-end-position)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-previous-block
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf42_prev_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Before\n#+BEGIN_SRC emacs-lisp\n(+ 1)\n#+END_SRC\nBetween\n#+BEGIN_QUOTE\nQ\n#+END_QUOTE\nAfter")
  (goto-char (point-max))
  (let ((r '()))
    (org-previous-block 1)
    (push (buffer-substring-no-properties (line-beginning-position) (line-end-position)) r)
    (org-previous-block 1)
    (push (buffer-substring-no-properties (line-beginning-position) (line-end-position)) r)
    (nreverse r)))"##,
    );
}

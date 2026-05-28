//! Strong uncovered-features-46 oracle tests — org-cycle, org-show, org-flag.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-cycle overview
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf46_cycle_overview() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody\n* H1b")
  (goto-char (point-min))
  (org-overview)
  (buffer-substring-no-properties (point-min) (point-max)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-cycle content
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf46_cycle_content() {
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
// org-cycle all
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf46_cycle_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody\n* H1b")
  (org-show-all)
  (buffer-substring-no-properties (point-min) (point-max)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-cycle children
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf46_cycle_children() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\n** H2\n*** H3\nBody\n* H1b")
  (goto-char (point-min))
  (org-cycle 'children)
  (buffer-substring-no-properties (point-min) (point-max)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-cycle subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf46_cycle_subtree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\n** H2\n*** H3\nBody\n* H1b")
  (goto-char (point-min))
  (org-cycle 'subtree)
  (buffer-substring-no-properties (point-min) (point-max)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-global-cycle
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf46_global_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody\n* H1b\n** H2b")
  (let ((r '()))
    (org-global-cycle nil)
    (push (list :first (buffer-substring-no-properties (point-min) (point-max))) r)
    (org-global-cycle nil)
    (push (list :second (buffer-substring-no-properties (point-min) (point-max))) r)
    (org-global-cycle nil)
    (push (list :third (buffer-substring-no-properties (point-min) (point-max))) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-set-startup-visibility
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf46_startup() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+STARTUP: overview\n* H1\n** H2\n*** H3\nBody\n* H1b")
  (org-set-startup-visibility)
  (buffer-substring-no-properties (point-min) (point-max)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-show-context
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf46_show_context() {
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
fn uf46_show_set() {
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
// org-flag-region
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf46_flag() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Line1\nLine2\nLine3\nLine4")
  (let ((r '()))
    (org-flag-region (point-min) (+ (point-min) 10) t 'org-hide-block)
    (push (list :hidden (get-char-property (point-min) 'invisible)) r)
    (org-flag-region (point-min) (+ (point-min) 10) nil 'org-hide-block)
    (push (list :shown (get-char-property (point-min) 'invisible)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-hide-block-toggle
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf46_block_toggle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1)\n#+END_SRC")
  (goto-char (point-min))
  (let ((r '()))
    (org-hide-block-toggle)
    (push (list :hidden (get-char-property (+ (point-min) 20) 'invisible)) r)
    (org-hide-block-toggle)
    (push (list :shown (get-char-property (+ (point-min) 20) 'invisible)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-hide-drawer-toggle
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf46_drawer_toggle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:END:")
  (goto-char (point-min))
  (let ((r '()))
    (org-hide-drawer-toggle)
    (push (list :hidden (get-char-property (search-forward ":A:") 'invisible)) r)
    (org-hide-drawer-toggle)
    (push (list :shown (get-char-property (search-forward ":A:") 'invisible)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-cycle-hide-drawers
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf46_hide_drawers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
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
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-reveal
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf46_reveal() {
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
fn uf46_show_all() {
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
// org-overview
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf46_overview() {
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
fn uf46_content() {
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
// org-cycle with TAB
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf46_cycle_tab() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody\n* H1b")
  (goto-char (point-min))
  (let ((r '()))
    (org-cycle)
    (push (list :after1 (buffer-substring-no-properties (point-min) (point-max))) r)
    (org-cycle)
    (push (list :after2 (buffer-substring-no-properties (point-min) (point-max))) r)
    (org-cycle)
    (push (list :after3 (buffer-substring-no-properties (point-min) (point-max))) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-cycle with S-TAB
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf46_cycle_stab() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody\n* H1b")
  (goto-char (point-min))
  (let ((r '()))
    (org-shifttab)
    (push (list :after1 (buffer-substring-no-properties (point-min) (point-max))) r)
    (org-shifttab)
    (push (list :after2 (buffer-substring-no-properties (point-min) (point-max))) r)
    (org-shifttab)
    (push (list :after3 (buffer-substring-no-properties (point-min) (point-max))) r)
    (nreverse r)))"##,
    );
}

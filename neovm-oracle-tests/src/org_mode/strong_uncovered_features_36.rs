//! Strong uncovered-features-36 oracle tests — org-indent, org-lint, org-ctags.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-indent-mode
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf36_indent() {
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
fn uf36_indent_buffer() {
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
// org-indent-indent-region
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf36_indent_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\nBody\n** H2\nSub\n*** H3\nDeep")
  (org-indent-indent-region (point-min) (point-max))
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
// org-indent-add-properties
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf36_indent_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\nBody\n** H2\nSub")
  (org-indent-add-properties (point-min) (point-max))
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
// org-indent-remove-properties
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf36_indent_remove() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\nBody\n** H2\nSub")
  (org-indent-add-properties (point-min) (point-max))
  (org-indent-remove-properties (point-min) (point-max))
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
// org-indent-refresh-maybe
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf36_indent_refresh() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\nBody\n** H2\nSub")
  (org-indent-refresh-maybe (point-min) (point-max) nil)
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
fn uf36_lint() {
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
fn uf36_lint_report() {
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
// org-lint-add-checker
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf36_lint_add() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-lint-add-checker 'test-checker
      :description "Test checker"
      :verify (lambda () nil))
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-ctags
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf36_ctags() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-ctags)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-ctags-create-tags
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf36_ctags_create() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-ctags-create-tags)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-ctags-find-tag
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf36_ctags_find() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-ctags-find-tag "test-tag")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-ctags-generate-tags
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf36_ctags_gen() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-ctags-generate-tags)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-ctags-update-tags
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf36_ctags_update() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-ctags-update-tags)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-ctags-visit-tags-table
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf36_ctags_visit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-ctags-visit-tags-table)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-checklist (org-checklist-create)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf36_checklist_create() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n- [X] a\n- [ ] b\n- [X] c")
  (goto-char (point-min))
  (let ((done (org-element-map (org-element-parse-buffer) 'item
                (lambda (i) (eq (org-element-property :checkbox i) 'on)))))
    (list (length done))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-cache status after modifications
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf36_cache_modify() {
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
// org-element-cache after heading level change
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf36_cache_level() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2")
  (goto-char (point-min))
  (org-metaright)
  (let ((r '()))
    (push (list :after (org-element-map (org-element-parse-buffer) 'headline
                          (lambda (h) (org-element-property :level h)))) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-cache after todo change
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf36_cache_todo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H")
  (goto-char (point-min))
  (org-todo)
  (let ((r '()))
    (push (list :after (org-element-property :todo-keyword (org-element-at-point))) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-cache after tag change
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf36_cache_tag() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H")
  (goto-char (point-min))
  (org-set-tags '("tag1"))
  (let ((r '()))
    (push (list :after (org-get-tags)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-cache after property change
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf36_cache_prop() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H")
  (goto-char (point-min))
  (org-set-property "A" "1")
  (let ((r '()))
    (push (list :after (org-entry-get nil "A")) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-cache after planning change
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf36_cache_plan() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H")
  (goto-char (point-min))
  (org-schedule nil "<2026-01-15>")
  (let ((r '()))
    (push (list :after (org-element-map (org-element-parse-buffer) 'planning
                          (lambda (p) (org-element-property :scheduled p)))) r)
    (nreverse r)))"##,
    );
}

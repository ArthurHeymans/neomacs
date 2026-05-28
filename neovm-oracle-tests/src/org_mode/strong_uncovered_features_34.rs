//! Strong uncovered-features-34 oracle tests — org-pcomplete, org-list, org-table.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-pcomplete
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_pcomplete() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEG")
  (condition-case nil
      (pcomplete)
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-pcomplete-initial
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_pcomplete_init() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+")
  (condition-case nil
      (org-pcomplete-initial)
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-pcomplete-thing-at-point
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_pcomplete_thing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test")
  (goto-char (point-min))
  (condition-case nil
      (org-pcomplete-thing-at-point)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-struct
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_list_struct() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n  - B\n  - C\n- D")
  (goto-char (point-min))
  (org-list-struct))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-prevs-alist
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_list_prevs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n  - B\n  - C\n- D")
  (let ((struct (org-list-struct)))
    (org-list-prevs-alist struct)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-parents-alist
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_list_parents() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n  - B\n  - C\n- D")
  (let ((struct (org-list-struct)))
    (org-list-parents-alist struct)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-get-nth
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_list_nth() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n  - B\n  - C\n- D")
  (let ((struct (org-list-struct)))
    (list (org-list-get-nth 0 struct)
          (org-list-get-nth 1 struct))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-get-item-end
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_list_item_end() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n  - B\n  - C\n- D")
  (let ((struct (org-list-struct)))
    (list (org-list-get-item-end 1 struct)
          (org-list-get-item-end 2 struct))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-get-item-begin
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_list_item_begin() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n  - B\n  - C\n- D")
  (let ((struct (org-list-struct)))
    (list (org-list-get-item-begin 1 struct)
          (org-list-get-item-begin 2 struct))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-get-bullet
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_list_bullet() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n  1. B\n  2. C\n+ D")
  (let ((struct (org-list-struct)))
    (list (org-list-get-bullet 1 struct)
          (org-list-get-bullet 2 struct)
          (org-list-get-bullet 3 struct))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-get-checkbox
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_list_checkbox() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- [X] A\n- [ ] B\n- [-] C")
  (let ((struct (org-list-struct)))
    (list (org-list-get-checkbox 1 struct)
          (org-list-get-checkbox 2 struct)
          (org-list-get-checkbox 3 struct))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-get-depth
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_list_depth() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n  - B\n    - C\n- D")
  (let ((struct (org-list-struct)))
    (list (org-list-get-depth 1 struct)
          (org-list-get-depth 2 struct)
          (org-list-get-depth 3 struct))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-get-parent
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_list_parent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n  - B\n  - C\n- D")
  (let ((struct (org-list-struct)))
    (list (org-list-get-parent 1 struct)
          (org-list-get-parent 2 struct)
          (org-list-get-parent 3 struct))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-get-children
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_list_children() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n  - B\n  - C\n- D")
  (let ((struct (org-list-struct)))
    (list (org-list-get-children 1 struct)
          (org-list-get-children 2 struct))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-get-siblings
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_list_siblings() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n  - B\n  - C\n- D")
  (let ((struct (org-list-struct)))
    (list (org-list-get-siblings 1 struct)
          (org-list-get-siblings 2 struct))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-get-top-point
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_list_top() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Before\n- A\n  - B\n  - C\n- D\nAfter")
  (org-list-get-top-point))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-get-bottom-point
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_list_bottom() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Before\n- A\n  - B\n  - C\n- D\nAfter")
  (org-list-get-bottom-point))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-struct-apply
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_list_apply() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n  - B\n  - C\n- D")
  (let* ((struct (org-list-struct))
         (new-struct (copy-sequence struct)))
    (org-list-struct-apply new-struct)
    (buffer-string)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-send-item
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_list_send() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n  - B\n  - C\n- D")
  (goto-char (point-min))
  (let ((struct (org-list-struct)))
    (condition-case nil
        (org-list-send-item 'down struct)
      (error nil))
    (buffer-string)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-exchange-items
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_list_exchange() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n- B\n- C")
  (let ((struct (org-list-struct)))
    (condition-case nil
        (org-list-exchange-items 1 2 struct)
      (error nil))
    (buffer-string)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-write-struct
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_list_write() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n  - B\n  - C\n- D")
  (let ((struct (org-list-struct)))
    (org-list-write-struct struct)
    (buffer-string)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-indent-item-generic
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_list_indent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n- B\n- C")
  (goto-char (point-min))
  (forward-line 1)
  (let ((struct (org-list-struct)))
    (org-list-indent-item-generic 1 t struct)
    (buffer-string)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-fix-item-bullet
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_list_fix_bullet() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n  1. B\n  2. C\n- D")
  (let ((struct (org-list-struct)))
    (org-list-fix-item-bullet 2 struct)
    (buffer-string)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-fix-bullet
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_list_fix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n  1. B\n  2. C\n- D")
  (let ((struct (org-list-struct)))
    (org-list-fix-bullet struct)
    (buffer-string)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-struct-fix-box
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_list_fix_box() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- [X] A\n- [ ] B\n- [ ] C")
  (let ((struct (org-list-struct)))
    (org-list-struct-fix-box struct (org-list-parents-alist struct))
    (buffer-string)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-struct-apply-struct
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf34_list_apply_struct() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n  - B\n  - C\n- D")
  (let ((struct (org-list-struct)))
    (org-list-struct-apply-struct struct)
    (buffer-string)))"##,
    );
}

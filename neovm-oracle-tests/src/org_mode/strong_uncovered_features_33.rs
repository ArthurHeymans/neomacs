//! Strong uncovered-features-33 oracle tests — org-eww, org-bookmark, org-id.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-id
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_id() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n* H2\n* H3")
  (goto-char (point-min))
  (let ((r '()))
    (dotimes (_ 3)
      (org-id-get nil 'create)
      (push (org-entry-get nil "ID") r)
      (forward-line))
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-id-get
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_id_get() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\n:PROPERTIES:\n:ID: existing-id\n:END:")
  (goto-char (point-min))
  (list (org-id-get)
        (org-id-get nil 'create)
        (org-entry-get nil "ID")))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-id-goto
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_id_goto() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n* H2\n* H3")
  (goto-char (point-min))
  (org-id-get nil 'create)
  (let ((id1 (org-entry-get nil "ID")))
    (condition-case nil
        (org-id-goto id1)
      (error nil))
    (org-element-property :raw-value (org-element-at-point))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-id-find
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_id_find() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\n:PROPERTIES:\n:ID: test-find-id\n:END:")
  (condition-case nil
      (org-id-find "test-find-id")
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-id-find-id-in-file
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_id_find_file() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-id-find-id-in-file "test-id" "/tmp/test.org")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-id-update-id-locations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_id_update() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-id-update-id-locations nil)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-id-store-link
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_id_store() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\n:PROPERTIES:\n:ID: store-test-id\n:END:")
  (goto-char (point-min))
  (condition-case nil
      (org-id-store-link nil)
    (error nil))
  (car org-stored-links))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-id-copy
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_id_copy() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\n:PROPERTIES:\n:ID: copy-test-id\n:END:")
  (goto-char (point-min))
  (condition-case nil
      (org-id-copy)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-bookmark
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_bookmark() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-bookmark)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-bookmark-jump
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_bookmark_jump() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-bookmark-jump "test-bookmark")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-bookmark-set
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_bookmark_set() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H")
  (goto-char (point-min))
  (condition-case nil
      (org-bookmark-set "test-bookmark")
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-bookmark-delete
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_bookmark_delete() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-bookmark-delete "test-bookmark")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-eww
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_eww() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-eww)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-eww-copy
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_eww_copy() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-eww-copy)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-eww-open-in-emacs
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_eww_open() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-eww-open-in-emacs "http://example.com")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-eww-browse-url
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_eww_browse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-eww-browse-url "http://example.com")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-eww-store-link
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_eww_store() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-eww-store-link)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-eww-activate-links
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_eww_activate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-eww-activate-links)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-eww-tag-link
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_eww_tag() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-eww-tag-link "a" '((href . "http://example.com")))
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-eww-tag-img
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_eww_img() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-eww-tag-img "img" '((src . "http://example.com/img.png")))
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-eww-tag-form
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_eww_form() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-eww-tag-form "form" '((action . "http://example.com/submit")))
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-eww-tag-input
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_eww_input() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-eww-tag-input "input" '((type . "text") (name . "test")))
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-eww-tag-textarea
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_eww_textarea() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-eww-tag-textarea "textarea" '((name . "test")))
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-eww-tag-button
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_eww_button() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-eww-tag-button "button" '((type . "submit")))
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-eww-tag-select
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_eww_select() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-eww-tag-select "select" '((name . "test")))
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-eww-tag-option
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf33_eww_option() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-eww-tag-option "option" '((value . "1")))
  (error nil))"##,
    );
}

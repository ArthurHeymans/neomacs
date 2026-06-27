//! Strict combo oracle probes, batch 58: DEEP characterization of the shr
//! HTML->text rendering divergence. Probes inline tags (b/i/strong/em/span)
//! after text, nested inline, text-element-text siblings, and adjacent block
//! elements to pinpoint which rendering rule differs.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity_with_load;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_l2_shr_inline_tags_after_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-27 (systematic shr newline bug): for every
    // inline tag (b/i/strong/em/span) following text, GNU Emacs inserts a
    // newline+indent between the text node and the element, while Neomacs
    // keeps them inline ("a b c" vs GNU "a\n    b\n    c" style). Same root
    // cause as div_l2_shr_text_element_text_sibling / nested / block /
    // single-element below and the batch-45 shr tests.
    assert_oracle_parity_with_load(
        r##"
(list (with-temp-buffer (shr-insert-document '(p nil "a " (b nil "b") " c")) (buffer-string))
      (with-temp-buffer (shr-insert-document '(p nil "a " (i nil "i") " c")) (buffer-string))
      (with-temp-buffer (shr-insert-document '(p nil "a " (strong nil "s") " c")) (buffer-string))
      (with-temp-buffer (shr-insert-document '(p nil "a " (em nil "e") " c")) (buffer-string))
      (with-temp-buffer (shr-insert-document '(p nil "a " (span nil "sp") " c")) (buffer-string)))
"##,
        &["net/shr.el"],
    );
}

#[test]
fn div_l2_shr_text_element_text_sibling() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-27: "Hello world" (inline) vs GNU "Hello\n
    // world" — shr newline-between-siblings bug (see div_l2_shr_inline_tags).
    assert_oracle_parity_with_load(
        r##"
(with-temp-buffer
  (shr-insert-document '(p nil "Hello " (b nil "world")))
  (buffer-string))
"##,
        &["net/shr.el"],
    );
}

#[test]
fn div_l2_shr_nested_inline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-27: nested inline (b > i) also breaks per-
    // sibling in GNU; Neomacs inline. shr newline bug.
    assert_oracle_parity_with_load(
        r##"
(with-temp-buffer
  (shr-insert-document '(p nil "a " (b nil "b " (i nil "bi")) " c"))
  (buffer-string))
"##,
        &["net/shr.el"],
    );
}

#[test]
fn div_l2_shr_adjacent_block_elements() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-27: adjacent <p> children also break
    // differently ("para one" inline vs GNU "para\n one"). shr newline bug.
    assert_oracle_parity_with_load(
        r##"
(with-temp-buffer
  (shr-insert-document '(div nil (p nil "para one") (p nil "para two")))
  (buffer-string))
"##,
        &["net/shr.el"],
    );
}

#[test]
fn div_l2_shr_single_element_no_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-27: even a single <b> with no preceding
    // text node ("only bold") breaks in GNU vs inline in Neomacs. shr newline
    // bug is systematic across structures.
    assert_oracle_parity_with_load(
        r##"
(with-temp-buffer
  (shr-insert-document '(p nil (b nil "only bold")))
  (buffer-string))
"##,
        &["net/shr.el"],
    );
}

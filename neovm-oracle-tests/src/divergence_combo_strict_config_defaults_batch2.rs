//! Strict combo oracle probes, batch 21: another defaults sweep —
//! coding/charset defaults, search/replace config, kill-ring/undo limits,
//! cursor/display config, comment syntax defaults, fill/indent config, and
//! gc/read/eval config. The defaults class has been the richest divergence
//! source.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_f6_coding_charset_defaults() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (default-value 'buffer-file-coding-system)
      (coding-system-p (default-value 'buffer-file-coding-system))
      default-process-coding-system
      (coding-system-base (car default-process-coding-system))
      (coding-system-base (cdr default-process-coding-system))
      (length (charset-priority-list)))
"##,
    );
}

#[test]
fn div_f6_search_replace_defaults() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list case-fold-search
      case-replace
      (default-value 'search-upper-case)
      (default-value 'isearch-lazy-highlight)
      (default-value 'isearch-lazy-count)
      (default-value 'search-nonincremental-instead-forward))
"##,
    );
}

#[test]
fn div_f6_kill_ring_undo_defaults() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list kill-ring-max
      (length kill-ring)
      (default-value 'interprogram-cut-function)
      (default-value 'interprogram-paste-function)
      undo-limit
      undo-strong-limit
      (default-value 'undo-in-region))
"##,
    );
}

#[test]
fn div_f6_cursor_display_defaults() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (default-value 'cursor-type)
      (default-value 'cursor-in-non-selected-windows)
      (default-value 'blink-cursor-interval)
      (default-value 'x-stretch-cursor)
      (default-value 'visible-cursor)
      (default-value 'void-text-area-pointer))
"##,
    );
}

#[test]
fn div_f6_comment_syntax_defaults() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list comment-start
      comment-end
      comment-column
      comment-multi-line
      comment-indent-function
      (default-value 'comment-start-skip))
"##,
    );
}

#[test]
fn div_f6_fill_indent_defaults() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list indent-tabs-mode
      tab-width
      fill-column
      fill-prefix
      adaptive-fill-mode
      (default-value 'adaptive-fill-regexp)
      (default-value 'colon-double-space))
"##,
    );
}

#[test]
fn div_f6_gc_read_eval_defaults() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list gc-cons-threshold
      gc-cons-percentage
      (default-value 'read-circle)
      (default-value 'eval-expression-print-level)
      (default-value 'eval-expression-print-length)
      (default-value 'load-read-function)
      (default-value 'max-lisp-eval-depth))
"##,
    );
}

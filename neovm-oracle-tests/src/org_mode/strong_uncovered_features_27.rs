//! Strong uncovered-features-27 oracle tests — org-protocol, org-collect, org-plot.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-collect-keywords
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf27_collect() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n#+AUTHOR: Me\n#+DATE: 2026-01-15\n#+OPTIONS: toc:nil\n#+FILETAGS: :t1:t2:")
  (org-collect-keywords '("TITLE" "AUTHOR" "DATE" "OPTIONS" "FILETAGS")))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-collect-keywords with multiple values
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf27_collect_multi() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: T1\n#+TITLE: T2\n#+AUTHOR: A\n#+AUTHOR: B")
  (org-collect-keywords '("TITLE" "AUTHOR")))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-collect-keywords with categories
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf27_collect_cat() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+CATEGORY: default\n* H1\n:PROPERTIES:\n:CATEGORY: custom\n:END:")
  (org-collect-keywords '("CATEGORY")))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-plot/gnuplot
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf27_plot() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+PLOT: title:\"Test\" type:2d with:lines\n| x | y |\n|---+---|\n| 1 | 2 |\n| 2 | 4 |\n| 3 | 6 |")
  (goto-char (point-min))
  (condition-case nil
      (org-plot/gnuplot)
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-plot/gnuplot with options
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf27_plot_opts() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+PLOT: title:\"Test\" type:3d with:lines set:\"xlabel 'X'\" set:\"ylabel 'Y'\"\n| x | y | z |\n|---+---+---|\n| 1 | 2 | 3 |\n| 4 | 5 | 6 |")
  (goto-char (point-min))
  (condition-case nil
      (org-plot/gnuplot)
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-protocol-protocol-handler
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf27_protocol() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-protocol-protocol-handler "org-protocol://store-link?url=http://example.com&title=Test")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-protocol-parse-parameters
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf27_protocol_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(org-protocol-parse-parameters "org-protocol://store-link?url=http://example.com&title=Test")"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-protocol-sanitize-uri
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf27_protocol_sanitize() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-protocol-sanitize-uri "http://example.com")
        (org-protocol-sanitize-uri "https://test.org/path?a=1&b=2")
        (org-protocol-sanitize-uri "file:///tmp/test.txt"))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-protocol-check-protocol-for
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf27_protocol_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(org-protocol-check-protocol-for "store-link")"##);
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-cache-status
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf27_cache() {
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
fn uf27_cache_reset() {
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
// org-table-get/put-range
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf27_table_range() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b | c |\n| 1 | 2 | 3 |\n| 4 | 5 | 6 |")
  (goto-char (point-min))
  (list (org-table-get "1" "2")
        (org-table-get "2" "3")
        (progn (org-table-put "1" "2" "X") (org-table-get "1" "2"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-get-elem
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf27_table_elem() {
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
// org-table-current-line/column
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf27_table_current() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |\n| 3 | 4 |")
  (goto-char (point-min))
  (forward-line 1)
  (list (org-table-current-line)
        (org-table-current-column)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-analyze
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf27_table_analyze() {
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
fn uf27_table_eval() {
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
fn uf27_table_iter() {
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
// org-table-iterate-buffer-tables
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf27_table_iter_buf() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 |   |\n#+TBLFM: $2=$1*2\n\n| c | d |\n| 3 |   |\n#+TBLFM: $2=$1*3")
  (org-table-iterate-buffer-tables)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-export
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf27_table_export() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |")
  (condition-case nil
      (org-table-export "/tmp/test.csv" "orgtbl-to-csv")
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-import
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf27_table_import() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-file "/tmp/test.csv"
  (insert "a,b\n1,2\n3,4"))
(with-temp-buffer
  (org-mode)
  (condition-case nil
      (org-table-import "/tmp/test.csv" nil)
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-convert-region
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf27_table_convert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "a\tb\n1\t2\n3\t4")
  (goto-char (point-min))
  (org-table-convert-region (point-min) (point-max))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-to-lisp
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf27_table_lisp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n|---+---|\n| 1 | 2 |\n| 3 | 4 |")
  (org-table-to-lisp))"##,
    );
}

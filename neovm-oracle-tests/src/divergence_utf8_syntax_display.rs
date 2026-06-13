//! UTF-8 / multibyte *syntax, movement, columns & char-fold* divergence probes.
//!
//! Probes `char-syntax` over multibyte, `forward-word`/`skip-syntax-forward`
//! word boundaries around non-ASCII, `current-column` display accounting with
//! wide chars, word-boundary regex (`\<`/`\>`), and `char-fold-to-regexp`
//! (Unicode folding tables, separate from the general-category tables).

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

// --- char-syntax over multibyte ---------------------------------------------

#[test]
fn div_utf8_char_syntax_non_ascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (char-syntax ?a) (char-syntax ?A) (char-syntax ?1)
      (char-syntax ?é) (char-syntax ?\x3042) (char-syntax ?\x4e2d)
      (char-syntax ?\s) (char-syntax ?\n) (char-syntax ?-)
      (char-syntax ?ß))
"#,
    );
}

// --- word movement around multibyte -----------------------------------------

#[test]
fn div_utf8_forward_word_multibyte_boundaries() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "café 世界 hello")
  (goto-char 1)
  (list (progn (forward-word 1) (point))
        (progn (forward-word 1) (point))
        (progn (forward-word 1) (point))))
"#,
    );
}

#[test]
fn div_utf8_skip_syntax_forward_word_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "café123")
  (skip-syntax-forward "w")
  (point))
"#,
    );
}

#[test]
fn div_utf8_backward_word_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "hello 世界 café")
  (goto-char (point-max))
  (list (progn (backward-word 1) (point))
        (progn (backward-word 1) (point))))
"#,
    );
}

// --- display column accounting ----------------------------------------------

#[test]
fn div_utf8_current_column_wide_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "a世界b😀")
  (current-column))
"#,
    );
}

#[test]
fn div_utf8_indent_to_and_column_with_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "café")
  (end-of-line)
  (list (current-column)
        (progn (move-to-column 10 t) (current-column))
        (buffer-substring (point-min) (point-max))))
"#,
    );
}

// --- word-boundary regex ----------------------------------------------------

#[test]
fn div_utf8_word_boundary_regex_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(progn
  (string-match "\\<café\\>" "le café here and cafébar")
  (list (match-beginning 0) (match-end 0)))
"#,
    );
}

// --- char-fold (Unicode folding tables) -------------------------------------

#[test]
fn div_utf8_char_fold_to_regexp_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (char-fold-to-regexp ?a)
      (char-fold-to-regexp ?A)
      (length (char-fold-to-regexp ?é))
      (length (char-fold-to-regexp ?ß))
      (length (char-fold-to-regexp ?\x3042)))
"#,
    );
}

#[test]
fn div_utf8_char_fold_search_accent_insensitive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // char-fold should match café even when searching for cafe (base+combining
    // equivalence) depending on search defaults.
    assert_oracle_parity(
        r#"
(let ((search-default-mode #'char-fold-to-regexp))
  (list (string-match (char-fold-to-regexp ?e) "café")
        (string-match (char-fold-to-regexp ?é) "cafe")))
"#,
    );
}

// --- regex repeat & group over multibyte ------------------------------------

#[test]
fn div_utf8_regex_multibyte_group_and_repeat() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(progn
  (string-match "\\(世界\\)+"
                "hello 世界 世界 done")
  (list (match-beginning 0) (match-end 0) (match-string 0)
        (match-beginning 1) (match-end 1)))
"#,
    );
}

#[test]
fn div_utf8_regex_char_alternation_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (string-match "[éèêë]" "cafe")
      (progn (string-match "[éèêë]" "cëfe")
             (list (match-beginning 0) (match-end 0)))
      (string-match "[一-龥]" "中文字"))
"#,
    );
}

//! Ignored GNU regexp parity probes for known Neomacs divergences.
//!
//! These tests document behavior confirmed against local GNU Emacs source and
//! oracle runs. They are ignored so default oracle runs stay green while the
//! cases remain easy to execute during regex compatibility work.

use super::common::{
    assert_oracle_parity_with_bootstrap, return_if_neovm_enable_oracle_proptest_not_set,
};

#[test]
#[ignore = "known divergence: mid-pattern ^ and $ are literals in GNU Emacs"]
fn oracle_prop_regexp_gnu_mid_pattern_anchors_are_literals() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(let ((probe (lambda (regexp string)
                       (let ((pos (string-match regexp string)))
                         (list regexp string pos
                               (and pos (match-string 0 string)))))))
      (list
       (funcall probe "a^b" "a^b")
       (funcall probe "a^b" "ab")
       (funcall probe "a$b" "a$b")
       (funcall probe "a$b" "ab")
       (funcall probe "\\(a\\|b^c\\)" "b^c")))"#;
    assert_oracle_parity_with_bootstrap(form);
}

#[test]
#[ignore = "known divergence: GNU Emacs treats \\d and \\D as escaped literals"]
fn oracle_prop_regexp_gnu_backslash_d_is_literal() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(let ((probe (lambda (regexp string)
                       (let ((pos (string-match regexp string)))
                         (list regexp string pos
                               (and pos (match-string 0 string)))))))
      (list
       (funcall probe "\\d" "5")
       (funcall probe "\\d" "d")
       (funcall probe "\\D" "x")
       (funcall probe "\\D" "D")
       (funcall probe "a\\db" "adb")
       (funcall probe "a\\db" "a5b")))"#;
    assert_oracle_parity_with_bootstrap(form);
}

#[test]
#[ignore = "known divergence: GNU Emacs treats escaped control-letter names as literals in regexps"]
fn oracle_prop_regexp_gnu_escaped_control_letters_are_literals() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(let ((probe (lambda (regexp string)
                       (let ((pos (string-match regexp string)))
                         (list regexp string pos
                               (and pos (match-string 0 string)))))))
      (list
       (funcall probe "\\t" "t")
       (funcall probe "\\t" "\t")
       (funcall probe "\\n" "n")
       (funcall probe "\\n" "\n")
       (funcall probe "\\r" "r")
       (funcall probe "\\r" "\r")
       (funcall probe "\\f" "f")
       (funcall probe "\\f" "\f")
       (funcall probe "\\a" "a")
       (funcall probe "\\a" (string 7))
       (funcall probe "\\e" "e")
       (funcall probe "\\e" (string 27))))"#;
    assert_oracle_parity_with_bootstrap(form);
}

#[test]
#[ignore = "known divergence: GNU Emacs string-match does not use \\= as a string-start assertion"]
fn oracle_prop_regexp_gnu_at_point_anchor_is_not_for_string_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(let ((probe (lambda (regexp string &optional start)
                       (let ((pos (string-match regexp string start)))
                         (list regexp string start pos
                               (and pos (match-string 0 string)))))))
      (list
       (funcall probe "\\=" "")
       (funcall probe "\\=" "abc")
       (funcall probe "a\\=b" "ab")
       (funcall probe "\\=b" "ab" 1)))"#;
    assert_oracle_parity_with_bootstrap(form);
}

#[test]
#[ignore = "known divergence: GNU Emacs treats interval syntax without a preceding atom as literal text"]
fn oracle_prop_regexp_gnu_bare_intervals_are_literals() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(let ((probe (lambda (regexp string)
                       (let ((pos (string-match regexp string)))
                         (list regexp string pos
                               (and pos (match-string 0 string)))))))
      (list
       (funcall probe "\\{1\\}" "{1}")
       (funcall probe "\\{1,2\\}" "{1,2}")
       (funcall probe "\\{,2\\}" "{,2}")))"#;
    assert_oracle_parity_with_bootstrap(form);
}

#[test]
#[ignore = "known divergence: GNU Emacs rejects malformed \\_ symbol-boundary escapes"]
fn oracle_prop_regexp_gnu_malformed_symbol_boundary_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(list
      (condition-case err
          (string-match "\\_x" "_x")
        (error (list :error (car err) (cadr err))))
      (condition-case err
          (string-match "\\_" "_")
        (error (list :error (car err) (cadr err)))))"#;
    assert_oracle_parity_with_bootstrap(form);
}

#[test]
#[ignore = "known divergence: GNU Emacs does not signal for invalid \\s/\\S syntax-class designators"]
fn oracle_prop_regexp_gnu_invalid_syntax_class_designators() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(let ((probe (lambda (regexp string)
                       (condition-case err
                           (let ((pos (string-match regexp string)))
                             (list regexp string pos
                                   (and pos (match-string 0 string))))
                         (error (list regexp string :error
                                      (car err) (cadr err)))))))
      (list
       (funcall probe "\\sz" "z")
       (funcall probe "\\sq" "q")
       (funcall probe "\\s0" "0")
       (funcall probe "\\S0" "0")
       (funcall probe "\\S0" "a")))"#;
    assert_oracle_parity_with_bootstrap(form);
}

#[test]
#[ignore = "known divergence: GNU Emacs rejects unknown (?...) group extensions"]
fn oracle_prop_regexp_gnu_unknown_group_extension_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(list
      (condition-case err
          (string-match "\\(?x:a\\)" "(?x:a)")
        (error (list :error (car err) (cadr err))))
      (condition-case err
          (string-match "\\(??:a\\)" "(??:a)")
        (error (list :error (car err) (cadr err))))
      (condition-case err
          (string-match "\\(?-1:a\\)" "(?-1:a)")
        (error (list :error (car err) (cadr err)))))"#;
    assert_oracle_parity_with_bootstrap(form);
}

#[test]
#[ignore = "known divergence: Neomacs uses approximate built-in category predicates"]
fn oracle_prop_regexp_gnu_category_tables() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(let ((probe (lambda (regexp string)
                       (let ((pos (string-match regexp string)))
                         (list regexp string pos
                               (and pos (match-string 0 string)))))))
      (list
       (funcall probe "\\ca" "\n")
       (funcall probe "\\ca" "\t")
       (funcall probe "\\ca" "A")
       (funcall probe "\\c|" (string #x4e2d))
       (funcall probe "\\c6" (string #x0664))))"#;
    assert_oracle_parity_with_bootstrap(form);
}

#[test]
#[ignore = "known divergence: Neomacs case folding is incomplete for non-ASCII regexp matches"]
fn oracle_prop_regexp_gnu_unicode_case_folding() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(let ((case-fold-search t)
      (probe (lambda (regexp string)
               (let ((pos (string-match regexp string)))
                 (list regexp string pos
                       (and pos (match-string 0 string)))))))
  (list
   (funcall probe (string #x03a9) (string #x03c9))
   (funcall probe (string #x0414) (string #x0434))
   (funcall probe (string #x00e9) (string #x00c9))
   (funcall probe "[[:upper:]]+" "abc")
   (funcall probe "[[:lower:]]+" "ABC")))"#;
    assert_oracle_parity_with_bootstrap(form);
}

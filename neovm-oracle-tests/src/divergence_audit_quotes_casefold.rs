//! Quote-style (format-message) + asymmetric case-fold divergences.
//!
//! Two source-audit veins:
//!  (a) `format-message` converts ` → ‘ and ' → ’; many error/warn paths use it,
//!      so GNU emits curly quotes where Neomacs (missing the conversion) emits
//!      straight quotes — affects every error-message comparison.
//!  (b) asymmetric case-fold: `string-match` lower→upper folding is one-directional
//!      in Neomacs (σ fails to match Σ, though Σ→σ works).

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

// --- format-message quote conversion ----------------------------------------

#[test]
fn div_aq_format_message_backtick_curly() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(format-message "a `b' c")"##);
}

#[test]
fn div_aq_format_message_apostrophe_curly() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(format-message "don't `do' it")"##);
}

#[test]
fn div_aq_error_message_quote_style() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e (replace-regexp-in-string "x" "\\z" "x") (error (cadr e)))
"##,
    );
}

#[test]
fn div_aq_user_error_quote_style() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e (user-error "bad `%s'" 'foo) (error (cadr e)))
"##,
    );
}

#[test]
fn div_aq_signal_message_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e (signal 'wrong-type-argument (list 'stringp 5)) (error (cadr e)))
"##,
    );
}

#[test]
fn div_aq_message_with_format_message() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // message uses format (straight); format-message uses curly.
    assert_oracle_parity(
        r##"
(list (format "a `%s'" 'b) (format-message "a `%s'" 'b))
"##,
    );
}

// --- asymmetric case-fold (lower -> upper) ----------------------------------

#[test]
fn div_acf_sigma_lower_to_upper() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(let ((case-fold-search t)) (string-match "σ" "Σ"))"##);
}

#[test]
fn div_acf_sigma_upper_to_lower() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(let ((case-fold-search t)) (string-match "Σ" "σ"))"##);
}

#[test]
fn div_acf_alpha_lower_to_upper() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(let ((case-fold-search t)) (string-match "α" "Α"))"##);
}

#[test]
fn div_acf_omega_lower_to_upper() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(let ((case-fold-search t)) (string-match "ω" "Ω"))"##);
}

#[test]
fn div_acf_greek_lowercase_loop() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Probe several Greek lowercase -> uppercase case-fold matches.
    assert_oracle_parity(
        r##"
(let ((case-fold-search t))
  (mapcar (lambda (p) (if (string-match (char-to-string (car p))
                                         (char-to-string (cdr p)))
                          t nil))
          '((945 . 913) (946 . 914) (956 . 924) (969 . 937))))
"##,
    );
}

#[test]
fn div_acf_cyrillic_lower_to_upper() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((case-fold-search t))
  (list (string-match "б" "Б") (string-match "я" "Я")))
"##,
    );
}

#[test]
fn div_acf_ascii_case_fold_control() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // ASCII case-fold should work in both directions.
    assert_oracle_parity(
        r##"
(let ((case-fold-search t))
  (list (string-match "a" "A") (string-match "A" "a")))
"##,
    );
}

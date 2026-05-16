//! Oracle parity tests for GNU `key-parse' and `kbd' edge semantics.

use super::common::assert_oracle_parity_with_bootstrap;

#[test]
fn oracle_kbd_repeats_and_historical_macro_delimiters() {
    let form = r#"
(list
 (kbd "3*a")
 (key-description (kbd "3*a"))
 (kbd "2*C-x")
 (key-description (kbd "2*C-x"))
 (kbd "C-x ( C-d C-x )")
 (key-description (kbd "C-x ( C-d C-x )")))"#;
    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_key_parse_comments_and_line_boundaries() {
    let form = r#"
(list
 (kbd "a ;; ignored to end of line
b")
 (key-description (kbd "a ;; ignored to end of line
b"))
 (kbd "a REM ignored to end of line
b")
 (key-description (kbd "a REM ignored to end of line
b")))"#;
    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_key_parse_octal_events() {
    let form = r#"
(list
 (kbd "\\101")
 (key-description (kbd "\\101"))
 (kbd "\\377")
 (key-description (kbd "\\377"))
 (listify-key-sequence (kbd "\\377")))"#;
    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_key_parse_angle_tokens_with_embedded_spaces() {
    let form = r#"
(list
 (key-parse "<mouse-1>")
 (key-parse "<mouse-1> a")
 (condition-case err (key-parse "<mouse 1>") (error (car err)))
 (condition-case err (kbd "<mouse 1>") (error (car err))))"#;
    assert_oracle_parity_with_bootstrap(form);
}

//! Strict combo oracle probes, batch 45: HTML/CSS rendering loaded libraries
//! via assert_oracle_parity_with_load — net/shr.el (HTML dom -> text) and
//! textmodes/css.el (CSS parsing/expansion). These are complex and commonly
//! used by eww/notmuch/etc.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity_with_load;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_i2_shr_render_document() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-27:
    // GNU Emacs: OK #("Hello\nworld\n" ...) — shr renders the <b> child on a
    //   new line after the plain text node within the <p>.
    // Neomacs:   OK #("Hello world\n" ...) — shr keeps the inline <b> content
    //   on the same line as the preceding text.
    // shr-insert-document renders inline sibling content differently.
    assert_oracle_parity_with_load(
        r##"
(with-temp-buffer
  (let ((dom '(html nil (body nil (p nil "Hello " (b nil "world"))))))
    (shr-insert-document dom))
  (buffer-string))
"##,
        &["net/shr.el"],
    );
}

#[test]
fn div_i2_shr_render_list_and_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-27: shr <ul>/<li>/<a> rendering diverges
    // from GNU (rendered+propertized output differs in length and content:
    // ~1355 vs ~1670 bytes). HTML->text list/link rendering is not equivalent.
    assert_oracle_parity_with_load(
        r##"
(with-temp-buffer
  (let ((dom '(html nil
                (body nil
                  (ul nil (li nil "one") (li nil "two"))
                  (a ((href . "http://x")) "link")))))
    (shr-insert-document dom))
  (buffer-string))
"##,
        &["net/shr.el"],
    );
}

#[test]
fn div_i2_css_expand_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(list (condition-case err (css-expand-value 'margin '(1 2 3 4)) (error (cons 'err (car err))))
      (condition-case err (css-expand-value 'color "red") (error (cons 'err (car err)))))
"##,
        &["textmodes/css.el"],
    );
}

#[test]
fn div_i2_css_color_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(list (condition-case err (css-color-string-to-hsl "#ff0000") (error (cons 'err (car err))))
      (condition-case err (css-color-parse-hex "#00ff00") (error (cons 'err (car err)))))
"##,
        &["textmodes/css.el"],
    );
}

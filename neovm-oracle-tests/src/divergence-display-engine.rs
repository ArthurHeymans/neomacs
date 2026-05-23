//! Divergence tests: display engine, glyphless chars, display tables.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_display_table_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'make-display-table)
  (fboundp 'display-table-slot)
  (fboundp 'set-display-table-slot)
  (fboundp 'standard-display-table)
  (fboundp 'buffer-display-table)
  (fboundp 'window-display-table)) "#,
    );
}

#[test]
fn divergence_glyphless_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (boundp 'glyphless-char-display)
  (boundp 'glyphless-char-display-control)
  (listp glyphless-char-display-control)
  (fboundp 'glyphless-char-p)) "#,
    );
}

#[test]
fn divergence_redisplay_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'redisplay)
  (fboundp 'force-window-update)
  (fboundp 'window-text-height)
  (fboundp 'window-text-width)
  (boundp 'redisplay-dont-pause)
  (boundp 'redisplay-skip-initialization)) "#,
    );
}

#[test]
fn divergence_line_height() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'default-line-height)
  (fboundp 'line-pixel-height)
  (fboundp 'window-line-height)
  (boundp 'line-spacing)
  (numberp line-spacing)) "#,
    );
}

#[test]
fn divergence_invisible_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (boundp 'buffer-invisibility-spec)
  (listp buffer-invisibility-spec)
  (fboundp 'add-to-invisibility-spec)
  (fboundp 'remove-from-invisibility-spec)) "#,
    );
}

#[test]
fn divergence_selective_display() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (boundp 'selective-display)
  (boundp 'selective-display-ellipses)
  (fboundp 'set-selective-display)) "#,
    );
}

#[test]
fn divergence_overlay_arrow() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (boundp 'overlay-arrow-position)
  (boundp 'overlay-arrow-string)
  (fboundp 'set-overlay-arrow)) "#,
    );
}

#[test]
fn divergence_truncate_lines() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (boundp 'truncate-lines)
  (booleanp truncate-lines)
  (boundp 'truncate-partial-width-windows)
  (numberp truncate-partial-width-windows)) "#,
    );
}

#[test]
fn divergence_word_wrap() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (boundp 'word-wrap)
  (booleanp word-wrap)
  (boundp 'wrap-prefix)
  (boundp 'wrap-prefix-function)) "#,
    );
}

#[test]
fn divergence_display_margin() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'set-window-margins)
  (fboundp 'window-margins)
  (fboundp 'set-window-fringes)
  (fboundp 'window-fringes)
  (fboundp 'set-window-scroll-bars)
  (fboundp 'window-scroll-bars)) "#,
    );
}

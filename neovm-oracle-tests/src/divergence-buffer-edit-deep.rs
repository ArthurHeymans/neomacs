//! Divergence tests: buffer substring, insert, delete, replace edge.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_buffer_substring_variants() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "Hello World")
  (put-text-property 1 6 'face 'bold)
  (list (buffer-substring 1 12)
        (buffer-substring-no-properties 1 12)
        (length (buffer-substring 1 12))
        (length (buffer-substring-no-properties 1 12)))) "#,
    );
}

#[test]
fn divergence_insert_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert-char ?X 5)
  (buffer-string)) "#,
    );
}

#[test]
fn divergence_insert_before_markers() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "Hello")
  (let ((m (point-marker)))
    (set-marker-insertion-type m t)
    (goto-char 3)
    (insert-before-markers "XY")
    (list (buffer-string) (marker-position m)))) "#,
    );
}

#[test]
fn divergence_delete_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "Hello World")
  (delete-region 6 12)
  (buffer-string)) "#,
    );
}

#[test]
fn divergence_delete_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "Hello World")
  (goto-char 1)
  (delete-char 5)
  (buffer-string)) "#,
    );
}

#[test]
fn divergence_delete_backward() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "Hello World")
  (goto-char 12)
  (delete-backward-char 6)
  (buffer-string)) "#,
    );
}

#[test]
fn divergence_replace_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "foo bar foo baz foo")
  (goto-char 1)
  (while (search-forward "foo" nil t)
    (replace-match "quux"))
  (buffer-string)) "#,
    );
}

#[test]
fn divergence_substitute_command_keys() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'substitute-command-keys)
  (stringp (substitute-command-keys "hello"))
  (substitute-command-keys "hello")) "#,
    );
}

#[test]
fn divergence_format_mode_line_spec() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'format-mode-line)
  (stringp (format-mode-line "%b"))
  (stringp (format-mode-line "%m"))
  (stringp (format-mode-line "%p"))) "#,
    );
}

#[test]
fn divergence_propertize() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((s (propertize "hello" 'face 'bold 'mouse-face 'highlight)))
  (list (get-text-property 0 'face s)
        (get-text-property 0 'mouse-face s)
        (length s)
        (stringp s))) "#,
    );
}

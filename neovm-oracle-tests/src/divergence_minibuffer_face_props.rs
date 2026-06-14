//! Minibuffer × face/overlay/propertize cross-cutting coverage.
//!
//! Verifies that the face/text-property application across the completion and
//! prompt machinery matches GNU: propertization of completion strings
//! (completions-common-part / completions-first-difference / completion--string
//! / completions-highlight / mouse-face), format-prompt (no properties on the
//! returned string — face applied at display via minibuffer-prompt-properties),
//! completion face default attributes, and minibuffer-message.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_mfp_all_completions_string_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((cs (let ((completion-styles '(basic)))
             (completion-all-completions "a" '("apple" "apricot" "banana") nil 1)))
       (s (car cs)))
  (list (text-properties-at 0 s)
        (text-properties-at 1 s)
        (text-properties-at 2 s)))
"##,
    );
}

#[test]
fn div_mfp_all_completions_base_size() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((completion-styles '(basic)))
  (completion-all-completions "a" '("apple" "apricot") nil 0))
"##,
    );
}

#[test]
fn div_mfp_insert_strings_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (completion--insert-strings '("apple" "apricot"))
      (list (buffer-substring-no-properties 1 (point-max))
            (text-properties-at 1)))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_mfp_completion_face_attributes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (facep 'completions-common-part)
      (facep 'completions-first-difference)
      (facep 'completions-annotations)
      (facep 'completions-highlight)
      (face-attribute 'completions-common-part :foreground)
      (face-attribute 'completions-first-difference :foreground))
"##,
    );
}

#[test]
fn div_mfp_format_prompt_no_inline_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // The prompt face is applied at display time via minibuffer-prompt-properties,
    // not stored on the format-prompt string itself.
    assert_oracle_parity(
        r##"
(let ((p (format-prompt "Prompt" "d")))
  (list (text-properties-at 0 p)
        (text-properties-at 5 p)
        (text-properties-at (- (length p) 1) p)
        p))
"##,
    );
}

#[test]
fn div_mfp_minibuffer_message_no_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e (minibuffer-message "msg %s" "x") (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_mfp_display_completion_list_error_parity() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // display-completion-list in batch errors identically (wrong-type-argument
    // on a buffer position) — no divergence in the error path.
    assert_oracle_parity(
        r##"
(condition-case e
    (with-current-buffer (get-buffer-create "*mfp-comp*")
      (completion-list-mode)
      (display-completion-list '("apple" "apricot" "banana") "a")
      (text-properties-at 1))
  (error (cons 'errored (car e))))
"##,
    );
}

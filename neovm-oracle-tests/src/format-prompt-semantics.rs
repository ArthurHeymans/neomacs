//! Oracle parity tests for GNU `minibuffer.el` `format-prompt` semantics.
//!
//! `format-prompt` composes prompt text, optional format arguments, list
//! defaults, empty defaults, and `minibuffer-default-prompt-format`.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_prop_format_prompt_default_presence() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((minibuffer-default-prompt-format " (default %s)"))
  (list
   (format-prompt "Name" "alice")
   (format-prompt "Name" nil)
   (format-prompt "Name" "")
   (format-prompt "Name" 42)
   (format-prompt "Name" '(first second third))))
"#;

    assert_oracle_parity(form);
}

#[test]
fn oracle_prop_format_prompt_prompt_format_arguments() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((minibuffer-default-prompt-format " [%s]"))
  (list
   (format-prompt "Open %s" "README.md" "file")
   (format-prompt "Replace %s with %s" "new" "old" "new")
   (format-prompt "%s/%s" '("main" "ignored") "branch" "remote")))
"#;

    assert_oracle_parity(form);
}

#[test]
fn oracle_prop_format_prompt_custom_default_prompt_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (let ((minibuffer-default-prompt-format " {%s}"))
   (format-prompt "Project" "neomacs"))
 (let ((minibuffer-default-prompt-format " default=%S"))
   (format-prompt "Value" '(alpha beta)))
 (let ((minibuffer-default-prompt-format ""))
   (format-prompt "No suffix" "x")))
"#;

    assert_oracle_parity(form);
}

#[test]
fn oracle_prop_format_prompt_substitute_command_keys() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((minibuffer-default-prompt-format " (default `%s')"))
  (list
   (format-prompt "Use \\[find-file]" "path")
   (format-prompt "Command `%s'" "M-x" "compile")))
"#;

    assert_oracle_parity(form);
}

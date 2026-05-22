//! Divergence tests: treesit, tree-sitter integration deep.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_treesit_available() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (featurep 'treesit)
  (fboundp 'treesit-available-p)
  (fboundp 'treesit-language-available-p))"#,
    );
}

#[test]
fn divergence_treesit_parser() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'treesit-parser-create)
  (fboundp 'treesit-parser-delete)
  (fboundp 'treesit-parser-root-node)
  (fboundp 'treesit-parse-string))"#,
    );
}

#[test]
fn divergence_treesit_node() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'treesit-node-type)
  (fboundp 'treesit-node-text)
  (fboundp 'treesit-node-start)
  (fboundp 'treesit-node-end)
  (fboundp 'treesit-node-parent))"#,
    );
}

#[test]
fn divergence_treesit_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'treesit-query-compile)
  (fboundp 'treesit-query-capture)
  (fboundp 'treesit-query-string))"#,
    );
}

#[test]
fn divergence_treesit_indent() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'treesit-simple-indent-rules)
  (fboundp 'treesit-indent)
  (fboundp 'treesit-check-indent))"#,
    );
}

#[test]
fn divergence_treesit_fontify() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'treesit-font-lock-rules)
  (fboundp 'treesit-font-lock-feature-list))"#,
    );
}

#[test]
fn divergence_treesit_search() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'treesit-search-subtree)
  (fboundp 'treesit-search-forward)
  (fboundp 'treesit-search-forward-goto))"#,
    );
}

#[test]
fn divergence_treesit_thing() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'treesit-thing-at-point)
  (fboundp 'treesit-nav-start-of-name)
  (fboundp 'treesit-nav-end-of-name))"#,
    );
}

#[test]
fn divergence_treesit_transpose() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'treesit-transpose-sexps)
  (fboundp 'treesit-forward-sexp)
  (fboundp 'treesit-backward-sexp))"#,
    );
}

#[test]
fn divergence_treesit_inspect() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'treesit-explore-mode)
  (fboundp 'treesit-inspect-mode)
  (fboundp 'treesit-node-check))"#,
    );
}

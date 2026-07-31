use std::time::Duration;

use crate::{
    ASCIIDOC_MODE_MELPA_PIN, CachedMelpaOracle, EmacsRuntime, elisp_string,
    prepare_cached_tree_sitter_grammar_from_subdirectory,
};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod activation;
mod completion;
mod diagnostics;
mod editing;
mod font_lock;
mod navigation;
mod surface;

const ASCIIDOC_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(180);
const ASCIIDOC_TREE_SITTER_REPOSITORY: &str = "https://github.com/cathaysia/tree-sitter-asciidoc";
const ASCIIDOC_TREE_SITTER_REVISION: &str = "68b1a5b9bdb01dcd049c1252ea0c33a2063df72e";

fn asciidoc_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    let gnu_emacs = EmacsRuntime::gnu_emacs();
    let block_grammar_dir = prepare_cached_tree_sitter_grammar_from_subdirectory(
        &gnu_emacs,
        "asciidoc",
        ASCIIDOC_TREE_SITTER_REPOSITORY,
        ASCIIDOC_TREE_SITTER_REVISION,
        "tree-sitter-asciidoc/src",
    )
    .expect("prepare pinned AsciiDoc block grammar below ./tmp");
    let inline_grammar_dir = prepare_cached_tree_sitter_grammar_from_subdirectory(
        &gnu_emacs,
        "asciidoc-inline",
        ASCIIDOC_TREE_SITTER_REPOSITORY,
        ASCIIDOC_TREE_SITTER_REVISION,
        "tree-sitter-asciidoc_inline/src",
    )
    .expect("prepare pinned AsciiDoc inline grammar below ./tmp");
    let block_grammar_dir = elisp_string(&block_grammar_dir.to_string_lossy());
    let inline_grammar_dir = elisp_string(&inline_grammar_dir.to_string_lossy());

    CachedMelpaOracle::new(ASCIIDOC_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned asciidoc-mode source below ./tmp")
        .with_prelude(format!(
            "(setq treesit-extra-load-path (list {block_grammar_dir} {inline_grammar_dir}))"
        ))
        .with_timeout(ASCIIDOC_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed asciidoc-mode parity test")
        .into()
}

fn assert_asciidoc_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = asciidoc_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("asciidoc-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_asciidoc_mode_parity(elisp_form: &str, expected: Expect) {
    assert_asciidoc_mode_source_parity("asciidoc-mode.el", elisp_form, expected);
}

pub(crate) fn assert_asciidoc_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_asciidoc_mode_source_parity("asciidoc-mode-autoloads.el", elisp_form, expected);
}





/// Multi-probe batch for `assert_asciidoc_mode_autoload_parity` cases (2a).
pub(crate) fn assert_asciidoc_mode_autoload_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        asciidoc_mode_oracle("asciidoc-mode-autoloads.el"),
        &name,
        "asciidoc_mode_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_asciidoc_mode_parity` cases (2a).
pub(crate) fn assert_asciidoc_mode_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        asciidoc_mode_oracle("asciidoc-mode.el"),
        &name,
        "asciidoc_mode_parity",
        cases,
    );
}

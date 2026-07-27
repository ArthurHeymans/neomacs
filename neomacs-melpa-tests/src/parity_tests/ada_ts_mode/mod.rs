use std::time::Duration;

use crate::{
    ADA_TS_MODE_MELPA_PIN, CachedMelpaOracle, EmacsRuntime, elisp_string,
    prepare_cached_tree_sitter_grammar,
};
use expect_test::Expect;

mod adapters;
mod configuration;
mod editing;
mod indentation;
mod projects;
mod registry;
mod state_machine;
mod treesit_behavior;
mod treesit_matrix;
mod upstream_matrix;

const ADA_TS_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const ADA_TREE_SITTER_REPOSITORY: &str = "https://github.com/briot/tree-sitter-ada";
const ADA_TREE_SITTER_REVISION: &str = "6b58259a08b1a22ba0247a7ce30be384db618da6";

fn ada_ts_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    let grammar_dir = prepare_cached_tree_sitter_grammar(
        &EmacsRuntime::gnu_emacs(),
        "ada",
        ADA_TREE_SITTER_REPOSITORY,
        ADA_TREE_SITTER_REVISION,
    )
    .expect("prepare pinned Ada Tree-sitter grammar below ./tmp");
    let grammar_dir = elisp_string(&grammar_dir.to_string_lossy());
    CachedMelpaOracle::new(ADA_TS_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned ada-ts-mode source below ./tmp")
        .with_prelude(format!(
            "(setq treesit-extra-load-path (list {grammar_dir}))"
        ))
        .with_timeout(ADA_TS_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ada-ts-mode parity test")
        .into()
}

fn assert_ada_ts_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ada_ts_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ada-ts-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ada_ts_mode_parity(elisp_form: &str, expected: Expect) {
    assert_ada_ts_mode_source_parity("ada-ts-mode.el", elisp_form, expected);
}

pub(crate) fn assert_ada_ts_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_ada_ts_mode_source_parity("ada-ts-mode-autoloads.el", elisp_form, expected);
}

pub(crate) fn assert_ada_ts_mode_eglot_parity(elisp_form: &str, expected: Expect) {
    assert_ada_ts_mode_source_parity("ada-ts-lspclient-eglot.el", elisp_form, expected);
}

pub(crate) fn assert_ada_ts_mode_lsp_mode_parity(elisp_form: &str, expected: Expect) {
    assert_ada_ts_mode_source_parity("ada-ts-lspclient-lsp-mode.el", elisp_form, expected);
}

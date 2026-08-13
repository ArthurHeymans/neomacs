use std::str::FromStr;

use super::{Frontend, ScenarioId, scenario, scenarios};

#[test]
fn catalog_exposes_the_rust_lsp_typing_workload_as_a_typed_scenario() {
    let scenarios = scenarios();
    assert_eq!(scenarios.len(), 1);

    let rust_lsp = scenario(ScenarioId::RustLspTyping).expect("registered scenario");
    assert_eq!(rust_lsp.id, ScenarioId::RustLspTyping);
    assert_eq!(rust_lsp.id.to_string(), "rust-lsp-typing");
    assert_eq!(
        ScenarioId::from_str("rust-lsp-typing"),
        Ok(ScenarioId::RustLspTyping)
    );
    assert_eq!(
        rust_lsp.default_frontend,
        Frontend::Tui {
            rows: 40,
            columns: 120,
        }
    );
    assert!(rust_lsp.description.contains("Tree-sitter"));
    assert!(rust_lsp.description.contains("LSP Mode"));
}

#[test]
fn unknown_scenario_names_are_rejected_instead_of_silently_falling_back() {
    let error = ScenarioId::from_str("rust-typing").expect_err("unknown scenario must fail");
    assert!(error.to_string().contains("rust-typing"));
}

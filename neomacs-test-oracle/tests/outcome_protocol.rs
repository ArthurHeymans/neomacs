use neomacs_test_oracle::{EvalOutcome, extract_marked_outcome, wrap_elisp_outcome};

const MARKER: &str = "NEOMACS-TEST-OUTCOME:";

#[test]
fn parses_values_and_signals_from_editor_output() {
    assert_eq!(
        extract_marked_outcome("package noise\nNEOMACS-TEST-OUTCOME:OK (2 4 6 8)\n", MARKER)
            .unwrap(),
        EvalOutcome::Value("(2 4 6 8)".to_string())
    );
    assert_eq!(
        extract_marked_outcome(
            "NEOMACS-TEST-OUTCOME:ERR (wrong-type-argument numberp \"x\")\n",
            MARKER,
        )
        .unwrap(),
        EvalOutcome::Signal("(wrong-type-argument numberp \"x\")".to_string())
    );
}

#[test]
fn rejects_missing_or_malformed_outcomes() {
    assert!(extract_marked_outcome("ordinary output", MARKER).is_err());
    assert!(extract_marked_outcome("NEOMACS-TEST-OUTCOME:MAYBE t", MARKER).is_err());
}

#[test]
fn elisp_wrapper_captures_the_last_value_and_errors() {
    let wrapper = wrap_elisp_outcome("(message \"setup\")", "(list 1 2 3)", MARKER);

    assert!(wrapper.contains(r##"(message "setup")"##));
    assert!(wrapper.contains("(list 1 2 3)"));
    assert!(wrapper.contains("(condition-case"));
    assert!(wrapper.contains("OK "));
    assert!(wrapper.contains("ERR "));
    assert!(wrapper.contains(MARKER));
    assert!(wrapper.contains("NEOMACS_TEST_SANDBOX_ROOT"));
    assert!(wrapper.contains("NEOMACS_TEST_WORKSPACE_ROOT"));
    assert!(wrapper.contains("neomacs--test-oracle-normalized"));
    assert!(wrapper.contains("(print-escape-newlines t)"));
    assert!(wrapper.contains("(print-escape-control-characters t)"));
}

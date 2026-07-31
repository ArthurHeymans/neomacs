use neomacs_test_oracle::{
    BatchProbe, EvalOutcome, extract_marked_batch_outcomes, extract_marked_outcome,
    validate_batch_case_id, wrap_elisp_batch_outcomes, wrap_elisp_outcome,
};

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

#[test]
fn parses_batch_outcomes_in_order() {
    let stdout = "\
noise
NEOMACS-TEST-OUTCOME:alpha:OK (1 2)
more noise
NEOMACS-TEST-OUTCOME:beta:ERR (user-error \"nope\")
";
    let cases = extract_marked_batch_outcomes(stdout, MARKER).unwrap();
    assert_eq!(
        cases,
        vec![
            neomacs_test_oracle::MarkedBatchOutcome {
                id: "alpha".into(),
                outcome: EvalOutcome::Value("(1 2)".into()),
            },
            neomacs_test_oracle::MarkedBatchOutcome {
                id: "beta".into(),
                outcome: EvalOutcome::Signal("(user-error \"nope\")".into()),
            },
        ]
    );
}

#[test]
fn rejects_duplicate_or_missing_batch_outcomes() {
    assert!(extract_marked_batch_outcomes("no markers", MARKER).is_err());
    assert!(extract_marked_batch_outcomes(
        "NEOMACS-TEST-OUTCOME:a:OK t\nNEOMACS-TEST-OUTCOME:a:OK nil\n",
        MARKER,
    )
    .is_err());
}

#[test]
fn batch_wrapper_runs_setup_once_and_names_each_probe() {
    let cases = [
        BatchProbe {
            id: "reads",
            probe: "(+ 1 2)",
        },
        BatchProbe {
            id: "writes",
            probe: "(list 'a 'b)",
        },
    ];
    let wrapper = wrap_elisp_batch_outcomes("(setq x 1)", &cases, MARKER).unwrap();
    assert!(wrapper.contains("(setq x 1)"));
    assert!(wrapper.contains("(+ 1 2)"));
    assert!(wrapper.contains("(list 'a 'b)"));
    assert!(wrapper.contains(r#""reads""#));
    assert!(wrapper.contains(r#""writes""#));
    // One shared setup progn, two per-case condition-case forms.
    assert_eq!(wrapper.matches("(condition-case").count(), 2);
    assert!(wrapper.contains("neomacs--test-oracle-normalized"));
}

#[test]
fn batch_case_ids_reject_colon_and_whitespace() {
    assert!(validate_batch_case_id("ok_id").is_ok());
    assert!(validate_batch_case_id("bad:id").is_err());
    assert!(validate_batch_case_id("bad id").is_err());
    assert!(validate_batch_case_id("").is_err());
}

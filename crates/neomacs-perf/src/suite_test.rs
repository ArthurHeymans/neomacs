use std::fs;
use std::path::PathBuf;

use super::{
    ComparisonSampleCount, MachinePolicy, PerfError, SUITE_ARTIFACT_SCHEMA_VERSION, ScenarioId,
    SuiteArtifact, SuiteId, SuiteScenarioResult, SuiteVerdict, evaluate_suite, read_history_link,
};

fn result(scenario: ScenarioId, threshold: f64, change: Option<f64>) -> SuiteScenarioResult {
    SuiteScenarioResult {
        scenario,
        maximum_regression_percent: threshold,
        percent_change: change,
        comparison_artifact: PathBuf::from(format!("{scenario}/comparison.json")),
    }
}

#[test]
fn suite_thresholds_distinguish_improvements_regressions_and_rejections() {
    assert_eq!(
        evaluate_suite(&[
            result(ScenarioId::Startup, 12.0, Some(11.9)),
            result(ScenarioId::RegexSearch, 8.0, Some(-20.0)),
        ]),
        SuiteVerdict::Passed
    );

    let SuiteVerdict::Regressed { regressions } = evaluate_suite(&[
        result(ScenarioId::Startup, 12.0, Some(12.1)),
        result(ScenarioId::RegexSearch, 8.0, Some(-20.0)),
    ]) else {
        panic!("a change beyond the scenario budget must regress the suite")
    };
    assert_eq!(regressions.len(), 1);
    assert_eq!(regressions[0].scenario, ScenarioId::Startup);

    assert_eq!(
        evaluate_suite(&[result(ScenarioId::OrgEditing, 8.0, None)]),
        SuiteVerdict::Rejected {
            scenarios: vec![ScenarioId::OrgEditing],
        }
    );
}

#[test]
fn history_rejects_an_unknown_suite_artifact_schema() {
    let workspace_tmp = PathBuf::from(env!("CARGO_WORKSPACE_DIR")).join("tmp");
    fs::create_dir_all(&workspace_tmp).expect("create workspace scratch root");
    let directory = tempfile::Builder::new()
        .prefix("neomacs-perf-suite-history-")
        .tempdir_in(workspace_tmp)
        .expect("create suite history scratch directory");
    let path = directory.path().join("suite.json");
    let mut artifact = SuiteArtifact {
        schema_version: 99,
        suite_id: "old-standard-suite".to_string(),
        suite: SuiteId::Standard,
        baseline_editor: PathBuf::from("baseline-emacs"),
        candidate_editor: PathBuf::from("candidate-emacs"),
        samples_per_side: ComparisonSampleCount::new(3).expect("valid sample count"),
        machine: MachinePolicy::default(),
        counters: None,
        started_unix_ms: 1,
        total_elapsed_us: 2,
        previous_suite: None,
        scenarios: Vec::new(),
        verdict: SuiteVerdict::Passed,
    };
    fs::write(
        &path,
        serde_json::to_vec_pretty(&artifact).expect("serialize old suite artifact"),
    )
    .expect("write old suite artifact");

    let error = read_history_link(&path).expect_err("unknown suite schemas must not form lineage");
    let PerfError::InvalidSuiteHistory { message, .. } = error else {
        panic!("suite history schema rejection must be typed")
    };
    assert!(message.contains("schema version 99"));

    artifact.schema_version = SUITE_ARTIFACT_SCHEMA_VERSION;
    fs::write(
        &path,
        serde_json::to_vec_pretty(&artifact).expect("serialize current suite artifact"),
    )
    .expect("write current suite artifact");
    let link = read_history_link(&path).expect("current suite schema forms valid lineage");
    assert_eq!(link.suite_id, "old-standard-suite");
    assert_eq!(link.retained_path, PathBuf::from("previous-suite.json"));
    assert_eq!(link.sha256.len(), 64);
    assert!(link.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()));
}

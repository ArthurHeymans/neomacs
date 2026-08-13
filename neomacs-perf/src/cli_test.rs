use std::ffi::OsString;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::time::Duration;

use super::{ComparisonSampleCount, Frontend, PerfCommand, ScenarioId, parse_perf_command};

fn parse(args: &[&str]) -> Result<PerfCommand, super::PerfCliError> {
    parse_perf_command(args.iter().map(OsString::from))
}

#[test]
fn run_command_parses_into_a_typed_workload_request() {
    assert_eq!(
        parse(&[
            "run",
            "rust-lsp-typing",
            "--editor",
            "target/profiling/neomacs",
            "--iterations",
            "25",
            "--frontend",
            "gui",
            "--timeout-secs",
            "90",
        ])
        .expect("parse valid run command"),
        PerfCommand::Run {
            scenario: ScenarioId::RustLspTyping,
            editor: Some(PathBuf::from("target/profiling/neomacs")),
            iterations: NonZeroU32::new(25).expect("non-zero literal"),
            frontend: Some(Frontend::Gui {
                width: 1200,
                height: 800,
            }),
            timeout: Duration::from_secs(90),
        }
    );
}

#[test]
fn run_command_rejects_zero_iterations_before_launch() {
    let error = parse(&["run", "rust-lsp-typing", "--iterations", "0"])
        .expect_err("zero iterations must fail");
    assert!(error.to_string().contains("greater than zero"));
}

#[test]
fn compare_command_requires_two_editors_and_parses_repetition_controls() {
    assert_eq!(
        parse(&[
            "compare",
            "rust-lsp-typing",
            "--baseline-editor",
            "target/release/neomacs",
            "--candidate-editor",
            "target/release-pgo/neomacs",
            "--samples",
            "7",
            "--iterations",
            "20",
            "--frontend",
            "tui",
            "--timeout-secs",
            "180",
        ])
        .expect("parse valid compare command"),
        PerfCommand::Compare {
            scenario: ScenarioId::RustLspTyping,
            baseline_editor: PathBuf::from("target/release/neomacs"),
            candidate_editor: PathBuf::from("target/release-pgo/neomacs"),
            samples: ComparisonSampleCount::new(7).expect("valid sample count"),
            iterations: NonZeroU32::new(20).expect("non-zero literal"),
            frontend: Some(Frontend::Tui {
                rows: 40,
                columns: 120,
            }),
            timeout: Duration::from_secs(180),
        }
    );
}

#[test]
fn compare_command_rejects_a_missing_candidate_before_launch() {
    let error = parse(&[
        "compare",
        "rust-lsp-typing",
        "--baseline-editor",
        "target/release/neomacs",
    ])
    .expect_err("both editor identities are required");
    assert!(error.to_string().contains("--candidate-editor is required"));
}

#[test]
fn compare_command_rejects_fewer_than_three_samples_per_side() {
    let error = parse(&[
        "compare",
        "rust-lsp-typing",
        "--baseline-editor",
        "target/release/neomacs",
        "--candidate-editor",
        "target/release-pgo/neomacs",
        "--samples",
        "2",
    ])
    .expect_err("two samples cannot characterize run-to-run dispersion");
    assert!(error.to_string().contains("--samples must be at least 3"));
}

#[test]
fn list_and_help_are_explicit_commands() {
    assert_eq!(parse(&["list"]).expect("parse list"), PerfCommand::List);
    assert_eq!(parse(&["--help"]).expect("parse help"), PerfCommand::Help);
}

use std::ffi::OsString;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::time::Duration;

use super::{Frontend, PerfCommand, ScenarioId, parse_perf_command};

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
fn list_and_help_are_explicit_commands() {
    assert_eq!(parse(&["list"]).expect("parse list"), PerfCommand::List);
    assert_eq!(parse(&["--help"]).expect("parse help"), PerfCommand::Help);
}

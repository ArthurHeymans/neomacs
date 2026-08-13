use std::ffi::OsString;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::time::Duration;

use thiserror::Error;

use crate::{
    ComparisonRequest, ComparisonSampleCount, ComparisonVerdict, Frontend, PerfError, PerfHarness,
    RunRequest, ScenarioId, scenarios,
};

const DEFAULT_ITERATIONS: NonZeroU32 = NonZeroU32::new(100).expect("100 is non-zero");
const DEFAULT_SAMPLES: ComparisonSampleCount =
    ComparisonSampleCount::new(5).expect("5 meets the minimum sample count");
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PerfCommand {
    List,
    Run {
        scenario: ScenarioId,
        editor: Option<PathBuf>,
        iterations: NonZeroU32,
        frontend: Option<Frontend>,
        timeout: Duration,
    },
    Compare {
        scenario: ScenarioId,
        baseline_editor: PathBuf,
        candidate_editor: PathBuf,
        samples: ComparisonSampleCount,
        iterations: NonZeroU32,
        frontend: Option<Frontend>,
        timeout: Duration,
    },
    Help,
}

#[derive(Debug, Error)]
pub enum PerfCliError {
    #[error("{message}\n\n{usage}", usage = perf_usage())]
    Usage { message: String },
    #[error(transparent)]
    Harness(#[from] PerfError),
    #[error("performance run was rejected; inspect {artifact}: {reason}")]
    RunRejected { artifact: PathBuf, reason: String },
    #[error("performance comparison was rejected; inspect {artifact}: {reason}")]
    ComparisonRejected { artifact: PathBuf, reason: String },
}

pub fn parse_perf_command(
    args: impl IntoIterator<Item = OsString>,
) -> Result<PerfCommand, PerfCliError> {
    let mut args = args.into_iter();
    let Some(command) = args.next() else {
        return Err(usage_error("performance command is required"));
    };
    match command.to_str() {
        Some("list") => {
            reject_trailing_args(args, "list")?;
            Ok(PerfCommand::List)
        }
        Some("run") => parse_run_command(args),
        Some("compare") => parse_compare_command(args),
        Some("help" | "--help" | "-h") => Ok(PerfCommand::Help),
        Some(unknown) => Err(usage_error(format!(
            "unknown performance command `{unknown}`"
        ))),
        None => Err(usage_error("performance command is not valid UTF-8")),
    }
}

fn parse_compare_command(
    mut args: impl Iterator<Item = OsString>,
) -> Result<PerfCommand, PerfCliError> {
    let scenario = required_scenario(&mut args, "compare")?;
    let mut baseline_editor = None;
    let mut candidate_editor = None;
    let mut samples = DEFAULT_SAMPLES;
    let mut iterations = DEFAULT_ITERATIONS;
    let mut frontend = None;
    let mut timeout = DEFAULT_TIMEOUT;

    while let Some(option) = args.next() {
        let option_text = option
            .to_str()
            .ok_or_else(|| usage_error("performance option is not valid UTF-8"))?;
        match option_text {
            "--baseline-editor" => {
                baseline_editor = Some(PathBuf::from(required_value(
                    &mut args,
                    "--baseline-editor",
                )?));
            }
            "--candidate-editor" => {
                candidate_editor = Some(PathBuf::from(required_value(
                    &mut args,
                    "--candidate-editor",
                )?));
            }
            "--samples" => {
                let raw = required_utf8_value(&mut args, "--samples")?;
                let parsed = raw.parse::<u32>().map_err(|_| {
                    usage_error(format!(
                        "--samples requires an unsigned integer, got `{raw}`"
                    ))
                })?;
                samples = ComparisonSampleCount::new(parsed).ok_or_else(|| {
                    usage_error(format!(
                        "--samples must be at least {}",
                        ComparisonSampleCount::MINIMUM
                    ))
                })?;
            }
            "--iterations" => {
                iterations = parse_non_zero_u32(&mut args, "--iterations")?;
            }
            "--frontend" => {
                frontend = Some(parse_frontend(&mut args)?);
            }
            "--timeout-secs" => {
                timeout = parse_timeout(&mut args)?;
            }
            "--help" | "-h" => return Ok(PerfCommand::Help),
            unknown => {
                return Err(usage_error(format!("unknown compare option `{unknown}`")));
            }
        }
    }

    Ok(PerfCommand::Compare {
        scenario,
        baseline_editor: baseline_editor
            .ok_or_else(|| usage_error("--baseline-editor is required"))?,
        candidate_editor: candidate_editor
            .ok_or_else(|| usage_error("--candidate-editor is required"))?,
        samples,
        iterations,
        frontend,
        timeout,
    })
}

fn parse_run_command(
    mut args: impl Iterator<Item = OsString>,
) -> Result<PerfCommand, PerfCliError> {
    let scenario = required_scenario(&mut args, "run")?;
    let mut editor = None;
    let mut iterations = DEFAULT_ITERATIONS;
    let mut frontend = None;
    let mut timeout = DEFAULT_TIMEOUT;

    while let Some(option) = args.next() {
        let option_text = option
            .to_str()
            .ok_or_else(|| usage_error("performance option is not valid UTF-8"))?;
        match option_text {
            "--editor" => {
                editor = Some(PathBuf::from(required_value(&mut args, "--editor")?));
            }
            "--iterations" => {
                iterations = parse_non_zero_u32(&mut args, "--iterations")?;
            }
            "--frontend" => {
                frontend = Some(parse_frontend(&mut args)?);
            }
            "--timeout-secs" => {
                timeout = parse_timeout(&mut args)?;
            }
            "--help" | "-h" => return Ok(PerfCommand::Help),
            unknown => {
                return Err(usage_error(format!("unknown run option `{unknown}`")));
            }
        }
    }

    Ok(PerfCommand::Run {
        scenario,
        editor,
        iterations,
        frontend,
        timeout,
    })
}

fn required_scenario(
    args: &mut impl Iterator<Item = OsString>,
    command: &str,
) -> Result<ScenarioId, PerfCliError> {
    let scenario_name = args
        .next()
        .ok_or_else(|| usage_error(format!("{command} requires a scenario name")))?;
    let scenario_name = scenario_name
        .to_str()
        .ok_or_else(|| usage_error("scenario name is not valid UTF-8"))?;
    scenario_name
        .parse::<ScenarioId>()
        .map_err(|error| usage_error(error.to_string()))
}

fn parse_non_zero_u32(
    args: &mut impl Iterator<Item = OsString>,
    option: &str,
) -> Result<NonZeroU32, PerfCliError> {
    let raw = required_utf8_value(args, option)?;
    let parsed = raw.parse::<u32>().map_err(|_| {
        usage_error(format!(
            "{option} requires an unsigned integer, got `{raw}`"
        ))
    })?;
    NonZeroU32::new(parsed)
        .ok_or_else(|| usage_error(format!("{option} must be greater than zero")))
}

fn parse_frontend(args: &mut impl Iterator<Item = OsString>) -> Result<Frontend, PerfCliError> {
    let raw = required_utf8_value(args, "--frontend")?;
    match raw.as_str() {
        "batch" => Ok(Frontend::Batch),
        "tui" => Ok(Frontend::Tui {
            rows: 40,
            columns: 120,
        }),
        "gui" => Ok(Frontend::Gui {
            width: 1200,
            height: 800,
        }),
        unknown => Err(usage_error(format!(
            "unknown frontend `{unknown}`; expected batch, tui, or gui"
        ))),
    }
}

fn parse_timeout(args: &mut impl Iterator<Item = OsString>) -> Result<Duration, PerfCliError> {
    let raw = required_utf8_value(args, "--timeout-secs")?;
    let seconds = raw.parse::<u64>().map_err(|_| {
        usage_error(format!(
            "--timeout-secs requires an unsigned integer, got `{raw}`"
        ))
    })?;
    if seconds == 0 {
        return Err(usage_error("--timeout-secs must be greater than zero"));
    }
    Ok(Duration::from_secs(seconds))
}

fn required_value(
    args: &mut impl Iterator<Item = OsString>,
    option: &str,
) -> Result<OsString, PerfCliError> {
    args.next()
        .ok_or_else(|| usage_error(format!("{option} requires a value")))
}

fn required_utf8_value(
    args: &mut impl Iterator<Item = OsString>,
    option: &str,
) -> Result<String, PerfCliError> {
    let value = required_value(args, option)?;
    value
        .into_string()
        .map_err(|_| usage_error(format!("{option} value is not valid UTF-8")))
}

fn reject_trailing_args(
    mut args: impl Iterator<Item = OsString>,
    command: &str,
) -> Result<(), PerfCliError> {
    if let Some(argument) = args.next() {
        return Err(usage_error(format!(
            "{command} does not accept argument `{}`",
            argument.to_string_lossy()
        )));
    }
    Ok(())
}

fn usage_error(message: impl Into<String>) -> PerfCliError {
    PerfCliError::Usage {
        message: message.into(),
    }
}

pub fn run_cli(
    workspace_root: impl AsRef<Path>,
    args: impl IntoIterator<Item = OsString>,
) -> Result<(), PerfCliError> {
    let workspace_root = workspace_root.as_ref();
    match parse_perf_command(args)? {
        PerfCommand::List => {
            for scenario in scenarios() {
                println!("{}\t{}", scenario.id, scenario.description);
            }
            Ok(())
        }
        PerfCommand::Help => {
            print!("{}", perf_usage());
            Ok(())
        }
        PerfCommand::Run {
            scenario,
            editor,
            iterations,
            frontend,
            timeout,
        } => {
            let editor = editor.unwrap_or_else(|| workspace_root.join("target/release/neomacs"));
            let mut request = RunRequest::new(scenario, editor, iterations).with_timeout(timeout);
            if let Some(frontend) = frontend {
                request = request.with_frontend(frontend);
            }
            let report = PerfHarness::new(workspace_root).run(&request)?;
            println!("artifact = {}", report.artifact_path.display());
            if report.artifact.verdict.is_valid() {
                println!("verdict  = valid");
                Ok(())
            } else {
                Err(PerfCliError::RunRejected {
                    artifact: report.artifact_path,
                    reason: format!("{:?}", report.artifact.verdict),
                })
            }
        }
        PerfCommand::Compare {
            scenario,
            baseline_editor,
            candidate_editor,
            samples,
            iterations,
            frontend,
            timeout,
        } => {
            let mut request = ComparisonRequest::new(
                scenario,
                baseline_editor,
                candidate_editor,
                samples,
                iterations,
            )
            .with_timeout(timeout);
            if let Some(frontend) = frontend {
                request = request.with_frontend(frontend);
            }
            let report = PerfHarness::new(workspace_root).compare(&request)?;
            println!("comparison = {}", report.artifact_path.display());
            match &report.artifact.verdict {
                ComparisonVerdict::Valid { summary } => {
                    println!("verdict    = valid");
                    println!(
                        "baseline   = {:.3} ± {:.3} MAD {:?}",
                        summary.baseline_median,
                        summary.baseline_median_absolute_deviation,
                        summary.unit
                    );
                    println!(
                        "candidate  = {:.3} ± {:.3} MAD {:?}",
                        summary.candidate_median,
                        summary.candidate_median_absolute_deviation,
                        summary.unit
                    );
                    println!("change     = {:+.2}%", summary.percent_change);
                    Ok(())
                }
                ComparisonVerdict::Rejected { reasons } => Err(PerfCliError::ComparisonRejected {
                    artifact: report.artifact_path,
                    reason: format!("{reasons:?}"),
                }),
            }
        }
    }
}

pub fn perf_usage() -> &'static str {
    "\
Usage:
  cargo xtask perf list
  cargo xtask perf run SCENARIO [--editor PATH] [--iterations N]
                         [--frontend batch|tui|gui] [--timeout-secs N]
  cargo xtask perf compare SCENARIO --baseline-editor PATH --candidate-editor PATH
                         [--samples N>=3] [--iterations N]
                         [--frontend batch|tui|gui] [--timeout-secs N]

Every run writes a structured artifact below ./tmp/perf. Only a run whose
fixture invariants all pass receives a valid verdict and performance samples.
A comparison is valid only when every baseline and candidate run is valid.
"
}

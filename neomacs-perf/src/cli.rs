use std::ffi::OsString;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::time::Duration;

use thiserror::Error;

use crate::{Frontend, PerfError, PerfHarness, RunRequest, ScenarioId, scenarios};

const DEFAULT_ITERATIONS: NonZeroU32 = NonZeroU32::new(100).expect("100 is non-zero");
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
        Some("help" | "--help" | "-h") => Ok(PerfCommand::Help),
        Some(unknown) => Err(usage_error(format!(
            "unknown performance command `{unknown}`"
        ))),
        None => Err(usage_error("performance command is not valid UTF-8")),
    }
}

fn parse_run_command(
    mut args: impl Iterator<Item = OsString>,
) -> Result<PerfCommand, PerfCliError> {
    let scenario_name = args
        .next()
        .ok_or_else(|| usage_error("run requires a scenario name"))?;
    let scenario_name = scenario_name
        .to_str()
        .ok_or_else(|| usage_error("scenario name is not valid UTF-8"))?;
    let scenario = scenario_name
        .parse::<ScenarioId>()
        .map_err(|error| usage_error(error.to_string()))?;
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
                let raw = required_utf8_value(&mut args, "--iterations")?;
                let parsed = raw.parse::<u32>().map_err(|_| {
                    usage_error(format!(
                        "--iterations requires an unsigned integer, got `{raw}`"
                    ))
                })?;
                iterations = NonZeroU32::new(parsed)
                    .ok_or_else(|| usage_error("--iterations must be greater than zero"))?;
            }
            "--frontend" => {
                let raw = required_utf8_value(&mut args, "--frontend")?;
                frontend = Some(match raw.as_str() {
                    "batch" => Frontend::Batch,
                    "tui" => Frontend::Tui {
                        rows: 40,
                        columns: 120,
                    },
                    "gui" => Frontend::Gui {
                        width: 1200,
                        height: 800,
                    },
                    unknown => {
                        return Err(usage_error(format!(
                            "unknown frontend `{unknown}`; expected batch, tui, or gui"
                        )));
                    }
                });
            }
            "--timeout-secs" => {
                let raw = required_utf8_value(&mut args, "--timeout-secs")?;
                let seconds = raw.parse::<u64>().map_err(|_| {
                    usage_error(format!(
                        "--timeout-secs requires an unsigned integer, got `{raw}`"
                    ))
                })?;
                if seconds == 0 {
                    return Err(usage_error("--timeout-secs must be greater than zero"));
                }
                timeout = Duration::from_secs(seconds);
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
    }
}

pub fn perf_usage() -> &'static str {
    "\
Usage:
  cargo xtask perf list
  cargo xtask perf run SCENARIO [--editor PATH] [--iterations N]
                         [--frontend batch|tui|gui] [--timeout-secs N]

Every run writes a structured artifact below ./tmp/perf. Only a run whose
fixture invariants all pass receives a valid verdict and performance samples.
"
}

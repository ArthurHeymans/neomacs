mod artifact;
mod artifact_store;
mod catalog;
mod cli;
mod comparison;
mod harness;

pub use artifact::{
    ArtifactFile, ArtifactKind, CorrectnessMismatch, EditorProvenance, Measurement, MetricName,
    MetricUnit, RunArtifact, RunVerdict,
};
pub use catalog::{Frontend, ScenarioId, ScenarioSpec, scenario, scenarios};
pub use cli::{PerfCliError, PerfCommand, parse_perf_command, run_cli};
pub use comparison::{
    ComparisonArtifact, ComparisonInput, ComparisonMetricSummary, ComparisonRejection,
    ComparisonReport, ComparisonRequest, ComparisonRun, ComparisonRunOutcome, ComparisonRunRole,
    ComparisonSampleCount, ComparisonVerdict, comparison_schedule,
};
#[cfg(test)]
pub(crate) use comparison::{ComparisonObservation, evaluate_comparison};
pub use harness::{PerfError, PerfHarness, RunReport, RunRequest};
#[cfg(test)]
pub(crate) use harness::{collect_editor_provenance, configure_benchmark_environment};

#[cfg(test)]
mod artifact_test;
#[cfg(test)]
mod catalog_test;
#[cfg(test)]
mod cli_test;
#[cfg(test)]
mod comparison_test;
#[cfg(test)]
mod harness_test;

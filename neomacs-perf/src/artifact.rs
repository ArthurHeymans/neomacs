use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{Frontend, ScenarioId};

/// Why a completed workload cannot contribute a performance sample.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectnessMismatch {
    pub invariant: String,
    pub expected: String,
    pub actual: String,
}

/// Semantic outcome of a run, kept separate from process execution success.
///
/// Only `Valid` may be aggregated or compared. A completed process with the
/// wrong editor state is a correctness failure, not a fast benchmark result.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum RunVerdict {
    Valid {
        measurements: Vec<Measurement>,
    },
    CorrectnessMismatch {
        mismatches: Vec<CorrectnessMismatch>,
    },
    InfrastructureFailure {
        message: String,
    },
}

impl RunVerdict {
    pub const fn is_valid(&self) -> bool {
        matches!(self, Self::Valid { .. })
    }

    pub fn measurements(&self) -> Option<&[Measurement]> {
        match self {
            Self::Valid { measurements } => Some(measurements),
            Self::CorrectnessMismatch { .. } | Self::InfrastructureFailure { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MetricName {
    WorkloadCpuTime,
    ProcessWallTime,
    PerEditCpuTime,
    Iterations,
    Edits,
    Redisplays,
    OverlayCount,
    LspDiagnosticCount,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MetricUnit {
    Microseconds,
    MicrosecondsPerEdit,
    Count,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Measurement {
    pub name: MetricName,
    pub value: f64,
    pub unit: MetricUnit,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactKind {
    ScenarioResult,
    ScenarioFixture,
    SourceFixture,
    LspReplay,
    PackageStartup,
    TreeSitterGrammar,
    TerminalByteStream,
    Stdout,
    Stderr,
    FrontendLog,
    CompositorLog,
    InputProvenance,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactFile {
    pub kind: ArtifactKind,
    pub path: PathBuf,
}

/// Machine-readable record persisted for every attempted workload run.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunArtifact {
    pub schema_version: u32,
    pub run_id: String,
    pub scenario: ScenarioId,
    pub frontend: Frontend,
    pub editor: PathBuf,
    pub iterations: u32,
    pub started_unix_ms: u128,
    pub total_elapsed_us: u128,
    pub verdict: RunVerdict,
    pub files: Vec<ArtifactFile>,
}

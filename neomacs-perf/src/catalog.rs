use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Stable identity of a committed performance workload.
///
/// A closed enum prevents a typo from selecting a different fixture or
/// silently creating a new time series.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScenarioId {
    RustLspTyping,
}

impl ScenarioId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RustLspTyping => "rust-lsp-typing",
        }
    }
}

impl fmt::Display for ScenarioId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnknownScenarioId(String);

impl fmt::Display for UnknownScenarioId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unknown performance scenario `{}`", self.0)
    }
}

impl std::error::Error for UnknownScenarioId {}

impl FromStr for ScenarioId {
    type Err = UnknownScenarioId;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "rust-lsp-typing" => Ok(Self::RustLspTyping),
            unknown => Err(UnknownScenarioId(unknown.to_string())),
        }
    }
}

/// Display adapter selected for a workload run.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Frontend {
    Batch,
    Tui { rows: u16, columns: u16 },
    Gui { width: u32, height: u32 },
}

/// Immutable definition of one committed performance workload.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScenarioSpec {
    pub id: ScenarioId,
    pub description: &'static str,
    pub default_frontend: Frontend,
}

const SCENARIOS: &[ScenarioSpec] = &[ScenarioSpec {
    id: ScenarioId::RustLspTyping,
    description: "Rust Tree-sitter typing with revision-pinned LSP Mode and deterministic diagnostic replay",
    default_frontend: Frontend::Tui {
        rows: 40,
        columns: 120,
    },
}];

pub fn scenarios() -> &'static [ScenarioSpec] {
    SCENARIOS
}

pub fn scenario(id: ScenarioId) -> Option<&'static ScenarioSpec> {
    SCENARIOS.iter().find(|candidate| candidate.id == id)
}

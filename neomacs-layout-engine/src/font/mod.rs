//! Font selection, metrics, and probing (moved from flat font_* files).

pub mod font_match;
pub mod fontconfig;
pub(crate) mod frame_metrics;
pub mod metrics;
pub mod probe;
pub mod resolver;
pub(crate) mod selection;

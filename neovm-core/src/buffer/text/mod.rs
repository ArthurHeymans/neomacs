pub(in crate::buffer) mod backend;
mod edit;
mod kind;
mod metrics;

pub use edit::{TextEditRange, TextExtent, TextInsertion};
pub use kind::BufferTextBackendKind;
pub use metrics::TextMetrics;

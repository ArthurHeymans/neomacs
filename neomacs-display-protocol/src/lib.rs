//! Shared protocol types between layout, renderer, and runtime crates.

/// Version of the serialized display-protocol surface.
///
/// Stamped into every serialized [`glyph_matrix::FrameDisplayState`] (and the
/// `neomacs--frame-snapshot` JSON document) and validated on deserialization:
/// a snapshot produced by a different protocol version fails to load with an
/// explicit error instead of silently skewing.
///
/// Bump policy: increment on ANY change to the serialized snapshot/golden
/// layout (field added/removed/renamed/re-typed anywhere in the
/// `FrameDisplayState` serde graph, including enum representation changes)
/// or to a `#[repr(C)]` FFI surface (`FaceDataFFI`, `Face` prefix layout).
/// Purely additive `#[serde(default)]` fields still require a bump: older
/// readers would otherwise silently drop them on a round-trip.
pub const PROTOCOL_VERSION: u32 = 1;

pub mod cursor;
pub mod cursor_effect_command;
pub mod effect_config;
pub mod face;
pub mod font;
pub mod frame_glyphs;
pub mod glyph_matrix;
pub mod gradient;
pub mod scene;
pub mod scroll_animation;
pub mod snapshot_text;
pub mod transition_policy;
pub mod types;
pub mod ui_types;
pub use glyph_matrix::*;
pub mod tty_rif;

pub use cursor_effect_command::*;
pub use effect_config::*;
pub use face::*;
pub use frame_glyphs::*;
pub use gradient::*;
pub use scene::*;
pub use scroll_animation::*;
pub use transition_policy::*;
pub use types::*;
pub use ui_types::*;

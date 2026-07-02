//! Shared protocol types between layout, renderer, and runtime crates.

pub mod cursor;
pub mod cursor_effect_command;
pub mod effect_config;
pub mod face;
pub mod frame_glyphs;
pub mod glyph_matrix;
pub mod gradient;
pub mod perf_trace;
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
pub use perf_trace::*;
pub use scene::*;
pub use scroll_animation::*;
pub use transition_policy::*;
pub use types::*;
pub use ui_types::*;

#[cfg(test)]
#[path = "size_budget_test.rs"]
mod size_budget_test;

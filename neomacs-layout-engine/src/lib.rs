//! Rust Display Layout Engine.
//!
//! Replaces the C display engine (xdisp.c) for computing glyph layout.
//! Reads window/buffer state from neovm-core and publishes immutable
//! `FrameDisplayState` snapshots that renderers materialize downstream.

#![allow(unsafe_op_in_unsafe_fn)] // FFI-heavy layout code; migrate to explicit blocks incrementally.

pub mod bidi;
pub mod bidi_layout;
pub mod composition;
pub(crate) mod coords;
pub mod display_backend;
pub(crate) mod display_face_layout;
pub(crate) mod display_item;
pub mod display_iterator;
pub(crate) mod display_media;
pub(crate) mod display_origin;
pub mod display_pixel_calc;
pub(crate) mod display_property;
pub(crate) mod display_row;
pub(crate) mod display_row_append;
pub(crate) mod display_row_builder;
pub(crate) mod display_row_sink;
pub(crate) mod display_source;
pub(crate) mod display_source_resolver;
pub mod display_space;
pub mod display_spec;
pub mod display_status_line;
pub mod engine;
pub mod font_loader;
pub mod font_match;
pub mod font_metrics;
pub mod fontconfig;
pub(crate) mod glyph_advance;
pub mod gui_chrome;
pub mod hit_test;
pub mod matrix_builder;
pub mod mock_frame;
pub mod neovm_bridge;
pub mod tty_menu_bar;
pub mod types;
pub mod unicode;
pub mod window_output;

pub use engine::*;
pub use hit_test::{hit_test_charpos_at_pixel, hit_test_window_charpos};
pub use types::*;

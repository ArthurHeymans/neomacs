//! Rust Display Layout Engine.
//!
//! Replaces the C display engine (xdisp.c) for computing glyph layout.
//! Reads window/buffer state from neovm-core and publishes immutable
//! `FrameDisplayState` snapshots that renderers materialize downstream.

#![allow(unsafe_op_in_unsafe_fn)] // FFI-heavy layout code; migrate to explicit blocks incrementally.

pub mod bidi;
pub mod composition;
pub(crate) mod coords;
pub(crate) mod display_buffer_text_append;
pub(crate) mod display_buffer_text_item_append;
pub(crate) mod display_buffer_text_render;
pub(crate) mod display_buffer_text_source;
pub(crate) mod display_buffer_text_walk;
pub(crate) mod display_cursor;
pub(crate) mod display_face_id;
pub(crate) mod display_face_layout;
pub(crate) mod display_face_policy;
pub(crate) mod display_frame_output;
pub(crate) mod display_item;
pub mod display_iterator;
pub(crate) mod display_media;
pub(crate) mod display_mock_frame;
pub(crate) mod display_origin;
pub mod display_pixel_calc;
pub(crate) mod display_property;
pub(crate) mod display_row;
pub(crate) mod display_row_append_context;
pub(crate) mod display_row_builder;
pub(crate) mod display_row_finalizer;
pub(crate) mod display_row_geometry;
pub(crate) mod display_row_line_number_margin;
pub(crate) mod display_row_lisp_string;
pub(crate) mod display_row_matrix_install;
pub(crate) mod display_row_overlay_string;
pub(crate) mod display_row_replacement;
pub(crate) mod display_row_source_append;
pub(crate) mod display_row_source_render;
pub(crate) mod display_row_special_glyphs;
pub(crate) mod display_row_transition;
pub(crate) mod display_row_walk_state;
pub(crate) mod display_source;
pub(crate) mod display_source_resolver;
pub mod display_space;
pub mod display_spec;
pub mod display_status_line;
pub(crate) mod display_text_run_measurement;
pub mod engine;
pub mod font_loader;
pub mod font_match;
pub mod font_metrics;
pub mod fontconfig;
pub(crate) mod glyph_advance;
pub(crate) mod glyph_row_writer;
pub mod gui_chrome;
pub mod hit_test;
pub(crate) mod matrix_builder;
pub mod mock_frame;
pub mod neovm_bridge;
pub mod tty_menu_bar;
pub mod types;
pub mod unicode;
pub mod window_output;

pub use engine::*;
pub use hit_test::{hit_test_charpos_at_pixel, hit_test_window_charpos};
pub use types::*;

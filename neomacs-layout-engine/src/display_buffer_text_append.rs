//! Buffer-text-window-specific append surface construction.
//!
//! This module holds helpers that translate a buffer text window's geometry
//! and chrome reservation policy into a generic `DisplayRowAppendSurface`.
//! Keeping it separate from `display_row_append.rs` lets the append layer stay
//! source-agnostic while the buffer text walker owns its own setup logic.

use crate::display_row::insert_resolved_display_row_face;
use crate::display_row_append::{DisplayRowAppendArea, DisplayRowAppendSurface};
use crate::display_row_builder::DisplayTabPolicy;
use crate::display_row_special_glyphs::install_last_window_right_border_from_source_requests;
use crate::display_status_line::ChromeRowRenderServices;
use crate::matrix_builder::GlyphMatrixBuilder;
use crate::window_output::{
    TextWindowCursorEffects, TextWindowRightBorder, install_text_window_cursor_effects,
};
use neomacs_display_protocol::effect_config::EffectsConfig;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextWindowAppendSurfaceRequest<'a> {
    content_x: f32,
    text_width: f32,
    line_number_width: f32,
    reserve_right_border_col: bool,
    reserve_right_special_col: bool,
    char_width: f32,
    tab_width: i32,
    tab_stop_list: &'a [i32],
}

impl<'a> TextWindowAppendSurfaceRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        content_x: f32,
        text_width: f32,
        line_number_width: f32,
        reserve_right_border_col: bool,
        reserve_right_special_col: bool,
        char_width: f32,
        tab_width: i32,
        tab_stop_list: &'a [i32],
    ) -> Self {
        Self {
            content_x,
            text_width,
            line_number_width,
            reserve_right_border_col,
            reserve_right_special_col,
            char_width,
            tab_width,
            tab_stop_list,
        }
    }

    fn reserved_width(self) -> f32 {
        let right_border = if self.reserve_right_border_col {
            self.char_width
        } else {
            0.0
        };
        let right_special = if self.reserve_right_special_col {
            self.char_width
        } else {
            0.0
        };
        right_border + right_special
    }

    fn append_width(self) -> f32 {
        (self.text_width - self.line_number_width - self.reserved_width()).max(self.char_width)
    }

    pub(crate) fn into_surface(self) -> DisplayRowAppendSurface {
        DisplayRowAppendSurface::new(
            DisplayRowAppendArea::new(
                self.content_x,
                self.append_width(),
                self.text_width,
                self.line_number_width,
            ),
            DisplayTabPolicy::from_tab_width_and_stops(
                self.content_x,
                self.tab_width,
                self.tab_stop_list,
            ),
        )
    }
}

pub(crate) struct BufferTextWindowCursorEffectsRequest {
    window_id: i64,
    effects: Option<EffectsConfig>,
}

impl BufferTextWindowCursorEffectsRequest {
    pub(crate) fn new(window_id: i64, effects: Option<EffectsConfig>) -> Self {
        Self { window_id, effects }
    }

    pub(crate) fn install_and_apply(self, builder: &mut GlyphMatrixBuilder) -> bool {
        let Some(effects) = self.effects else {
            return false;
        };
        install_text_window_cursor_effects(
            builder,
            TextWindowCursorEffects {
                window_id: self.window_id,
                effects,
            },
        );
        true
    }
}

pub(crate) struct BufferTextWindowTerminalRightBorderRequest {
    ch: char,
    face_name: &'static str,
    char_width: f32,
}

impl BufferTextWindowTerminalRightBorderRequest {
    pub(crate) fn new(char_width: f32) -> Self {
        Self {
            ch: '|',
            face_name: "vertical-border",
            char_width,
        }
    }

    pub(crate) fn install_and_apply(
        self,
        builder: &mut GlyphMatrixBuilder,
        mut render_services: ChromeRowRenderServices<'_, '_>,
    ) -> u32 {
        let border_face = render_services
            .face_resolver()
            .resolve_named_face(self.face_name);
        let border_face_id = border_face.face_id;
        insert_resolved_display_row_face(builder, border_face_id, &border_face, None);
        install_last_window_right_border_from_source_requests(
            builder,
            render_services.reborrow(),
            TextWindowRightBorder {
                ch: self.ch,
                face_id: border_face_id,
                char_width: self.char_width,
            },
            &border_face,
        );
        border_face_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_window_append_surface_request_reserves_right_columns() {
        let tab_stops = vec![4, 12];
        let surface =
            TextWindowAppendSurfaceRequest::new(20.0, 200.0, 16.0, true, true, 8.0, 6, &tab_stops)
                .into_surface();

        assert_eq!(surface.content_x(), 20.0);
        assert_eq!(surface.right_edge(), 188.0);
        assert_eq!(surface.full_text_right_edge(), 204.0);
    }
}

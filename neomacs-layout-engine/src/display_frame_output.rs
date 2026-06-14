use crate::matrix_builder::GlyphMatrixBuilder;
use crate::types::WindowParams;
use neomacs_display_protocol::frame_glyphs::{GlyphRowRole, WindowInfo};
use neomacs_display_protocol::glyph_matrix::ScrollBarItem;
use neomacs_display_protocol::types::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WindowScrollBarMetrics {
    pub(crate) position: i64,
    pub(crate) portion: i64,
    pub(crate) whole: i64,
    pub(crate) thumb_start: f32,
    pub(crate) thumb_size: f32,
}

pub(crate) struct WindowScrollBarsRenderRequest<'a> {
    params: &'a WindowParams,
    info: &'a WindowInfo,
}

impl<'a> WindowScrollBarsRenderRequest<'a> {
    pub(crate) fn new(params: &'a WindowParams, info: &'a WindowInfo) -> Self {
        Self { params, info }
    }

    pub(crate) fn render_and_apply(self, builder: &mut GlyphMatrixBuilder) {
        let track_color = Color::new(0.7, 0.7, 0.7, 1.0);
        let thumb_color = Color::new(0.5, 0.5, 0.5, 1.0);
        let chrome_top = self.params.header_line_height + self.params.tab_line_height;
        let chrome_bottom = self.params.mode_line_height + self.params.scroll_bar_pixel_height;

        if let Some(ref side) = self.params.vertical_scroll_bar_side {
            let track_height = (self.params.bounds.height - chrome_top - chrome_bottom).max(0.0);
            if track_height <= 0.0 {
                return;
            }
            let track_width = self.params.scroll_bar_pixel_width;

            let x = if side == "left" {
                self.params.bounds.x
            } else {
                self.params.bounds.x + self.params.bounds.width - track_width
            };
            let y = self.params.bounds.y + chrome_top;

            let accessible_start = self.params.accessible_start_charpos().get();
            let accessible_end = self.params.accessible_end_charpos().get();
            let metrics = WindowScrollBarMetrics::vertical(
                self.info.window_start,
                self.info.window_end,
                accessible_start,
                accessible_end,
                track_height,
            );

            builder.push_scroll_bar(ScrollBarItem {
                window_id: self.params.window_id,
                row_role: GlyphRowRole::Text,
                clip_rect: Some(self.params.bounds),
                horizontal: false,
                x,
                y,
                width: track_width,
                height: track_height,
                position: metrics.position,
                portion: metrics.portion,
                whole: metrics.whole,
                thumb_start: metrics.thumb_start,
                thumb_size: metrics.thumb_size,
                track_color,
                thumb_color,
            });
        }

        if self.params.horizontal_scroll_bar {
            let track_width = self.params.bounds.width;
            let track_height = self.params.scroll_bar_pixel_height;
            let x = self.params.bounds.x;
            let y = self.params.bounds.y + self.params.bounds.height
                - self.params.mode_line_height
                - self.params.scroll_bar_pixel_height;

            let hscroll_px = self.params.hscroll as f32 * self.params.char_width;
            let visible_px = self.params.text_bounds.width.max(1.0);
            let thumb_size = if track_width > 0.0 {
                (visible_px / (visible_px + hscroll_px + track_width)) * track_width
            } else {
                track_width
            }
            .clamp(8.0, track_width);
            let thumb_start = if track_width > 0.0 && hscroll_px + visible_px > 0.0 {
                (hscroll_px / (hscroll_px + visible_px)) * (track_width - thumb_size)
            } else {
                0.0
            };

            builder.push_scroll_bar(ScrollBarItem {
                window_id: self.params.window_id,
                row_role: GlyphRowRole::Text,
                clip_rect: Some(self.params.bounds),
                horizontal: true,
                x,
                y,
                width: track_width,
                height: track_height,
                position: self.params.hscroll as i64,
                portion: visible_px.round().max(1.0) as i64,
                whole: (visible_px + hscroll_px).round().max(1.0) as i64,
                thumb_start,
                thumb_size,
                track_color,
                thumb_color,
            });
        }
    }
}

impl WindowScrollBarMetrics {
    /// Mirrors GNU `set_vertical_scroll_bar` (xdisp.c): whole = ZV - BEGV,
    /// start = window_start - BEGV, end = Z - window_end_pos - BEGV.
    pub(crate) fn vertical(
        window_start: i64,
        window_end: i64,
        buffer_begv: i64,
        buffer_size: i64,
        track_height: f32,
    ) -> Self {
        let whole = (buffer_size - buffer_begv).max(1);
        let position = (window_start - 1 - buffer_begv).max(0);
        let end = if window_end > 0 {
            (window_end - 1 - buffer_begv).max(position)
        } else {
            position
        };
        let portion = (end - position).max(1);
        let effective_whole = whole.max(portion);

        let thumb_start = (position as f32 / effective_whole as f32) * track_height;
        let thumb_size = (portion as f32 / effective_whole as f32) * track_height;
        let min_thumb = 20.0f32.min(track_height * 0.2);
        let thumb_size = thumb_size.max(min_thumb).min(track_height);
        let thumb_start = thumb_start
            .max(0.0)
            .min((track_height - thumb_size).max(0.0));

        Self {
            position,
            portion,
            whole: effective_whole,
            thumb_start,
            thumb_size,
        }
    }
}

#[cfg(test)]
#[path = "display_frame_output_test.rs"]
mod tests;

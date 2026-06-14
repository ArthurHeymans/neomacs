use crate::display_row_append::BufferTextWindowTerminalRightBorderRequest;
use crate::matrix_builder::GlyphMatrixBuilder;
use crate::neovm_bridge::FaceResolver;
use crate::types::{FrameParams, WindowParams};
use neomacs_display_protocol::frame_glyphs::{
    FrameGlyphBuffer, GlyphRowRole, WindowEffectHint, WindowInfo,
};
use neomacs_display_protocol::glyph_matrix::ScrollBarItem;
use neomacs_display_protocol::types::{Color, Rect};
use std::collections::HashMap;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct WindowFrameMetadata {
    pub(crate) buffer_file_name: String,
    pub(crate) modified: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WindowFrameGeometry {
    pub(crate) right_edge: f32,
    pub(crate) bottom_edge: f32,
    pub(crate) is_rightmost: bool,
    pub(crate) is_bottommost: bool,
    pub(crate) reserve_terminal_right_border_col: bool,
}

pub(crate) struct WindowFrameGeometryRequest<'a> {
    params: &'a WindowParams,
    frame_params: &'a FrameParams,
    main_area_bottom: f32,
}

impl<'a> WindowFrameGeometryRequest<'a> {
    pub(crate) fn new(
        params: &'a WindowParams,
        frame_params: &'a FrameParams,
        main_area_bottom: f32,
    ) -> Self {
        Self {
            params,
            frame_params,
            main_area_bottom,
        }
    }

    pub(crate) fn resolve(self) -> WindowFrameGeometry {
        let right_edge = self.params.bounds.x + self.params.bounds.width;
        let bottom_edge = self.params.bounds.y + self.params.bounds.height;
        let is_rightmost = right_edge >= self.frame_params.width - 1.0;
        let is_bottommost = self.params.is_minibuffer || bottom_edge >= self.main_area_bottom - 1.0;
        let reserve_terminal_right_border_col = !self.frame_params.window_system
            && self.frame_params.right_divider_width == 0
            && !is_rightmost
            && !self.params.is_minibuffer;

        WindowFrameGeometry {
            right_edge,
            bottom_edge,
            is_rightmost,
            is_bottommost,
            reserve_terminal_right_border_col,
        }
    }
}

pub(crate) struct WindowFrameInfoRenderRequest<'a> {
    params: &'a WindowParams,
    metadata: WindowFrameMetadata,
}

impl<'a> WindowFrameInfoRenderRequest<'a> {
    pub(crate) fn new(params: &'a WindowParams, metadata: WindowFrameMetadata) -> Self {
        Self { params, metadata }
    }

    pub(crate) fn render_and_apply(self, builder: &mut GlyphMatrixBuilder) {
        builder.push_background(
            self.params.bounds,
            Color::from_pixel(self.params.default_bg),
        );
        builder.push_window_info(WindowInfo {
            window_id: self.params.window_id,
            buffer_id: self.params.buffer_id,
            window_start: self.params.window_start,
            window_end: 0,
            buffer_size: self.params.buffer_size,
            bounds: Rect::new(
                self.params.bounds.x,
                self.params.bounds.y,
                self.params.bounds.width,
                self.params.bounds.height,
            ),
            mode_line_height: self.params.mode_line_height,
            header_line_height: self.params.header_line_height,
            tab_line_height: self.params.tab_line_height,
            selected: self.params.selected,
            is_minibuffer: self.params.is_minibuffer,
            char_height: self.params.char_height,
            buffer_file_name: self.metadata.buffer_file_name,
            modified: self.metadata.modified,
        });
    }
}

pub(crate) struct WindowFrameInfoEffectsRenderRequest<'a> {
    prev_window_infos: &'a HashMap<i64, WindowInfo>,
}

impl<'a> WindowFrameInfoEffectsRenderRequest<'a> {
    pub(crate) fn new(prev_window_infos: &'a HashMap<i64, WindowInfo>) -> Self {
        Self { prev_window_infos }
    }

    pub(crate) fn render_latest_and_apply(
        self,
        builder: &mut GlyphMatrixBuilder,
        curr_window_infos: &mut HashMap<i64, WindowInfo>,
    ) {
        let Some(curr) = builder.window_infos().last().cloned() else {
            return;
        };
        self.record_transition_hint(builder, &curr);
        self.record_effect_hints(builder, &curr);
        curr_window_infos.insert(curr.window_id, curr);
    }

    fn record_transition_hint(&self, builder: &mut GlyphMatrixBuilder, curr: &WindowInfo) {
        let Some(prev) = self.prev_window_infos.get(&curr.window_id) else {
            return;
        };
        if let Some(hint) = FrameGlyphBuffer::derive_transition_hint(prev, curr) {
            builder.push_transition_hint(hint);
        }
    }

    fn record_effect_hints(&self, builder: &mut GlyphMatrixBuilder, curr: &WindowInfo) {
        if curr.is_minibuffer {
            return;
        }

        let Some(prev) = self.prev_window_infos.get(&curr.window_id) else {
            return;
        };
        if prev.buffer_id == 0 || curr.buffer_id == 0 {
            return;
        }

        if prev.buffer_id != curr.buffer_id {
            builder.push_effect_hint(WindowEffectHint::TextFadeIn {
                window_id: curr.window_id,
                bounds: curr.bounds,
            });
            return;
        }

        if prev.window_start == curr.window_start {
            return;
        }

        let direction = if curr.window_start > prev.window_start {
            1
        } else {
            -1
        };
        let delta = (curr.window_start - prev.window_start).unsigned_abs() as f32;
        builder.push_effect_hint(WindowEffectHint::TextFadeIn {
            window_id: curr.window_id,
            bounds: curr.bounds,
        });
        builder.push_effect_hint(WindowEffectHint::ScrollLineSpacing {
            window_id: curr.window_id,
            bounds: curr.bounds,
            direction,
        });
        builder.push_effect_hint(WindowEffectHint::ScrollMomentum {
            window_id: curr.window_id,
            bounds: curr.bounds,
            direction,
        });
        builder.push_effect_hint(WindowEffectHint::ScrollVelocityFade {
            window_id: curr.window_id,
            bounds: curr.bounds,
            delta,
        });
    }
}

pub(crate) struct WindowFrameDecorationsRenderRequest<'a> {
    params: &'a WindowParams,
    frame_params: &'a FrameParams,
    geometry: WindowFrameGeometry,
    info: &'a WindowInfo,
    face_resolver: &'a FaceResolver,
}

impl<'a> WindowFrameDecorationsRenderRequest<'a> {
    pub(crate) fn new(
        params: &'a WindowParams,
        frame_params: &'a FrameParams,
        geometry: WindowFrameGeometry,
        info: &'a WindowInfo,
        face_resolver: &'a FaceResolver,
    ) -> Self {
        Self {
            params,
            frame_params,
            geometry,
            info,
            face_resolver,
        }
    }

    pub(crate) fn render_and_apply(self, builder: &mut GlyphMatrixBuilder) {
        WindowScrollBarsRenderRequest::new(self.params, self.info).render_and_apply(builder);
        self.render_right_divider(builder);
        self.render_bottom_divider(builder);
    }

    fn render_right_divider(&self, builder: &mut GlyphMatrixBuilder) {
        if self.params.is_minibuffer || self.geometry.is_rightmost {
            return;
        }

        if self.frame_params.right_divider_width > 0 {
            let width = self.frame_params.right_divider_width as f32;
            let height = self.params.bounds.height
                - if self.frame_params.bottom_divider_width > 0 && !self.geometry.is_bottommost {
                    self.frame_params.bottom_divider_width as f32
                } else {
                    0.0
                };
            WindowDividerRectsRenderRequest::new(
                self.params.window_id,
                self.geometry.right_edge - width,
                self.params.bounds.y,
                width,
                height.max(0.0),
                WindowDividerOrientation::Vertical,
                self.frame_params,
            )
            .render_and_apply(builder);
            return;
        }

        if self.frame_params.window_system {
            builder.push_border(
                self.params.window_id,
                self.geometry.right_edge - 1.0,
                self.params.bounds.y,
                1.0,
                self.params.bounds.height.max(0.0),
                Color::from_pixel(self.frame_params.vertical_border_fg),
            );
        } else {
            BufferTextWindowTerminalRightBorderRequest::new(self.frame_params.char_width)
                .install_and_apply(builder, self.face_resolver);
        }
    }

    fn render_bottom_divider(&self, builder: &mut GlyphMatrixBuilder) {
        if self.params.is_minibuffer
            || self.geometry.is_bottommost
            || self.frame_params.bottom_divider_width <= 0
        {
            return;
        }

        let height = self.frame_params.bottom_divider_width as f32;
        let width = self.params.bounds.width
            - if self.frame_params.right_divider_width > 0 && !self.geometry.is_rightmost {
                self.frame_params.right_divider_width as f32
            } else {
                0.0
            };
        WindowDividerRectsRenderRequest::new(
            self.params.window_id,
            self.params.bounds.x,
            self.geometry.bottom_edge - height,
            width.max(0.0),
            height,
            WindowDividerOrientation::Horizontal,
            self.frame_params,
        )
        .render_and_apply(builder);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WindowDividerOrientation {
    Horizontal,
    Vertical,
}

struct WindowDividerRectsRenderRequest<'a> {
    window_id: i64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    orientation: WindowDividerOrientation,
    frame_params: &'a FrameParams,
}

impl<'a> WindowDividerRectsRenderRequest<'a> {
    fn new(
        window_id: i64,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        orientation: WindowDividerOrientation,
        frame_params: &'a FrameParams,
    ) -> Self {
        Self {
            window_id,
            x,
            y,
            width,
            height,
            orientation,
            frame_params,
        }
    }

    fn render_and_apply(self, builder: &mut GlyphMatrixBuilder) {
        if self.width <= 0.0 || self.height <= 0.0 {
            return;
        }

        let inner = Color::from_pixel(self.frame_params.divider_fg);
        if self.primary_size() < 3.0 {
            builder.push_border(
                self.window_id,
                self.x,
                self.y,
                self.width,
                self.height,
                inner,
            );
            return;
        }

        let first = Color::from_pixel(self.frame_params.divider_first_fg);
        let last = Color::from_pixel(self.frame_params.divider_last_fg);
        match self.orientation {
            WindowDividerOrientation::Vertical => {
                builder.push_border(self.window_id, self.x, self.y, 1.0, self.height, first);
                builder.push_border(
                    self.window_id,
                    self.x + 1.0,
                    self.y,
                    (self.width - 2.0).max(0.0),
                    self.height,
                    inner,
                );
                builder.push_border(
                    self.window_id,
                    self.x + self.width - 1.0,
                    self.y,
                    1.0,
                    self.height,
                    last,
                );
            }
            WindowDividerOrientation::Horizontal => {
                builder.push_border(self.window_id, self.x, self.y, self.width, 1.0, first);
                builder.push_border(
                    self.window_id,
                    self.x,
                    self.y + 1.0,
                    self.width,
                    (self.height - 2.0).max(0.0),
                    inner,
                );
                builder.push_border(
                    self.window_id,
                    self.x,
                    self.y + self.height - 1.0,
                    self.width,
                    1.0,
                    last,
                );
            }
        }
    }

    fn primary_size(&self) -> f32 {
        match self.orientation {
            WindowDividerOrientation::Horizontal => self.height,
            WindowDividerOrientation::Vertical => self.width,
        }
    }
}

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

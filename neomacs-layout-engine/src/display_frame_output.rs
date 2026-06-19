use crate::display_buffer_text_append::BufferTextWindowTerminalRightBorderRequest;
use crate::display_row::insert_resolved_display_row_face;
use crate::display_status_line::ChromeRowRenderServices;
use crate::font_metrics::FontMetrics;
use crate::matrix_builder::{
    GlyphMatrixBuilder, MatrixFrameArtifactInstallRequest, MatrixFrameIdentityInstallRequest,
    MatrixFrameStateInstallRequest,
};
use crate::neovm_bridge::ResolvedFace;
use crate::types::{FrameParams, WindowParams};
use crate::window_output::TextWindowOutputRenderState;
use neomacs_display_protocol::frame_glyphs::{
    FrameGlyphBuffer, GlyphRowRole, PhysCursor, WindowEffectHint, WindowInfo, WindowTransitionHint,
    WindowTransitionKind,
};
use neomacs_display_protocol::glyph_matrix::{CursorItem, ScrollBarItem};
use neomacs_display_protocol::types::{Color, Rect};
use std::collections::{HashMap, HashSet};

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FrameOutputIdentity {
    pub(crate) frame_id: u64,
    pub(crate) parent_id: u64,
    pub(crate) parent_x: f32,
    pub(crate) parent_y: f32,
    pub(crate) z_order: i32,
    pub(crate) undecorated: bool,
    pub(crate) border_width: f32,
    pub(crate) border_color: Color,
    pub(crate) background_alpha: f32,
    pub(crate) no_accept_focus: bool,
}

impl From<FrameOutputIdentity> for MatrixFrameIdentityInstallRequest {
    fn from(identity: FrameOutputIdentity) -> Self {
        Self {
            frame_id: identity.frame_id,
            parent_id: identity.parent_id,
            parent_x: identity.parent_x,
            parent_y: identity.parent_y,
            z_order: identity.z_order,
            undecorated: identity.undecorated,
            border_width: identity.border_width,
            border_color: identity.border_color,
            background_alpha: identity.background_alpha,
            no_accept_focus: identity.no_accept_focus,
        }
    }
}

pub(crate) struct FrameOutputRenderState<'a> {
    builder: &'a mut GlyphMatrixBuilder,
}

impl<'a> FrameOutputRenderState<'a> {
    pub(crate) fn new(builder: &'a mut GlyphMatrixBuilder) -> Self {
        Self { builder }
    }

    fn builder_mut(&mut self) -> &mut GlyphMatrixBuilder {
        self.builder
    }

    fn install_frame_state(&mut self, request: MatrixFrameStateInstallRequest) {
        self.builder.install_frame_state(request);
    }

    fn install_frame_artifact(&mut self, request: MatrixFrameArtifactInstallRequest) {
        self.builder.install_frame_artifact(request);
    }

    fn window_infos(&self) -> &[WindowInfo] {
        self.builder.window_infos()
    }

    fn transition_hints(&self) -> &[WindowTransitionHint] {
        self.builder.transition_hints()
    }

    fn background_color(&self) -> Color {
        *self.builder.background_color()
    }

    fn phys_cursor(&self) -> Option<&PhysCursor> {
        self.builder.phys_cursor()
    }

    fn cursors(&self) -> &[CursorItem] {
        self.builder.cursors()
    }
}

pub(crate) struct FrameOutputStateRenderRequest<'a> {
    identity: Option<FrameOutputIdentity>,
    background_color: Color,
    font_pixel_size: f32,
    default_face: &'a ResolvedFace,
    default_metrics: Option<FontMetrics>,
}

impl<'a> FrameOutputStateRenderRequest<'a> {
    pub(crate) fn new(
        identity: Option<FrameOutputIdentity>,
        background_color: Color,
        font_pixel_size: f32,
        default_face: &'a ResolvedFace,
        default_metrics: Option<FontMetrics>,
    ) -> Self {
        Self {
            identity,
            background_color,
            font_pixel_size,
            default_face,
            default_metrics,
        }
    }

    pub(crate) fn render_and_apply(self, state: &mut FrameOutputRenderState<'_>) {
        if let Some(identity) = self.identity {
            state.install_frame_state(MatrixFrameStateInstallRequest::Identity(identity.into()));
        }
        state.install_frame_state(MatrixFrameStateInstallRequest::BackgroundColor(
            self.background_color,
        ));
        state.install_frame_state(MatrixFrameStateInstallRequest::FontPixelSize(
            self.font_pixel_size,
        ));
        insert_resolved_display_row_face(
            state.builder_mut(),
            0,
            self.default_face,
            self.default_metrics,
        );
    }
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
        let is_bottommost =
            self.params.is_minibuffer() || bottom_edge >= self.main_area_bottom - 1.0;
        let reserve_terminal_right_border_col = !self.frame_params.window_system
            && self.frame_params.right_divider_width == 0
            && !is_rightmost
            && !self.params.is_minibuffer();

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

    pub(crate) fn render_and_apply(self, state: &mut FrameOutputRenderState<'_>) {
        state.install_frame_artifact(MatrixFrameArtifactInstallRequest::Background {
            bounds: self.params.bounds,
            color: Color::from_pixel(self.params.default_bg),
        });
        state.install_frame_artifact(MatrixFrameArtifactInstallRequest::WindowInfo(WindowInfo {
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
            is_minibuffer: self.params.is_minibuffer(),
            char_height: self.params.char_height,
            buffer_file_name: self.metadata.buffer_file_name,
            modified: self.metadata.modified,
        }));
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
        state: &mut FrameOutputRenderState<'_>,
        curr_window_infos: &mut HashMap<i64, WindowInfo>,
    ) {
        let Some(curr) = state.window_infos().last().cloned() else {
            return;
        };
        self.record_transition_hint(state, &curr);
        self.record_effect_hints(state, &curr);
        curr_window_infos.insert(curr.window_id, curr);
    }

    fn record_transition_hint(&self, state: &mut FrameOutputRenderState<'_>, curr: &WindowInfo) {
        let Some(prev) = self.prev_window_infos.get(&curr.window_id) else {
            return;
        };
        if let Some(hint) = FrameGlyphBuffer::derive_transition_hint(prev, curr) {
            state.install_frame_artifact(MatrixFrameArtifactInstallRequest::TransitionHint(hint));
        }
    }

    fn record_effect_hints(&self, state: &mut FrameOutputRenderState<'_>, curr: &WindowInfo) {
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
            state.install_frame_artifact(MatrixFrameArtifactInstallRequest::EffectHint(
                WindowEffectHint::TextFadeIn {
                    window_id: curr.window_id,
                    bounds: curr.bounds,
                },
            ));
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
        state.install_frame_artifact(MatrixFrameArtifactInstallRequest::EffectHint(
            WindowEffectHint::TextFadeIn {
                window_id: curr.window_id,
                bounds: curr.bounds,
            },
        ));
        state.install_frame_artifact(MatrixFrameArtifactInstallRequest::EffectHint(
            WindowEffectHint::ScrollLineSpacing {
                window_id: curr.window_id,
                bounds: curr.bounds,
                direction,
            },
        ));
        state.install_frame_artifact(MatrixFrameArtifactInstallRequest::EffectHint(
            WindowEffectHint::ScrollMomentum {
                window_id: curr.window_id,
                bounds: curr.bounds,
                direction,
            },
        ));
        state.install_frame_artifact(MatrixFrameArtifactInstallRequest::EffectHint(
            WindowEffectHint::ScrollVelocityFade {
                window_id: curr.window_id,
                bounds: curr.bounds,
                delta,
            },
        ));
    }
}

pub(crate) struct FrameLineAnimationHintsRenderRequest<'a> {
    prev_window_infos: &'a HashMap<i64, WindowInfo>,
    curr_window_infos: &'a HashMap<i64, WindowInfo>,
}

impl<'a> FrameLineAnimationHintsRenderRequest<'a> {
    pub(crate) fn new(
        prev_window_infos: &'a HashMap<i64, WindowInfo>,
        curr_window_infos: &'a HashMap<i64, WindowInfo>,
    ) -> Self {
        Self {
            prev_window_infos,
            curr_window_infos,
        }
    }

    pub(crate) fn render_and_apply(self, state: &mut FrameOutputRenderState<'_>) {
        for (window_id, curr) in self.curr_window_infos {
            if curr.is_minibuffer {
                continue;
            }
            let Some(prev) = self.prev_window_infos.get(window_id) else {
                continue;
            };
            if prev.buffer_id == 0 || curr.buffer_id == 0 {
                continue;
            }
            if prev.buffer_id != curr.buffer_id
                || prev.window_start != curr.window_start
                || prev.buffer_size == curr.buffer_size
            {
                continue;
            }

            if let Some(edit_y) = find_window_cursor_y_in_state(state, curr) {
                let offset = if curr.buffer_size > prev.buffer_size {
                    -curr.char_height
                } else {
                    curr.char_height
                };
                state.install_frame_artifact(MatrixFrameArtifactInstallRequest::EffectHint(
                    WindowEffectHint::LineAnimation {
                        window_id: curr.window_id,
                        bounds: curr.bounds,
                        edit_y: edit_y + curr.char_height,
                        offset,
                    },
                ));
            }
        }
    }
}

pub(crate) struct FrameWindowSwitchHintRenderRequest<'a> {
    prev_selected_window_id: &'a mut i64,
}

impl<'a> FrameWindowSwitchHintRenderRequest<'a> {
    pub(crate) fn new(prev_selected_window_id: &'a mut i64) -> Self {
        Self {
            prev_selected_window_id,
        }
    }

    pub(crate) fn render_and_apply(self, state: &mut FrameOutputRenderState<'_>) {
        let new_selected = state
            .window_infos()
            .iter()
            .find(|info| info.selected && !info.is_minibuffer)
            .map(|info| (info.window_id, info.bounds));
        if let Some((window_id, bounds)) = new_selected {
            if *self.prev_selected_window_id != 0 && *self.prev_selected_window_id != window_id {
                state.install_frame_artifact(MatrixFrameArtifactInstallRequest::EffectHint(
                    WindowEffectHint::WindowSwitchFade { window_id, bounds },
                ));
            }
            *self.prev_selected_window_id = window_id;
        }
    }
}

pub(crate) struct FrameThemeTransitionHintRenderRequest<'a> {
    prev_background: &'a mut Option<(f32, f32, f32, f32)>,
    frame_width: f32,
    frame_height: f32,
}

impl<'a> FrameThemeTransitionHintRenderRequest<'a> {
    pub(crate) fn new(
        prev_background: &'a mut Option<(f32, f32, f32, f32)>,
        frame_width: f32,
        frame_height: f32,
    ) -> Self {
        Self {
            prev_background,
            frame_width,
            frame_height,
        }
    }

    pub(crate) fn render_and_apply(self, state: &mut FrameOutputRenderState<'_>) {
        let bg = state.background_color();
        let new_bg = (bg.r, bg.g, bg.b, bg.a);
        if let Some(old_bg) = *self.prev_background
            && color_changed_for_theme_transition(old_bg, new_bg)
        {
            let full_h = frame_content_height_before_minibuffer(state, self.frame_height);
            state.install_frame_artifact(MatrixFrameArtifactInstallRequest::EffectHint(
                WindowEffectHint::ThemeTransition {
                    bounds: Rect::new(0.0, 0.0, self.frame_width, full_h),
                },
            ));
        }
        *self.prev_background = Some(new_bg);
    }
}

pub(crate) struct FrameTopologyTransitionHintRenderRequest<'a> {
    prev_window_infos: &'a HashMap<i64, WindowInfo>,
    curr_window_infos: &'a HashMap<i64, WindowInfo>,
    frame_width: f32,
    frame_height: f32,
}

impl<'a> FrameTopologyTransitionHintRenderRequest<'a> {
    pub(crate) fn new(
        prev_window_infos: &'a HashMap<i64, WindowInfo>,
        curr_window_infos: &'a HashMap<i64, WindowInfo>,
        frame_width: f32,
        frame_height: f32,
    ) -> Self {
        Self {
            prev_window_infos,
            curr_window_infos,
            frame_width,
            frame_height,
        }
    }

    pub(crate) fn render_and_apply(self, state: &mut FrameOutputRenderState<'_>) {
        if self.prev_window_infos.is_empty() {
            return;
        }

        let prev_non_mini = non_minibuffer_window_ids(self.prev_window_infos);
        let curr_non_mini = non_minibuffer_window_ids(self.curr_window_infos);

        if prev_non_mini.is_empty()
            || curr_non_mini.is_empty()
            || prev_non_mini == curr_non_mini
            || state.transition_hints().iter().any(|hint| {
                hint.window_id == 0 && matches!(hint.kind, WindowTransitionKind::Crossfade)
            })
        {
            return;
        }

        let full_h = frame_content_height_before_minibuffer(state, self.frame_height);
        state.install_frame_artifact(MatrixFrameArtifactInstallRequest::TransitionHint(
            WindowTransitionHint {
                window_id: 0,
                bounds: Rect::new(0.0, 0.0, self.frame_width, full_h),
                kind: WindowTransitionKind::Crossfade,
                effect: None,
                easing: None,
            },
        ));
    }
}

fn find_window_cursor_y_in_state(
    state: &FrameOutputRenderState<'_>,
    info: &WindowInfo,
) -> Option<f32> {
    let in_window = |x: f32, y: f32, hollow: bool| -> bool {
        !hollow
            && x >= info.bounds.x
            && x < info.bounds.x + info.bounds.width
            && y >= info.bounds.y
            && y < info.bounds.y + info.bounds.height
    };
    if let Some(phys) = state.phys_cursor()
        && in_window(phys.x, phys.y, phys.style.is_hollow())
    {
        return Some(phys.y);
    }
    for cursor in state.cursors() {
        if in_window(cursor.x, cursor.y, cursor.style.is_hollow()) {
            return Some(cursor.y);
        }
    }
    None
}

fn color_changed_for_theme_transition(
    old_bg: (f32, f32, f32, f32),
    new_bg: (f32, f32, f32, f32),
) -> bool {
    (new_bg.0 - old_bg.0).abs() > 0.02
        || (new_bg.1 - old_bg.1).abs() > 0.02
        || (new_bg.2 - old_bg.2).abs() > 0.02
}

fn frame_content_height_before_minibuffer(
    state: &FrameOutputRenderState<'_>,
    frame_height: f32,
) -> f32 {
    state
        .window_infos()
        .iter()
        .find(|w| w.is_minibuffer)
        .map_or(frame_height, |w| w.bounds.y)
}

fn non_minibuffer_window_ids(window_infos: &HashMap<i64, WindowInfo>) -> HashSet<i64> {
    window_infos
        .iter()
        .filter(|(_, info)| !info.is_minibuffer)
        .map(|(window_id, _)| *window_id)
        .collect()
}

pub(crate) struct WindowFrameDecorationsRenderRequest<'a> {
    params: &'a WindowParams,
    frame_params: &'a FrameParams,
    geometry: WindowFrameGeometry,
    info: &'a WindowInfo,
}

impl<'a> WindowFrameDecorationsRenderRequest<'a> {
    pub(crate) fn new(
        params: &'a WindowParams,
        frame_params: &'a FrameParams,
        geometry: WindowFrameGeometry,
        info: &'a WindowInfo,
    ) -> Self {
        Self {
            params,
            frame_params,
            geometry,
            info,
        }
    }

    pub(crate) fn render_and_apply(
        self,
        state: &mut FrameOutputRenderState<'_>,
        mut render_services: ChromeRowRenderServices<'_, '_>,
    ) {
        WindowScrollBarsRenderRequest::new(self.params, self.info).render_and_apply(state);
        self.render_right_divider(state, render_services.reborrow());
        self.render_bottom_divider(state);
    }

    fn render_right_divider(
        &self,
        state: &mut FrameOutputRenderState<'_>,
        render_services: ChromeRowRenderServices<'_, '_>,
    ) {
        if self.params.is_minibuffer() || self.geometry.is_rightmost {
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
            .render_and_apply(state);
            return;
        }

        if self.frame_params.window_system {
            state.install_frame_artifact(MatrixFrameArtifactInstallRequest::Border {
                window_id: self.params.window_id,
                x: self.geometry.right_edge - 1.0,
                y: self.params.bounds.y,
                width: 1.0,
                height: self.params.bounds.height.max(0.0),
                color: Color::from_pixel(self.frame_params.vertical_border_fg),
            });
        } else {
            BufferTextWindowTerminalRightBorderRequest::new(self.frame_params.char_width)
                .install_and_apply(
                    &mut TextWindowOutputRenderState::without_output(state.builder_mut()),
                    render_services,
                );
        }
    }

    fn render_bottom_divider(&self, state: &mut FrameOutputRenderState<'_>) {
        if self.params.is_minibuffer()
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
        .render_and_apply(state);
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

    fn render_and_apply(self, state: &mut FrameOutputRenderState<'_>) {
        if self.width <= 0.0 || self.height <= 0.0 {
            return;
        }

        let inner = Color::from_pixel(self.frame_params.divider_fg);
        if self.primary_size() < 3.0 {
            state.install_frame_artifact(MatrixFrameArtifactInstallRequest::Border {
                window_id: self.window_id,
                x: self.x,
                y: self.y,
                width: self.width,
                height: self.height,
                color: inner,
            });
            return;
        }

        let first = Color::from_pixel(self.frame_params.divider_first_fg);
        let last = Color::from_pixel(self.frame_params.divider_last_fg);
        match self.orientation {
            WindowDividerOrientation::Vertical => {
                state.install_frame_artifact(MatrixFrameArtifactInstallRequest::Border {
                    window_id: self.window_id,
                    x: self.x,
                    y: self.y,
                    width: 1.0,
                    height: self.height,
                    color: first,
                });
                state.install_frame_artifact(MatrixFrameArtifactInstallRequest::Border {
                    window_id: self.window_id,
                    x: self.x + 1.0,
                    y: self.y,
                    width: (self.width - 2.0).max(0.0),
                    height: self.height,
                    color: inner,
                });
                state.install_frame_artifact(MatrixFrameArtifactInstallRequest::Border {
                    window_id: self.window_id,
                    x: self.x + self.width - 1.0,
                    y: self.y,
                    width: 1.0,
                    height: self.height,
                    color: last,
                });
            }
            WindowDividerOrientation::Horizontal => {
                state.install_frame_artifact(MatrixFrameArtifactInstallRequest::Border {
                    window_id: self.window_id,
                    x: self.x,
                    y: self.y,
                    width: self.width,
                    height: 1.0,
                    color: first,
                });
                state.install_frame_artifact(MatrixFrameArtifactInstallRequest::Border {
                    window_id: self.window_id,
                    x: self.x,
                    y: self.y + 1.0,
                    width: self.width,
                    height: (self.height - 2.0).max(0.0),
                    color: inner,
                });
                state.install_frame_artifact(MatrixFrameArtifactInstallRequest::Border {
                    window_id: self.window_id,
                    x: self.x,
                    y: self.y + self.height - 1.0,
                    width: self.width,
                    height: 1.0,
                    color: last,
                });
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

    pub(crate) fn render_and_apply(self, state: &mut FrameOutputRenderState<'_>) {
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

            state.install_frame_artifact(MatrixFrameArtifactInstallRequest::ScrollBar(
                ScrollBarItem {
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
                },
            ));
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

            state.install_frame_artifact(MatrixFrameArtifactInstallRequest::ScrollBar(
                ScrollBarItem {
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
                },
            ));
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

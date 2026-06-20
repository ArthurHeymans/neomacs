use crate::composition::last_text_cluster_tail_in_row;
use crate::display_output_builder::{
    DisplayOutputBuilder, FRAME_CHROME_WINDOW_ID, OutputCurrentRowDecorationRequest,
    OutputCursorInstallRequest, OutputFrameArtifactInstallRequest, OutputFrameStateInstallRequest,
    OutputMediaInstallRequest, OutputRetryCheckpointRestoreRequest,
    OutputRowDecorationInstallRequest, OutputRowDecorator, OutputRowLifecycleRequest,
    OutputTextWindowDisplayRangeInstallRequest, ResolvedOutputMediaInstallTarget,
};
#[cfg(test)]
use crate::display_row::display_row_output_end_position;
use crate::display_row::{
    DisplayRowOwner, FrameChromeKind, MeasuredDisplayRow, RenderedDisplayRow,
    RenderedDisplayRowMedia, RenderedDisplayRowMediaKind, WindowChromeKind,
};
use crate::display_row_builder::apply_display_row_source_slot_bounds;
#[cfg(test)]
use crate::display_row_builder::{
    DisplayRowPosition, display_row_text_is_empty, merge_display_row_source_slot_bounds,
};
#[cfg(test)]
use crate::window_output::{TextRowOutput, WindowOutputEmitter};
use neomacs_display_protocol::effect_config::EffectsConfig;
use neomacs_display_protocol::face::Face;
use neomacs_display_protocol::frame_glyphs::{
    CursorStyle, DisplaySlotId, GlyphRowRole, PhysCursor,
};
use neomacs_display_protocol::glyph_matrix::{FrameChromeRow, GlyphRow};
use neomacs_display_protocol::types::{Color, Rect};
#[cfg(test)]
use neovm_core::emacs_core::Context;

struct DisplayRowOutputInstall<'a> {
    display_row_index: usize,
    row: &'a GlyphRow,
    source_slots: Option<&'a [crate::display_row_builder::DisplayRowGlyphSlot]>,
    pixel_y: f32,
    height_px: f32,
    ascent_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayOutputRowStoredMetrics {
    pub(crate) pixel_y: f32,
    pub(crate) height_px: f32,
    pub(crate) ascent_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayOutputTextRowMetricsInstallRequest {
    display_row_index: usize,
    absolute_y: f32,
    height_px: f32,
    ascent_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayOutputTextWindowBeginInstallRequest {
    window_id: u64,
    rows: usize,
    cols: usize,
    bounds: Rect,
    text_bounds: Rect,
    selected: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplayOutputCursorArtifactInstallRequest {
    window_id: i64,
    slot_id: DisplaySlotId,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    style: CursorStyle,
    color: Color,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TextWindowRowDecorationRequest {
    MarkCurrentTruncatedLeft,
}

impl<'a> DisplayRowOutputInstall<'a> {
    fn from_row(display_row_index: usize, row: &'a GlyphRow) -> Self {
        Self {
            display_row_index,
            row,
            source_slots: None,
            pixel_y: row.pixel_y,
            height_px: row.height_px,
            ascent_px: row.ascent_px,
        }
    }

    fn from_rendered(
        display_row_index: usize,
        rendered: &'a RenderedDisplayRow,
        bounds: Rect,
        height_px: f32,
        ascent_px: f32,
    ) -> Self {
        Self {
            display_row_index,
            row: &rendered.row,
            source_slots: Some(&rendered.source_slots),
            pixel_y: bounds.y,
            height_px,
            ascent_px,
        }
    }

    fn install(self, builder: &mut DisplayOutputBuilder) {
        let pixel_bounds = builder.current_window_pixel_bounds();
        let mut row = self.row.clone();
        if let Some(source_slots) = self.source_slots {
            apply_display_row_source_slot_bounds(&mut row, source_slots);
        }
        row.pixel_y = self.pixel_y - pixel_bounds.y;
        row.height_px = self.height_px;
        row.ascent_px = self.ascent_px;
        builder.install_output_row_lifecycle(OutputRowLifecycleRequest::complete(
            self.display_row_index,
            row.role,
            row.mode_line,
            row,
        ));
    }
}

impl DisplayOutputTextWindowBeginInstallRequest {
    pub(crate) fn new(
        window_id: u64,
        rows: usize,
        cols: usize,
        bounds: Rect,
        text_bounds: Rect,
        selected: bool,
    ) -> Self {
        Self {
            window_id,
            rows,
            cols,
            bounds,
            text_bounds,
            selected,
        }
    }

    fn install(self, builder: &mut DisplayOutputBuilder) {
        builder.begin_output_window(
            self.window_id,
            self.rows,
            self.cols,
            self.bounds,
            self.text_bounds,
            self.selected,
        );
    }
}

impl DisplayOutputCursorArtifactInstallRequest {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        window_id: i64,
        slot_id: DisplaySlotId,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        style: CursorStyle,
        color: Color,
    ) -> Self {
        Self {
            window_id,
            slot_id,
            x,
            y,
            width,
            height,
            style,
            color,
        }
    }

    fn install(self, builder: &mut DisplayOutputBuilder) {
        builder.install_output_cursor(OutputCursorInstallRequest::new(
            self.window_id,
            self.slot_id,
            self.x,
            self.y,
            self.width,
            self.height,
            self.style,
            self.color,
        ));
    }
}

impl DisplayOutputRowStoredMetrics {
    fn from_absolute_window_y(
        builder: &DisplayOutputBuilder,
        request_y: f32,
        height_px: f32,
        ascent_px: f32,
    ) -> Self {
        let window_y = builder.current_window_pixel_bounds().y;
        Self {
            pixel_y: request_y - window_y,
            height_px,
            ascent_px,
        }
    }
}

impl DisplayOutputTextRowMetricsInstallRequest {
    pub(crate) fn new(
        display_row_index: usize,
        absolute_y: f32,
        height_px: f32,
        ascent_px: f32,
    ) -> Self {
        Self {
            display_row_index,
            absolute_y,
            height_px,
            ascent_px,
        }
    }

    pub(crate) fn display_row_index(self) -> usize {
        self.display_row_index
    }

    fn stored_metrics(self, builder: &DisplayOutputBuilder) -> DisplayOutputRowStoredMetrics {
        DisplayOutputRowStoredMetrics::from_absolute_window_y(
            builder,
            self.absolute_y,
            self.height_px,
            self.ascent_px,
        )
    }

    fn install(self, builder: &mut DisplayOutputBuilder) -> DisplayOutputRowStoredMetrics {
        let metrics = self.stored_metrics(builder);
        builder.install_output_row_lifecycle(OutputRowLifecycleRequest::metrics(
            self.display_row_index,
            metrics.pixel_y,
            metrics.height_px,
            metrics.ascent_px,
        ));
        metrics
    }
}

pub(crate) fn begin_text_output_window(
    builder: &mut DisplayOutputBuilder,
    request: DisplayOutputTextWindowBeginInstallRequest,
) {
    request.install(builder);
}

pub(crate) fn end_text_output_window(builder: &mut DisplayOutputBuilder) {
    builder.end_output_window();
}

pub(crate) fn install_text_output_display_range(
    builder: &mut DisplayOutputBuilder,
    window_id: i64,
    window_start: i64,
    window_end: i64,
) {
    builder.install_window_metadata(OutputTextWindowDisplayRangeInstallRequest::new(
        window_id,
        window_start,
        window_end,
    ));
}

pub(crate) fn restore_text_output_retry_checkpoint(
    builder: &mut DisplayOutputBuilder,
    transition_hints_len: usize,
    effect_hints_len: usize,
) {
    builder.install_window_metadata(OutputRetryCheckpointRestoreRequest::new(
        transition_hints_len,
        effect_hints_len,
    ));
}

pub(crate) fn install_text_output_row_decoration(
    builder: &mut DisplayOutputBuilder,
    request: TextWindowRowDecorationRequest,
) {
    match request {
        TextWindowRowDecorationRequest::MarkCurrentTruncatedLeft => {
            builder.install_output_row_lifecycle(OutputRowLifecycleRequest::current_decoration(
                OutputCurrentRowDecorationRequest::MarkTruncatedLeft,
            ));
        }
    }
}

pub(crate) fn install_text_output_cursor_effects(
    builder: &mut DisplayOutputBuilder,
    window_id: i64,
    effects: EffectsConfig,
) {
    builder.install_output_frame_state(OutputFrameStateInstallRequest::cursor_effects(
        window_id, effects,
    ));
}

pub(crate) fn install_text_output_cursor_artifact(
    builder: &mut DisplayOutputBuilder,
    request: DisplayOutputCursorArtifactInstallRequest,
) {
    request.install(builder);
}

pub(crate) fn install_text_output_row_cursor(
    builder: &mut DisplayOutputBuilder,
    display_row_index: usize,
    col: u16,
    style: CursorStyle,
) {
    builder.install_output_row_lifecycle(OutputRowLifecycleRequest::cursor(
        display_row_index,
        col,
        style,
    ));
}

pub(crate) fn store_text_output_phys_cursor(
    builder: &mut DisplayOutputBuilder,
    cursor: PhysCursor,
) {
    builder.install_output_frame_artifact(OutputFrameArtifactInstallRequest::phys_cursor(cursor));
}

pub(crate) fn install_current_text_output_row_decoration<D>(
    builder: &mut DisplayOutputBuilder,
    display_row_index: usize,
    decorator: D,
) where
    D: OutputRowDecorator,
{
    builder.install_row_decoration(OutputRowDecorationInstallRequest::current_window_row(
        display_row_index,
        decorator,
    ));
}

pub(crate) fn install_last_text_output_rows_decoration<D>(
    builder: &mut DisplayOutputBuilder,
    decorator: D,
) where
    D: OutputRowDecorator,
{
    builder.install_row_decoration(OutputRowDecorationInstallRequest::last_window_rows(
        decorator,
    ));
}

pub(crate) fn begin_text_output_row(
    builder: &mut DisplayOutputBuilder,
    display_row_index: usize,
) -> usize {
    builder.install_output_row_lifecycle(OutputRowLifecycleRequest::begin(
        display_row_index,
        GlyphRowRole::Text,
        false,
    ));
    display_row_index
}

pub(crate) fn install_text_output_row_metrics(
    builder: &mut DisplayOutputBuilder,
    request: DisplayOutputTextRowMetricsInstallRequest,
) -> DisplayOutputRowStoredMetrics {
    request.install(builder)
}

pub(crate) fn finish_text_output_row(
    builder: &mut DisplayOutputBuilder,
    request: DisplayOutputTextRowMetricsInstallRequest,
) -> DisplayOutputRowStoredMetrics {
    let metrics = install_text_output_row_metrics(builder, request);
    builder.install_output_row_lifecycle(OutputRowLifecycleRequest::finalize(
        request.display_row_index(),
    ));
    metrics
}

pub(crate) fn finalize_text_output_row(
    builder: &mut DisplayOutputBuilder,
    display_row_index: usize,
) {
    builder.install_output_row_lifecycle(OutputRowLifecycleRequest::finalize(display_row_index));
}

pub(crate) fn install_display_row(
    builder: &mut DisplayOutputBuilder,
    display_row_index: usize,
    row: &GlyphRow,
) {
    DisplayRowOutputInstall::from_row(display_row_index, row).install(builder);
}

pub(crate) fn install_measured_window_display_row(
    builder: &mut DisplayOutputBuilder,
    measured: &MeasuredDisplayRow,
) {
    MeasuredWindowDisplayRowInstallRequest { measured }.install(builder);
}

pub(crate) fn install_measured_frame_chrome_display_row(
    builder: &mut DisplayOutputBuilder,
    frame_chrome_rows: &mut Vec<FrameChromeRow>,
    measured: &MeasuredDisplayRow,
) {
    MeasuredFrameChromeRowInstallRequest {
        frame_chrome_rows,
        measured,
    }
    .install(builder);
}

pub(crate) fn install_rendered_display_row_fragment_assets(
    builder: &mut DisplayOutputBuilder,
    role: GlyphRowRole,
    display_row_index: usize,
    faces: &[Face],
    media: &[RenderedDisplayRowMedia],
) {
    RenderedDisplayRowAssetsInstall::fragment(role, display_row_index, faces, media)
        .install(builder);
}

struct DisplayRowCurrentRowInstaller<'builder> {
    builder: &'builder mut DisplayOutputBuilder,
}

pub(crate) struct DisplayRowCurrentRowOutput<'builder> {
    installer: DisplayRowCurrentRowInstaller<'builder>,
}

pub(crate) trait DisplayCurrentRowMutation {
    type Output;

    fn apply(self, row: &mut GlyphRow) -> Self::Output;
}

impl<'builder> DisplayRowCurrentRowInstaller<'builder> {
    fn new(builder: &'builder mut DisplayOutputBuilder) -> Self {
        Self { builder }
    }

    fn reborrow(&mut self) -> DisplayRowCurrentRowInstaller<'_> {
        DisplayRowCurrentRowInstaller {
            builder: self.builder,
        }
    }

    fn edit_current_row<R>(&mut self, f: impl FnOnce(&mut GlyphRow) -> R) -> Option<R> {
        self.builder.edit_current_output_row(f)
    }

    fn current_row_snapshot(&self) -> Option<GlyphRow> {
        self.builder.current_row_for_render().cloned()
    }

    fn apply_current_row_mutation<M>(&mut self, mutation: M) -> Option<M::Output>
    where
        M: DisplayCurrentRowMutation,
    {
        self.edit_current_row(|row| mutation.apply(row))
    }

    fn apply_current_row_scratch_mutation<M>(&self, mutation: M) -> Option<M::Output>
    where
        M: DisplayCurrentRowMutation,
    {
        let mut row = self.current_row_snapshot()?;
        Some(mutation.apply(&mut row))
    }

    #[cfg(test)]
    fn append_rendered_fragment(
        &mut self,
        rendered: &RenderedDisplayRow,
    ) -> Option<DisplayRowPosition> {
        let end = display_row_output_end_position(rendered.progress);
        self.edit_current_row(|row| {
            row.enabled = true;
            row.role = rendered.row.role;
            row.mode_line = matches!(rendered.row.role, GlyphRowRole::ModeLine);
            row.displays_text |=
                rendered.row.displays_text || !display_row_text_is_empty(&rendered.row);
            row.glyphs[neomacs_display_protocol::glyph_matrix::GlyphArea::Text.index()].extend(
                rendered.row.glyphs
                    [neomacs_display_protocol::glyph_matrix::GlyphArea::Text.index()]
                .iter()
                .cloned(),
            );
            row.height_px = row.height_px.max(rendered.row.height_px);
            row.ascent_px = row
                .ascent_px
                .max(rendered.row.ascent_px)
                .min(row.height_px.max(1.0));
            merge_display_row_source_slot_bounds(row, &rendered.source_slots);
        })?;
        Some(end)
    }
}

impl<'builder> DisplayRowCurrentRowOutput<'builder> {
    fn from_installer(installer: DisplayRowCurrentRowInstaller<'builder>) -> Self {
        Self { installer }
    }

    pub(crate) fn from_output_builder(builder: &'builder mut DisplayOutputBuilder) -> Self {
        Self::from_installer(DisplayRowCurrentRowInstaller::new(builder))
    }

    pub(crate) fn reborrow(&mut self) -> DisplayRowCurrentRowOutput<'_> {
        DisplayRowCurrentRowOutput {
            installer: self.installer.reborrow(),
        }
    }

    pub(crate) fn apply_current_row_mutation<M>(&mut self, mutation: M) -> Option<M::Output>
    where
        M: DisplayCurrentRowMutation,
    {
        self.installer.apply_current_row_mutation(mutation)
    }

    pub(crate) fn apply_current_row_scratch_mutation<M>(&self, mutation: M) -> Option<M::Output>
    where
        M: DisplayCurrentRowMutation,
    {
        self.installer.apply_current_row_scratch_mutation(mutation)
    }

    pub(crate) fn cluster_tail(&self) -> Option<(char, bool)> {
        self.installer
            .current_row_snapshot()
            .as_ref()
            .and_then(last_text_cluster_tail_in_row)
    }
}

struct MeasuredWindowDisplayRowInstallRequest<'a> {
    measured: &'a MeasuredDisplayRow,
}

impl MeasuredWindowDisplayRowInstallRequest<'_> {
    fn install(self, builder: &mut DisplayOutputBuilder) {
        let measured = self.measured;
        let DisplayRowOwner::WindowChrome { window_id, kind } = measured.owner else {
            panic!("frame chrome rows must install through frame chrome rows");
        };
        debug_assert!(window_id > 0);
        debug_assert_eq!(builder.current_window_id_i64(), window_id as i64);
        debug_assert!(matches!(
            kind,
            WindowChromeKind::TabLine | WindowChromeKind::HeaderLine | WindowChromeKind::ModeLine
        ));
        let display_row_index = measured.row_index as usize;
        RenderedDisplayRowAssetsInstall::window_row(
            &measured.rendered,
            measured.row_index,
            measured.bounds,
        )
        .install(builder);
        DisplayRowOutputInstall::from_rendered(
            display_row_index,
            &measured.rendered,
            measured.bounds,
            measured.row_height(),
            measured.row_ascent(),
        )
        .install(builder);
    }
}

struct MeasuredFrameChromeRowInstallRequest<'a, 'rows> {
    frame_chrome_rows: &'rows mut Vec<FrameChromeRow>,
    measured: &'a MeasuredDisplayRow,
}

impl MeasuredFrameChromeRowInstallRequest<'_, '_> {
    fn install(self, builder: &mut DisplayOutputBuilder) {
        let measured = self.measured;
        let DisplayRowOwner::FrameChrome { kind } = measured.owner else {
            panic!("window-owned rows must install through window chrome");
        };
        debug_assert!(matches!(kind, FrameChromeKind::TabBar));
        RenderedDisplayRowAssetsInstall::frame_chrome(
            &measured.rendered,
            measured.row_index,
            measured.bounds,
        )
        .install(builder);
        let mut row = measured.rendered.row.clone();
        apply_display_row_source_slot_bounds(&mut row, &measured.rendered.source_slots);
        let _ = crate::glyph_row_writer::reorder_row_bidi(&mut row, None);
        row.pixel_y = measured.bounds.y;
        row.height_px = measured.row_height();
        row.ascent_px = measured.row_ascent();
        self.frame_chrome_rows.push(FrameChromeRow {
            row_index: measured.row_index,
            pixel_bounds: measured.bounds,
            row,
        });
    }
}

#[derive(Clone, Copy)]
enum RenderedDisplayRowAssetInstallTarget {
    CurrentWindowRow(usize),
    WindowRow { row_index: u32, bounds: Rect },
    FrameChrome { row_index: u32, bounds: Rect },
}

struct RenderedDisplayRowAssetsInstall<'a> {
    role: GlyphRowRole,
    faces: &'a [Face],
    media: &'a [RenderedDisplayRowMedia],
    target: RenderedDisplayRowAssetInstallTarget,
}

impl<'a> RenderedDisplayRowAssetsInstall<'a> {
    fn fragment(
        role: GlyphRowRole,
        display_row_index: usize,
        faces: &'a [Face],
        media: &'a [RenderedDisplayRowMedia],
    ) -> Self {
        Self {
            role,
            faces,
            media,
            target: RenderedDisplayRowAssetInstallTarget::CurrentWindowRow(display_row_index),
        }
    }

    fn from_rendered(
        rendered: &'a RenderedDisplayRow,
        target: RenderedDisplayRowAssetInstallTarget,
    ) -> Self {
        Self {
            role: rendered.row.role,
            faces: &rendered.faces,
            media: &rendered.media,
            target,
        }
    }

    fn window_row(rendered: &'a RenderedDisplayRow, row_index: u32, bounds: Rect) -> Self {
        Self::from_rendered(
            rendered,
            RenderedDisplayRowAssetInstallTarget::WindowRow { row_index, bounds },
        )
    }

    fn frame_chrome(rendered: &'a RenderedDisplayRow, row_index: u32, bounds: Rect) -> Self {
        Self::from_rendered(
            rendered,
            RenderedDisplayRowAssetInstallTarget::FrameChrome { row_index, bounds },
        )
    }

    fn install(self, builder: &mut DisplayOutputBuilder) {
        for face in self.faces {
            builder.install_output_frame_state(OutputFrameStateInstallRequest::face(
                face.id,
                face.clone(),
            ));
        }
        for medium in self.media {
            self.install_medium(builder, medium);
        }
    }

    fn install_medium(&self, builder: &mut DisplayOutputBuilder, medium: &RenderedDisplayRowMedia) {
        let target = DisplayRowMediaInstallTarget::resolve(builder, medium.col, self.target);
        let target = ResolvedOutputMediaInstallTarget::new(
            target.window_id,
            self.role,
            target.clip,
            target.slot_id,
        );
        match medium.kind {
            RenderedDisplayRowMediaKind::Image { image_id } => {
                builder.install_output_media(OutputMediaInstallRequest::image(
                    target,
                    image_id,
                    medium.x,
                    medium.y,
                    medium.width,
                    medium.height,
                ));
            }
            RenderedDisplayRowMediaKind::Video {
                video_id,
                loop_count,
                autoplay,
            } => {
                builder.install_output_media(OutputMediaInstallRequest::video(
                    target,
                    video_id,
                    loop_count,
                    autoplay,
                    medium.x,
                    medium.y,
                    medium.width,
                    medium.height,
                ));
            }
            RenderedDisplayRowMediaKind::Xwidget { xwidget_id } => {
                builder.install_output_media(OutputMediaInstallRequest::xwidget(
                    target,
                    xwidget_id,
                    medium.x,
                    medium.y,
                    medium.width,
                    medium.height,
                ));
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DisplayRowMediaInstallTarget {
    window_id: i64,
    clip: Option<Rect>,
    slot_id: DisplaySlotId,
}

impl DisplayRowMediaInstallTarget {
    fn resolve(
        builder: &DisplayOutputBuilder,
        col: u16,
        target: RenderedDisplayRowAssetInstallTarget,
    ) -> Self {
        match target {
            RenderedDisplayRowAssetInstallTarget::CurrentWindowRow(display_row_index) => {
                let window_id = builder.current_window_id_i64();
                let clip = builder.current_window_text_pixel_bounds();
                let row = display_row_index.min(u32::MAX as usize) as u32;
                Self {
                    window_id,
                    clip: Some(clip),
                    slot_id: DisplaySlotId {
                        window_id,
                        row,
                        col,
                    },
                }
            }
            RenderedDisplayRowAssetInstallTarget::WindowRow { row_index, bounds } => {
                let window_id = builder.current_window_id_i64();
                Self {
                    window_id,
                    clip: Some(bounds),
                    slot_id: DisplaySlotId {
                        window_id,
                        row: row_index,
                        col,
                    },
                }
            }
            RenderedDisplayRowAssetInstallTarget::FrameChrome { row_index, bounds } => Self {
                window_id: FRAME_CHROME_WINDOW_ID,
                clip: Some(bounds),
                slot_id: DisplaySlotId {
                    window_id: FRAME_CHROME_WINDOW_ID,
                    row: row_index,
                    col,
                },
            },
        }
    }
}

#[cfg(test)]
pub(crate) fn append_rendered_display_row_fragment_to_current_row(
    builder: &mut DisplayOutputBuilder,
    rendered: &RenderedDisplayRow,
    display_row_index: usize,
) -> DisplayRowPosition {
    for face in &rendered.faces {
        builder.install_output_frame_state(OutputFrameStateInstallRequest::face(
            face.id,
            face.clone(),
        ));
    }
    let end = DisplayRowCurrentRowInstaller::new(builder)
        .append_rendered_fragment(rendered)
        .expect("current row");
    RenderedDisplayRowAssetsInstall::fragment(
        rendered.row.role,
        display_row_index,
        &[],
        &rendered.media,
    )
    .install(builder);
    end
}

#[cfg(test)]
pub(crate) fn append_rendered_display_row_fragment_to_text_row_and_emit(
    builder: &mut DisplayOutputBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    rendered: &RenderedDisplayRow,
    output: TextRowOutput,
) -> DisplayRowPosition {
    let end = append_rendered_display_row_fragment_to_current_row(builder, rendered, output.row);
    output_emitter.emit_text_source_slots(evaluator, output, &rendered.source_slots, end);
    end
}

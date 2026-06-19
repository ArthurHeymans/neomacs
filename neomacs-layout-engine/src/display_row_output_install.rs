use crate::composition::last_text_cluster_tail_in_row;
use crate::display_cursor::CursorVisualColumnResolutionContext;
use crate::display_output_artifact_install::OutputArtifactInstallSurface;
use crate::display_output_builder::{
    DisplayOutputBuilder, FRAME_CHROME_WINDOW_ID, OutputRowDecorationInstallRequest,
    OutputRowDecorator,
};
#[cfg(test)]
use crate::display_row::display_row_output_end_position;
use crate::display_row::{
    DisplayRowOwner, FrameChromeKind, MeasuredDisplayRow, RenderedDisplayRow,
    RenderedDisplayRowMedia, RenderedDisplayRowMediaKind, WindowChromeKind,
};
use crate::display_row_builder::apply_display_row_source_slot_bounds;
use crate::display_row_builder::{DisplayRowGlyphSlot, merge_display_row_source_slot_bounds};
#[cfg(test)]
use crate::display_row_builder::{DisplayRowPosition, display_row_text_is_empty};
use crate::font_metrics::FontMetrics;
use crate::neovm_bridge::ResolvedFace;
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
        let pixel_bounds = DisplayRowWindowContextSurface::from_output_builder(builder)
            .current_window_pixel_bounds();
        let mut row = self.row.clone();
        if let Some(source_slots) = self.source_slots {
            apply_display_row_source_slot_bounds(&mut row, source_slots);
        }
        row.pixel_y = self.pixel_y - pixel_bounds.y;
        row.height_px = self.height_px;
        row.ascent_px = self.ascent_px;
        builder.row_installer().install_complete_row(
            self.display_row_index,
            row.role,
            row.mode_line,
            row,
        );
    }
}

struct DisplayRowInstaller<'builder, 'rows> {
    builder: &'builder mut DisplayOutputBuilder,
    frame_chrome_rows: Option<&'rows mut Vec<FrameChromeRow>>,
}

pub(crate) struct DisplayRowInstallSurface<'builder, 'rows> {
    installer: DisplayRowInstaller<'builder, 'rows>,
}

struct DisplayRowCurrentRowInstaller<'builder> {
    builder: &'builder mut DisplayOutputBuilder,
}

pub(crate) struct DisplayRowCurrentRowSurface<'builder> {
    installer: DisplayRowCurrentRowInstaller<'builder>,
}

struct DisplayRowFaceInstaller<'builder> {
    builder: &'builder mut DisplayOutputBuilder,
}

pub(crate) struct DisplayRowFaceInstallSurface<'builder> {
    installer: DisplayRowFaceInstaller<'builder>,
}

struct DisplayRowLifecycleInstaller<'builder> {
    builder: &'builder mut DisplayOutputBuilder,
}

pub(crate) struct DisplayRowLifecycleSurface<'builder> {
    installer: DisplayRowLifecycleInstaller<'builder>,
}

struct DisplayRowArtifactInstaller<'builder> {
    builder: &'builder mut DisplayOutputBuilder,
}

pub(crate) struct DisplayRowArtifactInstallSurface<'builder> {
    installer: DisplayRowArtifactInstaller<'builder>,
}

struct DisplayRowAssetsInstaller<'builder> {
    builder: &'builder mut DisplayOutputBuilder,
}

struct DisplayRowAssetsInstallSurface<'builder> {
    installer: DisplayRowAssetsInstaller<'builder>,
}

struct DisplayRowDecorationInstaller<'builder> {
    builder: &'builder mut DisplayOutputBuilder,
}

pub(crate) struct DisplayRowDecorationSurface<'builder> {
    installer: DisplayRowDecorationInstaller<'builder>,
}

pub(crate) struct DisplayRowWindowContextSurface<'builder> {
    builder: &'builder DisplayOutputBuilder,
}

impl<'builder, 'rows> DisplayRowInstaller<'builder, 'rows> {
    fn new(builder: &'builder mut DisplayOutputBuilder) -> Self {
        Self {
            builder,
            frame_chrome_rows: None,
        }
    }

    fn from_output_builder_with_frame_chrome_rows(
        builder: &'builder mut DisplayOutputBuilder,
        frame_chrome_rows: &'rows mut Vec<FrameChromeRow>,
    ) -> Self {
        Self {
            builder,
            frame_chrome_rows: Some(frame_chrome_rows),
        }
    }

    fn install_row(&mut self, display_row_index: usize, row: &GlyphRow) {
        DisplayRowOutputInstall::from_row(display_row_index, row).install(self.builder);
    }

    fn install_measured(&mut self, measured: &MeasuredDisplayRow) {
        match measured.owner {
            DisplayRowOwner::WindowChrome { .. } => {
                MeasuredWindowDisplayRowInstallRequest { measured }.install(self.builder);
            }
            DisplayRowOwner::FrameChrome { .. } => {
                let frame_chrome_rows = self
                    .frame_chrome_rows
                    .as_deref_mut()
                    .expect("frame chrome rows are required to install frame chrome");
                MeasuredFrameChromeRowInstallRequest {
                    frame_chrome_rows,
                    measured,
                }
                .install(self.builder);
            }
        }
    }

    fn install_fragment_assets(
        &mut self,
        role: GlyphRowRole,
        display_row_index: usize,
        faces: &[Face],
        media: &[RenderedDisplayRowMedia],
    ) {
        RenderedDisplayRowAssetsInstall::fragment(role, display_row_index, faces, media).install(
            &mut DisplayRowAssetsInstallSurface::from_output_builder(self.builder),
        );
    }
}

impl<'builder> DisplayRowInstallSurface<'builder, 'static> {
    pub(crate) fn from_output_builder(builder: &'builder mut DisplayOutputBuilder) -> Self {
        Self {
            installer: DisplayRowInstaller::new(builder),
        }
    }
}

impl<'builder, 'rows> DisplayRowInstallSurface<'builder, 'rows> {
    pub(crate) fn from_output_builder_with_frame_chrome_rows(
        builder: &'builder mut DisplayOutputBuilder,
        frame_chrome_rows: &'rows mut Vec<FrameChromeRow>,
    ) -> Self {
        Self {
            installer: DisplayRowInstaller::from_output_builder_with_frame_chrome_rows(
                builder,
                frame_chrome_rows,
            ),
        }
    }

    pub(crate) fn install_row(&mut self, display_row_index: usize, row: &GlyphRow) {
        self.installer.install_row(display_row_index, row);
    }

    pub(crate) fn install_measured(&mut self, measured: &MeasuredDisplayRow) {
        self.installer.install_measured(measured);
    }

    pub(crate) fn install_fragment_assets(
        &mut self,
        role: GlyphRowRole,
        display_row_index: usize,
        faces: &[Face],
        media: &[RenderedDisplayRowMedia],
    ) {
        self.installer
            .install_fragment_assets(role, display_row_index, faces, media);
    }
}

impl<'builder> DisplayRowFaceInstaller<'builder> {
    fn new(builder: &'builder mut DisplayOutputBuilder) -> Self {
        Self { builder }
    }

    fn install_face(&mut self, face: &Face) {
        OutputArtifactInstallSurface::from_output_builder(self.builder).install_face(face);
    }

    fn install_resolved_face(
        &mut self,
        face_id: u32,
        face: &ResolvedFace,
        metrics: Option<FontMetrics>,
    ) {
        OutputArtifactInstallSurface::from_output_builder(self.builder)
            .install_resolved_face(face_id, face, metrics);
    }
}

impl<'builder> DisplayRowFaceInstallSurface<'builder> {
    pub(crate) fn from_output_builder(builder: &'builder mut DisplayOutputBuilder) -> Self {
        Self {
            installer: DisplayRowFaceInstaller::new(builder),
        }
    }

    pub(crate) fn install_face(&mut self, face: &Face) {
        self.installer.install_face(face);
    }

    pub(crate) fn install_resolved_face(
        &mut self,
        face_id: u32,
        face: &ResolvedFace,
        metrics: Option<FontMetrics>,
    ) {
        self.installer.install_resolved_face(face_id, face, metrics);
    }
}

impl<'builder> DisplayRowLifecycleInstaller<'builder> {
    fn new(builder: &'builder mut DisplayOutputBuilder) -> Self {
        Self { builder }
    }

    fn begin_row(&mut self, display_row_index: usize, role: GlyphRowRole, mode_line: bool) {
        self.builder
            .row_installer()
            .begin(display_row_index, role, mode_line);
    }

    fn set_metrics(&mut self, display_row_index: usize, pixel_y: f32, height: f32, ascent: f32) {
        self.builder
            .row_installer()
            .set_metrics(display_row_index, pixel_y, height, ascent);
    }

    fn finalize_row(&mut self, display_row_index: usize) {
        self.builder.row_installer().finalize(display_row_index);
    }

    fn set_cursor(&mut self, row: usize, col: u16, style: CursorStyle) {
        self.builder.row_installer().set_cursor(row, col, style);
    }

    fn mark_current_truncated_left(&mut self) {
        self.builder.row_installer().mark_current_truncated_left();
    }
}

impl<'builder> DisplayRowLifecycleSurface<'builder> {
    pub(crate) fn from_output_builder(builder: &'builder mut DisplayOutputBuilder) -> Self {
        Self {
            installer: DisplayRowLifecycleInstaller::new(builder),
        }
    }

    pub(crate) fn begin_row(
        &mut self,
        display_row_index: usize,
        role: GlyphRowRole,
        mode_line: bool,
    ) {
        self.installer.begin_row(display_row_index, role, mode_line);
    }

    pub(crate) fn set_metrics(
        &mut self,
        display_row_index: usize,
        pixel_y: f32,
        height: f32,
        ascent: f32,
    ) {
        self.installer
            .set_metrics(display_row_index, pixel_y, height, ascent);
    }

    pub(crate) fn finalize_row(&mut self, display_row_index: usize) {
        self.installer.finalize_row(display_row_index);
    }

    pub(crate) fn set_cursor(&mut self, row: usize, col: u16, style: CursorStyle) {
        self.installer.set_cursor(row, col, style);
    }

    pub(crate) fn mark_current_truncated_left(&mut self) {
        self.installer.mark_current_truncated_left();
    }
}

impl<'builder> DisplayRowArtifactInstaller<'builder> {
    fn new(builder: &'builder mut DisplayOutputBuilder) -> Self {
        Self { builder }
    }

    fn add_cursor(
        &mut self,
        window_id: i64,
        slot_id: DisplaySlotId,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        style: CursorStyle,
        color: Color,
    ) {
        OutputArtifactInstallSurface::from_output_builder(self.builder)
            .add_cursor(window_id, slot_id, x, y, width, height, style, color);
    }

    fn set_cursor_effects(&mut self, window_id: i64, effects: EffectsConfig) {
        OutputArtifactInstallSurface::from_output_builder(self.builder)
            .set_cursor_effects(window_id, effects);
    }

    fn store_phys_cursor(&mut self, cursor: PhysCursor) {
        OutputArtifactInstallSurface::from_output_builder(self.builder).store_phys_cursor(cursor);
    }

    fn add_image_media(
        &mut self,
        window_id: i64,
        role: GlyphRowRole,
        clip: Option<Rect>,
        slot_id: DisplaySlotId,
        image_id: u32,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) {
        OutputArtifactInstallSurface::from_output_builder(self.builder).add_image_media(
            window_id, role, clip, slot_id, image_id, x, y, width, height,
        );
    }

    fn add_video_media(
        &mut self,
        window_id: i64,
        role: GlyphRowRole,
        clip: Option<Rect>,
        slot_id: DisplaySlotId,
        video_id: u32,
        loop_count: i32,
        autoplay: bool,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) {
        OutputArtifactInstallSurface::from_output_builder(self.builder).add_video_media(
            window_id, role, clip, slot_id, video_id, loop_count, autoplay, x, y, width, height,
        );
    }

    fn add_xwidget_media(
        &mut self,
        window_id: i64,
        role: GlyphRowRole,
        clip: Option<Rect>,
        slot_id: DisplaySlotId,
        xwidget_id: u32,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) {
        OutputArtifactInstallSurface::from_output_builder(self.builder).add_xwidget_media(
            window_id, role, clip, slot_id, xwidget_id, x, y, width, height,
        );
    }
}

impl<'builder> DisplayRowArtifactInstallSurface<'builder> {
    pub(crate) fn from_output_builder(builder: &'builder mut DisplayOutputBuilder) -> Self {
        Self {
            installer: DisplayRowArtifactInstaller::new(builder),
        }
    }

    pub(crate) fn add_cursor(
        &mut self,
        window_id: i64,
        slot_id: DisplaySlotId,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        style: CursorStyle,
        color: Color,
    ) {
        self.installer
            .add_cursor(window_id, slot_id, x, y, width, height, style, color);
    }

    pub(crate) fn set_cursor_effects(&mut self, window_id: i64, effects: EffectsConfig) {
        self.installer.set_cursor_effects(window_id, effects);
    }

    pub(crate) fn store_phys_cursor(&mut self, cursor: PhysCursor) {
        self.installer.store_phys_cursor(cursor);
    }

    fn add_image_media(
        &mut self,
        window_id: i64,
        role: GlyphRowRole,
        clip: Option<Rect>,
        slot_id: DisplaySlotId,
        image_id: u32,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) {
        self.installer.add_image_media(
            window_id, role, clip, slot_id, image_id, x, y, width, height,
        );
    }

    fn add_video_media(
        &mut self,
        window_id: i64,
        role: GlyphRowRole,
        clip: Option<Rect>,
        slot_id: DisplaySlotId,
        video_id: u32,
        loop_count: i32,
        autoplay: bool,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) {
        self.installer.add_video_media(
            window_id, role, clip, slot_id, video_id, loop_count, autoplay, x, y, width, height,
        );
    }

    fn add_xwidget_media(
        &mut self,
        window_id: i64,
        role: GlyphRowRole,
        clip: Option<Rect>,
        slot_id: DisplaySlotId,
        xwidget_id: u32,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) {
        self.installer.add_xwidget_media(
            window_id, role, clip, slot_id, xwidget_id, x, y, width, height,
        );
    }
}

impl<'builder> DisplayRowAssetsInstaller<'builder> {
    fn new(builder: &'builder mut DisplayOutputBuilder) -> Self {
        Self { builder }
    }

    fn install_faces(&mut self, faces: &[Face]) {
        let mut face_installer = DisplayRowFaceInstallSurface::from_output_builder(self.builder);
        for face in faces {
            face_installer.install_face(face);
        }
    }

    fn install_media(
        &mut self,
        role: GlyphRowRole,
        target: RenderedDisplayRowAssetInstallTarget,
        media: &[RenderedDisplayRowMedia],
    ) {
        for medium in media {
            self.install_medium(role, target, medium);
        }
    }

    fn install_medium(
        &mut self,
        role: GlyphRowRole,
        target: RenderedDisplayRowAssetInstallTarget,
        medium: &RenderedDisplayRowMedia,
    ) {
        let target = DisplayRowMediaInstallTarget::resolve(self.builder, medium.col, target);
        let mut artifact_installer =
            DisplayRowArtifactInstallSurface::from_output_builder(self.builder);
        match medium.kind {
            RenderedDisplayRowMediaKind::Image { image_id } => {
                artifact_installer.add_image_media(
                    target.window_id,
                    role,
                    target.clip,
                    target.slot_id,
                    image_id,
                    medium.x,
                    medium.y,
                    medium.width,
                    medium.height,
                );
            }
            RenderedDisplayRowMediaKind::Video {
                video_id,
                loop_count,
                autoplay,
            } => {
                artifact_installer.add_video_media(
                    target.window_id,
                    role,
                    target.clip,
                    target.slot_id,
                    video_id,
                    loop_count,
                    autoplay,
                    medium.x,
                    medium.y,
                    medium.width,
                    medium.height,
                );
            }
            RenderedDisplayRowMediaKind::Xwidget { xwidget_id } => {
                artifact_installer.add_xwidget_media(
                    target.window_id,
                    role,
                    target.clip,
                    target.slot_id,
                    xwidget_id,
                    medium.x,
                    medium.y,
                    medium.width,
                    medium.height,
                );
            }
        }
    }
}

impl<'builder> DisplayRowAssetsInstallSurface<'builder> {
    fn from_output_builder(builder: &'builder mut DisplayOutputBuilder) -> Self {
        Self {
            installer: DisplayRowAssetsInstaller::new(builder),
        }
    }

    fn install(&mut self, request: RenderedDisplayRowAssetsInstall<'_>) {
        self.installer.install_faces(request.faces);
        self.installer
            .install_media(request.role, request.target, request.media);
    }
}

impl<'builder> DisplayRowDecorationInstaller<'builder> {
    fn new(builder: &'builder mut DisplayOutputBuilder) -> Self {
        Self { builder }
    }

    fn decorate_current_window_row<D>(&mut self, row_idx: usize, decorator: D)
    where
        D: OutputRowDecorator,
    {
        self.builder
            .install_row_decoration(OutputRowDecorationInstallRequest::current_window_row(
                row_idx, decorator,
            ));
    }

    fn decorate_last_window_rows<D>(&mut self, decorator: D)
    where
        D: OutputRowDecorator,
    {
        self.builder
            .install_row_decoration(OutputRowDecorationInstallRequest::last_window_rows(
                decorator,
            ));
    }
}

impl<'builder> DisplayRowDecorationSurface<'builder> {
    pub(crate) fn from_output_builder(builder: &'builder mut DisplayOutputBuilder) -> Self {
        Self {
            installer: DisplayRowDecorationInstaller::new(builder),
        }
    }

    pub(crate) fn decorate_current_window_row<D>(&mut self, row_idx: usize, decorator: D)
    where
        D: OutputRowDecorator,
    {
        self.installer
            .decorate_current_window_row(row_idx, decorator);
    }

    pub(crate) fn decorate_last_window_rows<D>(&mut self, decorator: D)
    where
        D: OutputRowDecorator,
    {
        self.installer.decorate_last_window_rows(decorator);
    }
}

impl<'builder> DisplayRowWindowContextSurface<'builder> {
    pub(crate) fn from_output_builder(builder: &'builder DisplayOutputBuilder) -> Self {
        Self { builder }
    }

    fn current_window_id_i64(&self) -> i64 {
        self.builder.current_window_id_i64()
    }

    pub(crate) fn current_window_pixel_bounds(&self) -> Rect {
        self.builder.current_window_pixel_bounds()
    }

    fn current_window_text_pixel_bounds(&self) -> Rect {
        self.builder.current_window_text_pixel_bounds()
    }

    pub(crate) fn cursor_visual_column_context(&self) -> CursorVisualColumnResolutionContext<'_> {
        self.builder.cursor_visual_column_context()
    }
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
        self.builder.row_installer().edit_current_row(f)
    }

    fn current_row_snapshot(&self) -> Option<GlyphRow> {
        self.builder.current_row_for_render().cloned()
    }

    fn merge_source_slot_bounds(&mut self, slots: &[DisplayRowGlyphSlot]) {
        let _ = self.edit_current_row(|row| {
            merge_display_row_source_slot_bounds(row, slots);
        });
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

impl<'builder> DisplayRowCurrentRowSurface<'builder> {
    fn from_installer(installer: DisplayRowCurrentRowInstaller<'builder>) -> Self {
        Self { installer }
    }

    pub(crate) fn from_output_builder(builder: &'builder mut DisplayOutputBuilder) -> Self {
        Self::from_installer(DisplayRowCurrentRowInstaller::new(builder))
    }

    pub(crate) fn reborrow(&mut self) -> DisplayRowCurrentRowSurface<'_> {
        DisplayRowCurrentRowSurface {
            installer: self.installer.reborrow(),
        }
    }

    pub(crate) fn edit_current_row<R>(&mut self, f: impl FnOnce(&mut GlyphRow) -> R) -> Option<R> {
        self.installer.edit_current_row(f)
    }

    pub(crate) fn current_row_snapshot(&self) -> Option<GlyphRow> {
        self.installer.current_row_snapshot()
    }

    pub(crate) fn cluster_tail(&self) -> Option<(char, bool)> {
        self.current_row_snapshot()
            .as_ref()
            .and_then(last_text_cluster_tail_in_row)
    }

    pub(crate) fn merge_source_slot_bounds(&mut self, slots: &[DisplayRowGlyphSlot]) {
        self.installer.merge_source_slot_bounds(slots);
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
        debug_assert_eq!(
            DisplayRowWindowContextSurface::from_output_builder(builder).current_window_id_i64(),
            window_id as i64
        );
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
        .install(&mut DisplayRowAssetsInstallSurface::from_output_builder(
            builder,
        ));
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
        .install(&mut DisplayRowAssetsInstallSurface::from_output_builder(
            builder,
        ));
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

    fn install(self, surface: &mut DisplayRowAssetsInstallSurface<'_>) {
        surface.install(self);
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
                let window_context = DisplayRowWindowContextSurface::from_output_builder(builder);
                let window_id = window_context.current_window_id_i64();
                let clip = window_context.current_window_text_pixel_bounds();
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
                let window_id = DisplayRowWindowContextSurface::from_output_builder(builder)
                    .current_window_id_i64();
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
    {
        let mut face_installer = DisplayRowFaceInstallSurface::from_output_builder(builder);
        for face in &rendered.faces {
            face_installer.install_face(face);
        }
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
    .install(&mut DisplayRowAssetsInstallSurface::from_output_builder(
        builder,
    ));
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

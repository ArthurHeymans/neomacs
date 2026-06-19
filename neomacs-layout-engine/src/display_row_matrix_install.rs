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
use crate::matrix_builder::{FRAME_CHROME_WINDOW_ID, GlyphMatrixBuilder};
use crate::neovm_bridge::ResolvedFace;
#[cfg(test)]
use crate::window_output::{TextRowOutput, WindowOutputEmitter};
use neomacs_display_protocol::face::Face;
use neomacs_display_protocol::frame_glyphs::{DisplaySlotId, GlyphRowRole};
use neomacs_display_protocol::glyph_matrix::{FrameChromeRow, GlyphRow};
use neomacs_display_protocol::types::Rect;
#[cfg(test)]
use neovm_core::emacs_core::Context;

struct DisplayRowMatrixInstall<'a> {
    matrix_row: usize,
    row: &'a GlyphRow,
    source_slots: Option<&'a [crate::display_row_builder::DisplayRowGlyphSlot]>,
    pixel_y: f32,
    height_px: f32,
    ascent_px: f32,
}

impl<'a> DisplayRowMatrixInstall<'a> {
    fn from_row(matrix_row: usize, row: &'a GlyphRow) -> Self {
        Self {
            matrix_row,
            row,
            source_slots: None,
            pixel_y: row.pixel_y,
            height_px: row.height_px,
            ascent_px: row.ascent_px,
        }
    }

    fn from_rendered(
        matrix_row: usize,
        rendered: &'a RenderedDisplayRow,
        bounds: Rect,
        height_px: f32,
        ascent_px: f32,
    ) -> Self {
        Self {
            matrix_row,
            row: &rendered.row,
            source_slots: Some(&rendered.source_slots),
            pixel_y: bounds.y,
            height_px,
            ascent_px,
        }
    }

    fn install(self, builder: &mut GlyphMatrixBuilder) {
        let pixel_bounds = builder.current_window_pixel_bounds();
        let mut row = self.row.clone();
        if let Some(source_slots) = self.source_slots {
            apply_display_row_source_slot_bounds(&mut row, source_slots);
        }
        row.pixel_y = self.pixel_y - pixel_bounds.y;
        row.height_px = self.height_px;
        row.ascent_px = self.ascent_px;
        builder
            .row_installer()
            .install_complete_row(self.matrix_row, row.role, row.mode_line, row);
    }
}

pub(crate) struct DisplayRowInstaller<'builder, 'rows> {
    builder: &'builder mut GlyphMatrixBuilder,
    frame_chrome_rows: Option<&'rows mut Vec<FrameChromeRow>>,
}

pub(crate) struct DisplayRowCurrentRowInstaller<'builder> {
    builder: &'builder mut GlyphMatrixBuilder,
}

pub(crate) struct DisplayRowFaceInstaller<'builder> {
    builder: &'builder mut GlyphMatrixBuilder,
}

impl<'builder, 'rows> DisplayRowInstaller<'builder, 'rows> {
    pub(crate) fn new(builder: &'builder mut GlyphMatrixBuilder) -> Self {
        Self {
            builder,
            frame_chrome_rows: None,
        }
    }

    pub(crate) fn with_frame_chrome_rows(
        builder: &'builder mut GlyphMatrixBuilder,
        frame_chrome_rows: &'rows mut Vec<FrameChromeRow>,
    ) -> Self {
        Self {
            builder,
            frame_chrome_rows: Some(frame_chrome_rows),
        }
    }

    pub(crate) fn install_row(&mut self, matrix_row: usize, row: &GlyphRow) {
        DisplayRowMatrixInstall::from_row(matrix_row, row).install(self.builder);
    }

    pub(crate) fn install_measured(&mut self, measured: &MeasuredDisplayRow) {
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
}

impl<'builder> DisplayRowFaceInstaller<'builder> {
    pub(crate) fn new(builder: &'builder mut GlyphMatrixBuilder) -> Self {
        Self { builder }
    }

    pub(crate) fn install_face(&mut self, face: &Face) {
        self.builder
            .artifact_installer()
            .set_face(face.id, face.clone());
    }

    pub(crate) fn install_resolved_face(
        &mut self,
        face_id: u32,
        face: &ResolvedFace,
        metrics: Option<FontMetrics>,
    ) {
        self.builder
            .artifact_installer()
            .set_resolved_display_row_face(face_id, face, metrics);
    }
}

impl<'builder> DisplayRowCurrentRowInstaller<'builder> {
    pub(crate) fn new(builder: &'builder mut GlyphMatrixBuilder) -> Self {
        Self { builder }
    }

    pub(crate) fn edit_current_row<R>(&mut self, f: impl FnOnce(&mut GlyphRow) -> R) -> Option<R> {
        self.builder.row_installer().edit_current_row(f)
    }

    pub(crate) fn current_row_snapshot(&self) -> Option<GlyphRow> {
        self.builder.current_row_for_render().cloned()
    }

    pub(crate) fn merge_source_slot_bounds(&mut self, slots: &[DisplayRowGlyphSlot]) {
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

struct MeasuredWindowDisplayRowInstallRequest<'a> {
    measured: &'a MeasuredDisplayRow,
}

impl MeasuredWindowDisplayRowInstallRequest<'_> {
    fn install(self, builder: &mut GlyphMatrixBuilder) {
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
        let matrix_row = measured.row_index as usize;
        RenderedDisplayRowAssetsInstall::window_row(
            &measured.rendered,
            measured.row_index,
            measured.bounds,
        )
        .install(builder);
        DisplayRowMatrixInstall::from_rendered(
            matrix_row,
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
    fn install(self, builder: &mut GlyphMatrixBuilder) {
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
    MatrixRow(usize),
    WindowRow { row_index: u32, bounds: Rect },
    FrameChrome { row_index: u32, bounds: Rect },
}

pub(crate) struct RenderedDisplayRowAssetsInstall<'a> {
    role: GlyphRowRole,
    faces: &'a [Face],
    media: &'a [RenderedDisplayRowMedia],
    target: RenderedDisplayRowAssetInstallTarget,
}

impl<'a> RenderedDisplayRowAssetsInstall<'a> {
    pub(crate) fn fragment(
        role: GlyphRowRole,
        matrix_row: usize,
        faces: &'a [Face],
        media: &'a [RenderedDisplayRowMedia],
    ) -> Self {
        Self {
            role,
            faces,
            media,
            target: RenderedDisplayRowAssetInstallTarget::MatrixRow(matrix_row),
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

    pub(crate) fn install(self, builder: &mut GlyphMatrixBuilder) {
        {
            let mut face_installer = DisplayRowFaceInstaller::new(builder);
            for face in self.faces {
                face_installer.install_face(face);
            }
        }
        for media in self.media {
            match self.target {
                RenderedDisplayRowAssetInstallTarget::MatrixRow(matrix_row) => {
                    media.install(builder, self.role, matrix_row);
                }
                RenderedDisplayRowAssetInstallTarget::WindowRow { row_index, bounds } => {
                    media.install_window_row(builder, self.role, row_index, bounds);
                }
                RenderedDisplayRowAssetInstallTarget::FrameChrome { row_index, bounds } => {
                    media.install_frame_chrome(builder, self.role, row_index, bounds);
                }
            }
        }
    }
}

#[cfg(test)]
pub(crate) fn append_rendered_display_row_fragment_to_current_row(
    builder: &mut GlyphMatrixBuilder,
    rendered: &RenderedDisplayRow,
    matrix_row: usize,
) -> DisplayRowPosition {
    {
        let mut face_installer = DisplayRowFaceInstaller::new(builder);
        for face in &rendered.faces {
            face_installer.install_face(face);
        }
    }
    let end = DisplayRowCurrentRowInstaller::new(builder)
        .append_rendered_fragment(rendered)
        .expect("current row");
    for media in &rendered.media {
        media.install(builder, rendered.row.role, matrix_row);
    }
    end
}

#[cfg(test)]
pub(crate) fn append_rendered_display_row_fragment_to_text_row_and_emit(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    rendered: &RenderedDisplayRow,
    output: TextRowOutput,
) -> DisplayRowPosition {
    let end = append_rendered_display_row_fragment_to_current_row(builder, rendered, output.row);
    output_emitter.emit_text_source_slots(evaluator, output, &rendered.source_slots, end);
    end
}

impl RenderedDisplayRowMedia {
    fn install(&self, builder: &mut GlyphMatrixBuilder, role: GlyphRowRole, matrix_row: usize) {
        let window_id = builder.current_window_id_i64();
        let clip = builder.current_window_text_pixel_bounds();
        let row = matrix_row.min(u32::MAX as usize) as u32;
        let slot_id = DisplaySlotId {
            window_id,
            row,
            col: self.col,
        };
        self.install_with_target(builder, window_id, role, Some(clip), slot_id);
    }

    fn install_window_row(
        &self,
        builder: &mut GlyphMatrixBuilder,
        role: GlyphRowRole,
        row: u32,
        clip: Rect,
    ) {
        let window_id = builder.current_window_id_i64();
        let slot_id = DisplaySlotId {
            window_id,
            row,
            col: self.col,
        };
        self.install_with_target(builder, window_id, role, Some(clip), slot_id);
    }

    fn install_frame_chrome(
        &self,
        builder: &mut GlyphMatrixBuilder,
        role: GlyphRowRole,
        row: u32,
        clip: Rect,
    ) {
        let slot_id = DisplaySlotId {
            window_id: FRAME_CHROME_WINDOW_ID,
            row,
            col: self.col,
        };
        self.install_with_target(builder, FRAME_CHROME_WINDOW_ID, role, Some(clip), slot_id);
    }

    fn install_with_target(
        &self,
        builder: &mut GlyphMatrixBuilder,
        window_id: i64,
        role: GlyphRowRole,
        clip: Option<Rect>,
        slot_id: DisplaySlotId,
    ) {
        match self.kind {
            RenderedDisplayRowMediaKind::Image { image_id } => {
                builder.artifact_installer().add_image_media(
                    window_id,
                    role,
                    clip,
                    slot_id,
                    image_id,
                    self.x,
                    self.y,
                    self.width,
                    self.height,
                );
            }
            RenderedDisplayRowMediaKind::Video {
                video_id,
                loop_count,
                autoplay,
            } => {
                builder.artifact_installer().add_video_media(
                    window_id,
                    role,
                    clip,
                    slot_id,
                    video_id,
                    loop_count,
                    autoplay,
                    self.x,
                    self.y,
                    self.width,
                    self.height,
                );
            }
            RenderedDisplayRowMediaKind::Xwidget { xwidget_id } => {
                builder.artifact_installer().add_xwidget_media(
                    window_id,
                    role,
                    clip,
                    slot_id,
                    xwidget_id,
                    self.x,
                    self.y,
                    self.width,
                    self.height,
                );
            }
        }
    }
}

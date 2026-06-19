use crate::composition::last_text_cluster_tail_in_row;
use crate::display_output_builder::{DisplayOutputBuilder, FRAME_CHROME_WINDOW_ID};
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
#[cfg(test)]
use crate::window_output::{TextRowOutput, WindowOutputEmitter};
use neomacs_display_protocol::face::Face;
use neomacs_display_protocol::frame_glyphs::{DisplaySlotId, GlyphRowRole};
use neomacs_display_protocol::glyph_matrix::{FrameChromeRow, GlyphRow};
use neomacs_display_protocol::types::Rect;
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
        let pixel_bounds = builder.current_window_pixel_bounds();
        let mut row = self.row.clone();
        if let Some(source_slots) = self.source_slots {
            apply_display_row_source_slot_bounds(&mut row, source_slots);
        }
        row.pixel_y = self.pixel_y - pixel_bounds.y;
        row.height_px = self.height_px;
        row.ascent_px = self.ascent_px;
        builder.install_complete_output_row(self.display_row_index, row.role, row.mode_line, row);
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

struct DisplayRowAssetsInstaller<'builder> {
    builder: &'builder mut DisplayOutputBuilder,
}

struct DisplayRowAssetsInstallSurface<'builder> {
    installer: DisplayRowAssetsInstaller<'builder>,
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

impl<'builder> DisplayRowAssetsInstaller<'builder> {
    fn new(builder: &'builder mut DisplayOutputBuilder) -> Self {
        Self { builder }
    }

    fn install_faces(&mut self, faces: &[Face]) {
        for face in faces {
            self.builder.install_output_face(face.id, face.clone());
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
        match medium.kind {
            RenderedDisplayRowMediaKind::Image { image_id } => {
                self.builder.add_output_image_media(
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
                self.builder.add_output_video_media(
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
                self.builder.add_output_xwidget_media(
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
        builder.install_output_face(face.id, face.clone());
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

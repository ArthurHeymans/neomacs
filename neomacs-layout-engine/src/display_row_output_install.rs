use crate::composition::last_text_cluster_tail_in_row;
use crate::display_output_builder::{DisplayOutputBuilder, FRAME_CHROME_WINDOW_ID};
use crate::display_output_install_request::{
    OutputFrameStateInstallRequest, OutputMediaInstallRequest, ResolvedOutputMediaInstallTarget,
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
use crate::display_row_text_output::TextRowOutput;
use crate::display_text_output_install::DisplayRowOutputInstall;
#[cfg(test)]
use crate::window_output::WindowOutputEmitter;
use neomacs_display_protocol::face::Face;
use neomacs_display_protocol::frame_glyphs::{DisplaySlotId, GlyphRowRole};
use neomacs_display_protocol::glyph_matrix::{FrameChromeRow, GlyphRow};
use neomacs_display_protocol::types::Rect;
#[cfg(test)]
use neovm_core::emacs_core::Context;

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
        crate::glyph_row_writer::finalize_external_row(&mut row);
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
    output_emitter.emit_text_output_spans(
        evaluator,
        output,
        output.spans_for_source_slots(&rendered.source_slots),
        end,
    );
    end
}

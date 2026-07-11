use crate::display_output_builder::{DisplayOutputBuilder, FRAME_CHROME_WINDOW_ID};
use crate::display_output_install_request::{
    OutputFrameStateInstallRequest, OutputMediaInstallRequest, ResolvedOutputMediaInstallTarget,
};
use crate::display_row_measured_state::{
    DisplayRowOwner, FrameChromeKind, MeasuredDisplayRow, WindowChromeKind,
};
use crate::display_row_render_state::{
    RenderedDisplayRow, RenderedDisplayRowMedia, RenderedDisplayRowMediaKind,
};
use neomacs_display_protocol::face::Face;
use neomacs_display_protocol::frame_chrome::{BandRect, ChromeDisplayRow, ChromeMedia};
use neomacs_display_protocol::frame_glyphs::{DisplaySlotId, GlyphRowRole};
use neomacs_display_protocol::types::{DisplayWindowId, ImageId, Rect, VideoId, XwidgetId};

pub(crate) fn install_measured_window_display_row(
    builder: &mut DisplayOutputBuilder,
    measured: &MeasuredDisplayRow,
) {
    MeasuredWindowDisplayRowInstallRequest { measured }.install(builder);
}

pub(crate) fn install_measured_frame_chrome_display_row(
    builder: &mut DisplayOutputBuilder,
    measured: &MeasuredDisplayRow,
) {
    MeasuredFrameChromeAssetsInstallRequest { measured }.install(builder);
}

pub(crate) fn frame_chrome_display_row(measured: &MeasuredDisplayRow) -> ChromeDisplayRow {
    let mut row = measured.frame_chrome_output_row();
    crate::display_row_finalizer::GlyphRowFinalizationContext::new(
        FRAME_CHROME_WINDOW_ID as u64,
        measured.row_index() as usize,
        measured.bounds(),
    )
    .finalize_row(&mut row, 0, None);
    let window_id = DisplayWindowId::new(FRAME_CHROME_WINDOW_ID);
    let media = measured
        .rendered()
        .media()
        .iter()
        .map(|medium| {
            let local_bounds = BandRect::new(
                medium.x,
                medium.y - measured.bounds().y,
                medium.width,
                medium.height,
            )
            .expect("rendered frame chrome media must have valid local bounds");
            let slot_id = Some(DisplaySlotId {
                window_id,
                row: measured.row_index(),
                col: medium.col,
            });
            match medium.kind {
                RenderedDisplayRowMediaKind::Image { image_id } => ChromeMedia::Image {
                    local_bounds,
                    image_id: ImageId::new(image_id),
                    slot_id,
                },
                RenderedDisplayRowMediaKind::Video {
                    video_id,
                    loop_count,
                    autoplay,
                } => ChromeMedia::Video {
                    local_bounds,
                    video_id: VideoId::new(video_id),
                    slot_id,
                    loop_count,
                    autoplay,
                },
                RenderedDisplayRowMediaKind::Xwidget { xwidget_id } => ChromeMedia::Xwidget {
                    local_bounds,
                    xwidget_id: XwidgetId::new(xwidget_id),
                    slot_id,
                },
            }
        })
        .collect();
    ChromeDisplayRow::new(row, media)
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

struct MeasuredWindowDisplayRowInstallRequest<'a> {
    measured: &'a MeasuredDisplayRow,
}

impl MeasuredWindowDisplayRowInstallRequest<'_> {
    fn install(self, builder: &mut DisplayOutputBuilder) {
        let measured = self.measured;
        let DisplayRowOwner::WindowChrome { window_id, kind } = measured.owner() else {
            panic!("frame chrome rows must install through frame chrome rows");
        };
        debug_assert!(window_id > 0);
        debug_assert_eq!(builder.current_window_id_i64(), window_id as i64);
        debug_assert!(matches!(
            kind,
            WindowChromeKind::TabLine | WindowChromeKind::HeaderLine | WindowChromeKind::ModeLine
        ));
        let display_row_index = measured.row_index() as usize;
        RenderedDisplayRowAssetsInstall::window_row(
            measured.rendered(),
            measured.row_index(),
            measured.bounds(),
        )
        .install(builder);
        let row = measured.window_relative_output_row(builder.current_window_pixel_bounds());
        builder.install_output_row_lifecycle(
            crate::display_output_row_request::OutputRowLifecycleRequest::complete(
                display_row_index,
                row.role,
                row.mode_line,
                row,
            ),
        );
    }
}

struct MeasuredFrameChromeAssetsInstallRequest<'a> {
    measured: &'a MeasuredDisplayRow,
}

impl MeasuredFrameChromeAssetsInstallRequest<'_> {
    fn install(self, builder: &mut DisplayOutputBuilder) {
        let measured = self.measured;
        let DisplayRowOwner::FrameChrome { kind } = measured.owner() else {
            panic!("window-owned rows must install through window chrome");
        };
        debug_assert!(matches!(kind, FrameChromeKind::TabBar));
        for face in measured.rendered().faces() {
            builder.install_output_frame_state(OutputFrameStateInstallRequest::face(
                face.id,
                face.clone(),
            ));
        }
    }
}

#[derive(Clone, Copy)]
enum RenderedDisplayRowAssetInstallTarget {
    CurrentWindowRow(usize),
    WindowRow { row_index: u32, bounds: Rect },
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
            role: rendered.row().role,
            faces: rendered.faces(),
            media: rendered.media(),
            target,
        }
    }

    fn window_row(rendered: &'a RenderedDisplayRow, row_index: u32, bounds: Rect) -> Self {
        Self::from_rendered(
            rendered,
            RenderedDisplayRowAssetInstallTarget::WindowRow { row_index, bounds },
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
                    ImageId::new(image_id),
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
                    VideoId::new(video_id),
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
                    XwidgetId::new(xwidget_id),
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
    window_id: DisplayWindowId,
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
                let window_id = DisplayWindowId::new(builder.current_window_id_i64());
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
                let window_id = DisplayWindowId::new(builder.current_window_id_i64());
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
        }
    }
}

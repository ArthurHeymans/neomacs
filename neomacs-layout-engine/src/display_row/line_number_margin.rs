//! Display-line-number left-margin rendering — GNU `maybe_produce_line_number` (xdisp.c:25447). Relocated out of display_row_append.rs (pure move, no behavior change).

use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayLength, DisplayStretch, DisplayStretchWidth,
    DisplayTextRun, RenderFaceRef, SourceSpan,
};
use crate::display_row::geometry::DisplayRowGeometryState;
use crate::display_row::source_render::TextRowSourceRenderState;
use crate::display_row::source_state::DisplayRowSourceState;
use crate::display_row::walk_state::{FaceScanCheckpoint, LineNumberRenderState};
use crate::display_source::{DisplayItemSource, DisplaySourceContext};
use crate::frame_face_arena::FrameFaceAttempt;
use neomacs_display_protocol::glyph_matrix::GlyphArea;
use neomacs_display_protocol::types::FaceId;

const LINE_NUMBER_MARGIN_SOURCE_ID: u64 = 0x6c6e_756d;

#[derive(Clone, Copy, Debug, PartialEq)]
struct TextWindowLineNumberMargin<'a> {
    text: &'a str,
    cols: i32,
    face_id: FaceId,
}

struct LineNumberMarginItemSource {
    items: std::vec::IntoIter<DisplayItem>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferLineNumberMarginRenderRequest {
    mode: u8,
    current_absolute: bool,
    offset: i64,
    major_tick: i32,
    cols: i32,
}

impl BufferLineNumberMarginRenderRequest {
    pub(crate) fn new(
        mode: u8,
        current_absolute: bool,
        offset: i64,
        major_tick: i32,
        cols: i32,
    ) -> Self {
        Self {
            mode,
            current_absolute,
            offset,
            major_tick,
            cols,
        }
    }

    pub(crate) fn render_pending_with_source_state(
        self,
        line_numbers: &mut LineNumberRenderState,
        source_render: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceAttempt,
        row_geometry: &DisplayRowGeometryState,
        face_scan: &mut FaceScanCheckpoint,
        char_width: f32,
    ) -> bool {
        let Some(line_number_request) = line_numbers.take_margin_render_request(
            self.mode,
            self.current_absolute,
            self.offset,
            self.major_tick,
            self.cols,
        ) else {
            return false;
        };

        let line_number_face =
            source_render.resolve_named_face(line_number_request.face().face_name());
        let line_number_face_id = crate::display_row::face_state::stable_face_id_for_resolved(
            face_ids,
            &line_number_face,
        );
        source_render.insert_resolved_face(line_number_face_id, &line_number_face);

        let text = line_number_request.text();
        let margin_request = TextWindowLineNumberMargin {
            text: &text,
            cols: line_number_request.cols(),
            face_id: line_number_face_id,
        };
        let mut source = LineNumberMarginItemSource::new(&margin_request);
        let mut source_state = DisplayRowSourceState::default();
        let margin_cols = margin_request.cols.max(1) as usize + 1;
        source_render.render_natural_fragment_from_row_geometry_columns(
            row_geometry,
            &mut source,
            &mut source_state,
            margin_cols,
            char_width,
            neomacs_display_protocol::frame_glyphs::GlyphRowRole::Text,
            line_number_face_id,
            &line_number_face,
            0,
            margin_cols,
            GlyphArea::LeftMargin,
            face_ids,
        );

        face_scan.invalidate();
        true
    }
}

fn line_number_margin_text_item(text: &str, face_id: FaceId, start_offset: usize) -> DisplayItem {
    let end_offset = start_offset.saturating_add(text.chars().count());
    DisplayItem::new(
        SourceSpan::synthetic(LINE_NUMBER_MARGIN_SOURCE_ID, start_offset, end_offset),
        RenderFaceRef::FaceId(face_id),
        DisplayItemKind::TextRun(DisplayTextRun::new(text.to_owned())),
    )
}

fn line_number_margin_stretch_item(cols: u16, face_id: FaceId, start_offset: usize) -> DisplayItem {
    DisplayItem::new(
        SourceSpan::synthetic(
            LINE_NUMBER_MARGIN_SOURCE_ID,
            start_offset,
            start_offset.saturating_add(usize::from(cols)),
        ),
        RenderFaceRef::FaceId(face_id),
        DisplayItemKind::Stretch(DisplayStretch {
            width: DisplayStretchWidth::Length(DisplayLength::Columns(cols)),
            height: None,
            ascent: None,
        }),
    )
}

impl LineNumberMarginItemSource {
    fn new(request: &TextWindowLineNumberMargin<'_>) -> Self {
        let mut items = Vec::new();
        let mut source_offset = 0usize;
        let padding = (request.cols - 1) - request.text.chars().count() as i32;
        if padding > 0 {
            let cols = padding.min(i32::from(u16::MAX)) as u16;
            items.push(line_number_margin_stretch_item(
                cols,
                request.face_id,
                source_offset,
            ));
            source_offset = source_offset.saturating_add(usize::from(cols));
        }
        if !request.text.is_empty() {
            items.push(line_number_margin_text_item(
                request.text,
                request.face_id,
                source_offset,
            ));
            source_offset = source_offset.saturating_add(request.text.chars().count());
        }
        items.push(line_number_margin_stretch_item(
            1,
            request.face_id,
            source_offset,
        ));
        Self {
            items: items.into_iter(),
        }
    }
}

impl DisplayItemSource for LineNumberMarginItemSource {
    fn next_item(&mut self, _context: &mut DisplaySourceContext<'_>) -> Option<DisplayItem> {
        self.items.next()
    }
}

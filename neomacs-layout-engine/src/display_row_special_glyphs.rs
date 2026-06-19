//! Right-edge truncation/continuation markers and the right window border — GNU's special glyphs (`produce_special_glyphs`, xdisp.c; `IT_TRUNCATION`/`IT_CONTINUATION`). Relocated out of display_row_append.rs (pure move, no behavior change).

use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayTextRun, RenderFaceRef, SourceSpan,
};
use crate::display_row::{DisplayRowSourceFragmentFrame, DisplayRowSourceState};
use crate::display_row_builder::{
    display_row_total_glyph_count, pop_display_row_trailing_text_char,
    trim_display_row_text_to_total_glyph_count,
};
use crate::display_row_geometry::DisplayRowFlagKind;
use crate::display_source::{DisplayItemSource, DisplaySourceContext, SyntheticTextItemSource};
use crate::display_status_line::ChromeRowRenderServices;
use crate::matrix_builder::{
    GlyphMatrixBuilder, MatrixCurrentWindowRowDecorationRequest,
    MatrixLastWindowRowsDecorationRequest, MatrixRowDecorator,
};
use crate::neovm_bridge::ResolvedFace;
use crate::window_output::{TextWindowRightBorder, TextWindowRightEdgeMarkers};
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{GlyphArea, GlyphRow};

const RIGHT_EDGE_MARKER_SOURCE_ID: u64 = 0x7265_6467;
const RIGHT_BORDER_SOURCE_ID: u64 = 0x7262_6f72;

fn right_border_text_source(
    text: impl Into<Box<str>>,
    face_id: u32,
    start_offset: usize,
) -> SyntheticTextItemSource {
    SyntheticTextItemSource::new(
        RIGHT_BORDER_SOURCE_ID,
        text,
        RenderFaceRef::FaceId(face_id),
        start_offset,
    )
}

struct RightEdgeMarkerItemSource {
    items: std::vec::IntoIter<DisplayItem>,
}

impl RightEdgeMarkerItemSource {
    fn new(padding_cols: usize, marker: char, face_id: u32) -> Self {
        let mut source_offset = 0usize;
        let mut items = Vec::with_capacity(usize::from(padding_cols > 0) + 1);
        if padding_cols > 0 {
            items.push(synthetic_special_glyph_text_item(
                RIGHT_EDGE_MARKER_SOURCE_ID,
                " ".repeat(padding_cols),
                face_id,
                source_offset,
            ));
            source_offset = source_offset.saturating_add(padding_cols);
        }
        items.push(synthetic_special_glyph_text_item(
            RIGHT_EDGE_MARKER_SOURCE_ID,
            marker.to_string(),
            face_id,
            source_offset,
        ));
        Self {
            items: items.into_iter(),
        }
    }
}

impl DisplayItemSource for RightEdgeMarkerItemSource {
    fn next_item(&mut self, _context: &mut DisplaySourceContext<'_>) -> Option<DisplayItem> {
        self.items.next()
    }
}

fn synthetic_special_glyph_text_item(
    source_id: u64,
    text: impl Into<Box<str>>,
    face_id: u32,
    start_offset: usize,
) -> DisplayItem {
    let text = text.into();
    let end_offset = start_offset.saturating_add(text.chars().count());
    DisplayItem::new(
        SourceSpan::synthetic(source_id, start_offset, end_offset),
        RenderFaceRef::FaceId(face_id),
        DisplayItemKind::TextRun(DisplayTextRun::new(text)),
    )
}

fn render_right_edge_marker_source(
    row: &mut GlyphRow,
    render_services: &mut ChromeRowRenderServices<'_, '_>,
    source: &mut RightEdgeMarkerItemSource,
    face_id: u32,
    base_face: &ResolvedFace,
    char_width: f32,
    matrix_cols: usize,
) {
    let start_col = display_row_total_glyph_count(row);
    let mut source_state = DisplayRowSourceState::default();
    let request = DisplayRowSourceFragmentFrame::from_glyph_row_columns(
        row,
        matrix_cols,
        char_width,
        GlyphRowRole::Text,
        face_id,
        base_face,
    )
    .render_request_from_column(start_col, matrix_cols);
    render_services.render_item_source_fragment_into_row(request, row, source, &mut source_state);
}

fn install_right_edge_marker_from_source_request(
    row: &mut GlyphRow,
    target_col: usize,
    marker: char,
    face_id: u32,
    base_face: &ResolvedFace,
    char_width: f32,
    matrix_cols: usize,
    render_services: &mut ChromeRowRenderServices<'_, '_>,
) {
    if matrix_cols == 0 {
        return;
    }
    row.enabled = true;
    let clamped_col = target_col.min(matrix_cols - 1);
    trim_display_row_text_to_total_glyph_count(row, clamped_col);

    let padding_cols = clamped_col.saturating_sub(display_row_total_glyph_count(row));
    let mut source = RightEdgeMarkerItemSource::new(padding_cols, marker, face_id);
    render_right_edge_marker_source(
        row,
        render_services,
        &mut source,
        face_id,
        base_face,
        char_width,
        matrix_cols,
    );
}

struct RightEdgeMarkerRowDecorator<'a, 'resolver, 'frame> {
    target_col: usize,
    marker: char,
    face_id: u32,
    base_face: &'a ResolvedFace,
    char_width: f32,
    render_services: &'a mut ChromeRowRenderServices<'resolver, 'frame>,
}

impl MatrixRowDecorator for RightEdgeMarkerRowDecorator<'_, '_, '_> {
    fn decorate_row(&mut self, row: &mut GlyphRow, matrix_cols: usize) {
        install_right_edge_marker_from_source_request(
            row,
            self.target_col,
            self.marker,
            self.face_id,
            self.base_face,
            self.char_width,
            matrix_cols,
            self.render_services,
        );
    }
}

pub(crate) fn install_right_edge_markers_from_source_requests(
    builder: &mut GlyphMatrixBuilder,
    mut render_services: ChromeRowRenderServices<'_, '_>,
    request: TextWindowRightEdgeMarkers<'_>,
) {
    let base_face = render_services.face_resolver().default_face().clone();
    let target_col = request.column.target_col(request.matrix_cols);
    for row_idx in 0..request.row_flags.len() {
        let matrix_row = request.text_matrix_row_base + row_idx;
        let marker = if request
            .row_flags
            .is_set(row_idx, DisplayRowFlagKind::Truncated)
        {
            Some('$')
        } else if request
            .row_flags
            .is_set(row_idx, DisplayRowFlagKind::Continued)
        {
            Some('\\')
        } else {
            None
        };
        let Some(marker) = marker else {
            continue;
        };
        builder.decorate_current_window_row(MatrixCurrentWindowRowDecorationRequest::new(
            matrix_row,
            RightEdgeMarkerRowDecorator {
                target_col,
                marker,
                face_id: request.face_id,
                base_face: &base_face,
                char_width: request.char_width,
                render_services: &mut render_services,
            },
        ));
    }
}

struct RightBorderTextRenderRequest<'face> {
    text: String,
    area: GlyphArea,
    face_id: u32,
    base_face: &'face ResolvedFace,
    char_width: f32,
    matrix_cols: usize,
    source_offset: usize,
    start_col: usize,
}

fn render_right_border_text(
    row: &mut GlyphRow,
    render_services: &mut ChromeRowRenderServices<'_, '_>,
    request: RightBorderTextRenderRequest<'_>,
) {
    if request.text.is_empty() {
        return;
    }
    let mut source = right_border_text_source(request.text, request.face_id, request.source_offset);
    let mut source_state = DisplayRowSourceState::default();
    let row_request = DisplayRowSourceFragmentFrame::from_glyph_row_columns(
        row,
        request.matrix_cols,
        request.char_width,
        GlyphRowRole::Text,
        request.face_id,
        request.base_face,
    )
    .render_request_from_column_for_area(request.start_col, request.matrix_cols, request.area);
    render_services.render_item_source_fragment_into_row(
        row_request,
        row,
        &mut source,
        &mut source_state,
    );
}

fn install_right_border_from_source_request(
    row: &mut GlyphRow,
    target_col: usize,
    request: TextWindowRightBorder,
    base_face: &ResolvedFace,
    matrix_cols: usize,
    render_services: &mut ChromeRowRenderServices<'_, '_>,
) {
    if matrix_cols == 0 {
        return;
    }

    let prior_displays_text = row.displays_text;
    row.enabled = true;
    let target_col = target_col.min(matrix_cols - 1);
    let preserved_trailing = pop_display_row_trailing_text_char(row, '$');
    let preserved_cols = usize::from(preserved_trailing.is_some());
    let before_final_cols = target_col.saturating_sub(preserved_cols);
    trim_display_row_text_to_total_glyph_count(row, before_final_cols);

    let mut source_offset = 0usize;
    let leading_padding = before_final_cols.saturating_sub(display_row_total_glyph_count(row));
    if leading_padding > 0 {
        render_right_border_text(
            row,
            render_services,
            RightBorderTextRenderRequest {
                text: " ".repeat(leading_padding),
                area: GlyphArea::Text,
                face_id: request.face_id,
                base_face,
                char_width: request.char_width,
                matrix_cols,
                source_offset,
                start_col: display_row_total_glyph_count(row),
            },
        );
        source_offset = source_offset.saturating_add(leading_padding);
    }

    if let Some(glyph) = preserved_trailing {
        render_right_border_text(
            row,
            render_services,
            RightBorderTextRenderRequest {
                text: "$".into(),
                area: GlyphArea::Text,
                face_id: glyph.face_id,
                base_face,
                char_width: request.char_width,
                matrix_cols,
                source_offset,
                start_col: display_row_total_glyph_count(row),
            },
        );
        source_offset = source_offset.saturating_add(preserved_cols);
    }

    let trailing_padding = target_col.saturating_sub(display_row_total_glyph_count(row));
    if trailing_padding > 0 {
        render_right_border_text(
            row,
            render_services,
            RightBorderTextRenderRequest {
                text: " ".repeat(trailing_padding),
                area: GlyphArea::Text,
                face_id: request.face_id,
                base_face,
                char_width: request.char_width,
                matrix_cols,
                source_offset,
                start_col: display_row_total_glyph_count(row),
            },
        );
        source_offset = source_offset.saturating_add(trailing_padding);
    }

    render_right_border_text(
        row,
        render_services,
        RightBorderTextRenderRequest {
            text: request.ch.to_string(),
            area: GlyphArea::RightMargin,
            face_id: request.face_id,
            base_face,
            char_width: request.char_width,
            matrix_cols,
            source_offset,
            start_col: target_col,
        },
    );
    row.displays_text = prior_displays_text;
}

struct RightBorderRowsDecorator<'a, 'resolver, 'frame> {
    request: TextWindowRightBorder,
    base_face: &'a ResolvedFace,
    render_services: &'a mut ChromeRowRenderServices<'resolver, 'frame>,
}

impl MatrixRowDecorator for RightBorderRowsDecorator<'_, '_, '_> {
    fn decorate_row(&mut self, row: &mut GlyphRow, matrix_cols: usize) {
        if matrix_cols == 0 {
            return;
        }
        install_right_border_from_source_request(
            row,
            matrix_cols - 1,
            self.request,
            self.base_face,
            matrix_cols,
            self.render_services,
        );
    }
}

pub(crate) fn install_last_window_right_border_from_source_requests(
    builder: &mut GlyphMatrixBuilder,
    mut render_services: ChromeRowRenderServices<'_, '_>,
    request: TextWindowRightBorder,
    base_face: &ResolvedFace,
) {
    builder.decorate_last_window_rows(MatrixLastWindowRowsDecorationRequest::new(
        RightBorderRowsDecorator {
            request,
            base_face,
            render_services: &mut render_services,
        },
    ));
}

//! Right-edge truncation/continuation markers and the right window border — GNU's special glyphs (`produce_special_glyphs`, xdisp.c; `IT_TRUNCATION`/`IT_CONTINUATION`). Relocated out of display_row_append.rs (pure move, no behavior change).

use crate::display_row::{
    DisplayRowGeometry, DisplayRowMaxX, DisplayRowRenderBounds,
    DisplayRowSourceFragmentRenderRequest, DisplayRowSourceRequestPolicy, DisplayRowSourceState,
};
use crate::display_row_builder::{
    DisplayRowPosition, DisplayTabPolicy, display_row_total_glyph_count,
    pop_display_row_trailing_text_char, trim_display_row_text_to_total_glyph_count,
};
use crate::display_row_geometry::DisplayRowFlagKind;
use crate::display_status_line::ChromeRowRenderServices;
use crate::matrix_builder::GlyphMatrixBuilder;
use crate::neovm_bridge::ResolvedFace;
use crate::window_output::{
    RightEdgeMarkerItemSource, TextWindowRightBorder, TextWindowRightEdgeMarkers,
    right_border_text_source,
};
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{GlyphArea, GlyphRow};

fn current_row_marker_geometry(
    row: &GlyphRow,
    width_cols: usize,
    char_width: f32,
) -> DisplayRowGeometry {
    let char_width = char_width.max(1.0);
    DisplayRowGeometry {
        y: row.pixel_y,
        width: width_cols.max(1) as f32 * char_width,
        height: row.height_px.max(1.0),
        ascent: row.ascent_px.max(0.0).min(row.height_px.max(1.0)),
        char_width,
        tab_policy: DisplayTabPolicy::every(8),
    }
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
    let char_width = char_width.max(1.0);
    let start_col = display_row_total_glyph_count(row);
    let start = DisplayRowPosition {
        x_px: start_col as f32 * char_width,
        col: start_col,
    };
    let mut source_state = DisplayRowSourceState::default();
    let request =
        DisplayRowSourceFragmentRenderRequest::from_base_face_id_policy_with_render_bounds(
            DisplayRowSourceRequestPolicy::from_display_row_geometry(
                current_row_marker_geometry(row, matrix_cols, char_width),
                GlyphRowRole::Text,
            ),
            face_id,
            base_face,
            DisplayRowRenderBounds {
                start,
                max_x: DisplayRowMaxX::Bounded(matrix_cols as f32 * char_width),
            },
        );
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
        let Some((mut row, matrix_cols)) = builder.current_window_row_snapshot(matrix_row) else {
            continue;
        };
        install_right_edge_marker_from_source_request(
            &mut row,
            target_col,
            marker,
            request.face_id,
            &base_face,
            request.char_width,
            matrix_cols,
            &mut render_services,
        );
        builder.replace_current_window_row(matrix_row, row);
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
    let char_width = request.char_width.max(1.0);
    let start = DisplayRowPosition {
        x_px: request.start_col as f32 * char_width,
        col: request.start_col,
    };
    let mut source = right_border_text_source(request.text, request.face_id, request.source_offset);
    let mut source_state = DisplayRowSourceState::default();
    let row_request =
        DisplayRowSourceFragmentRenderRequest::from_base_face_id_policy_with_render_bounds(
            DisplayRowSourceRequestPolicy::from_display_row_geometry(
                current_row_marker_geometry(row, request.matrix_cols, char_width),
                GlyphRowRole::Text,
            ),
            request.face_id,
            request.base_face,
            DisplayRowRenderBounds {
                start,
                max_x: DisplayRowMaxX::Bounded(request.matrix_cols as f32 * char_width),
            },
        )
        .with_glyph_area(request.area);
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

pub(crate) fn install_last_window_right_border_from_source_requests(
    builder: &mut GlyphMatrixBuilder,
    mut render_services: ChromeRowRenderServices<'_, '_>,
    request: TextWindowRightBorder,
    base_face: &ResolvedFace,
) {
    let Some((mut rows, matrix_cols)) = builder.last_window_rows_snapshot() else {
        return;
    };
    if matrix_cols == 0 {
        return;
    }
    let target_col = matrix_cols - 1;
    for row in &mut rows {
        install_right_border_from_source_request(
            row,
            target_col,
            request,
            base_face,
            matrix_cols,
            &mut render_services,
        );
    }
    builder.replace_last_window_rows(rows);
}

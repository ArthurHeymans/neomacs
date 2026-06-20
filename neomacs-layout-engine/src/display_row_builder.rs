use crate::composition::base_width_cols;
use crate::display_face_ref::render_face_ref_id;
use crate::display_item::{
    DisplayGlyphless, DisplayItem, DisplayItemKind, DisplayItemLayout, DisplayLength,
    DisplayLengthExpr, DisplaySourcePosition, DisplayStretch, DisplayStretchWidth, GlyphlessMethod,
    RenderFaceRef, SourceSpan, control_char_caret_char,
};
#[cfg(test)]
use crate::display_output_builder::DisplayOutputBuilder;
use crate::display_row_append_context::{
    DisplayRowTextCharState, DisplayRowTextNaturalAdvanceKind, DisplayRowTextNaturalAdvancePolicy,
    DisplayRowTextNaturalAdvanceRequest,
};
#[cfg(test)]
use crate::display_source::{DisplayItemSource, DisplaySourceContext};
use crate::glyph_row_writer;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{Glyph, GlyphArea, GlyphRow, GlyphType};
use neovm_core::buffer::{CharPos0, EmacsBytePos};

use crate::display_text_run_measurement::DisplayTextRunMeasurement;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplayRowLayout {
    pub(crate) role: GlyphRowRole,
    pub(crate) y_px: f32,
    pub(crate) width_px: f32,
    pub(crate) height_px: f32,
    pub(crate) ascent_px: f32,
    pub(crate) char_width_px: f32,
    pub(crate) tab_policy: DisplayTabPolicy,
    pub(crate) base_face: RenderFaceRef,
    pub(crate) symbol_values: std::collections::HashMap<String, DisplayLengthExpr>,
}

impl DisplayRowLayout {
    fn natural_text_advance_policy(&self) -> DisplayRowTextNaturalAdvancePolicy {
        DisplayRowTextNaturalAdvancePolicy::new(self.tab_policy.clone(), self.char_width_px)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplayTabPolicy {
    pub(crate) origin_x_px: f32,
    pub(crate) width_cols: u16,
    pub(crate) stop_cols: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayTabAdvance {
    pub(crate) pixel_width: f32,
    pub(crate) width_cols: usize,
}

#[derive(Clone, Copy, Debug)]
struct DisplayRowTextCharAdvanceRequest<'a> {
    char_state: DisplayRowTextCharState,
    face_id: u32,
    position: DisplayRowPosition,
    char_offset: usize,
    byte_offset: usize,
    measurement: &'a DisplayTextRunMeasurement,
}

impl<'a> DisplayRowTextCharAdvanceRequest<'a> {
    fn new(
        char_state: DisplayRowTextCharState,
        face_id: u32,
        position: DisplayRowPosition,
        char_offset: usize,
        byte_offset: usize,
        measurement: &'a DisplayTextRunMeasurement,
    ) -> Self {
        Self {
            char_state,
            face_id,
            position,
            char_offset,
            byte_offset,
            measurement,
        }
    }

    fn per_char(
        char_state: DisplayRowTextCharState,
        face_id: u32,
        position: DisplayRowPosition,
        measurement: &'a DisplayTextRunMeasurement,
    ) -> Self {
        Self::new(char_state, face_id, position, 0, 0, measurement)
    }

    fn ch(self) -> char {
        self.char_state.ch()
    }

    fn kind(self) -> DisplayRowTextNaturalAdvanceKind {
        self.char_state.kind()
    }

    fn natural_advance_request(self) -> DisplayRowTextNaturalAdvanceRequest {
        self.char_state
            .natural_advance_request(self.position, self.face_id)
    }

    fn measured_advance(self) -> Option<f32> {
        self.measurement
            .advance_for(self.char_offset, self.byte_offset)
    }

    fn resolve_advance_px(
        self,
        policy: &DisplayRowTextNaturalAdvancePolicy,
        glyph_advance_px: impl FnMut(char, u32, usize) -> f32,
    ) -> f32 {
        let measured = match self.kind() {
            DisplayRowTextNaturalAdvanceKind::Tab
            | DisplayRowTextNaturalAdvanceKind::ClusterContinuation => None,
            DisplayRowTextNaturalAdvanceKind::ComplexRunMember
            | DisplayRowTextNaturalAdvanceKind::FaceColumns { .. } => self.measured_advance(),
        };
        measured.unwrap_or_else(|| {
            policy.resolve_with(self.natural_advance_request(), glyph_advance_px)
        })
    }

    fn resolve_advance_px_with_writer(self, writer: &mut DisplayRowWriter<'_, '_, '_>) -> f32 {
        let policy = writer.layout.natural_text_advance_policy();
        self.resolve_advance_px(&policy, |ch, face_id, columns| {
            writer.glyph_advance_px(ch, face_id, columns)
        })
    }
}

impl DisplayTabPolicy {
    pub(crate) fn every(width_cols: u16) -> Self {
        Self {
            origin_x_px: 0.0,
            width_cols: width_cols.max(1),
            stop_cols: Vec::new(),
        }
    }

    pub(crate) fn from_tab_width_and_stops(
        origin_x_px: f32,
        tab_width: i32,
        tab_stop_list: &[i32],
    ) -> Self {
        Self {
            origin_x_px,
            width_cols: tab_width.max(1).min(i32::from(u16::MAX)) as u16,
            stop_cols: tab_stop_list
                .iter()
                .copied()
                .filter(|stop| *stop >= 0)
                .map(|stop| stop as usize)
                .collect(),
        }
    }

    pub(crate) fn advance_from(
        &self,
        position: DisplayRowPosition,
        char_width_px: f32,
    ) -> DisplayTabAdvance {
        let char_width_px = char_width_px.max(1.0);
        let tab_width_px = f32::from(self.width_cols.max(1)) * char_width_px;
        let tab_x_px = (position.x_px - self.origin_x_px).max(0.0);
        let next_tab_x_px = if !self.stop_cols.is_empty() {
            self.stop_cols
                .iter()
                .copied()
                .map(|stop| stop as f32 * char_width_px)
                .find(|stop_px| *stop_px > tab_x_px)
                .unwrap_or_else(|| {
                    let last =
                        self.stop_cols.last().copied().unwrap_or_default() as f32 * char_width_px;
                    if tab_x_px >= last && tab_width_px > 0.0 {
                        last + ((tab_x_px - last) / tab_width_px).floor() * tab_width_px
                            + tab_width_px
                    } else {
                        last
                    }
                })
        } else if tab_width_px > 0.0 {
            ((tab_x_px / tab_width_px).floor() + 1.0) * tab_width_px
        } else {
            tab_x_px + char_width_px
        };
        let next_tab_x_px = if next_tab_x_px - tab_x_px < char_width_px {
            next_tab_x_px + tab_width_px
        } else {
            next_tab_x_px
        };
        let pixel_width = (next_tab_x_px - tab_x_px).max(char_width_px);
        let next_col = ((next_tab_x_px / char_width_px).round() as usize).max(position.col + 1);
        DisplayTabAdvance {
            pixel_width,
            width_cols: next_col.saturating_sub(position.col).max(1),
        }
    }
}

pub(crate) trait DisplayGlyphMeasurer {
    fn glyph_advance_px(
        &mut self,
        ch: char,
        face_id: u32,
        columns: u8,
        fallback_advance_px: f32,
    ) -> Option<f32>;

    fn text_run_advances_px(
        &mut self,
        _text: &str,
        _face_id: u32,
        _fallback_char_width_px: f32,
    ) -> DisplayTextRunMeasurement {
        DisplayTextRunMeasurement::PerChar
    }
}

pub(crate) enum DisplayRowItemMeasurement {
    Default,
    TextRun(DisplayTextRunMeasurement),
}

#[cfg(test)]
pub(crate) struct FixedGlyphAdvance {
    ch: char,
    face_id: u32,
    advance_px: f32,
}

#[cfg(test)]
impl FixedGlyphAdvance {
    pub(crate) fn new(ch: char, face_id: u32, advance_px: f32) -> Self {
        Self {
            ch,
            face_id,
            advance_px,
        }
    }
}

#[cfg(test)]
impl DisplayGlyphMeasurer for FixedGlyphAdvance {
    fn glyph_advance_px(
        &mut self,
        ch: char,
        face_id: u32,
        _columns: u8,
        _fallback_advance_px: f32,
    ) -> Option<f32> {
        (self.ch == ch && self.face_id == face_id).then_some(self.advance_px)
    }
}

#[cfg(test)]
#[derive(Default)]
pub(crate) struct FixedGlyphAdvances {
    advances: std::collections::HashMap<(char, u32), f32>,
}

#[cfg(test)]
impl FixedGlyphAdvances {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn insert(&mut self, ch: char, face_id: u32, advance_px: f32) {
        self.advances.insert((ch, face_id), advance_px);
    }
}

#[cfg(test)]
impl DisplayGlyphMeasurer for FixedGlyphAdvances {
    fn glyph_advance_px(
        &mut self,
        ch: char,
        face_id: u32,
        _columns: u8,
        _fallback_advance_px: f32,
    ) -> Option<f32> {
        self.advances.get(&(ch, face_id)).copied()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct DisplayRowWriteMetrics {
    pub(crate) width_px: f32,
    pub(crate) width_cols: usize,
}

impl DisplayRowWriteMetrics {
    fn from_glyphs(glyphs: &[Glyph], char_width_px: f32) -> Self {
        glyphs.iter().fold(Self::default(), |mut metrics, glyph| {
            let width_cols = match &glyph.glyph_type {
                GlyphType::Stretch { width_cols } => usize::from((*width_cols).max(1)),
                GlyphType::Glyphless { .. } if glyph.pixel_width > 0.0 => {
                    (glyph.pixel_width / char_width_px.max(1.0)).ceil().max(1.0) as usize
                }
                // A composed grapheme cluster advances the column by GNU's
                // `cmp->width` (= `string-width` of the cluster), not a single
                // cell — combining marks within it contribute 0.
                GlyphType::Composite { text } => crate::composition::composed_cluster_cols(text),
                _ if glyph.padding && glyph.pixel_width <= 0.0 => 0,
                _ if glyph.wide => 2,
                _ => 1,
            };
            let width_px = if glyph.pixel_width > 0.0 {
                glyph.pixel_width
            } else {
                width_cols as f32 * char_width_px.max(1.0)
            };
            metrics.width_cols += width_cols;
            metrics.width_px += width_px;
            metrics
        })
    }

    fn add(&mut self, other: Self) {
        self.width_px += other.width_px;
        self.width_cols += other.width_cols;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DisplayRowGlyphCheckpoint {
    area_lengths: [usize; 3],
    displays_text: bool,
}

impl DisplayRowGlyphCheckpoint {
    fn capture(row: &GlyphRow) -> Self {
        Self {
            area_lengths: [
                row.glyphs[GlyphArea::LeftMargin.index()].len(),
                row.glyphs[GlyphArea::Text.index()].len(),
                row.glyphs[GlyphArea::RightMargin.index()].len(),
            ],
            displays_text: row.displays_text,
        }
    }

    fn restore(self, row: &mut GlyphRow) {
        row.glyphs[GlyphArea::LeftMargin.index()].truncate(self.area_lengths[0]);
        row.glyphs[GlyphArea::Text.index()].truncate(self.area_lengths[1]);
        row.glyphs[GlyphArea::RightMargin.index()].truncate(self.area_lengths[2]);
        row.displays_text = self.displays_text;
    }
}

pub(crate) fn new_display_row(layout: &DisplayRowLayout) -> GlyphRow {
    let mut row = new_display_row_for_role(layout.role);
    row.pixel_y = layout.y_px;
    row.height_px = layout.height_px.max(1.0);
    row.ascent_px = layout.ascent_px.max(0.0).min(row.height_px);
    row
}

pub(crate) fn new_display_row_for_role(role: GlyphRowRole) -> GlyphRow {
    let mut row = GlyphRow::new(role);
    row.enabled = true;
    row
}

pub(crate) fn display_row_text_glyph_count(row: &GlyphRow) -> usize {
    row.glyphs[GlyphArea::Text.index()].len()
}

pub(crate) fn display_row_text_is_empty(row: &GlyphRow) -> bool {
    display_row_text_glyph_count(row) == 0
}

pub(crate) fn display_row_total_glyph_count(row: &GlyphRow) -> usize {
    row.glyphs[GlyphArea::LeftMargin.index()].len()
        + row.glyphs[GlyphArea::Text.index()].len()
        + row.glyphs[GlyphArea::RightMargin.index()].len()
}

pub(crate) fn trim_display_row_text_to_total_glyph_count(row: &mut GlyphRow, target: usize) {
    while display_row_total_glyph_count(row) > target {
        let text_area = &mut row.glyphs[GlyphArea::Text.index()];
        if text_area.is_empty() {
            break;
        }
        text_area.pop();
    }
}

pub(crate) fn pop_display_row_trailing_text_char(row: &mut GlyphRow, ch: char) -> Option<Glyph> {
    if row.glyphs[GlyphArea::Text.index()].last().is_some_and(
        |glyph| matches!(glyph.glyph_type, GlyphType::Char { ch: glyph_ch } if glyph_ch == ch),
    ) {
        row.glyphs[GlyphArea::Text.index()].pop()
    } else {
        None
    }
}

pub(crate) fn apply_display_row_source_slot_bounds(
    row: &mut GlyphRow,
    slots: &[DisplayRowGlyphSlot],
) {
    let Some((start, end)) = display_row_buffer_source_slot_bounds(slots) else {
        return;
    };
    set_display_row_buffer_source_bounds(row, start, end);
}

pub(crate) fn merge_display_row_source_slot_bounds(
    row: &mut GlyphRow,
    slots: &[DisplayRowGlyphSlot],
) {
    let Some((start, end)) = display_row_buffer_source_slot_bounds(slots) else {
        return;
    };
    merge_display_row_buffer_source_bounds(row, start, end);
}

fn display_row_buffer_source_slot_bounds(slots: &[DisplayRowGlyphSlot]) -> Option<(usize, usize)> {
    slots.iter().fold(None::<(usize, usize)>, |bounds, slot| {
        let DisplaySourcePosition::Buffer { char_pos, .. } = slot.source else {
            return bounds;
        };
        let start = char_pos.get();
        let end = start.saturating_add(1);
        Some(match bounds {
            Some((old_start, old_end)) => (old_start.min(start), old_end.max(end)),
            None => (start, end),
        })
    })
}

fn merge_display_row_buffer_source_bounds(row: &mut GlyphRow, start: usize, end: usize) {
    if row.start_charpos == row.end_charpos {
        set_display_row_buffer_source_bounds(row, start, end);
        return;
    }
    set_display_row_buffer_source_bounds(
        row,
        row.start_charpos.min(start),
        row.end_charpos.max(end),
    );
}

fn set_display_row_buffer_source_bounds(row: &mut GlyphRow, start: usize, end: usize) {
    row.start_charpos = start;
    row.end_charpos = end;
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct DisplayRowPosition {
    pub(crate) x_px: f32,
    pub(crate) col: usize,
}

fn append_start_position(
    requested: DisplayRowPosition,
    current_tail: DisplayRowPosition,
) -> DisplayRowPosition {
    if current_tail.col > requested.col || current_tail.x_px > requested.x_px {
        current_tail
    } else {
        requested
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DisplayRowAppendStatus {
    Complete,
    Clipped,
    RowBreak,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplayRowGlyphSlot {
    pub(crate) source: DisplaySourcePosition,
    pub(crate) x_px: f32,
    pub(crate) col: usize,
    pub(crate) width_px: f32,
    pub(crate) width_cols: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplayRowAppendProgress {
    pub(crate) start: DisplayRowPosition,
    pub(crate) end: DisplayRowPosition,
    pub(crate) metrics: DisplayRowWriteMetrics,
    pub(crate) status: DisplayRowAppendStatus,
    pub(crate) slots: Vec<DisplayRowGlyphSlot>,
}

impl DisplayRowAppendProgress {
    pub(crate) fn new(
        start: DisplayRowPosition,
        end: DisplayRowPosition,
        metrics: DisplayRowWriteMetrics,
        status: DisplayRowAppendStatus,
        slots: Vec<DisplayRowGlyphSlot>,
    ) -> Self {
        Self {
            start,
            end,
            metrics,
            status,
            slots,
        }
    }

    pub(crate) fn from_positions(
        start: DisplayRowPosition,
        end: DisplayRowPosition,
        status: DisplayRowAppendStatus,
        slots: Vec<DisplayRowGlyphSlot>,
    ) -> Self {
        Self::new(
            start,
            end,
            DisplayRowWriteMetrics {
                width_px: (end.x_px - start.x_px).max(0.0),
                width_cols: end.col.saturating_sub(start.col),
            },
            status,
            slots,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DisplayTextSourceMapping {
    NaturalText,
    SourceMapped,
}

impl DisplayTextSourceMapping {
    fn charpos(self, start_char: usize, char_offset: usize) -> usize {
        match self {
            Self::NaturalText => start_char + char_offset,
            Self::SourceMapped => start_char,
        }
    }

    fn slot_source(
        self,
        span_start: &DisplaySourcePosition,
        char_offset: usize,
        byte_offset: usize,
    ) -> DisplaySourcePosition {
        match self {
            Self::NaturalText => source_position_advance(span_start, char_offset, byte_offset),
            Self::SourceMapped => span_start.clone(),
        }
    }
}

#[cfg(test)]
struct DisplayRowBuilder<'a> {
    layout: DisplayRowLayout,
    row: GlyphRow,
    glyph_measurer: Option<&'a mut dyn DisplayGlyphMeasurer>,
}

struct DisplayRowWriter<'layout, 'row, 'measurer> {
    layout: &'layout DisplayRowLayout,
    row: &'row mut GlyphRow,
    glyph_measurer: Option<&'measurer mut dyn DisplayGlyphMeasurer>,
    area_index: usize,
}

pub(crate) struct DisplayRowProgressWriter<'layout, 'row, 'measurer> {
    writer: DisplayRowWriter<'layout, 'row, 'measurer>,
    position: DisplayRowPosition,
    max_x_px: f32,
    text_run_measurement: Option<DisplayTextRunMeasurement>,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct DisplayRowAppendCursor {
    position: DisplayRowPosition,
    max_x_px: f32,
}

#[cfg(test)]
impl DisplayRowAppendCursor {
    fn new(position: DisplayRowPosition, max_x_px: f32) -> Self {
        Self { position, max_x_px }
    }

    fn position(&self) -> DisplayRowPosition {
        self.position
    }

    #[cfg(test)]
    fn append_item_to_current_text_row(
        &mut self,
        builder: &mut DisplayOutputBuilder,
        layout: &DisplayRowLayout,
        item: DisplayItem,
    ) -> Option<DisplayRowAppendProgress> {
        let progress = append_display_item_to_current_text_row(
            builder,
            layout,
            item,
            self.position,
            self.max_x_px,
        )?;
        self.position = progress.end;
        Some(progress)
    }

    fn append_measured_item_to_current_text_row(
        &mut self,
        builder: &mut DisplayOutputBuilder,
        layout: &DisplayRowLayout,
        item: DisplayItem,
        glyph_measurer: &mut dyn DisplayGlyphMeasurer,
    ) -> Option<DisplayRowAppendProgress> {
        let progress = append_measured_display_item_to_current_text_row(
            builder,
            layout,
            item,
            glyph_measurer,
            self.position,
            self.max_x_px,
        )?;
        self.position = progress.end;
        Some(progress)
    }
}

#[cfg(test)]
fn append_display_item_to_current_text_row(
    builder: &mut DisplayOutputBuilder,
    layout: &DisplayRowLayout,
    item: DisplayItem,
    position: DisplayRowPosition,
    max_x_px: f32,
) -> Option<DisplayRowAppendProgress> {
    builder.edit_current_row_for_test(|row| {
        let mut writer = DisplayRowProgressWriter::new(layout, row, position, max_x_px);
        writer.push_item(item)
    })
}

#[cfg(test)]
fn append_measured_display_item_to_current_text_row(
    builder: &mut DisplayOutputBuilder,
    layout: &DisplayRowLayout,
    item: DisplayItem,
    glyph_measurer: &mut dyn DisplayGlyphMeasurer,
    position: DisplayRowPosition,
    max_x_px: f32,
) -> Option<DisplayRowAppendProgress> {
    builder.edit_current_row_for_test(|row| {
        let mut writer = DisplayRowProgressWriter::with_glyph_measurer(
            layout,
            row,
            glyph_measurer,
            position,
            max_x_px,
        );
        writer.push_item(item)
    })
}

#[cfg(test)]
impl DisplayRowBuilder<'_> {
    fn new(layout: DisplayRowLayout) -> Self {
        let row = new_display_row(&layout);
        Self {
            layout,
            row,
            glyph_measurer: None,
        }
    }
}

#[cfg(test)]
impl<'a> DisplayRowBuilder<'a> {
    #[cfg(test)]
    fn with_glyph_measurer(
        layout: DisplayRowLayout,
        glyph_measurer: &'a mut dyn DisplayGlyphMeasurer,
    ) -> Self {
        let mut builder = Self::new(layout);
        builder.glyph_measurer = Some(glyph_measurer);
        builder
    }

    fn push_item(&mut self, item: DisplayItem) -> DisplayRowWriteMetrics {
        if let Some(glyph_measurer) = self.glyph_measurer.as_deref_mut() {
            let mut writer =
                DisplayRowWriter::with_glyph_measurer(&self.layout, &mut self.row, glyph_measurer);
            writer.push_item(item)
        } else {
            let mut writer = DisplayRowWriter::new(&self.layout, &mut self.row);
            writer.push_item(item)
        }
    }

    fn push_measured_item(
        &mut self,
        item: DisplayItem,
        glyph_measurer: &mut dyn DisplayGlyphMeasurer,
    ) -> DisplayRowWriteMetrics {
        let mut writer =
            DisplayRowWriter::with_glyph_measurer(&self.layout, &mut self.row, glyph_measurer);
        writer.push_item(item)
    }

    fn push_source(
        &mut self,
        source: &mut impl DisplayItemSource,
        context: &mut DisplaySourceContext<'_>,
    ) -> DisplayRowWriteMetrics {
        if let Some(glyph_measurer) = self.glyph_measurer.as_deref_mut() {
            let mut writer =
                DisplayRowWriter::with_glyph_measurer(&self.layout, &mut self.row, glyph_measurer);
            writer.push_source(source, context)
        } else {
            let mut writer = DisplayRowWriter::new(&self.layout, &mut self.row);
            writer.push_source(source, context)
        }
    }

    fn finish(mut self) -> GlyphRow {
        glyph_row_writer::normalize_external_row(&mut self.row);
        self.row
    }
}

impl<'layout, 'row> DisplayRowProgressWriter<'layout, 'row, '_> {
    #[cfg(test)]
    pub(crate) fn new(
        layout: &'layout DisplayRowLayout,
        row: &'row mut GlyphRow,
        position: DisplayRowPosition,
        max_x_px: f32,
    ) -> Self {
        let writer = DisplayRowWriter::new(layout, row);
        let position = append_start_position(position, writer.current_text_position());
        Self {
            writer,
            position,
            max_x_px,
            text_run_measurement: None,
        }
    }
}

impl<'layout, 'row, 'measurer> DisplayRowProgressWriter<'layout, 'row, 'measurer> {
    #[cfg(test)]
    pub(crate) fn with_glyph_measurer(
        layout: &'layout DisplayRowLayout,
        row: &'row mut GlyphRow,
        glyph_measurer: &'measurer mut dyn DisplayGlyphMeasurer,
        position: DisplayRowPosition,
        max_x_px: f32,
    ) -> Self {
        Self::with_glyph_measurer_for_area(
            layout,
            row,
            glyph_measurer,
            position,
            max_x_px,
            GlyphArea::Text,
        )
    }

    pub(crate) fn with_glyph_measurer_for_area(
        layout: &'layout DisplayRowLayout,
        row: &'row mut GlyphRow,
        glyph_measurer: &'measurer mut dyn DisplayGlyphMeasurer,
        position: DisplayRowPosition,
        max_x_px: f32,
        area: GlyphArea,
    ) -> Self {
        let writer =
            DisplayRowWriter::with_glyph_measurer_for_area(layout, row, glyph_measurer, area);
        let position = append_start_position(position, writer.current_text_position());
        Self {
            writer,
            position,
            max_x_px,
            text_run_measurement: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_text_run_measurement(
        layout: &'layout DisplayRowLayout,
        row: &'row mut GlyphRow,
        text_run_measurement: DisplayTextRunMeasurement,
        position: DisplayRowPosition,
        max_x_px: f32,
    ) -> Self {
        Self::with_text_run_measurement_for_area(
            layout,
            row,
            text_run_measurement,
            position,
            max_x_px,
            GlyphArea::Text,
        )
    }

    pub(crate) fn with_text_run_measurement_for_area(
        layout: &'layout DisplayRowLayout,
        row: &'row mut GlyphRow,
        text_run_measurement: DisplayTextRunMeasurement,
        position: DisplayRowPosition,
        max_x_px: f32,
        area: GlyphArea,
    ) -> Self {
        let writer = DisplayRowWriter::for_area(layout, row, area);
        let position = append_start_position(position, writer.current_text_position());
        Self {
            writer,
            position,
            max_x_px,
            text_run_measurement: Some(text_run_measurement),
        }
    }

    #[cfg(test)]
    pub(crate) fn position(&self) -> DisplayRowPosition {
        self.position
    }

    pub(crate) fn push_item(&mut self, item: DisplayItem) -> DisplayRowAppendProgress {
        let start = self.position;
        let mut metrics = DisplayRowWriteMetrics::default();
        let mut slots = Vec::new();
        let DisplayItem {
            span,
            face,
            kind,
            layout: item_layout,
        } = item;
        let status = match kind {
            DisplayItemKind::RowBreak(_) => DisplayRowAppendStatus::RowBreak,
            DisplayItemKind::TextRun(run) => self.push_text_item(
                &span,
                face,
                item_layout,
                run.text.as_ref(),
                DisplayTextSourceMapping::NaturalText,
                &mut metrics,
                &mut slots,
            ),
            DisplayItemKind::SourceMappedText(text) => self.push_text_item(
                &span,
                face,
                item_layout,
                text.text.as_ref(),
                DisplayTextSourceMapping::SourceMapped,
                &mut metrics,
                &mut slots,
            ),
            kind => {
                let slot_start = self.position;
                let slot_source = span.start.clone();
                let checkpoint = DisplayRowGlyphCheckpoint::capture(self.writer.row);
                let written = self
                    .writer
                    .push_item(DisplayItem::new(span, face, kind).with_layout(item_layout));
                if written.width_px > 0.0 && self.position.x_px + written.width_px > self.max_x_px {
                    checkpoint.restore(self.writer.row);
                    return DisplayRowAppendProgress::new(
                        start,
                        self.position,
                        metrics,
                        DisplayRowAppendStatus::Clipped,
                        slots,
                    );
                }
                if written.width_px > 0.0 || written.width_cols > 0 {
                    slots.push(DisplayRowGlyphSlot {
                        source: slot_source,
                        x_px: slot_start.x_px,
                        col: slot_start.col,
                        width_px: written.width_px,
                        width_cols: written.width_cols,
                    });
                }
                self.advance(written);
                metrics.add(written);
                DisplayRowAppendStatus::Complete
            }
        };

        DisplayRowAppendProgress::new(start, self.position, metrics, status, slots)
    }

    fn push_text_item(
        &mut self,
        span: &SourceSpan,
        face: RenderFaceRef,
        item_layout: DisplayItemLayout,
        text: &str,
        source_mapping: DisplayTextSourceMapping,
        metrics: &mut DisplayRowWriteMetrics,
        slots: &mut Vec<DisplayRowGlyphSlot>,
    ) -> DisplayRowAppendStatus {
        let face_id = self.writer.face_id(face);
        let measurement = self.text_run_measurement(text, face_id);
        let start_char = source_span_start_char(span);
        let mut status = DisplayRowAppendStatus::Complete;
        let mut char_offset = 0usize;
        let mut byte_offset = 0usize;
        for ch in text.chars() {
            let char_state = self.writer.text_char_state(ch);
            let advance_request = DisplayRowTextCharAdvanceRequest::new(
                char_state,
                face_id,
                self.position,
                char_offset,
                byte_offset,
                &measurement,
            );
            let advance = advance_request.resolve_advance_px_with_writer(&mut self.writer);
            if advance > 0.0 && self.position.x_px + advance > self.max_x_px {
                status = DisplayRowAppendStatus::Clipped;
                break;
            }

            let before_len = self.area_len();
            let slot_start = self.position;
            self.writer.push_text_char_state_with_advance_request(
                source_mapping.charpos(start_char, char_offset),
                advance_request,
            );
            self.writer.apply_item_layout_since(before_len, item_layout);
            let written = self.metrics_since(before_len);
            slots.push(DisplayRowGlyphSlot {
                source: source_mapping.slot_source(&span.start, char_offset, byte_offset),
                x_px: slot_start.x_px,
                col: slot_start.col,
                width_px: written.width_px,
                width_cols: written.width_cols,
            });
            self.advance(written);
            metrics.add(written);
            char_offset += 1;
            byte_offset += ch.len_utf8();
        }
        status
    }

    fn area_len(&self) -> usize {
        self.writer.row.glyphs[self.writer.area_index].len()
    }

    fn metrics_since(&self, before_len: usize) -> DisplayRowWriteMetrics {
        DisplayRowWriteMetrics::from_glyphs(
            &self.writer.row.glyphs[self.writer.area_index][before_len..],
            self.writer.layout.char_width_px,
        )
    }

    fn advance(&mut self, metrics: DisplayRowWriteMetrics) {
        self.position.x_px += metrics.width_px;
        self.position.col += metrics.width_cols;
    }

    fn text_run_measurement(&mut self, text: &str, face_id: u32) -> DisplayTextRunMeasurement {
        self.text_run_measurement
            .clone()
            .unwrap_or_else(|| self.writer.text_run_measurement(text, face_id))
    }
}

impl<'layout, 'row, 'measurer> DisplayRowWriter<'layout, 'row, 'measurer> {
    #[cfg(test)]
    fn new(layout: &'layout DisplayRowLayout, row: &'row mut GlyphRow) -> Self {
        Self::for_area(layout, row, GlyphArea::Text)
    }

    fn for_area(
        layout: &'layout DisplayRowLayout,
        row: &'row mut GlyphRow,
        area: GlyphArea,
    ) -> Self {
        row.enabled = true;
        row.role = layout.role;
        row.mode_line = matches!(layout.role, GlyphRowRole::ModeLine);
        row.pixel_y = layout.y_px;
        row.height_px = layout.height_px.max(1.0);
        row.ascent_px = layout.ascent_px.max(0.0).min(row.height_px);
        Self {
            layout,
            row,
            glyph_measurer: None,
            area_index: area.index(),
        }
    }

    #[cfg(test)]
    fn with_glyph_measurer(
        layout: &'layout DisplayRowLayout,
        row: &'row mut GlyphRow,
        glyph_measurer: &'measurer mut dyn DisplayGlyphMeasurer,
    ) -> Self {
        Self::with_glyph_measurer_for_area(layout, row, glyph_measurer, GlyphArea::Text)
    }

    fn with_glyph_measurer_for_area(
        layout: &'layout DisplayRowLayout,
        row: &'row mut GlyphRow,
        glyph_measurer: &'measurer mut dyn DisplayGlyphMeasurer,
        area: GlyphArea,
    ) -> Self {
        row.enabled = true;
        row.role = layout.role;
        row.mode_line = matches!(layout.role, GlyphRowRole::ModeLine);
        row.pixel_y = layout.y_px;
        row.height_px = layout.height_px.max(1.0);
        row.ascent_px = layout.ascent_px.max(0.0).min(row.height_px);
        Self {
            layout,
            row,
            glyph_measurer: Some(glyph_measurer),
            area_index: area.index(),
        }
    }

    fn push_item(&mut self, item: DisplayItem) -> DisplayRowWriteMetrics {
        let item_layout = item.layout;
        let face_id = self.face_id(item.face);
        let area_index = self.area_index;
        let before_len = self.row.glyphs[area_index].len();
        match item.kind {
            DisplayItemKind::TextRun(run) => {
                self.push_text_item(
                    run.text.as_ref(),
                    face_id,
                    &item.span,
                    DisplayTextSourceMapping::NaturalText,
                );
            }
            DisplayItemKind::SourceMappedText(text) => {
                self.push_text_item(
                    text.text.as_ref(),
                    face_id,
                    &item.span,
                    DisplayTextSourceMapping::SourceMapped,
                );
            }
            DisplayItemKind::Stretch(stretch) => self.push_stretch(stretch, face_id),
            DisplayItemKind::MediaReplacement(media) => {
                self.push_stretch(media.replacement_stretch(), face_id);
            }
            DisplayItemKind::ControlChar { ch } => {
                self.push_control_char(ch, face_id, source_span_start_char(&item.span));
            }
            DisplayItemKind::Glyphless(glyphless) => {
                self.push_glyphless(glyphless, face_id, source_span_start_char(&item.span));
            }
            DisplayItemKind::RowBreak(_)
            | DisplayItemKind::BufferDisplayPropertyReplacement(_)
            | DisplayItemKind::CursorAnchor(_)
            | DisplayItemKind::HitTestAnchor(_) => {}
        }
        self.apply_item_layout_since(before_len, item_layout);
        DisplayRowWriteMetrics::from_glyphs(
            &self.row.glyphs[area_index][before_len..],
            self.layout.char_width_px,
        )
    }

    #[cfg(test)]
    fn push_source(
        &mut self,
        source: &mut impl DisplayItemSource,
        context: &mut DisplaySourceContext<'_>,
    ) -> DisplayRowWriteMetrics {
        let mut metrics = DisplayRowWriteMetrics::default();
        while let Some(item) = source.next_item(context) {
            metrics.add(self.push_item(item));
        }
        metrics
    }

    fn push_text_item(
        &mut self,
        text: &str,
        face_id: u32,
        span: &SourceSpan,
        source_mapping: DisplayTextSourceMapping,
    ) {
        let measurement = self.text_run_measurement(text, face_id);
        let start_char = source_span_start_char(span);
        let mut char_offset = 0usize;
        let mut byte_offset = 0usize;
        for ch in text.chars() {
            let char_state = self.text_char_state(ch);
            self.push_text_char_with_measurement(
                char_state,
                face_id,
                source_mapping.charpos(start_char, char_offset),
                char_offset,
                byte_offset,
                &measurement,
            );
            char_offset += 1;
            byte_offset += ch.len_utf8();
        }
    }

    fn push_text_char(&mut self, ch: char, face_id: u32, charpos: usize) {
        let char_state = self.text_char_state(ch);
        self.push_text_char_state_at_position(
            char_state,
            face_id,
            charpos,
            self.current_text_position(),
        );
    }

    fn push_text_char_with_measurement(
        &mut self,
        char_state: DisplayRowTextCharState,
        face_id: u32,
        charpos: usize,
        char_offset: usize,
        byte_offset: usize,
        measurement: &DisplayTextRunMeasurement,
    ) {
        self.push_text_char_state_with_advance_request(
            charpos,
            DisplayRowTextCharAdvanceRequest::new(
                char_state,
                face_id,
                self.current_text_position(),
                char_offset,
                byte_offset,
                measurement,
            ),
        );
    }

    fn push_text_char_state_at_position(
        &mut self,
        char_state: DisplayRowTextCharState,
        face_id: u32,
        charpos: usize,
        position: DisplayRowPosition,
    ) {
        let measurement = DisplayTextRunMeasurement::PerChar;
        self.push_text_char_state_with_advance_request(
            charpos,
            DisplayRowTextCharAdvanceRequest::per_char(char_state, face_id, position, &measurement),
        );
    }

    fn push_text_char_state_with_advance_request(
        &mut self,
        charpos: usize,
        advance_request: DisplayRowTextCharAdvanceRequest<'_>,
    ) {
        let ch = advance_request.ch();
        match advance_request.kind() {
            DisplayRowTextNaturalAdvanceKind::Tab => {
                self.push_tab_at_position(advance_request.face_id, advance_request.position);
            }
            DisplayRowTextNaturalAdvanceKind::ClusterContinuation => {
                glyph_row_writer::push_cluster_continuation_to_area(
                    &mut self.row,
                    self.area_index,
                    ch,
                    advance_request.face_id,
                    charpos,
                );
            }
            DisplayRowTextNaturalAdvanceKind::ComplexRunMember => {
                let advance = advance_request.resolve_advance_px_with_writer(self);
                glyph_row_writer::push_run_member_to_area(
                    &mut self.row,
                    self.area_index,
                    ch,
                    advance_request.face_id,
                    charpos,
                    advance,
                );
            }
            DisplayRowTextNaturalAdvanceKind::FaceColumns { columns } if columns > 1 => {
                let advance = advance_request.resolve_advance_px_with_writer(self);
                glyph_row_writer::push_wide_char_to_area(
                    &mut self.row,
                    self.area_index,
                    ch,
                    advance_request.face_id,
                    charpos,
                    advance,
                );
            }
            DisplayRowTextNaturalAdvanceKind::FaceColumns { .. } => {
                let advance = advance_request.resolve_advance_px_with_writer(self);
                glyph_row_writer::push_char_to_area(
                    &mut self.row,
                    self.area_index,
                    ch,
                    advance_request.face_id,
                    charpos,
                    advance,
                );
            }
        }
    }

    fn apply_item_layout_since(&mut self, before_len: usize, item_layout: DisplayItemLayout) {
        let vertical_offset_px = item_layout.vertical_offset_px(self.layout.height_px);
        if vertical_offset_px == 0.0 {
            return;
        }
        for glyph in &mut self.row.glyphs[self.area_index][before_len..] {
            glyph.vertical_offset_px = vertical_offset_px;
        }
    }

    fn text_char_state(&self, ch: char) -> DisplayRowTextCharState {
        DisplayRowTextCharState::for_glyphs(ch, &self.row.glyphs[self.area_index])
    }

    fn text_run_measurement(&mut self, text: &str, face_id: u32) -> DisplayTextRunMeasurement {
        self.glyph_measurer
            .as_mut()
            .map(|measurer| {
                measurer.text_run_advances_px(text, face_id, self.layout.char_width_px.max(1.0))
            })
            .unwrap_or(DisplayTextRunMeasurement::PerChar)
    }

    fn push_tab_at_position(&mut self, face_id: u32, position: DisplayRowPosition) {
        let advance = self
            .layout
            .tab_policy
            .advance_from(position, self.layout.char_width_px);
        let width_cols = advance.width_cols.min(usize::from(u16::MAX)) as u16;
        glyph_row_writer::push_stretch_to_area(
            &mut self.row,
            self.area_index,
            width_cols,
            face_id,
            advance.pixel_width,
            0.0,
            0.0,
        );
    }

    fn current_text_position(&self) -> DisplayRowPosition {
        let metrics = self.current_text_metrics();
        DisplayRowPosition {
            x_px: self.layout.tab_policy.origin_x_px + metrics.width_px,
            col: metrics.width_cols,
        }
    }

    fn current_text_metrics(&self) -> DisplayRowWriteMetrics {
        DisplayRowWriteMetrics::from_glyphs(
            &self.row.glyphs[self.area_index],
            self.layout.char_width_px,
        )
    }

    fn glyph_advance_px(&mut self, ch: char, face_id: u32, columns: usize) -> f32 {
        let fallback = self.layout.char_width_px.max(1.0) * columns.max(1) as f32;
        let measured_columns = columns.min(usize::from(u8::MAX)) as u8;
        self.glyph_measurer
            .as_mut()
            .and_then(|measurer| measurer.glyph_advance_px(ch, face_id, measured_columns, fallback))
            .filter(|advance| advance.is_finite() && *advance >= 0.0)
            .unwrap_or(fallback)
    }

    fn push_stretch(&mut self, stretch: DisplayStretch, face_id: u32) {
        let Some((width_cols, pixel_width)) = self.stretch_width(&stretch.width) else {
            return;
        };
        let pixel_height = stretch
            .height
            .as_ref()
            .and_then(|length| self.length_pixels(length, self.layout.height_px))
            .unwrap_or(0.0);
        let pixel_ascent = stretch
            .ascent
            .as_ref()
            .and_then(|length| self.length_pixels(length, self.layout.ascent_px))
            .unwrap_or(0.0);

        glyph_row_writer::push_stretch_to_area(
            &mut self.row,
            self.area_index,
            width_cols,
            face_id,
            pixel_width,
            pixel_height,
            pixel_ascent,
        );
        self.promote_row_metrics_for_explicit_stretch();
    }

    fn promote_row_metrics_for_explicit_stretch(&mut self) {
        let Some(glyph) = self.row.glyphs[self.area_index].last() else {
            return;
        };
        if glyph.pixel_height <= 0.0 {
            return;
        }
        self.row.height_px = self.row.height_px.max(glyph.pixel_height).max(1.0);
        self.row.ascent_px = self
            .row
            .ascent_px
            .max(glyph.pixel_ascent)
            .min(self.row.height_px);
    }

    fn push_control_char(&mut self, ch: char, face_id: u32, charpos: usize) {
        let Some(caret_char) = control_char_caret_char(ch) else {
            return;
        };
        self.push_text_char('^', face_id, charpos);
        self.push_text_char(caret_char, face_id, charpos);
    }

    fn push_glyphless(&mut self, glyphless: DisplayGlyphless, face_id: u32, charpos: usize) {
        let Some(pixel_width) = self.glyphless_pixel_width(&glyphless) else {
            return;
        };
        let glyph = Glyph {
            glyph_type: GlyphType::Glyphless { ch: glyphless.ch },
            face_id,
            charpos,
            bidi_level: 0,
            wide: false,
            pixel_width,
            pixel_height: 0.0,
            pixel_ascent: 0.0,
            vertical_offset_px: 0.0,
            padding: false,
        };
        self.row.glyphs[self.area_index].push(glyph);
        if self.area_index == GlyphArea::Text.index() {
            self.row.displays_text = true;
        }
    }

    fn glyphless_pixel_width(&self, glyphless: &DisplayGlyphless) -> Option<f32> {
        let char_width_px = self.layout.char_width_px.max(1.0);
        match glyphless.method {
            GlyphlessMethod::ZeroWidth => None,
            GlyphlessMethod::ThinSpace => Some(char_width_px * 0.25),
            GlyphlessMethod::EmptyBox => {
                let width_cols = base_width_cols(glyphless.ch).clamp(1, 4);
                Some(char_width_px * f32::from(width_cols))
            }
            GlyphlessMethod::HexCode => {
                let label_cols = if (glyphless.ch as u32) < 0x10000 {
                    6
                } else {
                    8
                };
                Some(char_width_px * label_cols as f32)
            }
        }
    }

    fn stretch_width(&self, width: &DisplayStretchWidth) -> Option<(u16, f32)> {
        match width {
            DisplayStretchWidth::Length(length) => {
                let pixels = self.length_pixels(length, self.layout.char_width_px)?;
                let cols = (pixels / self.layout.char_width_px.max(1.0))
                    .ceil()
                    .max(1.0) as u16;
                Some((cols, pixels))
            }
            DisplayStretchWidth::AlignTo(expr) => {
                let target = self.length_expr_pixels(expr)?;
                let current = self.current_text_metrics().width_px;
                let pixels = (target - current).max(0.0);
                let cols = (pixels / self.layout.char_width_px.max(1.0)).round() as u16;
                Some((cols, pixels))
            }
        }
    }

    fn length_pixels(&self, length: &DisplayLength, em_px: f32) -> Option<f32> {
        match length {
            DisplayLength::Columns(cols) => Some(f32::from(*cols) * self.layout.char_width_px),
            DisplayLength::Pixels(px) => Some(*px),
            DisplayLength::Em(em) => Some(*em * em_px.max(1.0)),
            DisplayLength::Expr(expr) => self.length_expr_pixels(expr),
        }
        .filter(|pixels| pixels.is_finite() && *pixels >= 0.0)
    }

    fn length_expr_pixels(&self, expr: &DisplayLengthExpr) -> Option<f32> {
        match expr {
            DisplayLengthExpr::Pixels(px) => Some(*px),
            DisplayLengthExpr::Em(em) => Some(*em * self.layout.char_width_px.max(1.0)),
            DisplayLengthExpr::Symbol(symbol) => match symbol {
                crate::display_item::DisplayLengthSymbol::Width => Some(self.layout.char_width_px),
                crate::display_item::DisplayLengthSymbol::Height => Some(self.layout.height_px),
                crate::display_item::DisplayLengthSymbol::Text
                | crate::display_item::DisplayLengthSymbol::Left => Some(0.0),
                crate::display_item::DisplayLengthSymbol::Right => Some(self.layout.width_px),
                crate::display_item::DisplayLengthSymbol::Center => {
                    Some(self.layout.width_px / 2.0)
                }
                crate::display_item::DisplayLengthSymbol::LeftFringe
                | crate::display_item::DisplayLengthSymbol::RightFringe
                | crate::display_item::DisplayLengthSymbol::LeftMargin
                | crate::display_item::DisplayLengthSymbol::RightMargin
                | crate::display_item::DisplayLengthSymbol::ScrollBar => Some(0.0),
            },
            DisplayLengthExpr::Variable(name) => self
                .layout
                .symbol_values
                .get(name.as_ref())
                .and_then(|expr| self.length_expr_pixels(expr)),
            DisplayLengthExpr::Add(parts) => parts.iter().try_fold(0.0, |sum, part| {
                self.length_expr_pixels(part).map(|value| sum + value)
            }),
            DisplayLengthExpr::Sub(parts) => {
                let mut iter = parts.iter();
                let first = self.length_expr_pixels(iter.next()?)?;
                iter.try_fold(first, |sum, part| {
                    self.length_expr_pixels(part).map(|value| sum - value)
                })
            }
        }
        .filter(|pixels| pixels.is_finite())
    }

    fn face_id(&self, face: RenderFaceRef) -> u32 {
        render_face_ref_id(face, render_face_ref_id(self.layout.base_face, 0))
    }
}

fn source_span_start_char(span: &SourceSpan) -> usize {
    match &span.start {
        DisplaySourcePosition::Buffer { char_pos, .. } => char_pos.get(),
        DisplaySourcePosition::LispString { char_index, .. } => *char_index,
        DisplaySourcePosition::Synthetic { offset, .. } => *offset,
    }
}

fn source_position_advance(
    start: &DisplaySourcePosition,
    char_offset: usize,
    byte_offset: usize,
) -> DisplaySourcePosition {
    match start {
        DisplaySourcePosition::Buffer {
            buffer_id,
            char_pos,
            byte_pos,
        } => DisplaySourcePosition::buffer(
            *buffer_id,
            CharPos0::new(char_pos.get() + char_offset),
            EmacsBytePos::new(byte_pos.get() + byte_offset),
        ),
        DisplaySourcePosition::LispString {
            source_id,
            char_index,
            byte_index,
        } => DisplaySourcePosition::lisp_string(
            source_id.get(),
            char_index + char_offset,
            byte_index + byte_offset,
        ),
        DisplaySourcePosition::Synthetic { source_id, offset } => {
            DisplaySourcePosition::synthetic(source_id.get(), offset + char_offset)
        }
    }
}

#[cfg(test)]
#[path = "display_row_builder_test.rs"]
mod tests;

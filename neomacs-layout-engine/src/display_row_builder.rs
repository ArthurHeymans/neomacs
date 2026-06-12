use crate::composition::{base_width_cols, continues_cluster, continues_complex_run};
use crate::display_item::{
    DisplayGlyphless, DisplayItem, DisplayItemKind, DisplayItemLayout, DisplayLength,
    DisplayLengthExpr, DisplaySourceMappedText, DisplaySourcePosition, DisplayStretch,
    DisplayStretchWidth, GlyphlessMethod, RenderFaceRef, SourceSpan, control_char_caret_char,
};
#[cfg(test)]
use crate::display_source::{DisplayItemSource, DisplaySourceContext};
use crate::matrix_builder::GlyphMatrixBuilder;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{Glyph, GlyphArea, GlyphRow, GlyphType};
use neovm_core::buffer::{CharPos0, EmacsBytePos};

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

pub(crate) enum DisplayRowItemMeasurement<'a> {
    Default,
    Measured(&'a mut dyn DisplayGlyphMeasurer),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplayTextRunAdvance {
    pub(crate) char_offset: usize,
    pub(crate) byte_offset: usize,
    pub(crate) advance_px: f32,
}

impl DisplayTextRunAdvance {
    pub(crate) fn new(char_offset: usize, byte_offset: usize, advance_px: f32) -> Self {
        Self {
            char_offset,
            byte_offset,
            advance_px,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DisplayTextRunMeasurement {
    PerChar,
    Measured(Vec<DisplayTextRunAdvance>),
}

impl DisplayTextRunMeasurement {
    fn advance_for(&self, char_offset: usize, byte_offset: usize) -> Option<f32> {
        match self {
            Self::PerChar => None,
            Self::Measured(advances) => advances
                .iter()
                .find(|advance| {
                    advance.char_offset == char_offset && advance.byte_offset == byte_offset
                })
                .and_then(|advance| {
                    (advance.advance_px.is_finite() && advance.advance_px >= 0.0)
                        .then_some(advance.advance_px)
                }),
        }
    }
}

pub(crate) trait DisplayRowItemMeasurer {
    fn measurement_for<'a>(
        &'a mut self,
        item: &DisplayItem,
        face_id: u32,
    ) -> DisplayRowItemMeasurement<'a>;
}

pub(crate) struct FixedGlyphAdvance {
    ch: char,
    face_id: u32,
    advance_px: f32,
}

impl FixedGlyphAdvance {
    pub(crate) fn new(ch: char, face_id: u32, advance_px: f32) -> Self {
        Self {
            ch,
            face_id,
            advance_px,
        }
    }
}

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

#[derive(Default)]
pub(crate) struct FixedGlyphAdvances {
    advances: std::collections::HashMap<(char, u32), f32>,
}

impl FixedGlyphAdvances {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn insert(&mut self, ch: char, face_id: u32, advance_px: f32) {
        self.advances.insert((ch, face_id), advance_px);
    }
}

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
            let width_cols = match glyph.glyph_type {
                GlyphType::Stretch { width_cols } => usize::from(width_cols.max(1)),
                GlyphType::Glyphless { .. } if glyph.pixel_width > 0.0 => {
                    (glyph.pixel_width / char_width_px.max(1.0)).ceil().max(1.0) as usize
                }
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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct DisplayRowPosition {
    pub(crate) x_px: f32,
    pub(crate) col: usize,
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

#[cfg(test)]
pub(crate) struct DisplayRowBuilder<'a> {
    layout: DisplayRowLayout,
    row: GlyphRow,
    glyph_measurer: Option<&'a mut dyn DisplayGlyphMeasurer>,
}

pub(crate) struct DisplayRowWriter<'layout, 'row, 'measurer> {
    layout: &'layout DisplayRowLayout,
    row: &'row mut GlyphRow,
    glyph_measurer: Option<&'measurer mut dyn DisplayGlyphMeasurer>,
}

pub(crate) struct DisplayRowProgressWriter<'layout, 'row, 'measurer> {
    writer: DisplayRowWriter<'layout, 'row, 'measurer>,
    position: DisplayRowPosition,
    max_x_px: f32,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowAppendCursor {
    position: DisplayRowPosition,
    max_x_px: f32,
}

#[cfg(test)]
impl DisplayRowAppendCursor {
    pub(crate) fn new(position: DisplayRowPosition, max_x_px: f32) -> Self {
        Self { position, max_x_px }
    }

    pub(crate) fn position(&self) -> DisplayRowPosition {
        self.position
    }

    #[cfg(test)]
    pub(crate) fn append_item_to_current_matrix_row(
        &mut self,
        builder: &mut GlyphMatrixBuilder,
        layout: &DisplayRowLayout,
        item: DisplayItem,
    ) -> Option<DisplayRowAppendProgress> {
        let progress = append_display_item_to_current_matrix_row(
            builder,
            layout,
            item,
            self.position,
            self.max_x_px,
        )?;
        self.position = progress.end;
        Some(progress)
    }

    pub(crate) fn append_measured_item_to_current_matrix_row(
        &mut self,
        builder: &mut GlyphMatrixBuilder,
        layout: &DisplayRowLayout,
        item: DisplayItem,
        glyph_measurer: &mut dyn DisplayGlyphMeasurer,
    ) -> Option<DisplayRowAppendProgress> {
        let progress = append_measured_display_item_to_current_matrix_row(
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
pub(crate) fn append_display_item_to_current_matrix_row(
    builder: &mut GlyphMatrixBuilder,
    layout: &DisplayRowLayout,
    item: DisplayItem,
    position: DisplayRowPosition,
    max_x_px: f32,
) -> Option<DisplayRowAppendProgress> {
    builder.with_current_row_mut(|row| {
        let mut writer = DisplayRowProgressWriter::new(layout, row, position, max_x_px);
        writer.push_item(item)
    })
}

#[cfg(test)]
pub(crate) fn append_measured_display_item_to_current_matrix_row(
    builder: &mut GlyphMatrixBuilder,
    layout: &DisplayRowLayout,
    item: DisplayItem,
    glyph_measurer: &mut dyn DisplayGlyphMeasurer,
    position: DisplayRowPosition,
    max_x_px: f32,
) -> Option<DisplayRowAppendProgress> {
    builder.with_current_row_mut(|row| {
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
    pub(crate) fn new(layout: DisplayRowLayout) -> Self {
        let mut row = GlyphRow::new(layout.role);
        row.enabled = true;
        row.pixel_y = layout.y_px;
        row.height_px = layout.height_px.max(1.0);
        row.ascent_px = layout.ascent_px.max(0.0).min(row.height_px);
        Self {
            layout,
            row,
            glyph_measurer: None,
        }
    }
}

#[cfg(test)]
impl<'a> DisplayRowBuilder<'a> {
    pub(crate) fn with_glyph_measurer(
        layout: DisplayRowLayout,
        glyph_measurer: &'a mut dyn DisplayGlyphMeasurer,
    ) -> Self {
        let mut builder = Self::new(layout);
        builder.glyph_measurer = Some(glyph_measurer);
        builder
    }

    pub(crate) fn push_item(&mut self, item: DisplayItem) -> DisplayRowWriteMetrics {
        if let Some(glyph_measurer) = self.glyph_measurer.as_deref_mut() {
            let mut writer =
                DisplayRowWriter::with_glyph_measurer(&self.layout, &mut self.row, glyph_measurer);
            writer.push_item(item)
        } else {
            let mut writer = DisplayRowWriter::new(&self.layout, &mut self.row);
            writer.push_item(item)
        }
    }

    pub(crate) fn push_measured_item(
        &mut self,
        item: DisplayItem,
        glyph_measurer: &mut dyn DisplayGlyphMeasurer,
    ) -> DisplayRowWriteMetrics {
        let mut writer =
            DisplayRowWriter::with_glyph_measurer(&self.layout, &mut self.row, glyph_measurer);
        writer.push_item(item)
    }

    pub(crate) fn push_source(
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

    pub(crate) fn finish(mut self) -> GlyphRow {
        GlyphMatrixBuilder::normalize_external_row(&mut self.row);
        self.row
    }
}

#[cfg(test)]
impl<'layout, 'row> DisplayRowWriter<'layout, 'row, '_> {
    pub(crate) fn new(layout: &'layout DisplayRowLayout, row: &'row mut GlyphRow) -> Self {
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
        }
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
        Self {
            writer: DisplayRowWriter::new(layout, row),
            position,
            max_x_px,
        }
    }
}

impl<'layout, 'row, 'measurer> DisplayRowProgressWriter<'layout, 'row, 'measurer> {
    pub(crate) fn with_glyph_measurer(
        layout: &'layout DisplayRowLayout,
        row: &'row mut GlyphRow,
        glyph_measurer: &'measurer mut dyn DisplayGlyphMeasurer,
        position: DisplayRowPosition,
        max_x_px: f32,
    ) -> Self {
        Self {
            writer: DisplayRowWriter::with_glyph_measurer(layout, row, glyph_measurer),
            position,
            max_x_px,
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
            DisplayItemKind::TextRun(run) => {
                let face_id = self.writer.face_id(face);
                let measurement = self.writer.text_run_measurement(run.text.as_ref(), face_id);
                let mut charpos = source_span_start_char(&span);
                let mut char_offset = 0usize;
                let mut byte_offset = 0usize;
                let mut status = DisplayRowAppendStatus::Complete;
                for ch in run.text.chars() {
                    let advance = self
                        .writer
                        .text_char_advance_px_at_position_with_measurement(
                            ch,
                            face_id,
                            self.position,
                            char_offset,
                            byte_offset,
                            &measurement,
                        );
                    if advance > 0.0 && self.position.x_px + advance > self.max_x_px {
                        status = DisplayRowAppendStatus::Clipped;
                        break;
                    }

                    let before_len = self.text_area_len();
                    let slot_start = self.position;
                    self.writer.push_text_char_at_position_with_measurement(
                        ch,
                        face_id,
                        charpos,
                        self.position,
                        char_offset,
                        byte_offset,
                        &measurement,
                    );
                    self.writer.apply_item_layout_since(before_len, item_layout);
                    let written = self.metrics_since(before_len);
                    slots.push(DisplayRowGlyphSlot {
                        source: source_position_advance(&span.start, char_offset, byte_offset),
                        x_px: slot_start.x_px,
                        col: slot_start.col,
                        width_px: written.width_px,
                        width_cols: written.width_cols,
                    });
                    self.advance(written);
                    metrics.add(written);
                    charpos += 1;
                    char_offset += 1;
                    byte_offset += ch.len_utf8();
                }
                status
            }
            DisplayItemKind::SourceMappedText(text) => {
                let face_id = self.writer.face_id(face);
                let measurement = self
                    .writer
                    .text_run_measurement(text.text.as_ref(), face_id);
                let charpos = source_span_start_char(&span);
                let mut char_offset = 0usize;
                let mut byte_offset = 0usize;
                let mut status = DisplayRowAppendStatus::Complete;
                for ch in text.text.chars() {
                    let advance = self
                        .writer
                        .text_char_advance_px_at_position_with_measurement(
                            ch,
                            face_id,
                            self.position,
                            char_offset,
                            byte_offset,
                            &measurement,
                        );
                    if advance > 0.0 && self.position.x_px + advance > self.max_x_px {
                        status = DisplayRowAppendStatus::Clipped;
                        break;
                    }

                    let before_len = self.text_area_len();
                    let slot_start = self.position;
                    self.writer.push_text_char_at_position_with_measurement(
                        ch,
                        face_id,
                        charpos,
                        self.position,
                        char_offset,
                        byte_offset,
                        &measurement,
                    );
                    self.writer.apply_item_layout_since(before_len, item_layout);
                    let written = self.metrics_since(before_len);
                    slots.push(DisplayRowGlyphSlot {
                        source: span.start.clone(),
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
            kind => {
                let slot_start = self.position;
                let slot_source = span.start.clone();
                let checkpoint = DisplayRowGlyphCheckpoint::capture(self.writer.row);
                let written = self
                    .writer
                    .push_item(DisplayItem::new(span, face, kind).with_layout(item_layout));
                if written.width_px > 0.0 && self.position.x_px + written.width_px > self.max_x_px {
                    checkpoint.restore(self.writer.row);
                    return DisplayRowAppendProgress {
                        start,
                        end: self.position,
                        metrics,
                        status: DisplayRowAppendStatus::Clipped,
                        slots,
                    };
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

        DisplayRowAppendProgress {
            start,
            end: self.position,
            metrics,
            status,
            slots,
        }
    }

    fn text_area_len(&self) -> usize {
        self.writer.row.glyphs[GlyphArea::Text.index()].len()
    }

    fn metrics_since(&self, before_len: usize) -> DisplayRowWriteMetrics {
        DisplayRowWriteMetrics::from_glyphs(
            &self.writer.row.glyphs[GlyphArea::Text.index()][before_len..],
            self.writer.layout.char_width_px,
        )
    }

    fn advance(&mut self, metrics: DisplayRowWriteMetrics) {
        self.position.x_px += metrics.width_px;
        self.position.col += metrics.width_cols;
    }
}

impl<'layout, 'row, 'measurer> DisplayRowWriter<'layout, 'row, 'measurer> {
    pub(crate) fn with_glyph_measurer(
        layout: &'layout DisplayRowLayout,
        row: &'row mut GlyphRow,
        glyph_measurer: &'measurer mut dyn DisplayGlyphMeasurer,
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
        }
    }

    pub(crate) fn push_item(&mut self, item: DisplayItem) -> DisplayRowWriteMetrics {
        let item_layout = item.layout;
        let face_id = self.face_id(item.face);
        let text_area = GlyphArea::Text.index();
        let before_len = self.row.glyphs[text_area].len();
        match item.kind {
            DisplayItemKind::TextRun(run) => {
                let measurement = self.text_run_measurement(run.text.as_ref(), face_id);
                let mut charpos = source_span_start_char(&item.span);
                let mut char_offset = 0usize;
                let mut byte_offset = 0usize;
                for ch in run.text.chars() {
                    self.push_text_char_with_measurement(
                        ch,
                        face_id,
                        charpos,
                        char_offset,
                        byte_offset,
                        &measurement,
                    );
                    charpos += 1;
                    char_offset += 1;
                    byte_offset += ch.len_utf8();
                }
            }
            DisplayItemKind::SourceMappedText(text) => {
                let measurement = self.text_run_measurement(text.text.as_ref(), face_id);
                self.push_source_mapped_text(
                    text,
                    face_id,
                    source_span_start_char(&item.span),
                    &measurement,
                );
            }
            DisplayItemKind::Stretch(stretch) => self.push_stretch(stretch, face_id),
            DisplayItemKind::Image(image) => {
                let image_id = image.image_id.max(0);
                let glyph = Glyph {
                    glyph_type: GlyphType::Image { image_id },
                    face_id,
                    charpos: source_span_start_char(&item.span),
                    bidi_level: 0,
                    wide: false,
                    pixel_width: 0.0,
                    pixel_height: 0.0,
                    pixel_ascent: 0.0,
                    vertical_offset_px: 0.0,
                    padding: false,
                };
                self.row.glyphs[GlyphArea::Text.index()].push(glyph);
                self.row.displays_text = true;
            }
            DisplayItemKind::ControlChar { ch } => {
                self.push_control_char(ch, face_id, source_span_start_char(&item.span));
            }
            DisplayItemKind::Glyphless(glyphless) => {
                self.push_glyphless(glyphless, face_id, source_span_start_char(&item.span));
            }
            DisplayItemKind::Video(_)
            | DisplayItemKind::Xwidget(_)
            | DisplayItemKind::RowBreak(_)
            | DisplayItemKind::CursorAnchor(_)
            | DisplayItemKind::HitTestAnchor(_) => {}
        }
        self.apply_item_layout_since(before_len, item_layout);
        DisplayRowWriteMetrics::from_glyphs(
            &self.row.glyphs[text_area][before_len..],
            self.layout.char_width_px,
        )
    }

    #[cfg(test)]
    pub(crate) fn push_source(
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

    fn push_text_char(&mut self, ch: char, face_id: u32, charpos: usize) {
        self.push_text_char_at_position(ch, face_id, charpos, self.current_text_position());
    }

    fn push_text_char_with_measurement(
        &mut self,
        ch: char,
        face_id: u32,
        charpos: usize,
        char_offset: usize,
        byte_offset: usize,
        measurement: &DisplayTextRunMeasurement,
    ) {
        self.push_text_char_at_position_with_measurement(
            ch,
            face_id,
            charpos,
            self.current_text_position(),
            char_offset,
            byte_offset,
            measurement,
        );
    }

    fn push_text_char_at_position(
        &mut self,
        ch: char,
        face_id: u32,
        charpos: usize,
        position: DisplayRowPosition,
    ) {
        self.push_text_char_at_position_with_measurement(
            ch,
            face_id,
            charpos,
            position,
            0,
            0,
            &DisplayTextRunMeasurement::PerChar,
        );
    }

    fn push_text_char_at_position_with_measurement(
        &mut self,
        ch: char,
        face_id: u32,
        charpos: usize,
        position: DisplayRowPosition,
        char_offset: usize,
        byte_offset: usize,
        measurement: &DisplayTextRunMeasurement,
    ) {
        if ch == '\t' {
            self.push_tab_at_position(face_id, position);
            return;
        }

        let tail = GlyphMatrixBuilder::last_text_cluster_tail_in_row(&self.row);
        if continues_cluster(ch, tail) {
            GlyphMatrixBuilder::push_cluster_continuation_to_row(
                &mut self.row,
                ch,
                face_id,
                charpos,
            );
            return;
        }
        if continues_complex_run(ch, tail) {
            let advance = self.text_char_advance_px_at_position_with_measurement(
                ch,
                face_id,
                position,
                char_offset,
                byte_offset,
                measurement,
            );
            GlyphMatrixBuilder::push_run_member_to_row(
                &mut self.row,
                ch,
                face_id,
                charpos,
                advance,
            );
            return;
        }
        let cols = base_width_cols(ch);
        let advance = self.text_char_advance_px_at_position_with_measurement(
            ch,
            face_id,
            position,
            char_offset,
            byte_offset,
            measurement,
        );
        if cols > 1 {
            GlyphMatrixBuilder::push_wide_char_to_row(&mut self.row, ch, face_id, charpos, advance);
        } else {
            GlyphMatrixBuilder::push_char_to_row(&mut self.row, ch, face_id, charpos, advance);
        }
    }

    fn apply_item_layout_since(&mut self, before_len: usize, item_layout: DisplayItemLayout) {
        let vertical_offset_px = item_layout.vertical_offset_px(self.layout.height_px);
        if vertical_offset_px == 0.0 {
            return;
        }
        let text_area = GlyphArea::Text.index();
        for glyph in &mut self.row.glyphs[text_area][before_len..] {
            glyph.vertical_offset_px = vertical_offset_px;
        }
    }

    fn text_char_advance_px_at_position_with_measurement(
        &mut self,
        ch: char,
        face_id: u32,
        position: DisplayRowPosition,
        char_offset: usize,
        byte_offset: usize,
        measurement: &DisplayTextRunMeasurement,
    ) -> f32 {
        if ch == '\t' {
            return self
                .layout
                .tab_policy
                .advance_from(position, self.layout.char_width_px)
                .pixel_width;
        }

        let tail = GlyphMatrixBuilder::last_text_cluster_tail_in_row(&self.row);
        if continues_cluster(ch, tail) {
            return 0.0;
        }

        if let Some(advance) = measurement.advance_for(char_offset, byte_offset) {
            return advance;
        }

        if continues_complex_run(ch, tail) {
            return self.glyph_advance_px(ch, face_id, 1);
        }
        self.glyph_advance_px(ch, face_id, base_width_cols(ch))
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
        GlyphMatrixBuilder::push_stretch_to_row(
            &mut self.row,
            width_cols,
            face_id,
            advance.pixel_width,
            0.0,
            0.0,
        );
    }

    fn current_text_position(&self) -> DisplayRowPosition {
        DisplayRowPosition {
            x_px: self.layout.tab_policy.origin_x_px + self.current_text_width_px(),
            col: usize::from(self.current_text_cols()),
        }
    }

    fn current_text_cols(&self) -> u16 {
        self.row.glyphs[GlyphArea::Text.index()]
            .iter()
            .filter(|glyph| !glyph.padding)
            .map(|glyph| match glyph.glyph_type {
                GlyphType::Stretch { width_cols } => width_cols.max(1),
                _ if glyph.wide => 2,
                _ => 1,
            })
            .sum()
    }

    fn glyph_advance_px(&mut self, ch: char, face_id: u32, columns: u8) -> f32 {
        let fallback = self.layout.char_width_px.max(1.0) * f32::from(columns.max(1));
        self.glyph_measurer
            .as_mut()
            .and_then(|measurer| measurer.glyph_advance_px(ch, face_id, columns, fallback))
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

        GlyphMatrixBuilder::push_stretch_to_row(
            &mut self.row,
            width_cols,
            face_id,
            pixel_width,
            pixel_height,
            pixel_ascent,
        );
        self.promote_row_metrics_for_explicit_stretch();
    }

    fn promote_row_metrics_for_explicit_stretch(&mut self) {
        let Some(glyph) = self.row.glyphs[GlyphArea::Text.index()].last() else {
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

    fn push_source_mapped_text(
        &mut self,
        text: DisplaySourceMappedText,
        face_id: u32,
        charpos: usize,
        measurement: &DisplayTextRunMeasurement,
    ) {
        let mut char_offset = 0usize;
        let mut byte_offset = 0usize;
        for ch in text.text.chars() {
            self.push_text_char_with_measurement(
                ch,
                face_id,
                charpos,
                char_offset,
                byte_offset,
                measurement,
            );
            char_offset += 1;
            byte_offset += ch.len_utf8();
        }
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
        self.row.glyphs[GlyphArea::Text.index()].push(glyph);
        self.row.displays_text = true;
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
                let current = self.current_text_width_px();
                let pixels = (target - current).max(0.0);
                let cols = (pixels / self.layout.char_width_px.max(1.0)).round() as u16;
                Some((cols, pixels))
            }
        }
    }

    fn current_text_width_px(&self) -> f32 {
        self.row.glyphs[GlyphArea::Text.index()]
            .iter()
            .map(|glyph| match glyph.glyph_type {
                GlyphType::Stretch { width_cols } => {
                    if glyph.pixel_width > 0.0 {
                        glyph.pixel_width
                    } else {
                        f32::from(width_cols) * self.layout.char_width_px.max(1.0)
                    }
                }
                _ if glyph.pixel_width > 0.0 => glyph.pixel_width,
                _ if glyph.wide => self.layout.char_width_px.max(1.0) * 2.0,
                _ if glyph.padding => 0.0,
                _ => self.layout.char_width_px.max(1.0),
            })
            .sum()
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
        match face {
            RenderFaceRef::FaceId(face_id) => face_id,
            RenderFaceRef::Inherit => match self.layout.base_face {
                RenderFaceRef::FaceId(face_id) => face_id,
                RenderFaceRef::Inherit => 0,
            },
        }
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

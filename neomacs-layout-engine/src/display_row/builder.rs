use crate::composition::base_width_cols;
use crate::display_face_ref::render_face_ref_id;
use crate::display_item::{
    DisplayGlyphless, DisplayItem, DisplayItemKind, DisplayItemLayout, DisplayLength,
    DisplayMediaReplacement, DisplayMediaReplacementKind, DisplaySourcePosition, DisplayStretch,
    DisplayStretchWidth, GlyphlessMethod, RenderFaceRef, SourceSpan, control_char_caret_char,
};
use crate::display_pixel_calc::{PixelCalcContext, calc_pixel_width_or_height};
use crate::display_row::append_context::{
    DisplayRowTextCharState, DisplayRowTextNaturalAdvanceKind, DisplayRowTextNaturalAdvancePolicy,
    DisplayRowTextNaturalAdvanceRequest,
};
#[cfg(test)]
use crate::display_source::{DisplayItemSource, DisplaySourceContext};
use crate::glyph_row_writer;
#[cfg(test)]
use crate::output::builder::DisplayOutputBuilder;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{Glyph, GlyphArea, GlyphRow, GlyphType};
use neomacs_display_protocol::types::FaceId;
use neovm_core::buffer::{CharPos0, EmacsBytePos};

use crate::display_text_run_measurement::DisplayTextRunMeasurement;

/// Which axis a `(space …)` length measures — GNU's `width_p` argument to
/// `calc_pixel_width_or_height`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PixelCalcAxis {
    Horizontal,
    Vertical,
}

impl PixelCalcAxis {
    const fn is_horizontal(self) -> bool {
        matches!(self, Self::Horizontal)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DisplayRowLayout {
    pub(crate) role: GlyphRowRole,
    pub(crate) y_px: f32,
    pub(crate) height_px: f32,
    pub(crate) ascent_px: f32,
    pub(crate) char_width_px: f32,
    pub(crate) tab_policy: DisplayTabPolicy,
    pub(crate) base_face: RenderFaceRef,
    /// Window/face/frame pixel state for the single GNU-faithful
    /// `(space :width/:align-to …)` evaluator. Mode-line, header-line and
    /// tab-line rows resolve region symbols (`text`, `right`, fringes, …)
    /// through this context, the same authority the buffer text path uses.
    pub(crate) pixel_calc: PixelCalcContext,
    /// Window inputs needed to resolve `(image …)` operands appearing inside a
    /// `(space :width/:align-to …)` expression on this row. `None` for rows
    /// built without window context (tests, terminal frames), which matches
    /// GNU's `FRAME_WINDOW_P` guard: the operand fails and `:align-to` is not
    /// applied.
    pub(crate) space_image_params: Option<crate::display_pixel_calc::PixelCalcImageInputs>,
}

impl DisplayRowLayout {
    /// The pixel-calc context for evaluating `spec`, with any `(image …)`
    /// operands it contains resolved to real pixel sizes.
    fn pixel_calc_for_space_spec(&self, spec: &neovm_core::emacs_core::Value) -> PixelCalcContext {
        let Some(inputs) = self.space_image_params.as_ref() else {
            return self.pixel_calc.clone();
        };
        let sizes =
            crate::display_pixel_calc::PixelCalcImageSizes::resolve_for_space_spec(spec, inputs);
        if sizes.is_empty() {
            return self.pixel_calc.clone();
        }
        let mut pixel_calc = self.pixel_calc.clone();
        pixel_calc.image_sizes = sizes;
        pixel_calc
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowVerticalMetrics {
    height_px: f32,
    ascent_px: f32,
}

impl DisplayRowVerticalMetrics {
    pub(crate) fn new(height_px: f32, ascent_px: f32) -> Self {
        Self {
            height_px,
            ascent_px,
        }
    }

    #[cfg(test)]
    pub(crate) fn from_row(row: &GlyphRow) -> Self {
        Self::new(row.height_px, row.ascent_px)
    }

    fn from_glyph(glyph: &Glyph) -> Option<Self> {
        (glyph.pixel_height > 0.0).then(|| Self::new(glyph.pixel_height, glyph.pixel_ascent))
    }

    fn with_vertical_offset(self, offset_px: f32) -> Self {
        let ascent = self.ascent_px.max(0.0).min(self.height_px.max(0.0));
        let descent = (self.height_px - ascent).max(0.0);
        let shifted_ascent = (ascent - offset_px).max(0.0);
        let shifted_descent = (descent + offset_px).max(0.0);
        Self::new(shifted_ascent + shifted_descent, shifted_ascent)
    }

    pub(crate) fn include_in_row(self, row: &mut GlyphRow) {
        if self.height_px <= 0.0 {
            return;
        }
        let row_descent = (row.height_px - row.ascent_px).max(0.0);
        let glyph_ascent = self.ascent_px.max(0.0);
        let glyph_descent = (self.height_px - glyph_ascent).max(0.0);
        row.ascent_px = row.ascent_px.max(glyph_ascent);
        row.height_px = (row.ascent_px + row_descent.max(glyph_descent)).max(1.0);
    }
}

fn display_row_glyph_count(row: &GlyphRow) -> usize {
    row.glyphs.iter().map(Vec::len).sum()
}

impl DisplayRowLayout {
    fn natural_text_advance_policy(&self) -> DisplayRowTextNaturalAdvancePolicy {
        DisplayRowTextNaturalAdvancePolicy::new(self.tab_policy.clone())
    }

    fn row_height_px(&self) -> f32 {
        self.height_px.max(1.0)
    }

    fn row_ascent_px(&self) -> f32 {
        self.ascent_px.max(0.0).min(self.row_height_px())
    }

    fn apply_to_row(&self, row: &mut GlyphRow) {
        row.enabled = true;
        row.role = self.role;
        row.mode_line = matches!(self.role, GlyphRowRole::ModeLine);
        row.pixel_y = self.y_px;
        let layout_metrics =
            DisplayRowVerticalMetrics::new(self.row_height_px(), self.row_ascent_px());
        if display_row_glyph_count(row) == 0 {
            row.height_px = self.row_height_px();
            row.ascent_px = self.row_ascent_px();
        } else {
            layout_metrics.include_in_row(row);
        }
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
    face_id: FaceId,
    position: DisplayRowPosition,
    char_offset: usize,
    byte_offset: usize,
    measurement: &'a DisplayTextRunMeasurement,
}

impl<'a> DisplayRowTextCharAdvanceRequest<'a> {
    fn new(
        char_state: DisplayRowTextCharState,
        face_id: FaceId,
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
        face_id: FaceId,
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
        glyph_advance_px: impl FnMut(char, FaceId, usize) -> f32,
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
        let char_width = TabGridPixel::from_renderer_px(char_width_px.max(1.0));
        let tab_width = char_width * i64::from(self.width_cols.max(1));
        let tab_x = TabGridPixel::from_renderer_px((position.x_px - self.origin_x_px).max(0.0));
        let next_tab_x = if !self.stop_cols.is_empty() {
            self.stop_cols
                .iter()
                .copied()
                .map(|stop| char_width * i64::try_from(stop).unwrap_or(i64::MAX))
                .find(|stop| *stop > tab_x)
                .unwrap_or_else(|| {
                    let last = char_width
                        * i64::try_from(self.stop_cols.last().copied().unwrap_or_default())
                            .unwrap_or(i64::MAX);
                    if tab_x >= last && tab_width > TabGridPixel::ZERO {
                        let repeated_stops = (tab_x - last).raw() / tab_width.raw() + 1;
                        last + tab_width * repeated_stops
                    } else {
                        last
                    }
                })
        } else if tab_width > TabGridPixel::ZERO {
            tab_width * (tab_x.raw() / tab_width.raw() + 1)
        } else {
            tab_x + char_width
        };
        // GNU's minimum-one-space guard (gui_produce_glyphs: a tab landing
        // exactly on a stop advances a full stop) works in integer pixels.
        // `TabGridPixel' makes that domain distinct from both renderer f32
        // coordinates and the protocol's subpixel `LayoutUnit'.
        let next_tab_x = if next_tab_x - tab_x < char_width {
            next_tab_x + tab_width
        } else {
            next_tab_x
        };
        let target_x_px = self.origin_x_px + next_tab_x.to_renderer_px();
        let pixel_width = (target_x_px - position.x_px).max(0.0);
        let next_col =
            usize::try_from((next_tab_x.raw() + char_width.raw() / 2) / char_width.raw())
                .unwrap_or(usize::MAX)
                .max(position.col + 1);
        DisplayTabAdvance {
            pixel_width,
            width_cols: next_col.saturating_sub(position.col).max(1),
        }
    }
}

/// An integer pixel in GNU Emacs's tab-stop coordinate system.
///
/// Glyph shaping and GPU placement remain subpixel, but GNU xdisp computes
/// TAB stops with integer `current_x' and integer `font->space_width'.  Keeping
/// this as a separate type prevents a deterministic fixed-point coordinate
/// from being mistaken for GNU's integer-pixel domain.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
struct TabGridPixel(i64);

impl TabGridPixel {
    const ZERO: Self = Self(0);

    fn from_renderer_px(px: f32) -> Self {
        Self(px.round() as i64)
    }

    const fn raw(self) -> i64 {
        self.0
    }

    fn to_renderer_px(self) -> f32 {
        self.0 as f32
    }
}

impl std::ops::Add for TabGridPixel {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }
}

impl std::ops::Sub for TabGridPixel {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }
}

impl std::ops::Mul<i64> for TabGridPixel {
    type Output = Self;

    fn mul(self, factor: i64) -> Self {
        Self(self.0.saturating_mul(factor))
    }
}

pub(crate) trait DisplayGlyphMeasurer {
    fn glyph_advance_px(
        &mut self,
        ch: char,
        face_id: FaceId,
        columns: u8,
        fallback_advance_px: f32,
    ) -> Option<f32>;

    fn glyph_vertical_metrics_px(
        &mut self,
        _ch: char,
        _face_id: FaceId,
    ) -> Option<DisplayRowVerticalMetrics> {
        None
    }

    /// Global metrics of the face's primary font, independent of which font
    /// would cover a concrete character.  GNU uses these for stretch glyphs
    /// produced by `(space-width ...)`.
    fn face_vertical_metrics_px(&mut self, _face_id: FaceId) -> Option<DisplayRowVerticalMetrics> {
        None
    }

    /// The face primary font's space advance.  This deliberately differs
    /// from `glyph_advance_px(' ', ...)`, which may select an ASCII fallback
    /// when the requested face is a symbol-only font.
    fn face_space_width_px(&mut self, _face_id: FaceId) -> Option<f32> {
        None
    }

    fn text_run_advances_px(
        &mut self,
        _text: &str,
        _face_id: FaceId,
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
    face_id: FaceId,
    advance_px: f32,
}

#[cfg(test)]
impl FixedGlyphAdvance {
    pub(crate) fn new(ch: char, face_id: FaceId, advance_px: f32) -> Self {
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
        face_id: FaceId,
        _columns: u8,
        _fallback_advance_px: f32,
    ) -> Option<f32> {
        (self.ch == ch && self.face_id == face_id).then_some(self.advance_px)
    }
}

#[cfg(test)]
#[derive(Default)]
pub(crate) struct FixedGlyphAdvances {
    advances: std::collections::HashMap<(char, FaceId), f32>,
}

#[cfg(test)]
impl FixedGlyphAdvances {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn insert(&mut self, ch: char, face_id: FaceId, advance_px: f32) {
        self.advances.insert((ch, face_id), advance_px);
    }
}

#[cfg(test)]
impl DisplayGlyphMeasurer for FixedGlyphAdvances {
    fn glyph_advance_px(
        &mut self,
        ch: char,
        face_id: FaceId,
        _columns: u8,
        _fallback_advance_px: f32,
    ) -> Option<f32> {
        self.advances.get(&(ch, face_id)).copied()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct DisplayRowWriteMetrics {
    width_px: f32,
    width_cols: usize,
}

impl DisplayRowWriteMetrics {
    pub(crate) const fn new(width_px: f32, width_cols: usize) -> Self {
        Self {
            width_px,
            width_cols,
        }
    }

    pub(crate) fn width_px(self) -> f32 {
        self.width_px
    }

    pub(crate) fn width_cols(self) -> usize {
        self.width_cols
    }

    pub(crate) fn has_positive_width(self) -> bool {
        self.width_px > 0.0
    }

    pub(crate) fn is_empty(self) -> bool {
        self.width_px <= 0.0 && self.width_cols == 0
    }

    fn from_glyphs(glyphs: &[Glyph], char_width_px: f32) -> Self {
        glyphs.iter().fold(Self::default(), |mut metrics, glyph| {
            // A complex-run member's padding cell carries its own grapheme (a
            // non-blank Char or Composite) plus a POSITIVE `pixel_width` so the
            // GUI advances x and the TTY can decompose the run one cell each.
            // But the run's base `Composite` already accounts for the whole
            // run's width via `composed_cluster_cols` (= GNU's `cmp->width`,
            // set once in `produce_composite_glyph`, src/term.c:1859; the
            // caller advances `it->current_x` by it a single time, :1762).
            // Counting these padding cells again would double-count the run
            // (the etc/HELLO Arabic/Indic left-shift): so they contribute 0
            // cols and 0 px, just like the old zero-width padding shape did.
            if glyph_row_writer::is_run_member_padding(glyph) {
                return metrics;
            }
            let width_cols = match &glyph.glyph_type {
                GlyphType::Stretch { width_cols } => usize::from((*width_cols).max(1)),
                GlyphType::Image { width_cols, .. }
                | GlyphType::Video { width_cols, .. }
                | GlyphType::Xwidget { width_cols, .. } => usize::from((*width_cols).max(1)),
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct DisplayRowGlyphCheckpoint {
    area_lengths: [usize; 3],
    displays_text: bool,
    pointer_appearances_len: usize,
}

impl DisplayRowGlyphCheckpoint {
    pub(crate) fn capture(row: &GlyphRow) -> Self {
        Self {
            area_lengths: [
                row.glyphs[GlyphArea::LeftMargin.index()].len(),
                row.glyphs[GlyphArea::Text.index()].len(),
                row.glyphs[GlyphArea::RightMargin.index()].len(),
            ],
            displays_text: row.displays_text,
            pointer_appearances_len: row.pointer_appearances().len(),
        }
    }

    pub(crate) fn restore(self, row: &mut GlyphRow) {
        row.glyphs[GlyphArea::LeftMargin.index()].truncate(self.area_lengths[0]);
        row.glyphs[GlyphArea::Text.index()].truncate(self.area_lengths[1]);
        row.glyphs[GlyphArea::RightMargin.index()].truncate(self.area_lengths[2]);
        row.displays_text = self.displays_text;
        row.truncate_pointer_appearances(self.pointer_appearances_len);
    }

    /// Derive a checkpoint `added` text glyphs further along than `self`. Used by
    /// the whole-text-run word-wrap path, which records candidates *after* the
    /// run is appended: the base checkpoint snapshots the row before the run, and
    /// each candidate's boundary is `base + char_offset` text glyphs (natural
    /// text runs map one source char to one text glyph). Any added text glyph
    /// means the row now displays text.
    pub(crate) fn with_added_text_glyphs(
        self,
        added: usize,
        after_append: DisplayRowGlyphCheckpoint,
    ) -> Self {
        let mut area_lengths = self.area_lengths;
        area_lengths[GlyphArea::Text.index()] += added;
        Self {
            area_lengths,
            displays_text: self.displays_text || added > 0,
            pointer_appearances_len: if added > 0 {
                after_append.pointer_appearances_len
            } else {
                self.pointer_appearances_len
            },
        }
    }
}

pub(crate) fn new_display_row(layout: &DisplayRowLayout) -> GlyphRow {
    let mut row = new_display_row_for_role(layout.role);
    layout.apply_to_row(&mut row);
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
    // Row bounds are real from the row's BEGIN (stamped with the walk-start
    // position), so glyph spans always MERGE — never replace. No numeric
    // "unset" state exists to test for.
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
    x_px: f32,
    col: usize,
}

impl DisplayRowPosition {
    pub(crate) const fn new(x_px: f32, col: usize) -> Self {
        Self { x_px, col }
    }

    pub(crate) fn x_px(self) -> f32 {
        self.x_px
    }

    pub(crate) fn col(self) -> usize {
        self.col
    }

    pub(crate) fn advance_by(self, metrics: DisplayRowWriteMetrics) -> Self {
        Self {
            x_px: self.x_px + metrics.width_px,
            col: self.col + metrics.width_cols,
        }
    }

    pub(crate) fn saturating_width_to(self, end: Self) -> DisplayRowWriteMetrics {
        DisplayRowWriteMetrics::new(
            (end.x_px - self.x_px).max(0.0),
            end.col.saturating_sub(self.col),
        )
    }
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
    source: DisplaySourcePosition,
    x_px: f32,
    col: usize,
    width_px: f32,
    width_cols: usize,
}

impl DisplayRowGlyphSlot {
    #[cfg(test)]
    pub(crate) fn new(
        source: DisplaySourcePosition,
        x_px: f32,
        col: usize,
        width_px: f32,
        width_cols: usize,
    ) -> Self {
        Self::with_pointer_appearance(source, x_px, col, width_px, width_cols, None)
    }

    pub(crate) fn with_pointer_appearance(
        source: DisplaySourcePosition,
        x_px: f32,
        col: usize,
        width_px: f32,
        width_cols: usize,
        _pointer_appearance: Option<crate::display_item::DisplayPointerAppearance>,
    ) -> Self {
        Self {
            source,
            x_px,
            col,
            width_px,
            width_cols,
        }
    }

    pub(crate) fn source(&self) -> DisplaySourcePosition {
        self.source.clone()
    }

    pub(crate) fn x_px(&self) -> f32 {
        self.x_px
    }

    pub(crate) fn col(&self) -> usize {
        self.col
    }

    pub(crate) fn width_px(&self) -> f32 {
        self.width_px
    }

    pub(crate) fn width_cols(&self) -> usize {
        self.width_cols
    }

    pub(crate) fn start_position(&self) -> DisplayRowPosition {
        DisplayRowPosition::new(self.x_px(), self.col())
    }

    pub(crate) fn end_position(&self) -> DisplayRowPosition {
        DisplayRowPosition::new(
            self.x_px() + self.width_px(),
            self.col() + self.width_cols(),
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplayRowAppendProgress {
    start: DisplayRowPosition,
    end: DisplayRowPosition,
    metrics: DisplayRowWriteMetrics,
    status: DisplayRowAppendStatus,
    slots: Vec<DisplayRowGlyphSlot>,
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
        Self::new(start, end, start.saturating_width_to(end), status, slots)
    }

    pub(crate) fn start(&self) -> DisplayRowPosition {
        self.start
    }

    pub(crate) fn end(&self) -> DisplayRowPosition {
        self.end
    }

    pub(crate) fn metrics(&self) -> DisplayRowWriteMetrics {
        self.metrics
    }

    pub(crate) fn status(&self) -> DisplayRowAppendStatus {
        self.status
    }

    pub(crate) fn slots(&self) -> &[DisplayRowGlyphSlot] {
        &self.slots
    }

    pub(crate) fn is_complete_with_positive_width(&self) -> bool {
        self.status == DisplayRowAppendStatus::Complete && self.metrics.has_positive_width()
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
        self.position = progress.end();
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
        self.position = progress.end();
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
        let mut writer = DisplayRowWriter::new(layout, row);
        writer.set_empty_row_start(position);
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
        let mut writer =
            DisplayRowWriter::with_glyph_measurer_for_area(layout, row, glyph_measurer, area);
        writer.set_empty_row_start(position);
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

    #[cfg(test)]
    pub(crate) fn with_text_run_measurement_for_area(
        layout: &'layout DisplayRowLayout,
        row: &'row mut GlyphRow,
        text_run_measurement: DisplayTextRunMeasurement,
        position: DisplayRowPosition,
        max_x_px: f32,
        area: GlyphArea,
    ) -> Self {
        let mut writer = DisplayRowWriter::for_area(layout, row, area);
        writer.set_empty_row_start(position);
        let position = append_start_position(position, writer.current_text_position());
        Self {
            writer,
            position,
            max_x_px,
            text_run_measurement: Some(text_run_measurement),
        }
    }

    pub(crate) fn with_text_run_measurement_and_glyph_measurer_for_area(
        layout: &'layout DisplayRowLayout,
        row: &'row mut GlyphRow,
        text_run_measurement: DisplayTextRunMeasurement,
        glyph_measurer: &'measurer mut dyn DisplayGlyphMeasurer,
        position: DisplayRowPosition,
        max_x_px: f32,
        area: GlyphArea,
    ) -> Self {
        let mut writer =
            DisplayRowWriter::with_glyph_measurer_for_area(layout, row, glyph_measurer, area);
        writer.set_empty_row_start(position);
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
            pointer_appearance,
        } = item;
        let status = match kind {
            DisplayItemKind::RowBreak(_) => DisplayRowAppendStatus::RowBreak,
            DisplayItemKind::TextRun(run) => self.push_text_item(
                &span,
                face,
                item_layout,
                run.text.as_ref(),
                DisplayTextSourceMapping::NaturalText,
                pointer_appearance.as_ref(),
                &mut metrics,
                &mut slots,
            ),
            DisplayItemKind::SourceMappedText(text) => self.push_text_item(
                &span,
                face,
                item_layout,
                text.text.as_ref(),
                DisplayTextSourceMapping::SourceMapped,
                pointer_appearance.as_ref(),
                &mut metrics,
                &mut slots,
            ),
            kind => {
                let slot_start = self.position;
                let slot_source = span.start.clone();
                let checkpoint = DisplayRowGlyphCheckpoint::capture(self.writer.row);
                let written = self.writer.push_item(
                    DisplayItem::new(span, face, kind)
                        .with_layout(item_layout)
                        .with_pointer_appearance(pointer_appearance.clone()),
                );
                if written.has_positive_width()
                    && self.position.x_px() + written.width_px() > self.max_x_px
                {
                    checkpoint.restore(self.writer.row);
                    return DisplayRowAppendProgress::new(
                        start,
                        self.position,
                        metrics,
                        DisplayRowAppendStatus::Clipped,
                        slots,
                    );
                }
                if !written.is_empty() {
                    slots.push(DisplayRowGlyphSlot::with_pointer_appearance(
                        slot_source,
                        slot_start.x_px(),
                        slot_start.col(),
                        written.width_px(),
                        written.width_cols(),
                        pointer_appearance.clone(),
                    ));
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
        pointer_appearance: Option<&crate::display_item::DisplayPointerAppearance>,
        metrics: &mut DisplayRowWriteMetrics,
        slots: &mut Vec<DisplayRowGlyphSlot>,
    ) -> DisplayRowAppendStatus {
        let face_id = self.writer.face_id(face);
        let measurement = self.text_run_measurement(text, face_id);
        let start_char = source_span_start_char(span);
        let glyph_pointer_appearance =
            pointer_appearance.and_then(|appearance| appearance.glyph_metadata());
        let mut pointer_metadata = None;
        let mut status = DisplayRowAppendStatus::Complete;
        let mut byte_offset = 0usize;
        for (char_offset, ch) in text.chars().enumerate() {
            let char_state = self.writer.text_char_state(ch);
            let advance_request = DisplayRowTextCharAdvanceRequest::new(
                char_state,
                face_id,
                self.position,
                char_offset,
                byte_offset,
                &measurement,
            );
            let natural_advance = advance_request.resolve_advance_px_with_writer(&mut self.writer);
            let advance =
                self.writer
                    .item_horizontal_advance_px(ch, face_id, natural_advance, item_layout);
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
            self.writer
                .apply_item_layout_since(before_len, face_id, item_layout);
            if self.area_len() > before_len
                && pointer_metadata.is_none()
                && let Some(appearance) = glyph_pointer_appearance
            {
                pointer_metadata = self.writer.row.intern_pointer_appearance(appearance);
            }
            for glyph in &mut self.writer.row.glyphs[self.writer.area_index][before_len..] {
                glyph.pointer_appearance = pointer_metadata;
            }
            let written = self.metrics_since(before_len);
            slots.push(DisplayRowGlyphSlot::with_pointer_appearance(
                source_mapping.slot_source(&span.start, char_offset, byte_offset),
                slot_start.x_px(),
                slot_start.col(),
                written.width_px(),
                written.width_cols(),
                pointer_appearance.cloned(),
            ));
            self.advance(written);
            metrics.add(written);
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
        self.position = self.position.advance_by(metrics);
    }

    fn text_run_measurement(&mut self, text: &str, face_id: FaceId) -> DisplayTextRunMeasurement {
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

    #[cfg(test)]
    fn for_area(
        layout: &'layout DisplayRowLayout,
        row: &'row mut GlyphRow,
        area: GlyphArea,
    ) -> Self {
        layout.apply_to_row(row);
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
        layout.apply_to_row(row);
        Self {
            layout,
            row,
            glyph_measurer: Some(glyph_measurer),
            area_index: area.index(),
        }
    }

    fn push_item(&mut self, item: DisplayItem) -> DisplayRowWriteMetrics {
        let item_layout = item.layout;
        let pointer_appearance = item
            .pointer_appearance
            .as_ref()
            .and_then(|appearance| appearance.glyph_metadata());
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
                self.push_media(media, face_id, source_span_start_char(&item.span))
            }
            DisplayItemKind::ControlChar { ch } => {
                self.push_control_char(ch, face_id, source_span_start_char(&item.span));
            }
            DisplayItemKind::Glyphless(glyphless) => {
                self.push_glyphless(glyphless, face_id, source_span_start_char(&item.span));
            }
            DisplayItemKind::RowBreak(_) => {}
        }
        self.apply_item_layout_since(before_len, face_id, item_layout);
        let pointer_appearance = if self.row.glyphs[area_index].len() > before_len {
            pointer_appearance.and_then(|appearance| self.row.intern_pointer_appearance(appearance))
        } else {
            None
        };
        for glyph in &mut self.row.glyphs[area_index][before_len..] {
            glyph.pointer_appearance = pointer_appearance;
        }
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
        face_id: FaceId,
        span: &SourceSpan,
        source_mapping: DisplayTextSourceMapping,
    ) {
        let measurement = self.text_run_measurement(text, face_id);
        let start_char = source_span_start_char(span);
        let mut byte_offset = 0usize;
        for (char_offset, ch) in text.chars().enumerate() {
            let char_state = self.text_char_state(ch);
            self.push_text_char_with_measurement(
                char_state,
                face_id,
                source_mapping.charpos(start_char, char_offset),
                char_offset,
                byte_offset,
                &measurement,
            );
            byte_offset += ch.len_utf8();
        }
    }

    fn push_text_char(&mut self, ch: char, face_id: FaceId, charpos: usize) {
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
        face_id: FaceId,
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
        face_id: FaceId,
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
        let face_id = advance_request.face_id;
        let before_len = self.row.glyphs[self.area_index].len();
        match advance_request.kind() {
            DisplayRowTextNaturalAdvanceKind::Tab => {
                self.push_tab_at_position(advance_request.face_id, advance_request.position);
            }
            DisplayRowTextNaturalAdvanceKind::ClusterContinuation => {
                glyph_row_writer::push_cluster_continuation_to_area(
                    self.row,
                    self.area_index,
                    ch,
                    advance_request.face_id,
                    charpos,
                );
            }
            DisplayRowTextNaturalAdvanceKind::ComplexRunMember => {
                let advance = advance_request.resolve_advance_px_with_writer(self);
                glyph_row_writer::push_run_member_to_area(
                    self.row,
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
                    self.row,
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
                    self.row,
                    self.area_index,
                    ch,
                    advance_request.face_id,
                    charpos,
                    advance,
                );
            }
        }
        let Some(metrics) = self
            .glyph_measurer
            .as_deref_mut()
            .and_then(|measurer| measurer.glyph_vertical_metrics_px(ch, face_id))
        else {
            return;
        };
        for glyph in &mut self.row.glyphs[self.area_index][before_len..] {
            if !glyph.padding {
                glyph.pixel_height = metrics.height_px;
                glyph.pixel_ascent = metrics.ascent_px;
            }
        }
    }

    fn apply_item_layout_since(
        &mut self,
        before_len: usize,
        face_id: FaceId,
        item_layout: DisplayItemLayout,
    ) {
        let face_space_width = item_layout.space_width.and_then(|_| {
            self.glyph_measurer
                .as_deref_mut()
                .and_then(|measurer| measurer.face_space_width_px(face_id))
                .filter(|width| width.is_finite() && *width > 0.0)
        });
        let face_vertical_metrics = item_layout.space_width.and_then(|_| {
            self.glyph_measurer
                .as_deref_mut()
                .and_then(|measurer| measurer.face_vertical_metrics_px(face_id))
        });
        for glyph in &mut self.row.glyphs[self.area_index][before_len..] {
            if matches!(glyph.glyph_type, GlyphType::Char { ch: ' ' }) {
                // GNU xdisp turns a space carrying `(space-width FACTOR)`
                // into a stretch glyph.  Its width and vertical box come
                // from the face's PRIMARY font, not from the fallback font
                // that happens to cover U+0020.
                if item_layout.space_width.is_some() {
                    glyph.glyph_type = GlyphType::Stretch { width_cols: 1 };
                    let natural_width = face_space_width.unwrap_or(glyph.pixel_width);
                    glyph.pixel_width = item_layout.horizontal_advance_px(' ', natural_width);
                    if let Some(metrics) = face_vertical_metrics {
                        glyph.pixel_height = metrics.height_px;
                        glyph.pixel_ascent = metrics.ascent_px;
                    }
                }
            }
            let reference_height = if glyph.pixel_height > 0.0 {
                glyph.pixel_height
            } else {
                self.layout.height_px
            };
            glyph.vertical_offset_px = item_layout.vertical_offset_px(reference_height);
        }
        let vertical_metrics = self.row.glyphs[self.area_index][before_len..]
            .iter()
            .filter_map(|glyph| {
                DisplayRowVerticalMetrics::from_glyph(glyph)
                    .map(|metrics| metrics.with_vertical_offset(glyph.vertical_offset_px))
            })
            .collect::<Vec<_>>();
        for metrics in vertical_metrics {
            metrics.include_in_row(self.row);
        }
    }

    fn item_horizontal_advance_px(
        &mut self,
        ch: char,
        face_id: FaceId,
        natural_advance_px: f32,
        item_layout: DisplayItemLayout,
    ) -> f32 {
        let natural_advance_px = if ch == ' ' && item_layout.space_width.is_some() {
            self.glyph_measurer
                .as_deref_mut()
                .and_then(|measurer| measurer.face_space_width_px(face_id))
                .filter(|width| width.is_finite() && *width > 0.0)
                .unwrap_or(natural_advance_px)
        } else {
            natural_advance_px
        };
        item_layout.horizontal_advance_px(ch, natural_advance_px)
    }

    fn text_char_state(&self, ch: char) -> DisplayRowTextCharState {
        DisplayRowTextCharState::for_glyphs(ch, &self.row.glyphs[self.area_index])
    }

    fn text_run_measurement(&mut self, text: &str, face_id: FaceId) -> DisplayTextRunMeasurement {
        self.glyph_measurer
            .as_mut()
            .map(|measurer| {
                measurer.text_run_advances_px(text, face_id, self.layout.char_width_px.max(1.0))
            })
            .unwrap_or(DisplayTextRunMeasurement::PerChar)
    }

    fn push_tab_at_position(&mut self, face_id: FaceId, position: DisplayRowPosition) {
        // GNU's gui_produce_glyphs uses the TAB face's primary font
        // `space_width`, not character fallback selection for U+0020.
        let space_width_px = self
            .glyph_measurer
            .as_deref_mut()
            .and_then(|measurer| measurer.face_space_width_px(face_id))
            .filter(|width| width.is_finite() && *width > 0.0)
            .unwrap_or_else(|| self.glyph_advance_px(' ', face_id, 1));
        let advance = self
            .layout
            .tab_policy
            .advance_from(position, space_width_px);
        let width_cols = advance.width_cols.min(usize::from(u16::MAX)) as u16;
        glyph_row_writer::push_stretch_to_area(
            self.row,
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
        DisplayRowPosition::new(
            self.layout.tab_policy.origin_x_px + self.row.pixel_x + metrics.width_px(),
            usize::from(self.row.start_col) + metrics.width_cols(),
        )
    }

    /// Capture the initial pen and display-slot position on the row itself.
    ///
    /// The old media side channel remembered this position separately while
    /// ordinary glyph materialization always began at column zero.  Stamp an
    /// empty text row once so every primitive follows the same geometry.  A
    /// non-empty row already owns its origin and must never be repositioned by
    /// a later append fragment.
    fn set_empty_row_start(&mut self, requested: DisplayRowPosition) {
        if self.area_index != GlyphArea::Text.index() || display_row_glyph_count(self.row) != 0 {
            return;
        }

        let origin_x = self.layout.tab_policy.origin_x_px;
        if requested.x_px() >= origin_x {
            self.row.pixel_x = requested.x_px() - origin_x;
        }
        self.row.start_col = requested.col().min(usize::from(u16::MAX)) as u16;
    }

    fn current_text_metrics(&self) -> DisplayRowWriteMetrics {
        DisplayRowWriteMetrics::from_glyphs(
            &self.row.glyphs[self.area_index],
            self.layout.char_width_px,
        )
    }

    fn glyph_advance_px(&mut self, ch: char, face_id: FaceId, columns: usize) -> f32 {
        let fallback = self.layout.char_width_px.max(1.0) * columns.max(1) as f32;
        let measured_columns = columns.min(usize::from(u8::MAX)) as u8;
        self.glyph_measurer
            .as_mut()
            .and_then(|measurer| measurer.glyph_advance_px(ch, face_id, measured_columns, fallback))
            .filter(|advance| advance.is_finite() && *advance >= 0.0)
            .unwrap_or(fallback)
    }

    fn push_stretch(&mut self, stretch: DisplayStretch, face_id: FaceId) {
        let Some((width_cols, pixel_width)) = self.stretch_width(&stretch.width) else {
            return;
        };
        let pixel_height = stretch
            .height
            .as_ref()
            .and_then(|length| {
                self.length_pixels(length, self.layout.height_px, PixelCalcAxis::Vertical)
            })
            .unwrap_or(0.0);
        let pixel_ascent = stretch
            .ascent
            .as_ref()
            .and_then(|length| {
                self.length_pixels(length, self.layout.ascent_px, PixelCalcAxis::Vertical)
            })
            .unwrap_or(0.0);

        glyph_row_writer::push_stretch_to_area(
            self.row,
            self.area_index,
            width_cols,
            face_id,
            pixel_width,
            pixel_height,
            pixel_ascent,
        );
        self.promote_row_metrics_for_explicit_stretch();
    }

    fn push_media(&mut self, media: DisplayMediaReplacement, face_id: FaceId, charpos: usize) {
        let Some((width_cols, pixel_width)) =
            self.stretch_width(&media.replacement_stretch().width)
        else {
            return;
        };
        let glyph_type = match media.kind {
            DisplayMediaReplacementKind::Image {
                image_id,
                horizontal_margin,
                vertical_margin,
                opaque_background,
            } => GlyphType::Image {
                image_id: image_id as i32,
                width_cols,
                horizontal_margin,
                vertical_margin,
                opaque_background,
            },
            DisplayMediaReplacementKind::Video {
                video_id,
                loop_count,
                autoplay,
            } => GlyphType::Video {
                video_id: video_id as i32,
                width_cols,
                loop_count,
                autoplay,
            },
            DisplayMediaReplacementKind::Xwidget { xwidget_id } => GlyphType::Xwidget {
                xwidget_id: xwidget_id as i32,
                width_cols,
            },
            DisplayMediaReplacementKind::Surface { surface_id } => GlyphType::Surface {
                surface_id: surface_id as i32,
                width_cols,
            },
        };
        let mut glyph = Glyph::stretch(width_cols, face_id).with_pixel_geometry(
            pixel_width,
            media.height,
            media.ascent,
        );
        glyph.glyph_type = glyph_type;
        glyph.charpos = charpos;
        self.row.glyphs[self.area_index].push(glyph);
        if self.area_index == GlyphArea::Text.index() {
            self.row.displays_text = true;
        }
        self.promote_row_metrics_for_explicit_stretch();
    }

    fn promote_row_metrics_for_explicit_stretch(&mut self) {
        let Some(glyph) = self.row.glyphs[self.area_index].last() else {
            return;
        };
        if let Some(metrics) = DisplayRowVerticalMetrics::from_glyph(glyph) {
            if display_row_glyph_count(self.row) == 1 {
                self.row.height_px = metrics.height_px.max(1.0);
                self.row.ascent_px = metrics.ascent_px.max(0.0);
            } else {
                metrics.include_in_row(self.row);
            }
        }
    }

    fn push_control_char(&mut self, ch: char, face_id: FaceId, charpos: usize) {
        let Some(caret_char) = control_char_caret_char(ch) else {
            return;
        };
        self.push_text_char('^', face_id, charpos);
        self.push_text_char(caret_char, face_id, charpos);
    }

    fn push_glyphless(&mut self, glyphless: DisplayGlyphless, face_id: FaceId, charpos: usize) {
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
            pointer_appearance: None,
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
                let pixels = self.length_pixels(
                    length,
                    self.layout.char_width_px,
                    PixelCalcAxis::Horizontal,
                )?;
                let cols = (pixels / self.layout.char_width_px.max(1.0))
                    .ceil()
                    .max(1.0) as u16;
                Some((cols, pixels))
            }
            DisplayStretchWidth::AlignTo(prop) => {
                // GNU `display_line`/`produce_stretch_glyph` (xdisp.c) feeds
                // the `:align-to` operand to `calc_pixel_width_or_height` with
                // `*align_to == -1`, then takes the difference from the current
                // pen X. We mirror the buffer text path exactly (see
                // `DisplaySpaceWidthPolicy::resolve`), substituting
                // the chrome row's pen X as `current_x`. For window chrome
                // rows, GNU then adds `window_box_left_offset (TEXT_AREA)` to
                // raw numeric targets; region-symbol targets have already set
                // `align_to >= 0` and keep the resolved region coordinate.
                let mut align_to: i32 = -1;
                // An `(image …)` operand resolves to the image's own pixel
                // width (GNU `lookup_image`, xdisp.c:30506). Resolve the
                // operand's images for this evaluation; the sizes are owned, so
                // the evaluator itself stays free of the display host.
                let pixel_calc = self.layout.pixel_calc_for_space_spec(prop);
                let evaluated =
                    calc_pixel_width_or_height(&pixel_calc, prop, true, Some(&mut align_to));
                // GNU xdisp.c:32878 — when no width form computes, the stretch
                // falls back to the canonical char width rather than vanishing.
                let Some(pixels) = evaluated else {
                    return Some((1, self.layout.char_width_px.max(1.0)));
                };
                let content_x = self.layout.tab_policy.origin_x_px;
                let raw_align_base_x = if align_to < 0
                    && matches!(
                        self.layout.role,
                        GlyphRowRole::ModeLine | GlyphRowRole::HeaderLine | GlyphRowRole::TabLine
                    ) {
                    self.layout.pixel_calc.text_area_left as f32
                } else {
                    content_x
                };
                let target_x = if align_to >= 0 {
                    align_to as f32 + pixels as f32
                } else {
                    raw_align_base_x + pixels as f32
                };
                let current =
                    self.layout.tab_policy.origin_x_px + self.current_text_metrics().width_px();
                let width_px = (target_x - current).max(0.0);
                if !width_px.is_finite() {
                    return None;
                }
                let cols = (width_px / self.layout.char_width_px.max(1.0)).round() as u16;
                Some((cols, width_px))
            }
        }
    }

    /// Evaluate a `(space :width/:height/:ascent …)` length.
    ///
    /// `axis` selects GNU's `width_p`: `:width` is horizontal (xdisp.c:32794),
    /// while `:height` (:32893) and `:ascent` (:32914) are vertical, so bare
    /// numbers scale by `FRAME_LINE_HEIGHT` rather than `FRAME_COLUMN_WIDTH`
    /// and `in`/`mm`/`cm` convert through the vertical resolution.
    fn length_pixels(
        &self,
        length: &DisplayLength,
        em_px: f32,
        axis: PixelCalcAxis,
    ) -> Option<f32> {
        match length {
            DisplayLength::Columns(cols) => Some(f32::from(*cols) * self.layout.char_width_px),
            DisplayLength::Pixels(px) => Some(*px),
            DisplayLength::Em(em) => Some(*em * em_px.max(1.0)),
            // `:width`/`:height` arithmetic forms route through the single
            // GNU-faithful evaluator. `em_px` (the caller's base unit) is the
            // frame column/line size in the pixel-calc context already, so the
            // authority scales bare numbers consistently with the explicit
            // `Em` arm above.
            DisplayLength::Expr(prop) => {
                let pixel_calc = self.layout.pixel_calc_for_space_spec(prop);
                calc_pixel_width_or_height(&pixel_calc, prop, axis.is_horizontal(), None)
                    .map(|pixels| pixels as f32)
            }
        }
        .filter(|pixels| pixels.is_finite() && *pixels >= 0.0)
    }

    fn face_id(&self, face: RenderFaceRef) -> FaceId {
        render_face_ref_id(
            face,
            render_face_ref_id(self.layout.base_face, FaceId::new(0)),
        )
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
#[path = "builder_test.rs"]
mod tests;

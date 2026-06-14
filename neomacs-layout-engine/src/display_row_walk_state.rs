use crate::display_row_geometry::{
    DisplayRowGeometryState, DisplayRowHitRange, DisplayRowMarker, DisplayRowStartMarker,
};
use neomacs_display_protocol::types::Color;
use neovm_core::buffer::LispCharPos1;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct WordWrapBreakCandidate {
    byte_idx: usize,
    charpos: i64,
    display_point_count: usize,
    row_first_display_pos: Option<LispCharPos1>,
    row_last_display_pos: Option<LispCharPos1>,
    available: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WordWrapRenderState {
    enabled: bool,
    may_wrap: bool,
    candidate: WordWrapBreakCandidate,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct HorizontalScrollSkipState {
    configured_columns: i32,
    remaining_columns: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LineNumberRenderState {
    enabled: bool,
    current_line: i64,
    point_line: i64,
    render_pending: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FaceScanCheckpoint {
    next_check: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum BoxFaceRowState {
    Inactive,
    Active { row: DisplayRowMarker, start_x: f32 },
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ActiveDisplayPropertySpan<T> {
    value: Option<T>,
    end_charpos: i64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TrailingWhitespaceRenderState {
    background: Option<Color>,
    start_marker: DisplayRowStartMarker,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HitRowRangeTracker {
    start_charpos: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TextPropertyScanCheckpoints {
    invisible_next: i64,
    display_next: i64,
}

impl WordWrapBreakCandidate {
    pub(crate) fn record(
        &mut self,
        byte_idx: usize,
        charpos: i64,
        display_point_count: usize,
        row_display_positions: (Option<LispCharPos1>, Option<LispCharPos1>),
    ) {
        self.byte_idx = byte_idx;
        self.charpos = charpos;
        self.display_point_count = display_point_count;
        self.row_first_display_pos = row_display_positions.0;
        self.row_last_display_pos = row_display_positions.1;
        self.available = true;
    }

    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn is_available(&self) -> bool {
        self.available
    }

    pub(crate) fn byte_idx(&self) -> usize {
        self.byte_idx
    }

    pub(crate) fn charpos(&self) -> i64 {
        self.charpos
    }

    pub(crate) fn display_point_count(&self) -> usize {
        self.display_point_count
    }

    pub(crate) fn row_display_positions(&self) -> (Option<LispCharPos1>, Option<LispCharPos1>) {
        (self.row_first_display_pos, self.row_last_display_pos)
    }
}

impl WordWrapRenderState {
    pub(crate) fn new(enabled: bool) -> Self {
        Self {
            enabled,
            may_wrap: false,
            candidate: WordWrapBreakCandidate::default(),
        }
    }

    pub(crate) fn can_record_candidate(self, ch: char) -> bool {
        self.enabled && self.may_wrap && char_can_wrap_before_basic(ch)
    }

    pub(crate) fn record_candidate(
        &mut self,
        ch: char,
        byte_idx: usize,
        charpos: i64,
        display_point_count: usize,
        row_display_positions: (Option<LispCharPos1>, Option<LispCharPos1>),
    ) {
        if self.can_record_candidate(ch) {
            self.candidate.record(
                byte_idx,
                charpos,
                display_point_count,
                row_display_positions,
            );
        }
    }

    pub(crate) fn allow_after_current_char(&mut self, ch: char) {
        self.may_wrap = char_can_wrap_after_basic(ch);
    }

    pub(crate) fn disallow_after_current_char(&mut self) {
        self.may_wrap = false;
    }

    pub(crate) fn reset_after_row_transition(&mut self) {
        self.may_wrap = false;
        self.candidate.clear();
    }

    pub(crate) fn has_candidate(self) -> bool {
        self.enabled && self.candidate.is_available()
    }

    pub(crate) fn candidate(self) -> WordWrapBreakCandidate {
        self.candidate
    }
}

impl HorizontalScrollSkipState {
    pub(crate) fn new(truncate_lines: bool, hscroll_columns: i32) -> Self {
        let configured_columns = if truncate_lines {
            hscroll_columns.max(0) as i32
        } else {
            0
        };
        Self {
            configured_columns,
            remaining_columns: configured_columns,
        }
    }

    pub(crate) fn reset_line(&mut self) {
        self.remaining_columns = self.configured_columns;
    }

    pub(crate) fn should_skip(self) -> bool {
        self.remaining_columns > 0
    }

    pub(crate) fn should_show_left_truncation(self) -> bool {
        self.configured_columns > 0
    }

    pub(crate) fn consumed_columns(self) -> i32 {
        self.configured_columns - self.remaining_columns
    }

    pub(crate) fn consume_columns(&mut self, columns: i32) {
        self.remaining_columns -= columns.max(0).min(self.remaining_columns);
    }
}

impl LineNumberRenderState {
    pub(crate) fn new(enabled: bool, current_line: i64, point_line: i64) -> Self {
        Self {
            enabled,
            current_line,
            point_line,
            render_pending: enabled,
        }
    }

    pub(crate) fn should_render(self) -> bool {
        self.enabled && self.render_pending
    }

    pub(crate) fn consume_render_request(&mut self) {
        self.render_pending = false;
    }

    pub(crate) fn advance_line(&mut self) {
        self.current_line += 1;
        self.render_pending = self.enabled;
    }

    pub(crate) fn advance_hidden_line(&mut self) {
        self.current_line += 1;
    }

    pub(crate) fn current_line(self) -> i64 {
        self.current_line
    }

    #[cfg(test)]
    pub(crate) fn point_line(self) -> i64 {
        self.point_line
    }

    pub(crate) fn is_current_line(self) -> bool {
        self.current_line == self.point_line
    }

    pub(crate) fn display_number(self, mode: u8, current_absolute: bool, offset: i64) -> i64 {
        match mode {
            2 | 3 => {
                if current_absolute && self.is_current_line() {
                    (self.current_line + offset).abs()
                } else {
                    (self.current_line - self.point_line).abs()
                }
            }
            _ => (self.current_line + offset).abs(),
        }
    }
}

impl FaceScanCheckpoint {
    pub(crate) fn initial() -> Self {
        Self { next_check: 0 }
    }

    pub(crate) fn should_resolve_at(self, charpos: usize) -> bool {
        charpos >= self.next_check
    }

    pub(crate) fn invalidate(&mut self) {
        self.next_check = 0;
    }

    pub(crate) fn next_check_mut(&mut self) -> &mut usize {
        &mut self.next_check
    }
}

impl BoxFaceRowState {
    pub(crate) fn inactive() -> Self {
        Self::Inactive
    }

    pub(crate) fn activate(&mut self, row: DisplayRowMarker, start_x: f32) {
        *self = Self::Active { row, start_x };
    }

    pub(crate) fn continue_on_row(&mut self, row: DisplayRowMarker, start_x: f32) {
        if self.is_active() {
            self.activate(row, start_x);
        }
    }

    pub(crate) fn clear(&mut self) {
        *self = Self::Inactive;
    }

    pub(crate) fn is_active(&self) -> bool {
        matches!(self, Self::Active { .. })
    }

    pub(crate) fn start_x(&self) -> Option<f32> {
        match self {
            Self::Active { start_x, .. } => Some(*start_x),
            Self::Inactive => None,
        }
    }

    pub(crate) fn row(&self) -> DisplayRowMarker {
        match self {
            Self::Active { row, .. } => *row,
            Self::Inactive => DisplayRowMarker::Inactive,
        }
    }
}

impl<T: Copy> ActiveDisplayPropertySpan<T> {
    pub(crate) fn inactive() -> Self {
        Self {
            value: None,
            end_charpos: 0,
        }
    }

    pub(crate) fn set(&mut self, value: T, end_charpos: i64) {
        self.value = Some(value);
        self.end_charpos = end_charpos;
    }

    pub(crate) fn clear(&mut self) {
        self.value = None;
        self.end_charpos = 0;
    }

    pub(crate) fn clear_if_expired(&mut self, charpos: i64, inactive_end_charpos: i64) -> bool {
        if self.value.is_some()
            && self.end_charpos > inactive_end_charpos
            && charpos >= self.end_charpos
        {
            self.clear();
            true
        } else {
            false
        }
    }

    pub(crate) fn value(&self) -> Option<T> {
        self.value
    }

    pub(crate) fn value_or(&self, default: T) -> T {
        self.value.unwrap_or(default)
    }
}

impl TrailingWhitespaceRenderState {
    pub(crate) fn new(enabled: bool, background_pixel: u32) -> Self {
        Self {
            background: enabled.then(|| Color::from_pixel(background_pixel)),
            start_marker: DisplayRowStartMarker::Inactive,
        }
    }

    #[cfg(test)]
    pub(crate) fn background(self) -> Option<Color> {
        self.background
    }

    #[cfg(test)]
    pub(crate) fn start_marker(self) -> DisplayRowStartMarker {
        self.start_marker
    }

    pub(crate) fn reset_after_row_transition(&mut self) {
        self.start_marker = DisplayRowStartMarker::Inactive;
    }

    pub(crate) fn track_rendered_char(&mut self, ch: char, start_marker: DisplayRowStartMarker) {
        if self.background.is_none() {
            return;
        }

        if ch == ' ' || ch == '\t' {
            if !self.start_marker.is_active() {
                self.start_marker = start_marker;
            }
        } else {
            self.reset_after_row_transition();
        }
    }

    pub(crate) fn highlight_start_x(
        self,
        geometry: &DisplayRowGeometryState,
    ) -> Option<(Color, f32)> {
        Some((self.background?, self.start_marker.x_on(geometry)?))
    }
}

impl HitRowRangeTracker {
    pub(crate) fn new(start_charpos: i64) -> Self {
        Self { start_charpos }
    }

    pub(crate) fn start(self) -> i64 {
        self.start_charpos
    }

    pub(crate) fn range_to(self, end_charpos: i64) -> DisplayRowHitRange {
        DisplayRowHitRange {
            charpos_start: self.start_charpos,
            charpos_end: end_charpos,
        }
    }

    pub(crate) fn advance_to(&mut self, start_charpos: i64) {
        self.start_charpos = start_charpos;
    }

    pub(crate) fn should_finish_current_row(
        self,
        current_charpos: i64,
        has_pending_row_output: bool,
    ) -> bool {
        current_charpos > self.start_charpos || has_pending_row_output
    }
}

impl TextPropertyScanCheckpoints {
    pub(crate) fn new(start_charpos: i64) -> Self {
        Self {
            invisible_next: start_charpos,
            display_next: start_charpos,
        }
    }

    pub(crate) fn should_check_invisible(self, charpos: i64) -> bool {
        charpos >= self.invisible_next
    }

    pub(crate) fn should_check_display(self, charpos: i64) -> bool {
        charpos >= self.display_next
    }

    pub(crate) fn record_invisible_next(&mut self, charpos: i64) {
        self.invisible_next = charpos;
    }

    pub(crate) fn record_display_next(&mut self, charpos: i64) {
        self.display_next = charpos;
    }

    pub(crate) fn display_skip_to(self, accessible_end: i64) -> i64 {
        self.display_next.min(accessible_end)
    }

    pub(crate) fn display_next(self) -> i64 {
        self.display_next
    }
}

#[inline]
fn is_word_wrap_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\t')
}

#[inline]
fn char_can_wrap_before_basic(ch: char) -> bool {
    !matches!(ch, ' ' | '\t' | '\n' | '\r')
}

#[inline]
fn char_can_wrap_after_basic(ch: char) -> bool {
    is_word_wrap_whitespace(ch)
}

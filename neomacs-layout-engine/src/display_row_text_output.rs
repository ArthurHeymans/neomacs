use crate::coords::layout_i64_char_pos_to_lisp_char_pos;
use crate::display_item::DisplaySourcePosition;
use crate::display_row_builder::{DisplayRowGlyphSlot, DisplayRowPosition};
use neovm_core::buffer::LispCharPos1;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextOutputSpanContext {
    row: usize,
    row_y: f32,
    glyph_y: f32,
    height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextRowOutput {
    pub(crate) row: usize,
    pub(crate) row_y: f32,
    pub(crate) glyph_y: f32,
    pub(crate) height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextOutputSpan {
    pub(crate) buffer_pos: LispCharPos1,
    pub(crate) row: usize,
    pub(crate) row_y: f32,
    pub(crate) glyph_y: f32,
    pub(crate) height: f32,
    pub(crate) start: DisplayRowPosition,
    pub(crate) end: DisplayRowPosition,
}

impl TextRowOutput {
    pub(crate) fn span_context(self) -> TextOutputSpanContext {
        TextOutputSpanContext::new(self.row, self.row_y, self.glyph_y, self.height)
    }

    pub(crate) fn spans_for_source_slots(
        self,
        slots: &[DisplayRowGlyphSlot],
    ) -> Vec<TextOutputSpan> {
        self.span_context().spans_for_source_slots(slots)
    }
}

impl TextOutputSpanContext {
    pub(crate) fn new(row: usize, row_y: f32, glyph_y: f32, height: f32) -> Self {
        Self {
            row,
            row_y,
            glyph_y,
            height,
        }
    }

    fn span_for_buffer_slot(self, slot: &DisplayRowGlyphSlot) -> Option<TextOutputSpan> {
        let DisplaySourcePosition::Buffer { char_pos, .. } = slot.source else {
            return None;
        };
        Some(TextOutputSpan {
            buffer_pos: layout_i64_char_pos_to_lisp_char_pos(char_pos.get() as i64),
            row: self.row,
            row_y: self.row_y,
            glyph_y: self.glyph_y,
            height: self.height,
            start: DisplayRowPosition {
                x_px: slot.x_px,
                col: slot.col,
            },
            end: DisplayRowPosition {
                x_px: slot.x_px + slot.width_px,
                col: slot.col + slot.width_cols,
            },
        })
    }

    pub(crate) fn spans_for_source_slots(
        self,
        slots: &[DisplayRowGlyphSlot],
    ) -> Vec<TextOutputSpan> {
        let mut spans: Vec<TextOutputSpan> = Vec::new();
        for slot in slots {
            let Some(span) = self.span_for_buffer_slot(slot) else {
                continue;
            };
            if let Some(pending) = spans.last_mut()
                && pending.can_merge(span)
            {
                pending.merge(span);
                continue;
            }
            spans.push(span);
        }
        spans
    }
}

impl TextOutputSpan {
    fn can_merge(self, next: Self) -> bool {
        self.buffer_pos == next.buffer_pos
            && self.row == next.row
            && self.row_y == next.row_y
            && self.glyph_y == next.glyph_y
            && self.height == next.height
            && self.end == next.start
    }

    fn merge(&mut self, next: Self) {
        self.end = next.end;
    }
}

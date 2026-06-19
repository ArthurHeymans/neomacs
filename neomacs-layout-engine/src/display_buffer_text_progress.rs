use crate::display_buffer_text_source::BufferTextSourcePosition;
use crate::display_row_builder::DisplayRowPosition;

pub(crate) struct BufferTextWindowRowProgressState<'emit> {
    pub(crate) x: &'emit mut f32,
    pub(crate) col: &'emit mut usize,
}

impl<'emit> BufferTextWindowRowProgressState<'emit> {
    pub(crate) fn new(x: &'emit mut f32, col: &'emit mut usize) -> Self {
        Self { x, col }
    }

    pub(crate) fn row_position(&self) -> DisplayRowPosition {
        DisplayRowPosition {
            x_px: *self.x,
            col: *self.col,
        }
    }

    pub(crate) fn reborrow(&mut self) -> BufferTextWindowRowProgressState<'_> {
        BufferTextWindowRowProgressState {
            x: self.x,
            col: self.col,
        }
    }

    pub(crate) fn apply_position(&mut self, position: DisplayRowPosition) {
        *self.x = position.x_px;
        *self.col = position.col;
    }
}

pub(crate) struct BufferTextWindowProgressState<'emit> {
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) row: BufferTextWindowRowProgressState<'emit>,
}

impl<'emit> BufferTextWindowProgressState<'emit> {
    pub(crate) fn new(
        byte_idx: &'emit mut usize,
        charpos: &'emit mut i64,
        x: &'emit mut f32,
        col: &'emit mut usize,
    ) -> Self {
        Self {
            byte_idx,
            charpos,
            row: BufferTextWindowRowProgressState::new(x, col),
        }
    }

    pub(crate) fn row_position(&self) -> DisplayRowPosition {
        self.row.row_position()
    }

    pub(crate) fn charpos(&self) -> i64 {
        *self.charpos
    }

    pub(crate) fn source_position(&self) -> BufferTextSourcePosition {
        BufferTextSourcePosition::new(*self.byte_idx, *self.charpos)
    }

    pub(crate) fn apply_source_position(&mut self, position: BufferTextSourcePosition) {
        *self.byte_idx = position.byte_idx();
        *self.charpos = position.charpos();
    }

    pub(crate) fn reborrow(&mut self) -> BufferTextWindowProgressState<'_> {
        BufferTextWindowProgressState {
            byte_idx: self.byte_idx,
            charpos: self.charpos,
            row: self.row.reborrow(),
        }
    }
}

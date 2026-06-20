use crate::display_buffer_text_source::BufferTextSourceItem;
use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplaySourcePosition, DisplayTextRun, RenderFaceRef, SourceSpan,
};
use crate::display_property::DisplayPropertyClassification;
use crate::display_source::BufferDisplayReplacementSource;
use neovm_core::buffer::{CharPos0, EmacsBytePos};
use neovm_core::emacs_core::Value;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextReplacementItem {
    value: Value,
    classification: DisplayPropertyClassification,
    replacement_source: BufferDisplayReplacementSource,
    start_byte_pos: EmacsBytePos,
    end_byte_pos: EmacsBytePos,
    start_charpos: CharPos0,
    end_charpos: CharPos0,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BufferTextReplacementSourceAnchor {
    byte_idx: usize,
    charpos: i64,
}

impl BufferTextReplacementItem {
    pub(crate) fn new(
        value: Value,
        classification: DisplayPropertyClassification,
        replacement_source: BufferDisplayReplacementSource,
        start_byte_pos: EmacsBytePos,
        end_byte_pos: EmacsBytePos,
        start_charpos: CharPos0,
        end_charpos: CharPos0,
    ) -> Self {
        Self {
            value,
            classification,
            replacement_source,
            start_byte_pos,
            end_byte_pos,
            start_charpos,
            end_charpos,
        }
    }

    pub(crate) fn value(&self) -> Value {
        self.value
    }

    pub(crate) fn classification(&self) -> &DisplayPropertyClassification {
        &self.classification
    }

    pub(crate) fn replacement_source(&self) -> BufferDisplayReplacementSource {
        self.replacement_source
    }

    pub(crate) fn start_byte_idx(&self, text_start_byte: usize) -> Option<usize> {
        self.start_byte_pos.get().checked_sub(text_start_byte)
    }

    pub(crate) fn source_anchor(
        &self,
        text_start_byte: usize,
    ) -> Option<BufferTextReplacementSourceAnchor> {
        Some(BufferTextReplacementSourceAnchor {
            byte_idx: self.start_byte_idx(text_start_byte)?,
            charpos: self.start_charpos(),
        })
    }

    pub(crate) fn start_charpos(&self) -> i64 {
        self.start_charpos.get() as i64
    }

    pub(crate) fn start_charpos0(&self) -> CharPos0 {
        self.start_charpos
    }

    pub(crate) fn end_charpos(&self) -> i64 {
        self.end_charpos.get() as i64
    }

    pub(crate) fn source_text<'a>(
        &self,
        text_start_byte: usize,
        text: &'a [u8],
    ) -> Option<&'a [u8]> {
        Some(text.get(self.start_byte_idx(text_start_byte)?..)?)
    }

    pub(crate) fn fallback_source_item(
        &self,
        text_start_byte: usize,
        text: &[u8],
        face: RenderFaceRef,
    ) -> Option<BufferTextSourceItem> {
        let start_byte_idx = self.start_byte_idx(text_start_byte)?;
        let end_byte_idx = self.end_byte_pos.get().checked_sub(text_start_byte)?;
        let source_text = std::str::from_utf8(text.get(start_byte_idx..end_byte_idx)?).ok()?;
        if source_text.is_empty() {
            return None;
        }
        let source_char = source_text.chars().next();
        let item = DisplayItem::new(
            SourceSpan::new(
                DisplaySourcePosition::buffer(
                    self.replacement_source.buffer_id(),
                    self.start_charpos,
                    self.start_byte_pos,
                ),
                DisplaySourcePosition::buffer(
                    self.replacement_source.buffer_id(),
                    self.end_charpos,
                    self.end_byte_pos,
                ),
            ),
            face,
            DisplayItemKind::TextRun(DisplayTextRun::new(source_text.to_owned())),
        );
        Some(BufferTextSourceItem::new(
            item,
            start_byte_idx,
            self.start_charpos(),
            source_char,
        ))
    }
}

impl BufferTextReplacementSourceAnchor {
    pub(crate) fn matches(self, byte_idx: usize, charpos: i64) -> bool {
        self.byte_idx == byte_idx && self.charpos == charpos
    }
}

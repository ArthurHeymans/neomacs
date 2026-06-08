#![allow(dead_code)]

use neovm_core::buffer::{BufferId, CharPos0, EmacsBytePos};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct DisplaySourceId(u64);

impl DisplaySourceId {
    pub(crate) const fn new(id: u64) -> Self {
        Self(id)
    }

    pub(crate) const fn get(self) -> u64 {
        self.0
    }
}

impl From<u64> for DisplaySourceId {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum DisplaySourcePosition {
    Buffer {
        buffer_id: BufferId,
        char_pos: CharPos0,
        byte_pos: EmacsBytePos,
    },
    LispString {
        source_id: DisplaySourceId,
        char_index: usize,
        byte_index: usize,
    },
    Synthetic {
        source_id: DisplaySourceId,
        offset: usize,
    },
}

impl DisplaySourcePosition {
    pub(crate) const fn buffer(
        buffer_id: BufferId,
        char_pos: CharPos0,
        byte_pos: EmacsBytePos,
    ) -> Self {
        Self::Buffer {
            buffer_id,
            char_pos,
            byte_pos,
        }
    }

    pub(crate) const fn lisp_string(source_id: u64, char_index: usize, byte_index: usize) -> Self {
        Self::LispString {
            source_id: DisplaySourceId::new(source_id),
            char_index,
            byte_index,
        }
    }

    pub(crate) const fn synthetic(source_id: u64, offset: usize) -> Self {
        Self::Synthetic {
            source_id: DisplaySourceId::new(source_id),
            offset,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct SourceSpan {
    pub(crate) start: DisplaySourcePosition,
    pub(crate) end: DisplaySourcePosition,
}

impl SourceSpan {
    pub(crate) const fn new(start: DisplaySourcePosition, end: DisplaySourcePosition) -> Self {
        Self { start, end }
    }

    pub(crate) const fn lisp_string(
        source_id: u64,
        start_char: usize,
        end_char: usize,
        start_byte: usize,
        end_byte: usize,
    ) -> Self {
        Self::new(
            DisplaySourcePosition::lisp_string(source_id, start_char, start_byte),
            DisplaySourcePosition::lisp_string(source_id, end_char, end_byte),
        )
    }

    pub(crate) const fn synthetic(source_id: u64, start_offset: usize, end_offset: usize) -> Self {
        Self::new(
            DisplaySourcePosition::synthetic(source_id, start_offset),
            DisplaySourcePosition::synthetic(source_id, end_offset),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RenderFaceRef {
    Inherit,
    FaceId(u32),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplayItem {
    pub(crate) span: SourceSpan,
    pub(crate) face: RenderFaceRef,
    pub(crate) kind: DisplayItemKind,
}

impl DisplayItem {
    pub(crate) const fn new(span: SourceSpan, face: RenderFaceRef, kind: DisplayItemKind) -> Self {
        Self { span, face, kind }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DisplayItemKind {
    TextRun(DisplayTextRun),
    SourceMappedText(DisplaySourceMappedText),
    ControlChar { ch: char },
    Glyphless(DisplayGlyphless),
    Stretch(DisplayStretch),
    Image(DisplayImageItem),
    Video(DisplayVideoItem),
    Xwidget(DisplayXwidgetItem),
    RowBreak(DisplayRowBreak),
    CursorAnchor(CursorAnchor),
    HitTestAnchor(DisplayHitTestAnchor),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DisplayTextRun {
    pub(crate) text: Box<str>,
}

impl DisplayTextRun {
    pub(crate) fn new(text: impl Into<Box<str>>) -> Self {
        Self { text: text.into() }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DisplaySourceMappedText {
    pub(crate) text: Box<str>,
}

impl DisplaySourceMappedText {
    pub(crate) fn new(text: impl Into<Box<str>>) -> Self {
        Self { text: text.into() }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GlyphlessMethod {
    ZeroWidth,
    ThinSpace,
    HexCode,
    EmptyBox,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GlyphlessJoinerPolicy {
    ClassifyAsGlyphless,
    PreserveForComposition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DisplayGlyphless {
    pub(crate) ch: char,
    pub(crate) method: GlyphlessMethod,
}

pub(crate) fn control_char_caret_char(ch: char) -> Option<char> {
    match ch {
        '\u{0000}'..='\u{001f}' => Some(char::from((ch as u8) + b'@')),
        '\u{007f}' => Some('?'),
        _ => None,
    }
}

pub(crate) fn glyphless_method_for_char(
    ch: char,
    joiner_policy: GlyphlessJoinerPolicy,
) -> Option<GlyphlessMethod> {
    if joiner_policy == GlyphlessJoinerPolicy::PreserveForComposition
        && crate::composition::is_composition_joiner(ch)
    {
        return None;
    }

    let cp = ch as u32;
    match cp {
        0x80..=0x9f | 0xfff0..=0xfff8 => Some(GlyphlessMethod::HexCode),
        0xfffc => Some(GlyphlessMethod::EmptyBox),
        0xfeff
        | 0x200b..=0x200f
        | 0x2028..=0x2029
        | 0xe0001..=0xe007f
        | 0xe0100..=0xe01ef
        | 0xfe00..=0xfe0f => Some(GlyphlessMethod::ZeroWidth),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DisplayLength {
    Columns(u16),
    Pixels(f32),
    Em(f32),
    Expr(DisplayLengthExpr),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DisplayLengthSymbol {
    Height,
    Width,
    Text,
    Left,
    Right,
    Center,
    LeftFringe,
    RightFringe,
    LeftMargin,
    RightMargin,
    ScrollBar,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DisplayLengthExpr {
    Pixels(f32),
    Em(f32),
    Symbol(DisplayLengthSymbol),
    Variable(Box<str>),
    Add(Vec<DisplayLengthExpr>),
    Sub(Vec<DisplayLengthExpr>),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DisplayStretchWidth {
    Length(DisplayLength),
    AlignTo(DisplayLengthExpr),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplayStretch {
    pub(crate) width: DisplayStretchWidth,
    pub(crate) height: Option<DisplayLength>,
    pub(crate) ascent: Option<DisplayLength>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DisplayImageItem {
    pub(crate) image_id: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DisplayVideoItem {
    pub(crate) video_id: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DisplayXwidgetItem {
    pub(crate) xwidget_id: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DisplayRowBreak {
    pub(crate) reason: DisplayRowBreakReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DisplayRowBreakReason {
    ExplicitNewline,
    Wrap,
    Truncate,
    EndOfSource,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CursorAnchor {
    pub(crate) kind: CursorAnchorKind,
    pub(crate) position: DisplaySourcePosition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CursorAnchorKind {
    Point,
    WindowStart,
    SourceBoundary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DisplayHitTestAnchor {
    pub(crate) position: DisplaySourcePosition,
}

#[cfg(test)]
#[path = "display_item_test.rs"]
mod tests;

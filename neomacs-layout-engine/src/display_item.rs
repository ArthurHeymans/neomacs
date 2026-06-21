use crate::display_property::DisplayPropertyClassification;
use neovm_core::buffer::{BufferId, CharPos0, EmacsBytePos};
use neovm_core::emacs_core::Value;

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

    pub(crate) fn buffer_end_charpos(&self) -> Option<CharPos0> {
        let DisplaySourcePosition::Buffer { char_pos, .. } = self.end else {
            return None;
        };
        Some(char_pos)
    }

    pub(crate) fn buffer_byte_len(&self) -> Option<usize> {
        let DisplaySourcePosition::Buffer {
            byte_pos: start, ..
        } = self.start
        else {
            return None;
        };
        let DisplaySourcePosition::Buffer { byte_pos: end, .. } = self.end else {
            return None;
        };
        end.get().checked_sub(start.get())
    }

    #[cfg(test)]
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
    #[allow(dead_code)]
    Inherit,
    FaceId(u32),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplayItem {
    pub(crate) span: SourceSpan,
    pub(crate) face: RenderFaceRef,
    pub(crate) kind: DisplayItemKind,
    pub(crate) layout: DisplayItemLayout,
}

impl DisplayItem {
    pub(crate) const fn new(span: SourceSpan, face: RenderFaceRef, kind: DisplayItemKind) -> Self {
        Self {
            span,
            face,
            kind,
            layout: DisplayItemLayout {
                raise: None,
                height: None,
            },
        }
    }

    pub(crate) const fn with_layout(mut self, layout: DisplayItemLayout) -> Self {
        self.layout = layout;
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct DisplayItemLayout {
    pub(crate) raise: Option<f32>,
    pub(crate) height: Option<f32>,
}

impl DisplayItemLayout {
    pub(crate) fn vertical_offset_px(self, row_height_px: f32) -> f32 {
        self.raise
            .filter(|factor| factor.is_finite())
            .map(|factor| -(factor * row_height_px.max(1.0)))
            .unwrap_or(0.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DisplayItemKind {
    TextRun(DisplayTextRun),
    SourceMappedText(DisplaySourceMappedText),
    ControlChar { ch: char },
    Glyphless(DisplayGlyphless),
    Stretch(DisplayStretch),
    MediaReplacement(DisplayMediaReplacement),
    RowBreak(DisplayRowBreak),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferDisplayReplacementSource {
    buffer_id: BufferId,
    char_pos: CharPos0,
    byte_pos: EmacsBytePos,
    end_char_pos: CharPos0,
    end_byte_pos: EmacsBytePos,
}

impl BufferDisplayReplacementSource {
    #[cfg(test)]
    pub(crate) fn new(buffer_id: BufferId, char_pos: CharPos0, byte_pos: EmacsBytePos) -> Self {
        Self {
            buffer_id,
            char_pos,
            byte_pos,
            end_char_pos: char_pos.add_len(neovm_core::buffer::CharLen::new(1)),
            end_byte_pos: byte_pos,
        }
    }

    pub(crate) fn spanning(
        buffer_id: BufferId,
        char_pos: CharPos0,
        byte_pos: EmacsBytePos,
        end_char_pos: CharPos0,
        end_byte_pos: EmacsBytePos,
    ) -> Self {
        Self {
            buffer_id,
            char_pos,
            byte_pos,
            end_char_pos,
            end_byte_pos,
        }
    }

    pub(crate) fn buffer_id(self) -> BufferId {
        self.buffer_id
    }

    fn span(self) -> SourceSpan {
        SourceSpan::new(
            DisplaySourcePosition::buffer(self.buffer_id, self.char_pos, self.byte_pos),
            DisplaySourcePosition::buffer(self.buffer_id, self.end_char_pos, self.end_byte_pos),
        )
    }

    fn item(self, face_id: u32, kind: DisplayItemKind) -> DisplayItem {
        self.item_with_face(RenderFaceRef::FaceId(face_id), kind)
    }

    pub(crate) fn display_item(self, face_id: u32, kind: DisplayItemKind) -> DisplayItem {
        self.item(face_id, kind)
    }

    fn item_with_face(self, face: RenderFaceRef, kind: DisplayItemKind) -> DisplayItem {
        DisplayItem::new(self.span(), face, kind)
    }

    pub(crate) fn item_from_replacement_string_item(self, item: DisplayItem) -> DisplayItem {
        let kind = match item.kind {
            DisplayItemKind::TextRun(run) => {
                DisplayItemKind::SourceMappedText(DisplaySourceMappedText::new(run.text))
            }
            kind => kind,
        };
        self.item_with_face(item.face, kind)
            .with_layout(item.layout)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplayPropertyReplacementDescriptor {
    value: Value,
    classification: DisplayPropertyClassification,
    replacement_source: BufferDisplayReplacementSource,
    anchor_charpos: CharPos0,
    skip_to_charpos: CharPos0,
}

impl DisplayPropertyReplacementDescriptor {
    pub(crate) fn new(
        value: Value,
        classification: DisplayPropertyClassification,
        replacement_source: BufferDisplayReplacementSource,
        anchor_charpos: CharPos0,
        skip_to_charpos: CharPos0,
    ) -> Self {
        Self {
            value,
            classification,
            replacement_source,
            anchor_charpos,
            skip_to_charpos,
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

    pub(crate) fn anchor_charpos(&self) -> CharPos0 {
        self.anchor_charpos
    }

    pub(crate) fn skip_to_charpos(&self) -> i64 {
        self.skip_to_charpos.get() as i64
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferDisplayPropertyReplacementItem {
    descriptor: DisplayPropertyReplacementDescriptor,
    start_byte_pos: EmacsBytePos,
    end_byte_pos: EmacsBytePos,
    start_charpos: CharPos0,
    end_charpos: CharPos0,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferDisplayPropertyFallbackItem {
    item: DisplayItem,
    start_byte_idx: usize,
    start_charpos: i64,
    source_char: Option<char>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BufferDisplayPropertyReplacementAnchor {
    byte_idx: usize,
    charpos: i64,
}

impl BufferDisplayPropertyReplacementItem {
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
            descriptor: DisplayPropertyReplacementDescriptor::new(
                value,
                classification,
                replacement_source,
                start_charpos,
                end_charpos,
            ),
            start_byte_pos,
            end_byte_pos,
            start_charpos,
            end_charpos,
        }
    }

    pub(crate) fn descriptor(&self) -> &DisplayPropertyReplacementDescriptor {
        &self.descriptor
    }

    pub(crate) fn start_byte_idx(&self, text_start_byte: usize) -> Option<usize> {
        self.start_byte_pos.get().checked_sub(text_start_byte)
    }

    pub(crate) fn source_anchor(
        &self,
        text_start_byte: usize,
    ) -> Option<BufferDisplayPropertyReplacementAnchor> {
        Some(BufferDisplayPropertyReplacementAnchor {
            byte_idx: self.start_byte_idx(text_start_byte)?,
            charpos: self.start_charpos(),
        })
    }

    pub(crate) fn start_charpos(&self) -> i64 {
        self.start_charpos.get() as i64
    }

    pub(crate) fn source_text<'a>(
        &self,
        text_start_byte: usize,
        text: &'a [u8],
    ) -> Option<&'a [u8]> {
        Some(text.get(self.start_byte_idx(text_start_byte)?..)?)
    }

    pub(crate) fn fallback_display_item(
        &self,
        text_start_byte: usize,
        text: &[u8],
        face: RenderFaceRef,
    ) -> Option<BufferDisplayPropertyFallbackItem> {
        let start_byte_idx = self.start_byte_idx(text_start_byte)?;
        let end_byte_idx = self.end_byte_pos.get().checked_sub(text_start_byte)?;
        let source_text = std::str::from_utf8(text.get(start_byte_idx..end_byte_idx)?).ok()?;
        if source_text.is_empty() {
            return None;
        }
        let source_char = source_text.chars().next();
        let replacement_source = self.descriptor.replacement_source();
        let item = DisplayItem::new(
            SourceSpan::new(
                DisplaySourcePosition::buffer(
                    replacement_source.buffer_id(),
                    self.start_charpos,
                    self.start_byte_pos,
                ),
                DisplaySourcePosition::buffer(
                    replacement_source.buffer_id(),
                    self.end_charpos,
                    self.end_byte_pos,
                ),
            ),
            face,
            DisplayItemKind::TextRun(DisplayTextRun::new(source_text.to_owned())),
        );
        Some(BufferDisplayPropertyFallbackItem {
            item,
            start_byte_idx,
            start_charpos: self.start_charpos(),
            source_char,
        })
    }
}

impl BufferDisplayPropertyFallbackItem {
    pub(crate) fn into_parts(self) -> (DisplayItem, usize, i64, Option<char>) {
        (
            self.item,
            self.start_byte_idx,
            self.start_charpos,
            self.source_char,
        )
    }
}

impl BufferDisplayPropertyReplacementAnchor {
    pub(crate) fn matches(self, byte_idx: usize, charpos: i64) -> bool {
        self.byte_idx == byte_idx && self.charpos == charpos
    }
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayImageItem {
    pub(crate) image_id: i32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayVideoItem {
    pub(crate) video_id: i32,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) loop_count: i32,
    pub(crate) autoplay: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayXwidgetItem {
    pub(crate) xwidget_id: i32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayMediaReplacement {
    pub(crate) kind: DisplayMediaReplacementKind,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum DisplayMediaReplacementKind {
    Image {
        image_id: u32,
    },
    Video {
        video_id: u32,
        loop_count: i32,
        autoplay: bool,
    },
    Xwidget {
        xwidget_id: u32,
    },
}

impl DisplayMediaReplacement {
    pub(crate) fn replacement_item(self, mut item: DisplayItem) -> DisplayItem {
        item.kind = DisplayItemKind::Stretch(self.replacement_stretch());
        item
    }

    pub(crate) fn replacement_stretch(self) -> DisplayStretch {
        DisplayStretch {
            width: DisplayStretchWidth::Length(DisplayLength::Pixels(self.width)),
            height: Some(DisplayLength::Pixels(self.height)),
            ascent: Some(DisplayLength::Pixels(self.height)),
        }
    }

    pub(crate) fn image(image: DisplayImageItem) -> Self {
        Self {
            kind: DisplayMediaReplacementKind::Image {
                image_id: image.image_id.max(0) as u32,
            },
            width: display_replacement_dimension(image.width),
            height: display_replacement_dimension(image.height),
        }
    }

    pub(crate) fn video(video: DisplayVideoItem) -> Self {
        Self {
            kind: DisplayMediaReplacementKind::Video {
                video_id: video.video_id.max(0) as u32,
                loop_count: video.loop_count,
                autoplay: video.autoplay,
            },
            width: display_replacement_dimension(video.width),
            height: display_replacement_dimension(video.height),
        }
    }

    pub(crate) fn xwidget(xwidget: DisplayXwidgetItem) -> Self {
        Self {
            kind: DisplayMediaReplacementKind::Xwidget {
                xwidget_id: xwidget.xwidget_id.max(0) as u32,
            },
            width: display_replacement_dimension(xwidget.width),
            height: display_replacement_dimension(xwidget.height),
        }
    }
}

fn display_replacement_dimension(value: f32) -> f32 {
    if value.is_finite() {
        value.max(1.0)
    } else {
        1.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DisplayRowBreak {
    pub(crate) reason: DisplayRowBreakReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DisplayRowBreakReason {
    ExplicitNewline,
    #[allow(dead_code)]
    Wrap,
    #[allow(dead_code)]
    Truncate,
    #[allow(dead_code)]
    EndOfSource,
}

#[cfg(test)]
#[path = "display_item_test.rs"]
mod tests;

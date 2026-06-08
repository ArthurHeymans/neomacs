#![allow(dead_code)]

use crate::display_item::{
    DisplayGlyphless, DisplayItem, DisplayItemKind, DisplayLength, DisplayLengthExpr,
    DisplayLengthSymbol, DisplayRowBreak, DisplayRowBreakReason, DisplaySourcePosition,
    DisplayStretch, DisplayStretchWidth, DisplayTextRun, GlyphlessMethod, RenderFaceRef,
    SourceSpan,
};
use crate::display_space::{DisplaySpaceKey, is_display_space_spec};
use crate::neovm_bridge::LayoutBufferView;
use crate::unicode::decode_utf8;
use neovm_core::buffer::{
    BufferId, CharLen, CharPos0, EmacsBytePos, EmacsByteRange, text_props::TextPropertyTable,
};
use neovm_core::emacs_core::Value;
use neovm_core::emacs_core::value::{get_string_text_properties_table_for_value, list_to_vec};

pub(crate) struct DisplaySourceContext<'a> {
    face_resolver: Option<&'a mut dyn DisplayItemFaceResolver>,
}

impl<'a> DisplaySourceContext<'a> {
    pub(crate) const fn empty() -> Self {
        Self {
            face_resolver: None,
        }
    }

    pub(crate) fn with_face_resolver(resolver: &'a mut dyn DisplayItemFaceResolver) -> Self {
        Self {
            face_resolver: Some(resolver),
        }
    }

    fn resolve_face_ref(&mut self, base: RenderFaceRef, face_value: Value) -> RenderFaceRef {
        self.face_resolver
            .as_mut()
            .map(|resolver| resolver.resolve_face_ref(base, face_value))
            .unwrap_or(base)
    }
}

impl Default for DisplaySourceContext<'_> {
    fn default() -> Self {
        Self::empty()
    }
}

pub(crate) trait DisplayItemSource {
    fn next_item(&mut self, context: &mut DisplaySourceContext<'_>) -> Option<DisplayItem>;
    fn source_position(&self) -> DisplaySourcePosition;
}

pub(crate) trait DisplayItemFaceResolver {
    fn resolve_face_ref(&mut self, base: RenderFaceRef, face_value: Value) -> RenderFaceRef;
}

pub(crate) struct LispStringSourceCursor {
    stack: LispStringSourceStack,
}

impl LispStringSourceCursor {
    pub(crate) fn new(source_id: u64, value: Value, base_face: RenderFaceRef) -> Option<Self> {
        Some(Self {
            stack: LispStringSourceStack::with_root(source_id, value, base_face)?,
        })
    }
}

impl DisplayItemSource for LispStringSourceCursor {
    fn next_item(&mut self, context: &mut DisplaySourceContext<'_>) -> Option<DisplayItem> {
        self.stack.next_item(context)
    }

    fn source_position(&self) -> DisplaySourcePosition {
        self.stack.source_position()
    }
}

pub(crate) struct BufferTextSourceCursor<'a, B: LayoutBufferView + ?Sized> {
    buffer_id: BufferId,
    buffer: &'a B,
    char_pos: CharPos0,
    end: CharPos0,
    base_face: RenderFaceRef,
    replacement_strings: LispStringSourceStack,
}

impl<'a, B: LayoutBufferView + ?Sized> BufferTextSourceCursor<'a, B> {
    pub(crate) fn new(
        buffer_id: BufferId,
        buffer: &'a B,
        start: CharPos0,
        end: CharPos0,
        base_face: RenderFaceRef,
    ) -> Self {
        let accessible_end = buffer.layout_point_max_char_pos();
        let start = start.min(accessible_end);
        let end = end.min(accessible_end).max(start);
        Self {
            buffer_id,
            buffer,
            char_pos: start,
            end,
            base_face,
            replacement_strings: LispStringSourceStack::empty(1),
        }
    }

    fn byte_pos(&self, char_pos: CharPos0) -> EmacsBytePos {
        self.buffer.layout_char_pos_to_emacs_byte_pos(char_pos)
    }

    fn char_at(&self, char_pos: CharPos0) -> Option<char> {
        if char_pos >= self.end {
            return None;
        }
        let start = self.byte_pos(char_pos);
        let end = self.byte_pos(char_pos.add_len(CharLen::new(1)).min(self.end));
        let mut bytes = Vec::new();
        self.buffer
            .layout_copy_emacs_byte_range_to(EmacsByteRange::new(start, end), &mut bytes);
        let (ch, len) = decode_utf8(&bytes);
        (len > 0).then_some(ch)
    }

    fn text_slice(&self, start: CharPos0, end: CharPos0) -> String {
        let mut bytes = Vec::new();
        self.buffer.layout_copy_emacs_byte_range_to(
            EmacsByteRange::new(self.byte_pos(start), self.byte_pos(end)),
            &mut bytes,
        );
        let mut text = String::new();
        let mut offset = 0usize;
        while offset < bytes.len() {
            let (ch, len) = decode_utf8(&bytes[offset..]);
            if len == 0 {
                break;
            }
            text.push(ch);
            offset += len;
        }
        text
    }

    fn span(&self, start: CharPos0, end: CharPos0) -> SourceSpan {
        SourceSpan::new(
            DisplaySourcePosition::buffer(self.buffer_id, start, self.byte_pos(start)),
            DisplaySourcePosition::buffer(self.buffer_id, end, self.byte_pos(end)),
        )
    }

    fn next_property_change(&self, char_pos: CharPos0) -> CharPos0 {
        self.buffer
            .layout_next_text_prop_change_after_emacs_byte_pos(self.byte_pos(char_pos))
            .map(|byte_pos| self.buffer.layout_emacs_byte_pos_to_char_pos(byte_pos))
            .unwrap_or(self.end)
            .min(self.end)
    }

    fn display_prop_at(&self, char_pos: CharPos0) -> Option<Value> {
        self.buffer
            .layout_text_prop_at_emacs_byte_pos(self.byte_pos(char_pos), Value::symbol("display"))
    }

    fn face_at(&self, char_pos: CharPos0, context: &mut DisplaySourceContext<'_>) -> RenderFaceRef {
        let face = self
            .buffer
            .layout_text_prop_at_emacs_byte_pos(self.byte_pos(char_pos), Value::symbol("face"))
            .or_else(|| {
                self.buffer.layout_text_prop_at_emacs_byte_pos(
                    self.byte_pos(char_pos),
                    Value::symbol("font-lock-face"),
                )
            });
        face.map(|value| context.resolve_face_ref(self.base_face, value))
            .unwrap_or(self.base_face)
    }

    fn next_text_run_end(&self, start: CharPos0, limit: CharPos0) -> CharPos0 {
        let mut end = start;
        while end < limit {
            let Some(ch) = self.char_at(end) else {
                break;
            };
            if ch == '\n' || is_control_char(ch) || glyphless_method_for_char(ch).is_some() {
                break;
            }
            end = end.add_len(CharLen::new(1));
        }
        end.max(start.add_len(CharLen::new(1))).min(limit)
    }
}

impl<B: LayoutBufferView + ?Sized> DisplayItemSource for BufferTextSourceCursor<'_, B> {
    fn next_item(&mut self, context: &mut DisplaySourceContext<'_>) -> Option<DisplayItem> {
        loop {
            if let Some(item) = self.replacement_strings.next_item(context) {
                return Some(item);
            }

            if self.char_pos >= self.end {
                return None;
            }

            let start = self.char_pos;
            let property_end = self
                .next_property_change(start)
                .max(start.add_len(CharLen::new(1)))
                .min(self.end);
            let face = self.face_at(start, context);
            let span = self.span(start, property_end);

            if let Some(display_prop) = self.display_prop_at(start) {
                self.char_pos = property_end;
                if display_prop.is_string() {
                    self.replacement_strings.push(display_prop, face);
                    continue;
                }
                if let Some(kind) = parse_display_property(display_prop) {
                    return Some(DisplayItem::new(span, face, kind));
                }
            }

            let ch = self.char_at(start)?;
            if ch == '\n' {
                self.char_pos = start.add_len(CharLen::new(1));
                return Some(DisplayItem::new(
                    self.span(start, self.char_pos),
                    face,
                    DisplayItemKind::RowBreak(DisplayRowBreak {
                        reason: DisplayRowBreakReason::ExplicitNewline,
                    }),
                ));
            }

            if is_control_char(ch) {
                self.char_pos = start.add_len(CharLen::new(1));
                return Some(DisplayItem::new(
                    self.span(start, self.char_pos),
                    face,
                    DisplayItemKind::ControlChar { ch },
                ));
            }

            if let Some(method) = glyphless_method_for_char(ch) {
                self.char_pos = start.add_len(CharLen::new(1));
                return Some(DisplayItem::new(
                    self.span(start, self.char_pos),
                    face,
                    DisplayItemKind::Glyphless(DisplayGlyphless { ch, method }),
                ));
            }

            let end = self.next_text_run_end(start, property_end);
            self.char_pos = end;
            return Some(DisplayItem::new(
                self.span(start, end),
                face,
                DisplayItemKind::TextRun(DisplayTextRun::new(self.text_slice(start, end))),
            ));
        }
    }

    fn source_position(&self) -> DisplaySourcePosition {
        if !self.replacement_strings.is_empty() {
            return self.replacement_strings.source_position();
        }
        DisplaySourcePosition::buffer(self.buffer_id, self.char_pos, self.byte_pos(self.char_pos))
    }
}

fn is_control_char(ch: char) -> bool {
    let code = ch as u32;
    (code <= 0x1f && ch != '\n' && ch != '\t') || code == 0x7f
}

fn glyphless_method_for_char(ch: char) -> Option<GlyphlessMethod> {
    if crate::composition::is_composition_joiner(ch) {
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

enum LispStringAction {
    PopFrame,
    PushReplacement {
        value: Value,
        base_face: RenderFaceRef,
    },
    Emit(DisplayItem),
}

struct LispStringSourceStack {
    frames: Vec<LispStringSourceFrame>,
    next_source_id: u64,
}

impl LispStringSourceStack {
    const fn empty(next_source_id: u64) -> Self {
        Self {
            frames: Vec::new(),
            next_source_id,
        }
    }

    fn with_root(source_id: u64, value: Value, base_face: RenderFaceRef) -> Option<Self> {
        let frame = LispStringSourceFrame::new(source_id, value, base_face)?;
        Some(Self {
            frames: vec![frame],
            next_source_id: source_id.saturating_add(1),
        })
    }

    fn push(&mut self, value: Value, base_face: RenderFaceRef) {
        let source_id = self.allocate_source_id();
        if let Some(frame) = LispStringSourceFrame::new(source_id, value, base_face) {
            self.frames.push(frame);
        }
    }

    fn next_item(&mut self, context: &mut DisplaySourceContext<'_>) -> Option<DisplayItem> {
        loop {
            let action = {
                let frame = self.frames.last_mut()?;
                frame.next_action(context)
            };

            match action {
                LispStringAction::PopFrame => {
                    self.frames.pop();
                }
                LispStringAction::PushReplacement { value, base_face } => {
                    self.push(value, base_face);
                }
                LispStringAction::Emit(item) => return Some(item),
            }
        }
    }

    fn source_position(&self) -> DisplaySourcePosition {
        self.frames
            .last()
            .map(LispStringSourceFrame::source_position)
            .unwrap_or_else(|| DisplaySourcePosition::synthetic(0, 0))
    }

    fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    fn allocate_source_id(&mut self) -> u64 {
        let id = self.next_source_id;
        self.next_source_id = self.next_source_id.saturating_add(1);
        id
    }
}

struct LispStringSourceFrame {
    source_id: u64,
    text: String,
    char_byte_offsets: Vec<usize>,
    props: Option<TextPropertyTable>,
    char_index: usize,
    base_face: RenderFaceRef,
}

impl LispStringSourceFrame {
    fn new(source_id: u64, value: Value, base_face: RenderFaceRef) -> Option<Self> {
        let text = value.as_runtime_string_owned()?;
        let mut char_byte_offsets = text
            .char_indices()
            .map(|(byte, _)| byte)
            .collect::<Vec<_>>();
        char_byte_offsets.push(text.len());
        Some(Self {
            source_id,
            text,
            char_byte_offsets,
            props: get_string_text_properties_table_for_value(value),
            char_index: 0,
            base_face,
        })
    }

    fn next_action(&mut self, context: &mut DisplaySourceContext<'_>) -> LispStringAction {
        if self.char_index >= self.char_count() {
            return LispStringAction::PopFrame;
        }

        let start = self.char_index;
        let property_end = self.next_property_change(start).max(start + 1);
        let face = self.face_at(start, context);
        let span = self.span(start, property_end);

        if let Some(display_prop) = self.display_prop_at(start) {
            self.char_index = property_end;
            if display_prop.is_string() {
                return LispStringAction::PushReplacement {
                    value: display_prop,
                    base_face: face,
                };
            }
            if let Some(kind) = parse_display_property(display_prop) {
                return LispStringAction::Emit(DisplayItem::new(span, face, kind));
            }
        }

        if self.char_at(start) == Some('\n') {
            self.char_index = start + 1;
            return LispStringAction::Emit(DisplayItem::new(
                self.span(start, start + 1),
                face,
                DisplayItemKind::RowBreak(DisplayRowBreak {
                    reason: DisplayRowBreakReason::ExplicitNewline,
                }),
            ));
        }

        let Some(ch) = self.char_at(start) else {
            return LispStringAction::PopFrame;
        };
        if is_control_char(ch) {
            self.char_index = start + 1;
            return LispStringAction::Emit(DisplayItem::new(
                self.span(start, start + 1),
                face,
                DisplayItemKind::ControlChar { ch },
            ));
        }

        if let Some(method) = glyphless_method_for_char(ch) {
            self.char_index = start + 1;
            return LispStringAction::Emit(DisplayItem::new(
                self.span(start, start + 1),
                face,
                DisplayItemKind::Glyphless(DisplayGlyphless { ch, method }),
            ));
        }

        let end = self.next_text_run_end(start, property_end);
        self.char_index = end;
        LispStringAction::Emit(DisplayItem::new(
            self.span(start, end),
            face,
            DisplayItemKind::TextRun(DisplayTextRun::new(self.text_slice(start, end))),
        ))
    }

    fn char_count(&self) -> usize {
        self.char_byte_offsets.len().saturating_sub(1)
    }

    fn source_position(&self) -> DisplaySourcePosition {
        DisplaySourcePosition::lisp_string(
            self.source_id,
            self.char_index,
            self.byte_offset(self.char_index),
        )
    }

    fn span(&self, start: usize, end: usize) -> SourceSpan {
        SourceSpan::new(
            DisplaySourcePosition::lisp_string(self.source_id, start, self.byte_offset(start)),
            DisplaySourcePosition::lisp_string(self.source_id, end, self.byte_offset(end)),
        )
    }

    fn byte_offset(&self, char_index: usize) -> usize {
        self.char_byte_offsets
            .get(char_index.min(self.char_count()))
            .copied()
            .unwrap_or(self.text.len())
    }

    fn char_at(&self, char_index: usize) -> Option<char> {
        let start = self.byte_offset(char_index);
        let end = self.byte_offset(char_index + 1);
        self.text.get(start..end)?.chars().next()
    }

    fn text_slice(&self, start: usize, end: usize) -> String {
        self.text
            .get(self.byte_offset(start)..self.byte_offset(end))
            .unwrap_or_default()
            .to_string()
    }

    fn next_property_change(&self, char_index: usize) -> usize {
        self.props
            .as_ref()
            .and_then(|props| {
                props
                    .next_property_change_after_char_pos(CharPos0::new(char_index))
                    .map(CharPos0::get)
            })
            .unwrap_or_else(|| self.char_count())
            .min(self.char_count())
    }

    fn next_text_run_end(&self, start: usize, limit: usize) -> usize {
        let mut end = start;
        while end < limit {
            let Some(ch) = self.char_at(end) else {
                break;
            };
            if ch == '\n' || is_control_char(ch) || glyphless_method_for_char(ch).is_some() {
                break;
            }
            end += 1;
        }
        end.max(start + 1).min(limit)
    }

    fn display_prop_at(&self, char_index: usize) -> Option<Value> {
        self.props
            .as_ref()?
            .get_property_at_char_pos(CharPos0::new(char_index), Value::symbol("display"))
    }

    fn face_at(&self, char_index: usize, context: &mut DisplaySourceContext<'_>) -> RenderFaceRef {
        let Some(props) = &self.props else {
            return self.base_face;
        };
        let char_pos = CharPos0::new(char_index);
        let face = props
            .get_property_at_char_pos(char_pos, Value::symbol("face"))
            .or_else(|| props.get_property_at_char_pos(char_pos, Value::symbol("font-lock-face")));
        face.map(|value| context.resolve_face_ref(self.base_face, value))
            .unwrap_or(self.base_face)
    }
}

fn parse_display_property(value: Value) -> Option<DisplayItemKind> {
    if is_display_space_spec(&value) {
        return parse_display_space(value).map(DisplayItemKind::Stretch);
    }
    None
}

fn parse_display_space(value: Value) -> Option<DisplayStretch> {
    let items = list_to_vec(&value)?;
    let mut width = None;
    let mut height = None;
    let mut ascent = None;
    let mut i = 1usize;
    while i + 1 < items.len() {
        let key = items[i];
        let val = items[i + 1];
        match DisplaySpaceKey::from_lisp_value(key) {
            Some(DisplaySpaceKey::Width | DisplaySpaceKey::RelativeWidth) => {
                width = parse_display_length(val).map(DisplayStretchWidth::Length);
            }
            Some(DisplaySpaceKey::AlignTo) => {
                width = parse_display_length_expr(val).map(DisplayStretchWidth::AlignTo);
            }
            Some(DisplaySpaceKey::Height | DisplaySpaceKey::RelativeHeight) => {
                height = parse_display_length(val);
            }
            Some(DisplaySpaceKey::Ascent) => {
                ascent = parse_display_length(val);
            }
            None => {}
        }
        i += 2;
    }

    width.map(|width| DisplayStretch {
        width,
        height,
        ascent,
    })
}

fn parse_display_length(value: Value) -> Option<DisplayLength> {
    if let Some(number) = lisp_number(value) {
        return Some(DisplayLength::Em(number));
    }
    parse_display_length_expr(value).map(DisplayLength::Expr)
}

pub(crate) fn parse_display_length_expr(value: Value) -> Option<DisplayLengthExpr> {
    if let Some(number) = lisp_number(value) {
        return Some(DisplayLengthExpr::Em(number));
    }

    if value.is_symbol() {
        let name = value.as_symbol_name()?;
        return Some(
            display_length_symbol(name)
                .map(DisplayLengthExpr::Symbol)
                .unwrap_or_else(|| DisplayLengthExpr::Variable(name.into())),
        );
    }

    if !value.is_cons() {
        return None;
    }

    let items = list_to_vec(&value)?;
    let head = items.first()?;
    if head.is_symbol_named("+") {
        return items[1..]
            .iter()
            .copied()
            .map(parse_display_length_expr)
            .collect::<Option<Vec<_>>>()
            .map(DisplayLengthExpr::Add);
    }
    if head.is_symbol_named("-") {
        return items[1..]
            .iter()
            .copied()
            .map(parse_display_length_expr)
            .collect::<Option<Vec<_>>>()
            .map(DisplayLengthExpr::Sub);
    }
    if items.len() == 1
        && let Some(number) = lisp_number(items[0])
    {
        return Some(DisplayLengthExpr::Pixels(number));
    }

    None
}

fn display_length_symbol(name: &str) -> Option<DisplayLengthSymbol> {
    match name {
        "height" => Some(DisplayLengthSymbol::Height),
        "width" => Some(DisplayLengthSymbol::Width),
        "text" => Some(DisplayLengthSymbol::Text),
        "left" => Some(DisplayLengthSymbol::Left),
        "right" => Some(DisplayLengthSymbol::Right),
        "center" => Some(DisplayLengthSymbol::Center),
        "left-fringe" => Some(DisplayLengthSymbol::LeftFringe),
        "right-fringe" => Some(DisplayLengthSymbol::RightFringe),
        "left-margin" => Some(DisplayLengthSymbol::LeftMargin),
        "right-margin" => Some(DisplayLengthSymbol::RightMargin),
        "scroll-bar" => Some(DisplayLengthSymbol::ScrollBar),
        _ => None,
    }
}

fn lisp_number(value: Value) -> Option<f32> {
    value
        .as_float()
        .or_else(|| value.as_fixnum().map(|number| number as f64))
        .filter(|number| number.is_finite())
        .map(|number| number as f32)
}

#[cfg(test)]
#[path = "display_source_test.rs"]
mod tests;

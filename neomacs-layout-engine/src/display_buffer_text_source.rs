use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayItemLayout, DisplayRowBreakReason, DisplaySourcePosition,
    DisplayTextRun, RenderFaceRef, SourceSpan,
};
use crate::display_source::{
    BufferTextSourceChar, BufferTextSourceRange, DisplayItemSource,
    DisplayPropertySourceCursorAction, DisplaySourceContext, LispStringSourceStack,
    TextSourceCharClassification, classify_text_source_char,
    display_item_kind_for_text_source_char, display_property_source_action,
};
use crate::neovm_bridge::{LayoutBufferView, RustBufferAccess};
use crate::types::{WindowKind, WindowParams};
use crate::unicode::decode_utf8;
use neovm_core::buffer::{BufferId, CharLen, CharPos0, EmacsBytePos, EmacsByteRange};
use neovm_core::emacs_core::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextWindowSource {
    window_start: i64,
    text_start_byte: usize,
    bytes_read: usize,
    point_charpos: i64,
    accessible_start: i64,
    accessible_end: i64,
    accessible_end_lisp_char: usize,
    accessible_end_emacs_byte: usize,
}

impl BufferTextWindowSource {
    pub(crate) const fn window_start(self) -> i64 {
        self.window_start
    }

    pub(crate) const fn text_start_byte(self) -> usize {
        self.text_start_byte
    }

    pub(crate) const fn bytes_read(self) -> usize {
        self.bytes_read
    }

    pub(crate) const fn point_charpos(self) -> i64 {
        self.point_charpos
    }

    pub(crate) const fn accessible_start(self) -> i64 {
        self.accessible_start
    }

    pub(crate) const fn accessible_end(self) -> i64 {
        self.accessible_end
    }

    pub(crate) const fn accessible_end_lisp_char(self) -> usize {
        self.accessible_end_lisp_char
    }

    pub(crate) const fn accessible_end_emacs_byte(self) -> usize {
        self.accessible_end_emacs_byte
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct BufferTextWindowSourceReadRequest<'a> {
    params: &'a WindowParams,
    max_rows: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextWindowSourceRequest {
    requested_window_start: i64,
    previous_window_end: Option<i64>,
    point_charpos: i64,
    accessible_start: i64,
    accessible_end: i64,
    max_rows: usize,
    visible_cols: i64,
    kind: WindowKind,
}

impl<'a> BufferTextWindowSourceReadRequest<'a> {
    pub(crate) fn new(params: &'a WindowParams, max_rows: usize) -> Self {
        Self { params, max_rows }
    }

    pub(crate) fn read_into<B: LayoutBufferView>(
        self,
        access: &RustBufferAccess<'_, B>,
        out: &mut Vec<u8>,
    ) -> BufferTextWindowSource {
        BufferTextWindowSourceRequest::from_window_params(self.params, self.max_rows)
            .read_into(access, out)
    }
}

impl BufferTextWindowSourceRequest {
    pub(crate) fn from_window_params(params: &WindowParams, max_rows: usize) -> Self {
        Self::new(
            params.window_start_charpos().get(),
            params.previous_window_end_charpos().map(|pos| pos.get()),
            params.point_charpos().get(),
            params.accessible_start_charpos().get(),
            params.accessible_end_charpos().get(),
            max_rows,
            visible_cols_for_window_params(params),
            params.kind,
        )
    }

    pub(crate) fn new(
        requested_window_start: i64,
        previous_window_end: Option<i64>,
        point_charpos: i64,
        accessible_start: i64,
        accessible_end: i64,
        max_rows: usize,
        visible_cols: i64,
        kind: WindowKind,
    ) -> Self {
        Self {
            requested_window_start,
            previous_window_end,
            point_charpos,
            accessible_start,
            accessible_end,
            max_rows,
            visible_cols: visible_cols.max(1),
            kind,
        }
    }

    pub(crate) fn read_into<B: LayoutBufferView>(
        self,
        access: &RustBufferAccess<'_, B>,
        out: &mut Vec<u8>,
    ) -> BufferTextWindowSource {
        let window_start =
            self.resolve_window_start(|charpos| access.byte_at(access.charpos_to_bytepos(charpos)));
        let text_start_byte = access.charpos_to_bytepos(window_start) as usize;
        let read_chars = self.accessible_end - window_start + 1;
        let bytes_read = if read_chars <= 0 {
            out.clear();
            0
        } else {
            let text_end = (window_start + read_chars).min(self.accessible_end);
            let byte_to = access.charpos_to_bytepos(text_end);
            access.copy_text(text_start_byte as i64, byte_to, out);
            out.len()
        };

        BufferTextWindowSource {
            window_start,
            text_start_byte,
            bytes_read,
            point_charpos: self.point_charpos,
            accessible_start: self.accessible_start,
            accessible_end: self.accessible_end,
            accessible_end_lisp_char: self.accessible_end.max(0) as usize + 1,
            accessible_end_emacs_byte: access.zv().max(0) as usize,
        }
    }

    fn resolve_window_start(self, byte_at_charpos: impl Fn(i64) -> Option<u8>) -> i64 {
        let mut window_start = self.requested_window_start.max(self.accessible_start);

        if window_start > self.accessible_start {
            let remaining_chars = self.accessible_end - window_start;
            if remaining_chars < self.max_rows as i64 && self.accessible_end > self.max_rows as i64
            {
                window_start =
                    self.scan_back_from_point((self.max_rows / 2).max(1), &byte_at_charpos);
            }
        }

        if self.point_charpos >= self.accessible_start && self.point_charpos < window_start {
            let adjusted = self.scan_back_from_point((self.max_rows / 4).max(1), &byte_at_charpos);
            tracing::debug!(
                "layout_window_rust: adjusted window_start {} -> {} (point={})",
                self.requested_window_start,
                adjusted,
                self.point_charpos
            );
            return adjusted;
        }

        if self.should_forward_scroll_without_layout(window_start) {
            let adjusted =
                self.scan_back_from_point(((self.max_rows * 3) / 4).max(1), &byte_at_charpos);
            tracing::debug!(
                "layout_window_rust: forward-adjusted window_start {} -> {} (point={}, prev_end={})",
                self.requested_window_start,
                adjusted,
                self.point_charpos,
                self.previous_window_end.unwrap_or(0)
            );
            return adjusted;
        }

        window_start
    }

    fn should_forward_scroll_without_layout(self, window_start: i64) -> bool {
        if self.point_charpos <= 0 || self.kind.is_minibuffer() {
            return false;
        }
        let has_prev_end = self
            .previous_window_end
            .is_some_and(|end| self.point_charpos > end);
        let max_visible_chars = (self.max_rows.max(1) as i64) * self.visible_cols;
        let far_below_without_prev_end = self.previous_window_end.is_none()
            && self.point_charpos - window_start > max_visible_chars;
        has_prev_end || far_below_without_prev_end
    }

    fn scan_back_from_point(
        self,
        target_rows_above: usize,
        byte_at_charpos: &impl Fn(i64) -> Option<u8>,
    ) -> i64 {
        let mut lines_back = 0usize;
        let mut scan_pos = self.point_charpos.max(self.accessible_start);
        while scan_pos > self.accessible_start && lines_back < target_rows_above {
            scan_pos -= 1;
            if byte_at_charpos(scan_pos) == Some(b'\n') {
                lines_back += 1;
            }
        }
        scan_pos.max(self.accessible_start)
    }
}

fn visible_cols_for_window_params(params: &WindowParams) -> i64 {
    let char_width = params.char_width.max(1.0);
    (params.text_bounds.width.max(1.0) / char_width)
        .floor()
        .max(1.0) as i64
}

/// A single character decoded from raw buffer text, together with its byte and
/// char positions in the original buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextDecodedSourceChar {
    ch: char,
    start_byte_idx: usize,
    start_charpos: i64,
}

impl BufferTextDecodedSourceChar {
    pub(crate) const fn new(ch: char, start_byte_idx: usize, start_charpos: i64) -> Self {
        Self {
            ch,
            start_byte_idx,
            start_charpos,
        }
    }

    pub(crate) fn ch(self) -> char {
        self.ch
    }

    pub(crate) fn start_byte_idx(self) -> usize {
        self.start_byte_idx
    }

    pub(crate) fn start_charpos(self) -> i64 {
        self.start_charpos
    }

    pub(crate) fn source_range(self) -> BufferTextSourceRange {
        BufferTextSourceRange::single_char(CharPos0::new(self.start_charpos as usize))
    }

    pub(crate) fn source_char(self, nobreak_display_policy: i32) -> BufferTextSourceChar {
        BufferTextSourceChar::new(self.ch, self.source_range().start(), nobreak_display_policy)
    }
}

/// A decoded event from a buffer text source: either a printable character/text
/// run or a line-break character.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BufferTextDecodedSourceEvent {
    LineBreak(BufferTextLineBreakSourceEvent),
    Text(BufferTextSourceTextEvent),
}

impl BufferTextDecodedSourceEvent {
    pub(crate) fn decoded_char(&self) -> BufferTextDecodedSourceChar {
        match self {
            Self::LineBreak(source_event) => (*source_event).decoded_char(),
            Self::Text(source_event) => source_event.decoded_char(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextLineBreakSourceEvent {
    source_char: BufferTextDecodedSourceChar,
}

impl BufferTextLineBreakSourceEvent {
    pub(crate) fn new(source_char: BufferTextDecodedSourceChar) -> Self {
        debug_assert_eq!(source_char.ch(), '\n');
        Self { source_char }
    }

    pub(crate) fn decoded_char(self) -> BufferTextDecodedSourceChar {
        self.source_char
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSourceTextEvent {
    source_char: BufferTextDecodedSourceChar,
    source_item: Option<DisplayItem>,
}

impl BufferTextSourceTextEvent {
    pub(crate) fn new(source_char: BufferTextDecodedSourceChar) -> Self {
        debug_assert_ne!(source_char.ch(), '\n');
        Self {
            source_char,
            source_item: None,
        }
    }

    pub(crate) fn from_source_item(
        source_char: BufferTextDecodedSourceChar,
        source_item: DisplayItem,
    ) -> Self {
        debug_assert_ne!(source_char.ch(), '\n');
        Self {
            source_char,
            source_item: Some(source_item),
        }
    }

    pub(crate) fn decoded_char(&self) -> BufferTextDecodedSourceChar {
        self.source_char
    }

    pub(crate) fn source_item(&self) -> Option<&DisplayItem> {
        self.source_item.as_ref()
    }

    pub(crate) fn source_char(&self, nobreak_display_policy: i32) -> BufferTextSourceChar {
        self.source_char.source_char(nobreak_display_policy)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextSourceEventAdapter {
    text_start_byte: usize,
}

impl BufferTextSourceEventAdapter {
    pub(crate) fn new(text_start_byte: usize) -> Self {
        Self { text_start_byte }
    }

    pub(crate) fn event_from_item(
        self,
        item: DisplayItem,
        byte_idx: &mut usize,
        charpos: i64,
    ) -> Option<BufferTextDecodedSourceEvent> {
        let buffer_byte_pos = match item.span.start {
            DisplaySourcePosition::Buffer { byte_pos, .. } => byte_pos,
            _ => {
                tracing::error!(
                    "BufferTextSourceEventAdapter: typed cursor yielded a non-buffer-span item; \
                     a display property escaped the render_next_step checkpoints"
                );
                return None;
            }
        };
        let start_byte_idx = buffer_byte_pos.get().checked_sub(self.text_start_byte)?;
        if start_byte_idx != *byte_idx {
            tracing::error!(
                "BufferTextSourceEventAdapter: typed cursor byte position {} did not match \
                 buffer walk byte index {}",
                start_byte_idx,
                *byte_idx
            );
            return None;
        }

        let DisplayItem {
            span,
            face,
            kind,
            layout,
        } = item;
        match kind {
            DisplayItemKind::TextRun(run) => {
                let ch = run.text.chars().next()?;
                *byte_idx = start_byte_idx + ch.len_utf8();
                let source_item = DisplayItem {
                    span: first_char_span(span, ch),
                    face,
                    kind: DisplayItemKind::TextRun(DisplayTextRun::new(ch.to_string())),
                    layout,
                };
                Some(BufferTextDecodedSourceEvent::Text(
                    text_event_for_source_item(
                        BufferTextDecodedSourceChar::new(ch, start_byte_idx, charpos),
                        source_item,
                    ),
                ))
            }
            DisplayItemKind::RowBreak(row_break)
                if row_break.reason == DisplayRowBreakReason::ExplicitNewline =>
            {
                *byte_idx = start_byte_idx + 1;
                Some(BufferTextDecodedSourceEvent::LineBreak(
                    BufferTextLineBreakSourceEvent::new(BufferTextDecodedSourceChar::new(
                        '\n',
                        start_byte_idx,
                        charpos,
                    )),
                ))
            }
            DisplayItemKind::ControlChar { ch } => {
                *byte_idx = start_byte_idx + ch.len_utf8();
                let source_item = DisplayItem {
                    span,
                    face,
                    kind: DisplayItemKind::ControlChar { ch },
                    layout,
                };
                Some(BufferTextDecodedSourceEvent::Text(
                    text_event_for_source_item(
                        BufferTextDecodedSourceChar::new(ch, start_byte_idx, charpos),
                        source_item,
                    ),
                ))
            }
            DisplayItemKind::Glyphless(glyphless) => {
                let ch = glyphless.ch;
                *byte_idx = start_byte_idx + ch.len_utf8();
                let source_item = DisplayItem {
                    span,
                    face,
                    kind: DisplayItemKind::Glyphless(glyphless),
                    layout,
                };
                Some(BufferTextDecodedSourceEvent::Text(
                    text_event_for_source_item(
                        BufferTextDecodedSourceChar::new(ch, start_byte_idx, charpos),
                        source_item,
                    ),
                ))
            }
            _ => {
                tracing::error!(
                    "BufferTextSourceEventAdapter: typed cursor yielded a non-text item kind; \
                     a display property escaped the render_next_step checkpoints"
                );
                None
            }
        }
    }
}

fn text_event_for_source_item(
    source_char: BufferTextDecodedSourceChar,
    source_item: DisplayItem,
) -> BufferTextSourceTextEvent {
    if source_item.layout == DisplayItemLayout::default() {
        BufferTextSourceTextEvent::from_source_item(source_char, source_item)
    } else {
        BufferTextSourceTextEvent::new(source_char)
    }
}

fn first_char_span(span: SourceSpan, ch: char) -> SourceSpan {
    let end = match &span.start {
        DisplaySourcePosition::Buffer {
            buffer_id,
            char_pos,
            byte_pos,
        } => DisplaySourcePosition::buffer(
            *buffer_id,
            char_pos.add_len(CharLen::new(1)),
            EmacsBytePos::new(byte_pos.get().saturating_add(ch.len_utf8())),
        ),
        DisplaySourcePosition::LispString {
            source_id,
            char_index,
            byte_index,
        } => DisplaySourcePosition::lisp_string(
            source_id.get(),
            char_index.saturating_add(1),
            byte_index.saturating_add(ch.len_utf8()),
        ),
        DisplaySourcePosition::Synthetic { source_id, offset } => {
            DisplaySourcePosition::synthetic(source_id.get(), offset.saturating_add(ch.len_utf8()))
        }
    };
    SourceSpan::new(span.start, end)
}

/// A `DisplayItemSource` that reads plain buffer text (with face and display
/// property boundaries) and emits `DisplayItem` values for the shared row
/// renderer. This is the new-path buffer source; the old monolithic walker in
/// `display_buffer_text_walk.rs` should eventually route through it.
#[allow(dead_code)]
pub(crate) struct BufferTextSourceCursor<'a, B: LayoutBufferView + ?Sized> {
    buffer_id: BufferId,
    buffer: &'a B,
    char_pos: CharPos0,
    end: CharPos0,
    base_face: RenderFaceRef,
    replacement_strings: LispStringSourceStack,
}

#[allow(dead_code)]
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

    pub(crate) fn current_char_pos(&self) -> CharPos0 {
        self.char_pos
    }

    pub(crate) fn current_byte_pos(&self) -> EmacsBytePos {
        self.byte_pos(self.char_pos)
    }

    pub(crate) fn reset_to(&mut self, char_pos: CharPos0) {
        let accessible_end = self.buffer.layout_point_max_char_pos();
        self.char_pos = char_pos.min(self.end).min(accessible_end);
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
            if classify_text_source_char(ch) != TextSourceCharClassification::Text {
                break;
            }
            end = end.add_len(CharLen::new(1));
        }
        end.max(start.add_len(CharLen::new(1))).min(limit)
    }

    pub(crate) fn source_position(&self) -> DisplaySourcePosition {
        if !self.replacement_strings.is_empty() {
            return self.replacement_strings.source_position();
        }
        DisplaySourcePosition::buffer(self.buffer_id, self.char_pos, self.byte_pos(self.char_pos))
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
                let item_layout = match display_property_source_action(context, display_prop, face)
                    .into_cursor_action(span, face)
                {
                    DisplayPropertySourceCursorAction::PushReplacement { value, base_face } => {
                        self.replacement_strings.push(value, base_face);
                        continue;
                    }
                    DisplayPropertySourceCursorAction::Emit(item) => {
                        return Some(item);
                    }
                    DisplayPropertySourceCursorAction::FallThrough { layout } => layout,
                };
                let ch = self.char_at(start)?;
                if let Some(kind) = display_item_kind_for_text_source_char(ch) {
                    self.char_pos = start.add_len(CharLen::new(1));
                    return Some(
                        DisplayItem::new(self.span(start, self.char_pos), face, kind)
                            .with_layout(item_layout),
                    );
                }

                let end = self.next_text_run_end(start, property_end);
                self.char_pos = end;
                return Some(
                    DisplayItem::new(
                        self.span(start, end),
                        face,
                        DisplayItemKind::TextRun(DisplayTextRun::new(self.text_slice(start, end))),
                    )
                    .with_layout(item_layout),
                );
            }

            let ch = self.char_at(start)?;
            if let Some(kind) = display_item_kind_for_text_source_char(ch) {
                self.char_pos = start.add_len(CharLen::new(1));
                return Some(DisplayItem::new(
                    self.span(start, self.char_pos),
                    face,
                    kind,
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
}

#[cfg(test)]
#[path = "display_buffer_text_source_test.rs"]
mod tests;

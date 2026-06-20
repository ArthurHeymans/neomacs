use crate::display_buffer_display_property_source::BufferTextReplacementItem;
use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayItemLayout, DisplayRowBreakReason, DisplaySourcePosition,
    DisplayTextRun, DisplayTextRunItemCursor, RenderFaceRef, SourceSpan,
};
use crate::display_property::DisplayPropertyClassification;
use crate::display_source::{
    BufferDisplayReplacementSource, BufferTextSourceChar, BufferTextSourceRange, DisplayItemSource,
    DisplayPropertySourceCursorAction, DisplayPropertySourcePlan, DisplaySourceContext,
    LispStringSourceStack, TextSourceCharClassification, classify_text_source_char,
    display_item_kind_for_text_source_char,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextSourcePosition {
    byte_idx: usize,
    charpos: i64,
}

impl BufferTextSourcePosition {
    pub(crate) const fn new(byte_idx: usize, charpos: i64) -> Self {
        Self { byte_idx, charpos }
    }

    pub(crate) const fn byte_idx(self) -> usize {
        self.byte_idx
    }

    pub(crate) const fn charpos(self) -> i64 {
        self.charpos
    }

    pub(crate) const fn with_charpos(self, charpos: i64) -> Self {
        Self {
            byte_idx: self.byte_idx,
            charpos,
        }
    }

    fn advance_byte_idx_to(&mut self, byte_idx: usize) {
        self.byte_idx = byte_idx;
    }

    pub(crate) fn advance_charpos_by_one(&mut self) {
        self.charpos = self.charpos.saturating_add(1);
    }

    fn advance_one_char(&mut self, ch_len: usize) {
        self.byte_idx = self.byte_idx.saturating_add(ch_len);
        self.charpos = self.charpos.saturating_add(1);
    }

    fn matches(self, byte_idx: usize, charpos: i64) -> bool {
        self.byte_idx == byte_idx && self.charpos == charpos
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BufferTextSourceAlignmentRequest {
    text_start_byte: usize,
    position: BufferTextSourcePosition,
    source_char: Option<char>,
}

impl BufferTextSourceAlignmentRequest {
    fn new(
        text_start_byte: usize,
        position: BufferTextSourcePosition,
        source_char: Option<char>,
    ) -> Self {
        Self {
            text_start_byte,
            position,
            source_char,
        }
    }

    fn for_position(text_start_byte: usize, position: BufferTextSourcePosition) -> Self {
        Self::new(text_start_byte, position, None)
    }

    fn align_display_item(self, item: DisplayItem) -> Option<BufferTextSourceItem> {
        let DisplaySourcePosition::Buffer { byte_pos, .. } = item.span.start else {
            tracing::error!(
                "BufferTextConsumedItemAdapter: typed cursor yielded a non-buffer-span item; \
                 a display property escaped the render_next_step checkpoints"
            );
            return None;
        };
        let start_byte_idx = byte_pos.get().checked_sub(self.text_start_byte)?;
        if start_byte_idx != self.position.byte_idx() {
            tracing::error!(
                "BufferTextConsumedItemAdapter: typed cursor byte position {} did not match \
                 buffer walk byte index {}",
                start_byte_idx,
                self.position.byte_idx()
            );
            return None;
        }
        let DisplaySourcePosition::Buffer { char_pos, .. } = item.span.start else {
            unreachable!("buffer byte position match implies buffer source position");
        };
        let start_charpos = char_pos.get() as i64;
        if start_charpos != self.position.charpos() {
            tracing::error!(
                "BufferTextConsumedItemAdapter: typed cursor char position {} did not match \
                 buffer walk char position {}",
                start_charpos,
                self.position.charpos()
            );
            return None;
        }
        Some(BufferTextSourceItem::new(
            item,
            start_byte_idx,
            start_charpos,
            self.source_char,
        ))
    }

    fn replacement_matches(self, item: &BufferTextReplacementItem) -> Option<bool> {
        let anchor = item.source_anchor(self.text_start_byte)?;
        Some(anchor.matches(self.position.byte_idx(), self.position.charpos()))
    }

    fn split_text_item_start(self, item: &DisplayItem) -> Option<(usize, i64)> {
        let DisplaySourcePosition::Buffer {
            char_pos, byte_pos, ..
        } = &item.span.start
        else {
            tracing::error!(
                "BufferTextConsumedItemAdapter: split text run yielded a non-buffer-span item"
            );
            return None;
        };
        let start_byte_idx = byte_pos.get().checked_sub(self.text_start_byte)?;
        let start_charpos = char_pos.get() as i64;
        if !self.position.matches(start_byte_idx, start_charpos) {
            tracing::debug!(
                "BufferTextConsumedItemAdapter: split text run at byte {} charpos {} did not \
                 match buffer walk byte {} charpos {}",
                start_byte_idx,
                start_charpos,
                self.position.byte_idx(),
                self.position.charpos()
            );
            return None;
        }
        Some((start_byte_idx, start_charpos))
    }

    fn split_text_item_end_byte_idx(self, item: &DisplayItem) -> Option<usize> {
        let DisplaySourcePosition::Buffer {
            byte_pos: end_byte_pos,
            ..
        } = &item.span.end
        else {
            tracing::error!(
                "BufferTextConsumedItemAdapter: split text run yielded a non-buffer end span"
            );
            return None;
        };
        end_byte_pos.get().checked_sub(self.text_start_byte)
    }
}

/// A single source character aligned with the current buffer byte and char
/// positions for the row walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextSourceStepChar {
    ch: char,
    start_byte_idx: usize,
    start_charpos: i64,
}

impl BufferTextSourceStepChar {
    pub(crate) const fn new(ch: char, start_byte_idx: usize, start_charpos: i64) -> Self {
        Self {
            ch,
            start_byte_idx,
            start_charpos,
        }
    }

    pub(crate) fn consume_from_position(
        text: &[u8],
        position: &mut BufferTextSourcePosition,
    ) -> Option<Self> {
        if position.byte_idx() >= text.len() {
            return None;
        }
        let start_byte_idx = position.byte_idx();
        let start_charpos = position.charpos();
        let (ch, ch_len) = decode_utf8(&text[start_byte_idx..]);
        if ch_len == 0 {
            return None;
        }
        position.advance_one_char(ch_len);
        Some(Self::new(ch, start_byte_idx, start_charpos))
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

/// A typed display item consumed by the buffer text row walk after it has been
/// aligned with the current buffer byte/char cursor.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextConsumedDisplayItem {
    source_char: BufferTextSourceStepChar,
    item: DisplayItem,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSourceItem {
    item: DisplayItem,
    start_byte_idx: usize,
    start_charpos: i64,
    source_char: Option<char>,
}

#[derive(Clone, Debug, PartialEq)]
enum BufferTextSourceCursorItem {
    Item(DisplayItem),
    Replacement(BufferTextReplacementItem),
}

#[derive(Clone, Debug, PartialEq)]
enum BufferTextAlignedSourceCursorItem {
    Item(BufferTextSourceItem),
    Replacement(BufferTextReplacementItem),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BufferTextDisplayReplacementMode {
    InlineSourceItems,
    ConsumedSourceItem,
}

impl BufferTextDisplayReplacementMode {
    fn consumes_typed_replacements(self) -> bool {
        matches!(self, Self::ConsumedSourceItem)
    }

    fn inlines_replacement_strings(self) -> bool {
        matches!(self, Self::InlineSourceItems)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BufferTextSourceCursorReadRequest {
    text_start_byte: usize,
    position: BufferTextSourcePosition,
}

impl BufferTextSourceCursorReadRequest {
    fn new(text_start_byte: usize, position: BufferTextSourcePosition) -> Self {
        Self {
            text_start_byte,
            position,
        }
    }

    fn expected_source_pos(self) -> CharPos0 {
        CharPos0::new(self.position.charpos().max(0) as usize)
    }

    fn alignment(self, source_char: Option<char>) -> BufferTextSourceAlignmentRequest {
        BufferTextSourceAlignmentRequest::new(self.text_start_byte, self.position, source_char)
    }

    fn replacement_alignment(self) -> BufferTextSourceAlignmentRequest {
        BufferTextSourceAlignmentRequest::for_position(self.text_start_byte, self.position)
    }

    fn read<B: LayoutBufferView + ?Sized>(
        self,
        source: &mut BufferTextSourceCursor<'_, B>,
        context: &mut DisplaySourceContext<'_>,
        replacement_mode: BufferTextDisplayReplacementMode,
    ) -> Option<BufferTextAlignedSourceCursorItem> {
        let expected_source_pos = self.expected_source_pos();
        if source.current_char_pos() != expected_source_pos {
            source.reset_to(expected_source_pos);
        }

        let source_char = source.char_at(expected_source_pos);
        match source.next_cursor_item(context, replacement_mode)? {
            BufferTextSourceCursorItem::Item(item) => self
                .alignment(source_char)
                .align_display_item(item)
                .map(BufferTextAlignedSourceCursorItem::Item),
            BufferTextSourceCursorItem::Replacement(item) => {
                if !self.replacement_alignment().replacement_matches(&item)? {
                    tracing::error!(
                        "BufferTextConsumedItemAdapter: display replacement did not match \
                         buffer walk byte {} charpos {}",
                        self.position.byte_idx(),
                        self.position.charpos()
                    );
                    return None;
                }
                Some(BufferTextAlignedSourceCursorItem::Replacement(item))
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BufferTextConsumedSourceItem {
    DisplayItem(BufferTextConsumedDisplayItem),
    Replacement(BufferTextReplacementItem),
}

impl BufferTextConsumedDisplayItem {
    pub(crate) fn new(source_char: BufferTextSourceStepChar, item: DisplayItem) -> Self {
        Self { source_char, item }
    }

    pub(crate) fn source_char(&self) -> BufferTextSourceStepChar {
        self.source_char
    }

    pub(crate) fn is_explicit_line_break(&self) -> bool {
        matches!(
            self.item.kind,
            DisplayItemKind::RowBreak(row_break)
                if row_break.reason == DisplayRowBreakReason::ExplicitNewline
        )
    }

    pub(crate) fn end_charpos(&self) -> i64 {
        display_item_buffer_end_charpos(&self.item)
            .unwrap_or_else(|| self.source_char.start_charpos().saturating_add(1))
    }

    pub(crate) fn into_parts(self) -> (BufferTextSourceStepChar, DisplayItem) {
        (self.source_char, self.item)
    }
}

fn display_item_buffer_end_charpos(item: &DisplayItem) -> Option<i64> {
    item.span
        .buffer_end_charpos()
        .map(|char_pos| char_pos.get() as i64)
}

fn display_item_buffer_byte_len(item: &DisplayItem) -> Option<usize> {
    item.span.buffer_byte_len()
}

impl BufferTextSourceItem {
    pub(crate) fn new(
        item: DisplayItem,
        start_byte_idx: usize,
        start_charpos: i64,
        source_char: Option<char>,
    ) -> Self {
        Self {
            item,
            start_byte_idx,
            start_charpos,
            source_char,
        }
    }

    #[cfg(test)]
    pub(crate) fn new_for_test(
        item: DisplayItem,
        start_byte_idx: usize,
        start_charpos: i64,
        source_char: Option<char>,
    ) -> Self {
        Self::new(item, start_byte_idx, start_charpos, source_char)
    }

    fn direct_source_char(&self) -> Option<char> {
        match &self.item.kind {
            DisplayItemKind::TextRun(run) => {
                let mut chars = run.text.chars();
                let ch = chars.next()?;
                chars.next().is_none().then_some(ch)
            }
            DisplayItemKind::RowBreak(row_break)
                if row_break.reason == DisplayRowBreakReason::ExplicitNewline =>
            {
                Some('\n')
            }
            DisplayItemKind::ControlChar { ch } => Some(*ch),
            DisplayItemKind::Glyphless(glyphless) => Some(glyphless.ch),
            DisplayItemKind::SourceMappedText(_) => self.source_char,
            _ => None,
        }
    }

    pub(crate) fn try_into_direct_consumed_display_item(
        self,
        position: &mut BufferTextSourcePosition,
    ) -> Result<BufferTextConsumedDisplayItem, Self> {
        if !position.matches(self.start_byte_idx, self.start_charpos) {
            tracing::error!(
                "BufferTextSourceItem: validated source item at byte {} charpos {} \
                 did not match buffer walk byte {} charpos {}",
                self.start_byte_idx,
                self.start_charpos,
                position.byte_idx(),
                position.charpos()
            );
            return Err(self);
        }
        let Some(ch) = self.direct_source_char() else {
            return Err(self);
        };
        let start_byte_idx = self.start_byte_idx;
        let start_charpos = self.start_charpos;
        let byte_len = display_item_buffer_byte_len(&self.item).unwrap_or_else(|| ch.len_utf8());
        position.advance_byte_idx_to(start_byte_idx.saturating_add(byte_len));
        Ok(BufferTextConsumedDisplayItem::new(
            BufferTextSourceStepChar::new(ch, start_byte_idx, start_charpos),
            self.item,
        ))
    }

    pub(crate) fn start_byte_idx(&self) -> usize {
        self.start_byte_idx
    }

    pub(crate) fn start_charpos(&self) -> i64 {
        self.start_charpos
    }

    #[cfg(test)]
    pub(crate) fn item(&self) -> &DisplayItem {
        &self.item
    }

    pub(crate) fn into_item(self) -> DisplayItem {
        self.item
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextConsumedItemAdapter {
    text_start_byte: usize,
    pending_text_run: Option<DisplayTextRunItemCursor>,
}

impl BufferTextConsumedItemAdapter {
    pub(crate) fn new(text_start_byte: usize) -> Self {
        Self {
            text_start_byte,
            pending_text_run: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn next_display_item_from_source<B: LayoutBufferView + ?Sized>(
        &mut self,
        source: &mut BufferTextSourceCursor<'_, B>,
        context: &mut DisplaySourceContext<'_>,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextConsumedDisplayItem> {
        if let Some(step) = self.next_pending_display_item(position) {
            return Some(step);
        }

        let item = self.next_item_from_source(source, context, position)?;
        self.consume_aligned_display_item(item, position)
    }

    fn next_pending_display_item(
        &mut self,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextConsumedDisplayItem> {
        self.next_pending_step(position)
    }

    #[cfg(test)]
    pub(crate) fn next_item_from_source<B: LayoutBufferView + ?Sized>(
        &mut self,
        source: &mut BufferTextSourceCursor<'_, B>,
        context: &mut DisplaySourceContext<'_>,
        position: &BufferTextSourcePosition,
    ) -> Option<BufferTextSourceItem> {
        if self.pending_text_run.is_some() {
            tracing::debug!(
                "BufferTextConsumedItemAdapter: requested typed item while a text run is pending"
            );
            return None;
        }

        match BufferTextSourceCursorReadRequest::new(self.text_start_byte, *position).read(
            source,
            context,
            BufferTextDisplayReplacementMode::InlineSourceItems,
        )? {
            BufferTextAlignedSourceCursorItem::Item(item) => Some(item),
            BufferTextAlignedSourceCursorItem::Replacement(_) => {
                debug_assert!(false, "inline source cursor surfaced a buffer replacement");
                None
            }
        }
    }

    pub(crate) fn next_consumed_source_item<B: LayoutBufferView + ?Sized>(
        &mut self,
        source: &mut BufferTextSourceCursor<'_, B>,
        context: &mut DisplaySourceContext<'_>,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextConsumedSourceItem> {
        if let Some(step) = self.next_pending_display_item(position) {
            return Some(BufferTextConsumedSourceItem::DisplayItem(step));
        }

        if self.pending_text_run.is_some() {
            tracing::debug!(
                "BufferTextConsumedItemAdapter: requested typed item while a text run is pending"
            );
            return None;
        }

        match BufferTextSourceCursorReadRequest::new(self.text_start_byte, *position).read(
            source,
            context,
            BufferTextDisplayReplacementMode::ConsumedSourceItem,
        )? {
            BufferTextAlignedSourceCursorItem::Item(item) => self
                .consume_aligned_display_item(item, position)
                .map(BufferTextConsumedSourceItem::DisplayItem),
            BufferTextAlignedSourceCursorItem::Replacement(item) => {
                Some(BufferTextConsumedSourceItem::Replacement(item))
            }
        }
    }

    pub(crate) fn consume_fallback_source_item(
        &mut self,
        item: BufferTextSourceItem,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextConsumedDisplayItem> {
        self.consume_aligned_display_item(item, position)
    }

    #[cfg(test)]
    pub(crate) fn consumed_display_item_from_item(
        &mut self,
        item: DisplayItem,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextConsumedDisplayItem> {
        let item = BufferTextSourceAlignmentRequest::for_position(self.text_start_byte, *position)
            .align_display_item(item)?;
        self.consume_aligned_display_item(item, position)
    }

    #[cfg(test)]
    pub(crate) fn consumed_display_item_from_source_item(
        &mut self,
        item: BufferTextSourceItem,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextConsumedDisplayItem> {
        self.consume_aligned_display_item(item, position)
    }

    fn consume_aligned_display_item(
        &mut self,
        item: BufferTextSourceItem,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextConsumedDisplayItem> {
        if !position.matches(item.start_byte_idx(), item.start_charpos()) {
            tracing::error!(
                "BufferTextConsumedItemAdapter: validated source item at byte {} charpos {} \
                 did not match buffer walk byte {} charpos {}",
                item.start_byte_idx(),
                item.start_charpos(),
                position.byte_idx(),
                position.charpos()
            );
            return None;
        }
        let item = match item.try_into_direct_consumed_display_item(position) {
            Ok(step) => return Some(step),
            Err(item) => item,
        };
        self.split_text_run_item(item, position)
    }

    fn split_text_run_item(
        &mut self,
        item: BufferTextSourceItem,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextConsumedDisplayItem> {
        match DisplayTextRunItemCursor::from_item(item.into_item()) {
            Ok(cursor) => {
                self.pending_text_run = Some(cursor);
                self.next_pending_step(position)
            }
            Err(_) => {
                tracing::error!(
                    "BufferTextConsumedItemAdapter: typed cursor yielded a non-text item kind; \
                     a direct item escaped source-item lowering"
                );
                None
            }
        }
    }

    fn next_pending_step(
        &mut self,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextConsumedDisplayItem> {
        let text_start_byte = self.text_start_byte;
        let pending = self.pending_text_run.as_mut()?;
        let item = pending.next_item();
        let finished = pending.is_finished();
        let step = item.and_then(|item| {
            Self::consumed_display_item_from_split_text_item(text_start_byte, item, position)
        });
        if finished || step.is_none() {
            self.pending_text_run = None;
        }
        step
    }

    fn consumed_display_item_from_split_text_item(
        text_start_byte: usize,
        item: DisplayItem,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextConsumedDisplayItem> {
        let alignment = BufferTextSourceAlignmentRequest::for_position(text_start_byte, *position);
        let (start_byte_idx, start_charpos) = alignment.split_text_item_start(&item)?;
        let ch = match &item.kind {
            DisplayItemKind::TextRun(run) => {
                let mut chars = run.text.chars();
                let ch = chars.next()?;
                if chars.next().is_some() {
                    tracing::error!(
                        "BufferTextConsumedItemAdapter: split text run yielded multiple chars"
                    );
                    return None;
                }
                ch
            }
            _ => {
                tracing::error!(
                    "BufferTextConsumedItemAdapter: split text run yielded non-text item"
                );
                return None;
            }
        };
        let end_byte_idx = alignment.split_text_item_end_byte_idx(&item)?;
        position.advance_byte_idx_to(end_byte_idx);
        Some(BufferTextConsumedDisplayItem::new(
            BufferTextSourceStepChar::new(ch, start_byte_idx, start_charpos),
            item,
        ))
    }
}

/// A `DisplayItemSource` that reads plain buffer text (with face and display
/// property boundaries) and emits `DisplayItem` values for the shared row
/// renderer. The main buffer walk consumes this cursor through
/// `BufferTextConsumedItemAdapter`, which preserves typed display items while
/// splitting text runs only where the remaining buffer walk still needs
/// per-character wrap/cursor decisions.
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

    pub(crate) fn current_char_pos(&self) -> CharPos0 {
        self.char_pos
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

    fn display_replacement_source(
        &self,
        start: CharPos0,
        end: CharPos0,
    ) -> BufferDisplayReplacementSource {
        BufferDisplayReplacementSource::spanning(
            self.buffer_id,
            start,
            self.byte_pos(start),
            end,
            self.byte_pos(end),
        )
    }

    fn display_replacement_item(
        &self,
        value: Value,
        classification: DisplayPropertyClassification,
        start: CharPos0,
        end: CharPos0,
    ) -> BufferTextReplacementItem {
        BufferTextReplacementItem::new(
            value,
            classification,
            self.display_replacement_source(start, end),
            self.byte_pos(start),
            self.byte_pos(end),
            start,
            end,
        )
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

    fn display_property_cursor_action(
        &self,
        context: &mut DisplaySourceContext<'_>,
        display_property: &DisplayPropertySourcePlan,
        face: RenderFaceRef,
        span: SourceSpan,
    ) -> DisplayPropertySourceCursorAction {
        display_property.cursor_action(context, span, face)
    }

    fn push_display_replacement_string(
        &mut self,
        value: Value,
        base_face: RenderFaceRef,
        start: CharPos0,
        end: CharPos0,
    ) {
        self.replacement_strings.push_with_replacement_source(
            value,
            base_face,
            Some(self.display_replacement_source(start, end)),
        );
    }

    fn next_text_item_with_layout(
        &mut self,
        start: CharPos0,
        property_end: CharPos0,
        face: RenderFaceRef,
        layout: DisplayItemLayout,
    ) -> Option<DisplayItem> {
        let ch = self.char_at(start)?;
        if let Some(kind) = display_item_kind_for_text_source_char(ch) {
            self.char_pos = start.add_len(CharLen::new(1));
            return Some(
                DisplayItem::new(self.span(start, self.char_pos), face, kind).with_layout(layout),
            );
        }

        let end = self.next_text_run_end(start, property_end);
        self.char_pos = end;
        Some(
            DisplayItem::new(
                self.span(start, end),
                face,
                DisplayItemKind::TextRun(DisplayTextRun::new(self.text_slice(start, end))),
            )
            .with_layout(layout),
        )
    }

    fn next_text_item(
        &mut self,
        start: CharPos0,
        property_end: CharPos0,
        face: RenderFaceRef,
    ) -> Option<DisplayItem> {
        self.next_text_item_with_layout(start, property_end, face, DisplayItemLayout::default())
    }

    #[cfg(test)]
    pub(crate) fn source_position(&self) -> DisplaySourcePosition {
        if !self.replacement_strings.is_empty() {
            return self.replacement_strings.source_position();
        }
        DisplaySourcePosition::buffer(self.buffer_id, self.char_pos, self.byte_pos(self.char_pos))
    }

    fn next_cursor_item(
        &mut self,
        context: &mut DisplaySourceContext<'_>,
        replacement_mode: BufferTextDisplayReplacementMode,
    ) -> Option<BufferTextSourceCursorItem> {
        loop {
            if let Some(item) = self.replacement_strings.next_item(context) {
                return Some(BufferTextSourceCursorItem::Item(item));
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
                let display_property = DisplayPropertySourcePlan::new(display_prop);
                if replacement_mode.consumes_typed_replacements()
                    && display_property.replacement().is_some()
                {
                    return Some(BufferTextSourceCursorItem::Replacement(
                        self.display_replacement_item(
                            display_prop,
                            display_property.into_classification(),
                            start,
                            property_end,
                        ),
                    ));
                }
                let item_layout = match self.display_property_cursor_action(
                    context,
                    &display_property,
                    face,
                    span,
                ) {
                    DisplayPropertySourceCursorAction::PushReplacement { value, base_face } => {
                        if replacement_mode.inlines_replacement_strings() {
                            self.push_display_replacement_string(
                                value,
                                base_face,
                                start,
                                property_end,
                            );
                            continue;
                        }
                        return Some(BufferTextSourceCursorItem::Replacement(
                            self.display_replacement_item(
                                value,
                                display_property.into_classification(),
                                start,
                                property_end,
                            ),
                        ));
                    }
                    DisplayPropertySourceCursorAction::Emit(item) => {
                        return Some(BufferTextSourceCursorItem::Item(item));
                    }
                    DisplayPropertySourceCursorAction::FallThrough { layout } => layout,
                };
                return Some(BufferTextSourceCursorItem::Item(
                    self.next_text_item_with_layout(start, property_end, face, item_layout)?,
                ));
            }

            return Some(BufferTextSourceCursorItem::Item(self.next_text_item(
                start,
                property_end,
                face,
            )?));
        }
    }
}

impl<B: LayoutBufferView + ?Sized> DisplayItemSource for BufferTextSourceCursor<'_, B> {
    fn next_item(&mut self, context: &mut DisplaySourceContext<'_>) -> Option<DisplayItem> {
        match self.next_cursor_item(context, BufferTextDisplayReplacementMode::InlineSourceItems)? {
            BufferTextSourceCursorItem::Item(item) => Some(item),
            BufferTextSourceCursorItem::Replacement(_) => {
                debug_assert!(false, "inline source cursor surfaced a buffer replacement");
                None
            }
        }
    }
}

#[cfg(test)]
#[path = "display_buffer_text_source_test.rs"]
mod tests;

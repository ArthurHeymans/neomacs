use crate::display_item::{
    DisplayGlyphless, DisplayItem, DisplayItemKind, DisplayItemLayout, DisplayLength,
    DisplayMediaReplacement, DisplayRowBreak, DisplayRowBreakReason, DisplaySourceMappedText,
    DisplaySourcePosition, DisplayStretch, DisplayStretchWidth, DisplayTextRun,
    GlyphlessJoinerPolicy, GlyphlessMethod, RenderFaceRef, SourceSpan, glyphless_method_for_char,
};
use crate::display_property::{DisplayReplacementProperty, classify_display_property};
use crate::neovm_bridge::LayoutBufferView;
use crate::unicode::decode_utf8;
use neovm_core::buffer::{
    BufferId, CharLen, CharPos0, EmacsBytePos, EmacsByteRange, text_props::TextPropertyTable,
};
use neovm_core::emacs_core::Value;
use neovm_core::emacs_core::value::get_string_text_properties_table_for_value;

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

    fn resolve_display_media_replacement(
        &mut self,
        display_prop: Value,
        face: RenderFaceRef,
    ) -> Option<DisplayMediaReplacement> {
        self.face_resolver
            .as_mut()
            .and_then(|resolver| resolver.resolve_display_media_replacement(display_prop, face))
    }
}

impl Default for DisplaySourceContext<'_> {
    fn default() -> Self {
        Self::empty()
    }
}

pub(crate) trait DisplayItemSource {
    fn next_item(&mut self, context: &mut DisplaySourceContext<'_>) -> Option<DisplayItem>;
}

pub(crate) struct DisplayItemOnceSource {
    item: Option<DisplayItem>,
}

impl DisplayItemOnceSource {
    pub(crate) fn new(item: DisplayItem) -> Self {
        Self { item: Some(item) }
    }
}

impl DisplayItemSource for DisplayItemOnceSource {
    fn next_item(&mut self, _context: &mut DisplaySourceContext<'_>) -> Option<DisplayItem> {
        self.item.take()
    }
}

pub(crate) trait DisplayItemFaceResolver {
    fn resolve_face_ref(&mut self, base: RenderFaceRef, face_value: Value) -> RenderFaceRef;

    fn resolve_display_media_replacement(
        &mut self,
        _display_prop: Value,
        _face: RenderFaceRef,
    ) -> Option<DisplayMediaReplacement> {
        None
    }
}

pub(crate) struct SyntheticTextItemSource {
    item: Option<DisplayItem>,
}

impl SyntheticTextItemSource {
    pub(crate) fn new(
        source_id: u64,
        text: impl Into<Box<str>>,
        face: RenderFaceRef,
        start_offset: usize,
    ) -> Self {
        let text = text.into();
        let end_offset = start_offset.saturating_add(text.chars().count());
        let item = DisplayItem::new(
            SourceSpan::synthetic(source_id, start_offset, end_offset),
            face,
            DisplayItemKind::TextRun(DisplayTextRun::new(text)),
        );
        Self { item: Some(item) }
    }
}

impl DisplayItemSource for SyntheticTextItemSource {
    fn next_item(&mut self, _context: &mut DisplaySourceContext<'_>) -> Option<DisplayItem> {
        self.item.take()
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct BufferTextItemSource {
    buffer_id: BufferId,
    start_char: CharPos0,
    start_byte: EmacsBytePos,
    end_char: CharPos0,
    end_byte: EmacsBytePos,
}

impl BufferTextItemSource {
    pub(crate) const fn new(
        buffer_id: BufferId,
        start_char: CharPos0,
        start_byte: EmacsBytePos,
        end_char: CharPos0,
        end_byte: EmacsBytePos,
    ) -> Self {
        Self {
            buffer_id,
            start_char,
            start_byte,
            end_char,
            end_byte,
        }
    }

    pub(crate) fn single_char(
        buffer_id: BufferId,
        char_pos: CharPos0,
        start_byte: EmacsBytePos,
        end_byte: EmacsBytePos,
    ) -> Self {
        Self::new(
            buffer_id,
            char_pos,
            start_byte,
            char_pos.add_len(CharLen::new(1)),
            end_byte,
        )
    }

    fn span(self) -> SourceSpan {
        SourceSpan::new(
            DisplaySourcePosition::buffer(self.buffer_id, self.start_char, self.start_byte),
            DisplaySourcePosition::buffer(self.buffer_id, self.end_char, self.end_byte),
        )
    }

    pub(crate) fn item(self, face: RenderFaceRef, kind: DisplayItemKind) -> DisplayItem {
        DisplayItem::new(self.span(), face, kind)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BufferTextSourceRange {
    start: CharPos0,
    end: CharPos0,
}

impl BufferTextSourceRange {
    pub(crate) fn new(start: CharPos0, end: CharPos0) -> Self {
        Self { start, end }
    }

    pub(crate) fn single_char(start: CharPos0) -> Self {
        Self::new(start, start.add_len(CharLen::new(1)))
    }

    pub(crate) fn start(self) -> CharPos0 {
        self.start
    }

    pub(crate) fn end(self) -> CharPos0 {
        self.end
    }

    pub(crate) fn is_single_char(self) -> bool {
        self.end == self.start.add_len(CharLen::new(1))
    }

    pub(crate) fn is_empty_or_reversed(self) -> bool {
        self.end <= self.start
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BufferTextSourceAppendItem {
    ControlChar { ch: char },
    SourceMappedText { text: Box<str> },
    Glyphless { ch: char, method: GlyphlessMethod },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BufferTextSourceSpecialDisplay {
    Control(BufferTextSourceAppendItem),
    Nobreak(BufferTextSourceAppendItem),
    Glyphless(BufferTextSourceAppendItem),
}

impl BufferTextSourceSpecialDisplay {
    pub(crate) fn for_precluster_char(ch: char, nobreak_display_policy: i32) -> Option<Self> {
        if Self::is_control_char(ch) {
            Some(Self::Control(BufferTextSourceAppendItem::ControlChar {
                ch,
            }))
        } else {
            BufferTextSourceAppendItem::nobreak_display(ch, nobreak_display_policy)
                .map(Self::Nobreak)
        }
    }

    pub(crate) fn for_cluster_state(cluster: BufferTextSourceClusterState) -> Option<Self> {
        BufferTextSourceAppendItem::glyphless_display(cluster).map(Self::Glyphless)
    }

    pub(crate) fn into_append_item(self) -> BufferTextSourceAppendItem {
        match self {
            Self::Control(item) | Self::Nobreak(item) | Self::Glyphless(item) => item,
        }
    }

    pub(crate) fn is_control(&self) -> bool {
        matches!(self, Self::Control(_))
    }

    #[cfg(test)]
    pub(crate) fn is_nobreak(&self) -> bool {
        matches!(self, Self::Nobreak(_))
    }

    fn is_control_char(ch: char) -> bool {
        (ch < ' ' && ch != '\n' && ch != '\t') || ch == '\x7F'
    }

    pub(crate) fn kind(&self) -> BufferTextSourceSpecialDisplayKind {
        match self {
            Self::Control(_) => BufferTextSourceSpecialDisplayKind::Control,
            Self::Nobreak(_) => BufferTextSourceSpecialDisplayKind::Nobreak,
            Self::Glyphless(_) => BufferTextSourceSpecialDisplayKind::Glyphless,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextSourceSpecialDisplayKind {
    Control,
    Nobreak,
    Glyphless,
}

impl BufferTextSourceSpecialDisplayKind {
    pub(crate) fn invalidates_face_after_append(self) -> bool {
        matches!(self, Self::Control | Self::Nobreak)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSourceChar {
    ch: char,
    range: BufferTextSourceRange,
    precluster_special_display: Option<BufferTextSourceSpecialDisplay>,
}

impl BufferTextSourceChar {
    pub(crate) fn new(ch: char, start: CharPos0, nobreak_display_policy: i32) -> Self {
        Self {
            ch,
            range: BufferTextSourceRange::single_char(start),
            precluster_special_display: BufferTextSourceSpecialDisplay::for_precluster_char(
                ch,
                nobreak_display_policy,
            ),
        }
    }

    pub(crate) fn range(&self) -> BufferTextSourceRange {
        self.range
    }

    pub(crate) fn precluster_special_display(&self) -> Option<&BufferTextSourceSpecialDisplay> {
        self.precluster_special_display.as_ref()
    }

    pub(crate) fn cluster_state(&self, tail: Option<(char, bool)>) -> BufferTextSourceClusterState {
        BufferTextSourceClusterState::for_char(self.ch, tail)
    }

    pub(crate) fn cluster_special_display(
        &self,
        tail: Option<(char, bool)>,
    ) -> Option<BufferTextSourceSpecialDisplay> {
        BufferTextSourceSpecialDisplay::for_cluster_state(self.cluster_state(tail))
    }

    fn special_request_for_display(
        &self,
        display: BufferTextSourceSpecialDisplay,
    ) -> BufferTextSpecialSourceCharRequest {
        BufferTextSpecialSourceCharRequest::new(self, display)
    }

    #[cfg(test)]
    pub(crate) fn control_special_request(&self) -> Option<BufferTextSpecialSourceCharRequest> {
        self.precluster_special_display()
            .filter(|display| display.is_control())
            .cloned()
            .map(|display| self.special_request_for_display(display))
    }

    #[cfg(test)]
    pub(crate) fn nobreak_special_request(&self) -> Option<BufferTextSpecialSourceCharRequest> {
        self.precluster_special_display()
            .filter(|display| display.is_nobreak())
            .cloned()
            .map(|display| self.special_request_for_display(display))
    }

    pub(crate) fn cluster_special_request(
        &self,
        tail: Option<(char, bool)>,
    ) -> Option<BufferTextSpecialSourceCharRequest> {
        self.cluster_special_display(tail)
            .map(|display| self.special_request_for_display(display))
    }

    pub(crate) fn special_request(
        &self,
        tail: Option<(char, bool)>,
    ) -> Option<BufferTextSpecialSourceCharRequest> {
        self.precluster_special_display()
            .cloned()
            .map(|display| self.special_request_for_display(display))
            .or_else(|| self.cluster_special_request(tail))
    }

    pub(crate) fn advance_request<'text>(
        &self,
        text: &'text [u8],
        byte_idx: usize,
        tail: Option<(char, bool)>,
    ) -> BufferTextSourceAdvanceRequest<'text> {
        BufferTextSourceAdvanceRequest::new(text, byte_idx, self.range(), self.cluster_state(tail))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSpecialSourceCharRequest {
    range: BufferTextSourceRange,
    special_display: BufferTextSourceSpecialDisplay,
}

impl BufferTextSpecialSourceCharRequest {
    pub(crate) fn new(
        source_char: &BufferTextSourceChar,
        special_display: BufferTextSourceSpecialDisplay,
    ) -> Self {
        Self {
            range: source_char.range(),
            special_display,
        }
    }

    pub(crate) fn kind(&self) -> BufferTextSourceSpecialDisplayKind {
        self.special_display.kind()
    }

    pub(crate) fn requires_overflow_measurement(&self) -> bool {
        self.special_display.is_control()
    }

    pub(crate) fn source_item_request(&self) -> BufferTextSourceItemRequest {
        BufferTextSourceItemRequest::new(
            self.range,
            self.special_display.clone().into_append_item(),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextDecodedSourceChar {
    ch: char,
    start_byte_idx: usize,
    start_charpos: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextDecodedSourceEvent {
    LineBreak(BufferTextLineBreakSourceEvent),
    Text(BufferTextSourceTextEvent),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextLineBreakSourceEvent {
    source_char: BufferTextDecodedSourceChar,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextSourceTextEvent {
    source_char: BufferTextDecodedSourceChar,
}

pub(crate) struct BufferTextSourceEventCursor<'text, 'state> {
    text: &'text [u8],
    byte_idx: &'state mut usize,
    charpos: i64,
}

impl BufferTextDecodedSourceEvent {
    pub(crate) fn decoded_char(self) -> BufferTextDecodedSourceChar {
        match self {
            Self::LineBreak(source_event) => source_event.decoded_char(),
            Self::Text(source_event) => source_event.decoded_char(),
        }
    }
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

impl BufferTextSourceTextEvent {
    pub(crate) fn new(source_char: BufferTextDecodedSourceChar) -> Self {
        debug_assert_ne!(source_char.ch(), '\n');
        Self { source_char }
    }

    pub(crate) fn decoded_char(self) -> BufferTextDecodedSourceChar {
        self.source_char
    }

    pub(crate) fn source_char(self, nobreak_display_policy: i32) -> BufferTextSourceChar {
        self.source_char.source_char(nobreak_display_policy)
    }
}

impl<'text, 'state> BufferTextSourceEventCursor<'text, 'state> {
    pub(crate) fn new(text: &'text [u8], byte_idx: &'state mut usize, charpos: i64) -> Self {
        Self {
            text,
            byte_idx,
            charpos,
        }
    }

    pub(crate) fn next_event(&mut self) -> Option<BufferTextDecodedSourceEvent> {
        BufferTextDecodedSourceChar::consume_from_text(self.text, self.byte_idx, self.charpos)
            .map(BufferTextDecodedSourceChar::into_event)
    }
}

impl BufferTextDecodedSourceChar {
    pub(crate) fn consume_from_text(
        text: &[u8],
        byte_idx: &mut usize,
        charpos: i64,
    ) -> Option<Self> {
        if *byte_idx >= text.len() {
            return None;
        }

        let start_byte_idx = *byte_idx;
        let (ch, ch_len) = decode_utf8(&text[*byte_idx..]);
        if ch_len == 0 {
            return None;
        }
        *byte_idx += ch_len;

        Some(Self {
            ch,
            start_byte_idx,
            start_charpos: charpos,
        })
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

    pub(crate) fn into_event(self) -> BufferTextDecodedSourceEvent {
        if self.ch == '\n' {
            BufferTextDecodedSourceEvent::LineBreak(BufferTextLineBreakSourceEvent::new(self))
        } else {
            BufferTextDecodedSourceEvent::Text(BufferTextSourceTextEvent::new(self))
        }
    }

    pub(crate) fn source_char(self, nobreak_display_policy: i32) -> BufferTextSourceChar {
        BufferTextSourceChar::new(self.ch, self.source_range().start(), nobreak_display_policy)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextSourceTextItemRequest {
    range: BufferTextSourceRange,
    ch: char,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ResolvedBufferTextSourceAdvance {
    Natural { advance_px: f32 },
    Resolved { advance_px: f32 },
}

impl ResolvedBufferTextSourceAdvance {
    pub(crate) fn natural(advance_px: f32) -> Self {
        Self::Natural { advance_px }
    }

    pub(crate) fn resolved(advance_px: f32) -> Self {
        Self::Resolved { advance_px }
    }

    pub(crate) fn advance_px(self) -> f32 {
        match self {
            Self::Natural { advance_px } | Self::Resolved { advance_px } => advance_px,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextSourceTextRequest {
    source_item: BufferTextSourceTextItemRequest,
    resolved_advance: ResolvedBufferTextSourceAdvance,
}

impl BufferTextSourceTextRequest {
    #[cfg(test)]
    pub(crate) fn new(
        range: BufferTextSourceRange,
        source_char: char,
        resolved_advance: ResolvedBufferTextSourceAdvance,
    ) -> Self {
        Self {
            source_item: BufferTextSourceTextItemRequest::new(range, source_char),
            resolved_advance,
        }
    }

    pub(crate) fn from_source_item(
        source_item: BufferTextSourceTextItemRequest,
        resolved_advance: ResolvedBufferTextSourceAdvance,
    ) -> Self {
        Self {
            source_item,
            resolved_advance,
        }
    }

    pub(crate) fn source_item(self) -> BufferTextSourceTextItemRequest {
        self.source_item
    }

    pub(crate) fn resolved_advance(self) -> ResolvedBufferTextSourceAdvance {
        self.resolved_advance
    }

    pub(crate) fn advance_px(self) -> f32 {
        self.resolved_advance.advance_px()
    }
}

impl BufferTextSourceTextItemRequest {
    pub(crate) fn new(range: BufferTextSourceRange, ch: char) -> Self {
        Self { range, ch }
    }

    pub(crate) fn for_range_and_cluster(
        range: BufferTextSourceRange,
        cluster: BufferTextSourceClusterState,
    ) -> Self {
        Self::new(range, cluster.ch())
    }

    pub(crate) fn range(self) -> BufferTextSourceRange {
        self.range
    }

    pub(crate) fn source_char(self) -> char {
        self.ch
    }

    pub(crate) fn into_display_item_kind(self) -> DisplayItemKind {
        DisplayItemKind::TextRun(DisplayTextRun::new(self.ch.to_string()))
    }

    pub(crate) fn into_display_item<B: LayoutBufferView + ?Sized>(
        self,
        buffer_id: BufferId,
        buffer: &B,
        face: RenderFaceRef,
    ) -> Option<DisplayItem> {
        let range = self.range();
        if !range.is_single_char() {
            return None;
        }

        let start = range.start();
        let end = range.end();
        Some(
            BufferTextItemSource::single_char(
                buffer_id,
                start,
                buffer.layout_char_pos_to_emacs_byte_pos(start),
                buffer.layout_char_pos_to_emacs_byte_pos(end),
            )
            .item(face, self.into_display_item_kind()),
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSourceItemRequest {
    range: BufferTextSourceRange,
    item: BufferTextSourceAppendItem,
}

impl BufferTextSourceItemRequest {
    pub(crate) fn new(range: BufferTextSourceRange, item: BufferTextSourceAppendItem) -> Self {
        Self { range, item }
    }

    pub(crate) fn range(&self) -> BufferTextSourceRange {
        self.range
    }

    pub(crate) fn item(&self) -> &BufferTextSourceAppendItem {
        &self.item
    }

    pub(crate) fn fallback_width_px(&self, fallback_char_width: f32) -> f32 {
        self.item.fallback_width_px(fallback_char_width)
    }

    pub(crate) fn into_display_item_kind(self) -> DisplayItemKind {
        self.item.into_display_item_kind()
    }

    pub(crate) fn into_display_item<B: LayoutBufferView + ?Sized>(
        self,
        buffer_id: BufferId,
        buffer: &B,
        face: RenderFaceRef,
    ) -> Option<DisplayItem> {
        let range = self.range();
        if range.is_empty_or_reversed() {
            return None;
        }

        let start = range.start();
        let end = range.end();
        Some(
            BufferTextItemSource::new(
                buffer_id,
                start,
                buffer.layout_char_pos_to_emacs_byte_pos(start),
                end,
                buffer.layout_char_pos_to_emacs_byte_pos(end),
            )
            .item(face, self.into_display_item_kind()),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextSourceClusterState {
    ch: char,
    tail: Option<(char, bool)>,
    is_cluster_continuation: bool,
}

impl BufferTextSourceClusterState {
    pub(crate) fn for_char(ch: char, tail: Option<(char, bool)>) -> Self {
        Self {
            ch,
            tail,
            is_cluster_continuation: crate::composition::continues_cluster(ch, tail),
        }
    }

    pub(crate) fn is_cluster_continuation(self) -> bool {
        self.is_cluster_continuation
    }

    pub(crate) fn ch(self) -> char {
        self.ch
    }

    pub(crate) fn has_tail(self) -> bool {
        self.tail.is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextSourceAdvanceRequest<'text> {
    text: &'text [u8],
    byte_idx: usize,
    range: BufferTextSourceRange,
    cluster: BufferTextSourceClusterState,
}

impl<'text> BufferTextSourceAdvanceRequest<'text> {
    pub(crate) fn new(
        text: &'text [u8],
        byte_idx: usize,
        range: BufferTextSourceRange,
        cluster: BufferTextSourceClusterState,
    ) -> Self {
        Self {
            text,
            byte_idx,
            range,
            cluster,
        }
    }

    pub(crate) fn text(self) -> &'text [u8] {
        self.text
    }

    pub(crate) fn byte_idx(self) -> usize {
        self.byte_idx
    }

    pub(crate) fn range(self) -> BufferTextSourceRange {
        self.range
    }

    pub(crate) fn cluster(self) -> BufferTextSourceClusterState {
        self.cluster
    }

    pub(crate) fn into_text_request(
        self,
        resolved_advance: ResolvedBufferTextSourceAdvance,
    ) -> BufferTextSourceTextRequest {
        BufferTextSourceTextRequest::from_source_item(
            BufferTextSourceTextItemRequest::for_range_and_cluster(self.range, self.cluster),
            resolved_advance,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextSourceAdvancePath {
    NaturalRenderedSource,
    ResolvedComplexRun,
}

impl BufferTextSourceAdvancePath {
    pub(crate) fn for_cluster_state(cluster: BufferTextSourceClusterState) -> Self {
        if crate::composition::needs_complex_shaping(cluster.ch()) {
            Self::ResolvedComplexRun
        } else {
            Self::NaturalRenderedSource
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextSourceNaturalFallbackAdvance {
    Tab,
    ClusterContinuation,
    FaceColumns { columns: usize },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextSourceNaturalAdvanceRequest {
    source_item: BufferTextSourceTextItemRequest,
    fallback: BufferTextSourceNaturalFallbackAdvance,
}

impl BufferTextSourceNaturalFallbackAdvance {
    pub(crate) fn for_cluster_state(cluster: BufferTextSourceClusterState) -> Self {
        let ch = cluster.ch();
        if ch == '\t' {
            Self::Tab
        } else if cluster.is_cluster_continuation() {
            Self::ClusterContinuation
        } else {
            Self::FaceColumns {
                columns: crate::composition::base_width_cols(ch) as usize,
            }
        }
    }
}

impl BufferTextSourceNaturalAdvanceRequest {
    pub(crate) fn for_range_and_cluster(
        range: BufferTextSourceRange,
        cluster: BufferTextSourceClusterState,
    ) -> Self {
        Self {
            source_item: BufferTextSourceTextItemRequest::for_range_and_cluster(range, cluster),
            fallback: BufferTextSourceNaturalFallbackAdvance::for_cluster_state(cluster),
        }
    }

    pub(crate) fn source_item(self) -> BufferTextSourceTextItemRequest {
        self.source_item
    }

    pub(crate) fn fallback(self) -> BufferTextSourceNaturalFallbackAdvance {
        self.fallback
    }
}

impl BufferTextSourceAppendItem {
    pub(crate) fn nobreak_display(ch: char, display_policy: i32) -> Option<Self> {
        let text = match (display_policy, ch) {
            (1, '\u{00A0}') => " ",
            (1, '\u{00AD}') => "-",
            (2, '\u{00A0}') => "\\ ",
            (2, '\u{00AD}') => "\\-",
            _ => return None,
        };
        Some(Self::SourceMappedText { text: text.into() })
    }

    pub(crate) fn glyphless_display(cluster: BufferTextSourceClusterState) -> Option<Self> {
        let ch = cluster.ch();
        if cluster.has_tail() && crate::composition::is_composition_joiner(ch) {
            return None;
        }
        let method = glyphless_method_for_char(ch, GlyphlessJoinerPolicy::ClassifyAsGlyphless)?;
        Some(Self::Glyphless { ch, method })
    }

    pub(crate) fn fallback_width_columns(&self) -> usize {
        match self {
            Self::ControlChar { .. } => 2,
            Self::SourceMappedText { text } => text.chars().count().max(1),
            Self::Glyphless { .. } => 1,
        }
    }

    pub(crate) fn fallback_width_px(&self, fallback_char_width: f32) -> f32 {
        self.fallback_width_columns() as f32 * fallback_char_width.max(1.0)
    }

    pub(crate) fn into_display_item_kind(self) -> DisplayItemKind {
        match self {
            Self::ControlChar { ch } => DisplayItemKind::ControlChar { ch },
            Self::SourceMappedText { text } => {
                DisplayItemKind::SourceMappedText(DisplaySourceMappedText::new(text))
            }
            Self::Glyphless { ch, method } => {
                DisplayItemKind::Glyphless(DisplayGlyphless { ch, method })
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct DisplayReplacementBox {
    width_px: f32,
    height_px: f32,
    ascent_px: f32,
}

impl DisplayReplacementBox {
    pub(crate) fn new(width_px: f32, height_px: f32, ascent_px: f32) -> Self {
        Self {
            width_px: width_px.max(0.0),
            height_px: height_px.max(0.0),
            ascent_px: ascent_px.max(0.0),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct BufferDisplayReplacementSource {
    buffer_id: BufferId,
    char_pos: CharPos0,
    byte_pos: EmacsBytePos,
}

impl BufferDisplayReplacementSource {
    pub(crate) const fn new(
        buffer_id: BufferId,
        char_pos: CharPos0,
        byte_pos: EmacsBytePos,
    ) -> Self {
        Self {
            buffer_id,
            char_pos,
            byte_pos,
        }
    }

    #[cfg(test)]
    pub(crate) fn byte_pos(self) -> EmacsBytePos {
        self.byte_pos
    }

    fn span(self) -> SourceSpan {
        let end = self.char_pos.add_len(CharLen::new(1));
        SourceSpan::new(
            DisplaySourcePosition::buffer(self.buffer_id, self.char_pos, self.byte_pos),
            DisplaySourcePosition::buffer(self.buffer_id, end, self.byte_pos),
        )
    }

    fn item(self, face_id: u32, kind: DisplayItemKind) -> DisplayItem {
        self.item_with_face(RenderFaceRef::FaceId(face_id), kind)
    }

    fn item_with_face(self, face: RenderFaceRef, kind: DisplayItemKind) -> DisplayItem {
        DisplayItem::new(self.span(), face, kind)
    }

    pub(crate) fn stretch_item(self, face_id: u32, geometry: DisplayReplacementBox) -> DisplayItem {
        self.item(
            face_id,
            DisplayItemKind::Stretch(DisplayStretch {
                width: DisplayStretchWidth::Length(DisplayLength::Pixels(geometry.width_px)),
                height: Some(DisplayLength::Pixels(geometry.height_px)),
                ascent: Some(DisplayLength::Pixels(geometry.ascent_px)),
            }),
        )
    }

    pub(crate) fn media_item(self, face_id: u32, media: DisplayMediaReplacement) -> DisplayItem {
        self.item(face_id, DisplayItemKind::MediaReplacement(media))
    }

    pub(crate) fn source_mapped_text_item(
        self,
        face_id: u32,
        text: impl Into<Box<str>>,
    ) -> DisplayItem {
        self.item(
            face_id,
            DisplayItemKind::SourceMappedText(DisplaySourceMappedText::new(text)),
        )
    }
}

#[derive(Clone, Debug)]
pub(crate) enum DisplayReplacementAppendItem {
    Media(DisplayMediaReplacement),
    Stretch(DisplayReplacementBox),
    SourceMappedText(Box<str>),
}

impl DisplayReplacementAppendItem {
    pub(crate) fn media(media: DisplayMediaReplacement) -> Self {
        Self::Media(media)
    }

    pub(crate) fn stretch(geometry: DisplayReplacementBox) -> Self {
        Self::Stretch(geometry)
    }

    pub(crate) fn source_mapped_text(text: impl Into<Box<str>>) -> Self {
        Self::SourceMappedText(text.into())
    }

    pub(crate) fn into_display_item(
        self,
        replacement_source: BufferDisplayReplacementSource,
        face_id: u32,
    ) -> DisplayItem {
        match self {
            Self::Media(media) => replacement_source.media_item(face_id, media),
            Self::Stretch(geometry) => replacement_source.stretch_item(face_id, geometry),
            Self::SourceMappedText(text) => {
                replacement_source.source_mapped_text_item(face_id, text)
            }
        }
    }
}

pub(crate) struct BufferDisplayReplacementStringSource<S> {
    replacement_source: BufferDisplayReplacementSource,
    source: S,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct BufferDisplayReplacementStringRequest {
    source_id: u64,
    value: Value,
    replacement_source: BufferDisplayReplacementSource,
}

impl BufferDisplayReplacementStringRequest {
    pub(crate) fn new(
        source_id: u64,
        value: Value,
        replacement_source: BufferDisplayReplacementSource,
    ) -> Self {
        Self {
            source_id,
            value,
            replacement_source,
        }
    }

    pub(crate) fn into_source(
        self,
        fallback_face_id: u32,
    ) -> Option<BufferDisplayReplacementStringSource<LispStringSourceCursor>> {
        let string_source = LispStringSourceCursor::new(
            self.source_id,
            self.value,
            RenderFaceRef::FaceId(fallback_face_id),
        )?;
        Some(BufferDisplayReplacementStringSource::new(
            self.replacement_source,
            string_source,
        ))
    }

    #[cfg(test)]
    pub(crate) fn source_id(self) -> u64 {
        self.source_id
    }

    #[cfg(test)]
    pub(crate) fn value(self) -> Value {
        self.value
    }
}

impl<S> BufferDisplayReplacementStringSource<S> {
    pub(crate) const fn new(replacement_source: BufferDisplayReplacementSource, source: S) -> Self {
        Self {
            replacement_source,
            source,
        }
    }
}

impl<S: DisplayItemSource> DisplayItemSource for BufferDisplayReplacementStringSource<S> {
    fn next_item(&mut self, context: &mut DisplaySourceContext<'_>) -> Option<DisplayItem> {
        let item = self.source.next_item(context)?;
        let kind = match item.kind {
            DisplayItemKind::TextRun(run) => {
                DisplayItemKind::SourceMappedText(DisplaySourceMappedText::new(run.text))
            }
            kind => kind,
        };
        Some(self.replacement_source.item_with_face(item.face, kind))
    }
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

    pub(crate) fn discard_until_row_break(&mut self) -> bool {
        let mut context = DisplaySourceContext::empty();
        while let Some(item) = self.next_item(&mut context) {
            if matches!(item.kind, DisplayItemKind::RowBreak(_)) {
                return true;
            }
        }
        false
    }
}

impl DisplayItemSource for LispStringSourceCursor {
    fn next_item(&mut self, context: &mut DisplaySourceContext<'_>) -> Option<DisplayItem> {
        self.stack.next_item(context)
    }
}

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
    #[cfg(test)]
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
    #[cfg(test)]
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

    #[allow(dead_code)]
    fn source_position(&self) -> DisplaySourcePosition {
        self.frames
            .last()
            .map(LispStringSourceFrame::source_position)
            .unwrap_or_else(|| DisplaySourcePosition::synthetic(0, 0))
    }

    #[allow(dead_code)]
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

        let mut item_layout = DisplayItemLayout::default();
        if let Some(display_prop) = self.display_prop_at(start) {
            self.char_index = property_end;
            match display_property_source_action(context, display_prop, face)
                .into_cursor_action(span, face)
            {
                DisplayPropertySourceCursorAction::PushReplacement { value, base_face } => {
                    return LispStringAction::PushReplacement { value, base_face };
                }
                DisplayPropertySourceCursorAction::Emit(item) => {
                    return LispStringAction::Emit(item);
                }
                DisplayPropertySourceCursorAction::FallThrough { layout } => {
                    item_layout = layout;
                }
            }
        }

        let Some(ch) = self.char_at(start) else {
            return LispStringAction::PopFrame;
        };
        if let Some(kind) = display_item_kind_for_text_source_char(ch) {
            self.char_index = start + 1;
            return LispStringAction::Emit(
                DisplayItem::new(self.span(start, start + 1), face, kind).with_layout(item_layout),
            );
        }

        let end = self.next_text_run_end(start, property_end);
        self.char_index = end;
        LispStringAction::Emit(
            DisplayItem::new(
                self.span(start, end),
                face,
                DisplayItemKind::TextRun(DisplayTextRun::new(self.text_slice(start, end))),
            )
            .with_layout(item_layout),
        )
    }

    fn char_count(&self) -> usize {
        self.char_byte_offsets.len().saturating_sub(1)
    }

    #[allow(dead_code)]
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
            if classify_text_source_char(ch) != TextSourceCharClassification::Text {
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

#[derive(Clone, Debug, PartialEq)]
enum DisplayPropertySourceAction {
    PushReplacement {
        value: Value,
        base_face: RenderFaceRef,
    },
    Emit {
        kind: DisplayItemKind,
        layout: DisplayItemLayout,
    },
    Ignore {
        layout: DisplayItemLayout,
    },
}

#[derive(Clone, Debug, PartialEq)]
enum DisplayPropertySourceCursorAction {
    PushReplacement {
        value: Value,
        base_face: RenderFaceRef,
    },
    Emit(DisplayItem),
    FallThrough {
        layout: DisplayItemLayout,
    },
}

impl DisplayPropertySourceAction {
    fn into_cursor_action(
        self,
        span: SourceSpan,
        face: RenderFaceRef,
    ) -> DisplayPropertySourceCursorAction {
        match self {
            Self::PushReplacement { value, base_face } => {
                DisplayPropertySourceCursorAction::PushReplacement { value, base_face }
            }
            Self::Emit { kind, layout } => DisplayPropertySourceCursorAction::Emit(
                DisplayItem::new(span, face, kind).with_layout(layout),
            ),
            Self::Ignore { layout } => DisplayPropertySourceCursorAction::FallThrough { layout },
        }
    }
}

enum DisplayPropertySourceReplacement {
    String(Value),
    Item(DisplayItemKind),
    Unresolved,
}

impl DisplayPropertySourceReplacement {
    fn resolve(
        context: &mut DisplaySourceContext<'_>,
        display_prop: Value,
        replacement: Option<&DisplayReplacementProperty>,
        face: RenderFaceRef,
    ) -> Self {
        match replacement {
            Some(DisplayReplacementProperty::String) => Self::String(display_prop),
            Some(DisplayReplacementProperty::Stretch(stretch)) => {
                Self::Item(DisplayItemKind::Stretch(stretch.clone()))
            }
            Some(DisplayReplacementProperty::Media(replacement)) => replacement
                .direct_replacement()
                .map(DisplayItemKind::MediaReplacement)
                .or_else(|| {
                    context
                        .resolve_display_media_replacement(display_prop, face)
                        .filter(|media| replacement.accepts_media_replacement(media))
                        .map(DisplayItemKind::MediaReplacement)
                })
                .map(Self::Item)
                .unwrap_or(Self::Unresolved),
            None => context
                .resolve_display_media_replacement(display_prop, face)
                .map(DisplayItemKind::MediaReplacement)
                .map(Self::Item)
                .unwrap_or(Self::Unresolved),
        }
    }
}

fn display_property_source_action(
    context: &mut DisplaySourceContext<'_>,
    display_prop: Value,
    face: RenderFaceRef,
) -> DisplayPropertySourceAction {
    let classification = classify_display_property(display_prop);
    match DisplayPropertySourceReplacement::resolve(
        context,
        display_prop,
        classification.replacement(),
        face,
    ) {
        DisplayPropertySourceReplacement::String(value) => {
            DisplayPropertySourceAction::PushReplacement {
                value,
                base_face: face,
            }
        }
        DisplayPropertySourceReplacement::Item(kind) => DisplayPropertySourceAction::Emit {
            kind,
            layout: classification.modifiers(),
        },
        DisplayPropertySourceReplacement::Unresolved => DisplayPropertySourceAction::Ignore {
            layout: classification.modifiers(),
        },
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TextSourceCharClassification {
    Text,
    RowBreak,
    ControlChar { ch: char },
    Glyphless { ch: char, method: GlyphlessMethod },
}

fn classify_text_source_char(ch: char) -> TextSourceCharClassification {
    if ch == '\n' {
        return TextSourceCharClassification::RowBreak;
    }
    if is_control_char(ch) {
        return TextSourceCharClassification::ControlChar { ch };
    }
    if let Some(method) =
        glyphless_method_for_char(ch, GlyphlessJoinerPolicy::PreserveForComposition)
    {
        return TextSourceCharClassification::Glyphless { ch, method };
    }
    TextSourceCharClassification::Text
}

fn display_item_kind_for_text_source_char(ch: char) -> Option<DisplayItemKind> {
    match classify_text_source_char(ch) {
        TextSourceCharClassification::Text => None,
        TextSourceCharClassification::RowBreak => {
            Some(DisplayItemKind::RowBreak(DisplayRowBreak {
                reason: DisplayRowBreakReason::ExplicitNewline,
            }))
        }
        TextSourceCharClassification::ControlChar { ch } => {
            Some(DisplayItemKind::ControlChar { ch })
        }
        TextSourceCharClassification::Glyphless { ch, method } => {
            Some(DisplayItemKind::Glyphless(DisplayGlyphless { ch, method }))
        }
    }
}

fn is_control_char(ch: char) -> bool {
    let code = ch as u32;
    (code <= 0x1f && ch != '\n' && ch != '\t') || code == 0x7f
}

#[cfg(test)]
#[path = "display_source_test.rs"]
mod tests;

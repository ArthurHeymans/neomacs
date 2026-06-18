use crate::display_item::{
    DisplayGlyphless, DisplayItem, DisplayItemKind, DisplayItemLayout, DisplayLength,
    DisplayMediaReplacement, DisplayRowBreak, DisplayRowBreakReason, DisplaySourceMappedText,
    DisplaySourcePosition, DisplayStretch, DisplayStretchWidth, DisplayTextRun,
    GlyphlessJoinerPolicy, GlyphlessMethod, RenderFaceRef, SourceSpan, glyphless_method_for_char,
};
use crate::display_origin::{DisplayOrigin, DisplayPropertySource};
use crate::display_property::{
    DisplayPropertyClassification, DisplayReplacementProperty, classify_display_property,
};
use crate::display_space::{DisplaySpaceKey, display_space_positive_number};
use crate::neovm_bridge::LayoutBufferView;
use crate::types::WindowParams;
use neovm_core::buffer::{
    BufferId, CharLen, CharPos0, EmacsBytePos, text_props::TextPropertyTable,
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

    pub(crate) fn resolve_face_ref(
        &mut self,
        base: RenderFaceRef,
        face_value: Value,
    ) -> RenderFaceRef {
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DisplayReplacementSourceMappedTextItem {
    text: Box<str>,
}

impl DisplayReplacementSourceMappedTextItem {
    pub(crate) fn new(text: impl Into<Box<str>>) -> Self {
        Self { text: text.into() }
    }

    pub(crate) fn into_text(self) -> Box<str> {
        self.text
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct DisplayReplacementStretchSourceItem {
    geometry: DisplayReplacementBox,
    width_px: f32,
    height_px: f32,
    ascent_px: f32,
    cursor_slot_width_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayReplacementSpaceGeometry {
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) ascent: f32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum DisplayReplacementSpaceWidthPolicy {
    Explicit(Value),
    Relative { factor: f32 },
    AlignTo(Value),
    Default,
}

impl DisplayReplacementSpaceWidthPolicy {
    pub(crate) fn from_items(items: &[Value]) -> Self {
        if let Some(prop) = display_space_plist_value(items, DisplaySpaceKey::Width)
            && !prop.is_nil()
        {
            Self::Explicit(prop)
        } else if let Some(prop) = display_space_plist_value(items, DisplaySpaceKey::RelativeWidth)
            && let Some(factor) = display_space_positive_number(prop)
        {
            Self::Relative { factor }
        } else if let Some(prop) = display_space_plist_value(items, DisplaySpaceKey::AlignTo)
            && !prop.is_nil()
        {
            Self::AlignTo(prop)
        } else {
            Self::Default
        }
    }

    fn zero_width_allowed(self) -> bool {
        matches!(self, Self::AlignTo(_))
    }

    fn resolve(
        self,
        pctx: &crate::display_pixel_calc::PixelCalcContext,
        current_x: f32,
        content_x: f32,
        display_char_width: f32,
        default_width: f32,
    ) -> f32 {
        use crate::display_pixel_calc::calc_pixel_width_or_height;

        match self {
            Self::Explicit(prop) => calc_pixel_width_or_height(pctx, &prop, true, None)
                .map(|pixels| pixels as f32)
                .unwrap_or(default_width),
            Self::Relative { factor } => factor * display_char_width.max(0.0),
            Self::AlignTo(prop) => {
                let mut align_to: i32 = -1;
                if let Some(pixels) =
                    calc_pixel_width_or_height(pctx, &prop, true, Some(&mut align_to))
                {
                    let target_x = if align_to >= 0 {
                        align_to as f32 + pixels as f32
                    } else {
                        content_x + pixels as f32
                    };
                    (target_x - current_x).max(0.0)
                } else {
                    default_width
                }
            }
            Self::Default => default_width,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum DisplayReplacementSpaceHeightPolicy {
    Explicit(Value),
    Relative { factor: f32 },
    Default,
}

impl DisplayReplacementSpaceHeightPolicy {
    pub(crate) fn from_items(items: &[Value]) -> Self {
        if let Some(prop) = display_space_plist_value(items, DisplaySpaceKey::Height)
            && !prop.is_nil()
        {
            Self::Explicit(prop)
        } else if let Some(prop) = display_space_plist_value(items, DisplaySpaceKey::RelativeHeight)
            && let Some(factor) = display_space_positive_number(prop)
        {
            Self::Relative { factor }
        } else {
            Self::Default
        }
    }

    fn zero_height_allowed(self) -> bool {
        matches!(self, Self::Explicit(_))
    }

    fn resolve(
        self,
        pctx: &crate::display_pixel_calc::PixelCalcContext,
        default_height: f32,
    ) -> f32 {
        use crate::display_pixel_calc::calc_pixel_width_or_height;

        match self {
            Self::Explicit(prop) => calc_pixel_width_or_height(pctx, &prop, false, None)
                .map(|pixels| pixels as f32)
                .unwrap_or(default_height),
            Self::Relative { factor } => default_height * factor,
            Self::Default => default_height,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum DisplayReplacementSpaceAscentPolicy {
    Percent { percent: f32 },
    Pixel(Value),
    Default,
}

impl DisplayReplacementSpaceAscentPolicy {
    pub(crate) fn from_items(items: &[Value]) -> Self {
        let Some(prop) = display_space_plist_value(items, DisplaySpaceKey::Ascent) else {
            return Self::Default;
        };
        if let Some(percent) = display_space_positive_number(prop)
            && percent <= 100.0
        {
            Self::Percent { percent }
        } else if !prop.is_nil() {
            Self::Pixel(prop)
        } else {
            Self::Default
        }
    }

    fn resolve(
        self,
        pctx: &crate::display_pixel_calc::PixelCalcContext,
        height: f32,
        default_ascent: f32,
        default_height: f32,
    ) -> f32 {
        use crate::display_pixel_calc::calc_pixel_width_or_height;

        match self {
            Self::Percent { percent } => height * percent / 100.0,
            Self::Pixel(prop) => calc_pixel_width_or_height(pctx, &prop, false, None)
                .map(|pixels| (pixels as f32).max(0.0).min(height))
                .unwrap_or_else(|| Self::default_ascent(height, default_ascent, default_height)),
            Self::Default => Self::default_ascent(height, default_ascent, default_height),
        }
    }

    fn default_ascent(height: f32, default_ascent: f32, default_height: f32) -> f32 {
        height * default_ascent / default_height
    }
}

fn display_space_plist_value(items: &[Value], wanted: DisplaySpaceKey) -> Option<Value> {
    let mut i = 1;
    while i + 1 < items.len() {
        if DisplaySpaceKey::from_lisp_value(items[i]) == Some(wanted) {
            return Some(items[i + 1]);
        }
        i += 2;
    }
    None
}

impl DisplayReplacementStretchSourceItem {
    pub(crate) fn from_extents(width_px: f32, height_px: f32, ascent_px: f32) -> Self {
        let width_px = width_px.max(0.0);
        let height_px = height_px.max(0.0);
        let ascent_px = ascent_px.max(0.0);
        Self {
            geometry: DisplayReplacementBox::new(width_px, height_px, ascent_px),
            width_px,
            height_px,
            ascent_px,
            cursor_slot_width_px: width_px,
        }
    }

    pub(crate) fn from_space_extents(
        width_px: f32,
        height_px: f32,
        ascent_px: f32,
        fallback_cursor_width_px: f32,
    ) -> Self {
        let mut item = Self::from_extents(width_px, height_px, ascent_px);
        item.cursor_slot_width_px = item.width_px.max(fallback_cursor_width_px);
        item
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn display_space_geometry(
        spec: &Value,
        current_x: f32,
        content_x: f32,
        face_char_w: f32,
        display_char_width: f32,
        default_height: f32,
        default_ascent: f32,
        params: &WindowParams,
    ) -> DisplayReplacementSpaceGeometry {
        use crate::display_pixel_calc::PixelCalcContext;

        let default_width = params.char_width.max(1.0);
        let default_height = if params.window_system {
            default_height.max(1.0)
        } else {
            params.char_height.max(1.0)
        };
        let default_ascent = if params.window_system {
            default_ascent.max(0.0).min(default_height)
        } else {
            default_height
        };
        let Some(items) = list_to_vec(spec) else {
            return DisplayReplacementSpaceGeometry {
                width: default_width,
                height: default_height,
                ascent: default_ascent,
            };
        };

        let pctx = PixelCalcContext {
            frame_column_width: params.char_width.max(1.0) as f64,
            frame_line_height: params.char_height.max(1.0) as f64,
            frame_res_x: 96.0,
            frame_res_y: 96.0,
            face_font_height: default_height as f64,
            face_font_width: face_char_w.round().max(1.0) as f64,
            text_area_left: params.text_bounds.x as f64,
            text_area_right: (params.text_bounds.x + params.text_bounds.width) as f64,
            text_area_width: params.text_bounds.width as f64,
            left_margin_left: (params.text_bounds.x
                - params.left_fringe_width
                - params.left_margin_width) as f64,
            left_margin_width: params.left_margin_width as f64,
            right_margin_left: (params.text_bounds.x
                + params.text_bounds.width
                + params.right_fringe_width) as f64,
            right_margin_width: params.right_margin_width as f64,
            left_fringe_width: params.left_fringe_width as f64,
            right_fringe_width: params.right_fringe_width as f64,
            fringes_outside_margins: false,
            scroll_bar_width: 0.0,
            scroll_bar_on_left: false,
            line_number_pixel_width: 0.0,
            symbol_values: std::collections::HashMap::new(),
        };

        let width_policy = DisplayReplacementSpaceWidthPolicy::from_items(&items);
        let mut width = width_policy.resolve(
            &pctx,
            current_x,
            content_x,
            display_char_width,
            default_width,
        );
        if width <= 0.0 && (width < 0.0 || !width_policy.zero_width_allowed()) {
            width = 1.0;
        }

        let (height, ascent) = if params.window_system {
            let height_policy = DisplayReplacementSpaceHeightPolicy::from_items(&items);
            let mut height = height_policy.resolve(&pctx, default_height);
            if height <= 0.0 && (height < 0.0 || !height_policy.zero_height_allowed()) {
                height = 1.0;
            }

            let ascent = DisplayReplacementSpaceAscentPolicy::from_items(&items).resolve(
                &pctx,
                height,
                default_ascent,
                default_height,
            );
            (height, ascent)
        } else {
            (1.0, 1.0)
        };

        DisplayReplacementSpaceGeometry {
            width,
            height,
            ascent: ascent.max(0.0).min(height),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_display_space_spec(
        spec: &Value,
        current_x: f32,
        content_x: f32,
        face_char_w: f32,
        display_char_width: f32,
        default_height: f32,
        default_ascent: f32,
        fallback_cursor_width_px: f32,
        params: &WindowParams,
    ) -> Self {
        let geometry = Self::display_space_geometry(
            spec,
            current_x,
            content_x,
            face_char_w,
            display_char_width,
            default_height,
            default_ascent,
            params,
        );
        Self::from_space_extents(
            geometry.width,
            geometry.height,
            geometry.ascent,
            fallback_cursor_width_px,
        )
    }

    pub(crate) fn width_px(self) -> f32 {
        self.width_px
    }

    pub(crate) fn height_px(self) -> f32 {
        self.height_px
    }

    pub(crate) fn ascent_px(self) -> f32 {
        self.ascent_px
    }

    pub(crate) fn cursor_slot_width_px(self) -> f32 {
        self.cursor_slot_width_px
    }

    pub(crate) fn geometry(self) -> DisplayReplacementBox {
        self.geometry
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayReplacementMediaSourceItem {
    media: DisplayMediaReplacement,
    cursor_face_height: f32,
    cursor_face_ascent: f32,
}

impl DisplayReplacementMediaSourceItem {
    pub(crate) fn new(
        media: DisplayMediaReplacement,
        face_height: f32,
        face_ascent: f32,
        uses_xwidget_cursor_extents: bool,
    ) -> Self {
        let (cursor_face_height, cursor_face_ascent) = if uses_xwidget_cursor_extents {
            (media.height.max(face_height), media.height.max(face_ascent))
        } else {
            (media.height, media.height)
        };
        Self {
            media,
            cursor_face_height,
            cursor_face_ascent,
        }
    }

    pub(crate) fn media(self) -> DisplayMediaReplacement {
        self.media
    }

    pub(crate) fn width_px(self) -> f32 {
        self.media.width
    }

    pub(crate) fn display_height_px(self) -> f32 {
        self.media.height
    }

    pub(crate) fn display_ascent_px(self) -> f32 {
        self.media.height
    }

    pub(crate) fn cursor_face_height_px(self) -> f32 {
        self.cursor_face_height
    }

    pub(crate) fn cursor_face_ascent_px(self) -> f32 {
        self.cursor_face_ascent
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DisplayReplacementMediaSourceResolution {
    Media(DisplayReplacementMediaSourceItem),
    Placeholder(DisplayReplacementSourceMappedTextItem),
}

#[derive(Clone)]
pub(crate) struct DisplayReplacementStringSourceItem {
    value: Value,
    origin: DisplayOrigin,
    source_id: u64,
    cursor_slot_width_px: f32,
    is_empty: bool,
}

impl DisplayReplacementStringSourceItem {
    pub(crate) fn display_property_string(
        value: Value,
        anchor_charpos: CharPos0,
        source: DisplayPropertySource,
        source_id: u64,
        cursor_slot_width_px: f32,
    ) -> Option<Self> {
        let replacement = value.as_utf8_str()?;
        Some(Self {
            value,
            origin: DisplayOrigin::DisplayPropertyString {
                anchor_charpos,
                source,
            },
            source_id,
            cursor_slot_width_px,
            is_empty: replacement.is_empty(),
        })
    }

    pub(crate) fn value(&self) -> Value {
        self.value
    }

    pub(crate) fn source_id(&self) -> u64 {
        self.source_id
    }

    pub(crate) fn cursor_slot_width_px(&self) -> f32 {
        self.cursor_slot_width_px
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.is_empty
    }

    pub(crate) fn origin(&self) -> DisplayOrigin {
        self.origin
    }

    #[cfg(test)]
    pub(crate) fn base_face_policy(&self) -> crate::display_face_policy::BaseFacePolicy {
        self.origin.default_base_face_policy()
    }
}

#[derive(Clone)]
pub(crate) enum DisplayPropertyReplacementSourceItem {
    String(DisplayReplacementStringSourceItem),
    Stretch(DisplayReplacementStretchSourceItem),
    Media(DisplayReplacementMediaSourceResolution),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum DisplayPropertyReplacementCursorPolicy {
    TextSlot {
        width_px: f32,
        stretch_like: bool,
    },
    DisplayBox {
        width_px: f32,
        cursor_face_height_px: f32,
        cursor_face_ascent_px: f32,
    },
    FaceChar,
}

impl DisplayPropertyReplacementSourceItem {
    pub(crate) fn cursor_policy(&self) -> DisplayPropertyReplacementCursorPolicy {
        match self {
            Self::String(item) => DisplayPropertyReplacementCursorPolicy::TextSlot {
                width_px: item.cursor_slot_width_px(),
                stretch_like: false,
            },
            Self::Stretch(item) => DisplayPropertyReplacementCursorPolicy::TextSlot {
                width_px: item.cursor_slot_width_px(),
                stretch_like: true,
            },
            Self::Media(DisplayReplacementMediaSourceResolution::Media(item)) => {
                DisplayPropertyReplacementCursorPolicy::DisplayBox {
                    width_px: item.width_px(),
                    cursor_face_height_px: item.cursor_face_height_px(),
                    cursor_face_ascent_px: item.cursor_face_ascent_px(),
                }
            }
            Self::Media(DisplayReplacementMediaSourceResolution::Placeholder(_)) => {
                DisplayPropertyReplacementCursorPolicy::FaceChar
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayPropertyReplacementSourceMetrics {
    char_width: f32,
    row_height: f32,
    ascent: f32,
}

impl DisplayPropertyReplacementSourceMetrics {
    pub(crate) fn new(char_width: f32, row_height: f32, ascent: f32) -> Self {
        Self {
            char_width,
            row_height,
            ascent,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DisplayPropertyReplacementSourceInputs {
    string_cursor_slot_width_px: Option<f32>,
    stretch_display_char_width_px: Option<f32>,
    media: Option<DisplayReplacementMediaSourceResolution>,
}

impl DisplayPropertyReplacementSourceInputs {
    pub(crate) const fn empty() -> Self {
        Self {
            string_cursor_slot_width_px: None,
            stretch_display_char_width_px: None,
            media: None,
        }
    }

    pub(crate) fn with_string_cursor_slot_width_px(mut self, width_px: f32) -> Self {
        self.string_cursor_slot_width_px = Some(width_px);
        self
    }

    pub(crate) fn with_stretch_display_char_width_px(mut self, width_px: f32) -> Self {
        self.stretch_display_char_width_px = Some(width_px);
        self
    }

    pub(crate) fn with_media(mut self, media: DisplayReplacementMediaSourceResolution) -> Self {
        self.media = Some(media);
        self
    }
}

impl DisplayPropertyReplacementSourceItem {
    pub(crate) fn from_display_property(
        display_property: &DisplayPropertyClassification,
        source_event: BufferDisplayPropertyTextSourceEvent<'_>,
        current_x: f32,
        content_x: f32,
        params: &WindowParams,
        metrics: DisplayPropertyReplacementSourceMetrics,
        inputs: DisplayPropertyReplacementSourceInputs,
    ) -> Option<Self> {
        match display_property.replacement()? {
            DisplayReplacementProperty::String => {
                DisplayReplacementStringSourceItem::display_property_string(
                    source_event.value(),
                    source_event.anchor_charpos(),
                    DisplayPropertySource::TextProperty,
                    1,
                    inputs.string_cursor_slot_width_px?,
                )
                .map(Self::String)
            }
            DisplayReplacementProperty::Stretch(_) => Some(Self::Stretch(
                DisplayReplacementStretchSourceItem::from_display_space_spec(
                    &source_event.value(),
                    current_x,
                    content_x,
                    metrics.char_width,
                    inputs.stretch_display_char_width_px?,
                    metrics.row_height,
                    metrics.ascent,
                    metrics.char_width,
                    params,
                ),
            )),
            DisplayReplacementProperty::Media(_) => inputs.media.map(Self::Media),
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

#[derive(Clone, Copy, Debug)]
pub(crate) struct BufferDisplayPropertyTextSourceEvent<'a> {
    value: Value,
    anchor_charpos: CharPos0,
    anchor_bytepos: EmacsBytePos,
    source_text: &'a [u8],
    skip_to: i64,
}

impl<'a> BufferDisplayPropertyTextSourceEvent<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        value: Value,
        text_start_byte: usize,
        text: &'a [u8],
        charpos: i64,
        byte_idx: usize,
        skip_to: i64,
    ) -> Self {
        Self {
            value,
            anchor_charpos: CharPos0::new(charpos.max(0) as usize),
            anchor_bytepos: EmacsBytePos::new(text_start_byte + byte_idx),
            source_text: text.get(byte_idx..).unwrap_or(&[]),
            skip_to,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_anchor(
        value: Value,
        anchor_charpos: CharPos0,
        anchor_bytepos: EmacsBytePos,
        source_text: &'a [u8],
        skip_to: i64,
    ) -> Self {
        Self {
            value,
            anchor_charpos,
            anchor_bytepos,
            source_text,
            skip_to,
        }
    }

    pub(crate) fn value(self) -> Value {
        self.value
    }

    pub(crate) fn anchor_charpos(self) -> CharPos0 {
        self.anchor_charpos
    }

    pub(crate) fn anchor_bytepos(self) -> EmacsBytePos {
        self.anchor_bytepos
    }

    pub(crate) fn source_text(self) -> &'a [u8] {
        self.source_text
    }

    pub(crate) fn skip_to(self) -> i64 {
        self.skip_to
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

enum LispStringAction {
    PopFrame,
    PushReplacement {
        value: Value,
        base_face: RenderFaceRef,
    },
    Emit(DisplayItem),
}

pub(crate) struct LispStringSourceStack {
    frames: Vec<LispStringSourceFrame>,
    next_source_id: u64,
}

impl LispStringSourceStack {
    pub(crate) const fn empty(next_source_id: u64) -> Self {
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

    pub(crate) fn push(&mut self, value: Value, base_face: RenderFaceRef) {
        let source_id = self.allocate_source_id();
        if let Some(frame) = LispStringSourceFrame::new(source_id, value, base_face) {
            self.frames.push(frame);
        }
    }

    pub(crate) fn next_item(
        &mut self,
        context: &mut DisplaySourceContext<'_>,
    ) -> Option<DisplayItem> {
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
    pub(crate) fn source_position(&self) -> DisplaySourcePosition {
        self.frames
            .last()
            .map(LispStringSourceFrame::source_position)
            .unwrap_or_else(|| DisplaySourcePosition::synthetic(0, 0))
    }

    #[allow(dead_code)]
    pub(crate) fn is_empty(&self) -> bool {
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
pub(crate) enum DisplayPropertySourceAction {
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
pub(crate) enum DisplayPropertySourceCursorAction {
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
    pub(crate) fn into_cursor_action(
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

pub(crate) fn display_property_source_action(
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
pub(crate) enum TextSourceCharClassification {
    Text,
    RowBreak,
    ControlChar { ch: char },
    Glyphless { ch: char, method: GlyphlessMethod },
}

pub(crate) fn classify_text_source_char(ch: char) -> TextSourceCharClassification {
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

pub(crate) fn display_item_kind_for_text_source_char(ch: char) -> Option<DisplayItemKind> {
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

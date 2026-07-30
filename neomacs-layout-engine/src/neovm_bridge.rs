//! Bridge between neovm-core data types and the layout engine.
//!
//! Provides functions to build `WindowParams` and `FrameParams` from
//! the Rust Context's state, replacing C FFI data sources.

use std::cmp::Ordering;

use neovm_core::buffer::{
    Buffer, BufferTextSnapshot, CharPos0, EmacsByteLen, EmacsBytePos, EmacsByteRange, LispCharPos1,
    buffer::{BUFFER_SLOT_COUNT, lookup_buffer_slot},
    overlay::OverlayList,
};
use neovm_core::emacs_core::effect_profile::{
    EffectScope, effect_name_from_lisp, effect_operation_from_lisp,
};
use neovm_core::emacs_core::image_catalog::image_scale_environment;
use neovm_core::emacs_core::intern;
use neovm_core::emacs_core::symbol::Obarray;
use neovm_core::emacs_core::value::{ValueKind, eq_value, list_to_vec};
use neovm_core::emacs_core::{Context, Value};
use neovm_core::face::{
    BoxStyle as NeoBoxStyle, Color as NeoColor, Face as NeoFace, FaceDecoration, FaceHeight,
    FaceTable, FontWeight, UnderlineStyle as NeoUnderlineStyle,
};
use neovm_core::window::{
    CursorTypeSymbol, Frame, FrameId, VerticalScrollBarType, Window,
    resolve_window_scroll_bar_geometry,
};

use super::types::{FrameParams, LineWrapMode, VisualCursorSpec, WindowKind, WindowParams};
use crate::coords::{
    clamped_lisp_charpos_to_layout_i64, layout_char_pos_from_i64, layout_emacs_byte_pos_from_i64,
    lisp_char_pos_to_layout_i64, lisp_charpos_to_layout_char_pos,
};
use crate::display_face_policy::BaseFacePolicy;
use crate::display_origin::DisplayOrigin;
use crate::fontconfig::FontSizing;
use neomacs_display_protocol::EffectsConfig;
use neomacs_display_protocol::cursor::{CursorBarWidth, CursorKind, CursorSpec};
use neomacs_display_protocol::face::BoxLineWidth;
use neomacs_display_protocol::types::{FaceId, Rect};
use strum::{EnumString, IntoStaticStr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DisplayLineNumbersMode {
    Off,
    Absolute,
    Relative,
    Visual,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
enum DisplayLineNumbersSymbol {
    Relative,
    Visual,
}

impl DisplayLineNumbersSymbol {
    fn from_symbol_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    #[cfg(test)]
    fn name(self) -> &'static str {
        self.into()
    }
}

impl DisplayLineNumbersMode {
    fn from_lisp_value(value: Option<Value>) -> Self {
        match value {
            Some(v) if v.bits() == Value::T.bits() => Self::Absolute,
            Some(value) => value
                .as_symbol_name()
                .and_then(DisplayLineNumbersSymbol::from_symbol_name)
                .map(|symbol| match symbol {
                    DisplayLineNumbersSymbol::Relative => Self::Relative,
                    DisplayLineNumbersSymbol::Visual => Self::Visual,
                })
                .unwrap_or(Self::Off),
            None => Self::Off,
        }
    }

    pub(crate) fn engine_code(self) -> u8 {
        match self {
            Self::Off => 0,
            Self::Absolute => 1,
            Self::Relative => 2,
            Self::Visual => 3,
        }
    }
}

pub(crate) trait LayoutBufferView {
    fn layout_buffer_local_value(&self, name: &str) -> Option<Value>;
    fn layout_point_min_emacs_byte_pos(&self) -> EmacsBytePos;
    fn layout_point_max_emacs_byte_pos(&self) -> EmacsBytePos;
    fn layout_point_max_char_pos(&self) -> CharPos0;
    fn layout_total_emacs_byte_len(&self) -> EmacsByteLen;
    fn layout_char_pos_to_emacs_byte_pos(&self, charpos: CharPos0) -> EmacsBytePos;
    fn layout_emacs_byte_pos_to_char_pos(&self, bytepos: EmacsBytePos) -> CharPos0;
    fn layout_copy_emacs_byte_range_to(&self, range: EmacsByteRange, out: &mut Vec<u8>);
    fn layout_try_for_each_emacs_byte_range_chunk<E>(
        &self,
        range: EmacsByteRange,
        f: impl FnMut(&[u8]) -> Result<(), E>,
    ) -> Result<(), E>;
    fn layout_emacs_byte_at_pos(&self, pos: EmacsBytePos) -> Option<u8>;
    fn layout_text_prop_at_emacs_byte_pos(&self, pos: EmacsBytePos, name: Value) -> Option<Value>;
    fn layout_next_text_prop_change_after_emacs_byte_pos(
        &self,
        pos: EmacsBytePos,
    ) -> Option<EmacsBytePos>;
    /// Next position after `pos` where the single text property `name` changes
    /// (compared by `eq`), ignoring changes to any other property.  Mirrors the
    /// text-property half of GNU `next_single_char_property_change`.
    fn layout_next_single_text_prop_change_after_emacs_byte_pos(
        &self,
        pos: EmacsBytePos,
        name: Value,
    ) -> Option<EmacsBytePos>;
    /// Bounded variant for display scans that only need the next boundary
    /// within the visible window (invisible/display): stops at `limit` and
    /// returns it as a soft boundary if `name` has not changed by then. Must
    /// NOT be used where the exact extent matters (e.g. mouse-face highlight).
    fn layout_next_single_text_prop_change_after_emacs_byte_pos_bounded(
        &self,
        pos: EmacsBytePos,
        name: Value,
        limit: EmacsBytePos,
    ) -> Option<EmacsBytePos>;
    fn layout_previous_single_text_prop_change_before_emacs_byte_pos(
        &self,
        pos: EmacsBytePos,
        name: Value,
    ) -> Option<EmacsBytePos>;
    fn layout_overlays(&self) -> &OverlayList;
}

#[derive(Clone)]
pub(crate) struct LayoutBufferSnapshot {
    name: String,
    text_snapshot: BufferTextSnapshot,
    accessible_start_emacs_byte: EmacsBytePos,
    accessible_end_emacs_byte: EmacsBytePos,
    accessible_end_char: CharPos0,
    local_var_alist: Value,
    slots: [Value; BUFFER_SLOT_COUNT],
    overlays: OverlayList,
    default_values: Vec<(neovm_core::emacs_core::intern::SymId, Value)>,
}

impl LayoutBufferSnapshot {
    pub fn from_buffer(buffer: &Buffer) -> Self {
        Self {
            name: buffer.name_runtime_string_owned(),
            text_snapshot: buffer.text_snapshot(),
            accessible_start_emacs_byte: buffer.point_min_emacs_byte_pos(),
            accessible_end_emacs_byte: buffer.point_max_emacs_byte_pos(),
            accessible_end_char: buffer.point_max_char_pos(),
            local_var_alist: buffer.local_var_alist_value(),
            slots: buffer.slot_values_snapshot(),
            overlays: buffer.overlays().clone(),
            default_values: Vec::new(),
        }
    }

    pub fn from_buffer_with_obarray(buffer: &Buffer, obarray: &Obarray) -> Self {
        let mut snapshot = Self::from_buffer(buffer);
        snapshot.default_values = layout_default_values(obarray);
        snapshot
    }

    fn default_value(&self, name: &str) -> Option<Value> {
        let id = intern::intern(name);
        self.default_values
            .iter()
            .find_map(|(sym_id, value)| (*sym_id == id).then_some(*value))
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }
}

const LAYOUT_DEFAULT_VALUE_SYMBOLS: &[&str] = &[
    // `char-property-alias-alist` lets a text property (e.g. `display`) be
    // recorded under an aliased key (GNU `lookup_char_property`). It is a plain
    // global (indent-bars mutates the default, not a buffer-local), so the
    // snapshot must capture the default value for the source walk to resolve
    // aliased `display` properties. See `compute_display_property_aliases`.
    "char-property-alias-alist",
    "display-fill-column-indicator",
    "display-fill-column-indicator-character",
    "display-fill-column-indicator-column",
    "display-line-numbers",
    "display-line-numbers-current-absolute",
    "display-line-numbers-major-tick",
    "display-line-numbers-minor-tick",
    "display-line-numbers-offset",
    "display-line-numbers-widen",
    "display-line-numbers-width",
    "face-remapping-alist",
    "line-prefix",
    "neomacs-cursor-effect",
    "neomacs-visual-cursors",
    "show-trailing-whitespace",
    "standard-display-table",
    "tab-stop-list",
    "wrap-prefix",
];

/// The `LAYOUT_DEFAULT_VALUE_SYMBOLS` names as interned ids, computed once.
///
/// `layout_default_values` runs per buffer snapshot per redisplay, and
/// interning all 19 names each time (a hash + obarray probe per name) showed
/// up in a typing profile under the redisplay callback. The names are a
/// compile-time constant, so their ids are too. Caching `SymId`s in a
/// process-global `OnceLock` is the established pattern here -- eval.rs's
/// `cached_symbol_id!` does exactly this for `quote`/`let`/etc. across every
/// Context the test suite creates.
fn layout_default_value_sym_ids() -> &'static [neovm_core::emacs_core::intern::SymId] {
    use std::sync::OnceLock;
    static IDS: OnceLock<Vec<neovm_core::emacs_core::intern::SymId>> = OnceLock::new();
    IDS.get_or_init(|| {
        LAYOUT_DEFAULT_VALUE_SYMBOLS
            .iter()
            .map(|name| intern::intern(name))
            .collect()
    })
}

fn layout_default_values(obarray: &Obarray) -> Vec<(neovm_core::emacs_core::intern::SymId, Value)> {
    layout_default_value_sym_ids()
        .iter()
        .filter_map(|&id| {
            obarray
                .default_value_id(id)
                .copied()
                .map(|value| (id, value))
        })
        .collect()
}

fn find_layout_local_var_alist_entry(alist: Value, key: Value) -> Option<Value> {
    let mut cursor = alist;
    while cursor.is_cons() {
        let entry = cursor.cons_car();
        cursor = cursor.cons_cdr();
        if entry.is_cons() && eq_value(&entry.cons_car(), &key) {
            return Some(entry.cons_cdr());
        }
    }
    None
}

impl LayoutBufferView for Buffer {
    fn layout_buffer_local_value(&self, name: &str) -> Option<Value> {
        self.buffer_local_value(name)
    }

    fn layout_point_min_emacs_byte_pos(&self) -> EmacsBytePos {
        self.point_min_emacs_byte_pos()
    }

    fn layout_point_max_emacs_byte_pos(&self) -> EmacsBytePos {
        self.point_max_emacs_byte_pos()
    }

    fn layout_point_max_char_pos(&self) -> CharPos0 {
        self.point_max_char_pos()
    }

    fn layout_total_emacs_byte_len(&self) -> EmacsByteLen {
        self.total_emacs_byte_len()
    }

    fn layout_char_pos_to_emacs_byte_pos(&self, charpos: CharPos0) -> EmacsBytePos {
        self.char_pos_to_emacs_byte_pos_clamped(charpos)
    }

    fn layout_emacs_byte_pos_to_char_pos(&self, bytepos: EmacsBytePos) -> CharPos0 {
        self.emacs_byte_pos_to_char_pos_clamped(bytepos)
    }

    fn layout_copy_emacs_byte_range_to(&self, range: EmacsByteRange, out: &mut Vec<u8>) {
        self.copy_emacs_byte_range_to(range, out);
    }

    fn layout_try_for_each_emacs_byte_range_chunk<E>(
        &self,
        range: EmacsByteRange,
        f: impl FnMut(&[u8]) -> Result<(), E>,
    ) -> Result<(), E> {
        self.try_for_each_emacs_byte_range_chunk(range, f)
    }

    fn layout_emacs_byte_at_pos(&self, pos: EmacsBytePos) -> Option<u8> {
        self.emacs_byte_at_pos(pos)
    }

    fn layout_text_prop_at_emacs_byte_pos(&self, pos: EmacsBytePos, name: Value) -> Option<Value> {
        self.text_props_get_property_at_emacs_byte_pos(pos, name)
    }

    fn layout_next_text_prop_change_after_emacs_byte_pos(
        &self,
        pos: EmacsBytePos,
    ) -> Option<EmacsBytePos> {
        self.text_props_next_change_after_emacs_byte_pos(pos)
    }

    fn layout_next_single_text_prop_change_after_emacs_byte_pos(
        &self,
        pos: EmacsBytePos,
        name: Value,
    ) -> Option<EmacsBytePos> {
        self.text_props_next_single_change_after_emacs_byte_pos(pos, name)
    }

    fn layout_next_single_text_prop_change_after_emacs_byte_pos_bounded(
        &self,
        pos: EmacsBytePos,
        name: Value,
        limit: EmacsBytePos,
    ) -> Option<EmacsBytePos> {
        self.text_props_next_single_change_after_emacs_byte_pos_bounded(pos, name, limit)
    }

    fn layout_previous_single_text_prop_change_before_emacs_byte_pos(
        &self,
        pos: EmacsBytePos,
        name: Value,
    ) -> Option<EmacsBytePos> {
        self.text_props_previous_single_change_before_emacs_byte_pos(pos, name)
    }

    fn layout_overlays(&self) -> &OverlayList {
        self.overlays()
    }
}

impl LayoutBufferView for LayoutBufferSnapshot {
    fn layout_buffer_local_value(&self, name: &str) -> Option<Value> {
        if let Some(info) = lookup_buffer_slot(name) {
            return Some(self.slots[info.offset.index()]);
        }
        let key = Value::from_sym_id(intern::intern(name));
        find_layout_local_var_alist_entry(self.local_var_alist, key)
            .and_then(|v| (!v.is_unbound()).then_some(v))
            .or_else(|| self.default_value(name))
    }

    fn layout_point_min_emacs_byte_pos(&self) -> EmacsBytePos {
        self.accessible_start_emacs_byte
    }

    fn layout_point_max_emacs_byte_pos(&self) -> EmacsBytePos {
        self.accessible_end_emacs_byte
    }

    fn layout_point_max_char_pos(&self) -> CharPos0 {
        self.accessible_end_char
    }

    fn layout_total_emacs_byte_len(&self) -> EmacsByteLen {
        self.text_snapshot.emacs_byte_len()
    }

    fn layout_char_pos_to_emacs_byte_pos(&self, charpos: CharPos0) -> EmacsBytePos {
        self.text_snapshot
            .char_pos_to_emacs_byte_pos(charpos.min(self.accessible_end_char))
    }

    fn layout_emacs_byte_pos_to_char_pos(&self, bytepos: EmacsBytePos) -> CharPos0 {
        self.text_snapshot
            .emacs_byte_pos_to_char_pos(bytepos.min(self.accessible_end_emacs_byte))
    }

    fn layout_copy_emacs_byte_range_to(&self, range: EmacsByteRange, out: &mut Vec<u8>) {
        self.text_snapshot.copy_emacs_byte_range_to(range, out);
    }

    fn layout_try_for_each_emacs_byte_range_chunk<E>(
        &self,
        range: EmacsByteRange,
        f: impl FnMut(&[u8]) -> Result<(), E>,
    ) -> Result<(), E> {
        self.text_snapshot
            .try_for_each_emacs_byte_range_chunk(range, f)
    }

    fn layout_emacs_byte_at_pos(&self, pos: EmacsBytePos) -> Option<u8> {
        self.text_snapshot.emacs_byte_at_pos(pos)
    }

    fn layout_text_prop_at_emacs_byte_pos(&self, pos: EmacsBytePos, name: Value) -> Option<Value> {
        self.text_snapshot.text_prop_at_emacs_byte_pos(pos, name)
    }

    fn layout_next_text_prop_change_after_emacs_byte_pos(
        &self,
        pos: EmacsBytePos,
    ) -> Option<EmacsBytePos> {
        self.text_snapshot
            .next_text_prop_change_after_emacs_byte_pos(pos)
    }

    fn layout_next_single_text_prop_change_after_emacs_byte_pos(
        &self,
        pos: EmacsBytePos,
        name: Value,
    ) -> Option<EmacsBytePos> {
        self.text_snapshot
            .next_single_text_prop_change_after_emacs_byte_pos(pos, name)
    }

    fn layout_next_single_text_prop_change_after_emacs_byte_pos_bounded(
        &self,
        pos: EmacsBytePos,
        name: Value,
        limit: EmacsBytePos,
    ) -> Option<EmacsBytePos> {
        self.text_snapshot
            .next_single_text_prop_change_after_emacs_byte_pos_bounded(pos, name, limit)
    }

    fn layout_previous_single_text_prop_change_before_emacs_byte_pos(
        &self,
        pos: EmacsBytePos,
        name: Value,
    ) -> Option<EmacsBytePos> {
        self.text_snapshot
            .previous_single_text_prop_change_before_emacs_byte_pos(pos, name)
    }

    fn layout_overlays(&self) -> &OverlayList {
        &self.overlays
    }
}

pub(crate) fn buffer_local_value<B: LayoutBufferView + ?Sized>(
    buffer: &B,
    name: &str,
) -> Option<Value> {
    // GNU `buffer_local_value` (`buffer.c:1359-1413`) returns a buffer's
    // local binding when present and otherwise falls through to the default
    // value.  Layout uses this helper for display variables such as
    // `display-line-numbers-current-absolute`; using the local-only predicate
    // here silently loses global/default display state.
    buffer.layout_buffer_local_value(name)
}

fn effective_buffer_value(buffer: &Buffer, obarray: &Obarray, name: &str) -> Option<Value> {
    buffer
        .buffer_local_value(name)
        .or_else(|| obarray.symbol_value(name).copied())
}

fn frame_parameter_int(frame: &Frame, name: &str, default: i64) -> i64 {
    frame
        .parameter(name)
        .and_then(|v| v.as_int())
        .unwrap_or(default)
}

/// Resolve a face's foreground as an Emacs packed pixel (`0x00RRGGBB`).
///
/// Mirrors the encoding GNU's display layer uses for face colors and
/// matches `NeoColor::from_pixel` (R<<16 | G<<8 | B) which the consumers
/// decode through. `resolve` follows `:inherit` chains, so faces like
/// `nobreak-space` (`:inherit escape-glyph`) report the inherited color.
/// `fallback` is returned when the face leaves its foreground unspecified.
fn face_fg_pixel(face_table: &FaceTable, name: &str, fallback: u32) -> u32 {
    face_table
        .resolve(name)
        .foreground
        .map(|c| (c.r as u32) << 16 | (c.g as u32) << 8 | c.b as u32)
        .unwrap_or(fallback)
}

/// Resolve a face's background as an Emacs packed pixel (`0x00RRGGBB`).
///
/// Sibling of [`face_fg_pixel`]; see it for the encoding and inheritance
/// semantics.
fn face_bg_pixel(face_table: &FaceTable, name: &str, fallback: u32) -> u32 {
    face_table
        .resolve(name)
        .background
        .map(|c| (c.r as u32) << 16 | (c.g as u32) << 8 | c.b as u32)
        .unwrap_or(fallback)
}

/// Build `FrameParams` from a neovm-core `Frame`, reading default face
/// colors from the face table.
pub fn frame_params_from_neovm(
    frame: &Frame,
    face_table: &FaceTable,
    obarray: &Obarray,
) -> FrameParams {
    // Read default face background from face table
    let default_face = face_table.get("default");
    let bg = default_face
        .and_then(|f| f.background)
        .map(|c| (c.r as u32) << 16 | (c.g as u32) << 8 | c.b as u32)
        .unwrap_or(0x00FFFFFF); // white fallback
    let fg = default_face
        .and_then(|f| f.foreground)
        .map(|c| (c.r as u32) << 16 | (c.g as u32) << 8 | c.b as u32)
        .unwrap_or(0x00000000); // black fallback

    FrameParams {
        width: frame.width as f32,
        height: frame.height as f32,
        menu_bar_height: frame.menu_bar_height as f32,
        tool_bar_height: frame.tool_bar_height as f32,
        compact_bar_height: frame.compact_bar_height as f32,
        tab_bar_height: frame.tab_bar_height as f32,
        char_width: frame.char_width,
        char_height: frame.char_height,
        font_pixel_size: frame.font_pixel_size,
        image_scale_environment: image_scale_environment(frame, obarray),
        window_system: frame.effective_window_system().is_some(),
        background: bg,
        vertical_border_fg: face_fg_pixel(face_table, "vertical-border", fg),
        right_divider_width: frame
            .parameter("right-divider-width")
            .and_then(|v| v.as_int())
            .unwrap_or(0)
            .max(0) as i32,
        bottom_divider_width: frame
            .parameter("bottom-divider-width")
            .and_then(|v| v.as_int())
            .unwrap_or(0)
            .max(0) as i32,
        divider_fg: face_fg_pixel(face_table, "window-divider", fg),
        divider_first_fg: face_fg_pixel(face_table, "window-divider-first-pixel", fg),
        divider_last_fg: face_fg_pixel(face_table, "window-divider-last-pixel", fg),
    }
}

/// Helper: extract an integer buffer-local variable.
pub(crate) fn buffer_local_int<B: LayoutBufferView>(buffer: &B, name: &str, default: i64) -> i64 {
    match buffer_local_value(buffer, name) {
        Some(v) if v.is_fixnum() => v.as_fixnum().unwrap(),
        _ => default,
    }
}

fn effective_buffer_int(buffer: &Buffer, obarray: &Obarray, name: &str, default: i64) -> i64 {
    match effective_buffer_value(buffer, obarray, name) {
        Some(v) if v.is_fixnum() => v.as_fixnum().unwrap(),
        _ => default,
    }
}

/// Helper: extract a boolean buffer-local variable (nil = false, anything else = true).
pub(crate) fn buffer_local_bool<B: LayoutBufferView>(buffer: &B, name: &str) -> bool {
    match buffer_local_value(buffer, name) {
        Some(v) if v.is_nil() => false,
        None => false,
        Some(_) => true,
    }
}

fn effective_buffer_bool(buffer: &Buffer, obarray: &Obarray, name: &str) -> bool {
    match effective_buffer_value(buffer, obarray, name) {
        Some(v) if v.is_nil() => false,
        None => false,
        Some(_) => true,
    }
}

fn value_non_nil(value: Option<Value>) -> bool {
    value.is_some_and(|value| !value.is_nil())
}

fn value_is_symbol(value: Option<Value>, name: &str) -> bool {
    value.is_some_and(|value| value.as_symbol_name() == Some(name))
}

fn window_parameter_by_name(window: &Window, name: &str) -> Option<Value> {
    window
        .parameters()
        .iter()
        .find(|(key, _)| key.as_symbol_name() == Some(name))
        .map(|(_, value)| *value)
}

fn window_wants_mode_line(
    window: &Window,
    buffer: &Buffer,
    frame: &Frame,
    obarray: &Obarray,
    is_minibuffer: bool,
) -> bool {
    let window_mode_line_format = window_parameter_by_name(window, "mode-line-format");
    window.is_leaf()
        && !is_minibuffer
        && !value_is_symbol(window_mode_line_format, "none")
        && (value_non_nil(window_mode_line_format)
            || value_non_nil(effective_buffer_value(buffer, obarray, "mode-line-format")))
        && window.bounds().height > frame.char_height
}

fn window_wants_header_line(
    window: &Window,
    buffer: &Buffer,
    frame: &Frame,
    obarray: &Obarray,
    is_minibuffer: bool,
    wants_mode_line: bool,
) -> bool {
    let window_header_line_format = window_parameter_by_name(window, "header-line-format");
    let required_rows = if wants_mode_line { 2.0 } else { 1.0 };
    window.is_leaf()
        && !is_minibuffer
        && !value_is_symbol(window_header_line_format, "none")
        && (value_non_nil(window_header_line_format)
            || value_non_nil(effective_buffer_value(
                buffer,
                obarray,
                "header-line-format",
            )))
        && window.bounds().height > required_rows * frame.char_height
}

fn window_wants_tab_line(
    window: &Window,
    buffer: &Buffer,
    frame: &Frame,
    obarray: &Obarray,
    is_minibuffer: bool,
    wants_mode_line: bool,
    wants_header_line: bool,
) -> bool {
    let window_tab_line_format = window_parameter_by_name(window, "tab-line-format");
    let required_rows = (if wants_mode_line { 1.0 } else { 0.0 })
        + (if wants_header_line { 1.0 } else { 0.0 })
        + 1.0;
    window.is_leaf()
        && !is_minibuffer
        && !value_is_symbol(window_tab_line_format, "none")
        && (value_non_nil(window_tab_line_format)
            || value_non_nil(effective_buffer_value(buffer, obarray, "tab-line-format")))
        && window.bounds().height > required_rows * frame.char_height
}

fn global_bool(obarray: &Obarray, name: &str) -> bool {
    obarray
        .symbol_value(name)
        .is_some_and(|value| !value.is_nil())
}

fn effective_nobreak_char_display(buffer: &Buffer, obarray: &Obarray) -> i32 {
    // `nobreak-char-display` is a `DEFVAR_LISP`, but GNU reads `Vnobreak_char_display`
    // while displaying buffer text (xdisp.c:8522, with the window's buffer current),
    // so a buffer-local binding takes effect. Read buffer-local-then-global, not the
    // raw global.
    match effective_buffer_value(buffer, obarray, "nobreak-char-display") {
        Some(value) if value.is_nil() => 0,
        Some(value) if value.as_int() == Some(2) => 2,
        Some(_) => 1,
        None => 0,
    }
}

fn frame_total_cols(frame: &Frame) -> i64 {
    frame
        .parameter("width")
        .and_then(|value| value.as_int())
        .unwrap_or(frame.columns() as i64)
}

fn window_total_cols(window: &Window, char_width: f32) -> i64 {
    let width = window.bounds().width;
    if char_width > 0.0 {
        (width / char_width) as i64
    } else {
        0
    }
}

fn effective_wrap_mode(
    window: &Window,
    buffer: &Buffer,
    frame: &Frame,
    obarray: &Obarray,
    hscroll: usize,
) -> LineWrapMode {
    if effective_buffer_bool(buffer, obarray, "truncate-lines") {
        return LineWrapMode::Truncate;
    }

    // GNU `xdisp.c:init_iterator` only enables wrapping when the
    // window is not horizontally scrolled.
    if hscroll != 0 {
        return LineWrapMode::Truncate;
    }

    let total_cols = window_total_cols(window, frame.char_width);
    let frame_cols = frame_total_cols(frame);

    if total_cols >= frame_cols {
        return LineWrapMode::Wrap;
    }

    let truncate = match effective_buffer_value(buffer, obarray, "truncate-partial-width-windows") {
        Some(value) if value.is_nil() => false,
        Some(value) if value.is_fixnum() => total_cols < value.as_fixnum().unwrap(),
        Some(_) => true,
        None => false,
    };

    if truncate {
        LineWrapMode::Truncate
    } else {
        LineWrapMode::Wrap
    }
}

fn chrome_face_pixel_height(face: &ResolvedFace, fallback_char_height: f32) -> f32 {
    // GNU Emacs frame.c:1184-1185 — non-window (TTY) frames have
    //   f->column_width = 1;
    //   f->line_height  = 1;
    // and chrome rows (mode-line, header-line, tab-line) are exactly
    // one character cell tall. Face font_line_height is a GUI pixel
    // measurement and must not contribute to row sizing on a TTY
    // frame: `fallback_char_height` is set to 1.0 by
    // `bootstrap_buffers` (main.rs:1691-1694) when the frame is a
    // TTY, so detect the TTY context by the 1.0-cell marker and
    // return the cell height directly.
    //
    // Without this early return, a mode-line face with a non-zero
    // `font_line_height` (e.g. 3 from the realized Hack font under
    // cosmic-text) produced a 3-row-tall mode-line region on TTY.
    // The mode-line text painted on the first row and the remaining
    // two rows rendered as blank padding, which looked like the
    // echo area having "3 lines" instead of GNU's single row.
    if fallback_char_height <= 1.0 {
        return fallback_char_height.max(1.0);
    }
    let line_height = if face.font_line_height > 0.0 {
        face.font_line_height.ceil()
    } else {
        fallback_char_height.ceil()
    };
    let box_pixels = if face.box_type != 0 {
        2.0 * face.box_line_width.row_expansion_per_edge() as f32
    } else {
        0.0
    };
    (line_height + box_pixels).max(1.0)
}

pub(crate) fn buffer_local_list_values<B: LayoutBufferView>(buffer: &B, name: &str) -> Vec<Value> {
    // `list_to_vec' takes `&Value'; feed the borrowed form since
    // `buffer_local_value' returns the `Copy' `Value' by value.
    buffer_local_value(buffer, name)
        .and_then(|v| list_to_vec(&v))
        .unwrap_or_default()
}

/// Resolve a LOGICAL fringe indicator (`empty-line`, `truncation`,
/// `continuation`, `up`, `down`, …) to a concrete fringe-bitmap registry index,
/// honoring `fringe-indicator-alist`. Direct port of GNU
/// `get_logical_fringe_bitmap` (`src/fringe.c`).
///
/// `right_p` selects the right vs left element; `partial_p` selects the
/// partial vs full element. The alist `cdr` for an indicator is either:
///   * a BARE SYMBOL — used for every (left/right/partial) case, or
///   * a list `(LEFT RIGHT [PARTIAL-LEFT PARTIAL-RIGHT])` — indexed by
///     `right_p ? (partial_p ? 3 : 1) : (partial_p ? 2 : 0)`, falling back to
///     the non-partial element (`ix1 = right_p`) when a partial element is
///     absent.
///
/// An element equal to `t` means "no bitmap here" (skip to the fallback). The
/// buffer-local alist is consulted first, then the global/default value.
///
/// Returns `None` when no bitmap applies (the `t`/missing/unregistered cases),
/// matching GNU's `NO_FRINGE_BITMAP`.
pub(crate) fn resolve_fringe_indicator_bitmap_index<B: LayoutBufferView>(
    buffer: &B,
    ctx: &Context,
    logical_sym: Value,
    right_p: bool,
    partial_p: bool,
) -> Option<u16> {
    let ix1 = usize::from(right_p);
    let ix2 = ix1 + if partial_p { 2 } else { 0 };

    // Look up the cdr of (LOGICAL . SPEC) in an alist value.
    fn assq_cdr(alist: Value, key: Value) -> Option<Value> {
        let mut cursor = alist;
        while cursor.is_cons() {
            let entry = cursor.cons_car();
            if entry.is_cons() && eq_value(&entry.cons_car(), &key) {
                return Some(entry.cons_cdr());
            }
            cursor = cursor.cons_cdr();
        }
        None
    }

    // From a resolved cdr SPEC, pick the bitmap SYMBOL for this (right/partial)
    // case. `t` (or out-of-range) yields `None`; a bare symbol is used directly.
    // When `partial_p` and the partial element is absent, fall back to `ix1`.
    fn pick_bitmap_symbol(spec: Value, ix1: usize, ix2: usize, partial_p: bool) -> Option<Value> {
        if spec.is_nil() {
            // GNU: a present-but-nil cdr means NO_FRINGE_BITMAP for the whole
            // indicator (handled by the caller treating `None` as "stop").
            return None;
        }
        if !spec.is_cons() {
            // BARE SYMBOL: used for all cases unless it is `t`.
            return (spec.bits() != Value::T.bits()).then_some(spec);
        }
        let items = list_to_vec(&spec)?;
        // Prefer the partial element when requested and present & not `t`.
        if partial_p
            && let Some(elem) = items.get(ix2).copied()
            && elem.bits() != Value::T.bits()
        {
            return Some(elem);
        }
        // Non-partial (or partial-absent) fallback: the ix1 element.
        let elem = items.get(ix1).copied()?;
        (elem.bits() != Value::T.bits()).then_some(elem)
    }

    let registry = ctx.fringe_bitmap_registry();
    let resolve_index = |sym: Value| -> Option<u16> {
        let sym_id = sym.as_symbol_id()?;
        let index = registry.index_of(sym_id)?;
        u16::try_from(index).ok()
    };

    // 1. Buffer-local `fringe-indicator-alist`.
    let local = buffer.layout_buffer_local_value("fringe-indicator-alist");
    // 2. Global/default value (GNU `BVAR(&buffer_defaults, fringe_indicator_alist)`).
    // `fringe-indicator-alist` is a forwarded per-buffer slot, so its default
    // lives in `buffer_defaults` — the obarray value cell is always nil.
    let global = ctx.buffer_default_value("fringe-indicator-alist");

    // Try the buffer-local binding first. GNU only falls through to the global
    // value when the local lookup yields no usable element (a `t` element or a
    // missing partial spec); a present non-`t` element short-circuits.
    if let Some(local) = local.filter(|v| !v.is_nil())
        && let Some(cdr) = assq_cdr(local, logical_sym)
    {
        if cdr.is_nil() {
            // Explicit nil cdr => NO_FRINGE_BITMAP for this indicator.
            return None;
        }
        if let Some(sym) = pick_bitmap_symbol(cdr, ix1, ix2, partial_p) {
            return resolve_index(sym);
        }
    }

    // Fall back to the global/default alist.
    let global = global.filter(|v| !v.is_nil())?;
    let cdr = assq_cdr(global, logical_sym)?;
    if cdr.is_nil() {
        return None;
    }
    let sym = pick_bitmap_symbol(cdr, ix1, ix2, partial_p)?;
    resolve_index(sym)
}

pub(crate) fn buffer_display_line_numbers_mode<B: LayoutBufferView>(
    buffer: &B,
) -> DisplayLineNumbersMode {
    DisplayLineNumbersMode::from_lisp_value(buffer_local_value(buffer, "display-line-numbers"))
}

fn buffer_fill_column_indicator(buffer: &Buffer, obarray: &Obarray) -> Option<(i32, char)> {
    // GNU `fill_column_indicator_column` in xdisp.c enables the indicator only
    // when `display-fill-column-indicator` is non-nil, the indicator character
    // satisfies CHARACTERP, and the effective column is a nonnegative integer.
    // Read the EFFECTIVE value (buffer-local, else global/default) for all four:
    // `display-fill-column-indicator-column` in particular defaults to `t` (use
    // fill-column) and is rarely set locally, so a buffer-local-only read returns
    // None and disables the indicator even when the mode is on.
    if !effective_buffer_bool(buffer, obarray, "display-fill-column-indicator") {
        return None;
    }

    let character_value =
        effective_buffer_value(buffer, obarray, "display-fill-column-indicator-character")?;
    if !character_value.is_char() {
        return None;
    }
    let character = character_value.as_char()?;

    let column_value =
        match effective_buffer_value(buffer, obarray, "display-fill-column-indicator-column") {
            Some(value) if value.bits() == Value::T.bits() => {
                effective_buffer_value(buffer, obarray, "fill-column")
            }
            value => value,
        }?;
    let column = column_value.as_fixnum()?;
    if column < 0 || column > i32::MAX as i64 {
        return None;
    }

    Some((column as i32, character))
}

/// Index of the selective-display / invisible ellipsis slot in a display
/// table's extra slots (`DISP_INVIS_VECTOR`, `disptab.h:35` -> `extras[4]`).
const DISP_INVIS_VECTOR_SLOT: usize = 4;

/// Decode a display-table glyph code to its character.
///
/// `make-glyph-code` (`disp-table.el`) encodes a glyph as either a bare
/// character fixnum, a fixnum packing the face id above the low 22 char bits,
/// or a `(char . face-id)` cons when the face id needs more than 6 bits.  Here
/// we only need the character; the face is currently rendered with the active
/// (preceding text) face, matching GNU's common `setup_for_ellipsis` path
/// where the glyph carries the default face.
fn glyph_code_char(glyph: Value) -> Option<char> {
    let code = if glyph.is_cons() {
        glyph.cons_car().as_fixnum()?
    } else {
        glyph.as_fixnum()?
    };
    char::from_u32((code as u64 & 0x3F_FFFF) as u32)
}

/// The active display table for a buffer: `buffer-display-table`, else
/// `standard-display-table` (GNU `disp_char_vector`'s `dp` argument selection).
/// Per-window display tables are not yet wired into the layout buffer view.
fn active_buffer_display_table<B: LayoutBufferView + ?Sized>(buffer: &B) -> Option<Value> {
    buffer_local_value(buffer, "buffer-display-table")
        .filter(|v| !v.is_nil())
        .or_else(|| buffer_local_value(buffer, "standard-display-table"))
        .filter(|v| !v.is_nil())
}

/// Resolve the ellipsis string GNU renders for invisible/selective-display
/// folds from the active display table's `DISP_INVIS_VECTOR` slot.
///
/// GNU `setup_for_ellipsis` (`xdisp.c:5654`) uses the buffer's display table
/// (`buffer-display-table`, else `standard-display-table`) selective-display
/// slot when it holds a vector of glyph codes, and otherwise falls back to the
/// default three `.` glyphs.  We mirror the character selection; returning
/// `None` lets the caller use the hard-coded `"..."` default.
pub(crate) fn buffer_invisible_ellipsis_text<B: LayoutBufferView>(buffer: &B) -> Option<String> {
    let table = active_buffer_display_table(buffer)?;
    let slot = table
        .as_char_table_obj()?
        .extras
        .get(DISP_INVIS_VECTOR_SLOT)
        .copied()?;
    let glyphs = slot.as_vector_data()?;
    if glyphs.is_empty() {
        return None;
    }
    let text: String = glyphs.iter().filter_map(|g| glyph_code_char(*g)).collect();
    (!text.is_empty()).then_some(text)
}

/// Resolve the per-character display-vector for `ch` from the active display
/// table, returning the decoded glyph characters to render in place of `ch`.
///
/// Mirrors GNU `get_next_display_element` (`xdisp.c:8463`): `dv =
/// DISP_CHAR_VECTOR(it->dp, c)`; when `dv` is a non-empty vector the character
/// is displayed as the sequence of glyph codes in the vector (each decoded by
/// `GLYPH_CODE_CHAR = code & MAX_CHAR`), all sharing the original char's buffer
/// position.  We return the decoded glyph string so the caller can emit it as a
/// single `SourceMappedText` item over the one source char (the whole vector is
/// one display item, consumed once — matching GNU's `dpvec_char_len`-once
/// advance), and so a `?\t` glyph inside the vector re-expands through the
/// normal tab path.
///
/// Returns `None` (the hot path) when there is no active display table, no
/// entry for `ch`, or the entry is not a vector — leaving `ch` to render
/// literally.  An EMPTY vector means "display nothing"; we return `Some("")`
/// so the char is replaced by no glyphs (GNU skips it entirely).
///
/// Per-glyph faces (`GLYPH_CODE_FACE = code >> 22`) are not yet honored here:
/// the whole mapped vector inherits the source char's active face, which is
/// correct for the common `make-glyph-code` form with no face (face id 0).
pub(crate) fn buffer_display_table_glyphs<B: LayoutBufferView + ?Sized>(
    buffer: &B,
    ch: char,
) -> Option<String> {
    let table = active_buffer_display_table(buffer)?;
    let entry = neovm_core::emacs_core::chartable::ct_ref(&table, ch as i64);
    let glyphs = entry.as_vector_data()?;
    Some(glyphs.iter().filter_map(|g| glyph_code_char(*g)).collect())
}

pub(crate) fn buffer_selective_display<B: LayoutBufferView>(buffer: &B) -> i32 {
    match buffer_local_value(buffer, "selective-display") {
        Some(v) if v.is_fixnum() => v.as_fixnum().unwrap() as i32,
        Some(v) if v.bits() == Value::T.bits() => i32::MAX,
        _ => 0,
    }
}

fn parse_color_pixel(value: &Value) -> Option<u32> {
    value
        .as_runtime_string_owned()
        .or_else(|| value.as_symbol_name().map(str::to_string))
        .and_then(|spec| NeoColor::parse(&spec))
        .map(|color| color_to_pixel(&color))
}

fn parse_cursor_spec(value: &Value) -> Option<CursorSpec> {
    if value.is_nil() {
        return Some(CursorSpec::no_cursor());
    }

    if value.bits() == Value::T.bits() {
        return Some(CursorSpec::filled_box());
    }
    if let Some(cursor_type) = CursorTypeSymbol::from_symbol_value(value) {
        return Some(match cursor_type {
            CursorTypeSymbol::Box => CursorSpec::filled_box(),
            CursorTypeSymbol::Hollow => CursorSpec::hollow_box(),
            CursorTypeSymbol::Bar => CursorSpec::bar(CursorBarWidth::TWO),
            CursorTypeSymbol::Hbar => CursorSpec::hbar(CursorBarWidth::TWO),
        });
    }
    if value.is_cons() {
        let car = value.cons_car();
        let cdr = value.cons_cdr();
        if let (Some(cursor_type), Some(bar_width)) = (
            CursorTypeSymbol::from_symbol_value(&car),
            cdr.as_fixnum().and_then(CursorBarWidth::from_lisp_fixnum),
        ) && cursor_type.accepts_width_tail()
        {
            return Some(match cursor_type {
                CursorTypeSymbol::Box => CursorSpec::new(CursorKind::FilledBox, bar_width),
                CursorTypeSymbol::Bar => CursorSpec::bar(bar_width),
                CursorTypeSymbol::Hbar => CursorSpec::hbar(bar_width),
                CursorTypeSymbol::Hollow => unreachable!("hollow does not accept a width tail"),
            });
        }
    }

    Some(CursorSpec::hollow_box())
}

fn frame_cursor_spec(frame: &Frame) -> CursorSpec {
    frame
        .parameter("cursor-type")
        .and_then(|value| parse_cursor_spec(&value))
        .unwrap_or(CursorSpec::filled_box())
}

fn default_cursor_color_pixel(face_table: &FaceTable) -> u32 {
    face_table
        .resolve("cursor")
        .background
        .or_else(|| face_table.resolve("default").foreground)
        .map(|color| color_to_pixel(&color))
        .unwrap_or(0x000000)
}

fn frame_background_color_pixel(frame: &Frame, face_table: &FaceTable) -> u32 {
    frame
        .parameter("background-color")
        .and_then(|value| parse_color_pixel(&value))
        .or_else(|| {
            face_table
                .resolve("default")
                .background
                .map(|color| color_to_pixel(&color))
        })
        .unwrap_or(0x00ffffff)
}

fn frame_foreground_color_pixel(frame: &Frame, face_table: &FaceTable) -> u32 {
    frame
        .parameter("foreground-color")
        .and_then(|value| parse_color_pixel(&value))
        .or_else(|| {
            face_table
                .resolve("default")
                .foreground
                .map(|color| color_to_pixel(&color))
        })
        .unwrap_or(0x000000)
}

fn frame_mouse_color_pixel(frame: &Frame) -> u32 {
    frame
        .parameter("mouse-color")
        .and_then(|value| parse_color_pixel(&value))
        .unwrap_or(0x000000)
}

fn frame_cursor_color_pixel(frame: &Frame, face_table: &FaceTable) -> u32 {
    let pixel = frame
        .parameter("cursor-color")
        .and_then(|value| parse_color_pixel(&value))
        .unwrap_or_else(|| default_cursor_color_pixel(face_table));

    // GNU GUI ports resolve `cursor-color` through x_set_cursor_color
    // (xfns.c): when the requested cursor pixel equals the frame background,
    // the actual physical cursor pixel falls back to the mouse pixel so an
    // empty-line or end-of-line filled box remains visible.  TTY frames keep
    // the terminal cursor color sentinel path, so only apply this to GUI
    // frames.
    if frame.effective_window_system().is_none() {
        return pixel;
    }

    let background = frame_background_color_pixel(frame, face_table);
    if pixel != background {
        return pixel;
    }

    let mouse = frame_mouse_color_pixel(frame);
    if mouse != background {
        return mouse;
    }

    // GNU first falls back to the mouse pixel. A dark child frame can make
    // cursor, mouse, and background all identical, however. Keep the stronger
    // renderer invariant that a filled cursor background remains visible on
    // an empty EOL/EOB slot, where there is no glyph foreground to rescue it.
    frame_foreground_color_pixel(frame, face_table)
}

fn cursor_effect_name_from_symbol(value: Value) -> Option<String> {
    effect_name_from_lisp(value, EffectScope::Cursor).ok()
}

fn apply_cursor_effect_form(effects: &mut EffectsConfig, form: Value) -> bool {
    if form.is_nil() {
        return false;
    }
    let Ok(operation) = effect_operation_from_lisp(form, EffectScope::Cursor) else {
        return false;
    };
    let Ok(updated) = effects.apply_effects(&[operation]) else {
        return false;
    };
    *effects = updated;
    true
}

fn parse_cursor_effect_profile(value: Value) -> Option<EffectsConfig> {
    if value.is_nil() {
        return None;
    }
    let mut effects = EffectsConfig::cursor_profile_baseline();
    if value.is_cons() {
        let forms = list_to_vec(&value)?;
        let is_single_command = forms
            .first()
            .is_some_and(|head| cursor_effect_name_from_symbol(*head).is_some());
        if is_single_command {
            apply_cursor_effect_form(&mut effects, value).then_some(effects)
        } else {
            let mut any = false;
            for form in forms {
                if !apply_cursor_effect_form(&mut effects, form) {
                    return None;
                }
                any = true;
            }
            any.then_some(effects)
        }
    } else {
        apply_cursor_effect_form(&mut effects, value).then_some(effects)
    }
}

fn parse_visual_cursor_spec(
    value: Value,
    index: usize,
    default_color: u32,
) -> Option<VisualCursorSpec> {
    let items = list_to_vec(&value)?;
    let mut charpos: Option<i64> = None;
    let mut cursor_type = Value::symbol("bar");
    let mut color = default_color;
    let mut effects = None;

    let mut iter = items.chunks_exact(2);
    for pair in &mut iter {
        let key = pair[0].as_symbol_name()?;
        let value = pair[1];
        match key {
            ":position" | ":pos" => {
                charpos = value.as_int().map(clamped_lisp_charpos_to_layout_i64);
            }
            ":cursor-type" | ":type" => {
                cursor_type = value;
            }
            ":color" => {
                if let Some(pixel) = parse_color_pixel(&value) {
                    color = pixel;
                }
            }
            ":effect" | ":effects" => {
                effects = parse_cursor_effect_profile(value);
            }
            _ => {}
        }
    }
    if !iter.remainder().is_empty() {
        return None;
    }

    let cursor = parse_cursor_spec(&cursor_type)?;
    Some(VisualCursorSpec {
        id: -1_000_000 - index as i32,
        charpos: charpos?,
        cursor_kind: cursor.cursor_kind,
        cursor_bar_width: cursor.bar_width,
        color,
        effects,
    })
}

fn parse_visual_cursors(buffer: &Buffer, default_color: u32) -> Vec<VisualCursorSpec> {
    let Some(value) = buffer_local_value(buffer, "neomacs-visual-cursors") else {
        return Vec::new();
    };
    let Some(items) = list_to_vec(&value) else {
        return Vec::new();
    };
    items
        .into_iter()
        .enumerate()
        .filter_map(|(index, item)| parse_visual_cursor_spec(item, index, default_color))
        .collect()
}

fn effective_cursor_spec(
    frame: &Frame,
    buffer: &Buffer,
    is_selected: bool,
    is_minibuffer: bool,
    window_cursor_type: Value,
) -> Option<CursorSpec> {
    let base = if window_cursor_type.bits() != Value::T.bits() {
        parse_cursor_spec(&window_cursor_type)
    } else if let Some(buffer_cursor_type) = buffer_local_value(buffer, "cursor-type") {
        if buffer_cursor_type.bits() == Value::T.bits() {
            Some(frame_cursor_spec(frame))
        } else {
            parse_cursor_spec(&buffer_cursor_type)
        }
    } else {
        Some(frame_cursor_spec(frame))
    }?;

    if is_selected {
        return Some(base);
    }

    if is_minibuffer {
        return None;
    }

    let alt_cursor = buffer_local_value(buffer, "cursor-in-non-selected-windows");
    if let Some(value) = alt_cursor
        && value.bits() != Value::T.bits()
    {
        return parse_cursor_spec(&value);
    }

    // GNU `xdisp.c::get_window_cursor_type` applies the non-selected
    // fallback after resolving the base cursor kind: FilledBox becomes
    // HollowBox, explicit alternate cursor types win, and BAR cursors
    // narrow by one pixel when `cursor-in-non-selected-windows` is `t`.
    let mut adjusted = base;
    if adjusted.cursor_kind == CursorKind::FilledBox {
        adjusted.cursor_kind = CursorKind::HollowBox;
    } else if adjusted.cursor_kind == CursorKind::Bar {
        adjusted.bar_width = adjusted.bar_width.narrowed_for_non_selected_bar();
    }
    Some(adjusted)
}

/// Build `WindowParams` from neovm-core window + buffer + frame data.
///
/// `is_selected` indicates whether this window is the frame's selected window.
/// `is_minibuffer` indicates whether this is the minibuffer window.
///
/// Returns `None` for internal (non-leaf) windows.
pub fn window_params_from_neovm(
    window: &Window,
    buffer: &Buffer,
    frame: &Frame,
    obarray: &Obarray,
    face_table: &FaceTable,
    default_font_ascent: Option<f32>,
    is_selected: bool,
    is_minibuffer: bool,
    window_cursor_type: Value,
    window_cursor_effect: Value,
) -> Option<WindowParams> {
    window_params_from_neovm_with_font_sizing(
        window,
        buffer,
        frame,
        obarray,
        face_table,
        default_font_ascent,
        is_selected,
        is_minibuffer,
        window_cursor_type,
        window_cursor_effect,
        FontSizing::xft(),
    )
}

pub fn window_params_from_neovm_with_font_sizing(
    window: &Window,
    buffer: &Buffer,
    frame: &Frame,
    obarray: &Obarray,
    face_table: &FaceTable,
    default_font_ascent: Option<f32>,
    is_selected: bool,
    is_minibuffer: bool,
    window_cursor_type: Value,
    window_cursor_effect: Value,
    font_sizing: FontSizing,
) -> Option<WindowParams> {
    // Only leaf windows can be laid out.
    let effective_window_system = frame.effective_window_system();
    let is_window_system = effective_window_system.is_some();
    let window_system =
        effective_window_system.and_then(|v| v.as_symbol_name().map(|s| s.to_string()));

    let (
        win_id,
        _buf_id,
        bounds,
        window_start,
        force_start,
        window_end_pos,
        window_end_valid,
        point,
        hscroll,
        vscroll,
        margins,
        left_fringe_width,
        right_fringe_width,
    ) = match window {
        Window::Leaf {
            id,
            buffer_id,
            bounds,
            window_start,
            force_start,
            window_end_pos,
            window_end_valid,
            point,
            hscroll,
            vscroll,
            margins,
            display,
            ..
        } => (
            *id,
            *buffer_id,
            bounds,
            *window_start,
            *force_start,
            *window_end_pos,
            *window_end_valid,
            *point,
            *hscroll,
            *vscroll,
            *margins,
            // Mirrors GNU window_body_width (window.c:1109-1111):
            //   - (FRAME_WINDOW_P (f) ? WINDOW_FRINGES_WIDTH (w) : 0)
            // Fringes only subtract from the text area on GUI frames.
            // TTY frames always have 0 fringes regardless of the
            // `left-fringe` / `right-fringe` frame parameter values.
            if is_window_system {
                if display.left_fringe_width >= 0 {
                    display.left_fringe_width
                } else {
                    frame_parameter_int(frame, "left-fringe", 8) as i32
                }
            } else {
                0
            },
            if is_window_system {
                if display.right_fringe_width >= 0 {
                    display.right_fringe_width
                } else {
                    frame_parameter_int(frame, "right-fringe", 8) as i32
                }
            } else {
                0
            },
        ),
        Window::Internal { .. } => return None,
    };

    let char_width = frame.char_width;
    let char_height = frame.char_height;
    // One authority for the default face's realized pixels, shared with the
    // image builtins so a spec keys the image cache identically from Lisp and
    // from layout (GNU: `lookup_image` via `DEFAULT_FACE_ID`).
    let (default_fg, default_bg) = face_table.default_face_colors();
    let face_resolver = FaceResolver::new_with_font_sizing(
        face_table,
        default_fg,
        default_bg,
        frame.font_pixel_size,
        window_system,
        font_sizing,
    );

    // Convert neovm-core Rect to display Rect (same fields, different types).
    let display_bounds = Rect::new(bounds.x, bounds.y, bounds.width, bounds.height);

    let scroll_bar_geometry = window
        .display()
        .map(|display| resolve_window_scroll_bar_geometry(frame, display, is_minibuffer))
        .unwrap_or_default();
    let vertical_scroll_bar_side = scroll_bar_geometry
        .vertical_type
        .and_then(|value| VerticalScrollBarType::from_symbol_value(&value))
        .map(|side| side.name().to_string());
    let left_sb = scroll_bar_geometry.left_area_width.max(0) as f32;
    let right_sb = scroll_bar_geometry.right_area_width.max(0) as f32;
    let scroll_bar_pixel_width = left_sb.max(right_sb);
    let scroll_bar_pixel_height = scroll_bar_geometry.horizontal_area_height.max(0) as f32;
    let horizontal_scroll_bar = scroll_bar_pixel_height > 0.0;

    // Compute text bounds (bounds minus scroll bars, fringes, and margins).
    let left_fringe = left_fringe_width.max(0) as f32;
    let right_fringe = right_fringe_width.max(0) as f32;
    let left_margin = margins.left() as f32 * char_width;
    let right_margin = margins.right() as f32 * char_width;
    let text_x = bounds.x + left_sb + left_fringe + left_margin;
    let text_width = (bounds.width
        - left_sb
        - right_sb
        - left_fringe
        - right_fringe
        - left_margin
        - right_margin)
        .max(0.0);
    let text_bounds = Rect::new(text_x, bounds.y, text_width, bounds.height);

    // Read buffer-local variables.
    let wrap_mode = effective_wrap_mode(window, buffer, frame, obarray, hscroll);
    let word_wrap = effective_buffer_bool(buffer, obarray, "word-wrap");
    let tab_width = effective_buffer_int(buffer, obarray, "tab-width", 8) as i32;
    // GNU `try_scrolling` reads `scroll-conservatively` / `scroll-margin`
    // (buffer-local with a global fallback) to choose between minimal scrolling
    // and recentering point when point jumps off-screen (src/xdisp.c).
    let scroll_conservatively = effective_buffer_int(buffer, obarray, "scroll-conservatively", 0);
    let scroll_step = effective_buffer_int(buffer, obarray, "scroll-step", 0);
    let scroll_minibuffer_conservatively = obarray
        .symbol_value("scroll-minibuffer-conservatively")
        .is_none_or(|value| !value.is_nil());
    let scroll_margin = effective_buffer_int(buffer, obarray, "scroll-margin", 0);

    // GNU window.c gates chrome reservation through window_wants_*:
    // a mode/header/tab line is shown only for leaf non-minibuffer
    // windows whose window parameter is not `none`, whose window
    // parameter or buffer-local format is non-nil, and whose window is
    // high enough to hold the requested chrome.
    let wants_mode_line = window_wants_mode_line(window, buffer, frame, obarray, is_minibuffer);
    let wants_header_line = window_wants_header_line(
        window,
        buffer,
        frame,
        obarray,
        is_minibuffer,
        wants_mode_line,
    );
    let wants_tab_line = window_wants_tab_line(
        window,
        buffer,
        frame,
        obarray,
        is_minibuffer,
        wants_mode_line,
        wants_header_line,
    );

    // GNU xdisp.c's estimate_mode_line_height starts from the frame line
    // height and lets realized face metrics grow from there.
    let mode_line_height = if wants_mode_line {
        chrome_face_pixel_height(
            &face_resolver.default_base_face_for_origin_without_buffer(&DisplayOrigin::ModeLine {
                selected: is_selected,
            }),
            char_height,
        )
    } else {
        0.0
    };

    let cursor_spec = effective_cursor_spec(
        frame,
        buffer,
        is_selected,
        is_minibuffer,
        window_cursor_type,
    )
    .unwrap_or(CursorSpec {
        cursor_kind: CursorKind::NoCursor,
        bar_width: CursorBarWidth::DEFAULT,
    });
    let cursor_color = frame_cursor_color_pixel(frame, face_table);
    let cursor_effects = parse_cursor_effect_profile(window_cursor_effect).or_else(|| {
        buffer_local_value(buffer, "neomacs-cursor-effect").and_then(parse_cursor_effect_profile)
    });
    let visual_cursors = parse_visual_cursors(buffer, cursor_color);
    let x_stretch_cursor = global_bool(obarray, "x-stretch-cursor");
    let fill_column_indicator = buffer_fill_column_indicator(buffer, obarray);

    let header_line_height = if wants_header_line {
        chrome_face_pixel_height(
            &face_resolver.default_base_face_for_origin_without_buffer(
                &DisplayOrigin::HeaderLine {
                    selected: is_selected,
                },
            ),
            char_height,
        )
    } else {
        0.0
    };

    let tab_line_height = if wants_tab_line {
        chrome_face_pixel_height(
            &face_resolver.default_base_face_for_origin_without_buffer(&DisplayOrigin::TabLine),
            char_height,
        )
    } else {
        0.0
    };

    let buffer_default_face = face_resolver.resolve_buffer_default_face(buffer);
    let default_fg = buffer_default_face.fg;
    let default_bg = buffer_default_face.bg;

    Some(WindowParams {
        // Filled in by `resolve_window_display_source_params`, the one place
        // every window's params pass through that also holds the evaluator.
        space_image_catalog: None,
        window_id: win_id.0 as i64,
        buffer_id: buffer.id().0,
        bounds: display_bounds,
        text_bounds,
        selected: is_selected,
        kind: if is_minibuffer {
            WindowKind::Minibuffer
        } else {
            WindowKind::Main
        },
        left_col: window.left_col(),
        top_line: window.top_line(),
        // Window::window_start tracks GNU marker positions (1-based).
        // Normalize to the layout engine's internal 0-based char positions.
        window_start: lisp_char_pos_to_layout_i64(window_start),
        force_start,
        // GNU stores this as an offset from Z; recover the Lisp position and
        // normalize to the layout engine's 0-based space.
        previous_visible_end: window_end_valid.then(|| {
            lisp_char_pos_to_layout_i64(LispCharPos1::from_one_based_usize(
                buffer
                    .point_max_char_pos()
                    .get()
                    .saturating_add(1)
                    .saturating_sub(window_end_pos)
                    .max(1),
            ))
        }),
        // Mirror GNU `window.c:window_point` (around line 1782):
        //
        //   return (w == XWINDOW (selected_window)
        //           ? BUF_PT (XBUFFER (w->contents))
        //           : XMARKER (w->pointm)->charpos);
        //
        // For the selected window, the authoritative point lives in the
        // buffer (`BUF_PT`), because editing commands like
        // self-insert-command advance `buf->pt` but do not touch
        // `w->pointm` until the window is later deselected (via
        // `select_window`, which saves the live buffer point into the
        // outgoing window's pointm marker).  Reading `Window::point` here
        // would see a stale pre-command value and place the cursor one
        // character behind where typing just landed.  For non-selected
        // windows, `Window::point` is the right source (it was snapshotted
        // from `buf->pt` the last time the window was deselected).
        //
        // Buffer point is already 0-based (matches the layout engine's
        // internal coordinate system); `Window::point` is GNU/Lisp 1-based
        // and gets normalized with the usual `-1`.
        point: if is_selected {
            buffer.point_char_pos().get() as i64
        } else {
            lisp_char_pos_to_layout_i64(point)
        },
        buffer_size: buffer.point_max_char_pos().get() as i64,
        buffer_begv: buffer.point_min_char_pos().get() as i64,
        hscroll: hscroll as i32,
        vscroll,
        wrap_mode,
        word_wrap,
        tab_width,
        scroll_conservatively,
        scroll_step,
        scroll_minibuffer_conservatively,
        scroll_margin,
        tab_stop_list: buffer_local_list_values(buffer, "tab-stop-list")
            .iter()
            .filter_map(|v| v.as_int().map(|n| n as i32))
            .collect(),
        default_fg,
        default_bg,
        char_width,
        char_height,
        window_system: is_window_system,
        font_pixel_size: frame.font_pixel_size,
        image_scale_environment: image_scale_environment(frame, obarray),
        font_ascent: if is_window_system {
            default_font_ascent
                .filter(|ascent| *ascent > 0.0)
                .unwrap_or(frame.font_pixel_size * 0.8)
        } else {
            // GNU terminal redisplay has no font object here.  Stretch
            // glyphs and ordinary rows use one terminal cell, not the
            // GUI default font pixel ascent.
            char_height.max(1.0)
        },
        mode_line_height,
        header_line_height,
        tab_line_height,
        cursor_kind: cursor_spec.cursor_kind,
        cursor_bar_width: cursor_spec.bar_width,
        x_stretch_cursor,
        cursor_color,
        cursor_effects,
        visual_cursors,
        left_fringe_width: left_fringe,
        right_fringe_width: right_fringe,
        fringes_outside_margins: window
            .display()
            .is_some_and(|display| display.fringes_outside_margins),
        // `indicate-empty-lines` and `show-trailing-whitespace` are per-buffer
        // display variables in GNU (DEFVAR_PER_BUFFER): a `setq-default` enables
        // them in every buffer that has no local override. Read the EFFECTIVE
        // value (buffer-local, else global/default) — `buffer_local_bool` sees
        // only an explicit local binding and so silently ignored `setq-default`.
        indicate_empty_lines: if effective_buffer_bool(buffer, obarray, "indicate-empty-lines") {
            1
        } else {
            0
        },
        show_trailing_whitespace: effective_buffer_bool(
            buffer,
            obarray,
            "show-trailing-whitespace",
        ),
        trailing_ws_bg: face_bg_pixel(face_table, "trailing-whitespace", 0),
        fill_column_indicator: fill_column_indicator
            .map(|(column, _)| column)
            .unwrap_or(-1),
        fill_column_indicator_char: fill_column_indicator
            .map(|(_, character)| character)
            .unwrap_or('|'),
        fill_column_indicator_fg: face_fg_pixel(face_table, "fill-column-indicator", 0),
        extra_line_spacing: match buffer_local_value(buffer, "line-spacing") {
            Some(v) if v.is_fixnum() => v.as_fixnum().unwrap() as f32,
            Some(v) if v.is_float() => v.xfloat() as f32,
            _ => 0.0,
        },
        selective_display: buffer_selective_display(buffer),
        escape_glyph_fg: face_fg_pixel(face_table, "escape-glyph", 0),
        nobreak_char_display: effective_nobreak_char_display(buffer, obarray),
        // GNU merges `nobreak-space` for non-ASCII spaces (and `nobreak-hyphen`
        // for hyphens) in highlight mode; `nobreak-space` inherits `escape-glyph`,
        // so `resolve` yields the escape-glyph color when nobreak-space itself
        // sets no foreground (xdisp.c:8594-8617, faces.el `nobreak-space`).
        nobreak_char_fg: face_fg_pixel(face_table, "nobreak-space", 0),
        glyphless_char_fg: face_fg_pixel(face_table, "glyphless-char", 0),
        wrap_prefix: Vec::new(),
        line_prefix: Vec::new(),
        left_margin_width: left_margin,
        left_margin_columns: i64::try_from(margins.left())
            .expect("left margin column count fits in evaluator geometry"),
        right_margin_width: right_margin,
        right_margin_columns: i64::try_from(margins.right())
            .expect("right margin column count fits in evaluator geometry"),
        vertical_scroll_bar_side,
        horizontal_scroll_bar,
        scroll_bar_pixel_width,
        scroll_bar_pixel_height,
    })
}

/// Collect all leaf windows from a frame (including minibuffer) and build
/// `WindowParams` for each.
///
/// Returns `(FrameParams, Vec<WindowParams>)`, or `None` if the frame does
/// not exist.
pub fn collect_layout_params(
    evaluator: &Context,
    frame_id: FrameId,
    default_font_ascent: Option<f32>,
) -> Option<(FrameParams, Vec<WindowParams>)> {
    collect_layout_params_with_font_sizing(
        evaluator,
        frame_id,
        default_font_ascent,
        FontSizing::xft(),
    )
}

pub fn collect_layout_params_with_font_sizing(
    evaluator: &Context,
    frame_id: FrameId,
    default_font_ascent: Option<f32>,
    font_sizing: FontSizing,
) -> Option<(FrameParams, Vec<WindowParams>)> {
    let frame = evaluator.frame_manager().get(frame_id)?;
    let frame_is_selected = evaluator
        .frame_manager()
        .selected_frame()
        .is_some_and(|selected| selected.id == frame_id);
    let frame_params = frame_params_from_neovm(frame, evaluator.face_table(), evaluator.obarray());

    let mut window_params = Vec::new();

    // Collect leaf windows from the root window tree.
    let leaf_ids = frame.root_window.leaf_ids();
    for win_id in &leaf_ids {
        let Some(window) = frame.root_window.find(*win_id) else {
            continue;
        };
        let Some(buf_id) = window.buffer_id() else {
            continue;
        };
        let Some(buffer) = evaluator.buffer_manager().get(buf_id) else {
            continue;
        };
        let is_selected = frame_is_selected && frame.selected_window == *win_id;
        let window_cursor_type = evaluator.frame_manager().window_cursor_type(*win_id);
        let window_cursor_effect = evaluator
            .frame_manager()
            .window_parameter(*win_id, &Value::symbol("neomacs-cursor-effect"))
            .unwrap_or(Value::NIL);
        if let Some(wp) = window_params_from_neovm_with_font_sizing(
            window,
            buffer,
            frame,
            evaluator.obarray(),
            evaluator.face_table(),
            default_font_ascent,
            is_selected,
            false,
            window_cursor_type,
            window_cursor_effect,
            font_sizing,
        ) {
            tracing::debug!(
                "layout window cursor: win={} selected={} minibuffer=false kind={:?} width={} color=#{:06x} window-cursor-type={:?}",
                wp.window_id,
                wp.selected,
                wp.cursor_kind,
                wp.cursor_bar_width,
                wp.cursor_color,
                window_cursor_type,
            );
            window_params.push(wp);
        }
    }

    if window_params.len() > 1 {
        tracing::debug!(
            "collect_layout_params: {} leaf windows, root bounds=({},{} {}x{})",
            window_params.len(),
            frame.root_window.bounds().x,
            frame.root_window.bounds().y,
            frame.root_window.bounds().width,
            frame.root_window.bounds().height,
        );
    }

    // Add minibuffer window if present.
    if let Some(mini_leaf) = &frame.minibuffer_leaf {
        let buf_id = mini_leaf.buffer_id();
        let buffer = buf_id.and_then(|id| evaluator.buffer_manager().get(id));
        if let Some(buffer) = buffer {
            let is_selected = frame_is_selected && frame.selected_window == mini_leaf.id();
            let window_cursor_type = evaluator.frame_manager().window_cursor_type(mini_leaf.id());
            let window_cursor_effect = evaluator
                .frame_manager()
                .window_parameter(mini_leaf.id(), &Value::symbol("neomacs-cursor-effect"))
                .unwrap_or(Value::NIL);
            if let Some(wp) = window_params_from_neovm_with_font_sizing(
                mini_leaf,
                buffer,
                frame,
                evaluator.obarray(),
                evaluator.face_table(),
                default_font_ascent,
                is_selected,
                true,
                window_cursor_type,
                window_cursor_effect,
                font_sizing,
            ) {
                tracing::debug!(
                    "layout window cursor: win={} selected={} minibuffer=true kind={:?} width={} color=#{:06x} window-cursor-type={:?}",
                    wp.window_id,
                    wp.selected,
                    wp.cursor_kind,
                    wp.cursor_bar_width,
                    wp.cursor_color,
                    window_cursor_type,
                );
                tracing::debug!(
                    "  minibuffer id={} bounds=({},{} {}x{})",
                    wp.window_id,
                    wp.bounds.x,
                    wp.bounds.y,
                    wp.bounds.width,
                    wp.bounds.height,
                );
                window_params.push(wp);
            }
        }
    }

    Some((frame_params, window_params))
}

/// Buffer accessor for the layout engine.
///
/// Wraps a reference to a neovm-core `Buffer` and provides the operations
/// that the layout engine needs: text byte copying, position conversion,
/// and line counting.
pub(crate) struct RustBufferAccess<'a, B: LayoutBufferView> {
    buffer: &'a B,
}

impl<'a, B: LayoutBufferView> RustBufferAccess<'a, B> {
    /// Create a new buffer accessor.
    pub fn new(buffer: &'a B) -> Self {
        Self { buffer }
    }

    /// Convert an internal neovm buffer character position to a byte position.
    ///
    /// `WindowParams` used by the pure-Rust layout path carry neovm-core's
    /// internal character positions, which are 0-based and use an exclusive
    /// accessible end (`accessible_end_char` / `buffer_size`).
    pub fn charpos_to_bytepos(&self, charpos: i64) -> i64 {
        buffer_i64_charpos_to_emacs_byte_pos(self.buffer, charpos).get() as i64
    }

    /// Convert a GNU Lisp-visible buffer position to a byte position.
    ///
    /// GNU Lisp positions are 1-based, so this is only appropriate for
    /// values coming from Lisp APIs such as `minibuffer-prompt-end`.
    pub fn lisp_charpos_to_bytepos(&self, charpos: i64) -> i64 {
        let Some(charpos) = lisp_charpos_to_layout_char_pos(charpos) else {
            return 0;
        };
        buffer_charpos_to_emacs_byte_pos(self.buffer, charpos).get() as i64
    }

    /// Copy buffer text bytes in the range `[byte_from, byte_to)` into `out`.
    ///
    /// Uses backend-neutral Emacs byte ranges so layout is independent of
    /// the concrete buffer storage.
    pub fn copy_text(&self, byte_from: i64, byte_to: i64, out: &mut Vec<u8>) {
        let Some(range) = clamped_layout_emacs_byte_range(self.buffer, byte_from, byte_to) else {
            out.clear();
            return;
        };
        self.buffer.layout_copy_emacs_byte_range_to(range, out);
    }

    /// Count the number of newlines in `[byte_from, byte_to)`.
    ///
    /// Used for line number display.
    pub fn count_lines(&self, byte_from: i64, byte_to: i64) -> i64 {
        let Some(range) = clamped_layout_emacs_byte_range(self.buffer, byte_from, byte_to) else {
            return 0;
        };
        let mut count: i64 = 0;
        self.buffer
            .layout_try_for_each_emacs_byte_range_chunk(range, |chunk| {
                count += chunk.iter().filter(|byte| **byte == b'\n').count() as i64;
                Ok::<(), std::convert::Infallible>(())
            })
            .expect("newline counting is infallible");
        count
    }

    /// Read a single byte at the given byte position.
    ///
    /// Returns `None` if the position is out of bounds.
    pub fn byte_at(&self, byte_pos: i64) -> Option<u8> {
        let pos = layout_emacs_byte_pos_from_i64(byte_pos)?;
        if pos < layout_total_emacs_byte_end_pos(self.buffer) {
            self.buffer.layout_emacs_byte_at_pos(pos)
        } else {
            None
        }
    }

    /// Get the buffer's narrowed beginning (begv) as byte position.
    pub fn begv(&self) -> i64 {
        self.buffer.layout_point_min_emacs_byte_pos().get() as i64
    }

    /// Get the buffer's narrowed end (zv) as byte position.
    pub fn zv(&self) -> i64 {
        self.buffer.layout_point_max_emacs_byte_pos().get() as i64
    }
}

/// Soft cap (in emacs bytes) on how far ahead the display `invisible` scan
/// walks the text-property interval tree per redisplay. Beyond this the scan
/// returns a soft boundary and the checkpoint cache re-scans a screenful later;
/// a code buffer with no invisible text no longer walks to buffer-end on every
/// redisplay. Chosen to comfortably span a tall window so most redisplays scan
/// once. Mirrors GNU's `TEXT_PROP_DISTANCE_LIMIT` in `compute_stop_pos`.
const INVISIBLE_SCAN_BYTE_LIMIT: usize = 2048;

/// Whether an overlay's contributions (face, before/after strings) apply in the
/// window currently being laid out — GNU `overlay_matches_window`: an overlay
/// carrying a `window` property contributes in that window only (e.g. hl-line
/// with a non-sticky flag sets it to the selected window, so the same buffer in
/// two windows highlights only the selected one), a missing or non-window `window`
/// property is unrestricted, and `current_window_id == None` (frame chrome / TTY)
/// applies every overlay.
///
/// Delegates to `OverlayList::overlay_applies_to_window`, which the core property
/// resolvers apply themselves. The rule lives THERE, next to the precedence
/// comparator, so a resolver and a hand-written overlay scan cannot disagree about
/// which overlays speak in this window — which is how hl-line's per-window
/// highlight leaked into every window before.
pub(crate) fn overlay_applies_to_window<B: LayoutBufferView + ?Sized>(
    buffer: &B,
    overlay_id: Value,
    current_window_id: Option<u64>,
) -> bool {
    buffer
        .layout_overlays()
        .overlay_applies_to_window(overlay_id, current_window_id)
}

/// The overlay-`face` portion of GNU `face_at_buffer_position`: overlay `face`
/// values at `bytepos` in ascending `sort_overlays` precedence (merge in order —
/// higher precedence wins by being applied last), windowed overlays filtered to
/// `window_id`.
///
/// Both the precedence order and the `window` filter come from
/// `OverlayList::overlay_property_values_ascending_at_emacs_byte_pos`, the same
/// core primitive family the single-winner resolvers (`display`, `invisible`,
/// `mouse-face`) use, so the two policies cannot drift apart in ordering or in
/// the `window` rule — which is exactly how hl-line's per-window highlight once
/// leaked into every window.
///
/// This is the single implementation of the face scan. Every face-resolution path
/// — the per-glyph buffer face (`BufferTextSourceCursor::face_at`) and the
/// resolved-face/row-fill path (`FaceResolver::face_at_pos`) — calls it, so the
/// `font-lock-face` fallback boundary stays consistent too.
pub(crate) struct OverlayFacesAtPosition {
    /// Overlay `face` values to merge, in ascending priority.
    pub(crate) faces: Vec<Value>,
    /// Nearest overlay end strictly after `bytepos` (across ALL overlays, so a
    /// resolved-face cache invalidates at every overlay boundary). `None` if
    /// there is no overlay ending ahead.
    pub(crate) next_boundary: Option<EmacsBytePos>,
}

pub(crate) fn overlay_faces_at<B: LayoutBufferView + ?Sized>(
    buffer: &B,
    bytepos: EmacsBytePos,
    current_window_id: Option<u64>,
) -> OverlayFacesAtPosition {
    let overlays = buffer.layout_overlays();
    let mut next_boundary: Option<EmacsBytePos> = None;
    // GNU `face_at_buffer_position` narrows its `endptr` at EVERY overlay end at
    // this position, not only at those carrying `face`, so a resolved-face cache
    // re-resolves at every overlay boundary.
    for oid in overlays.overlays_at_emacs_byte_pos(bytepos) {
        if let Some(end) = overlays.overlay_end_emacs_byte_pos(oid)
            && end.get() > bytepos.get()
        {
            next_boundary = Some(match next_boundary {
                Some(prev) if prev.get() <= end.get() => prev,
                _ => end,
            });
        }
    }
    // `face` is GNU's one MERGE policy: overlay faces stack over the
    // text-property face in ascending `sort_overlays` precedence. Ordering and the
    // `window`-property filter come from the shared core primitive, so this path
    // cannot drift from the single-winner resolvers -- it previously ordered by a
    // bare `priority` integer, which reads a `(PRIMARY . SECONDARY)` priority as 0
    // and drops GNU's containment rule.
    let faces = overlays.overlay_property_values_ascending_at_emacs_byte_pos(
        bytepos,
        Value::symbol("face"),
        current_window_id,
    );
    OverlayFacesAtPosition {
        faces,
        next_boundary,
    }
}

/// Text property and overlay accessor for the layout engine.
///
/// Wraps a reference to a neovm-core `Buffer` and provides query methods
/// for invisible text, display properties, overlay strings, and other
/// text property-based features.
pub(crate) struct RustTextPropAccess<'a, B: LayoutBufferView> {
    buffer: &'a B,
    window_id: Option<u64>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct OverlayDisplayString {
    pub(crate) string: Value,
    pub(crate) overlay_id: Value,
    /// True for an after-string, false for a before-string. Drives GNU's
    /// `compare_overlay_entries` interleaving order.
    pub(crate) after_string_p: bool,
    /// The overlay's `priority` plist value, captured at collection time so the
    /// comparator does not re-read the plist.
    pub(crate) priority: i64,
}

impl OverlayDisplayString {
    #[cfg(test)]
    pub(crate) fn bytes(self) -> Option<&'static [u8]> {
        self.string.as_lisp_string().map(|string| string.as_bytes())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct InvisibleStatus {
    pub(crate) hidden: bool,
    pub(crate) ellipsis: bool,
}

impl InvisibleStatus {
    const VISIBLE: Self = Self {
        hidden: false,
        ellipsis: false,
    };

    const HIDDEN_NO_ELLIPSIS: Self = Self {
        hidden: true,
        ellipsis: false,
    };

    const HIDDEN_WITH_ELLIPSIS: Self = Self {
        hidden: true,
        ellipsis: true,
    };
}

fn layout_total_emacs_byte_end_pos<B: LayoutBufferView>(buffer: &B) -> EmacsBytePos {
    EmacsBytePos::ZERO.add_len(buffer.layout_total_emacs_byte_len())
}

fn clamped_layout_char_pos<B: LayoutBufferView>(buffer: &B, charpos: i64) -> CharPos0 {
    layout_char_pos_from_i64(charpos)
        .unwrap_or(CharPos0::ZERO)
        .min(buffer.layout_point_max_char_pos())
}

fn buffer_charpos_to_emacs_byte_pos<B: LayoutBufferView>(
    buffer: &B,
    charpos: CharPos0,
) -> EmacsBytePos {
    buffer.layout_char_pos_to_emacs_byte_pos(charpos.min(buffer.layout_point_max_char_pos()))
}

fn buffer_i64_charpos_to_emacs_byte_pos<B: LayoutBufferView>(
    buffer: &B,
    charpos: i64,
) -> EmacsBytePos {
    buffer_charpos_to_emacs_byte_pos(buffer, clamped_layout_char_pos(buffer, charpos))
}

fn buffer_emacs_byte_pos_to_charpos<B: LayoutBufferView>(
    buffer: &B,
    bytepos: EmacsBytePos,
) -> usize {
    buffer
        .layout_emacs_byte_pos_to_char_pos(bytepos.min(buffer.layout_point_max_emacs_byte_pos()))
        .get()
        .min(buffer.layout_point_max_char_pos().get())
}

fn clamped_layout_emacs_byte_pos<B: LayoutBufferView>(
    buffer: &B,
    bytepos: i64,
) -> Option<EmacsBytePos> {
    layout_emacs_byte_pos_from_i64(bytepos)
        .map(|pos| pos.min(layout_total_emacs_byte_end_pos(buffer)))
}

fn clamped_layout_emacs_byte_range<B: LayoutBufferView>(
    buffer: &B,
    byte_from: i64,
    byte_to: i64,
) -> Option<EmacsByteRange> {
    let from = clamped_layout_emacs_byte_pos(buffer, byte_from)?;
    let to = clamped_layout_emacs_byte_pos(buffer, byte_to)?;
    (from < to).then(|| EmacsByteRange::new(from, to))
}

fn invisible_atom_status(prop_atom: Value, spec: Value) -> InvisibleStatus {
    if spec.is_nil() {
        return InvisibleStatus::VISIBLE;
    }
    if spec.is_t() {
        return InvisibleStatus::HIDDEN_NO_ELLIPSIS;
    }

    let mut cursor = spec;
    while cursor.is_cons() {
        let entry = cursor.cons_car();
        if entry.is_cons() {
            if eq_value(&entry.cons_car(), &prop_atom) {
                return if entry.cons_cdr().is_nil() {
                    InvisibleStatus::HIDDEN_NO_ELLIPSIS
                } else {
                    InvisibleStatus::HIDDEN_WITH_ELLIPSIS
                };
            }
        } else if eq_value(&entry, &prop_atom) {
            return InvisibleStatus::HIDDEN_NO_ELLIPSIS;
        }
        cursor = cursor.cons_cdr();
    }

    if eq_value(&spec, &prop_atom) {
        InvisibleStatus::HIDDEN_NO_ELLIPSIS
    } else {
        InvisibleStatus::VISIBLE
    }
}

fn invisible_prop_status(prop: Option<Value>, spec: Value) -> InvisibleStatus {
    let Some(prop) = prop else {
        return InvisibleStatus::VISIBLE;
    };
    if prop.is_nil() || spec.is_nil() {
        return InvisibleStatus::VISIBLE;
    }
    if spec.is_t() {
        return InvisibleStatus::HIDDEN_NO_ELLIPSIS;
    }

    if prop.is_cons() {
        let mut cursor = prop;
        while cursor.is_cons() {
            let status = invisible_atom_status(cursor.cons_car(), spec);
            if status.hidden {
                return status;
            }
            cursor = cursor.cons_cdr();
        }
        if !cursor.is_nil() {
            return invisible_atom_status(cursor, spec);
        }
        InvisibleStatus::VISIBLE
    } else {
        invisible_atom_status(prop, spec)
    }
}

impl<'a, B: LayoutBufferView> RustTextPropAccess<'a, B> {
    /// Create a new text property accessor.
    pub fn new(buffer: &'a B) -> Self {
        Self {
            buffer,
            window_id: None,
        }
    }

    /// Create a text property accessor scoped to the redisplay window.
    pub fn new_for_window(buffer: &'a B, window_id: u64) -> Self {
        Self {
            buffer,
            window_id: Some(window_id),
        }
    }

    /// Check if text at `charpos` is invisible.
    ///
    /// Returns `(status, next_visible_pos)`.
    /// `next_visible_pos` is the next char position where visibility might change.
    /// If no change is found, returns `buffer.zv` as the next boundary.
    pub fn check_invisible(&self, charpos: i64) -> (InvisibleStatus, i64) {
        let status = self.invisible_status_at(charpos);
        let mut next_change = self.next_invisible_boundary(charpos);
        if status.hidden {
            // GNU `handle_invisible_prop` collapses CONSECUTIVE invisible runs
            // into a single ellipsis even when the `invisible` value changes
            // within the run.  Example: a folded org subtree (overlay
            // `invisible` = `org-fold-outline`, shows ellipsis) that contains an
            // org-link whose URL is separately invisible (`org-link`, no
            // ellipsis) -> three runs of differing `invisible` value but all
            // hidden.  Without collapsing, each run emits its own ellipsis
            // (`... [...]  [...]  [...]`); GNU shows one.  Extend the region over
            // every consecutive hidden position; the ellipsis flag stays that of
            // the run that opened the region (`status`, matching GNU which sets
            // `display_ellipsis` from the entry position).
            let max = self.buffer.layout_point_max_char_pos().get() as i64;
            while next_change < max && self.invisible_status_at(next_change).hidden {
                next_change = self.next_invisible_boundary(next_change);
            }
        }
        (status, next_change)
    }

    /// Combined `invisible` status at `charpos` from the `invisible` text
    /// property and the highest-priority overlay (GNU `invisible_p`).
    fn invisible_status_at(&self, charpos: i64) -> InvisibleStatus {
        let bytepos = buffer_i64_charpos_to_emacs_byte_pos(self.buffer, charpos);
        let spec = self
            .buffer
            .layout_buffer_local_value("buffer-invisibility-spec")
            .unwrap_or(Value::T);
        // GNU `handle_invisible_prop` resolves `invisible` through
        // `get_char_property_and_overlay (pos, Qinvisible, it->window)`: the
        // highest-precedence overlay carrying `invisible` wins OUTRIGHT and
        // shadows the text property, and its value is then judged against
        // `buffer-invisibility-spec`.
        //
        // Hand-rolling that policy produced two rendering bugs, both of which hid
        // text GNU shows: scanning on PAST the winner until some overlay happened
        // to say "hidden" (so a low-priority `invisible` overrode a
        // higher-priority overlay whose value is not in the spec), and consulting
        // the text property FIRST (so a text property hid text that a covering
        // overlay declared visible).
        let value = self
            .buffer
            .layout_overlays()
            .highest_priority_overlay_property_value_at_emacs_byte_pos(
                bytepos,
                Value::symbol("invisible"),
                self.window_id,
            )
            .or_else(|| {
                self.buffer
                    .layout_text_prop_at_emacs_byte_pos(bytepos, Value::symbol("invisible"))
            });
        invisible_prop_status(value, spec)
    }

    /// Next position where the `invisible` property changes, combining the
    /// `invisible` text-property extent with overlay boundaries (the text-prop
    /// half of GNU `next_single_char_property_change(pos, Qinvisible, ...)`).
    /// Scanning only `invisible` (not *any* property) avoids fragmenting a
    /// contiguous invisible region at every `face` change in a fontified buffer.
    fn next_invisible_boundary(&self, charpos: i64) -> i64 {
        let bytepos = buffer_i64_charpos_to_emacs_byte_pos(self.buffer, charpos);
        // Bound the `invisible` scan to a window-sized distance ahead instead of
        // walking the whole interval tree to buffer-end. The checkpoint cache
        // (`InvisibleTextScanCheckpoint`) only re-scans once the walk reaches the
        // returned boundary, so a soft boundary at `bytepos + LIMIT` just makes
        // the next re-scan happen a screenful later -- the invisible status at
        // every position is still looked up exactly, so nothing rendered changes.
        // Turns this per-redisplay scan from O(buffer) into O(window).
        // Mirrors GNU's TEXT_PROP_DISTANCE_LIMIT cap in `compute_stop_pos`.
        let scan_limit = EmacsBytePos::new(bytepos.get().saturating_add(INVISIBLE_SCAN_BYTE_LIMIT));
        let next_text_change = self
            .buffer
            .layout_next_single_text_prop_change_after_emacs_byte_pos_bounded(
                bytepos,
                Value::symbol("invisible"),
                scan_limit,
            )
            .map(|next| buffer_emacs_byte_pos_to_charpos(self.buffer, next))
            .unwrap_or_else(|| self.buffer.layout_point_max_char_pos().get());
        let next_overlay_change = self
            .buffer
            .layout_overlays()
            .next_boundary_after_emacs_byte_pos(bytepos)
            .map(|next| buffer_emacs_byte_pos_to_charpos(self.buffer, next))
            .unwrap_or_else(|| self.buffer.layout_point_max_char_pos().get());
        next_text_change.min(next_overlay_change) as i64
    }

    /// Next overlay start/end boundary strictly after `charpos`, if any.
    /// Overlay before/after-strings anchor exactly at overlay start and end
    /// positions, so scanning for overlay strings only needs to probe these
    /// boundaries, not every character.
    pub fn next_overlay_boundary_charpos_after(&self, charpos: i64) -> Option<i64> {
        let bytepos = buffer_i64_charpos_to_emacs_byte_pos(self.buffer, charpos);
        self.buffer
            .layout_overlays()
            .next_boundary_after_emacs_byte_pos(bytepos)
            .map(|next| buffer_emacs_byte_pos_to_charpos(self.buffer, next) as i64)
    }

    /// Check for line-spacing text property at `charpos`.
    ///
    /// Returns extra line spacing in pixels (0.0 if no property).
    pub fn check_line_spacing(&self, charpos: i64, base_height: f32) -> f32 {
        let bytepos = buffer_i64_charpos_to_emacs_byte_pos(self.buffer, charpos);
        match self
            .buffer
            .layout_text_prop_at_emacs_byte_pos(bytepos, Value::symbol("line-spacing"))
        {
            Some(v) if v.is_fixnum() => v.as_fixnum().unwrap() as f32,
            Some(v) if v.is_float() => {
                let f = v.xfloat();
                if f < 1.0 {
                    // Fraction of base height
                    base_height * (f as f32)
                } else {
                    f as f32
                }
            }
            _ => 0.0,
        }
    }

    /// Collect overlay before-string and after-string at `charpos`.
    ///
    /// Before-strings come from overlays starting at charpos.
    /// After-strings come from overlays ending at charpos.
    ///
    /// Returns `(before_strings, after_strings)` where each entry preserves the
    /// Lisp string object.  GNU `reseat_to_string' keeps string intervals live
    /// for overlay strings, so redisplay must not flatten these to bytes before
    /// the layout iterator has handled text properties such as `display'.
    /// Collect the overlay before/after-strings active at `charpos` into ONE
    /// list ordered by GNU's `compare_overlay_entries` (`src/xdisp.c`): after-
    /// strings come in front of before-strings from *different* overlays, before-
    /// strings precede after-strings of the *same* overlay, before-strings sort
    /// by ascending priority and after-strings by descending priority. Returns a
    /// single interleaved list (not separate before/after lists) so consumers can
    /// render them in GNU's exact visual order.
    pub fn overlay_strings_at(&self, charpos: i64) -> Vec<OverlayDisplayString> {
        let bytepos = buffer_i64_charpos_to_emacs_byte_pos(self.buffer, charpos);
        let mut entries = Vec::new();

        // GNU `load_overlay_strings' (`src/xdisp.c') scans overlays that
        // start or end at the iterator position, not only overlays covering
        // the position.  Zero-length completion overlays sit at point/EOB and
        // carry their displayed candidates in `before-string', so `overlays_at'
        // would miss exactly the strings redisplay must show.
        let scan_range = EmacsByteRange::new(
            bytepos.saturating_sub_len(EmacsByteLen::new(1)),
            bytepos.add_len(EmacsByteLen::new(1)),
        );
        let mut overlay_ids = self
            .buffer
            .layout_overlays()
            .overlays_in_emacs_byte_range(scan_range);
        overlay_ids.sort();
        overlay_ids.dedup();

        for oid in overlay_ids {
            if !self.overlay_applies_to_window(oid) {
                continue;
            }
            let priority = overlay_string_priority(oid);
            if self
                .buffer
                .layout_overlays()
                .overlay_start_emacs_byte_pos(oid)
                == Some(bytepos)
                && let Some(val) = self
                    .buffer
                    .layout_overlays()
                    .overlay_get_named(oid, Value::symbol("before-string"))
                && val.is_string()
            {
                entries.push(OverlayDisplayString {
                    string: val,
                    overlay_id: oid,
                    after_string_p: false,
                    priority,
                });
            }

            if self
                .buffer
                .layout_overlays()
                .overlay_end_emacs_byte_pos(oid)
                == Some(bytepos)
                && let Some(val) = self
                    .buffer
                    .layout_overlays()
                    .overlay_get_named(oid, Value::symbol("after-string"))
                && val.is_string()
            {
                entries.push(OverlayDisplayString {
                    string: val,
                    overlay_id: oid,
                    after_string_p: true,
                    priority,
                });
            }
        }

        // Stable insertion sort by GNU compare_overlay_entries. A MANUAL sort is
        // used (not slice::sort_by) because compare_overlay_entries is NOT a
        // total order: a zero-length overlay carrying both a before- and an
        // after-string can create a comparison cycle, which GNU's qsort tolerates
        // but Rust's sort_by may reject with a panic. Insertion sort applies the
        // pairwise rules without requiring transitivity and only moves an element
        // when it is strictly Less than its predecessor (so equal/cyclic entries
        // keep collection order). Overlay-string counts at a position are tiny, so
        // O(n^2) is irrelevant.
        for i in 1..entries.len() {
            let mut j = i;
            while j > 0 && compare_overlay_entries(&entries[j], &entries[j - 1]) == Ordering::Less {
                entries.swap(j, j - 1);
                j -= 1;
            }
        }
        entries
    }

    /// Test-only helper: split the interleaved `overlay_strings_at` list back
    /// into (before-strings, after-strings), each preserving its GNU within-kind
    /// order (before ascending priority, after descending priority).
    #[cfg(test)]
    pub fn overlay_strings_split_at(
        &self,
        charpos: i64,
    ) -> (Vec<OverlayDisplayString>, Vec<OverlayDisplayString>) {
        let entries = self.overlay_strings_at(charpos);
        let before = entries
            .iter()
            .copied()
            .filter(|e| !e.after_string_p)
            .collect();
        let after = entries
            .iter()
            .copied()
            .filter(|e| e.after_string_p)
            .collect();
        (before, after)
    }

    fn overlay_applies_to_window(&self, overlay_id: Value) -> bool {
        overlay_applies_to_window(self.buffer, overlay_id, self.window_id)
    }

    /// Get the next position where any text property changes.
    ///
    /// Test-only helper for direct property-table regression coverage.
    #[cfg(test)]
    pub fn next_property_change(&self, charpos: i64) -> i64 {
        let bytepos = buffer_i64_charpos_to_emacs_byte_pos(self.buffer, charpos);
        self.buffer
            .layout_next_text_prop_change_after_emacs_byte_pos(bytepos)
            .map(|next| buffer_emacs_byte_pos_to_charpos(self.buffer, next))
            .unwrap_or_else(|| self.buffer.layout_point_max_char_pos().get()) as i64
    }

    /// Get a specific text property at a position.
    pub fn get_property(&self, charpos: i64, name: Value) -> Option<Value> {
        let bytepos = buffer_i64_charpos_to_emacs_byte_pos(self.buffer, charpos);
        self.buffer
            .layout_text_prop_at_emacs_byte_pos(bytepos, name)
    }
}

/// Rust port of GNU `compare_overlay_entries` (`src/xdisp.c`). Orders overlay
/// before/after-strings into one visual sequence:
/// - differing kind: same overlay → before-string precedes after-string;
///   different overlays → after-string precedes before-string;
/// - same kind, differing priority: after-strings sort by *decreasing* priority,
///   before-strings by *increasing* priority;
/// - else equal (a stable sort keeps collection order).
fn compare_overlay_entries(e1: &OverlayDisplayString, e2: &OverlayDisplayString) -> Ordering {
    if e1.after_string_p != e2.after_string_p {
        if e1.overlay_id.bits() == e2.overlay_id.bits() {
            // Same overlay: before-string in front of after-string.
            if e1.after_string_p {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        } else {
            // Different overlays: after-strings in front of before-strings.
            if e1.after_string_p {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }
    } else if e1.priority != e2.priority {
        if e1.after_string_p {
            // After-strings: decreasing priority.
            e2.priority.cmp(&e1.priority)
        } else {
            // Before-strings: increasing priority.
            e1.priority.cmp(&e2.priority)
        }
    } else {
        Ordering::Equal
    }
}

fn overlay_string_priority(overlay: Value) -> i64 {
    let Some(data) = overlay.as_overlay_data() else {
        return 0;
    };
    let Some(priority) =
        neovm_core::emacs_core::plist::plist_get(data.plist, &Value::symbol("priority"))
    else {
        return 0;
    };
    match priority.kind() {
        ValueKind::Fixnum(n) => n,
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// ResolvedFace — pure-Rust equivalent of FaceDataFFI
// ---------------------------------------------------------------------------

/// Convert a neovm-core `Color` to a packed sRGB pixel (0x00RRGGBB).
fn color_to_pixel(c: &NeoColor) -> u32 {
    c.to_pixel()
}

/// Check if two colors are perceptually close.
///
/// GNU Emacs uses this for `:distant-foreground`: when the foreground
/// is too similar to the background, swap to the distant foreground
/// for readability.  Uses simple RGB distance threshold.
fn colors_close(a: u32, b: u32) -> bool {
    let ar = (a >> 16) & 0xFF;
    let ag = (a >> 8) & 0xFF;
    let ab = a & 0xFF;
    let br = (b >> 16) & 0xFF;
    let bg = (b >> 8) & 0xFF;
    let bb = b & 0xFF;
    let dr = ar.abs_diff(br);
    let dg = ag.abs_diff(bg);
    let db = ab.abs_diff(bb);
    // Weighted Euclidean distance (human perception weights R more than B)
    // Threshold ~30 in each channel ≈ 2700 squared distance
    (dr * dr * 3 + dg * dg * 4 + db * db * 2) < 3000
}

/// Resolve a `:stipple` string — a built-in bitmap name (`gray3`, …) or an XBM
/// file — to a [`StipplePattern`](neomacs_display_protocol::StipplePattern),
/// mirroring GNU's `image_create_bitmap_from_file`. Built-ins are portable and
/// need no I/O; files are read and parsed once, then cached, because face
/// realization runs during layout and must never touch disk per frame. The
/// `x-bitmap-file-path` Lisp search list is not reachable from the layout
/// bridge; explicit paths and the standard X11 bitmap directories are searched.
fn stipple_from_name_or_file(name: &str) -> Option<neomacs_display_protocol::StipplePattern> {
    use neomacs_display_protocol::StipplePattern;
    if let Some(pattern) = StipplePattern::builtin(name) {
        return Some(pattern);
    }
    static CACHE: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<String, Option<StipplePattern>>>,
    > = std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(std::sync::Mutex::default);
    if let Ok(guard) = cache.lock()
        && let Some(hit) = guard.get(name)
    {
        return hit.clone();
    }
    let pattern = stipple_file_search_paths(name)
        .into_iter()
        .find_map(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| StipplePattern::from_xbm_source(&text));
    if let Ok(mut guard) = cache.lock() {
        guard.insert(name.to_string(), pattern.clone());
    }
    pattern
}

/// Candidate paths for a `:stipple` bitmap file name. An absolute or
/// cwd-relative path is used directly; a bare name is also looked up in the
/// standard X11 bitmap directories (GNU's `x-bitmap-file-path` default subset).
fn stipple_file_search_paths(name: &str) -> Vec<std::path::PathBuf> {
    let mut paths = vec![std::path::PathBuf::from(name)];
    if !std::path::Path::new(name).is_absolute() {
        for dir in [
            "/usr/include/X11/bitmaps",
            "/usr/share/X11/bitmaps",
            "/usr/X11R6/include/X11/bitmaps",
        ] {
            paths.push(std::path::Path::new(dir).join(name));
        }
    }
    paths
}

/// Resolved face attributes ready for the layout engine.
///
/// This is the neovm-core equivalent of `FaceDataFFI`.  All attributes are
/// fully realized (no `Option`s) so the layout engine can use them directly.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedFace {
    /// Foreground color (sRGB pixel: 0x00RRGGBB).
    pub fg: u32,
    /// Background color (sRGB pixel: 0x00RRGGBB).
    pub bg: u32,
    /// Use the terminal's default foreground instead of `fg`.
    pub use_default_foreground: bool,
    /// Use the terminal's default background instead of `bg`.
    pub use_default_background: bool,
    /// Font family name.
    pub font_family: String,
    /// Font weight (CSS 100-900).
    pub font_weight: u16,
    /// Italic flag.
    pub italic: bool,
    /// Font size in pixels.
    pub font_size: f32,
    /// Underline style (0=none, 1=line, 2=wave, 3=double, 4=dotted, 5=dashed).
    pub underline_style: u8,
    /// Underline color (sRGB pixel, 0 = use foreground).
    pub underline_color: u32,
    /// Strike-through enabled.
    pub strike_through: bool,
    /// Strike-through color (sRGB pixel, 0 = use foreground).
    pub strike_through_color: u32,
    /// Overline enabled.
    pub overline: bool,
    /// Overline color (sRGB pixel, 0 = use foreground).
    pub overline_color: u32,
    /// Box type (0=none, 1=line).
    pub box_type: u8,
    /// Box color (sRGB pixel).
    pub box_color: u32,
    /// GNU scalar box line width, including inside/outside sign semantics.
    pub box_line_width: BoxLineWidth,
    /// Extend background to end of line.
    pub extend: bool,
    /// Simulate bold by drawing twice at x and x+1.
    pub overstrike: bool,
    /// Preserve terminal inverse-video when both colors are terminal defaults.
    pub terminal_inverse_video: bool,
    /// Per-face measured character advance width (from FontMetricsService, 0.0 = use default).
    font_char_width: f32,
    /// Per-face font ascent (from FontMetricsService, 0.0 = use default).
    pub font_ascent: f32,
    /// Per-face line height (from FontMetricsService, 0.0 = use default).
    pub font_line_height: f32,
    /// Face cache ID — matches [`BasicFaceId`] for basic faces (0–19)
    /// or a dynamically allocated ID (≥20) for other faces.
    pub face_id: u32,
    /// Lisp face name this face was resolved from, when it came from a
    /// named face (GNU keeps the name reachable via `struct face::lface`).
    /// `None` for anonymous attribute-plist faces.
    pub lisp_name: Option<String>,
    /// Realized `:stipple` bitmap, if the face specified one. GNU realizes the
    /// `LFACE_STIPPLE_INDEX` spec to a pixmap id in `face->stipple`; neomacs
    /// realizes it directly to the XBM `StipplePattern` the renderer tiles.
    pub stipple: Option<neomacs_display_protocol::StipplePattern>,
}

impl Default for ResolvedFace {
    fn default() -> Self {
        Self {
            fg: 0x00000000,
            bg: 0x00FFFFFF,
            use_default_foreground: false,
            use_default_background: false,
            font_family: String::new(),
            font_weight: 400,
            italic: false,
            font_size: 14.0,
            underline_style: 0,
            underline_color: 0,
            strike_through: false,
            strike_through_color: 0,
            overline: false,
            overline_color: 0,
            box_type: 0,
            box_color: 0,
            box_line_width: BoxLineWidth::default(),
            extend: false,
            overstrike: false,
            terminal_inverse_video: false,
            font_char_width: 0.0,
            font_ascent: 0.0,
            font_line_height: 0.0,
            face_id: 0, // DEFAULT_FACE_ID
            lisp_name: None,
            stipple: None,
        }
    }
}

impl ResolvedFace {
    /// Typed view of this bridge-side face id. `ResolvedFace.face_id` stays a
    /// raw u32 (the neovm bridge boundary keeps raw reprs); this is THE
    /// conversion point where ids leave the bridge as [`FaceId`].
    pub(crate) fn display_face_id(&self) -> FaceId {
        FaceId::new(self.face_id)
    }

    /// Store a typed face id back into the raw bridge-side field.
    pub(crate) fn set_display_face_id(&mut self, id: FaceId) {
        self.face_id = id.get();
    }

    pub(crate) fn measured_char_width_px(&self) -> f32 {
        self.font_char_width
    }

    pub(crate) fn set_measured_char_width_px(&mut self, width: f32) {
        self.font_char_width = width;
    }
}

// ---------------------------------------------------------------------------
// FaceResolver
// ---------------------------------------------------------------------------

/// Outcome of testing whether a face-spec list is a GNU
/// `(:filtered FILTER SPEC)` form and, if so, whether FILTER matches.
enum FilteredFaceSpec {
    /// Not a `:filtered` form at all — the caller handles `items` as an inline
    /// face plist or a list of face specs.
    NotFiltered,
    /// A `:filtered` form whose FILTER did NOT match (or is malformed). The
    /// wrapped SPEC must be DROPPED — contribute no attributes — never
    /// re-interpreted as an inline plist. GNU `evaluate_face_filter` returns
    /// false here and the spec is skipped.
    Rejected,
    /// A `:filtered` form whose FILTER matched — the caller recurses into the
    /// unwrapped SPEC.
    Matched(Vec<Value>),
}

/// Resolves face attributes at buffer positions using the neovm-core
/// `FaceTable`, text properties, and overlays.
///
/// Replaces the C FFI `face_at_buffer_position()` path for the pure-Rust
/// backend.
pub struct FaceResolver {
    face_table: FaceTable,
    default_face: ResolvedFace,
    /// Window system in use: `None` for TTY, `Some("x")` for X11,
    /// `Some("wayland")` for Wayland, etc.  Used to evaluate
    /// `:filtered` face spec predicates.
    window_system: Option<String>,
    /// The window-parameters `(PARAM . VALUE)` alist of the window whose faces
    /// are currently being resolved. Set per-window (see
    /// `set_current_window_parameters`) so the GNU `(:window PARAM VALUE)`
    /// `:filtered` face predicate can match — the `FaceResolver` is created
    /// once per frame and shared `&` across windows, so this is interior
    /// mutability updated at each window boundary rather than a constructor arg.
    /// Empty for frame chrome / TTY / any non-window context, matching GNU's
    /// "no window ⇒ filter fails".
    current_window_parameters: std::cell::RefCell<Vec<(Value, Value)>>,
    /// Numeric id of the window whose faces are currently being resolved, or
    /// `None` for frame chrome / TTY / any non-window context. Set per-window
    /// alongside [`set_current_window_parameters`]. Used to honor an overlay's
    /// `window` property: GNU restricts such an overlay (e.g. hl-line with a
    /// non-sticky flag, which sets `window` to the selected window) to that one
    /// window, so the same buffer shown in two windows highlights only the
    /// selected one — see the overlay-face loop in `face_at_pos`.
    current_window_id: std::cell::Cell<Option<u64>>,
    font_sizing: FontSizing,
}

impl FaceResolver {
    pub(crate) fn is_window_system(&self) -> bool {
        self.window_system.is_some()
    }

    fn face_spec_is_plist(items: &[Value]) -> bool {
        match items.first() {
            Some(v) if v.is_keyword() => true,
            Some(item) => item
                .as_symbol_name()
                .is_some_and(|name| name.starts_with(':')),
            None => false,
        }
    }

    /// Create a new `FaceResolver`.
    ///
    /// Clones the `FaceTable` so the resolver owns its data and does not
    /// borrow from the `Context`.  This allows `layout_window_rust` to
    /// take `&mut Context` for `format-mode-line` evaluation while
    /// still using the `FaceResolver`.
    pub fn new(
        face_table: &FaceTable,
        default_fg: u32,
        default_bg: u32,
        default_font_size: f32,
        window_system: Option<String>,
    ) -> Self {
        Self::new_with_font_sizing(
            face_table,
            default_fg,
            default_bg,
            default_font_size,
            window_system,
            FontSizing::xft(),
        )
    }

    pub fn new_with_font_sizing(
        face_table: &FaceTable,
        default_fg: u32,
        default_bg: u32,
        default_font_size: f32,
        window_system: Option<String>,
        font_sizing: FontSizing,
    ) -> Self {
        let neo_default = face_table.resolve("default");
        let mut df = ResolvedFace::default();
        if let Some(color) = neo_default.foreground.as_ref() {
            df.fg = color_to_pixel(color);
        } else {
            df.fg = default_fg;
            df.use_default_foreground = true;
        }
        if let Some(color) = neo_default.background.as_ref() {
            df.bg = color_to_pixel(color);
        } else {
            df.bg = default_bg;
            df.use_default_background = true;
        }
        df.font_family = neo_default
            .family_runtime_string_owned()
            .unwrap_or_default();
        df.font_weight = neo_default
            .weight
            .map(FontWeight::css_weight)
            .unwrap_or(FontWeight::NORMAL.css_weight());
        df.italic = neo_default.slant.map(|s| s.is_italic()).unwrap_or(false);
        df.font_size = match &neo_default.height {
            Some(FaceHeight::Absolute(tenths)) => font_sizing.face_height_to_layout_pixels(*tenths),
            _ => default_font_size,
        };
        df.extend = neo_default.extend.unwrap_or(false);
        df.overstrike = neo_default.overstrike;

        // Underline
        if let Some(ul) = neo_default.underline.enabled() {
            df.underline_style = underline_style_to_u8(&ul.style);
            df.underline_color = ul.color.as_ref().map(color_to_pixel).unwrap_or(0);
        }
        // Overline
        if neo_default.overline == Some(true) {
            df.overline = true;
        }
        // Strike-through
        if neo_default.strike_through == Some(true) {
            df.strike_through = true;
        }
        // Box
        if let Some(bb) = &neo_default.box_border {
            df.box_type = box_style_to_u8(&bb.style);
            df.box_color = bb.color.as_ref().map(color_to_pixel).unwrap_or(0);
            df.box_line_width = BoxLineWidth::from_gnu(bb.width);
        }

        Self {
            face_table: face_table.clone(),
            default_face: df,
            window_system,
            current_window_parameters: std::cell::RefCell::new(Vec::new()),
            current_window_id: std::cell::Cell::new(None),
            font_sizing,
        }
    }

    /// Point the resolver at the window whose faces are about to be resolved,
    /// so a `(:window PARAM VALUE)` `:filtered` predicate can consult that
    /// window's parameters. Call at each window boundary (and with an empty
    /// alist for frame chrome / non-window contexts). GNU threads the window
    /// `w` into `evaluate_face_filter`; the resolver being frame-shared and
    /// used behind `&`, this is the equivalent interior-mutable hook.
    pub fn set_current_window_parameters(&self, params: Vec<(Value, Value)>) {
        self.current_window_parameters.replace(params);
    }

    /// Point the resolver at the window whose faces are about to be resolved so
    /// an overlay's `window` property can be honored (a windowed overlay applies
    /// only in its window). `None` for frame chrome / non-window contexts, which
    /// then applies every overlay (matching GNU's "no window ⇒ unrestricted").
    /// Call at each window boundary alongside `set_current_window_parameters`.
    pub fn set_current_window_id(&self, window_id: Option<u64>) {
        self.current_window_id.set(window_id);
    }

    /// Return a reference to the resolved default face.
    pub fn default_face(&self) -> &ResolvedFace {
        &self.default_face
    }

    /// Resolve a named face from the face table, assigning a stable
    /// face-cache ID.
    ///
    /// Basic faces (see [`BasicFaceId`]) get their fixed enum value.
    /// Other faces get a dynamically allocated ID ≥
    /// [`BasicFaceId::SENTINEL`] (20).
    pub fn resolve_named_face(&self, name: &str) -> ResolvedFace {
        use neomacs_display_protocol::face::BasicFaceId;
        let face = self.face_table.resolve(name);
        let mut resolved = self.realize_face(&face);
        resolved.lisp_name = Some(name.to_string());
        if let Some(basic) = BasicFaceId::from_name(name) {
            resolved.face_id = basic.into();
        } else {
            // Non-basic faces carry no stable id from here. Every consumer that
            // needs a matrix face id re-keys via the frame-scoped allocator
            // (GNU's single `face_cache->used`); the sentinel makes that
            // "must be re-keyed" contract explicit.
            resolved.face_id = BasicFaceId::SENTINEL;
        }
        resolved
    }

    /// Resolve a named face while ignoring its final `:inverse-video`
    /// attribute.
    ///
    /// GNU's toolkit-backed menu bars use the `menu` face resources for
    /// foreground/background/font, but the default `menu` defface has an
    /// empty `x-toolkit` branch instead of the TTY/fallback inverse-video
    /// branch.  Neomacs' GUI menu bar is custom-rendered, so use this helper
    /// at that toolkit boundary: preserve the face's concrete attributes, but
    /// do not swap foreground/background for `:inverse-video`.
    pub fn resolve_named_face_without_inverse_video(&self, name: &str) -> ResolvedFace {
        use neomacs_display_protocol::face::BasicFaceId;
        let mut face = self.face_table.resolve(name);
        face.inverse_video = None;
        let mut resolved = self.realize_face(&face);
        resolved.lisp_name = Some(name.to_string());
        if let Some(basic) = BasicFaceId::from_name(name) {
            resolved.face_id = basic.into();
        } else {
            // See `resolve_named_face`: non-basic faces are re-keyed by consumers
            // via the frame-scoped allocator (GNU `face_cache->used`).
            resolved.face_id = BasicFaceId::SENTINEL;
        }
        resolved.terminal_inverse_video = false;
        resolved
    }

    fn apply_specified_face_over(&self, base: &ResolvedFace, face: &NeoFace) -> ResolvedFace {
        let mut rf = base.clone();
        if let Some(c) = &face.foreground {
            rf.fg = color_to_pixel(c);
            rf.use_default_foreground = false;
        }
        if let Some(c) = &face.background {
            rf.bg = color_to_pixel(c);
            rf.use_default_background = false;
        }
        match face.inverse_video {
            Some(true) => {
                rf.terminal_inverse_video = rf.use_default_foreground && rf.use_default_background;
                std::mem::swap(&mut rf.fg, &mut rf.bg);
                std::mem::swap(
                    &mut rf.use_default_foreground,
                    &mut rf.use_default_background,
                );
            }
            Some(false) => rf.terminal_inverse_video = false,
            None => {}
        }

        if let Some(family) = face.family_runtime_string_owned() {
            rf.font_family = family;
        }
        if let Some(weight) = face.weight {
            rf.font_weight = weight.css_weight();
        }
        if let Some(slant) = face.slant {
            rf.italic = slant.is_italic();
        }
        if let Some(height) = &face.height {
            match height {
                FaceHeight::Absolute(tenths) => {
                    rf.font_size = self.font_sizing.face_height_to_layout_pixels(*tenths);
                }
                FaceHeight::Relative(factor) => {
                    rf.font_size = (rf.font_size * *factor as f32).max(1.0);
                }
            }
        }

        match &face.underline {
            FaceDecoration::Unspecified => {}
            FaceDecoration::Disabled => {
                rf.underline_style = 0;
                rf.underline_color = 0;
            }
            FaceDecoration::Enabled(underline) => {
                rf.underline_style = underline_style_to_u8(&underline.style);
                // GNU draws `:underline t` (no explicit color) in the face's
                // foreground -- e.g. `nobreak-space` inherits `escape-glyph`'s
                // brown fg and underlines in that same brown. Default the
                // unspecified color to `rf.fg` here (fg is applied earlier in
                // this merge), mirroring the box-border resolution below so a
                // downstream pixel-0 underline color means "explicit black"
                // uniformly with box.
                rf.underline_color = underline
                    .color
                    .as_ref()
                    .map(color_to_pixel)
                    .unwrap_or(rf.fg);
            }
        }
        if let Some(overline) = face.overline {
            rf.overline = overline;
            // GNU draws `:overline t` (no explicit color) in the face
            // foreground, like `:underline t` and `:box t` above. An explicit
            // `:overline COLOR` (handled just below) overrides.
            if overline {
                rf.overline_color = rf.fg;
            }
        }
        if let Some(color) = &face.overline_color {
            rf.overline_color = color_to_pixel(color);
        }
        if let Some(strike) = face.strike_through {
            rf.strike_through = strike;
            // Same rule for `:strike-through t` -> face foreground.
            if strike {
                rf.strike_through_color = rf.fg;
            }
        }
        if let Some(color) = &face.strike_through_color {
            rf.strike_through_color = color_to_pixel(color);
        }
        if let Some(box_border) = &face.box_border {
            rf.box_type = box_style_to_u8(&box_border.style);
            rf.box_color = box_border
                .color
                .as_ref()
                .map(color_to_pixel)
                .unwrap_or(rf.fg);
            rf.box_line_width = BoxLineWidth::from_gnu(box_border.width);
        }
        if let Some(extend) = face.extend {
            rf.extend = extend;
        }
        if face.overstrike {
            rf.overstrike = true;
        }

        // Distant-foreground: swap fg when too close to bg
        if let Some(dfg) = &face.distant_foreground
            && colors_close(rf.fg, rf.bg)
        {
            rf.fg = color_to_pixel(dfg);
            rf.use_default_foreground = false;
        }

        // Stipple: a face that specifies `:stipple` overrides the inherited
        // one; leaving it unspecified keeps the base face's stipple (GNU merge
        // semantics). This is the path buffer-text (text-property / overlay /
        // font-lock) faces take, e.g. `indent-bars`.
        if let Some(pat) = self.realize_stipple(face) {
            rf.stipple = Some(pat);
        }

        rf
    }

    fn apply_inline_face_over(&self, base: &ResolvedFace, face: &NeoFace) -> ResolvedFace {
        // Resolve `:inherit` first so the inline face's own attributes
        // below override the inherited ones. Mirrors GNU
        // `merge_face_vectors` (xfaces.c:2305-2314): inherited attrs are
        // merged first, then the face's own specified attributes take
        // precedence.
        let base_after_inherit = match face.inherit {
            Some(inherit_ref) => self
                .resolve_face_value_over(base, &inherit_ref)
                .unwrap_or_else(|| base.clone()),
            None => base.clone(),
        };
        self.apply_specified_face_over(&base_after_inherit, face)
    }

    fn resolve_named_face_overlay_spec(&self, name: &str, depth: usize) -> NeoFace {
        if depth > 40 {
            return NeoFace::default();
        }
        if name == "default" && depth > 0 {
            return NeoFace::default();
        }

        let Some(face) = self.face_table.get(name).cloned() else {
            return NeoFace::default();
        };
        self.resolve_face_overlay_spec(face, depth)
    }

    fn resolve_face_overlay_spec(&self, mut face: NeoFace, depth: usize) -> NeoFace {
        if depth > 40 {
            return NeoFace::default();
        }

        let parent = match face.inherit.take() {
            Some(inherit_ref) => self.resolve_face_ref_overlay_spec(inherit_ref, depth + 1),
            None => NeoFace::default(),
        };
        parent.merge(&face)
    }

    fn resolve_face_ref_overlay_spec(&self, face_ref: Value, depth: usize) -> NeoFace {
        if depth > 40 || face_ref.is_nil() || face_ref.is_symbol_named("nil") {
            return NeoFace::default();
        }

        if let Some(name) = Self::face_name_from_value(&face_ref) {
            return self.resolve_named_face_overlay_spec(name, depth);
        }

        let Some(items) = list_to_vec(&face_ref) else {
            return NeoFace::default();
        };
        if items.is_empty() {
            return NeoFace::default();
        }

        match self.eval_filtered_face_spec(&items) {
            FilteredFaceSpec::Matched(filtered_spec) => {
                return self.resolve_face_ref_overlay_spec(Value::list(filtered_spec), depth + 1);
            }
            // Filter didn't match → the wrapped spec contributes nothing.
            FilteredFaceSpec::Rejected => return NeoFace::default(),
            FilteredFaceSpec::NotFiltered => {}
        }
        if Self::face_spec_is_plist(&items) {
            let face = NeoFace::from_plist("--inline--", &items);
            return self.resolve_face_overlay_spec(face, depth + 1);
        }

        let mut result = NeoFace::default();
        for item in items.iter().rev() {
            let next = self.resolve_face_ref_overlay_spec(*item, depth + 1);
            result = result.merge(&next);
        }
        result
    }

    fn apply_named_face_over(&self, base: &ResolvedFace, name: &str) -> ResolvedFace {
        let face = self.resolve_named_face_overlay_spec(name, 0);
        self.apply_specified_face_over(base, &face)
    }

    fn face_name_from_value(value: &Value) -> Option<&str> {
        match value.kind() {
            ValueKind::Symbol(_) => value.as_symbol_name(),
            ValueKind::String => value.as_utf8_str(),
            _ => None,
        }
    }

    /// Classify `items` as a GNU `(:filtered FILTER SPEC…)` form and evaluate
    /// FILTER against the current context. Distinguishing the three outcomes
    /// matters: a REJECTED filter must drop its wrapped SPEC entirely — the
    /// earlier `Option` return conflated "rejected" with "not a filter", so the
    /// caller fell through and mis-applied `(:filtered …)` as an inline plist,
    /// ignoring the filter.
    ///
    /// Supported filter predicates:
    ///   `:window PARAMETER VALUE` — GNU's only real face filter
    ///                          (`evaluate_face_filter`, src/xfaces.c): matches
    ///                          when the current window's window-parameter
    ///                          PARAMETER is `eq` to VALUE. This is what
    ///                          indent-bars' per-window stipple-rotation remap
    ///                          uses (`(:window indent-bars-whr WHR)`).
    ///   `:window-system SYM`  — matches when `self.window_system == SYM`
    ///                          (nil for TTY, "x" for X11, etc.). Non-GNU
    ///                          neomacs extension, retained.
    fn eval_filtered_face_spec(&self, items: &[Value]) -> FilteredFaceSpec {
        let Some(name) = items.first().and_then(|first| first.as_symbol_name()) else {
            return FilteredFaceSpec::NotFiltered;
        };
        if name != "filtered" && name != ":filtered" {
            return FilteredFaceSpec::NotFiltered; // not a :filtered form
        }
        // From here it IS a `:filtered` form: a malformed or unmatched filter
        // DROPS the spec (GNU returns false); it is never re-read as a plist.
        if items.len() < 3 {
            return FilteredFaceSpec::Rejected; // malformed: need (:filtered FILTER SPEC)
        }

        let filter = &items[1];
        let spec = &items[2..];

        let ValueKind::Cons = filter.kind() else {
            return FilteredFaceSpec::Rejected; // non-list filter — malformed
        };
        // Evaluate filter predicates. All predicates must pass; mirrors GNU's
        // `face_spec_match_p` (`src/xfaces.c`).
        let filter_items = list_to_vec(filter).unwrap_or_default();
        let mut i = 0;
        while i < filter_items.len() {
            let Some(pred_name) = filter_items.get(i).and_then(|pred| pred.as_symbol_name()) else {
                return FilteredFaceSpec::Rejected;
            };
            match pred_name {
                ":window" | "window" => {
                    // GNU `evaluate_face_filter` (src/xfaces.c): the filter is
                    // `(:window PARAMETER VALUE)` and matches only when the
                    // current window has window-parameter PARAMETER `eq` VALUE.
                    // No current window (frame chrome / TTY) ⇒ no match.
                    let (Some(parameter), Some(value)) =
                        (filter_items.get(i + 1), filter_items.get(i + 2))
                    else {
                        return FilteredFaceSpec::Rejected; // malformed (:window …)
                    };
                    let params = self.current_window_parameters.borrow();
                    let matches = params.iter().any(|(param, val)| {
                        param.bits() == parameter.bits() && val.bits() == value.bits()
                    });
                    if !matches {
                        return FilteredFaceSpec::Rejected;
                    }
                    i += 2; // consumed PARAMETER + VALUE (pred skipped below)
                }
                ":window-system" | "window-system" => {
                    i += 1;
                    let ws_name = filter_items
                        .get(i)
                        .and_then(|val| val.as_symbol_name())
                        .unwrap_or("");
                    let current = self.window_system.as_deref().unwrap_or("nil");
                    if current != ws_name && ws_name != "nil" {
                        return FilteredFaceSpec::Rejected;
                    }
                    if ws_name == "nil" && self.window_system.is_some() {
                        return FilteredFaceSpec::Rejected; // TTY filter, but we're on GUI
                    }
                }
                _ => {
                    // Unknown predicate — fail (matches GNU).
                    return FilteredFaceSpec::Rejected;
                }
            }
            i += 1;
        }
        FilteredFaceSpec::Matched(spec.to_vec())
    }

    fn buffer_face_remapping_specs<B: LayoutBufferView>(
        buffer: &B,
        face_name: &str,
    ) -> Option<Value> {
        let mut cursor = buffer.layout_buffer_local_value("face-remapping-alist")?;
        loop {
            if !cursor.is_cons() {
                return None;
            }
            let entry_car = cursor.cons_car();
            let entry_cdr = cursor.cons_cdr();
            if entry_car.is_cons() {
                let mapping_car = entry_car.cons_car();
                let mapping_cdr = entry_car.cons_cdr();
                if Self::face_name_from_value(&mapping_car).is_some_and(|name| name == face_name) {
                    return Some(mapping_cdr);
                }
            }
            cursor = entry_cdr;
        }
    }

    fn resolve_buffer_face_value_over_inner<B: LayoutBufferView>(
        &self,
        buffer: &B,
        base: &ResolvedFace,
        val: &Value,
        remap_stack: &mut Vec<String>,
    ) -> Option<ResolvedFace> {
        match val.kind() {
            ValueKind::Nil => None,
            ValueKind::Symbol(_) | ValueKind::String => {
                let name = Self::face_name_from_value(val)?;
                if name == "nil" {
                    return None;
                }

                if !remap_stack.iter().any(|active| active == name)
                    && let Some(specs) = Self::buffer_face_remapping_specs(buffer, name)
                {
                    remap_stack.push(name.to_string());
                    let remapped = self.resolve_buffer_face_value_over_inner(
                        buffer,
                        base,
                        &specs,
                        remap_stack,
                    );
                    remap_stack.pop();
                    if remapped.is_some() {
                        return remapped;
                    }
                }

                Some(self.apply_named_face_over(base, name))
            }
            ValueKind::Cons => {
                let items = list_to_vec(val)?;
                if items.is_empty() {
                    return None;
                }
                match self.eval_filtered_face_spec(&items) {
                    FilteredFaceSpec::Matched(filtered_spec) => {
                        // Recurse into the filtered spec (unwrap the :filtered wrapper)
                        return self.resolve_buffer_face_value_over_inner(
                            buffer,
                            base,
                            &Value::list(filtered_spec),
                            remap_stack,
                        );
                    }
                    // Filter didn't match → the remap contributes nothing.
                    FilteredFaceSpec::Rejected => return None,
                    FilteredFaceSpec::NotFiltered => {}
                }
                if Self::face_spec_is_plist(&items) {
                    let inline = NeoFace::from_plist("--inline--", &items);
                    return Some(self.apply_inline_face_over(base, &inline));
                }

                let mut current = base.clone();
                let mut changed = false;
                for item in items.iter().rev() {
                    if let Some(next) = self.resolve_buffer_face_value_over_inner(
                        buffer,
                        &current,
                        item,
                        remap_stack,
                    ) {
                        current = next;
                        changed = true;
                    }
                }
                changed.then_some(current)
            }
            _ => None,
        }
    }

    pub(crate) fn resolve_buffer_face_value_over<B: LayoutBufferView>(
        &self,
        buffer: &B,
        base: &ResolvedFace,
        val: &Value,
    ) -> Option<ResolvedFace> {
        let mut remap_stack = Vec::new();
        self.resolve_buffer_face_value_over_inner(buffer, base, val, &mut remap_stack)
    }

    pub(crate) fn resolve_buffer_default_face<B: LayoutBufferView>(
        &self,
        buffer: &B,
    ) -> ResolvedFace {
        let mut remap_stack = Vec::new();
        self.resolve_buffer_face_value_over_inner(
            buffer,
            &self.default_face,
            &Value::symbol("default"),
            &mut remap_stack,
        )
        .unwrap_or_else(|| self.default_face.clone())
    }

    pub fn resolve_face_value_over(
        &self,
        base: &ResolvedFace,
        val: &Value,
    ) -> Option<ResolvedFace> {
        match val.kind() {
            ValueKind::Nil => None,
            ValueKind::Symbol(_) => {
                let name = val.as_symbol_name()?;
                (name != "nil").then(|| self.apply_named_face_over(base, name))
            }
            ValueKind::Cons => {
                let items = list_to_vec(val)?;
                if items.is_empty() {
                    return None;
                }
                match self.eval_filtered_face_spec(&items) {
                    FilteredFaceSpec::Matched(filtered_spec) => {
                        return self.resolve_face_value_over(base, &Value::list(filtered_spec));
                    }
                    FilteredFaceSpec::Rejected => return None,
                    FilteredFaceSpec::NotFiltered => {}
                }
                if Self::face_spec_is_plist(&items) {
                    let inline = NeoFace::from_plist("--inline--", &items);
                    return Some(self.apply_inline_face_over(base, &inline));
                }

                let mut current = base.clone();
                let mut changed = false;
                for item in items.iter().rev() {
                    if let Some(next) = self.resolve_face_value_over(&current, item) {
                        current = next;
                        changed = true;
                    }
                }
                changed.then_some(current)
            }
            _ => None,
        }
    }

    /// Resolve face attributes at a buffer position.
    ///
    /// Reads "face" and "font-lock-face" text properties, collects overlay
    /// faces (sorted by priority), merges them via `FaceTable`, and produces
    /// a fully-realized `ResolvedFace`.
    ///
    /// `next_check` is set to the minimum of all property change positions
    /// so the caller can skip per-character lookups until that boundary.
    fn face_at_pos<B: LayoutBufferView>(
        &self,
        buffer: &B,
        charpos: usize,
        next_check: &mut usize,
    ) -> ResolvedFace {
        let bytepos = buffer_charpos_to_emacs_byte_pos(buffer, CharPos0::new(charpos));
        let mut min_next = buffer.layout_point_max_char_pos().get();
        let mut resolved = self.resolve_buffer_default_face(buffer);
        let mut remap_stack = Vec::new();

        // GNU redisplay asks for the effective `face` property.  When
        // font-lock mode is active, `font-lock-face` acts as a fallback face,
        // but an explicit `face` property wins.  This matters for
        // font-locking comments that cover propertized help key strings in
        // the initial scratch message.
        let face_prop = buffer.layout_text_prop_at_emacs_byte_pos(bytepos, Value::symbol("face"));
        let font_lock_face_prop =
            buffer.layout_text_prop_at_emacs_byte_pos(bytepos, Value::symbol("font-lock-face"));

        // 1. Text face property, with font-lock-face fallback.
        if let Some(val) = face_prop.or(font_lock_face_prop)
            && let Some(next) =
                self.resolve_buffer_face_value_over_inner(buffer, &resolved, &val, &mut remap_stack)
        {
            resolved = next;
        }
        // Update next_check from text property boundaries
        if let Some(nc) = buffer.layout_next_text_prop_change_after_emacs_byte_pos(bytepos) {
            min_next = min_next.min(buffer_emacs_byte_pos_to_charpos(buffer, nc));
        }

        // 2. Overlay faces — collected once by the shared resolver (ascending
        //    priority, windowed overlays filtered to this window). Merge each in
        //    order; higher priority wins by being applied last.
        let overlays = overlay_faces_at(buffer, bytepos, self.current_window_id.get());
        if let Some(boundary) = overlays.next_boundary {
            min_next = min_next.min(buffer_emacs_byte_pos_to_charpos(buffer, boundary));
        }
        for face_value in overlays.faces {
            if let Some(next) = self.resolve_buffer_face_value_over_inner(
                buffer,
                &resolved,
                &face_value,
                &mut remap_stack,
            ) {
                resolved = next;
            }
        }

        // Also consider overlay boundaries so next_check doesn't skip past
        // positions where an overlay starts or ends.
        if let Some(nb) = buffer
            .layout_overlays()
            .next_boundary_after_emacs_byte_pos(bytepos)
        {
            min_next = min_next.min(buffer_emacs_byte_pos_to_charpos(buffer, nb));
        }

        *next_check = min_next;
        resolved
    }

    /// Resolve the base face for an overlay `before-string' or
    /// `after-string'.
    ///
    /// GNU redisplay treats overlay strings specially: their base face depends
    /// only on text properties at the anchor position and ignores overlay
    /// faces/current iterator face.  The overlay string's own text properties
    /// are merged later by the Lisp string display-source iterator.
    pub(crate) fn face_for_overlay_string<B: LayoutBufferView>(
        &self,
        buffer: &B,
        anchor_charpos: usize,
        next_check: &mut usize,
    ) -> ResolvedFace {
        let bytepos = buffer_charpos_to_emacs_byte_pos(buffer, CharPos0::new(anchor_charpos));
        let mut min_next = buffer.layout_point_max_char_pos().get();
        let mut resolved = self.resolve_buffer_default_face(buffer);
        let mut remap_stack = Vec::new();

        if let Some(face_prop) =
            buffer.layout_text_prop_at_emacs_byte_pos(bytepos, Value::symbol("face"))
            && let Some(next) = self.resolve_buffer_face_value_over_inner(
                buffer,
                &resolved,
                &face_prop,
                &mut remap_stack,
            )
        {
            resolved = next;
        }

        if let Some(nc) = buffer.layout_next_text_prop_change_after_emacs_byte_pos(bytepos) {
            min_next = min_next.min(buffer_emacs_byte_pos_to_charpos(buffer, nc));
        }

        *next_check = min_next;
        resolved
    }

    pub(crate) fn base_face_for_origin<B: LayoutBufferView>(
        &self,
        buffer: Option<&B>,
        origin: &DisplayOrigin,
        policy: BaseFacePolicy,
        next_check: &mut usize,
    ) -> ResolvedFace {
        match policy {
            BaseFacePolicy::BufferFaceIncludingOverlays => {
                let buffer = buffer.expect("buffer text face policy requires a buffer");
                let DisplayOrigin::BufferText { charpos } = origin else {
                    unreachable!(
                        "buffer text face policy received incompatible origin: {origin:?}"
                    );
                };
                self.face_at_pos(buffer, charpos.get(), next_check)
            }
            BaseFacePolicy::OverlayStringAtAnchor => {
                let buffer = buffer.expect("overlay string face policy requires a buffer");
                let DisplayOrigin::OverlayString { anchor_charpos, .. } = origin else {
                    unreachable!(
                        "overlay string face policy received incompatible origin: {origin:?}"
                    );
                };
                self.face_for_overlay_string(buffer, anchor_charpos.get(), next_check)
            }
            BaseFacePolicy::DisplayPropertyUnderlyingFace => {
                let buffer = buffer.expect("display property face policy requires a buffer");
                let DisplayOrigin::DisplayPropertyString { anchor_charpos, .. } = origin else {
                    unreachable!(
                        "display property face policy received incompatible origin: {origin:?}"
                    );
                };
                self.face_at_pos(buffer, anchor_charpos.get(), next_check)
            }
            BaseFacePolicy::DefaultFace => self.default_face.clone(),
            BaseFacePolicy::FixedBasicFace(face_id) => self.resolve_named_face(face_id.name()),
        }
    }

    pub(crate) fn default_base_face_for_origin<B: LayoutBufferView>(
        &self,
        buffer: Option<&B>,
        origin: &DisplayOrigin,
        next_check: &mut usize,
    ) -> ResolvedFace {
        self.base_face_for_origin(
            buffer,
            origin,
            origin.default_base_face_policy(),
            next_check,
        )
    }

    pub(crate) fn default_base_face_for_origin_without_buffer(
        &self,
        origin: &DisplayOrigin,
    ) -> ResolvedFace {
        match origin.default_base_face_policy() {
            BaseFacePolicy::DefaultFace => self.default_face.clone(),
            BaseFacePolicy::FixedBasicFace(face_id) => self.resolve_named_face(face_id.name()),
            policy => {
                panic!(
                    "display origin {origin:?} requires a buffer for base face policy {policy:?}"
                )
            }
        }
    }

    /// Extract face name(s) from a Lisp Value.
    ///
    /// Face property values can be:
    /// - A symbol naming a face: `Value::Symbol(id)` -> `vec!["face-name"]`
    /// - A list of symbols: each element is a face name
    /// - Nil: no face
    /// - Otherwise: empty vec (unrecognized format)
    pub fn resolve_face_value(val: &Value) -> Vec<String> {
        match val.kind() {
            ValueKind::Nil => Vec::new(),
            ValueKind::Symbol(_) => {
                if let Some(name) = val.as_symbol_name() {
                    if name == "nil" {
                        Vec::new()
                    } else {
                        vec![name.to_string()]
                    }
                } else {
                    Vec::new()
                }
            }
            ValueKind::Cons => {
                // Could be a list of face names, or a plist of face attributes.
                if let Some(items) = list_to_vec(val) {
                    // Check if first item is a keyword (plist like :foreground "red")
                    if Self::face_spec_is_plist(&items) {
                        // Plist face — handled by face_at_pos via face_from_plist().
                        // Return a sentinel that face_at_pos recognizes.
                        vec!["--plist-face--".to_string()]
                    } else {
                        // List of face name symbols.
                        items
                            .iter()
                            .filter_map(|item| {
                                item.as_symbol_name()
                                    .filter(|n| *n != "nil")
                                    .map(|n| n.to_string())
                            })
                            .collect()
                    }
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        }
    }

    /// Parse an inline face plist like `(:foreground "red" :weight bold)` into
    /// a `Face` object.  Handles the same keywords as GNU Emacs face specs.
    pub fn face_from_plist(val: &Value) -> Option<NeoFace> {
        let items = list_to_vec(val)?;
        Some(NeoFace::from_plist("--inline--", &items))
    }

    /// Convert a neovm-core `Face` into a fully-realized `ResolvedFace`.
    ///
    /// Unset fields fall back to the default face.  Handles `inverse_video`,
    /// `FaceHeight` (absolute/relative), underline, overline, strike-through,
    /// box, overstrike, and extend.
    pub fn realize_face(&self, face: &NeoFace) -> ResolvedFace {
        let mut rf = self.default_face.clone();
        // The clone starts from the default face; an anonymous face must not
        // inherit its *name*. Named resolvers overwrite this after realizing.
        rf.lisp_name = None;

        // Foreground
        if let Some(c) = &face.foreground {
            rf.fg = color_to_pixel(c);
            rf.use_default_foreground = false;
        }
        // Background
        if let Some(c) = &face.background {
            rf.bg = color_to_pixel(c);
            rf.use_default_background = false;
        }
        // Inverse video: swap fg and bg
        match face.inverse_video {
            Some(true) => {
                rf.terminal_inverse_video = rf.use_default_foreground && rf.use_default_background;
                std::mem::swap(&mut rf.fg, &mut rf.bg);
                std::mem::swap(
                    &mut rf.use_default_foreground,
                    &mut rf.use_default_background,
                );
            }
            Some(false) => rf.terminal_inverse_video = false,
            None => {}
        }

        // Font family
        if let Some(family) = face.family_runtime_string_owned() {
            rf.font_family = family;
        }
        // Font weight
        if let Some(w) = &face.weight {
            rf.font_weight = w.css_weight();
        }
        // Font slant
        if let Some(s) = &face.slant {
            rf.italic = s.is_italic();
        }
        // Font height
        if let Some(h) = &face.height {
            match h {
                FaceHeight::Absolute(tenths) => {
                    rf.font_size = self.font_sizing.face_height_to_layout_pixels(*tenths);
                }
                FaceHeight::Relative(factor) => {
                    rf.font_size = self.default_face.font_size * (*factor as f32);
                }
            }
        }

        // Underline
        match &face.underline {
            FaceDecoration::Unspecified => {}
            FaceDecoration::Disabled => {
                rf.underline_style = 0;
                rf.underline_color = 0;
            }
            FaceDecoration::Enabled(underline) => {
                rf.underline_style = underline_style_to_u8(&underline.style);
                // GNU draws `:underline t` (no explicit color) in the face's
                // foreground -- e.g. `nobreak-space` inherits `escape-glyph`'s
                // brown fg and underlines in that same brown. Default the
                // unspecified color to `rf.fg` here (fg is applied earlier in
                // this merge), mirroring the box-border resolution below so a
                // downstream pixel-0 underline color means "explicit black"
                // uniformly with box.
                rf.underline_color = underline
                    .color
                    .as_ref()
                    .map(color_to_pixel)
                    .unwrap_or(rf.fg);
            }
        }
        // Overline
        if let Some(over) = face.overline {
            rf.overline = over;
            // GNU draws `:overline t` (no explicit color) in the face
            // foreground, like `:underline t` and `:box t`. Explicit
            // `:overline COLOR` (just below) overrides.
            if over {
                rf.overline_color = rf.fg;
            }
        }
        if let Some(c) = &face.overline_color {
            rf.overline_color = color_to_pixel(c);
        }
        // Strike-through
        if let Some(st) = face.strike_through {
            rf.strike_through = st;
            // Same rule for `:strike-through t` -> face foreground.
            if st {
                rf.strike_through_color = rf.fg;
            }
        }
        if let Some(c) = &face.strike_through_color {
            rf.strike_through_color = color_to_pixel(c);
        }
        // Box border
        if let Some(bb) = &face.box_border {
            rf.box_type = box_style_to_u8(&bb.style);
            rf.box_color = bb.color.as_ref().map(color_to_pixel).unwrap_or(rf.fg);
            rf.box_line_width = BoxLineWidth::from_gnu(bb.width);
        }
        // Extend
        if let Some(ext) = face.extend {
            rf.extend = ext;
        }
        // Overstrike
        if face.overstrike {
            rf.overstrike = true;
        }

        // Distant-foreground: GNU Emacs (xfaces.c) uses this when the
        // foreground is too close to the background, improving readability.
        // Check if fg ≈ bg and substitute distant-foreground if available.
        if let Some(dfg) = &face.distant_foreground {
            let dfg_pixel = color_to_pixel(dfg);
            if colors_close(rf.fg, rf.bg) {
                rf.fg = dfg_pixel;
                rf.use_default_foreground = false;
            }
        }

        // Stipple: realize the `:stipple` spec to the XBM pattern the renderer
        // tiles behind glyphs. GNU realizes `LFACE_STIPPLE_INDEX` to a pixmap
        // id in `face->stipple`; neomacs keeps the small bitmap on the face.
        if let Some(pat) = self.realize_stipple(face) {
            rf.stipple = Some(pat);
        }

        rf
    }

    /// Realize a face's inline `:stipple (WIDTH HEIGHT DATA)` bitmap spec into
    /// the XBM [`StipplePattern`](neomacs_display_protocol::StipplePattern) the
    /// renderer tiles. Returns `None` when the face specifies no stipple (or a
    /// non-inline file/symbol form, which is not yet loaded); callers keep the
    /// base face's stipple in that case, matching GNU merge semantics.
    fn realize_stipple(&self, face: &NeoFace) -> Option<neomacs_display_protocol::StipplePattern> {
        let spec = face.stipple.as_ref()?;
        // Inline `(WIDTH HEIGHT DATA)` bitmap spec (what `indent-bars` emits).
        if let Some(items) = list_to_vec(spec) {
            if items.len() == 3
                && let Some(w) = items[0].as_fixnum()
                && let Some(h) = items[1].as_fixnum()
                && let Some(data) = items[2].as_lisp_string()
                && w > 0
                && h > 0
            {
                return Some(neomacs_display_protocol::StipplePattern {
                    width: w as u32,
                    height: h as u32,
                    bits: data.as_bytes().to_vec(),
                });
            }
            return None;
        }
        // A named built-in bitmap (`gray`/`gray1`/`gray3`/...) or an XBM file.
        if let Some(name) = spec.as_utf8_str() {
            return stipple_from_name_or_file(name);
        }
        None
    }

    /// Resolve a face from a Lisp Value (as found in overlay "face" property).
    ///
    /// Returns None if the value doesn't specify any known face names.
    pub fn resolve_face_from_value(&self, val: &Value) -> Option<ResolvedFace> {
        self.resolve_face_value_over(&self.default_face, val)
    }
}

fn underline_style_to_u8(style: &NeoUnderlineStyle) -> u8 {
    style.gnu_code()
}

fn box_style_to_u8(style: &NeoBoxStyle) -> u8 {
    style.gnu_code()
}

#[cfg(test)]
#[path = "neovm_bridge_test.rs"]
mod tests;

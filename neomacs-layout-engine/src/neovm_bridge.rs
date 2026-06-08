//! Bridge between neovm-core data types and the layout engine.
//!
//! Provides functions to build `WindowParams` and `FrameParams` from
//! the Rust Context's state, replacing C FFI data sources.

use neovm_core::buffer::{
    Buffer, BufferTextSnapshot, CharPos0, EmacsByteLen, EmacsBytePos, EmacsByteRange,
    buffer::{BUFFER_SLOT_COUNT, lookup_buffer_slot},
    overlay::OverlayList,
};
use neovm_core::emacs_core::intern;
use neovm_core::emacs_core::symbol::Obarray;
use neovm_core::emacs_core::value::{ValueKind, eq_value, list_to_vec};
use neovm_core::emacs_core::{Context, Value};
use neovm_core::face::{
    BoxStyle as NeoBoxStyle, Color as NeoColor, Face as NeoFace, FaceHeight, FaceTable, FontWeight,
    UnderlineStyle as NeoUnderlineStyle,
};
use neovm_core::window::{
    CursorTypeSymbol, Frame, FrameId, VerticalScrollBarType, Window,
    resolve_window_scroll_bar_geometry,
};

use super::types::{FrameParams, VisualCursorSpec, WindowParams};
use crate::coords::{
    clamped_lisp_charpos_to_layout_i64, layout_char_pos_from_i64, layout_emacs_byte_pos_from_i64,
    lisp_char_pos_to_layout_i64, lisp_charpos_to_layout_char_pos,
};
use crate::fontconfig::FontSizing;
use neomacs_display_protocol::cursor::{CursorBarWidth, CursorKind, CursorSpec};
use neomacs_display_protocol::cursor_effect_command::{CursorEffectArg, CursorEffectCommand};
use neomacs_display_protocol::effect_config::EffectsConfig;
use neomacs_display_protocol::types::Rect;
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

    pub(crate) fn accessible_end_char_pos(&self) -> CharPos0 {
        self.accessible_end_char
    }

    pub(crate) fn accessible_end_emacs_byte_pos(&self) -> EmacsBytePos {
        self.accessible_end_emacs_byte
    }

    pub(crate) fn overlays(&self) -> &OverlayList {
        &self.overlays
    }
}

const LAYOUT_DEFAULT_VALUE_SYMBOLS: &[&str] = &[
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
    "tab-stop-list",
    "wrap-prefix",
];

fn layout_default_values(obarray: &Obarray) -> Vec<(neovm_core::emacs_core::intern::SymId, Value)> {
    LAYOUT_DEFAULT_VALUE_SYMBOLS
        .iter()
        .filter_map(|name| {
            let id = intern::intern(name);
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

    fn layout_overlays(&self) -> &OverlayList {
        &self.overlays
    }
}

pub(crate) fn buffer_local_value<B: LayoutBufferView>(buffer: &B, name: &str) -> Option<Value> {
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

/// Build `FrameParams` from a neovm-core `Frame`, reading default face
/// colors from the face table.
pub fn frame_params_from_neovm(frame: &Frame, face_table: &FaceTable) -> FrameParams {
    fn face_fg_pixel(face_table: &FaceTable, name: &str, fallback: u32) -> u32 {
        face_table
            .resolve(name)
            .foreground
            .map(|c| (c.r as u32) << 16 | (c.g as u32) << 8 | c.b as u32)
            .unwrap_or(fallback)
    }

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

fn global_nobreak_char_display(obarray: &Obarray) -> i32 {
    match obarray.symbol_value("nobreak-char-display") {
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

fn effective_truncate_lines(
    window: &Window,
    buffer: &Buffer,
    frame: &Frame,
    obarray: &Obarray,
    hscroll: usize,
) -> bool {
    if effective_buffer_bool(buffer, obarray, "truncate-lines") {
        return true;
    }

    // GNU `xdisp.c:init_iterator` only enables wrapping when the
    // window is not horizontally scrolled.
    if hscroll != 0 {
        return true;
    }

    let total_cols = window_total_cols(window, frame.char_width);
    let frame_cols = frame_total_cols(frame);

    if total_cols >= frame_cols {
        return false;
    }

    match effective_buffer_value(buffer, obarray, "truncate-partial-width-windows") {
        Some(value) if value.is_nil() => false,
        Some(value) if value.is_fixnum() => total_cols < value.as_fixnum().unwrap(),
        Some(_) => true,
        None => false,
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
    let box_pixels = if face.box_type != 0 && face.box_line_width != 0 {
        2.0 * face.box_line_width.unsigned_abs() as f32
    } else {
        0.0
    };
    let minimum_row_height = fallback_char_height.ceil().max(1.0);
    (line_height + box_pixels).max(minimum_row_height)
}

pub(crate) fn buffer_local_list_values<B: LayoutBufferView>(buffer: &B, name: &str) -> Vec<Value> {
    // `list_to_vec' takes `&Value'; feed the borrowed form since
    // `buffer_local_value' returns the `Copy' `Value' by value.
    buffer_local_value(buffer, name)
        .and_then(|v| list_to_vec(&v))
        .unwrap_or_default()
}

pub(crate) fn buffer_display_line_numbers_mode<B: LayoutBufferView>(
    buffer: &B,
) -> DisplayLineNumbersMode {
    DisplayLineNumbersMode::from_lisp_value(buffer_local_value(buffer, "display-line-numbers"))
}

fn buffer_fill_column_indicator<B: LayoutBufferView>(buffer: &B) -> Option<(i32, char)> {
    // GNU `fill_column_indicator_column` in xdisp.c enables the indicator only
    // when `display-fill-column-indicator` is non-nil, the indicator character
    // satisfies CHARACTERP, and the effective column is a nonnegative integer.
    if !buffer_local_bool(buffer, "display-fill-column-indicator") {
        return None;
    }

    let character_value = buffer_local_value(buffer, "display-fill-column-indicator-character")?;
    if !character_value.is_char() {
        return None;
    }
    let character = character_value.as_char()?;

    let column_value = match buffer_local_value(buffer, "display-fill-column-indicator-column") {
        Some(value) if value.bits() == Value::T.bits() => buffer_local_value(buffer, "fill-column"),
        value => value,
    }?;
    let column = column_value.as_fixnum()?;
    if column < 0 || column > i32::MAX as i64 {
        return None;
    }

    Some((column as i32, character))
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
    if frame.effective_window_system().is_some()
        && pixel == frame_background_color_pixel(frame, face_table)
    {
        frame_mouse_color_pixel(frame)
    } else {
        pixel
    }
}

fn disabled_cursor_effects_profile() -> EffectsConfig {
    let mut effects = EffectsConfig::default();
    effects.cursor_color_cycle.enabled = false;
    effects
}

fn cursor_effect_arg_from_lisp(value: Value) -> Option<CursorEffectArg> {
    if value.is_nil() {
        Some(CursorEffectArg::Nil)
    } else if value.bits() == Value::T.bits() {
        Some(CursorEffectArg::Bool(true))
    } else if let Some(text) = value.as_utf8_str() {
        Some(CursorEffectArg::String(text.to_owned()))
    } else {
        value.as_number_f64().map(CursorEffectArg::Number)
    }
}

fn cursor_effect_name_from_symbol(value: Value) -> Option<String> {
    let name = value.as_symbol_name()?;
    Some(
        name.strip_prefix("neomacs-set-cursor-")
            .unwrap_or(name)
            .to_owned(),
    )
}

fn apply_cursor_effect_form(effects: &mut EffectsConfig, form: Value) -> bool {
    if form.is_nil() {
        return false;
    }
    let values = if form.is_cons() {
        let Some(values) = list_to_vec(&form) else {
            return false;
        };
        values
    } else {
        vec![form]
    };
    let Some((head, args)) = values.split_first() else {
        return false;
    };
    let Some(name) = cursor_effect_name_from_symbol(*head) else {
        return false;
    };
    let Some(args) = args
        .iter()
        .copied()
        .map(cursor_effect_arg_from_lisp)
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    CursorEffectCommand::new(name, args).apply_to(effects);
    true
}

fn parse_cursor_effect_profile(value: Value) -> Option<EffectsConfig> {
    if value.is_nil() {
        return None;
    }
    let mut effects = disabled_cursor_effects_profile();
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
                any |= apply_cursor_effect_form(&mut effects, form);
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
        window_end_pos,
        window_end_valid,
        point,
        hscroll,
        margins,
        left_fringe_width,
        right_fringe_width,
    ) = match window {
        Window::Leaf {
            id,
            buffer_id,
            bounds,
            window_start,
            window_end_pos,
            window_end_valid,
            point,
            hscroll,
            margins,
            display,
            ..
        } => (
            *id,
            *buffer_id,
            bounds,
            *window_start,
            *window_end_pos,
            *window_end_valid,
            *point,
            *hscroll,
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
    let default_face = face_table.resolve("default");
    let default_fg = default_face
        .foreground
        .map(|color| color_to_pixel(&color))
        .unwrap_or(0x000000);
    let default_bg = default_face
        .background
        .map(|color| color_to_pixel(&color))
        .unwrap_or(0x00FFFFFF);
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
    let truncate_lines = effective_truncate_lines(window, buffer, frame, obarray, hscroll);
    let word_wrap = effective_buffer_bool(buffer, obarray, "word-wrap");
    let tab_width = effective_buffer_int(buffer, obarray, "tab-width", 8) as i32;

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
        let mode_line_face_name = if is_selected {
            "mode-line-active"
        } else {
            "mode-line-inactive"
        };
        chrome_face_pixel_height(
            &face_resolver.resolve_named_face(mode_line_face_name),
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
    let fill_column_indicator = buffer_fill_column_indicator(buffer);

    let header_line_height = if wants_header_line {
        let header_line_face_name = if is_selected {
            "header-line-active"
        } else {
            "header-line-inactive"
        };
        chrome_face_pixel_height(
            &face_resolver.resolve_named_face(header_line_face_name),
            char_height,
        )
    } else {
        0.0
    };

    let tab_line_height = if wants_tab_line {
        chrome_face_pixel_height(&face_resolver.resolve_named_face("tab-line"), char_height)
    } else {
        0.0
    };

    Some(WindowParams {
        window_id: win_id.0 as i64,
        buffer_id: buffer.id().0,
        bounds: display_bounds,
        text_bounds,
        selected: is_selected,
        is_minibuffer,
        // Window::window_start tracks GNU marker positions (1-based).
        // Normalize to the layout engine's internal 0-based char positions.
        window_start: lisp_char_pos_to_layout_i64(window_start),
        // Previous visible end converted back to the layout engine's internal
        // 0-based char position space.  GNU stores this as an offset from Z.
        window_end: if window_end_valid {
            buffer
                .point_max_char_pos()
                .get()
                .saturating_add(1)
                .saturating_sub(window_end_pos)
                .saturating_sub(1) as i64
        } else {
            0
        },
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
        vscroll: 0,
        truncate_lines,
        word_wrap,
        tab_width,
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
        indicate_empty_lines: if buffer_local_bool(buffer, "indicate-empty-lines") {
            1
        } else {
            0
        },
        show_trailing_whitespace: buffer_local_bool(buffer, "show-trailing-whitespace"),
        trailing_ws_bg: 0,
        fill_column_indicator: fill_column_indicator
            .map(|(column, _)| column)
            .unwrap_or(-1),
        fill_column_indicator_char: fill_column_indicator
            .map(|(_, character)| character)
            .unwrap_or('|'),
        fill_column_indicator_fg: 0,
        extra_line_spacing: match buffer_local_value(buffer, "line-spacing") {
            Some(v) if v.is_fixnum() => v.as_fixnum().unwrap() as f32,
            Some(v) if v.is_float() => v.xfloat() as f32,
            _ => 0.0,
        },
        selective_display: buffer_selective_display(buffer),
        escape_glyph_fg: 0,
        nobreak_char_display: global_nobreak_char_display(obarray),
        nobreak_char_fg: 0,
        glyphless_char_fg: 0,
        wrap_prefix: Vec::new(),
        line_prefix: Vec::new(),
        left_margin_width: left_margin,
        right_margin_width: right_margin,
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
    let frame_params = frame_params_from_neovm(frame, evaluator.face_table());

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

    /// Convert an absolute byte position to the layout engine's internal
    /// 0-based char position space.
    pub fn bytepos_to_charpos(&self, bytepos: i64) -> i64 {
        let Some(bytepos) = layout_emacs_byte_pos_from_i64(bytepos) else {
            return 0;
        };
        buffer_emacs_byte_pos_to_charpos(self.buffer, bytepos) as i64
    }

    /// Get the buffer's narrowed end (zv) as byte position.
    pub fn zv(&self) -> i64 {
        self.buffer.layout_point_max_emacs_byte_pos().get() as i64
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
        let bytepos = buffer_i64_charpos_to_emacs_byte_pos(self.buffer, charpos);
        let text_invis = self
            .buffer
            .layout_text_prop_at_emacs_byte_pos(bytepos, Value::symbol("invisible"));
        let mut status = InvisibleStatus::VISIBLE;
        let spec = self
            .buffer
            .layout_buffer_local_value("buffer-invisibility-spec")
            .unwrap_or(Value::T);
        if let Some(value) = text_invis {
            status = invisible_prop_status(Some(value), spec);
        }
        if !status.hidden {
            let mut overlay_ids = self
                .buffer
                .layout_overlays()
                .overlays_at_emacs_byte_pos(bytepos);
            self.buffer
                .layout_overlays()
                .sort_overlay_ids_by_priority_desc(&mut overlay_ids);
            for oid in overlay_ids {
                let overlay_invis = self
                    .buffer
                    .layout_overlays()
                    .overlay_get_named(oid, Value::symbol("invisible"));
                status = invisible_prop_status(overlay_invis, spec);
                if status.hidden {
                    break;
                }
            }
        }

        // Find the next position where the invisible property changes
        let next_text_change = self
            .buffer
            .layout_next_text_prop_change_after_emacs_byte_pos(bytepos)
            .map(|next| buffer_emacs_byte_pos_to_charpos(self.buffer, next))
            .unwrap_or_else(|| self.buffer.layout_point_max_char_pos().get());
        let next_overlay_change = self
            .buffer
            .layout_overlays()
            .next_boundary_after_emacs_byte_pos(bytepos)
            .map(|next| buffer_emacs_byte_pos_to_charpos(self.buffer, next))
            .unwrap_or_else(|| self.buffer.layout_point_max_char_pos().get());
        let next_change = next_text_change.min(next_overlay_change);

        (status, next_change as i64)
    }

    /// Check for a display text property at `charpos`.
    ///
    /// Returns the display property value if present, along with the
    /// next position where display properties change.
    pub fn check_display_prop(&self, charpos: i64) -> (Option<Value>, i64) {
        let bytepos = buffer_i64_charpos_to_emacs_byte_pos(self.buffer, charpos);
        let display = self
            .buffer
            .layout_text_prop_at_emacs_byte_pos(bytepos, Value::symbol("display"));

        let next_change = self
            .buffer
            .layout_next_text_prop_change_after_emacs_byte_pos(bytepos)
            .map(|next| buffer_emacs_byte_pos_to_charpos(self.buffer, next))
            .unwrap_or_else(|| self.buffer.layout_point_max_char_pos().get());

        (display, next_change as i64)
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
    pub fn overlay_strings_at(
        &self,
        charpos: i64,
    ) -> (Vec<OverlayDisplayString>, Vec<OverlayDisplayString>) {
        let bytepos = buffer_i64_charpos_to_emacs_byte_pos(self.buffer, charpos);
        let mut before = Vec::new();
        let mut after = Vec::new();

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
            if self
                .buffer
                .layout_overlays()
                .overlay_start_emacs_byte_pos(oid)
                == Some(bytepos)
            {
                if let Some(val) = self
                    .buffer
                    .layout_overlays()
                    .overlay_get_named(oid, Value::symbol("before-string"))
                    && val.is_string()
                {
                    before.push(OverlayDisplayString {
                        string: val,
                        overlay_id: oid,
                    });
                }
            }

            if self
                .buffer
                .layout_overlays()
                .overlay_end_emacs_byte_pos(oid)
                == Some(bytepos)
            {
                if let Some(val) = self
                    .buffer
                    .layout_overlays()
                    .overlay_get_named(oid, Value::symbol("after-string"))
                    && val.is_string()
                {
                    after.push(OverlayDisplayString {
                        string: val,
                        overlay_id: oid,
                    });
                }
            }
        }

        before.sort_by(|left, right| {
            overlay_string_priority(left.overlay_id).cmp(&overlay_string_priority(right.overlay_id))
        });
        after.sort_by(|left, right| {
            overlay_string_priority(right.overlay_id).cmp(&overlay_string_priority(left.overlay_id))
        });

        (before, after)
    }

    fn overlay_applies_to_window(&self, overlay_id: Value) -> bool {
        let Some(window_prop) = self
            .buffer
            .layout_overlays()
            .overlay_get_named(overlay_id, Value::symbol("window"))
        else {
            return true;
        };
        let Some(target_window_id) = window_prop.as_window_id() else {
            return true;
        };
        self.window_id
            .is_none_or(|current_window_id| target_window_id == current_window_id)
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
    ((c.r as u32) << 16) | ((c.g as u32) << 8) | (c.b as u32)
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
    let dr = ar.abs_diff(br) as u32;
    let dg = ag.abs_diff(bg) as u32;
    let db = ab.abs_diff(bb) as u32;
    // Weighted Euclidean distance (human perception weights R more than B)
    // Threshold ~30 in each channel ≈ 2700 squared distance
    (dr * dr * 3 + dg * dg * 4 + db * db * 2) < 3000
}

/// Resolved face attributes ready for the layout engine.
///
/// This is the neovm-core equivalent of `FaceDataFFI`.  All attributes are
/// fully realized (no `Option`s) so the layout engine can use them directly.
#[derive(Clone, Debug)]
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
    /// Box line width.
    pub box_line_width: i32,
    /// Extend background to end of line.
    pub extend: bool,
    /// Simulate bold by drawing twice at x and x+1.
    pub overstrike: bool,
    /// Preserve terminal inverse-video when both colors are terminal defaults.
    pub terminal_inverse_video: bool,
    /// Per-face character advance width (from FontMetricsService, 0.0 = use default).
    pub font_char_width: f32,
    /// Per-face font ascent (from FontMetricsService, 0.0 = use default).
    pub font_ascent: f32,
    /// Per-face line height (from FontMetricsService, 0.0 = use default).
    pub font_line_height: f32,
    /// Face cache ID — matches [`BasicFaceId`] for basic faces (0–19)
    /// or a dynamically allocated ID (≥20) for other faces.
    pub face_id: u32,
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
            box_line_width: 0,
            extend: false,
            overstrike: false,
            terminal_inverse_video: false,
            font_char_width: 0.0,
            font_ascent: 0.0,
            font_line_height: 0.0,
            face_id: 0, // DEFAULT_FACE_ID
        }
    }
}

// ---------------------------------------------------------------------------
// FaceResolver
// ---------------------------------------------------------------------------

/// Resolves face attributes at buffer positions using the neovm-core
/// `FaceTable`, text properties, and overlays.
///
/// Replaces the C FFI `face_at_buffer_position()` path for the pure-Rust
/// backend.
pub struct FaceResolver {
    face_table: FaceTable,
    default_face: ResolvedFace,
    /// Next dynamic face ID.  Basic faces occupy 0–19 (matching
    /// [`BasicFaceId`]); dynamically realized faces start at 20+.
    next_dynamic_id: std::cell::Cell<u32>,
    /// Window system in use: `None` for TTY, `Some("x")` for X11,
    /// `Some("wayland")` for Wayland, etc.  Used to evaluate
    /// `:filtered` face spec predicates.
    window_system: Option<String>,
    font_sizing: FontSizing,
}

impl FaceResolver {
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
        if let Some(ul) = &neo_default.underline {
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
            df.box_line_width = bb.width;
        }

        Self {
            face_table: face_table.clone(),
            default_face: df,
            next_dynamic_id: std::cell::Cell::new(
                neomacs_display_protocol::face::BasicFaceId::SENTINEL,
            ),
            window_system,
            font_sizing,
        }
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
        if let Some(basic) = BasicFaceId::from_name(name) {
            resolved.face_id = basic.into();
        } else {
            let id = self.next_dynamic_id.get();
            self.next_dynamic_id.set(id + 1);
            resolved.face_id = id;
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
        if let Some(basic) = BasicFaceId::from_name(name) {
            resolved.face_id = basic.into();
        } else {
            let id = self.next_dynamic_id.get();
            self.next_dynamic_id.set(id + 1);
            resolved.face_id = id;
        }
        resolved.terminal_inverse_video = false;
        resolved
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
        let mut rf = base_after_inherit;

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

        if let Some(underline) = &face.underline {
            rf.underline_style = underline_style_to_u8(&underline.style);
            rf.underline_color = underline.color.as_ref().map(color_to_pixel).unwrap_or(0);
        }
        if let Some(overline) = face.overline {
            rf.overline = overline;
        }
        if let Some(color) = &face.overline_color {
            rf.overline_color = color_to_pixel(color);
        }
        if let Some(strike) = face.strike_through {
            rf.strike_through = strike;
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
            rf.box_line_width = box_border.width;
        }
        if let Some(extend) = face.extend {
            rf.extend = extend;
        }
        if face.overstrike {
            rf.overstrike = true;
        }

        // Distant-foreground: swap fg when too close to bg
        if let Some(dfg) = &face.distant_foreground {
            if colors_close(rf.fg, rf.bg) {
                rf.fg = color_to_pixel(dfg);
                rf.use_default_foreground = false;
            }
        }

        rf
    }

    fn apply_named_face_over(&self, base: &ResolvedFace, name: &str) -> ResolvedFace {
        let resolved = self.resolve_named_face(name);
        let default = self.default_face();
        let mut merged = base.clone();

        if resolved.fg != default.fg
            || resolved.use_default_foreground != default.use_default_foreground
        {
            merged.fg = resolved.fg;
            merged.use_default_foreground = resolved.use_default_foreground;
        }
        if resolved.bg != default.bg
            || resolved.use_default_background != default.use_default_background
        {
            merged.bg = resolved.bg;
            merged.use_default_background = resolved.use_default_background;
        }
        if !resolved.font_family.is_empty() && resolved.font_family != default.font_family {
            merged.font_family = resolved.font_family;
        }
        if resolved.font_weight != default.font_weight {
            merged.font_weight = resolved.font_weight;
        }
        if resolved.italic != default.italic {
            merged.italic = resolved.italic;
        }
        if resolved.terminal_inverse_video != default.terminal_inverse_video {
            merged.terminal_inverse_video = resolved.terminal_inverse_video;
        }
        if (resolved.font_size - default.font_size).abs() > f32::EPSILON {
            merged.font_size = resolved.font_size;
        }
        if resolved.underline_style != default.underline_style {
            merged.underline_style = resolved.underline_style;
            merged.underline_color = resolved.underline_color;
        }
        if resolved.strike_through != default.strike_through {
            merged.strike_through = resolved.strike_through;
            merged.strike_through_color = resolved.strike_through_color;
        }
        if resolved.overline != default.overline {
            merged.overline = resolved.overline;
            merged.overline_color = resolved.overline_color;
        }
        if resolved.box_type != default.box_type {
            merged.box_type = resolved.box_type;
            merged.box_color = resolved.box_color;
            merged.box_line_width = resolved.box_line_width;
        }
        if resolved.extend != default.extend {
            merged.extend = resolved.extend;
        }
        if resolved.overstrike != default.overstrike {
            merged.overstrike = resolved.overstrike;
        }

        merged
    }

    fn face_name_from_value<'a>(value: &'a Value) -> Option<&'a str> {
        match value.kind() {
            ValueKind::Symbol(_) => value.as_symbol_name(),
            ValueKind::String => value.as_utf8_str(),
            _ => None,
        }
    }

    /// If `items` is a `(:filtered FILTER . FACE-SPEC)` form, evaluate
    /// FILTER against the current frame context.  Returns `Some(face_spec)`
    /// when the filter matches, `None` when it doesn't, or `None` when
    /// `items` is not a `:filtered` form at all (caller should treat it as
    /// an inline face plist or face list).
    ///
    /// Supported filter predicates:
    ///   `:window-system SYM`  — matches when `self.window_system == SYM`
    ///                          (nil for TTY, "x" for X11, etc.)
    fn eval_filtered_face_spec(&self, items: &[Value]) -> Option<Vec<Value>> {
        let first = items.first()?;
        let name = if first.is_keyword() {
            first.as_symbol_name()?
        } else {
            first.as_symbol_name()?
        };
        if name != "filtered" && name != ":filtered" {
            return None; // not a :filtered form — caller handles
        }
        if items.len() < 3 {
            return None; // malformed: need (:filtered FILTER . SPEC)
        }

        let filter = &items[1];
        let spec = &items[2..];

        // Evaluate filter predicates.  All predicates in the filter
        // plist must pass; this mirrors GNU's `face_spec_match_p` in
        // `src/xfaces.c`.
        match filter.kind() {
            ValueKind::Cons => {
                let filter_items = list_to_vec(filter).unwrap_or_default();
                let mut i = 0;
                while i < filter_items.len() {
                    let pred = filter_items.get(i)?;
                    let pred_name = if pred.is_keyword() {
                        pred.as_symbol_name()?
                    } else {
                        pred.as_symbol_name()?
                    };
                    match pred_name {
                        ":window-system" | "window-system" => {
                            i += 1;
                            let val = filter_items.get(i)?;
                            let ws_name = val.as_symbol_name().unwrap_or("");
                            let current = self.window_system.as_deref().unwrap_or("nil");
                            if current != ws_name && ws_name != "nil" {
                                return None; // filter rejected
                            }
                            if ws_name == "nil" && self.window_system.is_some() {
                                return None; // TTY filter, but we're on GUI
                            }
                        }
                        _ => {
                            // Unknown predicate — skip conservatively
                            // (matches GNU: unknown predicates fail)
                            return None;
                        }
                    }
                    i += 1;
                }
                Some(spec.to_vec())
            }
            _ => {
                // Non-list filter — malformed, skip
                None
            }
        }
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

    fn resolve_buffer_face_value_over<B: LayoutBufferView>(
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
                    let remapped =
                        self.resolve_buffer_face_value_over(buffer, base, &specs, remap_stack);
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
                if let Some(filtered_spec) = self.eval_filtered_face_spec(&items) {
                    if filtered_spec.is_empty() {
                        return None;
                    }
                    // Recurse into the filtered spec (unwrap the :filtered wrapper)
                    return self.resolve_buffer_face_value_over(
                        buffer,
                        base,
                        &Value::list(filtered_spec),
                        remap_stack,
                    );
                }
                if Self::face_spec_is_plist(&items) {
                    let inline = NeoFace::from_plist("--inline--", &items);
                    return Some(self.apply_inline_face_over(base, &inline));
                }

                let mut current = base.clone();
                let mut changed = false;
                for item in items.iter().rev() {
                    if let Some(next) =
                        self.resolve_buffer_face_value_over(buffer, &current, item, remap_stack)
                    {
                        current = next;
                        changed = true;
                    }
                }
                changed.then_some(current)
            }
            _ => None,
        }
    }

    fn resolve_buffer_default_face<B: LayoutBufferView>(&self, buffer: &B) -> ResolvedFace {
        let mut remap_stack = Vec::new();
        self.resolve_buffer_face_value_over(
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
                if let Some(filtered_spec) = self.eval_filtered_face_spec(&items) {
                    if filtered_spec.is_empty() {
                        return None;
                    }
                    return self.resolve_face_value_over(base, &Value::list(filtered_spec));
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
    pub(crate) fn face_at_pos<B: LayoutBufferView>(
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
        if let Some(val) = face_prop.or(font_lock_face_prop) {
            if let Some(next) =
                self.resolve_buffer_face_value_over(buffer, &resolved, &val, &mut remap_stack)
            {
                resolved = next;
            }
        }
        // Update next_check from text property boundaries
        if let Some(nc) = buffer.layout_next_text_prop_change_after_emacs_byte_pos(bytepos) {
            min_next = min_next.min(buffer_emacs_byte_pos_to_charpos(buffer, nc));
        }

        // 2. Overlay faces (sorted by priority, lowest first)
        let overlay_ids = buffer.layout_overlays().overlays_at_emacs_byte_pos(bytepos);
        if !overlay_ids.is_empty() {
            let mut overlay_faces: Vec<(i64, Value)> = Vec::new();
            for oid in &overlay_ids {
                let oid = *oid;
                // Update next_check from overlay boundaries
                if let Some(end) = buffer.layout_overlays().overlay_end_emacs_byte_pos(oid) {
                    if end > bytepos {
                        min_next = min_next.min(buffer_emacs_byte_pos_to_charpos(buffer, end));
                    }
                }
                // Get priority (default 0)
                let priority = buffer
                    .layout_overlays()
                    .overlay_get_named(oid, Value::symbol("priority"))
                    .and_then(|v| v.as_int())
                    .unwrap_or(0);
                // Get face
                if let Some(val) = buffer
                    .layout_overlays()
                    .overlay_get_named(oid, Value::symbol("face"))
                {
                    overlay_faces.push((priority, val));
                }
            }
            // Sort by priority (ascending), so higher priority overlays
            // are merged later and override earlier ones.
            overlay_faces.sort_by_key(|(pri, _)| *pri);
            for (_pri, face_value) in overlay_faces {
                if let Some(next) = self.resolve_buffer_face_value_over(
                    buffer,
                    &resolved,
                    &face_value,
                    &mut remap_stack,
                ) {
                    resolved = next;
                }
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
        if let Some(ul) = &face.underline {
            rf.underline_style = underline_style_to_u8(&ul.style);
            rf.underline_color = ul.color.as_ref().map(color_to_pixel).unwrap_or(0);
        }
        // Overline
        if let Some(over) = face.overline {
            rf.overline = over;
        }
        if let Some(c) = &face.overline_color {
            rf.overline_color = color_to_pixel(c);
        }
        // Strike-through
        if let Some(st) = face.strike_through {
            rf.strike_through = st;
        }
        if let Some(c) = &face.strike_through_color {
            rf.strike_through_color = color_to_pixel(c);
        }
        // Box border
        if let Some(bb) = &face.box_border {
            rf.box_type = box_style_to_u8(&bb.style);
            rf.box_color = bb.color.as_ref().map(color_to_pixel).unwrap_or(rf.fg);
            rf.box_line_width = bb.width;
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

        rf
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

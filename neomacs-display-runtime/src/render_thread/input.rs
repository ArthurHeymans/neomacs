//! Input translation and window chrome hit-testing.

use crate::backend::wgpu::{NEOMACS_CTRL_MASK, NEOMACS_META_MASK, NEOMACS_SUPER_MASK};
use winit::keyboard::{Key, NamedKey};

use super::RenderApp;
use super::frame_windows::GuiFrameWindowState;
use super::state::WindowChrome;

impl RenderApp {
    /// Translate winit key to X11 keysym
    pub(super) fn translate_key(key: &Key) -> u32 {
        match key {
            Key::Named(named) => match named {
                // Function keys
                NamedKey::F1 => 0xffbe,
                NamedKey::F2 => 0xffbf,
                NamedKey::F3 => 0xffc0,
                NamedKey::F4 => 0xffc1,
                NamedKey::F5 => 0xffc2,
                NamedKey::F6 => 0xffc3,
                NamedKey::F7 => 0xffc4,
                NamedKey::F8 => 0xffc5,
                NamedKey::F9 => 0xffc6,
                NamedKey::F10 => 0xffc7,
                NamedKey::F11 => 0xffc8,
                NamedKey::F12 => 0xffc9,
                // Navigation
                NamedKey::Escape => 0xff1b,
                NamedKey::Enter => 0xff0d,
                NamedKey::Tab => 0xff09,
                NamedKey::Backspace => 0xff08,
                NamedKey::Delete => 0xffff,
                NamedKey::Insert => 0xff63,
                NamedKey::Home => 0xff50,
                NamedKey::End => 0xff57,
                NamedKey::PageUp => 0xff55,
                NamedKey::PageDown => 0xff56,
                NamedKey::ArrowLeft => 0xff51,
                NamedKey::ArrowUp => 0xff52,
                NamedKey::ArrowRight => 0xff53,
                NamedKey::ArrowDown => 0xff54,
                // Whitespace
                NamedKey::Space => 0x20,
                // Modifier keys are handled via ModifiersChanged, not as key events.
                // They fall through to the default `_ => 0` which suppresses them.
                // Other
                NamedKey::PrintScreen => 0xff61,
                NamedKey::ScrollLock => 0xff14,
                NamedKey::Pause => 0xff13,
                _ => 0,
            },
            Key::Character(c) => c.chars().next().map(|ch| ch as u32).unwrap_or(0),
            _ => 0,
        }
    }

    /// Prefer committed text over logical-key fallback for printable input
    /// when no command modifiers are active.
    pub(super) fn translate_committed_text(text: &str, modifiers: u32) -> Option<Vec<u32>> {
        let command_modifiers_active =
            modifiers & (NEOMACS_CTRL_MASK | NEOMACS_META_MASK | NEOMACS_SUPER_MASK) != 0;
        if command_modifiers_active {
            return None;
        }

        let keysyms: Vec<u32> = text
            .chars()
            .filter(|ch| !ch.is_control())
            .map(|ch| ch as u32)
            .filter(|keysym| *keysym != 0)
            .collect();

        if keysyms.is_empty() {
            None
        } else {
            Some(keysyms)
        }
    }

    /// Return whether a `KeyboardInput` event should use its committed-text
    /// payload before falling back to its logical key.
    ///
    /// GNU's GUI backends classify physical function keys like Backspace from
    /// their keysyms first. Some window systems also attach control text such
    /// as `\b` to that same key event; using the text first would turn
    /// Backspace into `C-h` and bypass GNU's `[backspace] -> DEL` translation.
    pub(super) fn should_use_committed_text(logical_key: &Key) -> bool {
        matches!(logical_key, Key::Character(_))
    }

    /// Extract a single control-character keysym from committed text.
    ///
    /// Some backends report `Ctrl+n` / `Ctrl+p` style input as a control-text
    /// payload even when modifier-state delivery is delayed relative to the key
    /// event. Preserve that byte so the keyboard layer can recover the GNU
    /// control event instead of silently degrading it into plain text.
    pub(super) fn translate_control_text(text: &str) -> Option<u32> {
        let mut chars = text.chars();
        let ch = chars.next()?;
        if chars.next().is_some() {
            return None;
        }
        if ch.is_control() {
            Some(ch as u32)
        } else {
            None
        }
    }

    /// Hit-test toolbar items. Returns the index of the item under (x, y), or None.
    /// The y coordinate is local to the toolbar row.
    pub(super) fn toolbar_hit_test(&self, x: f32, y: f32) -> Option<u32> {
        let Some(tool_bar) = self.tool_bar.as_ref() else {
            return None;
        };
        if tool_bar.height <= 0.0 || y >= tool_bar.height {
            return None;
        }
        let padding = self.toolbar_padding as f32;
        let icon_size = self.toolbar_icon_size as f32;
        let item_size = icon_size + padding * 2.0;
        let separator_width = 12.0_f32;
        let item_spacing = 2.0_f32;

        let mut item_x = padding;
        for item in &tool_bar.items {
            if item.is_separator {
                item_x += separator_width;
                continue;
            }
            let right = item_x + item_size;
            if x >= item_x && x < right {
                return Some(item.index);
            }
            item_x = right + item_spacing;
        }
        None
    }

    pub(super) fn toolbar_y_origin(&self) -> f32 {
        if let Some(tab_bar) = self.tab_bar.as_ref().filter(|tab_bar| tab_bar.height > 0.0) {
            tab_bar.y + tab_bar.height
        } else {
            self.menu_bar
                .as_ref()
                .map_or(0.0, |menu_bar| menu_bar.height)
                + self
                    .compact_bar
                    .as_ref()
                    .map_or(0.0, |compact_bar| compact_bar.height)
        }
    }

    pub(super) fn menu_bar_height(&self) -> f32 {
        self.menu_bar
            .as_ref()
            .map_or(0.0, |menu_bar| menu_bar.height)
    }

    pub(super) fn tool_bar_height(&self) -> f32 {
        self.tool_bar
            .as_ref()
            .map_or(0.0, |tool_bar| tool_bar.height)
    }

    pub(super) fn compact_bar_height(&self) -> f32 {
        self.compact_bar
            .as_ref()
            .map_or(0.0, |compact_bar| compact_bar.height)
    }

    pub(super) fn tab_bar_y(&self) -> f32 {
        self.tab_bar.as_ref().map_or(0.0, |tab_bar| tab_bar.y)
    }

    pub(super) fn tab_bar_height(&self) -> f32 {
        self.tab_bar.as_ref().map_or(0.0, |tab_bar| tab_bar.height)
    }

    pub(super) fn compact_bar_menu_width(&self) -> f32 {
        let padding_x = 8.0_f32;
        let char_width = self
            .primary_frame
            .as_ref()
            .map_or(8.0, |frame| frame.glyph_atlas.default_char_width());
        let menu_width = self
            .compact_bar
            .as_ref()
            .map_or([].as_slice(), |compact_bar| {
                compact_bar.menu_items.as_slice()
            })
            .iter()
            .fold(padding_x, |x, item| {
                x + item.label.len() as f32 * char_width + padding_x * 2.0
            });
        menu_width + padding_x
    }

    pub(super) fn compact_bar_menu_hit_test(&self, x: f32, y: f32) -> Option<u32> {
        let Some(compact_bar) = self.compact_bar.as_ref() else {
            return None;
        };
        if compact_bar.height <= 0.0 || y >= compact_bar.height || compact_bar.menu_items.is_empty()
        {
            return None;
        }
        let padding_x = 8.0_f32;
        let char_width = self
            .primary_frame
            .as_ref()
            .map_or(8.0, |frame| frame.glyph_atlas.default_char_width());
        let mut item_x = padding_x;
        for item in &compact_bar.menu_items {
            let label_width = item.label.len() as f32 * char_width + padding_x * 2.0;
            if x >= item_x && x < item_x + label_width {
                return Some(item.index);
            }
            item_x += label_width;
        }
        None
    }

    pub(super) fn compact_bar_tool_hit_test(&self, x: f32, y: f32) -> Option<u32> {
        let Some(compact_bar) = self.compact_bar.as_ref() else {
            return None;
        };
        if compact_bar.height <= 0.0 || y >= compact_bar.height || compact_bar.tool_items.is_empty()
        {
            return None;
        }
        let x = x - self.compact_bar_menu_width();
        if x < 0.0 {
            return None;
        }
        let padding = self.toolbar_padding as f32;
        let icon_size = self.toolbar_icon_size as f32;
        let item_size = icon_size + padding * 2.0;
        let separator_width = 12.0_f32;
        let item_spacing = 2.0_f32;
        let mut item_x = padding;
        for item in &compact_bar.tool_items {
            if item.is_separator {
                item_x += separator_width;
                continue;
            }
            let right = item_x + item_size;
            if x >= item_x && x < right {
                return Some(item.index);
            }
            item_x = right + item_spacing;
        }
        None
    }

    /// Hit-test tab bar items. Returns the index of the item under (x, y), or None.
    pub(super) fn tab_bar_hit_test(&self, x: f32, y: f32) -> Option<u32> {
        let Some(tab_bar) = self.tab_bar.as_ref() else {
            return None;
        };
        if tab_bar.height <= 0.0 || tab_bar.items.is_empty() {
            return None;
        }
        if y < tab_bar.y || y >= tab_bar.y + tab_bar.height {
            return None;
        }
        let padding_x = 8.0_f32;
        let tab_padding = 12.0_f32;
        let char_width = self
            .primary_frame
            .as_ref()
            .map_or(8.0, |frame| frame.glyph_atlas.default_char_width());

        let mut tab_x = padding_x;
        for item in &tab_bar.items {
            if item.is_separator {
                tab_x += 12.0;
                continue;
            }
            let tab_width = item.label.len() as f32 * char_width + tab_padding * 2.0;
            if x >= tab_x && x < tab_x + tab_width {
                return Some(item.index);
            }
            tab_x += tab_width + 2.0;
        }
        None
    }

    /// Hit-test menu bar items. Returns the index of the item under (x, y), or None.
    pub(super) fn menu_bar_hit_test(&self, x: f32, _y: f32) -> Option<u32> {
        let Some(menu_bar) = self.menu_bar.as_ref() else {
            return None;
        };
        if menu_bar.height <= 0.0 || menu_bar.items.is_empty() {
            return None;
        }
        let padding_x = 8.0_f32;
        let char_width = self
            .primary_frame
            .as_ref()
            .map_or(8.0, |frame| frame.glyph_atlas.default_char_width());

        let mut item_x = padding_x;
        for item in &menu_bar.items {
            let label_width = item.label.len() as f32 * char_width + padding_x * 2.0;
            if x >= item_x && x < item_x + label_width {
                return Some(item.index);
            }
            item_x += label_width;
        }
        None
    }

    /// Detect if the mouse is on a resize edge of a borderless window.
    /// Returns the resize direction if within the border zone, or None.
    pub(super) fn detect_resize_edge_for_chrome(
        chrome: &WindowChrome,
        logical_width: f32,
        logical_height: f32,
        x: f32,
        y: f32,
    ) -> Option<winit::window::ResizeDirection> {
        use winit::window::ResizeDirection;
        if chrome.decorations_enabled {
            return None;
        }
        let w = logical_width;
        let h = logical_height;
        let border = 5.0_f32;
        let on_left = x < border;
        let on_right = x >= w - border;
        let on_top = y < border;
        let on_bottom = y >= h - border;
        match (on_left, on_right, on_top, on_bottom) {
            (true, _, true, _) => Some(ResizeDirection::NorthWest),
            (_, true, true, _) => Some(ResizeDirection::NorthEast),
            (true, _, _, true) => Some(ResizeDirection::SouthWest),
            (_, true, _, true) => Some(ResizeDirection::SouthEast),
            (true, _, _, _) => Some(ResizeDirection::West),
            (_, true, _, _) => Some(ResizeDirection::East),
            (_, _, true, _) => Some(ResizeDirection::North),
            (_, _, _, true) => Some(ResizeDirection::South),
            _ => None,
        }
    }

    /// Detect if the mouse is on a resize edge of the primary borderless window.
    /// Returns the resize direction if within the border zone, or None.
    pub(super) fn detect_resize_edge(
        &self,
        x: f32,
        y: f32,
    ) -> Option<winit::window::ResizeDirection> {
        Self::detect_resize_edge_for_chrome(
            &self.chrome,
            self.width as f32 / self.scale_factor as f32,
            self.height as f32 / self.scale_factor as f32,
            x,
            y,
        )
    }

    /// Title bar button width in logical pixels.
    pub(super) const TITLEBAR_BUTTON_WIDTH: f32 = 46.0;

    /// Check if a point is in the custom title bar area.
    /// Returns: 0 = not in title bar, 1 = drag area, 2 = close, 3 = maximize, 4 = minimize
    pub(super) fn titlebar_hit_test_for_chrome(
        chrome: &WindowChrome,
        logical_width: f32,
        x: f32,
        y: f32,
    ) -> u32 {
        if chrome.decorations_enabled || chrome.is_fullscreen || chrome.titlebar_height <= 0.0 {
            return 0;
        }
        let w = logical_width;
        let tb_h = chrome.titlebar_height;
        if y >= tb_h {
            return 0; // Below title bar
        }
        // Buttons are on the right: [minimize] [maximize] [close]
        let btn_w = Self::TITLEBAR_BUTTON_WIDTH;
        let close_x = w - btn_w;
        let max_x = w - btn_w * 2.0;
        let min_x = w - btn_w * 3.0;
        if x >= close_x {
            2 // Close
        } else if x >= max_x {
            3 // Maximize
        } else if x >= min_x {
            4 // Minimize
        } else {
            1 // Drag area
        }
    }

    /// Check if a point is in the primary custom title bar area.
    /// Returns: 0 = not in title bar, 1 = drag area, 2 = close, 3 = maximize, 4 = minimize
    pub(super) fn titlebar_hit_test(&self, x: f32, y: f32) -> u32 {
        Self::titlebar_hit_test_for_chrome(
            &self.chrome,
            self.width as f32 / self.scale_factor as f32,
            x,
            y,
        )
    }

    pub(super) fn frame_window_titlebar_hit_test(
        window_state: &GuiFrameWindowState,
        x: f32,
        y: f32,
    ) -> u32 {
        Self::titlebar_hit_test_for_chrome(
            &window_state.native.chrome,
            window_state.native.width as f32 / window_state.native.scale_factor as f32,
            x,
            y,
        )
    }
}

#[cfg(test)]
#[path = "input_test.rs"]
mod tests;

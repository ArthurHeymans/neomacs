//! GUI frame window management for the render thread.
//!
//! GNU Emacs treats every top-level GUI frame as a native window. This module
//! owns the render-thread mapping between Emacs frame IDs and winit windows so
//! redraw, input, resize, focus, and destruction can be frame-addressed instead
//! of primary-window-addressed.

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Fullscreen, Window, WindowId};

use super::child_frames::ChildFrameManager;
use super::cursor::{CursorState, CursorTarget};
use super::state::{
    FpsCounter, GuiChromeInteractionState, IdleDimState, ImeCursorArea, TypingSpeedState,
    WindowChrome, effective_window_scale_factor, window_size_from_emacs_pixels,
};
use super::transitions::{TransitionState, clear_frame_transition_textures};
use super::x11_hints::apply_window_geometry_hints;
use crate::core::frame_glyphs::{FrameGlyph, FrameGlyphBuffer};
use neomacs_display_protocol::TransitionPolicy;
use neomacs_display_protocol::effect_config::IdleDimConfig;
use neomacs_display_protocol::glyph_matrix::{
    GuiCompactBarState, GuiMenuBarState, GuiToolBarState,
};
#[cfg(feature = "wpe-webkit")]
use neomacs_display_protocol::scene::FloatingWebKit;
use neomacs_renderer_wgpu::{
    PopupMenuState, RendererFrameEffects, TooltipState, WgpuGlyphAtlas, WgpuRenderer,
};
use neovm_core::window::GuiFrameGeometryHints;

/// Native window/surface state for a top-level GUI frame.
pub(crate) struct GuiFrameNativeWindowState {
    pub window: Arc<Window>,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    /// Whether this window's native pointer cursor is hidden during typing.
    pub mouse_hidden_for_typing: bool,
    /// Whether native IME composition is active in this frame window.
    pub ime_enabled: bool,
    /// Last native IME cursor rectangle sent to this frame window.
    pub(super) last_ime_cursor_area: Option<ImeCursorArea>,
    /// Borderless native-window chrome state for this frame window.
    pub(super) chrome: WindowChrome,
}

/// Frame-owned render, input, overlay, and transient visual state.
pub(crate) struct GuiFrameRenderState {
    /// The Emacs frame_id that owns this window (used for routing).
    pub emacs_frame_id: u64,
    /// Chromeless glyph composition and rendering state.
    pub compositor: FrameCompositor,
    /// GUI chrome (menu bar, tool bar, compact bar) for this frame window.
    pub chrome: ChromeState,
    /// Transient overlays (popup, tooltip, bell, fps, typing, idle, ime).
    pub overlays: OverlayState,
    /// Text cursor animation and blink state for this frame window.
    pub(super) cursor: CursorState,
    /// Last known pointer position in this frame's logical coordinates.
    pub mouse_pos: (f32, f32),
    /// Floating WebKit overlays rendered on this frame window.
    #[cfg(feature = "wpe-webkit")]
    pub floating_webkits: Vec<FloatingWebKit>,
}

/// GUI chrome state for a frame window.
#[derive(Default)]
pub(crate) struct ChromeState {
    pub menu_bar: Option<GuiMenuBarState>,
    pub tool_bar: Option<GuiToolBarState>,
    pub compact_bar: Option<GuiCompactBarState>,
    pub interaction: GuiChromeInteractionState,
}

/// Transient overlay state for a frame window.
pub(crate) struct OverlayState {
    pub popup_menu: Option<PopupMenuState>,
    pub tooltip: Option<TooltipState>,
    pub visual_bell_start: Option<Instant>,
    pub(super) fps: FpsCounter,
    pub(super) typing_speed: TypingSpeedState,
    pub(super) idle_dim: IdleDimState,
    pub ime_preedit_active: bool,
    pub ime_preedit_text: String,
}

/// Glyph composition and rendering state for a frame window.
pub(crate) struct FrameCompositor {
    pub current_frame: Option<FrameGlyphBuffer>,
    pub child_frames: ChildFrameManager,
    pub glyph_atlas: Option<WgpuGlyphAtlas>,
    pub dirty: bool,
    pub(super) visual_cursors: HashMap<i64, CursorState>,
    pub renderer_effects: RendererFrameEffects,
    pub transitions: TransitionState,
}

impl OverlayState {
    pub fn popup_is_open(&self) -> bool {
        self.popup_menu.is_some()
    }

    pub fn hide_popup(&mut self) -> bool {
        if self.popup_menu.is_some() {
            self.popup_menu = None;
            true
        } else {
            false
        }
    }

    pub fn hide_tooltip(&mut self) -> bool {
        if self.tooltip.is_some() {
            self.tooltip = None;
            true
        } else {
            false
        }
    }

    pub fn has_active_overlay(&self) -> bool {
        self.popup_menu.is_some() || self.tooltip.is_some()
    }
}

impl ChromeState {
    pub fn is_interacting(&self) -> bool {
        self.interaction.menu_bar_active.is_some()
            || self.interaction.compact_bar_menu_active.is_some()
            || self.interaction.compact_bar_tool_pressed.is_some()
            || self.interaction.toolbar_pressed.is_some()
            || self.interaction.tab_bar_pressed.is_some()
    }

    pub fn clear_interaction(&mut self) {
        self.interaction.menu_bar_active = None;
        self.interaction.compact_bar_menu_active = None;
        self.interaction.compact_bar_tool_pressed = None;
        self.interaction.toolbar_pressed = None;
        self.interaction.toolbar_press_captured = false;
        self.interaction.tab_bar_pressed = None;
        self.interaction.tab_bar_press_captured = false;
    }

    pub fn dismiss_menus(&mut self) {
        self.interaction.menu_bar_active = None;
        self.interaction.compact_bar_menu_active = None;
    }

    /// Release all pressed state and return what was active.
    /// Returns the pressed item if any, clearing interaction state.
    pub fn release_pressed(&mut self) -> Option<ChromePress> {
        if let Some(idx) = self.interaction.menu_bar_active.take() {
            return Some(ChromePress::MenuBar(idx));
        }
        if let Some(idx) = self.interaction.compact_bar_menu_active.take() {
            return Some(ChromePress::CompactMenu(idx));
        }
        if let Some(idx) = self.interaction.compact_bar_tool_pressed.take() {
            return Some(ChromePress::CompactTool(idx));
        }
        if let Some(idx) = self.interaction.toolbar_pressed.take() {
            self.interaction.toolbar_press_captured = false;
            return Some(ChromePress::ToolBar(idx));
        }
        if let Some(idx) = self.interaction.tab_bar_pressed.take() {
            self.interaction.tab_bar_press_captured = false;
            return Some(ChromePress::TabBar(idx));
        }
        None
    }

    /// Apply a mouse press on a chrome hit during a popup interaction.
    /// Dismisses popup-related state and records the interaction.
    pub fn press_with_popup(&mut self, press: &ChromePress) {
        self.interaction.menu_bar_active = None;
        self.interaction.compact_bar_menu_active = None;
        match press {
            ChromePress::MenuBar(idx) => self.interaction.menu_bar_active = Some(*idx),
            ChromePress::CompactMenu(idx) => self.interaction.compact_bar_menu_active = Some(*idx),
            ChromePress::CompactTool(idx) => self.interaction.compact_bar_tool_pressed = Some(*idx),
            ChromePress::ToolBar(idx) => self.interaction.toolbar_pressed = Some(*idx),
            ChromePress::TabBar(idx) => {
                self.interaction.tab_bar_press_captured = true;
                self.interaction.tab_bar_pressed = Some(*idx);
            }
        }
    }

    /// Update hover state for all chrome bars. Returns true if dirty.
    pub fn update_hover(
        &mut self,
        menu_bar_hover: Option<u32>,
        tab_bar_hover: Option<u32>,
        toolbar_hover: Option<u32>,
    ) -> bool {
        let mut dirty = false;
        if self.interaction.menu_bar_hovered != menu_bar_hover {
            self.interaction.menu_bar_hovered = menu_bar_hover;
            dirty = true;
        }
        if self.interaction.tab_bar_hovered != tab_bar_hover {
            self.interaction.tab_bar_hovered = tab_bar_hover;
            dirty = true;
        }
        if self.interaction.toolbar_hovered != toolbar_hover {
            self.interaction.toolbar_hovered = toolbar_hover;
            dirty = true;
        }
        dirty
    }
}

/// Result of a chrome interaction press.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ChromePress {
    MenuBar(u32),
    CompactMenu(u32),
    CompactTool(u32),
    ToolBar(u32),
    TabBar(u32),
}

/// Per-window state for a top-level GUI frame.
///
/// The frame window lifecycle is modeled as an explicit state machine.
/// Before `resumed`, operations queue into [`FrameLifecycle::Pending`].
/// After the winit window is created, they apply directly through
/// [`FrameLifecycle::Active`].
pub(crate) struct GuiFrameWindowState {
    pub(super) lifecycle: FrameLifecycle,
    pub render: GuiFrameRenderState,
}

/// Window lifecycle state machine.
///
/// Mirrors GNU Emacs's frame lifecycle: created, mapped (active),
/// unmapped / destroyed.  Operations are deferred until the native
/// window exists, eliminating ad-hoc `if native.is_some()` checks
/// scattered across ~30 methods.
pub(super) enum FrameLifecycle {
    /// Window before `resumed` — all operations queue here.
    Pending {
        width: u32,
        height: u32,
        scale_factor: f64,
        mouse_hidden_for_typing: bool,
        ime_enabled: bool,
        last_ime_cursor_area: Option<ImeCursorArea>,
        chrome: WindowChrome,
        geometry_hints: Option<GuiFrameGeometryHints>,
    },
    /// Window with a live winit native window and wgpu surface.
    Active {
        native: GuiFrameNativeWindowState,
        mouse_hidden_for_typing: bool,
        ime_enabled: bool,
        last_ime_cursor_area: Option<ImeCursorArea>,
    },
}

impl GuiFrameRenderState {
    pub(super) fn new(
        emacs_frame_id: u64,
        device: &wgpu::Device,
        scale_factor: f64,
        fps_enabled: bool,
    ) -> Self {
        Self {
            emacs_frame_id,
            compositor: FrameCompositor {
                current_frame: None,
                child_frames: ChildFrameManager::new(),
                glyph_atlas: Some(WgpuGlyphAtlas::new_with_scale(device, scale_factor as f32)),
                dirty: false,
                visual_cursors: HashMap::new(),
                renderer_effects: RendererFrameEffects::default(),
                transitions: TransitionState::default(),
            },
            chrome: ChromeState::default(),
            overlays: OverlayState {
                popup_menu: None,
                tooltip: None,
                visual_bell_start: None,
                fps: FpsCounter {
                    enabled: fps_enabled,
                    ..FpsCounter::default()
                },
                typing_speed: TypingSpeedState::default(),
                idle_dim: IdleDimState::default(),
                ime_preedit_active: false,
                ime_preedit_text: String::new(),
            },
            cursor: CursorState::default(),
            mouse_pos: (0.0, 0.0),
            #[cfg(feature = "wpe-webkit")]
            floating_webkits: Vec::new(),
        }
    }

    pub(super) fn new_without_device(emacs_frame_id: u64, fps_enabled: bool) -> Self {
        Self {
            emacs_frame_id,
            compositor: FrameCompositor {
                current_frame: None,
                child_frames: ChildFrameManager::new(),
                glyph_atlas: None,
                dirty: false,
                visual_cursors: HashMap::new(),
                renderer_effects: RendererFrameEffects::default(),
                transitions: TransitionState::default(),
            },
            chrome: ChromeState::default(),
            overlays: OverlayState {
                popup_menu: None,
                tooltip: None,
                visual_bell_start: None,
                fps: FpsCounter {
                    enabled: fps_enabled,
                    ..FpsCounter::default()
                },
                typing_speed: TypingSpeedState::default(),
                idle_dim: IdleDimState::default(),
                ime_preedit_active: false,
                ime_preedit_text: String::new(),
            },
            cursor: CursorState::default(),
            mouse_pos: (0.0, 0.0),
            #[cfg(feature = "wpe-webkit")]
            floating_webkits: Vec::new(),
        }
    }

    pub(super) fn populate_glyph_atlas(&mut self, device: &wgpu::Device, scale_factor: f64) {
        if self.compositor.glyph_atlas.is_none() {
            self.compositor.glyph_atlas = Some(WgpuGlyphAtlas::new_with_scale(device, scale_factor as f32));
        }
    }

    pub(super) fn current_frame_clone(&self) -> Option<FrameGlyphBuffer> {
        self.compositor.current_frame.clone()
    }

    pub(super) fn font_metrics(&self) -> (f32, f32, f32) {
        self.compositor.glyph_atlas.as_ref().map_or((13.0, 17.0, 13.0 * 0.6), |atlas| {
            (atlas.default_font_size(), atlas.default_line_height(), atlas.default_char_width())
        })
    }

    pub(super) fn set_popup_menu(&mut self, popup_menu: Option<PopupMenuState>) {
        self.overlays.popup_menu = popup_menu;
        self.compositor.dirty = true;
    }

    pub(super) fn set_tooltip(&mut self, tooltip: Option<TooltipState>) {
        self.overlays.tooltip = tooltip;
        self.compositor.dirty = true;
    }

    pub(super) fn set_menu_bar(&mut self, menu_bar: Option<GuiMenuBarState>) {
        self.chrome.menu_bar = menu_bar;
        self.compositor.dirty = true;
    }

    pub(super) fn set_tool_bar(&mut self, tool_bar: Option<GuiToolBarState>) {
        self.chrome.tool_bar = tool_bar;
        self.compositor.dirty = true;
    }

    pub(super) fn set_compact_bar(&mut self, compact_bar: Option<GuiCompactBarState>) {
        self.chrome.compact_bar = compact_bar;
        self.compositor.dirty = true;
    }

    pub(super) fn extend_current_frame_glyphs(&mut self, glyphs: Vec<FrameGlyph>) -> bool {
        if glyphs.is_empty() {
            return false;
        }
        let Some(frame) = self.compositor.current_frame.as_mut() else {
            return false;
        };
        frame.glyphs.extend(glyphs);
        self.compositor.dirty = true;
        true
    }

    pub(super) fn set_visual_bell_start(&mut self, start: Option<Instant>) {
        self.overlays.visual_bell_start = start;
        if start.is_some() {
            self.compositor.dirty = true;
        }
    }

    pub(super) fn set_ime_preedit(&mut self, text: String) {
        self.overlays.ime_preedit_active = !text.is_empty();
        self.overlays.ime_preedit_text = text;
        self.compositor.dirty = true;
    }

    pub(super) fn clear_ime_preedit(&mut self) {
        self.overlays.ime_preedit_active = false;
        self.overlays.ime_preedit_text.clear();
        self.compositor.dirty = true;
    }

    pub(super) fn set_fps_enabled(&mut self, enabled: bool) {
        self.overlays.fps.enabled = enabled;
        self.compositor.dirty = true;
    }

    #[cfg(feature = "wpe-webkit")]
    pub(super) fn push_floating_webkit(&mut self, overlay: FloatingWebKit) {
        self.floating_webkits.push(overlay);
        self.compositor.dirty = true;
    }

    #[cfg(feature = "wpe-webkit")]
    pub(super) fn remove_floating_webkit(&mut self, id: u32) -> bool {
        let old_len = self.floating_webkits.len();
        self.floating_webkits.retain(|w| w.webkit_id != id);
        let removed = self.floating_webkits.len() != old_len;
        if removed {
            self.compositor.dirty = true;
        }
        removed
    }

    pub(super) fn record_typing_keypress(&mut self, now: Instant) {
        self.overlays.typing_speed.key_press_times.push(now);
        self.compositor.dirty = true;
    }

    pub(super) fn record_idle_activity(&mut self, now: Instant) {
        self.overlays.idle_dim.last_activity_time = now;
        self.compositor.dirty = true;
    }

    pub(super) fn dismiss_all_chrome_menus(&mut self) {
        self.overlays.popup_menu = None;
        self.chrome.interaction.menu_bar_active = None;
        self.chrome.interaction.compact_bar_menu_active = None;
        self.mark_dirty();
    }

    pub(super) fn mark_dirty(&mut self) {
        self.compositor.dirty = true;
    }

    pub(super) fn set_dirty(&mut self, dirty: bool) {
        self.compositor.dirty = dirty;
    }

    pub(super) fn clear_all_chrome_pressed(&mut self) {
        self.chrome.interaction.tab_bar_pressed = None;
        self.chrome.interaction.tab_bar_press_captured = false;
        self.chrome.interaction.compact_bar_tool_pressed = None;
        self.chrome.interaction.toolbar_pressed = None;
        self.chrome.interaction.toolbar_press_captured = false;
        self.mark_dirty();
    }

    pub(super) fn set_emacs_frame_id(&mut self, frame_id: u64) {
        self.emacs_frame_id = frame_id;
    }

    pub(super) fn set_mouse_pos(&mut self, pos: (f32, f32)) {
        self.mouse_pos = pos;
    }

    pub(super) fn set_current_frame(
        &mut self,
        frame: Option<crate::core::frame_glyphs::FrameGlyphBuffer>,
    ) {
        self.compositor.current_frame = frame;
    }

    pub(super) fn with_chrome_interaction_mut(
        &mut self,
        f: impl FnOnce(&mut GuiChromeInteractionState),
    ) -> bool {
        let previous = self.chrome.interaction;
        f(&mut self.chrome.interaction);
        let changed = self.chrome.interaction != previous;
        if changed {
            self.compositor.dirty = true;
        }
        changed
    }

    pub(super) fn update_popup_hover(&mut self, x: f32, y: f32) -> bool {
        let Some(menu) = self.overlays.popup_menu.as_mut() else {
            return false;
        };
        let (hit_depth, hit_local) = menu.hit_test_all(x, y);
        let mut dirty = false;
        if hit_depth >= 0 {
            let target_depth = hit_depth as usize;
            while menu.submenu_panels.len() > target_depth {
                menu.submenu_panels.pop();
                dirty = true;
            }
            let panel = if target_depth == 0 {
                &mut menu.root_panel
            } else {
                &mut menu.submenu_panels[target_depth - 1]
            };
            if hit_local != panel.hover_index {
                panel.hover_index = hit_local;
                dirty = true;
                if hit_local >= 0 && (hit_local as usize) < panel.item_indices.len() {
                    let global_idx = panel.item_indices[hit_local as usize];
                    if menu.all_items[global_idx].submenu {
                        menu.open_submenu();
                    }
                }
            }
        }
        if dirty {
            self.compositor.dirty = true;
        }
        dirty
    }

    pub(super) fn trigger_visual_bell(
        &mut self,
        cursor_error_pulse_enabled: bool,
        edge_snap_enabled: bool,
        edge_snap_duration_ms: u32,
        now: Instant,
    ) {
        self.overlays.visual_bell_start = Some(now);
        if cursor_error_pulse_enabled {
            self.compositor.renderer_effects.trigger_cursor_error_pulse(now);
        }
        if edge_snap_enabled {
            let selected_info = self.compositor.current_frame.as_ref().and_then(|frame| {
                frame
                    .window_infos
                    .iter()
                    .find(|info| info.selected && !info.is_minibuffer)
                    .cloned()
            });
            if let Some(info) = selected_info {
                let at_top = info.window_start <= 1;
                let at_bottom = info.window_end >= info.buffer_size;
                if at_top || at_bottom {
                    self.compositor.renderer_effects.trigger_edge_snap(
                        info.bounds,
                        info.mode_line_height,
                        at_top,
                        at_bottom,
                        now,
                        edge_snap_duration_ms,
                    );
                }
            }
        }
        self.compositor.dirty = true;
    }

    pub(super) fn take_current_frame_for_render(&mut self) -> Option<FrameGlyphBuffer> {
        self.compositor.current_frame.as_mut().map(Self::take_frame_for_render)
    }

    pub(super) fn take_frame_for_render(current_frame: &mut FrameGlyphBuffer) -> FrameGlyphBuffer {
        let (transition_hints, effect_hints) = current_frame.take_runtime_hints();
        let mut frame = current_frame.clone();
        frame.transition_hints = transition_hints;
        frame.effect_hints = effect_hints;
        frame
    }

    pub(super) fn tick_cursor_animation(&mut self) -> bool {
        let mut dirty = self.cursor.tick_animation();
        for cursor in self.compositor.visual_cursors.values_mut() {
            dirty |= cursor.tick_animation();
        }
        if dirty {
            self.compositor.dirty = true;
        }
        dirty
    }

    pub(super) fn tick_cursor_blink(
        &mut self,
        now: Instant,
        cursor_wake_enabled: bool,
        renderer: Option<&WgpuRenderer>,
    ) -> bool {
        if !self.cursor.blink_enabled || self.cursor.target_cloned().is_none() {
            return false;
        }
        if now.duration_since(self.cursor.last_blink_toggle) < self.cursor.blink_interval {
            return false;
        }
        let was_off = !self.cursor.blink_on;
        self.cursor.blink_on = !self.cursor.blink_on;
        self.cursor.last_blink_toggle = now;
        if was_off
            && self.cursor.blink_on
            && cursor_wake_enabled
            && let Some(renderer) = renderer
        {
            renderer.trigger_transient_cursor_wake(&mut self.compositor.renderer_effects, now);
        }
        self.compositor.dirty = true;
        true
    }

    pub(super) fn force_cursor_blink_on(&mut self) -> bool {
        if !self.cursor.force_blink_on() {
            return false;
        }
        self.compositor.dirty = true;
        true
    }

    pub(super) fn mark_active_visuals_dirty(&mut self) -> bool {
        if !self.compositor.renderer_effects.needs_redraw() && !self.compositor.transitions.has_active() {
            return false;
        }
        self.compositor.dirty = true;
        true
    }

    pub(super) fn trigger_click_halo(&mut self, x: f32, y: f32, now: Instant, duration_ms: u32) {
        self.compositor.renderer_effects
            .trigger_click_halo(x, y, now, duration_ms);
        self.compositor.dirty = true;
    }

    pub(super) fn tick_cursor_size_animation(&mut self) -> bool {
        let mut dirty = self.cursor.tick_size_animation();
        for cursor in self.compositor.visual_cursors.values_mut() {
            dirty |= cursor.tick_size_animation();
        }
        if dirty {
            self.compositor.dirty = true;
        }
        dirty
    }

    pub(super) fn tick_idle_dim(&mut self, config: &IdleDimConfig) -> bool {
        let idle_time = self.overlays.idle_dim.last_activity_time.elapsed();
        let target_alpha = if idle_time >= config.delay {
            config.opacity
        } else {
            0.0
        };
        let diff = target_alpha - self.overlays.idle_dim.current_alpha;
        if diff.abs() > 0.001 {
            let fade_speed = if config.fade_duration.as_secs_f32() > 0.0 {
                1.0 / config.fade_duration.as_secs_f32() * 0.016
            } else {
                1.0
            };
            if diff > 0.0 {
                self.overlays.idle_dim.current_alpha =
                    (self.overlays.idle_dim.current_alpha + fade_speed * config.opacity).min(target_alpha);
            } else {
                self.overlays.idle_dim.current_alpha =
                    (self.overlays.idle_dim.current_alpha - fade_speed * config.opacity).max(0.0);
            }
            self.overlays.idle_dim.active = true;
            self.compositor.dirty = true;
            true
        } else if self.overlays.idle_dim.current_alpha > 0.001 {
            self.overlays.idle_dim.active = true;
            false
        } else {
            self.overlays.idle_dim.active = false;
            false
        }
    }

    pub(super) fn clear_idle_dim(&mut self) {
        self.overlays.idle_dim.active = false;
        self.overlays.idle_dim.current_alpha = 0.0;
    }

    pub(super) fn sync_visual_cursors_from_current_frame(
        &mut self,
        cursor_config: impl Fn(&mut CursorState),
    ) {
        let Some(current_frame) = self.compositor.current_frame.as_ref() else {
            self.compositor.visual_cursors.clear();
            return;
        };
        let mut live_visual_cursor_ids = HashSet::new();
        for cursor in &current_frame.window_cursors {
            if cursor.window_id >= 0 {
                continue;
            }
            live_visual_cursor_ids.insert(cursor.window_id);
            let state = self.compositor.visual_cursors.entry(cursor.window_id).or_default();
            cursor_config(state);
            let (_, target_moved) = state.set_target(CursorTarget {
                window_id: cursor.window_id,
                x: cursor.x,
                y: cursor.y,
                width: cursor.width,
                height: cursor.height,
                style: cursor.style,
                color: cursor.color,
                frame_id: self.emacs_frame_id,
            });
            if target_moved {
                self.compositor.dirty = true;
            }
        }
        self.compositor.visual_cursors
            .retain(|id, _| live_visual_cursor_ids.contains(id));
    }

    pub(super) fn sync_cursor_config(&mut self, defaults: &CursorState, dirty: bool) {
        self.cursor.copy_config_from(defaults);
        for cursor in self.compositor.visual_cursors.values_mut() {
            cursor.copy_config_from(defaults);
        }
        if dirty {
            self.compositor.dirty = true;
        }
    }

    pub(super) fn apply_visual_cursor_animations(&mut self) {
        if self.compositor.visual_cursors.is_empty() {
            return;
        }
        let visual_cursor_rects: HashMap<i64, (f32, f32, f32, f32)> = self
            .compositor.visual_cursors
            .iter()
            .map(|(id, state)| {
                (
                    *id,
                    (
                        state.current_x,
                        state.current_y,
                        state.current_w,
                        state.current_h,
                    ),
                )
            })
            .collect();
        let Some(frame) = self.compositor.current_frame.as_mut() else {
            return;
        };
        for cursor in &mut frame.window_cursors {
            let Some((x, y, width, height)) = visual_cursor_rects.get(&cursor.window_id) else {
                continue;
            };
            cursor.x = *x;
            cursor.y = *y;
            cursor.width = *width;
            cursor.height = *height;
        }
    }

    pub(super) fn remove_child_frame(&mut self, frame_id: u64) -> bool {
        let removed = self.compositor.child_frames.remove_frame(frame_id);
        if removed {
            self.compositor.dirty = true;
        }
        if self
            .cursor
            .target_cloned()
            .is_some_and(|target| target.frame_id == frame_id)
        {
            self.cursor.clear_target();
            self.overlays.ime_preedit_active = false;
            self.overlays.ime_preedit_text.clear();
            self.compositor.dirty = true;
            return true;
        }
        removed
    }

    pub(super) fn update_child_frame(&mut self, frame: FrameGlyphBuffer) {
        self.compositor.child_frames.update_frame(frame);
        self.compositor.dirty = true;
    }
}

impl FrameLifecycle {
    pub fn native(&self) -> Option<&GuiFrameNativeWindowState> {
        match self {
            Self::Active { native, .. } => Some(native),
            _ => None,
        }
    }

    pub fn native_mut(&mut self) -> Option<&mut GuiFrameNativeWindowState> {
        match self {
            Self::Active { native, .. } => Some(native),
            _ => None,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active { .. })
    }

    pub fn window(&self) -> Option<&Arc<Window>> {
        self.native().map(|n| &n.window)
    }

    pub fn native_size(&self) -> (u32, u32) {
        match self {
            Self::Active { native, .. } => (native.width, native.height),
            Self::Pending { width, height, .. } => (*width, *height),
        }
    }

    pub fn scale_factor(&self) -> f64 {
        match self {
            Self::Active { native, .. } => native.scale_factor,
            Self::Pending { scale_factor, .. } => *scale_factor,
        }
    }

    pub fn native_scale_factor(&self) -> f64 {
        self.native().map_or(1.0, |n| n.scale_factor)
    }

    pub fn chrome(&self) -> &WindowChrome {
        match self {
            Self::Active { native, .. } => &native.chrome,
            Self::Pending { chrome, .. } => chrome,
        }
    }

    pub fn chrome_mut(&mut self) -> &mut WindowChrome {
        match self {
            Self::Active { native, .. } => &mut native.chrome,
            Self::Pending { chrome, .. } => chrome,
        }
    }

    pub fn ime_enabled(&self) -> bool {
        match self {
            Self::Active { ime_enabled, .. } => *ime_enabled,
            Self::Pending { ime_enabled, .. } => *ime_enabled,
        }
    }

    pub fn set_ime_enabled(&mut self, enabled: bool) {
        match self {
            Self::Active { ime_enabled: ie, .. } => *ie = enabled,
            Self::Pending { ime_enabled: ie, .. } => *ie = enabled,
        }
    }

    pub fn mouse_hidden_for_typing(&self) -> bool {
        match self {
            Self::Active { mouse_hidden_for_typing: m, .. } => *m,
            Self::Pending { mouse_hidden_for_typing: m, .. } => *m,
        }
    }

    pub fn set_mouse_hidden_for_typing(&mut self, hidden: bool) {
        match self {
            Self::Active { mouse_hidden_for_typing: m, .. } => *m = hidden,
            Self::Pending { mouse_hidden_for_typing: m, .. } => *m = hidden,
        }
    }

    pub fn last_ime_cursor_area(&self) -> Option<ImeCursorArea> {
        match self {
            Self::Active { last_ime_cursor_area, .. } => *last_ime_cursor_area,
            Self::Pending { last_ime_cursor_area, .. } => *last_ime_cursor_area,
        }
    }

    pub fn set_last_ime_cursor_area(&mut self, area: Option<ImeCursorArea>) {
        match self {
            Self::Active { last_ime_cursor_area: l, .. } => *l = area,
            Self::Pending { last_ime_cursor_area: l, .. } => *l = area,
        }
    }

    pub fn geometry_hints(&self) -> Option<GuiFrameGeometryHints> {
        match self {
            Self::Pending { geometry_hints, .. } => *geometry_hints,
            _ => None,
        }
    }

    pub fn request_redraw(&self) {
        if let Self::Active { native, .. } = self {
            native.window.request_redraw();
        }
    }
}

impl GuiFrameWindowState {
    pub fn handle_resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        match &mut self.lifecycle {
            FrameLifecycle::Active { native, .. } => {
                native.width = width;
                native.height = height;
                native.surface_config.width = width;
                native.surface_config.height = height;
                native.surface.configure(device, &native.surface_config);
                clear_frame_transition_textures(&mut self.render.compositor.transitions);
                self.render.compositor.dirty = true;
            }
            FrameLifecycle::Pending { width: pw, height: ph, .. } => {
                *pw = width;
                *ph = height;
            }
        }
    }

    pub fn set_scale_factor(&mut self, scale_factor: f64) {
        let effective_scale = effective_window_scale_factor(scale_factor);
        match &mut self.lifecycle {
            FrameLifecycle::Active { native, .. } => {
                native.scale_factor = effective_scale;
                if let Some(atlas) = self.render.compositor.glyph_atlas.as_mut() {
                    atlas.set_scale_factor(effective_scale as f32);
                }
                self.render.compositor.dirty = true;
            }
            FrameLifecycle::Pending { scale_factor: sf, .. } => {
                *sf = effective_scale;
            }
        }
    }

    pub(super) fn set_title(&mut self, title: String) {
        match &mut self.lifecycle {
            FrameLifecycle::Active { native, .. } => {
                native.chrome.title = title.clone();
                native.window.set_title(&title);
                if !native.chrome.decorations_enabled {
                    self.render.compositor.dirty = true;
                }
            }
            FrameLifecycle::Pending { chrome, .. } => {
                chrome.title = title;
            }
        }
    }

    pub(super) fn set_fullscreen_mode(&mut self, mode: u32) {
        let FrameLifecycle::Active { native, .. } = &mut self.lifecycle else {
            return;
        };
        match mode {
            3 => {
                native.window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                native.chrome.is_fullscreen = true;
            }
            4 => {
                native.window.set_maximized(true);
                native.chrome.is_fullscreen = false;
            }
            _ => {
                native.window.set_fullscreen(None);
                native.window.set_maximized(false);
                native.chrome.is_fullscreen = false;
            }
        }
        self.render.compositor.dirty = true;
    }

    pub(super) fn request_inner_size(&mut self, width: u32, height: u32) {
        match &mut self.lifecycle {
            FrameLifecycle::Active { native, .. } => {
                let size = window_size_from_emacs_pixels(width, height);
                let _ = native.window.request_inner_size(size);
            }
            FrameLifecycle::Pending { width: pw, height: ph, .. } => {
                *pw = width;
                *ph = height;
            }
        }
    }

    pub(super) fn apply_geometry_hints(&mut self, geometry_hints: GuiFrameGeometryHints) {
        match &mut self.lifecycle {
            FrameLifecycle::Active { native, .. } => {
                apply_window_geometry_hints(&native.window, geometry_hints);
            }
            FrameLifecycle::Pending { geometry_hints: gh, .. } => {
                *gh = Some(geometry_hints);
            }
        }
    }

    pub(super) fn set_decorations(&mut self, decorated: bool) {
        match &mut self.lifecycle {
            FrameLifecycle::Active { native, .. } => {
                native.chrome.decorations_enabled = decorated;
                native.window.set_decorations(decorated);
                self.render.compositor.dirty = true;
            }
            FrameLifecycle::Pending { chrome, .. } => {
                chrome.decorations_enabled = decorated;
            }
        }
    }

    pub(super) fn set_mouse_hidden_for_typing(&mut self, hidden: bool) {
        if let FrameLifecycle::Active { native, mouse_hidden_for_typing, .. } = &mut self.lifecycle {
            if *mouse_hidden_for_typing != hidden {
                native.window.set_cursor_visible(!hidden);
            }
        }
        self.lifecycle.set_mouse_hidden_for_typing(hidden);
    }

    pub(super) fn reset_ime_cursor_area(&mut self) {
        match &mut self.lifecycle {
            FrameLifecycle::Active { native, last_ime_cursor_area, .. } => {
                *last_ime_cursor_area = None;
                native.window.set_ime_cursor_area(
                    PhysicalPosition::new(0.0, 0.0),
                    PhysicalSize::new(1.0, 1.0),
                );
            }
            FrameLifecycle::Pending { last_ime_cursor_area, .. } => {
                *last_ime_cursor_area = None;
            }
        }
    }

    pub(super) fn update_ime_cursor_area(&mut self, area: ImeCursorArea) {
        match &mut self.lifecycle {
            FrameLifecycle::Active { native, last_ime_cursor_area, .. } => {
                if *last_ime_cursor_area == Some(area) {
                    return;
                }
                native.window.set_ime_cursor_area(
                    PhysicalPosition::new(area.x as f64, area.y as f64),
                    PhysicalSize::new(area.width as f64, area.height as f64),
                );
                *last_ime_cursor_area = Some(area);
            }
            FrameLifecycle::Pending { last_ime_cursor_area, .. } => {
                *last_ime_cursor_area = Some(area);
            }
        }
    }

    pub(super) fn clear_ime_preedit(&mut self) {
        self.render.overlays.ime_preedit_active = false;
        self.render.overlays.ime_preedit_text.clear();
        self.reset_ime_cursor_area();
        self.render.compositor.dirty = true;
    }

    pub(super) fn remove_child_frame(&mut self, frame_id: u64) -> bool {
        let target_was_child = self
            .render
            .cursor
            .target_cloned()
            .is_some_and(|target| target.frame_id == frame_id);
        let changed = self.render.remove_child_frame(frame_id);
        if target_was_child {
            self.reset_ime_cursor_area();
        }
        changed
    }

    pub(super) fn drag_resize_for_current_edge(&self) -> bool {
        let FrameLifecycle::Active { native, .. } = &self.lifecycle else {
            return false;
        };
        let Some(dir) = native.chrome.resize_edge else {
            return false;
        };
        let _ = native.window.drag_resize_window(dir);
        true
    }

    pub(super) fn handle_titlebar_action(&mut self, action: u32) -> bool {
        let FrameLifecycle::Active { native, .. } = &mut self.lifecycle else {
            return false;
        };
        match action {
            1 => {
                let now = Instant::now();
                if now
                    .duration_since(native.chrome.last_titlebar_click)
                    .as_millis()
                    < 400
                {
                    native.window.set_maximized(!native.window.is_maximized());
                } else {
                    let _ = native.window.drag_window();
                }
                native.chrome.last_titlebar_click = now;
                true
            }
            3 => {
                native.window.set_maximized(!native.window.is_maximized());
                true
            }
            4 => {
                native.window.set_minimized(true);
                true
            }
            _ => false,
        }
    }

    pub(super) fn drag_window(&self) {
        if let Some(native) = self.lifecycle.native() {
            let _ = native.window.drag_window();
        }
    }

    pub fn native_size(&self) -> (u32, u32) {
        self.lifecycle.native_size()
    }

    pub fn scale_factor(&self) -> f64 {
        self.lifecycle.scale_factor()
    }

    pub(super) fn chrome(&self) -> &WindowChrome {
        self.lifecycle.chrome()
    }

    pub(super) fn chrome_mut(&mut self) -> &mut WindowChrome {
        self.lifecycle.chrome_mut()
    }

    pub fn ime_enabled(&self) -> bool {
        self.lifecycle.ime_enabled()
    }

    pub fn set_ime_enabled(&mut self, enabled: bool) {
        self.lifecycle.set_ime_enabled(enabled);
    }

    pub fn mouse_hidden_for_typing(&self) -> bool {
        self.lifecycle.mouse_hidden_for_typing()
    }

    pub fn set_mouse_hidden_for_typing_pending(&mut self, hidden: bool) {
        self.lifecycle.set_mouse_hidden_for_typing(hidden);
    }

    pub fn request_redraw(&self) {
        self.lifecycle.request_redraw();
    }

    pub fn window(&self) -> Option<&Arc<Window>> {
        self.lifecycle.window()
    }

    pub(super) fn last_ime_cursor_area(&self) -> Option<ImeCursorArea> {
        self.lifecycle.last_ime_cursor_area()
    }

    pub(super) fn set_last_ime_cursor_area(&mut self, area: Option<ImeCursorArea>) {
        self.lifecycle.set_last_ime_cursor_area(area);
    }

    pub fn geometry_hints(&self) -> Option<GuiFrameGeometryHints> {
        self.lifecycle.geometry_hints()
    }

    pub fn set_geometry_hints(&mut self, hints: GuiFrameGeometryHints) {
        match &mut self.lifecycle {
            FrameLifecycle::Active { native, .. } => {
                apply_window_geometry_hints(&native.window, hints);
            }
            FrameLifecycle::Pending { geometry_hints, .. } => {
                *geometry_hints = Some(hints);
            }
        }
    }

    pub fn set_pending_size(&mut self, width: u32, height: u32) {
        match &mut self.lifecycle {
            FrameLifecycle::Pending { width: pw, height: ph, .. } => {
                *pw = width;
                *ph = height;
            }
            _ => {}
        }
    }
}

/// Key for frame-window lookup in the manager's `windows` HashMap.
///
/// Matches GNU Emacs convention: 0 is never a valid frame ID
/// (`frame_next_id = 1` in GNU Emacs `frame.c:343`).  The primary
/// frame starts under [`FrameKey::Pending`] and is re-keyed to
/// [`FrameKey::Adopted`] once `adopt_primary_frame_id` is called.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub(crate) enum FrameKey {
    /// Primary frame before Emacs assigns a real frame ID (bootstrap).
    Pending,
    /// Frame with a real Emacs-assigned frame ID.
    Adopted(u64),
}

impl FrameKey {
    pub(super) fn from_primary(emacs_id: Option<u64>) -> Self {
        match emacs_id {
            Some(id) => Self::Adopted(id),
            None => Self::Pending,
        }
    }
}

/// Manages top-level GUI frame windows in the render thread.
pub(crate) struct GuiFrameWindowManager {
    /// All top-level frame windows, keyed by [`FrameKey`].
    pub windows: HashMap<FrameKey, GuiFrameWindowState>,
    /// Winit WindowId → Emacs frame_id (reverse mapping for event dispatch)
    pub winit_to_emacs: HashMap<WindowId, u64>,
    /// Emacs frame_id adopted by the primary process window.
    pub primary_emacs_frame_id: Option<u64>,
    /// winit id of the primary process window.
    pub primary_winit_id: Option<WindowId>,
    /// Pending window creation requests (processed in resumed/about_to_wait)
    pub pending_creates: Vec<PendingWindow>,
    /// Pending window destruction requests
    pub pending_destroys: Vec<u64>,
    /// Native chrome defaults applied to future secondary frame windows.
    pub(super) chrome_defaults: WindowChrome,
    /// Whether future secondary frame windows should start with FPS enabled.
    pub(super) fps_enabled: bool,
}

/// A request to create a new OS window.
pub(crate) struct PendingWindow {
    pub emacs_frame_id: u64,
    pub width: u32,
    pub height: u32,
    pub title: String,
    pub geometry_hints: GuiFrameGeometryHints,
}

impl GuiFrameWindowManager {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            winit_to_emacs: HashMap::new(),
            primary_emacs_frame_id: None,
            primary_winit_id: None,
            pending_creates: Vec::new(),
            pending_destroys: Vec::new(),
            chrome_defaults: WindowChrome::default(),
            fps_enabled: false,
        }
    }

    pub(super) fn cursor_target_for_frame(
        emacs_frame_id: u64,
        frame: &FrameGlyphBuffer,
    ) -> Option<CursorTarget> {
        frame.phys_cursor.as_ref().map(|cursor| CursorTarget {
            window_id: cursor.window_id,
            x: cursor.x,
            y: cursor.y,
            width: cursor.width,
            height: cursor.height,
            style: cursor.style,
            color: cursor.color,
            frame_id: emacs_frame_id,
        })
    }

    pub fn adopt_primary_frame_id(&mut self, emacs_frame_id: u64) {
        let old_key = FrameKey::from_primary(self.primary_emacs_frame_id);
        self.primary_emacs_frame_id = Some(emacs_frame_id);
        if let Some(window_state) = self.windows.remove(&old_key) {
            self.windows.insert(FrameKey::Adopted(emacs_frame_id), window_state);
        }
        if let Some(ws) = self.windows.get_mut(&FrameKey::Adopted(emacs_frame_id)) {
            ws.render.set_emacs_frame_id(emacs_frame_id);
        }
        self.sync_primary_mapping();
    }

    pub fn adopt_primary_winit_id(&mut self, winit_id: WindowId) {
        self.primary_winit_id = Some(winit_id);
        self.sync_primary_mapping();
    }

    pub fn primary_frame_id(&self) -> Option<u64> {
        self.primary_emacs_frame_id
    }

    pub fn primary_event_frame_id(&self) -> u64 {
        self.primary_emacs_frame_id.unwrap_or(0)
    }

    pub fn is_primary_frame_id(&self, emacs_frame_id: u64) -> bool {
        emacs_frame_id == 0 || self.primary_emacs_frame_id == Some(emacs_frame_id)
    }

    pub fn has_secondary_window(&self, emacs_frame_id: u64) -> bool {
        self.windows.contains_key(&FrameKey::Adopted(emacs_frame_id))
    }

    fn primary_frame_key(&self) -> FrameKey {
        FrameKey::from_primary(self.primary_emacs_frame_id)
    }

    pub(super) fn primary_window(&self) -> Option<&GuiFrameWindowState> {
        self.windows.get(&self.primary_frame_key())
    }

    pub(super) fn primary_window_mut(&mut self) -> Option<&mut GuiFrameWindowState> {
        self.windows.get_mut(&self.primary_frame_key())
    }

    pub(super) fn set_primary_window(&mut self, window_state: GuiFrameWindowState) {
        if let Some(native) = window_state.lifecycle.native() {
            self.primary_winit_id = Some(native.window.id());
        }
        let key = self.primary_frame_key();
        self.windows.insert(key, window_state);
        self.sync_primary_mapping();
    }

    pub(super) fn set_primary_pending(&mut self, window_state: GuiFrameWindowState) {
        self.windows.insert(FrameKey::Pending, window_state);
        self.sync_primary_mapping();
    }

    pub(super) fn populate_primary_native(&mut self, native: GuiFrameNativeWindowState) {
        let key = self.primary_frame_key();
        if let Some(window_state) = self.windows.get_mut(&key) {
            let winit_id = native.window.id();
            self.primary_winit_id = Some(winit_id);
            window_state.lifecycle = FrameLifecycle::Active {
                native,
                mouse_hidden_for_typing: window_state.lifecycle.mouse_hidden_for_typing(),
                ime_enabled: window_state.lifecycle.ime_enabled(),
                last_ime_cursor_area: window_state.lifecycle.last_ime_cursor_area(),
            };
            self.sync_primary_mapping();
        }
    }

    pub(super) fn take_primary_window(&mut self) -> Option<GuiFrameWindowState> {
        let key = self.primary_frame_key();
        if let Some(winit_id) = self.primary_winit_id.take() {
            self.winit_to_emacs.remove(&winit_id);
        }
        self.windows.remove(&key)
    }

    pub fn is_primary_winit(&self, winit_id: WindowId) -> bool {
        self.primary_winit_id == Some(winit_id)
    }

    pub fn clear_primary_mapping(&mut self) {
        if let Some(winit_id) = self.primary_winit_id.take() {
            self.winit_to_emacs.remove(&winit_id);
        }
        self.primary_emacs_frame_id = None;
    }

    fn sync_primary_mapping(&mut self) {
        if let (Some(winit_id), Some(emacs_frame_id)) =
            (self.primary_winit_id, self.primary_emacs_frame_id)
        {
            self.winit_to_emacs.insert(winit_id, emacs_frame_id);
        }
    }

    /// Schedule a new window to be created on the next event loop iteration.
    pub fn request_create(
        &mut self,
        emacs_frame_id: u64,
        width: u32,
        height: u32,
        title: String,
        geometry_hints: GuiFrameGeometryHints,
    ) {
        self.pending_creates.push(PendingWindow {
            emacs_frame_id,
            width,
            height,
            title,
            geometry_hints,
        });
    }

    /// Schedule a window for destruction.
    pub fn request_destroy(&mut self, emacs_frame_id: u64) {
        self.pending_destroys.push(emacs_frame_id);
    }

    /// Process pending window creations. Must be called from the event loop
    /// (requires ActiveEventLoop for window creation).
    pub fn process_creates(
        &mut self,
        event_loop: &ActiveEventLoop,
        instance: &wgpu::Instance,
        device: &wgpu::Device,
        adapter: &wgpu::Adapter,
    ) {
        let pending = std::mem::take(&mut self.pending_creates);
        for req in pending {
            if self.windows.contains_key(&FrameKey::Adopted(req.emacs_frame_id)) {
                tracing::warn!("Window for frame {} already exists", req.emacs_frame_id);
                continue;
            }

            let attrs = Window::default_attributes()
                .with_title(&req.title)
                .with_inner_size(window_size_from_emacs_pixels(req.width, req.height))
                .with_transparent(true)
                .with_decorations(self.chrome_defaults.decorations_enabled);

            match event_loop.create_window(attrs) {
                Ok(window) => {
                    let window = Arc::new(window);
                    crate::window_icon::apply_window_icon(&window);
                    let raw_scale_factor = window.scale_factor();
                    let scale_factor = effective_window_scale_factor(raw_scale_factor);
                    let phys = window.inner_size();

                    // Create surface for this window using the primary display-bound instance.
                    let surface = match instance.create_surface(window.clone()) {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::error!(
                                "Failed to create surface for frame {}: {:?}",
                                req.emacs_frame_id,
                                e
                            );
                            continue;
                        }
                    };

                    // Configure surface
                    let caps = surface.get_capabilities(adapter);
                    let format = caps
                        .formats
                        .iter()
                        .copied()
                        .find(|f| f.is_srgb())
                        .unwrap_or(caps.formats[0]);
                    let alpha_mode = if caps
                        .alpha_modes
                        .contains(&wgpu::CompositeAlphaMode::PreMultiplied)
                    {
                        wgpu::CompositeAlphaMode::PreMultiplied
                    } else {
                        caps.alpha_modes[0]
                    };
                    let config = wgpu::SurfaceConfiguration {
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                        format,
                        width: phys.width,
                        height: phys.height,
                        present_mode: wgpu::PresentMode::Fifo,
                        alpha_mode,
                        view_formats: vec![],
                        desired_maximum_frame_latency: 2,
                    };
                    surface.configure(device, &config);

                    // Enable IME
                    window.set_ime_allowed(true);
                    apply_window_geometry_hints(&window, req.geometry_hints);

                    let winit_id = window.id();
                    tracing::info!(
                        "Created window for frame {} (winit {:?}, {}x{}, raw_scale={}, effective_scale={})",
                        req.emacs_frame_id,
                        winit_id,
                        phys.width,
                        phys.height,
                        raw_scale_factor,
                        scale_factor
                    );

                    self.winit_to_emacs.insert(winit_id, req.emacs_frame_id);
                    let chrome = WindowChrome {
                        title: req.title.clone(),
                        titlebar_hover: 0,
                        resize_edge: None,
                        last_titlebar_click: Instant::now(),
                        ..self.chrome_defaults.clone()
                    };
                    self.windows.insert(
                        FrameKey::Adopted(req.emacs_frame_id),
                        GuiFrameWindowState {
                            lifecycle: FrameLifecycle::Active {
                                native: GuiFrameNativeWindowState {
                                    window,
                                    surface,
                                    surface_config: config,
                                    width: phys.width,
                                    height: phys.height,
                                    scale_factor,
                                    mouse_hidden_for_typing: false,
                                    ime_enabled: false,
                                    last_ime_cursor_area: None,
                                    chrome: chrome.clone(),
                                },
                                mouse_hidden_for_typing: false,
                                ime_enabled: false,
                                last_ime_cursor_area: None,
                            },
                            render: GuiFrameRenderState::new(
                                req.emacs_frame_id,
                                device,
                                scale_factor,
                                self.fps_enabled,
                            ),
                        },
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to create window for frame {}: {:?}",
                        req.emacs_frame_id,
                        e
                    );
                }
            }
        }
    }

    /// Process pending window destructions.
    pub fn process_destroys(&mut self) {
        let pending = std::mem::take(&mut self.pending_destroys);
        for frame_id in pending {
            if let Some(state) = self.windows.remove(&FrameKey::Adopted(frame_id)) {
                if let Some(native) = state.lifecycle.native() {
                    self.winit_to_emacs.remove(&native.window.id());
                }
                tracing::info!("Destroyed window for frame {}", frame_id);
            }
        }
    }

    /// Drop all windows and their wgpu surfaces (for clean shutdown).
    pub fn destroy_all(&mut self) {
        self.pending_creates.clear();
        self.pending_destroys.clear();
        self.winit_to_emacs.clear();
        self.primary_winit_id = None;
        self.primary_emacs_frame_id = None;
        self.windows.clear();
    }

    /// Look up the Emacs frame_id for a winit WindowId.
    pub fn emacs_frame_for_winit(&self, winit_id: WindowId) -> Option<u64> {
        self.winit_to_emacs.get(&winit_id).copied()
    }

    pub fn event_frame_for_winit(&self, winit_id: WindowId) -> Option<u64> {
        if self.is_primary_winit(winit_id) {
            Some(self.primary_event_frame_id())
        } else {
            self.emacs_frame_for_winit(winit_id)
        }
    }

    /// Get a window state by Emacs frame_id.
    pub fn get(&self, emacs_frame_id: u64) -> Option<&GuiFrameWindowState> {
        if self.is_primary_frame_id(emacs_frame_id) {
            self.primary_window()
        } else {
            self.windows.get(&FrameKey::Adopted(emacs_frame_id))
        }
    }

    /// Get a mutable window state by Emacs frame_id.
    pub fn get_mut(&mut self, emacs_frame_id: u64) -> Option<&mut GuiFrameWindowState> {
        if self.is_primary_frame_id(emacs_frame_id) {
            self.primary_window_mut()
        } else {
            self.windows.get_mut(&FrameKey::Adopted(emacs_frame_id))
        }
    }

    /// Get a window state by winit WindowId.
    pub fn get_by_winit(&self, winit_id: WindowId) -> Option<&GuiFrameWindowState> {
        if self.primary_winit_id == Some(winit_id) {
            return self.primary_window();
        }
        self.winit_to_emacs
            .get(&winit_id)
            .and_then(|id| self.windows.get(&FrameKey::Adopted(*id)))
    }

    /// Get a mutable window state by winit WindowId.
    pub fn get_by_winit_mut(&mut self, winit_id: WindowId) -> Option<&mut GuiFrameWindowState> {
        if self.primary_winit_id == Some(winit_id) {
            return self.primary_window_mut();
        }
        self.winit_to_emacs
            .get(&winit_id)
            .copied()
            .and_then(move |id| self.windows.get_mut(&FrameKey::Adopted(id)))
    }

    pub(super) fn for_each_top_level_window(&self, mut f: impl FnMut(&GuiFrameWindowState)) {
        for window_state in self.windows.values() {
            f(window_state);
        }
    }

    pub(super) fn for_each_top_level_window_mut(
        &mut self,
        mut f: impl FnMut(&mut GuiFrameWindowState),
    ) {
        for window_state in self.windows.values_mut() {
            f(window_state);
        }
    }

    pub(super) fn mark_top_level_dirty(&mut self) {
        self.for_each_top_level_window_mut(|window_state| {
            window_state.render.compositor.dirty = true;
        });
    }

    pub(super) fn any_top_level_dirty(&self) -> bool {
        self.windows
            .values()
            .any(|window_state| window_state.render.compositor.dirty)
    }

    pub(super) fn any_top_level_renderer_effects_need_redraw(&self) -> bool {
        self.windows
            .values()
            .any(|window_state| window_state.render.compositor.renderer_effects.needs_redraw())
    }

    pub(super) fn any_top_level_transitions_active(&self) -> bool {
        self.windows
            .values()
            .any(|window_state| window_state.render.compositor.transitions.has_active())
    }

    pub(super) fn mark_active_top_level_visuals_dirty(&mut self) -> bool {
        let mut dirty = false;
        self.for_each_top_level_window_mut(|window_state| {
            dirty |= window_state.render.mark_active_visuals_dirty();
        });
        dirty
    }

    pub(super) fn tick_top_level_cursor_blinks(
        &mut self,
        now: Instant,
        cursor_wake_enabled: bool,
        renderer: Option<&WgpuRenderer>,
    ) -> bool {
        let primary_winit_id = self.primary_winit_id;
        let mut dirty = false;
        self.for_each_top_level_window_mut(|window_state| {
            let is_primary = window_state.lifecycle.native().is_some_and(|n| primary_winit_id == Some(n.window.id()));
            dirty |= window_state.render.tick_cursor_blink(
                now,
                cursor_wake_enabled && is_primary,
                renderer,
            );
        });
        dirty
    }

    pub(super) fn tick_top_level_cursor_animations(&mut self) -> bool {
        let mut dirty = false;
        self.for_each_top_level_window_mut(|window_state| {
            dirty |= window_state.render.tick_cursor_animation();
        });
        dirty
    }

    pub(super) fn tick_top_level_cursor_size_animations(&mut self) -> bool {
        let mut dirty = false;
        self.for_each_top_level_window_mut(|window_state| {
            dirty |= window_state.render.tick_cursor_size_animation();
        });
        dirty
    }

    pub(super) fn tick_top_level_idle_dim(&mut self, config: &IdleDimConfig) -> bool {
        let mut dirty = false;
        self.for_each_top_level_window_mut(|window_state| {
            dirty |= window_state.render.tick_idle_dim(config);
        });
        dirty
    }

    pub(super) fn clear_top_level_idle_dim(&mut self) {
        self.for_each_top_level_window_mut(|window_state| {
            window_state.render.clear_idle_dim();
        });
    }

    pub(super) fn any_top_level_cursor_animating(&self) -> bool {
        self.windows
            .values()
            .any(|window_state| window_state.render.cursor.is_animating())
    }

    pub(super) fn any_top_level_idle_dim_active(&self) -> bool {
        self.windows
            .values()
            .any(|window_state| window_state.render.overlays.idle_dim.active)
    }

    pub(super) fn request_redraw_for_dirty_top_level_windows(&self) {
        self.for_each_top_level_window(|window_state| {
            if window_state.render.compositor.dirty {
                window_state.request_redraw();
            }
        });
    }

    pub(super) fn request_redraw_for_top_level_windows(&self) {
        self.for_each_top_level_window(|window_state| {
            window_state.request_redraw();
        });
    }

    pub(super) fn set_top_level_titlebar_height(&mut self, height: f32) {
        self.chrome_defaults.titlebar_height = height;
        self.for_each_top_level_window_mut(|window_state| {
            window_state.chrome_mut().titlebar_height = height;
            window_state.render.compositor.dirty = true;
        });
    }

    pub(super) fn set_top_level_corner_radius(&mut self, radius: f32) {
        self.chrome_defaults.corner_radius = radius;
        self.for_each_top_level_window_mut(|window_state| {
            window_state.chrome_mut().corner_radius = radius;
            window_state.render.compositor.dirty = true;
        });
    }

    pub(super) fn set_top_level_fps_enabled(&mut self, enabled: bool) {
        self.fps_enabled = enabled;
        self.for_each_top_level_window_mut(|window_state| {
            window_state.render.overlays.fps.enabled = enabled;
            window_state.render.compositor.dirty = true;
        });
    }

    pub(super) fn set_top_level_decorations(&mut self, decorated: bool) {
        self.chrome_defaults.decorations_enabled = decorated;
        self.for_each_top_level_window_mut(|window_state| {
            window_state.set_decorations(decorated);
        });
    }

    pub(super) fn hide_top_level_popup_menus(&mut self) {
        self.for_each_top_level_window_mut(|window_state| {
            if window_state.render.overlays.popup_menu.is_some() {
                window_state.render.overlays.popup_menu = None;
                window_state.render.compositor.dirty = true;
            }
        });
    }

    pub(super) fn hide_top_level_tooltips(&mut self) {
        self.for_each_top_level_window_mut(|window_state| {
            if window_state.render.overlays.tooltip.is_some() {
                window_state.render.overlays.tooltip = None;
                window_state.render.compositor.dirty = true;
            }
        });
    }

    pub(super) fn sync_top_level_transition_policy(&mut self, policy: TransitionPolicy) {
        self.for_each_top_level_window_mut(|window_state| {
            window_state.render.compositor.transitions.policy = policy;
            window_state.render.compositor.dirty = true;
        });
    }

    pub(super) fn clear_top_level_crossfade_transitions(&mut self) {
        self.for_each_top_level_window_mut(|window_state| {
            window_state.render.compositor.transitions.crossfades.clear();
        });
    }

    pub(super) fn clear_top_level_scroll_transitions(&mut self) {
        self.for_each_top_level_window_mut(|window_state| {
            window_state.render.compositor.transitions.scroll_slides.clear();
        });
    }

    pub(super) fn remove_child_frame_from_top_level_windows(&mut self, frame_id: u64) -> bool {
        let mut changed = false;
        self.for_each_top_level_window_mut(|window_state| {
            changed |= window_state.remove_child_frame(frame_id);
        });
        changed
    }

    pub(super) fn sync_top_level_cursor_config(&mut self, defaults: &CursorState, dirty: bool) {
        self.for_each_top_level_window_mut(|window_state| {
            window_state.render.sync_cursor_config(defaults, dirty);
        });
    }

    pub(super) fn tick_top_level_child_frames(&mut self) {
        self.for_each_top_level_window_mut(|window_state| {
            window_state.render.compositor.child_frames.tick();
        });
    }

    pub(super) fn force_top_level_cursor_blink_on(&mut self) -> bool {
        let mut dirty = false;
        self.for_each_top_level_window_mut(|window_state| {
            dirty |= window_state.render.force_cursor_blink_on();
        });
        dirty
    }

    pub(super) fn clear_top_level_glyph_atlases(&mut self) {
        self.for_each_top_level_window_mut(|window_state| {
            if let Some(atlas) = window_state.render.compositor.glyph_atlas.as_mut() {
                atlas.clear();
            }
        });
    }

    pub(super) fn apply_top_level_visual_cursor_animations(&mut self) {
        self.for_each_top_level_window_mut(|window_state| {
            window_state.render.apply_visual_cursor_animations();
        });
    }

    #[cfg(feature = "wpe-webkit")]
    pub(super) fn remove_floating_webkit_from_top_level_windows(&mut self, id: u32) {
        self.for_each_top_level_window_mut(|window_state| {
            window_state.render.remove_floating_webkit(id);
        });
    }

    #[cfg(feature = "wpe-webkit")]
    pub(super) fn destroy_floating_webkit_from_top_level_windows(&mut self, id: u32) {
        self.for_each_top_level_window_mut(|window_state| {
            window_state.render.remove_floating_webkit(id);
        });
    }

    pub fn count(&self) -> usize {
        self.windows.len()
    }
}

#[cfg(test)]
#[path = "frame_windows_test.rs"]
mod tests;

use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;

use winit::dpi::{LogicalSize, PhysicalSize, Size};
use winit::window::Window;

use crate::core::face::Face;
use crate::core::frame_glyphs::FrameGlyphBuffer;
use crate::core::frame_glyphs::FrameTabBarState;
pub use crate::thread_comm::MonitorInfo;
use crate::thread_comm::RenderComms;
use neomacs_display_protocol::glyph_matrix::{
    GuiCompactBarState, GuiMenuBarState, GuiToolBarState,
};
use neomacs_display_protocol::{EffectsConfig, TransitionPolicy};
use neomacs_renderer_wgpu::{PopupMenuState, TooltipState, WgpuRenderer};

use super::child_frames::ChildFrameManager;
use super::cursor::CursorState;
use super::frame_windows::{
    GuiFrameRenderState, GuiFrameWindowManager, GuiFrameWindowState,
};

#[cfg(feature = "wpe-webkit")]
use crate::backend::wpe::{WpeBackend, WpeWebView};

/// Shared storage for image dimensions accessible from both threads.
pub type SharedImageDimensions = Arc<(Mutex<HashMap<u32, (u32, u32)>>, Condvar)>;

/// Shared storage for monitor info accessible from both threads.
/// The Condvar is notified once monitors have been populated.
pub type SharedMonitorInfo = Arc<(Mutex<Vec<MonitorInfo>>, std::sync::Condvar)>;

pub(super) fn backend_uses_winit_logical_pixels() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("WAYLAND_DISPLAY").is_some()
    }
    #[cfg(not(target_os = "linux"))]
    {
        true
    }
}

pub(super) fn effective_window_scale_factor(raw_scale_factor: f64) -> f64 {
    // On X11 fontconfig already handles DPI — the font metrics returned are
    // already scaled for the display.  Only Wayland needs us to apply the
    // compositor scale factor to rendering.
    if backend_uses_winit_logical_pixels() {
        raw_scale_factor
    } else {
        1.0
    }
}

pub(super) fn window_size_from_emacs_pixels(width: u32, height: u32) -> Size {
    if backend_uses_winit_logical_pixels() {
        Size::Logical(LogicalSize::new(width as f64, height as f64))
    } else {
        // X11: physical pixels as-is, matching GNU Emacs.  fontconfig DPI
        // already scales font sizes, so window dimensions are already at
        // the correct physical size.
        Size::Physical(PhysicalSize::new(width, height))
    }
}

pub(super) fn emacs_pixels_from_window_size(
    width: u32,
    height: u32,
    scale_factor: f64,
) -> (u32, u32) {
    if backend_uses_winit_logical_pixels() {
        (
            (width as f64 / scale_factor).round() as u32,
            (height as f64 / scale_factor).round() as u32,
        )
    } else {
        // X11: fontconfig handles DPI.  Return physical pixels as-is
        // so Emacs computes the correct character grid with the already-
        // scaled font metrics.
        (width, height)
    }
}

#[cfg(feature = "wpe-webkit")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WebKitImportPolicy {
    /// Prefer raw pixel upload first, fallback to DMA-BUF.
    PixelsFirst,
    /// Prefer DMA-BUF import first, fallback to raw pixels.
    DmaBufFirst,
    /// Default compatibility mode (currently PixelsFirst).
    Auto,
}

#[cfg(feature = "wpe-webkit")]
impl WebKitImportPolicy {
    fn from_env() -> Self {
        match std::env::var("NEOMACS_WEBKIT_IMPORT").ok().as_deref() {
            Some("dmabuf-first") | Some("dmabuf") | Some("dma-buf-first") => {
                tracing::info!("NEOMACS_WEBKIT_IMPORT=dmabuf-first");
                Self::DmaBufFirst
            }
            Some("pixels-first") | Some("pixels") => {
                tracing::info!("NEOMACS_WEBKIT_IMPORT=pixels-first");
                Self::PixelsFirst
            }
            Some("auto") => {
                tracing::info!("NEOMACS_WEBKIT_IMPORT=auto (effective: pixels-first)");
                Self::Auto
            }
            Some(val) => {
                tracing::warn!(
                    "NEOMACS_WEBKIT_IMPORT={}: unrecognized value, defaulting to auto (effective: pixels-first)",
                    val
                );
                Self::Auto
            }
            None => {
                tracing::info!("NEOMACS_WEBKIT_IMPORT not set (effective: pixels-first)");
                Self::Auto
            }
        }
    }

    pub(super) fn effective(self) -> Self {
        match self {
            Self::Auto => Self::PixelsFirst,
            other => other,
        }
    }
}

/// FPS counter and frame time tracking state.
#[derive(Clone)]
pub(super) struct FpsCounter {
    pub(super) enabled: bool,
    pub(super) last_instant: Instant,
    pub(super) frame_count: u32,
    pub(super) display_value: f32,
    pub(super) frame_time_ms: f32,
    pub(super) render_start: Instant,
}

/// Typing-speed overlay state for one native GUI frame window.
#[derive(Default)]
pub(super) struct TypingSpeedState {
    /// Key press timestamps for WPM calculation.
    pub(super) key_press_times: Vec<Instant>,
    /// Smoothed WPM value for display.
    pub(super) displayed_wpm: f32,
}

/// Idle dim overlay state for one native GUI frame window.
pub(super) struct IdleDimState {
    pub(super) last_activity_time: Instant,
    pub(super) current_alpha: f32,
    pub(super) active: bool,
}

impl Default for IdleDimState {
    fn default() -> Self {
        Self {
            last_activity_time: Instant::now(),
            current_alpha: 0.0,
            active: false,
        }
    }
}

impl Default for FpsCounter {
    fn default() -> Self {
        Self {
            enabled: false,
            last_instant: Instant::now(),
            frame_count: 0,
            display_value: 0.0,
            frame_time_ms: 0.0,
            render_start: Instant::now(),
        }
    }
}

/// Borderless native-window chrome state (title bar, resize edges, decorations).
#[derive(Clone)]
pub(super) struct WindowChrome {
    pub(super) decorations_enabled: bool,
    pub(super) resize_edge: Option<winit::window::ResizeDirection>,
    pub(super) title: String,
    pub(super) titlebar_height: f32,
    pub(super) titlebar_hover: u32,
    pub(super) last_titlebar_click: Instant,
    pub(super) is_fullscreen: bool,
    pub(super) corner_radius: f32,
}

impl Default for WindowChrome {
    fn default() -> Self {
        Self {
            decorations_enabled: true,
            resize_edge: None,
            title: String::from("neomacs"),
            titlebar_height: 30.0,
            titlebar_hover: 0,
            last_titlebar_click: Instant::now(),
            is_fullscreen: false,
            corner_radius: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ImeCursorArea {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) width: u32,
    pub(super) height: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct GuiChromeInteractionState {
    pub(super) menu_bar_hovered: Option<u32>,
    pub(super) menu_bar_active: Option<u32>,
    pub(super) tab_bar_hovered: Option<u32>,
    pub(super) tab_bar_pressed: Option<u32>,
    pub(super) tab_bar_press_captured: bool,
    pub(super) toolbar_hovered: Option<u32>,
    pub(super) toolbar_pressed: Option<u32>,
    pub(super) toolbar_press_captured: bool,
    pub(super) compact_bar_menu_hovered: Option<u32>,
    pub(super) compact_bar_menu_active: Option<u32>,
    pub(super) compact_bar_tool_hovered: Option<u32>,
    pub(super) compact_bar_tool_pressed: Option<u32>,
}

impl GuiChromeInteractionState {
    pub(super) fn clear_menu_bar(&mut self) {
        self.menu_bar_hovered = None;
        self.menu_bar_active = None;
    }

    pub(super) fn clear_tab_bar(&mut self) {
        self.tab_bar_hovered = None;
        self.tab_bar_pressed = None;
        // Preserve press capture across chrome removal so a chrome press does
        // not leak a buffer release if the tab bar disappears mid-click.
    }

    pub(super) fn clear_toolbar(&mut self) {
        self.toolbar_hovered = None;
        self.toolbar_pressed = None;
        // Preserve press capture across chrome removal so a chrome press does
        // not leak a buffer release if the toolbar disappears mid-click.
    }

    pub(super) fn clear_compact_bar(&mut self) {
        self.compact_bar_menu_hovered = None;
        self.compact_bar_menu_active = None;
        self.compact_bar_tool_hovered = None;
        self.compact_bar_tool_pressed = None;
    }
}

pub(super) struct RenderGpuContext {
    pub(super) instance: wgpu::Instance,
    pub(super) adapter: wgpu::Adapter,
    pub(super) device: Arc<wgpu::Device>,
    pub(super) queue: Arc<wgpu::Queue>,
}

pub(super) struct RenderApp {
    pub(super) comms: RenderComms,
    pub(super) primary_window_destroyed: bool,

    pub(super) gpu: Option<RenderGpuContext>,
    pub(super) renderer: Option<WgpuRenderer>,
    #[cfg(test)]
    pub(super) primary_render_state_for_tests: Option<GuiFrameRenderState>,

    pub(super) faces: HashMap<u32, Face>,

    pub(super) modifiers: u32,

    pub(super) image_dimensions: SharedImageDimensions,

    pub(super) cursor_defaults: CursorState,

    pub(super) effects: EffectsConfig,

    pub(super) transition_policy: TransitionPolicy,
    #[cfg(feature = "wpe-webkit")]
    pub(super) wpe_backend: Option<WpeBackend>,

    #[cfg(feature = "wpe-webkit")]
    pub(super) webkit_views: HashMap<u32, WpeWebView>,

    #[cfg(feature = "wpe-webkit")]
    pub(super) webkit_import_policy: WebKitImportPolicy,

    #[cfg(feature = "neo-term")]
    pub(super) terminal_manager: crate::terminal::TerminalManager,
    #[cfg(feature = "neo-term")]
    pub(super) shared_terminals: crate::terminal::SharedTerminals,

    pub(super) frame_windows: GuiFrameWindowManager,

    pub(super) child_frame_corner_radius: f32,
    pub(super) child_frame_shadow_enabled: bool,
    pub(super) child_frame_shadow_layers: u32,
    pub(super) child_frame_shadow_offset: f32,
    pub(super) child_frame_shadow_opacity: f32,

    pub(super) toolbar_icon_textures: HashMap<String, u32>,
    pub(super) toolbar_icon_size: u32,
    pub(super) toolbar_padding: u32,

    pub(super) scroll_indicators_enabled: bool,
    pub(super) primary_fps_enabled: bool,

    pub(super) extra_line_spacing: f32,
    pub(super) extra_letter_spacing: f32,

    pub(super) shared_monitors: Option<SharedMonitorInfo>,
    pub(super) monitors_populated: bool,
    pub(super) last_monitor_snapshot: Vec<MonitorInfo>,
    pub(super) debug_first_frame_readback_pending: bool,
    pub(super) debug_surface_readback_frames_remaining: u32,
    pub(super) resumed_seen: bool,
    pub(super) about_to_wait_seen: bool,
    pub(super) poll_when_idle: bool,
}

impl RenderApp {
    pub(super) fn new(
        comms: RenderComms,
        width: u32,
        height: u32,
        title: String,
        image_dimensions: SharedImageDimensions,
        shared_monitors: SharedMonitorInfo,
        poll_when_idle: bool,
        #[cfg(feature = "neo-term")] shared_terminals: crate::terminal::SharedTerminals,
    ) -> Self {
        #[cfg(feature = "wpe-webkit")]
        let webkit_import_policy = WebKitImportPolicy::from_env();

        let mut frame_windows = GuiFrameWindowManager::new();
        frame_windows.set_primary_pending(GuiFrameWindowState {
            native: None,
            render: GuiFrameRenderState::new_without_device(0, false),
            pending_width: width,
            pending_height: height,
            pending_scale_factor: 1.0,
            pending_mouse_hidden_for_typing: false,
            pending_ime_enabled: false,
            pending_last_ime_cursor_area: None,
            pending_chrome: WindowChrome {
                title,
                ..WindowChrome::default()
            },
            pending_geometry_hints: None,
        });

        Self {
            comms,
            primary_window_destroyed: false,
            gpu: None,
            renderer: None,
            #[cfg(test)]
            primary_render_state_for_tests: None,
            faces: HashMap::new(),
            modifiers: 0,
            image_dimensions,
            cursor_defaults: CursorState::default(),
            effects: EffectsConfig::default(),
            transition_policy: TransitionPolicy::default(),
            #[cfg(feature = "wpe-webkit")]
            wpe_backend: None,
            #[cfg(feature = "wpe-webkit")]
            webkit_views: HashMap::new(),
            #[cfg(feature = "wpe-webkit")]
            webkit_import_policy,
            #[cfg(feature = "neo-term")]
            terminal_manager: crate::terminal::TerminalManager::new(),
            #[cfg(feature = "neo-term")]
            shared_terminals,
            frame_windows,
            child_frame_corner_radius: 8.0,
            child_frame_shadow_enabled: true,
            child_frame_shadow_layers: 4,
            child_frame_shadow_offset: 2.0,
            child_frame_shadow_opacity: 0.3,
            toolbar_icon_textures: HashMap::new(),
            toolbar_icon_size: 24,
            toolbar_padding: 5,
            scroll_indicators_enabled: false,
            primary_fps_enabled: false,
            extra_line_spacing: 0.0,
            extra_letter_spacing: 0.0,
            shared_monitors: Some(shared_monitors),
            monitors_populated: false,
            last_monitor_snapshot: Vec::new(),
            debug_first_frame_readback_pending: std::env::var_os(
                "NEOMACS_DEBUG_FIRST_FRAME_READBACK",
            )
            .is_some(),
            debug_surface_readback_frames_remaining: std::env::var(
                "NEOMACS_DEBUG_SURFACE_READBACK",
            )
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|count| *count > 0)
            .unwrap_or_else(|| {
                if std::env::var_os("NEOMACS_DEBUG_SURFACE_READBACK").is_some() {
                    32
                } else {
                    0
                }
            }),
            resumed_seen: false,
            about_to_wait_seen: false,
            poll_when_idle,
        }
    }

    pub(super) fn primary_window_state(&self) -> Option<&GuiFrameWindowState> {
        self.frame_windows.primary_window()
    }

    pub(super) fn primary_window_state_mut(&mut self) -> Option<&mut GuiFrameWindowState> {
        self.frame_windows.primary_window_mut()
    }

    pub(super) fn primary_render_state(&self) -> Option<&GuiFrameRenderState> {
        self.frame_windows.primary_window().map(|ws| &ws.render)
    }

    pub(super) fn primary_render_state_mut(&mut self) -> Option<&mut GuiFrameRenderState> {
        self.frame_windows.primary_window_mut().map(|ws| &mut ws.render)
    }

    #[cfg(test)]
    pub(super) fn set_primary_render_state_for_tests(&mut self, render: GuiFrameRenderState) {
        if let Some(window_state) = self.frame_windows.primary_window_mut() {
            window_state.render = render;
        } else {
            self.primary_render_state_for_tests = Some(render);
        }
    }

    pub(super) fn primary_current_frame(&self) -> Option<&FrameGlyphBuffer> {
        self.primary_render_state()
            .and_then(|frame| frame.current_frame.as_ref())
    }

    pub(super) fn primary_current_frame_mut(&mut self) -> Option<&mut FrameGlyphBuffer> {
        self.primary_render_state_mut()
            .and_then(|frame| frame.current_frame.as_mut())
    }

    pub(super) fn set_primary_current_frame(&mut self, frame: Option<FrameGlyphBuffer>) {
        if let Some(primary_frame) = self.primary_render_state_mut() {
            primary_frame.set_current_frame(frame);
        }
    }

    pub(super) fn primary_dirty(&self) -> bool {
        self.primary_render_state()
            .is_some_and(|frame| frame.frame_dirty)
    }

    pub(super) fn mark_primary_dirty(&mut self) {
        self.set_primary_dirty(true);
    }

    pub(super) fn mark_top_level_frame_windows_dirty(&mut self) {
        self.frame_windows.mark_top_level_dirty();
    }

    pub(super) fn set_top_level_titlebar_height(&mut self, height: f32) {
        self.frame_windows.set_top_level_titlebar_height(height);
    }

    pub(super) fn set_top_level_corner_radius(&mut self, radius: f32) {
        self.frame_windows.set_top_level_corner_radius(radius);
    }

    pub(super) fn set_top_level_fps_enabled(&mut self, enabled: bool) {
        self.primary_fps_enabled = enabled;
        self.frame_windows.set_top_level_fps_enabled(enabled);
    }

    pub(super) fn set_primary_dirty(&mut self, dirty: bool) {
        if let Some(primary_frame) = self.primary_render_state_mut() {
            primary_frame.frame_dirty = dirty;
        }
    }

    pub(super) fn primary_fps_enabled(&self) -> bool {
        self.primary_fps_enabled
    }

    pub(super) fn primary_char_width(&self) -> f32 {
        self.primary_render_state()
            .and_then(|frame| frame.glyph_atlas.as_ref())
            .map_or(8.0, |atlas| atlas.default_char_width())
    }

    pub(super) fn primary_popup_menu(&self) -> Option<&PopupMenuState> {
        self.primary_render_state()
            .and_then(|frame| frame.popup_menu.as_ref())
    }

    pub(super) fn primary_popup_menu_mut(&mut self) -> Option<&mut PopupMenuState> {
        self.primary_render_state_mut()
            .and_then(|frame| frame.popup_menu.as_mut())
    }

    pub(super) fn set_primary_popup_menu(&mut self, popup_menu: Option<PopupMenuState>) {
        if let Some(primary_frame) = self.primary_render_state_mut() {
            primary_frame.set_popup_menu(popup_menu);
        }
    }

    pub(super) fn hide_top_level_popup_menus(&mut self) {
        self.frame_windows.hide_top_level_popup_menus();
        if let Some(primary_frame) = self.primary_render_state_mut() {
            primary_frame.set_popup_menu(None);
        }
    }

    pub(super) fn set_primary_tooltip(&mut self, tooltip: Option<TooltipState>) {
        if let Some(primary_frame) = self.primary_render_state_mut() {
            primary_frame.set_tooltip(tooltip);
        }
    }

    pub(super) fn set_primary_menu_bar(&mut self, menu_bar: Option<GuiMenuBarState>) {
        if let Some(primary_frame) = self.primary_render_state_mut() {
            primary_frame.set_menu_bar(menu_bar);
        }
    }

    pub(super) fn set_primary_tool_bar(&mut self, tool_bar: Option<GuiToolBarState>) {
        if let Some(primary_frame) = self.primary_render_state_mut() {
            primary_frame.set_tool_bar(tool_bar);
        }
    }

    pub(super) fn set_primary_compact_bar(&mut self, compact_bar: Option<GuiCompactBarState>) {
        if let Some(primary_frame) = self.primary_render_state_mut() {
            primary_frame.set_compact_bar(compact_bar);
        }
    }

    pub(super) fn hide_top_level_tooltips(&mut self) {
        self.frame_windows.hide_top_level_tooltips();
        if let Some(primary_frame) = self.primary_render_state_mut() {
            primary_frame.set_tooltip(None);
        }
    }

    pub(super) fn set_primary_visual_bell_start(&mut self, start: Option<Instant>) {
        if let Some(primary_frame) = self.primary_render_state_mut() {
            primary_frame.set_visual_bell_start(start);
        }
    }

    pub(super) fn primary_mouse_pos(&self) -> (f32, f32) {
        self.primary_render_state()
            .map_or((0.0, 0.0), |frame| frame.mouse_pos)
    }

    pub(super) fn set_primary_mouse_pos(&mut self, mouse_pos: (f32, f32)) {
        if let Some(primary_frame) = self.primary_render_state_mut() {
            primary_frame.set_mouse_pos(mouse_pos);
        }
    }

    pub(super) fn primary_cursor(&self) -> &CursorState {
        self.primary_render_state()
            .map_or(&self.cursor_defaults, |frame| &frame.cursor)
    }

    pub(super) fn primary_cursor_mut(&mut self) -> Option<&mut CursorState> {
        self.primary_render_state_mut()
            .map(|frame| &mut frame.cursor)
    }

    pub(super) fn primary_child_frames(&self) -> &ChildFrameManager {
        if let Some(frame) = self.primary_render_state() {
            return &frame.child_frames;
        }
        #[cfg(test)]
        {
            if let Some(frame) = self.primary_render_state_for_tests.as_ref() {
                return &frame.child_frames;
            }
        }
        panic!("primary child frames")
    }

    pub(super) fn primary_child_frames_mut(&mut self) -> &mut ChildFrameManager {
        if self.frame_windows.primary_window_mut().is_some() {
            return &mut self.frame_windows.primary_window_mut().unwrap().render.child_frames;
        }
        #[cfg(test)]
        {
            if let Some(frame) = self.primary_render_state_for_tests.as_mut() {
                return &mut frame.child_frames;
            }
        }
        panic!("primary child frames mut")
    }

    pub(super) fn remove_primary_child_frame(&mut self, frame_id: u64) -> bool {
        let target_was_child = self
            .primary_cursor()
            .target_cloned()
            .is_some_and(|target| target.frame_id == frame_id);
        if let Some(primary_frame) = self.primary_render_state_mut() {
            let changed = primary_frame.remove_child_frame(frame_id);
            if target_was_child {
                self.reset_primary_ime_cursor_area();
            }
            changed
        } else {
            false
        }
    }

    pub(super) fn update_primary_child_frame(&mut self, frame: FrameGlyphBuffer) {
        if let Some(primary_frame) = self.primary_render_state_mut() {
            primary_frame.update_child_frame(frame);
        }
    }

    pub(super) fn primary_transitions_active(&self) -> bool {
        self.primary_render_state()
            .is_some_and(|frame| frame.transitions.has_active())
    }

    pub(super) fn primary_renderer_effects_need_redraw(&self) -> bool {
        self.primary_render_state()
            .is_some_and(|frame| frame.renderer_effects.needs_redraw())
    }

    pub(super) fn sync_primary_transition_policy_from_default(&mut self) {
        let transition_policy = self.transition_policy;
        if let Some(primary_frame) = self.primary_render_state_mut() {
            primary_frame.transitions.policy = transition_policy;
        }
    }

    pub(super) fn sync_top_level_transition_policy_from_default(&mut self) {
        self.frame_windows
            .sync_top_level_transition_policy(self.transition_policy);
        self.sync_primary_transition_policy_from_default();
    }

    pub(super) fn clear_top_level_crossfade_transitions(&mut self) {
        self.frame_windows.clear_top_level_crossfade_transitions();
        if let Some(primary_frame) = self.primary_render_state_mut() {
            primary_frame.transitions.crossfades.clear();
        }
    }

    pub(super) fn clear_top_level_scroll_transitions(&mut self) {
        self.frame_windows.clear_top_level_scroll_transitions();
        if let Some(primary_frame) = self.primary_render_state_mut() {
            primary_frame.transitions.scroll_slides.clear();
        }
    }

    pub(super) fn primary_menu_bar(&self) -> Option<&GuiMenuBarState> {
        self.primary_render_state()
            .and_then(|frame| frame.menu_bar.as_ref())
    }

    pub(super) fn primary_tool_bar(&self) -> Option<&GuiToolBarState> {
        self.primary_render_state()
            .and_then(|frame| frame.tool_bar.as_ref())
    }

    pub(super) fn primary_compact_bar(&self) -> Option<&GuiCompactBarState> {
        self.primary_render_state()
            .and_then(|frame| frame.compact_bar.as_ref())
    }

    pub(super) fn primary_tab_bar(&self) -> Option<&FrameTabBarState> {
        self.primary_current_frame()
            .and_then(|frame| frame.tab_bar.as_ref())
    }

    pub(super) fn primary_window(&self) -> Option<&Arc<Window>> {
        self.frame_windows.primary_window().and_then(|ws| ws.window())
    }

    pub(super) fn primary_native_size(&self) -> (u32, u32) {
        self.frame_windows.primary_window().map_or((0, 0), |ws| ws.native_size())
    }

    pub(super) fn primary_logical_size(&self) -> (f32, f32) {
        let (width, height) = self.primary_native_size();
        let scale_factor = self.primary_scale_factor() as f32;
        (width as f32 / scale_factor, height as f32 / scale_factor)
    }

    pub(super) fn primary_scale_factor(&self) -> f64 {
        self.frame_windows.primary_window().map_or(1.0, |ws| ws.scale_factor())
    }

    pub(super) fn set_primary_scale_factor(&mut self, scale_factor: f64) {
        if let Some(window_state) = self.frame_windows.primary_window_mut() {
            window_state.set_scale_factor(scale_factor);
        }
    }

    pub(super) fn primary_chrome(&self) -> &WindowChrome {
        self.frame_windows.primary_window().expect("primary window state").chrome()
    }

    pub(super) fn primary_chrome_mut(&mut self) -> &mut WindowChrome {
        let ws = self.frame_windows.primary_window_mut().expect("primary window state");
        ws.chrome_mut()
    }

    pub(super) fn primary_ime_enabled(&self) -> bool {
        self.frame_windows.primary_window().map_or(false, |ws| ws.ime_enabled())
    }

    pub(super) fn clear_primary_mouse_hidden_for_typing(&mut self) {
        if let Some(window_state) = self.frame_windows.primary_window_mut() {
            window_state.set_mouse_hidden_for_typing(false);
        }
    }

    pub(super) fn update_primary_popup_hover(&mut self, lx: f32, ly: f32) -> bool {
        self.primary_render_state_mut().is_some_and(|render| {
            let had_popup = render.popup_menu.is_some();
            render.update_popup_hover(lx, ly);
            had_popup
        })
    }

    pub(super) fn set_primary_ime_enabled(&mut self, enabled: bool) {
        if let Some(window_state) = self.frame_windows.primary_window_mut() {
            window_state.set_ime_enabled(enabled);
        }
    }

    pub(super) fn reset_primary_ime_cursor_area(&mut self) {
        if let Some(window_state) = self.frame_windows.primary_window_mut() {
            window_state.reset_ime_cursor_area();
        }
    }

    pub(super) fn primary_chrome_interaction(&self) -> GuiChromeInteractionState {
        self.primary_render_state()
            .map_or(GuiChromeInteractionState::default(), |frame| {
                frame.chrome_interaction
            })
    }

    pub(super) fn with_primary_chrome_interaction_mut(
        &mut self,
        f: impl FnOnce(&mut GuiChromeInteractionState),
    ) -> bool {
        if let Some(primary_frame) = self.primary_render_state_mut() {
            primary_frame.with_chrome_interaction_mut(f)
        } else {
            false
        }
    }

    pub(super) fn primary_ime_preedit_active(&self) -> bool {
        self.primary_render_state()
            .is_some_and(|frame| frame.ime_preedit_active)
    }

    pub(super) fn set_primary_ime_preedit(&mut self, text: String) {
        if let Some(primary_frame) = self.primary_render_state_mut() {
            primary_frame.set_ime_preedit(text);
        }
    }

    pub(super) fn clear_primary_ime_preedit(&mut self) {
        if let Some(primary_frame) = self.primary_render_state_mut() {
            primary_frame.clear_ime_preedit();
        }
    }

    pub(super) fn sync_primary_cursor_config_from_defaults(&mut self) {
        let defaults = &self.cursor_defaults;
        let values = (
            defaults.blink_enabled,
            defaults.blink_interval,
            defaults.anim_enabled,
            defaults.anim_speed,
            defaults.anim_style,
            defaults.anim_duration,
            defaults.trail_size,
            defaults.size_transition_enabled,
            defaults.size_transition_duration,
        );
        if let Some(cursor) = self.primary_cursor_mut() {
            cursor.copy_config_from_values(
                values.0, values.1, values.2, values.3, values.4, values.5, values.6, values.7,
                values.8,
            );
        }
    }

    pub(super) fn sync_top_level_cursor_config_from_defaults(&mut self) {
        self.frame_windows
            .sync_top_level_cursor_config(&self.cursor_defaults, true);
        self.sync_primary_cursor_config_from_defaults();
    }

    pub(super) fn sync_top_level_cursor_config_from_defaults_without_dirty(&mut self) {
        self.frame_windows
            .sync_top_level_cursor_config(&self.cursor_defaults, false);
        self.sync_primary_cursor_config_from_defaults();
    }
}

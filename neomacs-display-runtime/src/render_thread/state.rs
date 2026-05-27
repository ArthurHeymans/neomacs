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
use neomacs_display_protocol::EffectsConfig;
use neomacs_display_protocol::glyph_matrix::{
    GuiCompactBarState, GuiMenuBarState, GuiToolBarState,
};
use neomacs_renderer_wgpu::{PopupMenuState, RendererFrameEffects, TooltipState, WgpuRenderer};
use neovm_core::window::GuiFrameGeometryHints;

use super::child_frames::ChildFrameManager;
use super::cursor::CursorState;
use super::frame_windows::{GuiFrameRenderState, GuiFrameWindowManager};
use super::transitions::TransitionState;

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
    pub(super) toolbar_hovered: Option<u32>,
    pub(super) toolbar_pressed: Option<u32>,
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
    }

    pub(super) fn clear_toolbar(&mut self) {
        self.toolbar_hovered = None;
        self.toolbar_pressed = None;
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
    pub(super) window: Option<Arc<Window>>,
    pub(super) primary_window_destroyed: bool,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) title: String,
    pub(super) primary_geometry_hints: Option<GuiFrameGeometryHints>,

    // Shared wgpu context used by the primary surface and secondary windows.
    pub(super) gpu: Option<RenderGpuContext>,
    pub(super) renderer: Option<WgpuRenderer>,
    pub(super) surface: Option<wgpu::Surface<'static>>,
    pub(super) surface_config: Option<wgpu::SurfaceConfiguration>,
    /// Frame-owned render state for the adopted primary GUI frame.
    pub(super) primary_frame: Option<GuiFrameRenderState>,

    // Face cache built from frame data
    pub(super) faces: HashMap<u32, Face>,

    // Display scale factor (physical pixels / logical pixels)
    pub(super) scale_factor: f64,

    // Current modifier state (NEOMACS_*_MASK flags)
    pub(super) modifiers: u32,

    /// Whether the mouse cursor is hidden during keyboard input
    pub(super) mouse_hidden_for_typing: bool,

    // Shared image dimensions (written here, read from main thread)
    pub(super) image_dimensions: SharedImageDimensions,

    // Cursor state (blink, animation, size transition)
    pub(super) cursor: CursorState,
    // Render-only visual cursors keyed by their stable visual cursor id.
    pub(super) visual_cursors: HashMap<i64, CursorState>,

    // All visual effect configurations
    pub(super) effects: EffectsConfig,

    // Window transition state (crossfade, scroll)
    pub(super) transitions: TransitionState,
    /// Renderer runtime effects owned by the primary GUI frame window.
    pub(super) renderer_effects: RendererFrameEffects,

    // WebKit state (video cache is managed by renderer)
    #[cfg(feature = "wpe-webkit")]
    pub(super) wpe_backend: Option<WpeBackend>,

    #[cfg(feature = "wpe-webkit")]
    pub(super) webkit_views: HashMap<u32, WpeWebView>,

    #[cfg(feature = "wpe-webkit")]
    pub(super) webkit_import_policy: WebKitImportPolicy,

    // Floating WebKit overlays (position/size from C side, rendered on render thread)
    #[cfg(feature = "wpe-webkit")]
    pub(super) floating_webkits: Vec<crate::core::scene::FloatingWebKit>,

    // Terminal manager (neo-term)
    #[cfg(feature = "neo-term")]
    pub(super) terminal_manager: crate::terminal::TerminalManager,
    #[cfg(feature = "neo-term")]
    pub(super) shared_terminals: crate::terminal::SharedTerminals,

    // Top-level GUI frame windows.
    pub(super) frame_windows: GuiFrameWindowManager,
    // Child frames (posframe, which-key-posframe, etc.)
    pub(super) child_frames: ChildFrameManager,
    // Child frame visual style
    pub(super) child_frame_corner_radius: f32,
    pub(super) child_frame_shadow_enabled: bool,
    pub(super) child_frame_shadow_layers: u32,
    pub(super) child_frame_shadow_offset: f32,
    pub(super) child_frame_shadow_opacity: f32,

    // GUI menu bar snapshot for the primary frame, if visible.
    pub(super) menu_bar: Option<GuiMenuBarState>,

    // Frame tab bar metadata for the primary frame, if visible.
    pub(super) tab_bar: Option<FrameTabBarState>,

    // GUI toolbar snapshot for the primary frame, if visible.
    pub(super) tool_bar: Option<GuiToolBarState>,
    pub(super) toolbar_icon_textures: HashMap<String, u32>,
    pub(super) toolbar_icon_size: u32,
    pub(super) toolbar_padding: u32,

    // Compact GUI chrome snapshot for the primary frame, if visible.
    pub(super) compact_bar: Option<GuiCompactBarState>,
    pub(super) chrome_interaction: GuiChromeInteractionState,

    // IME state
    pub(super) ime_enabled: bool,
    pub(super) ime_preedit_active: bool,
    pub(super) ime_preedit_text: String,
    pub(super) last_ime_cursor_area: Option<ImeCursorArea>,

    // UI overlay state
    pub(super) scroll_indicators_enabled: bool,
    pub(super) primary_fps_enabled: bool,
    pub(super) pending_primary_popup_menu: Option<PopupMenuState>,
    pub(super) pending_primary_tooltip: Option<TooltipState>,
    pub(super) pending_primary_visual_bell_start: Option<Instant>,

    // Window chrome (borderless title bar, resize, decorations)
    pub(super) chrome: WindowChrome,
    /// Extra line spacing in pixels (added between rows)
    pub(super) extra_line_spacing: f32,
    /// Extra letter spacing in pixels (added between characters)
    pub(super) extra_letter_spacing: f32,

    /// Shared monitor info (populated in resumed(), read from FFI thread)
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

        Self {
            comms,
            window: None,
            primary_window_destroyed: false,
            width,
            height,
            title,
            primary_geometry_hints: None,
            scale_factor: 1.0,
            gpu: None,
            renderer: None,
            surface: None,
            surface_config: None,
            primary_frame: None,
            faces: HashMap::new(),
            modifiers: 0,
            mouse_hidden_for_typing: false,
            image_dimensions,
            cursor: CursorState::default(),
            visual_cursors: HashMap::new(),
            effects: EffectsConfig::default(),
            transitions: TransitionState::default(),
            renderer_effects: RendererFrameEffects::default(),
            #[cfg(feature = "wpe-webkit")]
            wpe_backend: None,
            #[cfg(feature = "wpe-webkit")]
            webkit_views: HashMap::new(),
            #[cfg(feature = "wpe-webkit")]
            webkit_import_policy,
            #[cfg(feature = "wpe-webkit")]
            floating_webkits: Vec::new(),
            #[cfg(feature = "neo-term")]
            terminal_manager: crate::terminal::TerminalManager::new(),
            #[cfg(feature = "neo-term")]
            shared_terminals,
            frame_windows: GuiFrameWindowManager::new(),
            child_frames: ChildFrameManager::new(),
            child_frame_corner_radius: 8.0,
            child_frame_shadow_enabled: true,
            child_frame_shadow_layers: 4,
            child_frame_shadow_offset: 2.0,
            child_frame_shadow_opacity: 0.3,
            menu_bar: None,
            tab_bar: None,
            tool_bar: None,
            toolbar_icon_textures: HashMap::new(),
            toolbar_icon_size: 24,
            toolbar_padding: 5,
            compact_bar: None,
            chrome_interaction: GuiChromeInteractionState::default(),
            ime_enabled: false,
            ime_preedit_active: false,
            ime_preedit_text: String::new(),
            last_ime_cursor_area: None,
            scroll_indicators_enabled: false,
            primary_fps_enabled: false,
            pending_primary_popup_menu: None,
            pending_primary_tooltip: None,
            pending_primary_visual_bell_start: None,
            chrome: WindowChrome::default(),
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

    pub(super) fn primary_current_frame(&self) -> Option<&FrameGlyphBuffer> {
        self.primary_frame
            .as_ref()
            .and_then(|frame| frame.current_frame.as_ref())
    }

    pub(super) fn primary_current_frame_mut(&mut self) -> Option<&mut FrameGlyphBuffer> {
        self.primary_frame
            .as_mut()
            .and_then(|frame| frame.current_frame.as_mut())
    }

    pub(super) fn set_primary_current_frame(&mut self, frame: Option<FrameGlyphBuffer>) {
        if let Some(primary_frame) = self.primary_frame.as_mut() {
            primary_frame.current_frame = frame;
        }
    }

    pub(super) fn primary_dirty(&self) -> bool {
        self.primary_frame
            .as_ref()
            .is_some_and(|frame| frame.frame_dirty)
    }

    pub(super) fn mark_primary_dirty(&mut self) {
        self.set_primary_dirty(true);
    }

    pub(super) fn set_primary_dirty(&mut self, dirty: bool) {
        if let Some(primary_frame) = self.primary_frame.as_mut() {
            primary_frame.frame_dirty = dirty;
        }
    }

    pub(super) fn primary_fps_enabled(&self) -> bool {
        self.primary_frame
            .as_ref()
            .map_or(self.primary_fps_enabled, |frame| frame.fps.enabled)
    }

    pub(super) fn primary_popup_menu(&self) -> Option<&PopupMenuState> {
        self.primary_frame
            .as_ref()
            .and_then(|frame| frame.popup_menu.as_ref())
    }

    pub(super) fn primary_popup_menu_mut(&mut self) -> Option<&mut PopupMenuState> {
        self.primary_frame
            .as_mut()
            .and_then(|frame| frame.popup_menu.as_mut())
    }

    pub(super) fn set_primary_popup_menu(&mut self, popup_menu: Option<PopupMenuState>) {
        if let Some(primary_frame) = self.primary_frame.as_mut() {
            primary_frame.popup_menu = popup_menu;
        } else {
            self.pending_primary_popup_menu = popup_menu;
        }
    }

    pub(super) fn set_primary_tooltip(&mut self, tooltip: Option<TooltipState>) {
        if let Some(primary_frame) = self.primary_frame.as_mut() {
            primary_frame.tooltip = tooltip;
        } else {
            self.pending_primary_tooltip = tooltip;
        }
    }

    pub(super) fn set_primary_visual_bell_start(&mut self, start: Option<Instant>) {
        if let Some(primary_frame) = self.primary_frame.as_mut() {
            primary_frame.visual_bell_start = start;
            primary_frame.frame_dirty = primary_frame.frame_dirty || start.is_some();
        } else {
            self.pending_primary_visual_bell_start = start;
        }
    }

    pub(super) fn primary_mouse_pos(&self) -> (f32, f32) {
        self.primary_frame
            .as_ref()
            .map_or((0.0, 0.0), |frame| frame.mouse_pos)
    }

    pub(super) fn set_primary_mouse_pos(&mut self, mouse_pos: (f32, f32)) {
        if let Some(primary_frame) = self.primary_frame.as_mut() {
            primary_frame.mouse_pos = mouse_pos;
        }
    }
}

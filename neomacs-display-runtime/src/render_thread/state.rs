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
use neomacs_display_protocol::{EffectsConfig, ToolBarImageSource, TransitionPolicy};
use neomacs_renderer_wgpu::{PopupMenuState, TooltipState, WgpuRenderer};

use super::child_frames::ChildFrameManager;
use super::cursor::CursorState;
use super::frame_windows::{
    FrameLifecycle, GuiFrameRenderState, GuiFrameWindowManager, GuiFrameWindowState,
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

pub(super) struct ChildFrameStyle {
    pub(super) corner_radius: f32,
    pub(super) shadow_enabled: bool,
    pub(super) shadow_layers: u32,
    pub(super) shadow_offset: f32,
    pub(super) shadow_opacity: f32,
}

impl Default for ChildFrameStyle {
    fn default() -> Self {
        Self {
            corner_radius: 8.0,
            shadow_enabled: true,
            shadow_layers: 4,
            shadow_offset: 2.0,
            shadow_opacity: 0.3,
        }
    }
}

pub(super) struct ToolbarResources {
    pub(super) icon_textures: HashMap<ToolBarImageSource, u32>,
    pub(super) icon_size: u32,
    pub(super) padding: u32,
}

impl Default for ToolbarResources {
    fn default() -> Self {
        Self {
            icon_textures: HashMap::new(),
            icon_size: 24,
            padding: 5,
        }
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

    pub(super) gpu: Option<RenderGpuContext>,
    pub(super) renderer: Option<WgpuRenderer>,

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

    pub(super) child_frame_style: ChildFrameStyle,
    pub(super) toolbar: ToolbarResources,

    pub(super) scroll_indicators_enabled: bool,

    pub(super) extra_line_spacing: f32,
    pub(super) extra_letter_spacing: f32,

    pub(super) shared_monitors: Option<SharedMonitorInfo>,
    pub(super) monitors_populated: bool,
    pub(super) last_monitor_snapshot: Vec<MonitorInfo>,
    pub(super) debug_first_frame_readback_pending: bool,
    pub(super) debug_surface_readback_frames_remaining: u32,
    pub(super) lifecycle_flags: RenderLifecycle,
}

pub(super) struct RenderLifecycle {
    pub resumed_seen: bool,
    pub about_to_wait_seen: bool,
    pub poll_when_idle: bool,
    pub shutdown_requested: bool,
}

impl RenderLifecycle {
    pub fn new(poll_when_idle: bool) -> Self {
        Self {
            resumed_seen: false,
            about_to_wait_seen: false,
            poll_when_idle,
            shutdown_requested: false,
        }
    }
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
            lifecycle: FrameLifecycle::Pending {
                width,
                height,
                scale_factor: 1.0,
                mouse_hidden_for_typing: false,
                ime_enabled: false,
                last_ime_cursor_area: None,
                chrome: WindowChrome {
                    title,
                    ..WindowChrome::default()
                },
                geometry_hints: None,
            },
            render: GuiFrameRenderState::new_without_device(0, false),
        });

        Self {
            comms,
            gpu: None,
            renderer: None,
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
            child_frame_style: ChildFrameStyle::default(),
            toolbar: ToolbarResources::default(),
            scroll_indicators_enabled: false,
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
            lifecycle_flags: RenderLifecycle::new(poll_when_idle),
        }
    }
}

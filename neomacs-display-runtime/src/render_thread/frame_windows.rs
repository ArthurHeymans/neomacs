//! GUI frame window management for the render thread.
//!
//! GNU Emacs treats every top-level GUI frame as a native window. This module
//! owns the render-thread mapping between Emacs frame IDs and winit windows so
//! redraw, input, resize, focus, and destruction can be frame-addressed instead
//! of primary-window-addressed.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use super::child_frames::ChildFrameManager;
use super::state::{
    GuiChromeInteractionState, effective_window_scale_factor, window_size_from_emacs_pixels,
};
use super::x11_hints::apply_window_geometry_hints;
use crate::core::frame_glyphs::FrameGlyphBuffer;
use neomacs_display_protocol::glyph_matrix::{
    GuiCompactBarState, GuiMenuBarState, GuiToolBarState,
};
#[cfg(feature = "wpe-webkit")]
use neomacs_display_protocol::scene::FloatingWebKit;
use neomacs_renderer_wgpu::{PopupMenuState, TooltipState, WgpuGlyphAtlas};
use neovm_core::window::GuiFrameGeometryHints;

/// Per-window state for a top-level GUI frame.
pub(crate) struct GuiFrameWindowState {
    /// The winit window.
    pub window: Arc<Window>,
    /// wgpu surface for this window.
    pub surface: wgpu::Surface<'static>,
    /// Surface configuration.
    pub surface_config: wgpu::SurfaceConfiguration,
    /// Physical width in pixels.
    pub width: u32,
    /// Physical height in pixels.
    pub height: u32,
    /// Display scale factor for this window's monitor.
    pub scale_factor: f64,
    /// The Emacs frame_id that owns this window (used for routing).
    pub emacs_frame_id: u64,
    /// Current root frame glyph buffer for this window.
    pub current_frame: Option<FrameGlyphBuffer>,
    /// Child frames rendered as overlays in this window.
    pub child_frames: ChildFrameManager,
    /// Glyph atlas rasterized at this window's current scale factor.
    pub glyph_atlas: WgpuGlyphAtlas,
    /// Whether this window needs a redraw.
    pub frame_dirty: bool,
    /// Last known pointer position in this frame's logical coordinates.
    pub mouse_pos: (f32, f32),
    /// Window title.
    pub title: String,
    /// GUI menu bar snapshot for this frame, if visible.
    pub menu_bar: Option<GuiMenuBarState>,
    /// GUI tool bar snapshot for this frame, if visible.
    pub tool_bar: Option<GuiToolBarState>,
    /// Compact GUI chrome snapshot for this frame, if visible.
    pub compact_bar: Option<GuiCompactBarState>,
    /// Hover/active/pressed state for GUI chrome in this frame window.
    pub chrome_interaction: GuiChromeInteractionState,
    /// Active popup menu shown in this frame window.
    pub popup_menu: Option<PopupMenuState>,
    /// Active tooltip shown in this frame window.
    pub tooltip: Option<TooltipState>,
    /// Visual bell flash start time for this frame window.
    pub visual_bell_start: Option<Instant>,
    /// Floating WebKit overlays rendered on this frame window.
    #[cfg(feature = "wpe-webkit")]
    pub floating_webkits: Vec<FloatingWebKit>,
}

impl GuiFrameWindowState {
    /// Resize this window's surface.
    pub fn handle_resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.width = width;
        self.height = height;
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(device, &self.surface_config);
        self.frame_dirty = true;
    }

    pub fn set_scale_factor(&mut self, scale_factor: f64) {
        let effective_scale = effective_window_scale_factor(scale_factor);
        self.scale_factor = effective_scale;
        self.glyph_atlas.set_scale_factor(effective_scale as f32);
        self.frame_dirty = true;
    }
}

/// Manages top-level GUI frame windows in the render thread.
///
/// Secondary windows live directly in `windows`. The adopted primary window is
/// still stored on `RenderApp` while this manager records its real Emacs frame
/// ID and native `WindowId`, so input can use GNU frame identity instead of the
/// historical render-thread sentinel `0`.
pub(crate) struct GuiFrameWindowManager {
    /// Emacs frame_id → native window-backed state for secondary top-level frames.
    pub windows: HashMap<u64, GuiFrameWindowState>,
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
        }
    }

    pub fn adopt_primary_frame_id(&mut self, emacs_frame_id: u64) {
        self.primary_emacs_frame_id = Some(emacs_frame_id);
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
            if self.windows.contains_key(&req.emacs_frame_id) {
                tracing::warn!("Window for frame {} already exists", req.emacs_frame_id);
                continue;
            }

            let attrs = Window::default_attributes()
                .with_title(&req.title)
                .with_inner_size(window_size_from_emacs_pixels(req.width, req.height))
                .with_transparent(true);

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
                    self.windows.insert(
                        req.emacs_frame_id,
                        GuiFrameWindowState {
                            window,
                            surface,
                            surface_config: config,
                            width: phys.width,
                            height: phys.height,
                            scale_factor,
                            emacs_frame_id: req.emacs_frame_id,
                            current_frame: None,
                            child_frames: ChildFrameManager::new(),
                            glyph_atlas: WgpuGlyphAtlas::new_with_scale(
                                device,
                                scale_factor as f32,
                            ),
                            frame_dirty: false,
                            mouse_pos: (0.0, 0.0),
                            title: req.title,
                            menu_bar: None,
                            tool_bar: None,
                            compact_bar: None,
                            chrome_interaction: GuiChromeInteractionState::default(),
                            popup_menu: None,
                            tooltip: None,
                            visual_bell_start: None,
                            #[cfg(feature = "wpe-webkit")]
                            floating_webkits: Vec::new(),
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
            if let Some(state) = self.windows.remove(&frame_id) {
                self.winit_to_emacs.remove(&state.window.id());
                tracing::info!("Destroyed window for frame {}", frame_id);
                // Window and surface are dropped here
            }
        }
    }

    /// Drop all windows and their wgpu surfaces (for clean shutdown).
    pub fn destroy_all(&mut self) {
        self.pending_creates.clear();
        self.pending_destroys.clear();
        self.winit_to_emacs.clear();
        self.primary_winit_id = None;
        // Drop all window states (surfaces, etc.)
        self.windows.clear();
    }

    /// Look up the Emacs frame_id for a winit WindowId.
    pub fn emacs_frame_for_winit(&self, winit_id: WindowId) -> Option<u64> {
        self.winit_to_emacs.get(&winit_id).copied()
    }

    /// Get a window state by Emacs frame_id.
    pub fn get(&self, emacs_frame_id: u64) -> Option<&GuiFrameWindowState> {
        self.windows.get(&emacs_frame_id)
    }

    /// Get a mutable window state by Emacs frame_id.
    pub fn get_mut(&mut self, emacs_frame_id: u64) -> Option<&mut GuiFrameWindowState> {
        self.windows.get_mut(&emacs_frame_id)
    }

    /// Get a window state by winit WindowId.
    pub fn get_by_winit(&self, winit_id: WindowId) -> Option<&GuiFrameWindowState> {
        self.winit_to_emacs
            .get(&winit_id)
            .and_then(|id| self.windows.get(id))
    }

    /// Get a mutable window state by winit WindowId.
    pub fn get_by_winit_mut(&mut self, winit_id: WindowId) -> Option<&mut GuiFrameWindowState> {
        self.winit_to_emacs
            .get(&winit_id)
            .copied()
            .and_then(move |id| self.windows.get_mut(&id))
    }

    /// Route a FrameGlyphBuffer to the appropriate window.
    /// Returns true if the frame was routed to a secondary window.
    pub fn route_frame(
        &mut self,
        frame: FrameGlyphBuffer,
        menu_bar: Option<GuiMenuBarState>,
        tool_bar: Option<GuiToolBarState>,
        compact_bar: Option<GuiCompactBarState>,
    ) -> bool {
        let frame_id = frame.frame_id;
        if frame_id != 0 {
            if frame.parent_id != 0 {
                // Child frame: route to the window that owns the parent
                // Find which window has the parent as its root frame
                for (_, ws) in self.windows.iter_mut() {
                    if ws.emacs_frame_id == frame.parent_id {
                        ws.child_frames.update_frame(frame);
                        ws.frame_dirty = true;
                        return true;
                    }
                }
            } else if let Some(ws) = self.windows.get_mut(&frame_id) {
                // Root frame for a secondary window
                if menu_bar.is_none() {
                    ws.chrome_interaction.clear_menu_bar();
                }
                if tool_bar.is_none() {
                    ws.chrome_interaction.clear_toolbar();
                }
                if compact_bar.is_none() {
                    ws.chrome_interaction.clear_compact_bar();
                }
                if frame.tab_bar.is_none() {
                    ws.chrome_interaction.clear_tab_bar();
                }
                ws.menu_bar = menu_bar;
                ws.tool_bar = tool_bar;
                ws.compact_bar = compact_bar;
                ws.current_frame = Some(frame);
                ws.frame_dirty = true;
                return true;
            }
        }
        false // Not handled — belongs to primary window
    }

    /// Check if any secondary window needs redrawing.
    pub fn any_dirty(&self) -> bool {
        self.windows.values().any(|ws| ws.frame_dirty)
    }

    /// Return number of secondary windows.
    pub fn count(&self) -> usize {
        self.windows.len()
    }

    /// Iterate over all windows that need rendering.
    pub fn dirty_windows(&mut self) -> Vec<u64> {
        self.windows
            .iter()
            .filter(|(_, ws)| ws.frame_dirty)
            .map(|(&id, _)| id)
            .collect()
    }
}

#[cfg(test)]
#[path = "frame_windows_test.rs"]
mod tests;

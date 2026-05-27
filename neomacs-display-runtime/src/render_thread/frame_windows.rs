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
use super::cursor::{CursorState, CursorTarget};
use super::state::{
    FpsCounter, GuiChromeInteractionState, ImeCursorArea, TypingSpeedState, WindowChrome,
    effective_window_scale_factor, window_size_from_emacs_pixels,
};
use super::transitions::{TransitionState, clear_frame_transition_textures};
use super::x11_hints::apply_window_geometry_hints;
use crate::core::frame_glyphs::FrameGlyphBuffer;
use neomacs_display_protocol::glyph_matrix::{
    GuiCompactBarState, GuiMenuBarState, GuiToolBarState,
};
#[cfg(feature = "wpe-webkit")]
use neomacs_display_protocol::scene::FloatingWebKit;
use neomacs_renderer_wgpu::{PopupMenuState, RendererFrameEffects, TooltipState, WgpuGlyphAtlas};
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
    /// Text cursor animation and blink state for this frame window.
    pub(super) cursor: CursorState,
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
    /// FPS overlay timing owned by this frame window.
    pub(super) fps: FpsCounter,
    /// Typing-speed overlay state owned by this frame window.
    pub(super) typing_speed: TypingSpeedState,
    /// Whether an IME preedit overlay is active in this frame window.
    pub ime_preedit_active: bool,
    /// Current IME preedit text for this frame window.
    pub ime_preedit_text: String,
    /// Window transition state owned by this frame window.
    pub transitions: TransitionState,
    /// Renderer runtime effects owned by this frame window.
    pub renderer_effects: RendererFrameEffects,
    /// Floating WebKit overlays rendered on this frame window.
    #[cfg(feature = "wpe-webkit")]
    pub floating_webkits: Vec<FloatingWebKit>,
}

/// Per-window state for a top-level GUI frame.
pub(crate) struct GuiFrameWindowState {
    pub native: GuiFrameNativeWindowState,
    pub render: GuiFrameRenderState,
}

impl GuiFrameRenderState {
    pub(super) fn current_frame_clone(&self) -> Option<FrameGlyphBuffer> {
        self.current_frame.clone()
    }

    pub(super) fn take_current_frame_for_render(&mut self) -> Option<FrameGlyphBuffer> {
        self.current_frame.as_mut().map(Self::take_frame_for_render)
    }

    pub(super) fn take_frame_for_render(current_frame: &mut FrameGlyphBuffer) -> FrameGlyphBuffer {
        let (transition_hints, effect_hints) = current_frame.take_runtime_hints();
        let mut frame = current_frame.clone();
        frame.transition_hints = transition_hints;
        frame.effect_hints = effect_hints;
        frame
    }
}

impl GuiFrameWindowState {
    /// Resize this window's surface.
    pub fn handle_resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.native.width = width;
        self.native.height = height;
        self.native.surface_config.width = width;
        self.native.surface_config.height = height;
        self.native
            .surface
            .configure(device, &self.native.surface_config);
        clear_frame_transition_textures(&mut self.render.transitions);
        self.render.frame_dirty = true;
    }

    pub fn set_scale_factor(&mut self, scale_factor: f64) {
        let effective_scale = effective_window_scale_factor(scale_factor);
        self.native.scale_factor = effective_scale;
        self.render
            .glyph_atlas
            .set_scale_factor(effective_scale as f32);
        self.render.frame_dirty = true;
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
                    self.windows.insert(
                        req.emacs_frame_id,
                        GuiFrameWindowState {
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
                                chrome: WindowChrome {
                                    title: req.title.clone(),
                                    titlebar_hover: 0,
                                    resize_edge: None,
                                    last_titlebar_click: Instant::now(),
                                    ..self.chrome_defaults.clone()
                                },
                            },
                            render: GuiFrameRenderState {
                                emacs_frame_id: req.emacs_frame_id,
                                current_frame: None,
                                child_frames: ChildFrameManager::new(),
                                glyph_atlas: WgpuGlyphAtlas::new_with_scale(
                                    device,
                                    scale_factor as f32,
                                ),
                                frame_dirty: false,
                                mouse_pos: (0.0, 0.0),
                                cursor: CursorState::default(),
                                menu_bar: None,
                                tool_bar: None,
                                compact_bar: None,
                                chrome_interaction: GuiChromeInteractionState::default(),
                                popup_menu: None,
                                tooltip: None,
                                visual_bell_start: None,
                                fps: FpsCounter {
                                    enabled: self.fps_enabled,
                                    ..FpsCounter::default()
                                },
                                typing_speed: TypingSpeedState::default(),
                                ime_preedit_active: false,
                                ime_preedit_text: String::new(),
                                transitions: TransitionState::default(),
                                renderer_effects: RendererFrameEffects::default(),
                                #[cfg(feature = "wpe-webkit")]
                                floating_webkits: Vec::new(),
                            },
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
                self.winit_to_emacs.remove(&state.native.window.id());
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
                    if ws.render.emacs_frame_id == frame.parent_id {
                        ws.render.child_frames.update_frame(frame);
                        ws.render.frame_dirty = true;
                        return true;
                    }
                }
            } else if let Some(ws) = self.windows.get_mut(&frame_id) {
                // Root frame for a secondary window
                if menu_bar.is_none() {
                    ws.render.chrome_interaction.clear_menu_bar();
                }
                if tool_bar.is_none() {
                    ws.render.chrome_interaction.clear_toolbar();
                }
                if compact_bar.is_none() {
                    ws.render.chrome_interaction.clear_compact_bar();
                }
                if frame.tab_bar.is_none() {
                    ws.render.chrome_interaction.clear_tab_bar();
                }
                ws.render.menu_bar = menu_bar;
                ws.render.tool_bar = tool_bar;
                ws.render.compact_bar = compact_bar;
                ws.render.current_frame = Some(frame);
                ws.render.frame_dirty = true;
                return true;
            }
        }
        false // Not handled — belongs to primary window
    }

    /// Check if any secondary window needs redrawing.
    pub fn any_dirty(&self) -> bool {
        self.windows.values().any(|ws| ws.render.frame_dirty)
    }

    /// Return number of secondary windows.
    pub fn count(&self) -> usize {
        self.windows.len()
    }

    /// Iterate over all windows that need rendering.
    pub fn dirty_windows(&mut self) -> Vec<u64> {
        self.windows
            .iter()
            .filter(|(_, ws)| ws.render.frame_dirty)
            .map(|(&id, _)| id)
            .collect()
    }
}

#[cfg(test)]
#[path = "frame_windows_test.rs"]
mod tests;

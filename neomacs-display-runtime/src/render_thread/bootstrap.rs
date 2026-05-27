use super::{
    RenderApp, RenderUserEvent, SharedImageDimensions, SharedMonitorInfo, surface_readback,
};
use crate::render_thread::frame_windows::{GuiFrameNativeWindowState, GuiFrameRenderState};
use crate::render_thread::state::RenderGpuContext;
use crate::thread_comm::{InputEvent, RenderComms};
use neomacs_renderer_wgpu::WgpuRenderer;
use std::sync::Arc;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
#[cfg(target_os = "linux")]
use winit::platform::wayland::EventLoopBuilderExtWayland;
#[cfg(target_os = "linux")]
use winit::platform::x11::EventLoopBuilderExtX11;
use winit::window::Window;
#[cfg(target_os = "linux")]
use x11_dl::xlib;

#[cfg(feature = "wpe-webkit")]
use crate::backend::wpe::WpeBackend;
#[cfg(all(feature = "wpe-webkit", wpe_platform_available))]
use crate::backend::wpe::sys::platform as plat;

impl RenderApp {
    /// Initialize wgpu with the window
    pub(super) fn init_wgpu(&mut self, event_loop: &ActiveEventLoop, window: Arc<Window>) {
        tracing::info!("Initializing wgpu for render thread");

        // GLES presentation needs the compositor/display handle, especially on Wayland.
        let instance_descriptor =
            crate::wgpu_instance_descriptor_with_display(event_loop.owned_display_handle());
        tracing::info!(
            "wgpu requested backends: {:?}",
            instance_descriptor.backends
        );
        let instance = wgpu::Instance::new(instance_descriptor);

        // Create surface from window
        let surface = match instance.create_surface(window.clone()) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to create wgpu surface: {:?}", e);
                return;
            }
        };

        // Request adapter
        let adapter =
            match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: crate::gpu_power_preference(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })) {
                Ok(a) => a,
                Err(e) => {
                    tracing::error!("Failed to find suitable GPU adapter: {:?}", e);
                    return;
                }
            };

        let adapter_info = adapter.get_info();
        tracing::info!(
            "wgpu adapter: {} (vendor={:04x}, device={:04x}, backend={:?})",
            adapter_info.name,
            adapter_info.vendor,
            adapter_info.device,
            adapter_info.backend
        );

        // Request device and queue
        let (device, queue) =
            match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("Neomacs Render Thread Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                trace: wgpu::Trace::Off,
            })) {
                Ok((d, q)) => (d, q),
                Err(e) => {
                    tracing::error!("Failed to create wgpu device: {:?}", e);
                    return;
                }
            };

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        // Configure surface
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        // Prefer PreMultiplied alpha for window transparency support
        let alpha_mode = if caps
            .alpha_modes
            .contains(&wgpu::CompositeAlphaMode::PreMultiplied)
        {
            wgpu::CompositeAlphaMode::PreMultiplied
        } else {
            caps.alpha_modes[0]
        };
        let surface_usage = surface_readback::surface_usage_for_debug_readback(
            caps.usages,
            &mut self.debug_first_frame_readback_pending,
            self.debug_surface_readback_frames_remaining > 0,
        );
        let config = wgpu::SurfaceConfiguration {
            usage: surface_usage,
            format,
            width: self.width,
            height: self.height,
            present_mode: wgpu::PresentMode::Fifo, // VSync
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Create renderer with existing device and surface format
        let renderer = WgpuRenderer::with_device(
            device.clone(),
            queue.clone(),
            self.width,
            self.height,
            format,
            self.scale_factor as f32,
        );

        // Create adopted-primary frame render state with its own glyph atlas.
        let primary_frame_id = self.frame_windows.primary_frame_id().unwrap_or(0);
        let mut primary_frame = GuiFrameRenderState::new(
            primary_frame_id,
            &device,
            self.scale_factor,
            self.primary_fps_enabled(),
        );
        primary_frame.popup_menu = self.pending_primary_popup_menu.take();
        primary_frame.tooltip = self.pending_primary_tooltip.take();
        primary_frame.visual_bell_start = self.pending_primary_visual_bell_start.take();
        if primary_frame.popup_menu.is_some()
            || primary_frame.tooltip.is_some()
            || primary_frame.visual_bell_start.is_some()
        {
            primary_frame.frame_dirty = true;
        }
        primary_frame.cursor.copy_config_from(&self.cursor_defaults);
        primary_frame.transitions.policy = self.transition_policy;
        primary_frame.child_frames = std::mem::replace(
            &mut self.pending_primary_child_frames,
            crate::render_thread::child_frames::ChildFrameManager::new(),
        );
        #[cfg(feature = "wpe-webkit")]
        {
            primary_frame.floating_webkits =
                std::mem::take(&mut self.pending_primary_floating_webkits);
            if !primary_frame.floating_webkits.is_empty() {
                primary_frame.frame_dirty = true;
            }
        }
        primary_frame.menu_bar = self.pending_primary_menu_bar.take();
        primary_frame.tool_bar = self.pending_primary_tool_bar.take();
        primary_frame.compact_bar = self.pending_primary_compact_bar.take();
        if primary_frame.menu_bar.is_some()
            || primary_frame.tool_bar.is_some()
            || primary_frame.compact_bar.is_some()
        {
            primary_frame.frame_dirty = true;
        }

        tracing::info!(
            "wgpu initialized: {}x{}, format: {:?}",
            self.width,
            self.height,
            format
        );

        self.gpu = Some(RenderGpuContext {
            instance,
            adapter,
            device: device.clone(),
            queue: queue.clone(),
        });
        self.renderer = Some(renderer);
        self.primary_native = Some(GuiFrameNativeWindowState {
            window,
            surface,
            surface_config: config,
            width: self.width,
            height: self.height,
            scale_factor: self.scale_factor,
            mouse_hidden_for_typing: self.mouse_hidden_for_typing,
            ime_enabled: self.ime_enabled,
            last_ime_cursor_area: self.last_ime_cursor_area,
            chrome: self.chrome.clone(),
        });
        self.primary_frame = Some(primary_frame);
        let pending_tool_items = self
            .primary_frame
            .as_ref()
            .and_then(|frame| frame.tool_bar.as_ref())
            .map(|tool_bar| tool_bar.items.clone());
        let pending_compact_tool_items = self
            .primary_frame
            .as_ref()
            .and_then(|frame| frame.compact_bar.as_ref())
            .map(|compact_bar| compact_bar.tool_items.clone());
        if let Some(items) = pending_tool_items.as_ref() {
            self.ensure_toolbar_icon_textures(items);
        }
        if let Some(items) = pending_compact_tool_items.as_ref() {
            self.ensure_toolbar_icon_textures(items);
        }

        // Initialize WPE backend for WebKit
        #[cfg(feature = "wpe-webkit")]
        {
            use crate::backend::wgpu::get_render_node_from_adapter_info;

            // Get DRM render node from adapter to ensure WebKit uses the same GPU
            let render_node = get_render_node_from_adapter_info(&adapter_info)
                .map(|p| p.to_string_lossy().into_owned());

            tracing::info!("Initializing WPE backend (render_node: {:?})", render_node);

            // SAFETY: We pass null for egl_display_hint as WPE Platform API doesn't use it
            match unsafe {
                WpeBackend::new_with_device(std::ptr::null_mut(), render_node.as_deref())
            } {
                Ok(backend) => {
                    tracing::info!("WPE backend initialized successfully");
                    self.wpe_backend = Some(backend);
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize WPE backend: {:?}", e);
                }
            }
        }

        // All GPU caches (image, video, webkit) are managed by the renderer
        #[cfg(feature = "video")]
        tracing::info!("Video cache initialized");
    }

    /// Handle surface resize
    pub(super) fn handle_resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        self.width = width;
        self.height = height;

        // Reconfigure surface
        self.configure_primary_surface(width, height);

        // Resize renderer
        if let Some(renderer) = &mut self.renderer {
            renderer.resize(width, height);
        }

        if let Some(primary_frame) = self.primary_frame.as_mut() {
            // Invalidate offscreen textures and active transitions for the old size.
            primary_frame.transitions.offscreen_a = None;
            primary_frame.transitions.offscreen_b = None;
            primary_frame.transitions.crossfades.clear();
            primary_frame.transitions.scroll_slides.clear();
        }

        // Trigger resize padding transition
        if self.effects.resize_padding.enabled {
            if let (Some(renderer), Some(primary_frame)) =
                (self.renderer.as_ref(), self.primary_frame.as_mut())
            {
                renderer.trigger_transient_resize_padding(
                    &mut primary_frame.renderer_effects,
                    std::time::Instant::now(),
                );
            }
        }

        // Force immediate re-render with old frame at new surface size.
        // Ensures the window always shows content during resize
        // (background fills new area, old glyphs stay at their positions).
        self.mark_primary_dirty();

        tracing::debug!("Surface resized to {}x{}", width, height);
    }
}

fn build_render_event_loop_impl(
    allow_any_thread: bool,
) -> Result<EventLoop<RenderUserEvent>, String> {
    #[cfg(target_os = "linux")]
    {
        validate_linux_display_before_winit()?;
        tracing::info!(
            "Building winit event loop (allow_any_thread={} wayland_display_present={})",
            allow_any_thread,
            std::env::var("WAYLAND_DISPLAY").is_ok(),
        );
        let mut builder = EventLoop::<RenderUserEvent>::with_user_event();
        // Try Wayland first, fall back to X11.
        if allow_any_thread {
            if std::env::var("WAYLAND_DISPLAY").is_ok() {
                EventLoopBuilderExtWayland::with_any_thread(&mut builder, true);
            } else {
                EventLoopBuilderExtX11::with_any_thread(&mut builder, true);
            }
        }
        let event_loop = builder
            .build()
            .map_err(|err| format!("Failed to create event loop: {err}"))?;
        tracing::info!("Built winit event loop");
        Ok(event_loop)
    }

    #[cfg(not(target_os = "linux"))]
    {
        EventLoop::<RenderUserEvent>::with_user_event()
            .build()
            .map_err(|err| format!("Failed to create event loop: {err}"))
    }
}

#[cfg(target_os = "linux")]
fn validate_linux_display_before_winit() -> Result<(), String> {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        return Ok(());
    }
    let Some(display) = std::env::var_os("DISPLAY") else {
        return Ok(());
    };
    if display.is_empty() {
        return Ok(());
    }

    let display_for_error = display.to_string_lossy().into_owned();
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("x11-display-probe".to_string())
        .spawn(move || {
            let result = x11_display_responds();
            let _ = tx.send(result);
        });

    match rx.recv_timeout(std::time::Duration::from_secs(1)) {
        Ok(true) => Ok(()),
        Ok(false) => Err(format!("Cannot open X display {display_for_error}")),
        Err(_) => Err(format!(
            "Cannot open X display {display_for_error}: connection timed out"
        )),
    }
}

#[cfg(target_os = "linux")]
fn x11_display_responds() -> bool {
    let Ok(xlib) = xlib::Xlib::open() else {
        return false;
    };
    let display = unsafe { (xlib.XOpenDisplay)(std::ptr::null()) };
    if display.is_null() {
        return false;
    }
    unsafe {
        (xlib.XCloseDisplay)(display);
    }
    true
}

/// Build a render event loop for the current OS thread.
pub fn build_render_event_loop() -> Result<EventLoop<RenderUserEvent>, String> {
    build_render_event_loop_impl(false)
}

/// Build a render event loop for the legacy render-thread helper.
pub(crate) fn build_render_event_loop_any_thread() -> Result<EventLoop<RenderUserEvent>, String> {
    build_render_event_loop_impl(true)
}

/// Run the render loop with an already-created event loop.
pub(crate) fn run_render_loop_with_event_loop(
    event_loop: EventLoop<RenderUserEvent>,
    comms: RenderComms,
    width: u32,
    height: u32,
    title: String,
    image_dimensions: SharedImageDimensions,
    shared_monitors: SharedMonitorInfo,
    poll_when_idle: bool,
    #[cfg(feature = "neo-term")] shared_terminals: crate::terminal::SharedTerminals,
) -> Result<(), String> {
    tracing::info!("Render thread starting");

    // CRITICAL: Set up a dedicated GMainContext for WebKit before any WebKit initialization.
    // This ensures WebKit attaches its GLib sources (IPC sockets, etc.) to this context,
    // not the default context. Only the render thread will dispatch events from this context,
    // preventing the Emacs main thread's xg_select from dispatching WebKit callbacks.
    #[cfg(all(feature = "wpe-webkit", wpe_platform_available))]
    let webkit_main_context = unsafe {
        let ctx = plat::g_main_context_new();
        if !ctx.is_null() {
            // Acquire the context so we can dispatch on it
            plat::g_main_context_acquire(ctx);
            // Push as thread-default - WebKit will attach sources here
            plat::g_main_context_push_thread_default(ctx);
            tracing::info!("Created dedicated GMainContext for WebKit: {:?}", ctx);
        } else {
            tracing::warn!("Failed to create dedicated GMainContext for WebKit");
        }
        ctx
    };

    // Start with WaitUntil to avoid busy-polling; about_to_wait() adjusts dynamically
    event_loop.set_control_flow(ControlFlow::WaitUntil(
        std::time::Instant::now() + std::time::Duration::from_millis(16),
    ));

    let mut app = RenderApp::new(
        comms,
        width,
        height,
        title,
        image_dimensions,
        shared_monitors,
        poll_when_idle,
        #[cfg(feature = "neo-term")]
        shared_terminals,
    );

    tracing::info!("Render thread entering winit event loop");
    let result = event_loop.run_app(&mut app);
    if let Err(ref e) = result {
        tracing::error!("Event loop error: {:?}", e);
    }

    // Notify Emacs that the render thread is exiting so it can shut down gracefully.
    // This handles cases like Wayland connection loss (ExitFailure(1)) where the
    // window disappears without an explicit close request.
    tracing::info!("Render thread exiting, sending WindowClose to Emacs");
    app.comms
        .send_input(InputEvent::WindowClose { emacs_frame_id: 0 });

    result.map_err(|err| format!("Event loop error: {err}"))
}

/// Run the render loop on the current OS thread. Product GUI startup uses
/// this path so winit/AppKit/Windows ownership stays on the process main
/// thread; evaluator-to-render traffic must wake it via EventLoopProxy.
pub fn run_render_loop_current_thread(
    event_loop: EventLoop<RenderUserEvent>,
    comms: RenderComms,
    width: u32,
    height: u32,
    title: String,
    image_dimensions: SharedImageDimensions,
    shared_monitors: SharedMonitorInfo,
) -> Result<(), String> {
    #[cfg(feature = "neo-term")]
    let shared_terminals =
        std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
    run_render_loop_with_event_loop(
        event_loop,
        comms,
        width,
        height,
        title,
        image_dimensions,
        shared_monitors,
        false,
        #[cfg(feature = "neo-term")]
        shared_terminals,
    )
}

/// Build the render event loop and run it on the render thread.
pub fn run_render_loop(
    comms: RenderComms,
    width: u32,
    height: u32,
    title: String,
    image_dimensions: SharedImageDimensions,
    shared_monitors: SharedMonitorInfo,
) -> Result<(), String> {
    #[cfg(feature = "neo-term")]
    let shared_terminals =
        std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
    let event_loop = build_render_event_loop()?;
    run_render_loop_with_event_loop(
        event_loop,
        comms,
        width,
        height,
        title,
        image_dimensions,
        shared_monitors,
        false,
        #[cfg(feature = "neo-term")]
        shared_terminals,
    )
}

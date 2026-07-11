use super::{RenderApp, RenderUserEvent, SharedImageMetadata, SharedMonitorInfo, surface_readback};
use crate::render_thread::frame_windows::{FrameLifecycle, GuiFrameNativeWindowState};
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
    pub(super) fn init_wgpu(&mut self, event_loop: &ActiveEventLoop, window: Arc<Window>) {
        tracing::info!("Initializing wgpu for render thread");

        let instance_descriptor =
            crate::wgpu_instance_descriptor_with_display(event_loop.owned_display_handle());
        tracing::info!(
            "wgpu requested backends: {:?}",
            instance_descriptor.backends
        );
        let instance = wgpu::Instance::new(instance_descriptor);

        let surface = match instance.create_surface(window.clone()) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to create wgpu surface: {:?}", e);
                return;
            }
        };

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

        let phys = window.inner_size();
        let raw_scale_factor = window.scale_factor();
        let effective_scale = super::state::effective_window_scale_factor(raw_scale_factor);

        let caps = surface.get_capabilities(&adapter);
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
        let surface_usage = surface_readback::surface_usage_for_debug_readback(
            caps.usages,
            &mut self.debug_first_frame_readback_pending,
            self.debug_surface_readback_frames_remaining > 0,
        );

        {
            let primary = self.frame_windows.primary_window_mut().unwrap();
            if let FrameLifecycle::Pending {
                width: pw,
                height: ph,
                scale_factor: sf,
                ..
            } = &mut primary.lifecycle
            {
                *pw = phys.width;
                *ph = phys.height;
                *sf = effective_scale;
            }
        }

        let (pending_width, pending_height) = self
            .frame_windows
            .primary_window()
            .unwrap()
            .lifecycle
            .native_size();
        let pending_scale_factor = self
            .frame_windows
            .primary_window()
            .unwrap()
            .lifecycle
            .scale_factor();

        let config = wgpu::SurfaceConfiguration {
            usage: surface_usage,
            format,
            width: pending_width,
            height: pending_height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let renderer = WgpuRenderer::with_device(
            device.clone(),
            queue.clone(),
            pending_width,
            pending_height,
            format,
            pending_scale_factor as f32,
        );

        tracing::info!(
            "wgpu initialized: {}x{}, format: {:?}",
            pending_width,
            pending_height,
            format
        );

        self.gpu = Some(RenderGpuContext {
            instance,
            adapter,
            device: device.clone(),
            queue: queue.clone(),
        });
        self.renderer = Some(renderer);

        self.frame_windows
            .populate_primary_native(GuiFrameNativeWindowState {
                window,
                surface,
                surface_config: config,
                width: pending_width,
                height: pending_height,
                scale_factor: pending_scale_factor,
                chrome: {
                    let primary = self.frame_windows.primary_window().unwrap();
                    primary.lifecycle.chrome().clone()
                },
            });

        {
            let primary = self.frame_windows.primary_window_mut().unwrap();
            primary
                .render
                .populate_glyph_atlas(&device, pending_scale_factor);
            primary
                .render
                .cursor
                .apply_config(self.cursor_defaults.config_snapshot());
            primary.render.compositor.transitions.policy = self.transition_policy;
        }

        let pending_frame_chrome = self
            .frame_windows
            .primary_window()
            .and_then(|ws| ws.render.compositor.current_frame.as_ref())
            .map(|frame| frame.frame_chrome.clone());
        if let Some(frame_chrome) = pending_frame_chrome.as_ref() {
            self.sync_frame_chrome_assets(frame_chrome);
        }

        #[cfg(feature = "wpe-webkit")]
        {
            use crate::backend::wgpu::get_render_node_from_adapter_info;

            let render_node = get_render_node_from_adapter_info(&adapter_info)
                .map(|p| p.to_string_lossy().into_owned());

            tracing::info!("Initializing WPE backend (render_node: {:?})", render_node);

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

        #[cfg(feature = "video")]
        tracing::info!("Video cache initialized");
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
    image_metadata: SharedImageMetadata,
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
    let _webkit_main_context = unsafe {
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
        image_metadata,
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
    image_metadata: SharedImageMetadata,
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
        image_metadata,
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
    image_metadata: SharedImageMetadata,
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
        image_metadata,
        shared_monitors,
        false,
        #[cfg(feature = "neo-term")]
        shared_terminals,
    )
}

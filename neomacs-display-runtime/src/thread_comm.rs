//! Thread communication infrastructure for two-thread architecture.
//!
//! Provides lock-free channels and wakeup mechanism between Emacs and render threads.

use crossbeam_channel::{Receiver, Sender, TrySendError, bounded, unbounded};
#[cfg(unix)]
use std::os::fd::{AsRawFd, OwnedFd, RawFd};
#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, OwnedHandle, RawHandle};
use std::time::Instant;

/// Platform file descriptor type for the wakeup pipe.
#[cfg(unix)]
pub type WakeupFd = RawFd;
#[cfg(windows)]
pub type WakeupFd = RawHandle;

use neomacs_display_protocol::ImageRealization;
use neomacs_display_protocol::SealedFramePresentation;
pub use neomacs_display_protocol::{
    CursorEffectCommand, EffectsConfig, MenuBarItem, PopupMenuItem, TabBarItem, ToolBarImageSource,
    ToolBarItem, ToolBarItemType, TransitionPolicy,
};
use neovm_core::window::GuiFrameGeometryHints;

/// Native selection owned by the display server.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ClipboardSelection {
    Clipboard,
    Primary,
}

/// Monitor information transported from the frontend to the evaluator.
#[derive(Debug, Clone, PartialEq)]
pub struct MonitorInfo {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub scale: f64,
    pub width_mm: i32,
    pub height_mm: i32,
    pub name: Option<String>,
}

/// Input event from render thread to Emacs
#[derive(Debug, Clone)]
pub enum InputEvent {
    Key {
        keysym: u32,
        modifiers: u32,
        pressed: bool,
        /// Emacs frame_id of the window that produced the key event
        emacs_frame_id: u64,
    },
    MouseButton {
        button: u32,
        x: f32,
        y: f32,
        pressed: bool,
        modifiers: u32,
        /// Emacs frame_id targeted by hit testing (root frame or child frame)
        target_frame_id: u64,
        /// WebKit view ID hit by render-thread glyph search (0 = none)
        webkit_id: u32,
        /// Coordinates relative to the WebKit view (valid when webkit_id != 0)
        webkit_rel_x: i32,
        webkit_rel_y: i32,
    },
    MouseMove {
        x: f32,
        y: f32,
        modifiers: u32,
        /// Emacs frame_id targeted by hit testing (root frame or child frame)
        target_frame_id: u64,
    },
    /// Semantic pointer observation resolved against the displayed presentation.
    PresentedRegion {
        presentation: u64,
        hit: Option<neomacs_display_protocol::PresentedHit>,
        x: f32,
        y: f32,
        target_frame_id: u64,
    },
    MouseScroll {
        delta_x: f32,
        delta_y: f32,
        x: f32,
        y: f32,
        modifiers: u32,
        /// True if deltas are in pixels (touchpad), false if in lines (mouse wheel)
        pixel_precise: bool,
        /// Emacs frame_id targeted by hit testing (root frame or child frame)
        target_frame_id: u64,
        /// WebKit view ID hit by render-thread glyph search (0 = none)
        webkit_id: u32,
        /// Coordinates relative to the WebKit view (valid when webkit_id != 0)
        webkit_rel_x: i32,
        webkit_rel_y: i32,
    },
    WindowResize {
        width: u32,
        height: u32,
        /// Physical device pixels per logical Emacs pixel.
        scale_factor: f64,
        /// Emacs frame_id of the window that resized
        emacs_frame_id: u64,
    },
    WindowClose {
        /// Emacs frame_id of the window being closed
        emacs_frame_id: u64,
    },
    WindowFocus {
        focused: bool,
        /// Emacs frame_id of the window that gained/lost focus
        emacs_frame_id: u64,
    },
    /// Monitor configuration changed on the active terminal.
    MonitorsChanged { monitors: Vec<MonitorInfo> },
    /// WebKit view title changed
    #[cfg(feature = "wpe-webkit")]
    WebKitTitleChanged { id: u32, title: String },
    /// WebKit view URL changed
    #[cfg(feature = "wpe-webkit")]
    WebKitUrlChanged { id: u32, url: String },
    /// WebKit view load progress changed
    #[cfg(feature = "wpe-webkit")]
    WebKitProgressChanged { id: u32, progress: f64 },
    /// WebKit view finished loading
    #[cfg(feature = "wpe-webkit")]
    WebKitLoadFinished { id: u32 },
    /// Image decoding reached a terminal state (ready or failed).
    ImageStateChanged { id: u32 },
    /// Terminal child process exited
    #[cfg(feature = "neo-term")]
    TerminalExited { id: u32 },
    /// Terminal title changed
    #[cfg(feature = "neo-term")]
    TerminalTitleChanged { id: u32, title: String },
    /// Popup menu selection made (index into menu items, -1 = cancelled)
    MenuSelection { index: i32 },
    /// File(s) dropped onto the window
    FileDrop { paths: Vec<String>, x: f32, y: f32 },
    /// Toolbar button clicked (index into toolbar items)
    ToolBarClick { index: i32, emacs_frame_id: u64 },
    /// Pointer observation resolved against an immutable displayed presentation.
    PresentedPointer {
        presentation: u64,
        interaction: u32,
        pressed: bool,
        button: u8,
        x: f32,
        y: f32,
        emacs_frame_id: u64,
    },
    /// Renderer installed this presentation as its drawing and hit-test source.
    PresentationActivated {
        presentation: u64,
        emacs_frame_id: u64,
    },
    /// Renderer rejected or superseded this presentation before activation.
    PresentationDiscarded {
        presentation: u64,
        emacs_frame_id: u64,
    },
    /// Renderer no longer displays or generates hits for this presentation.
    PresentationRetired { presentation: u64 },
    /// Menu bar item clicked. `menu_x` is the Emacs menu-bar column used by
    /// legacy Lisp paths; `key` is the exact rendered top-level menu key; and
    /// `anchor` is the frame-local logical-pixel rectangle used by the native
    /// popup renderer.
    MenuBarClick {
        index: i32,
        key: String,
        menu_x: f32,
        anchor: PopupAnchorRect,
        emacs_frame_id: u64,
    },
}

pub type PopupAnchorRect = neomacs_display_protocol::Rect;

/// Wrapper for effect update closures that implements Debug.
pub struct EffectUpdater(pub Box<dyn FnOnce(&mut EffectsConfig) + Send>);

impl std::fmt::Debug for EffectUpdater {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EffectUpdater(...)")
    }
}

/// Frame reference in commands flowing from Emacs to the render thread.
///
/// Replaces raw `u64` `emacs_frame_id` — no sentinel values.
/// Matches GNU Emacs convention: 0 is never a valid frame ID
/// (`frame_next_id = 1` in GNU Emacs `frame.c:343`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrameRef {
    /// Route to the primary frame (resolved at render-time).
    Primary,
    /// Route to a specific frame by its Emacs-assigned ID.
    Frame(u64),
}

impl FrameRef {
    pub fn raw_id(&self) -> u64 {
        match self {
            Self::Primary => 0,
            Self::Frame(id) => *id,
        }
    }
}

impl From<FrameRef> for u64 {
    fn from(f: FrameRef) -> u64 {
        f.raw_id()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowFullscreenMode {
    None,
    Fullboth,
    Fullscreen,
    Fullwidth,
    Fullheight,
    Maximized,
}

/// Lifecycle commands for the render thread.
#[derive(Debug)]
pub enum LifecycleCommand {
    /// Shutdown the render thread
    Shutdown,
    /// Suspend the active TTY frontend.
    SuspendTty,
    /// Resume the active TTY frontend.
    ResumeTty,
}

/// Window and chrome management commands.
#[derive(Debug)]
pub enum WindowCommand {
    /// Scroll blit pixels within pixel buffer
    ScrollBlit {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        from_y: i32,
        to_y: i32,
        bg_r: f32,
        bg_g: f32,
        bg_b: f32,
    },
    /// Change the mouse pointer cursor shape (arrow, hand, ibeam, etc.)
    SetMouseCursor { cursor_type: i32 },
    /// Warp (move) the mouse pointer to given pixel position
    WarpMouse { x: i32, y: i32 },
    /// Set the window title
    SetWindowTitle { title: String },
    /// Set the title for a specific GUI frame window.
    /// `frame.raw_id() == 0` also targets the adopted primary window.
    SetFrameWindowTitle { frame: FrameRef, title: String },
    /// Set fullscreen/maximized state for a GUI frame window.
    /// `frame.raw_id() == 0` also targets the adopted primary window.
    SetWindowFullscreen {
        frame: FrameRef,
        mode: WindowFullscreenMode,
    },
    /// Minimize/iconify the window
    SetWindowMinimized { minimized: bool },
    /// Set window position
    SetWindowPosition { x: i32, y: i32 },
    /// Request window inner size change
    SetWindowSize { width: u32, height: u32 },
    /// Request resizing a specific GUI frame window.
    /// `frame.raw_id() == 0` also targets the adopted primary window.
    ResizeWindow {
        frame: FrameRef,
        width: u32,
        height: u32,
        geometry_hints: GuiFrameGeometryHints,
    },
    /// Update geometry hints for a specific GUI frame window.
    /// `frame.raw_id() == 0` also targets the adopted primary window.
    SetFrameGeometryHints {
        frame: FrameRef,
        geometry_hints: GuiFrameGeometryHints,
    },
    /// Set window decorations (title bar, borders)
    SetWindowDecorated { decorated: bool },
    /// Create a new OS window for a top-level Emacs frame
    CreateWindow {
        frame: FrameRef,
        width: u32,
        height: u32,
        title: String,
        geometry_hints: GuiFrameGeometryHints,
    },
    /// Associate the already-created primary OS window with its real Emacs frame ID.
    AdoptPrimaryFrame { frame: FrameRef },
    /// Destroy an OS window for a top-level Emacs frame
    DestroyWindow { frame: FrameRef },
    /// Mark a child frame visible again.
    ShowChildFrame { frame_id: u64 },
    /// Remove a child frame (sent when frame is deleted, unparented, or hidden)
    RemoveChildFrame { frame_id: u64 },
    /// Request window attention (urgency hint / taskbar flash)
    RequestAttention { urgent: bool },
}

/// Source for media assets that can be loaded either from files or URIs.
#[derive(Debug)]
pub enum MediaSource {
    /// A filesystem path, matching Emacs Lisp `:file` display specs.
    File(String),
    /// A URI, matching Emacs Lisp `:uri` display specs.
    Uri(String),
}

impl MediaSource {
    pub fn as_str(&self) -> &str {
        match self {
            Self::File(path) | Self::Uri(path) => path,
        }
    }
}

/// Content source for a shader surface
/// (`doc/display-engine/SHADER_SURFACES.md`).
#[derive(Debug)]
pub enum SurfaceSource {
    /// User WGSL defining `fn mainImage(fragCoord: vec2<f32>) -> vec4<f32>`;
    /// the render thread composes it with the generated prelude. `uniforms`
    /// carries the named user uniforms in slot order with initial values;
    /// `channel0` optionally names another surface sampled as `iChannel0`.
    Wgsl {
        source: String,
        uniforms: Vec<neomacs_renderer_wgpu::SurfaceUniformInit>,
        channel0: Option<u32>,
    },
    /// Raw RGBA8 pixels, row-major, tightly packed.
    Pixels { data: Vec<u8> },
}

/// Asset and embedded-content commands.
#[derive(Debug)]
pub enum AssetCommand {
    /// Load image from file (async, ID pre-allocated)
    ImageLoadFile {
        id: u32,
        path: String,
        max_width: u32,
        max_height: u32,
        /// Immutable logical/device geometry captured for this load.
        realization: ImageRealization,
        /// Foreground color as 0xAARRGGBB for monochrome formats (XBM). 0 = default.
        fg_color: u32,
        /// Background color as 0xAARRGGBB for monochrome formats (XBM). 0 = default.
        bg_color: u32,
    },
    /// Load image from encoded data bytes (PNG, JPEG, SVG, etc.)
    ImageLoadData {
        id: u32,
        data: Vec<u8>,
        max_width: u32,
        max_height: u32,
        /// Immutable logical/device geometry captured for this load.
        realization: ImageRealization,
        /// Foreground color as 0xAARRGGBB for monochrome formats (XBM). 0 = default.
        fg_color: u32,
        /// Background color as 0xAARRGGBB for monochrome formats (XBM). 0 = default.
        bg_color: u32,
    },
    /// Load image from raw ARGB32 pixel data
    ImageLoadArgb32 {
        id: u32,
        data: Vec<u8>,
        width: u32,
        height: u32,
        stride: u32,
    },
    /// Load image from raw RGB24 pixel data
    ImageLoadRgb24 {
        id: u32,
        data: Vec<u8>,
        width: u32,
        height: u32,
        stride: u32,
    },
    /// Free an image from cache
    ImageFree {
        id: u32,
    },
    /// Create a WebKit view
    WebKitCreate {
        id: u32,
        width: u32,
        height: u32,
    },
    /// Load URL in WebKit view
    WebKitLoadUri {
        id: u32,
        url: String,
    },
    /// Resize WebKit view
    WebKitResize {
        id: u32,
        width: u32,
        height: u32,
    },
    /// Destroy WebKit view
    WebKitDestroy {
        id: u32,
    },
    /// Click in WebKit view
    WebKitClick {
        id: u32,
        x: i32,
        y: i32,
        button: u32,
    },
    /// Pointer event in WebKit view (raw API)
    WebKitPointerEvent {
        id: u32,
        event_type: u32,
        x: i32,
        y: i32,
        button: u32,
        state: u32,
        modifiers: u32,
    },
    /// Scroll in WebKit view
    WebKitScroll {
        id: u32,
        x: i32,
        y: i32,
        delta_x: i32,
        delta_y: i32,
    },
    /// Keyboard event in WebKit view
    WebKitKeyEvent {
        id: u32,
        keyval: u32,
        keycode: u32,
        pressed: bool,
        modifiers: u32,
    },
    /// Navigate back in WebKit view
    WebKitGoBack {
        id: u32,
    },
    /// Navigate forward in WebKit view
    WebKitGoForward {
        id: u32,
    },
    /// Reload WebKit view
    WebKitReload {
        id: u32,
    },
    /// Execute JavaScript in WebKit view
    WebKitExecuteJavaScript {
        id: u32,
        script: String,
    },
    /// Set floating WebKit overlay position and size
    WebKitSetFloating {
        frame: FrameRef,
        id: u32,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
    /// Remove floating WebKit overlay
    WebKitRemoveFloating {
        frame: FrameRef,
        id: u32,
    },
    /// Create a shader surface (doc/display-engine/SHADER_SURFACES.md)
    SurfaceCreate {
        id: u32,
        source: SurfaceSource,
        width: u32,
        height: u32,
        animate: bool,
    },
    /// Update one named uniform on a shader surface
    SurfaceSetUniform {
        id: u32,
        name: String,
        value: [f32; 4],
    },
    /// Free a shader surface
    SurfaceFree {
        id: u32,
    },
    /// Install (Some, already composed+validated WGSL) or remove (None) the
    /// full-frame post shader
    FrameShaderSet {
        composed: Option<String>,
    },
    /// Create video player
    VideoCreate {
        id: u32,
        source: MediaSource,
        loop_count: i32,
        autoplay: bool,
    },
    /// Control video playback
    VideoPlay {
        id: u32,
    },
    VideoPause {
        id: u32,
    },
    VideoDestroy {
        id: u32,
    },
}

/// Terminal commands.
#[cfg(feature = "neo-term")]
#[derive(Debug)]
pub enum TerminalCommand {
    /// Create a terminal
    TerminalCreate {
        id: u32,
        cols: u16,
        rows: u16,
        mode: u8, // 0=Window, 1=Inline, 2=Floating
        shell: Option<String>,
    },
    /// Write input to a terminal
    TerminalWrite { id: u32, data: Vec<u8> },
    /// Resize a terminal
    TerminalResize { id: u32, cols: u16, rows: u16 },
    /// Destroy a terminal
    TerminalDestroy { id: u32 },
    /// Set floating terminal position and opacity
    TerminalSetFloat {
        id: u32,
        x: f32,
        y: f32,
        opacity: f32,
    },
}

/// UI overlay commands.
#[derive(Debug)]
pub enum UiCommand {
    /// Show a popup menu anchored in the owning frame's logical-pixel space.
    ShowPopupMenu {
        /// Emacs frame_id of the owning top-level frame
        frame: FrameRef,
        placement: neomacs_display_protocol::PopupPlacement,
        items: Vec<PopupMenuItem>,
        title: Option<String>,
        /// Menu face colors (sRGB 0.0-1.0). None = use defaults.
        fg: Option<(f32, f32, f32)>,
        bg: Option<(f32, f32, f32)>,
    },
    /// Hide the active popup menu
    HidePopupMenu,
    /// Show a tooltip at position (x, y)
    ShowTooltip {
        /// Emacs frame_id of the owning top-level frame
        frame: FrameRef,
        x: f32,
        y: f32,
        text: String,
        fg_r: f32,
        fg_g: f32,
        fg_b: f32,
        bg_r: f32,
        bg_g: f32,
        bg_b: f32,
    },
    /// Hide the active tooltip
    HideTooltip,
    /// Trigger visual bell flash
    VisualBell {
        /// Emacs frame_id of the flashing top-level frame
        frame: FrameRef,
    },
}

/// Config and styling commands.
#[derive(Debug)]
pub enum ConfigCommand {
    /// Configure cursor blinking
    SetCursorBlink { enabled: bool, interval_ms: u32 },
    /// Configure cursor animation (smooth motion)
    SetCursorAnimation { enabled: bool, speed: f32 },
    /// Configure all animations
    SetAnimationConfig {
        cursor_enabled: bool,
        cursor_speed: f32,
        cursor_style: crate::core::types::CursorAnimStyle,
        cursor_duration_ms: u32,
        transition_policy: TransitionPolicy,
        trail_size: f32,
    },
    /// Configure smooth cursor size transition on text-scale-adjust
    SetCursorSizeTransition {
        enabled: bool,
        /// Transition duration in milliseconds
        duration_ms: u32,
    },
    /// Enable or disable font ligatures
    SetLigaturesEnabled { enabled: bool },
    /// Update visual effect configuration.
    /// The closure modifies the shared EffectsConfig in-place.
    UpdateEffect(EffectUpdater),
    /// Update a named cursor effect configuration.
    SetCursorEffect(CursorEffectCommand),
    /// Toggle scroll indicators and focus ring
    SetScrollIndicators { enabled: bool },
    /// Set custom title bar height (0 = hidden, >0 = show with given height)
    SetTitlebarHeight { height: f32 },
    /// Toggle FPS counter overlay
    SetShowFps { enabled: bool },
    /// Set window corner radius for borderless mode (0 = no rounding)
    SetCornerRadius { radius: f32 },
    /// Set extra spacing (line spacing in pixels, letter spacing in pixels)
    SetExtraSpacing {
        line_spacing: f32,
        letter_spacing: f32,
    },
    /// Configure rainbow indent guide colors (up to 6 cycling colors by depth)
    SetIndentGuideRainbow {
        enabled: bool,
        /// Colors as sRGB 0.0-1.0 tuples with opacity
        colors: Vec<(f32, f32, f32, f32)>,
    },
    /// Configure child frame visual style (drop shadow, rounded corners)
    SetChildFrameStyle {
        corner_radius: f32,
        shadow_enabled: bool,
        shadow_layers: u32,
        shadow_offset: f32,
        shadow_opacity: f32,
    },
}

/// Clipboard requests routed through the display owner to its serialized worker.
///
/// The evaluator may await `reply`, but the Winit event loop only forwards the
/// request and never performs native clipboard I/O itself.
#[derive(Debug)]
pub enum ClipboardCommand {
    SetText {
        selection: ClipboardSelection,
        text: Option<String>,
        expires_at: Instant,
        reply: Sender<Result<(), String>>,
    },
    GetText {
        selection: ClipboardSelection,
        expires_at: Instant,
        reply: Sender<Result<Option<String>, String>>,
    },
}

impl ClipboardCommand {
    pub(crate) fn is_expired(&self) -> bool {
        let expires_at = match self {
            Self::SetText { expires_at, .. } | Self::GetText { expires_at, .. } => expires_at,
        };
        Instant::now() >= *expires_at
    }
}

/// Command from Emacs to render thread
#[derive(Debug)]
pub enum RenderCommand {
    Lifecycle(LifecycleCommand),
    Window(WindowCommand),
    Asset(AssetCommand),
    #[cfg(feature = "neo-term")]
    Terminal(TerminalCommand),
    Ui(UiCommand),
    Config(ConfigCommand),
    Clipboard(ClipboardCommand),
}

#[cfg(unix)]
type OwnedWakeupEndpoint = OwnedFd;
#[cfg(windows)]
type OwnedWakeupEndpoint = OwnedHandle;

/// Emacs-owned read endpoint of the render-to-evaluator wakeup pipe.
pub struct WakeupReader {
    endpoint: OwnedWakeupEndpoint,
}

/// Render-owned write endpoint of the render-to-evaluator wakeup pipe.
pub struct WakeupWriter {
    endpoint: OwnedWakeupEndpoint,
}

/// Wakeup pipe before its endpoints are split between thread owners.
pub struct WakeupPipe {
    reader: WakeupReader,
    writer: WakeupWriter,
}

impl WakeupPipe {
    pub fn new() -> std::io::Result<Self> {
        let (read, write) = os_pipe::pipe()?;
        Ok(Self {
            reader: WakeupReader {
                endpoint: read.into(),
            },
            writer: WakeupWriter {
                endpoint: write.into(),
            },
        })
    }

    pub fn read_fd(&self) -> WakeupFd {
        self.reader.read_fd()
    }

    pub fn wake(&self) {
        self.writer.wake();
    }

    pub fn clear(&self) {
        self.reader.clear();
    }

    fn into_endpoints(self) -> (WakeupReader, WakeupWriter) {
        (self.reader, self.writer)
    }
}

#[cfg(unix)]
impl WakeupReader {
    pub fn read_fd(&self) -> WakeupFd {
        self.endpoint.as_raw_fd()
    }

    pub fn clear(&self) {
        let fd = self.read_fd();
        let mut buf = [0u8; 64];
        unsafe {
            let flags = libc::fcntl(fd, libc::F_GETFL);
            libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
            while libc::read(fd, buf.as_mut_ptr() as *mut _, buf.len()) > 0 {}
            libc::fcntl(fd, libc::F_SETFL, flags);
        }
    }
}

#[cfg(unix)]
impl WakeupWriter {
    pub fn wake(&self) {
        let fd = self.endpoint.as_raw_fd();
        unsafe {
            libc::write(fd, [1u8].as_ptr() as *const _, 1);
        }
    }
}

#[cfg(windows)]
impl WakeupReader {
    pub fn read_fd(&self) -> WakeupFd {
        self.endpoint.as_raw_handle()
    }

    pub fn clear(&self) {
        use windows_sys::Win32::Storage::FileSystem::ReadFile;
        use windows_sys::Win32::System::Pipes::PeekNamedPipe;
        let handle = self.read_fd();
        let mut buf = [0u8; 64];
        loop {
            let mut avail: u32 = 0;
            unsafe {
                PeekNamedPipe(
                    handle as _,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                    &mut avail,
                    std::ptr::null_mut(),
                );
            }
            if avail == 0 {
                break;
            }
            let mut read_bytes: u32 = 0;
            unsafe {
                ReadFile(
                    handle as _,
                    buf.as_mut_ptr() as _,
                    buf.len() as u32,
                    &mut read_bytes,
                    std::ptr::null_mut(),
                );
            }
            if read_bytes == 0 {
                break;
            }
        }
    }
}

#[cfg(windows)]
impl WakeupWriter {
    pub fn wake(&self) {
        use windows_sys::Win32::Storage::FileSystem::WriteFile;
        unsafe {
            WriteFile(
                self.endpoint.as_raw_handle() as _,
                [1u8].as_ptr() as _,
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        }
    }
}

/// Channel capacities
// Frame channel: unbounded so try_send never drops frames.
// The render thread drains all queued frames and keeps only the latest
// (see poll_frame()), so memory stays bounded in practice.
//
// GNU Emacs' `kbd_buffer` holds 4096 input events and `tty_read_avail_input`
// stops reading terminal bytes when the buffer is under pressure rather than
// silently dropping command input.  Keep Neomacs' render-to-evaluator input
// queue at the same scale and use backpressure for durable user input below.
const INPUT_CHANNEL_CAPACITY: usize = 4096;
const COMMAND_CHANNEL_CAPACITY: usize = 64;

/// Communication channels between threads
pub struct ThreadComms {
    /// Frame display state: Emacs → Render
    pub frame_tx: Sender<SealedFramePresentation>,
    pub frame_rx: Receiver<SealedFramePresentation>,

    /// Commands: Emacs → Render
    pub cmd_tx: Sender<RenderCommand>,
    pub cmd_rx: Receiver<RenderCommand>,

    /// Input events: Render → Emacs
    pub input_tx: Sender<InputEvent>,
    pub input_rx: Receiver<InputEvent>,

    /// Wakeup pipe: Render → Emacs
    pub wakeup: WakeupPipe,
}

impl ThreadComms {
    /// Create new thread communication channels
    pub fn new() -> std::io::Result<Self> {
        let (frame_tx, frame_rx) = unbounded();
        let (cmd_tx, cmd_rx) = bounded(COMMAND_CHANNEL_CAPACITY);
        let (input_tx, input_rx) = bounded(INPUT_CHANNEL_CAPACITY);
        let wakeup = WakeupPipe::new()?;

        Ok(Self {
            frame_tx,
            frame_rx,
            cmd_tx,
            cmd_rx,
            input_tx,
            input_rx,
            wakeup,
        })
    }

    /// Split into Emacs-side and Render-side handles
    pub fn split(self) -> (EmacsComms, RenderComms) {
        let (wakeup_reader, wakeup_writer) = self.wakeup.into_endpoints();
        let emacs = EmacsComms {
            frame_tx: self.frame_tx,
            cmd_tx: self.cmd_tx,
            input_rx: self.input_rx,
            wakeup_reader,
        };

        let render = RenderComms {
            frame_rx: self.frame_rx,
            cmd_rx: self.cmd_rx,
            input_tx: self.input_tx,
            wakeup: wakeup_writer,
        };

        (emacs, render)
    }
}

/// Emacs thread communication handle
pub struct EmacsComms {
    pub frame_tx: Sender<SealedFramePresentation>,
    pub cmd_tx: Sender<RenderCommand>,
    pub input_rx: Receiver<InputEvent>,
    pub wakeup_reader: WakeupReader,
}

/// Render thread communication handle
pub struct RenderComms {
    pub frame_rx: Receiver<SealedFramePresentation>,
    pub cmd_rx: Receiver<RenderCommand>,
    pub input_tx: Sender<InputEvent>,
    pub wakeup: WakeupWriter,
}

impl RenderComms {
    fn is_lossy_input_event(event: &InputEvent) -> bool {
        matches!(
            event,
            InputEvent::MouseMove { .. } | InputEvent::MenuSelection { index: -1 }
        ) || {
            #[cfg(feature = "wpe-webkit")]
            {
                matches!(event, InputEvent::WebKitProgressChanged { .. })
            }
            #[cfg(not(feature = "wpe-webkit"))]
            {
                false
            }
        }
    }

    fn should_log_delivery(event: &InputEvent) -> bool {
        matches!(
            event,
            InputEvent::WindowResize { .. }
                | InputEvent::WindowClose { .. }
                | InputEvent::WindowFocus { .. }
                | InputEvent::MonitorsChanged { .. }
        )
    }

    fn event_name(event: &InputEvent) -> &'static str {
        match event {
            InputEvent::Key { .. } => "key",
            InputEvent::MouseButton { .. } => "mouse-button",
            InputEvent::MouseMove { .. } => "mouse-move",
            InputEvent::PresentedRegion { .. } => "presented-region",
            InputEvent::MouseScroll { .. } => "mouse-scroll",
            InputEvent::WindowResize { .. } => "window-resize",
            InputEvent::WindowClose { .. } => "window-close",
            InputEvent::WindowFocus { .. } => "window-focus",
            InputEvent::MonitorsChanged { .. } => "monitors-changed",
            #[cfg(feature = "wpe-webkit")]
            InputEvent::WebKitTitleChanged { .. } => "webkit-title-changed",
            #[cfg(feature = "wpe-webkit")]
            InputEvent::WebKitUrlChanged { .. } => "webkit-url-changed",
            #[cfg(feature = "wpe-webkit")]
            InputEvent::WebKitProgressChanged { .. } => "webkit-progress-changed",
            #[cfg(feature = "wpe-webkit")]
            InputEvent::WebKitLoadFinished { .. } => "webkit-load-finished",
            InputEvent::ImageStateChanged { .. } => "image-state-changed",
            InputEvent::MenuSelection { .. } => "menu-selection",
            InputEvent::FileDrop { .. } => "file-drop",
            InputEvent::ToolBarClick { .. } => "toolbar-click",
            InputEvent::PresentedPointer { .. } => "presented-pointer",
            InputEvent::PresentationActivated { .. } => "presentation-activated",
            InputEvent::PresentationDiscarded { .. } => "presentation-discarded",
            InputEvent::PresentationRetired { .. } => "presentation-retired",
            InputEvent::MenuBarClick { .. } => "menubar-click",
            #[cfg(feature = "neo-term")]
            InputEvent::TerminalExited { .. } => "terminal-exited",
            #[cfg(feature = "neo-term")]
            InputEvent::TerminalTitleChanged { .. } => "terminal-title-changed",
        }
    }

    /// Send input event to Emacs and wake it up
    pub fn send_input(&self, event: InputEvent) {
        let log_delivery = Self::should_log_delivery(&event);
        let event_name = Self::event_name(&event);
        if Self::is_lossy_input_event(&event) {
            match self.input_tx.try_send(event) {
                Ok(()) => {
                    if log_delivery {
                        tracing::debug!("send_input: queued {}", event_name);
                    }
                    self.wakeup.wake();
                }
                Err(TrySendError::Full(event)) => {
                    tracing::debug!(
                        "send_input: dropped lossy {} because the input queue is full",
                        Self::event_name(&event)
                    );
                }
                Err(TrySendError::Disconnected(event)) => {
                    tracing::warn!(
                        "send_input: dropped {} because the input queue is disconnected",
                        Self::event_name(&event)
                    );
                }
            }
            return;
        }

        match self.input_tx.send(event) {
            Ok(()) => {
                if log_delivery {
                    tracing::debug!("send_input: queued {}", event_name);
                }
                self.wakeup.wake();
            }
            Err(err) => {
                tracing::warn!(
                    "send_input: dropped {} because the input queue is disconnected",
                    Self::event_name(&err.0)
                );
            }
        }
    }
}

#[cfg(test)]
#[path = "thread_comm_test.rs"]
mod tests;

//! Native inline web views for macOS, using `WKWebView`.
//!
//! `WKWebView` has no offscreen render path — WebKit2's process architecture
//! rules it out, as GNU Emacs records in the header comment of
//! `src/nsxwidget.m` — so an inline browser on macOS is a native view
//! positioned over the GPU surface, not a texture composited into it. This
//! module is the macOS sibling of `backend/wpe`, which takes the texture route
//! on Linux via dma-buf.
//!
//! The placement algorithm is GNU Emacs', ported function for function:
//!
//! | Emacs | here |
//! |---|---|
//! | `produce_xwidget_glyph` (`xdisp.c`) | `GlyphType::Xwidget`, already in the layout engine |
//! | `x_draw_xwidget_glyph_string` (`xwidget.c:2793`) | [`Placement::new`] + [`view::WkWebView::apply`] |
//! | `xwidget_end_redisplay` (`xwidget.c:4135`) | [`WkWebViewHost::sync_frame`] |
//! | `nsxwidget_hide_view` (`nsxwidget.m:607`) | [`view::WkWebView::hide`] |

mod view;

use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr::NonNull;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::NSView;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

pub use view::Placement;
use view::WkWebView;

/// Owns every inline web view for one top-level window.
pub struct WkWebViewHost {
    mtm: MainThreadMarker,
    /// The winit content view. Views are created lazily, so this can be bound
    /// after the first `WebKitCreate` arrives.
    host: Option<Retained<NSView>>,
    views: HashMap<u32, WkWebView>,
    /// Ids asked for before a host view existed, so they can be built once one
    /// does. Without this, a `WebKitCreate` racing window setup is lost.
    pending: HashMap<u32, (f64, f64)>,
}

impl WkWebViewHost {
    /// Returns `None` off the main thread. Every AppKit call below assumes the
    /// marker obtained here, so this is the single gate for the whole module.
    pub fn new() -> Option<Self> {
        let mtm = MainThreadMarker::new()?;
        Some(Self {
            mtm,
            host: None,
            views: HashMap::new(),
            pending: HashMap::new(),
        })
    }

    /// Bind to a window's content view. Idempotent, and cheap enough to call
    /// once per frame.
    pub fn attach(&mut self, window: &impl HasWindowHandle) {
        if self.host.is_some() {
            return;
        }
        let Ok(handle) = window.window_handle() else {
            return;
        };
        let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
            return;
        };
        // SAFETY: winit hands out a live NSView pointer for the window, and we
        // retain it for as long as the host lives.
        let ns_view: Retained<NSView> = unsafe {
            let ptr: NonNull<c_void> = appkit.ns_view;
            Retained::retain(ptr.as_ptr().cast())
                .expect("winit AppKit window handle carries a live NSView")
        };
        self.host = Some(ns_view);
        tracing::info!("wkwebview: bound to winit content view");

        let pending = std::mem::take(&mut self.pending);
        for (id, (width, height)) in pending {
            self.create(id, width, height);
        }
    }

    pub fn create(&mut self, id: u32, width: f64, height: f64) {
        let Some(host) = self.host.as_ref() else {
            // Replay once `attach` succeeds.
            self.pending.insert(id, (width, height));
            return;
        };
        if self.views.contains_key(&id) {
            tracing::warn!("wkwebview: view {id} already exists");
            return;
        }
        let view = WkWebView::new(self.mtm, host, width, height);
        self.views.insert(id, view);
        tracing::info!("wkwebview: created view {id} ({width}x{height})");
    }

    pub fn load_uri(&mut self, id: u32, url: &str) {
        match self.views.get(&id) {
            Some(view) => view.load_uri(url),
            None => tracing::warn!("wkwebview: load_uri for unknown view {id}"),
        }
    }

    pub fn resize(&mut self, id: u32, width: f64, height: f64) {
        match self.views.get_mut(&id) {
            Some(view) => view.resize(width, height),
            None => tracing::warn!("wkwebview: resize for unknown view {id}"),
        }
    }

    pub fn destroy(&mut self, id: u32) {
        self.pending.remove(&id);
        if self.views.remove(&id).is_some() {
            tracing::info!("wkwebview: destroyed view {id}");
        }
    }

    pub fn is_empty(&self) -> bool {
        self.views.is_empty() && self.pending.is_empty()
    }

    /// Place every web view this frame references and hide the rest.
    ///
    /// This is `xwidget_end_redisplay` (`xwidget.c:4135`), which Emacs calls
    /// from `dispnew.c:4626` once redisplay has settled. The design point
    /// worth keeping: **nothing signals that a view has left the screen**.
    /// Views are hidden by default unless this frame's glyphs vouched for
    /// them, so scrolling away, deleting a window, switching buffers and
    /// killing a buffer all work without any of those paths knowing that
    /// inline web views exist.
    ///
    /// `placements` yields `(view_id, Placement)` in logical points, top-down.
    /// It must be driven from the frame that was actually presented — see the
    /// call site in `render_thread::render_pass`.
    pub fn sync_frame<I>(&mut self, placements: I)
    where
        I: IntoIterator<Item = (u32, Placement)>,
    {
        if self.views.is_empty() {
            return;
        }
        let Some(host) = self.host.as_ref() else {
            return;
        };
        let host_flipped = host.isFlipped();
        let host_height = host.bounds().size.height;

        // Pass 1 — clear, matching `xwidget_start_redisplay`.
        for view in self.views.values_mut() {
            view.set_touched(false);
        }

        // Pass 2 — walk the frame and place what it references.
        for (id, placement) in placements {
            let Some(view) = self.views.get_mut(&id) else {
                continue;
            };
            if view.touched() {
                // The same model reached us twice in one frame, which means
                // two windows are displaying it. WebKit2's process
                // architecture does not allow one model to back two views —
                // GNU Emacs refuses outright here with "You can't share an
                // xwidget (webkit2) among windows." (`xwidget.c:2820-2836`).
                // We keep the first placement rather than letting the last
                // writer win, and say so once per view.
                if !view.warned_shared() {
                    tracing::warn!(
                        "wkwebview: view {id} is displayed in more than one window; \
                         WebKit allows only one view per model, so the extra \
                         window will not show it"
                    );
                    view.set_warned_shared(true);
                }
                continue;
            }
            view.set_touched(true);
            view.apply(placement, host_flipped, host_height);
        }

        // Pass 3 — hide anything the frame did not vouch for.
        for view in self.views.values_mut() {
            if !view.touched() {
                view.hide();
            }
        }
    }
}

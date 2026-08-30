//! One inline WKWebView, and the geometry that positions it.
//!
//! This is the macOS counterpart of a single `struct xwidget_view` in GNU
//! Emacs (`src/xwidget.h`), driven by the same arithmetic as
//! `x_draw_xwidget_glyph_string` in `src/xwidget.c`.
//!
//! Emacs nests three views: the Emacs view, a clip view (`XvWindow`), and a
//! holder for the web view (`XwWindow`). We use two — a clip view and the
//! `WKWebView` itself — because we do not need Emacs' separate holder.
//!
//! Emacs makes every view in that nest flipped (`isFlipped -> YES`:
//! `nsterm.m:8540` for the Emacs view, `nsxwidget.m:484`/`:554` for the other
//! two) so it can position all of them with top-down coordinates directly.
//! [`XwidgetClipView`] does the same for our clip view, which is why
//! [`Placement::inner_origin`] is Emacs' formula unchanged.
//!
//! The one orientation this file does not control is the host's: the winit
//! content view is whatever winit made it. `Placement` is therefore computed
//! top-down exactly as Emacs computes it, and only [`Placement::ns_origin`] --
//! the one frame expressed in the host's space -- converts.

use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::NSView;
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString, NSURL, NSURLRequest};
use objc2_web_kit::{WKWebView, WKWebViewConfiguration};

/// Where a view sits this frame, in logical (point) units, top-down.
///
/// Field-for-field the subset of Emacs' `struct xwidget_view` that placement
/// reads: `x`, `y`, `clip_left`, `clip_right`, `clip_top`, `clip_bottom`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub x: f64,
    pub y: f64,
    /// The widget box itself — Emacs reads this from the model (`xww->width`,
    /// `xww->height`). Kept here so the clip and the inner web-view frame are
    /// derived from one value and cannot drift apart.
    pub width: f64,
    pub height: f64,
    pub clip_left: f64,
    pub clip_right: f64,
    pub clip_top: f64,
    pub clip_bottom: f64,
}

impl Placement {
    /// Intersect a widget box with the window's text area.
    ///
    /// Ported from `x_draw_xwidget_glyph_string` (`xwidget.c:2841-2849`).
    /// Emacs works in widget-local coordinates and so do we: `clip_left` and
    /// `clip_top` are insets from the widget's own top-left corner, which is
    /// what makes the negative inner offset in [`WkWebView::apply`] work.
    ///
    /// `clip` is the window's text area in the same space as `x`/`y`; `None`
    /// means unclipped, matching a `FrameGlyph` that carries no `clip_rect`.
    pub fn new(
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        clip: Option<(f64, f64, f64, f64)>,
    ) -> Self {
        let Some((area_x, area_y, area_w, area_h)) = clip else {
            return Self {
                x,
                y,
                width,
                height,
                clip_left: 0.0,
                clip_right: width,
                clip_top: 0.0,
                clip_bottom: height,
            };
        };
        // The max() on right/bottom is Emacs': it keeps the rect from
        // inverting once the widget has scrolled entirely out of the area.
        let clip_left = 0.0_f64.max(area_x - x);
        let clip_right = clip_left.max(width.min(area_x + area_w - x));
        let clip_top = 0.0_f64.max(area_y - y);
        let clip_bottom = clip_top.max(height.min(area_y + area_h - y));
        Self {
            x,
            y,
            width,
            height,
            clip_left,
            clip_right,
            clip_top,
            clip_bottom,
        }
    }

    pub fn visible_width(&self) -> f64 {
        self.clip_right - self.clip_left
    }

    pub fn visible_height(&self) -> f64 {
        self.clip_bottom - self.clip_top
    }

    /// Nothing of the widget is on screen this frame.
    pub fn is_empty(&self) -> bool {
        self.visible_width() <= 0.0 || self.visible_height() <= 0.0
    }

    /// Top-left of the visible region, in top-down coordinates.
    ///
    /// Emacs: `x + clip_left`, `y + clip_top` (`xwidget.c:2888`).
    pub fn top_left(&self) -> (f64, f64) {
        (self.x + self.clip_left, self.y + self.clip_top)
    }

    /// Origin for the clip view in the *host's* own coordinate space.
    ///
    /// This is the one frame whose orientation this file does not own, so it
    /// is the one place `host_flipped` belongs. Emacs' host is flipped
    /// (`nsterm.m:8540`) so it uses `top_left` directly; a bottom-left host
    /// needs the height folded in — which is why the caller must rewrite the
    /// origin whenever the visible *height* or the *host* height changes, not
    /// only when the top-left moves.
    pub fn ns_origin(&self, host_flipped: bool, host_height: f64) -> (f64, f64) {
        let (left, top) = self.top_left();
        if host_flipped {
            (left, top)
        } else {
            (left, host_height - (top + self.visible_height()))
        }
    }

    /// Origin of the web view *inside* the clip view.
    ///
    /// Emacs: `(-clip_left, -clip_top)` (`xwidget.c:2996`), unconditionally.
    ///
    /// This frame is expressed in the *clip view's* coordinate system, not the
    /// host's, so the host's orientation has no bearing on it. Ours is
    /// [`XwidgetClipView`], which is flipped exactly as Emacs' `XvWindow` is,
    /// so Emacs' formula transfers unchanged.
    pub fn inner_origin(&self) -> (f64, f64) {
        (-self.clip_left, -self.clip_top)
    }

    /// Has the *on-screen* origin moved?
    ///
    /// Ported from `xwidget.c:2856`. Emacs compares the clipped origin rather
    /// than the widget origin, and the reason is worth preserving: the visible
    /// area can sit still while the widget moves (a window border crossing
    /// it), and the widget can sit still while the visible area moves.
    pub fn moved_from(&self, prev: &Placement) -> bool {
        prev.x + prev.clip_left != self.x + self.clip_left
            || prev.y + prev.clip_top != self.y + self.clip_top
    }

    /// Has the clip changed? Ported from `xwidget.c:2961`, which guards
    /// reclipping separately from movement.
    pub fn reclipped_from(&self, prev: &Placement) -> bool {
        prev.clip_left != self.clip_left
            || prev.clip_right != self.clip_right
            || prev.clip_top != self.clip_top
            || prev.clip_bottom != self.clip_bottom
    }
}

/// Does the clip view's origin have to be rewritten this frame?
///
/// Emacs gates this on movement alone (`xwidget.c:2951`) and is entitled to,
/// because [`Placement::ns_origin`] on a flipped host is just `top_left` and
/// so depends on nothing else. A bottom-left origin is
/// `host_height - (top + visible_height)`, which brings in two more inputs
/// that a placement diff cannot see:
///
/// - the *visible* height, so a pure reclip moves it (a window resized shorter
///   under a widget whose top has not moved);
/// - the *host's* height, so a window resize moves it even when the placement
///   is identical.
pub const fn needs_reposition(
    moved: bool,
    reclipped: bool,
    host_changed: bool,
    host_flipped: bool,
) -> bool {
    moved || (!host_flipped && (reclipped || host_changed))
}

define_class!(
    /// The clip view, flipped.
    ///
    /// Emacs' equivalent is `XvWindow` (`nsxwidget.m:553-555`), which exists
    /// for exactly this reason: a stock `NSView` is bottom-left, and every
    /// number `x_draw_xwidget_glyph_string` computes is top-down. Emacs makes
    /// the view match the arithmetic rather than converting the arithmetic to
    /// match the view, and so do we -- a converted `inner_origin` would have
    /// to know an orientation that belongs to a view two levels up.
    ///
    /// SAFETY:
    /// - `NSView` places no requirements on subclasses beyond being used on
    ///   the main thread, which `MainThreadOnly` enforces.
    /// - This type does not implement `Drop`.
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "NeomacsXwidgetClipView"]
    pub(crate) struct XwidgetClipView;

    impl XwidgetClipView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }
    }
);

impl XwidgetClipView {
    fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        // `NSView::initWithFrame` is generated against `Allocated<NSView>`,
        // so a subclass has to send the initializer itself.
        unsafe { msg_send![Self::alloc(mtm), initWithFrame: frame] }
    }
}

/// A live inline web view: the clip view and the `WKWebView` inside it.
pub(crate) struct WkWebView {
    clip: Retained<XwidgetClipView>,
    web: Retained<WKWebView>,
    /// Last applied placement, for the two dirty checks. `None` until the
    /// view has been placed at least once.
    applied: Option<Placement>,
    /// Host orientation and height as of the last placement.
    ///
    /// `ns_origin` reads both, and neither is visible to `moved_from` or
    /// `reclipped_from`, so without this a window resized taller or shorter
    /// under a widget that did not itself move would leave the view at a
    /// stale bottom-up origin.
    applied_host: Option<(bool, f64)>,
    /// Touched by the current frame's glyph walk. See `xwidget_touch` /
    /// `xwidget_touched` in `xwidget.c`.
    touched: bool,
    hidden: bool,
    /// One-shot latch so the "shared among windows" warning is not repeated
    /// on every frame.
    warned_shared: bool,
}

impl WkWebView {
    /// Build the view pair and add it to `host`. The view starts hidden: the
    /// first frame that references it will place and show it, and a view that
    /// is never referenced must never appear.
    pub fn new(mtm: MainThreadMarker, host: &NSView, width: f64, height: f64) -> Self {
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, height));

        let clip = XwidgetClipView::new(mtm, frame);
        // Emacs gets clipping from the nesting itself; we ask for it
        // explicitly rather than depend on the platform default, which has
        // changed across macOS releases.
        clip.setClipsToBounds(true);

        let config = unsafe { WKWebViewConfiguration::new(mtm) };
        let web = unsafe {
            WKWebView::initWithFrame_configuration(WKWebView::alloc(mtm), frame, &config)
        };

        clip.addSubview(&web);
        host.addSubview(&clip);
        clip.setHidden(true);

        Self {
            clip,
            web,
            applied: None,
            applied_host: None,
            touched: false,
            hidden: true,
            warned_shared: false,
        }
    }

    pub fn load_uri(&self, url: &str) {
        let Some(nsurl) = NSURL::URLWithString(&NSString::from_str(url)) else {
            tracing::warn!("wkwebview: refusing to load unparseable URL {url:?}");
            return;
        };
        let request = NSURLRequest::requestWithURL(&nsurl);
        let _ = unsafe { self.web.loadRequest(&request) };
    }

    /// Run JavaScript in the page.
    ///
    /// GNU's `xwidget-webkit-execute-script' can hand the script's return
    /// value to a callback; delivering that would need a result channel back
    /// to the Lisp thread, so the completion handler is nil here and the call
    /// is fire-and-forget.
    pub fn evaluate_javascript(&self, script: &str) {
        unsafe {
            self.web
                .evaluateJavaScript_completionHandler(&NSString::from_str(script), None);
        }
    }

    /// Resize the model.
    ///
    /// Applied to the web view straight away so the page reflows now rather
    /// than on the next frame that happens to place it, and `applied` is
    /// cleared so the next placement writes both frames through.
    ///
    /// The size is not retained here. `Placement` carries the widget box and
    /// is the single source for both the clip and the inner web-view frame,
    /// so a second copy on this struct could only drift out of step with it.
    pub fn resize(&mut self, width: f64, height: f64) {
        self.web.setFrameSize(NSSize::new(width, height));
        self.applied = None;
        self.applied_host = None;
    }

    pub fn set_touched(&mut self, touched: bool) {
        self.touched = touched;
    }

    pub fn touched(&self) -> bool {
        self.touched
    }

    pub fn warned_shared(&self) -> bool {
        self.warned_shared
    }

    pub fn set_warned_shared(&mut self, warned: bool) {
        self.warned_shared = warned;
    }

    /// Position the view pair for this frame.
    ///
    /// `host_height` is the host view's height in points, used only to convert
    /// to bottom-left coordinates when the host is not flipped.
    pub fn apply(&mut self, placement: Placement, host_flipped: bool, host_height: f64) {
        if placement.is_empty() {
            self.hide();
            return;
        }

        let moved = self
            .applied
            .as_ref()
            .is_none_or(|prev| placement.moved_from(prev));
        let reclipped = self
            .applied
            .as_ref()
            .is_none_or(|prev| placement.reclipped_from(prev));
        let host = (host_flipped, host_height);
        let host_changed = self.applied_host != Some(host);

        if reclipped {
            // Emacs: nsxwidget_resize_view + nsxwidget_move_widget_in_view.
            self.clip.setFrameSize(NSSize::new(
                placement.visible_width(),
                placement.visible_height(),
            ));
            let (inner_x, inner_y) = placement.inner_origin();
            self.web.setFrameOrigin(NSPoint::new(inner_x, inner_y));
            self.web
                .setFrameSize(NSSize::new(placement.width, placement.height));
        }

        // Emacs: nsxwidget_move_view (xv, x + clip_left, y + clip_top).
        if needs_reposition(moved, reclipped, host_changed, host_flipped) {
            let (ns_x, ns_y) = placement.ns_origin(host_flipped, host_height);
            self.clip.setFrameOrigin(NSPoint::new(ns_x, ns_y));
        }

        self.applied = Some(placement);
        self.applied_host = Some(host);
        self.show();
    }

    /// Emacs parks hidden views far offscreen (`nsxwidget.m:607`). `setHidden`
    /// is the better tool here: it also stops the view from taking hit tests,
    /// which matters because our clip view is a sibling of nothing else.
    pub fn hide(&mut self) {
        if !self.hidden {
            self.clip.setHidden(true);
            self.hidden = true;
        }
    }

    fn show(&mut self) {
        if self.hidden {
            self.clip.setHidden(false);
            self.hidden = false;
        }
    }
}

impl Drop for WkWebView {
    fn drop(&mut self) {
        self.web.removeFromSuperview();
        self.clip.removeFromSuperview();
    }
}

#[cfg(test)]
#[path = "view_test.rs"]
mod tests;

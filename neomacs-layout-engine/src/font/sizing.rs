//! Platform display rules for converting Emacs face heights to layout units.
//!
//! Font catalogs answer which font exists. They do not own frame DPI or the
//! conversion between GNU printer points, logical coordinates, and device
//! pixels. Keeping this module independent prevents X11 policy from leaking
//! into the CoreText and DirectWrite catalogs.

use neovm_core::face::{Face, FaceHeight};
use std::sync::OnceLock;

#[cfg(target_os = "linux")]
use std::ffi::{CStr, CString};
#[cfg(target_os = "linux")]
use std::ptr;
#[cfg(target_os = "linux")]
use x11_dl::xlib;

/// GNU uses the printer's point rather than the desktop-publishing 72 DPI
/// point for its `POINT_TO_PIXEL` conversion (`src/font.h`).
pub const GNU_POINTS_PER_INCH: f32 = 72.27;

/// The logical-coordinate rule selected by the active display frontend.
///
/// Device/backing scale is deliberately absent: it is applied later when a
/// realized logical size becomes a raster request.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum LogicalFontScale {
    /// GNU NS sets frame resolution to 72.27 so Emacs points map to Cocoa
    /// logical units before Retina backing scale is applied.
    GnuCocoaPoint,
    /// DirectWrite sizes are device-independent pixels at 96 logical DPI.
    WindowsDip,
    /// Neomacs's Wayland logical-coordinate policy, independently named so it
    /// cannot be confused with DirectWrite merely because both currently use
    /// 96 logical DPI.
    WaylandLogical,
    /// X11 uses the frame/display's effective Xft DPI.
    X11 { effective_dpi: f32 },
    /// Explicit frontend/test value.
    ExplicitDpi(f32),
}

impl LogicalFontScale {
    fn layout_dpi(self) -> f32 {
        let dpi = match self {
            Self::GnuCocoaPoint => GNU_POINTS_PER_INCH,
            Self::WindowsDip | Self::WaylandLogical => 96.0,
            Self::X11 { effective_dpi } | Self::ExplicitDpi(effective_dpi) => effective_dpi,
        };
        if dpi.is_finite() && dpi > 0.0 {
            dpi
        } else {
            96.0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontSizing {
    scale: LogicalFontScale,
}

impl FontSizing {
    pub const fn new(scale: LogicalFontScale) -> Self {
        Self { scale }
    }

    /// Compatibility constructor for X11 call sites. New GUI code should
    /// select a frontend-specific rule through [`Self::native_gui`].
    pub fn xft() -> Self {
        Self::new(LogicalFontScale::X11 {
            effective_dpi: xft_dpi(),
        })
    }

    /// Compatibility name for the existing 96-DPI logical rule.
    pub const fn logical() -> Self {
        Self::new(LogicalFontScale::WaylandLogical)
    }

    pub const fn gnu_cocoa() -> Self {
        Self::new(LogicalFontScale::GnuCocoaPoint)
    }

    pub const fn windows_dip() -> Self {
        Self::new(LogicalFontScale::WindowsDip)
    }

    pub const fn wayland() -> Self {
        Self::new(LogicalFontScale::WaylandLogical)
    }

    pub const fn for_layout_dpi(layout_dpi: f32) -> Self {
        Self::new(LogicalFontScale::ExplicitDpi(layout_dpi))
    }

    pub fn native_gui() -> Self {
        std::cfg_select! {
            target_os = "macos" => Self::gnu_cocoa(),
            windows => Self::windows_dip(),
            target_os = "linux" => Self::xft(),
            _ => Self::logical(),
        }
    }

    pub fn layout_dpi(self) -> f32 {
        self.scale.layout_dpi()
    }

    pub fn face_height_to_layout_pixels(self, tenths: i32) -> f32 {
        points_to_layout_pixels(tenths as f32 / 10.0, self.layout_dpi())
    }

    pub fn font_size_px_for_face(self, face: &Face) -> f32 {
        let default_font_size = self.face_height_to_layout_pixels(100);
        match &face.height {
            Some(FaceHeight::Absolute(tenths)) => self.face_height_to_layout_pixels(*tenths),
            Some(FaceHeight::Relative(scale)) => default_font_size * (*scale as f32),
            None => default_font_size,
        }
    }
}

pub fn points_to_layout_pixels(points: f32, dpi: f32) -> f32 {
    (points * dpi / GNU_POINTS_PER_INCH).round()
}

/// Compatibility helper for GNU X11 callers.
pub fn points_to_pixels(points: f32) -> f32 {
    points_to_layout_pixels(points, xft_dpi())
}

/// Compatibility helper for a face height in tenths of a point.
pub fn face_height_to_pixels(tenths: i32) -> f32 {
    points_to_pixels(tenths as f32 / 10.0)
}

static XFT_DPI: OnceLock<f32> = OnceLock::new();
static X_DPI_PROBE_DISABLED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

pub fn disable_x_dpi_probe() {
    X_DPI_PROBE_DISABLED.store(true, std::sync::atomic::Ordering::Relaxed);
}

pub fn xft_dpi() -> f32 {
    *XFT_DPI.get_or_init(|| {
        let dpi = query_xft_dpi().unwrap_or(100.0);
        tracing::info!("Xft.dpi: {}", dpi);
        dpi
    })
}

#[cfg(target_os = "linux")]
fn query_xft_dpi() -> Option<f32> {
    if X_DPI_PROBE_DISABLED.load(std::sync::atomic::Ordering::Relaxed)
        || std::env::var("DISPLAY").unwrap_or_default().is_empty()
    {
        return None;
    }

    let (tx, rx) = std::sync::mpsc::channel();
    let _handle = std::thread::Builder::new()
        .name("xft-dpi-probe".into())
        .spawn(move || {
            let result = query_xft_dpi_inner();
            let _ = tx.send(result);
        });
    match rx.recv_timeout(std::time::Duration::from_millis(100)) {
        Ok(result) => result,
        Err(_) => {
            tracing::warn!(
                "query_xft_dpi: X11 connection timed out (broken display?), using fallback DPI"
            );
            None
        }
    }
}

#[cfg(target_os = "linux")]
fn query_xft_dpi_inner() -> Option<f32> {
    let xlib = xlib::Xlib::open().ok()?;
    let display = unsafe { (xlib.XOpenDisplay)(ptr::null()) };
    if display.is_null() {
        return None;
    }

    let class = CString::new("Xft").ok()?;
    let name = CString::new("dpi").ok()?;
    let dpi = unsafe {
        let resource = (xlib.XGetDefault)(display, class.as_ptr(), name.as_ptr());
        let parsed = if resource.is_null() {
            None
        } else {
            CStr::from_ptr(resource)
                .to_str()
                .ok()
                .and_then(|value| value.trim().parse::<f32>().ok())
        };
        match parsed {
            Some(dpi) if dpi.is_finite() && dpi > 0.0 => Some(dpi),
            _ => {
                let screen = (xlib.XDefaultScreen)(display);
                let pixels = (xlib.XDisplayHeight)(display, screen);
                let mm = (xlib.XDisplayHeightMM)(display, screen);
                Some(fallback_frame_res_y(pixels, mm))
            }
        }
    };
    unsafe { (xlib.XCloseDisplay)(display) };
    dpi
}

#[cfg(not(target_os = "linux"))]
fn query_xft_dpi() -> Option<f32> {
    None
}

pub(crate) fn fallback_frame_res_y(display_height_px: i32, display_height_mm: i32) -> f32 {
    if display_height_mm < 1 {
        100.0
    } else {
        display_height_px as f32 * 25.4 / display_height_mm as f32
    }
}

#[cfg(test)]
#[path = "sizing_test.rs"]
mod tests;

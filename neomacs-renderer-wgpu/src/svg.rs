//! SVG measurement and rasterization through one cross-platform backend.
//!
//! Natural geometry and pixels must come from the same SVG implementation.
//! Otherwise a dimensionless document can be measured with one set of layout
//! rules and painted with another.

use cairo::{Format, ImageSurface};
use rsvg::{CairoRenderer, Length, LengthUnit, Loader, SvgHandle};

use crate::image_cache::constrain_dimensions;

pub(crate) struct DecodedSvg {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) rgba: Vec<u8>,
}

const DEFAULT_DPI: f64 = 96.0;

pub(crate) fn query_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    let handle = load(data)?;
    let renderer = CairoRenderer::new(&handle);
    let (width, height) = natural_dimensions(&renderer)?;
    Some((width.ceil() as u32, height.ceil() as u32))
}

pub(crate) fn decode(data: &[u8], max_width: u32, max_height: u32) -> Option<DecodedSvg> {
    let handle = load(data)?;
    let renderer = CairoRenderer::new(&handle);
    let (natural_width, natural_height) = natural_dimensions(&renderer)?;
    let (width, height) = constrain_dimensions(
        natural_width.ceil() as u32,
        natural_height.ceil() as u32,
        max_width,
        max_height,
    );

    let mut surface = ImageSurface::create(Format::ARgb32, width as i32, height as i32).ok()?;
    let context = cairo::Context::new(&surface).ok()?;
    // Render in the document's measured coordinate space, then scale that
    // complete space into the constrained output.  A dimensionless SVG has
    // no intrinsic viewport for librsvg to resize, so passing the smaller
    // output rectangle directly would clip absolute coordinates (including
    // CSS-pixel font sizes) instead of scaling them.  This is equivalent to
    // GNU Emacs's generated outer SVG with a natural-size viewBox.
    context.scale(
        f64::from(width) / natural_width,
        f64::from(height) / natural_height,
    );
    let viewport = cairo::Rectangle::new(0.0, 0.0, natural_width, natural_height);
    renderer.render_document(&context, &viewport).ok()?;
    drop(context);

    surface.flush();
    let stride = surface.stride() as usize;
    let pixels = surface.data().ok()?;
    let rgba = cairo_argb32_to_rgba(&pixels, stride, width, height);
    drop(pixels);

    Some(DecodedSvg {
        width,
        height,
        rgba,
    })
}

fn load(data: &[u8]) -> Option<SvgHandle> {
    let bytes = glib::Bytes::from(data);
    let stream = gio::MemoryInputStream::from_bytes(&bytes);
    Loader::new()
        .read_stream(&stream, None::<&gio::File>, None::<&gio::Cancellable>)
        .ok()
}

fn natural_dimensions(renderer: &CairoRenderer<'_>) -> Option<(f64, f64)> {
    if let Some((width, height)) = renderer.intrinsic_size_in_pixels()
        && valid_dimensions(width, height)
    {
        return Some((width, height));
    }

    let intrinsic = renderer.intrinsic_dimensions();
    let explicit_width = absolute_length_in_pixels(&intrinsic.width);
    let explicit_height = absolute_length_in_pixels(&intrinsic.height);
    let view_box = intrinsic
        .vbox
        .filter(|rect| valid_dimensions(rect.width(), rect.height()));

    match (explicit_width, explicit_height, view_box.as_ref()) {
        (Some(width), Some(height), _) if valid_dimensions(width, height) => {
            return Some((width, height));
        }
        (Some(width), None, Some(view_box)) if width > 0.0 => {
            return Some((width, width * view_box.height() / view_box.width()));
        }
        (None, Some(height), Some(view_box)) if height > 0.0 => {
            return Some((height * view_box.width() / view_box.height(), height));
        }
        (_, _, Some(view_box)) => return Some((view_box.width(), view_box.height())),
        _ => {}
    }

    // This is GNU Emacs's fallback for an SVG without usable intrinsic
    // dimensions: ask librsvg for the visible ink geometry in a maximal
    // viewport, then include any positive origin offset in the extent.
    let viewport = cairo::Rectangle::new(0.0, 0.0, f64::from(u32::MAX), f64::from(u32::MAX));
    let (ink, _) = renderer.geometry_for_layer(None, &viewport).ok()?;
    let width = ink.x() + ink.width();
    let height = ink.y() + ink.height();
    valid_dimensions(width, height).then_some((width, height))
}

fn absolute_length_in_pixels(length: &Length) -> Option<f64> {
    let value = length.length;
    match length.unit {
        LengthUnit::Px => Some(value),
        LengthUnit::In => Some(value * DEFAULT_DPI),
        LengthUnit::Cm => Some(value * DEFAULT_DPI / 2.54),
        LengthUnit::Mm => Some(value * DEFAULT_DPI / 25.4),
        LengthUnit::Pt => Some(value * DEFAULT_DPI / 72.0),
        LengthUnit::Pc => Some(value * DEFAULT_DPI / 6.0),
        _ => None,
    }
}

fn valid_dimensions(width: f64, height: f64) -> bool {
    width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0
}

fn cairo_argb32_to_rgba(pixels: &[u8], stride: usize, width: u32, height: u32) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for row in pixels.chunks(stride).take(height as usize) {
        for pixel in row.chunks_exact(4).take(width as usize) {
            let argb = u32::from_ne_bytes(pixel.try_into().expect("four-byte Cairo pixel"));
            let alpha = (argb >> 24) as u8;
            let red = unpremultiply((argb >> 16) as u8, alpha);
            let green = unpremultiply((argb >> 8) as u8, alpha);
            let blue = unpremultiply(argb as u8, alpha);
            rgba.extend_from_slice(&[red, green, blue, alpha]);
        }
    }
    rgba
}

fn unpremultiply(channel: u8, alpha: u8) -> u8 {
    match alpha {
        0 | 255 => channel,
        _ => ((u32::from(channel) * 255) / u32::from(alpha)).min(255) as u8,
    }
}

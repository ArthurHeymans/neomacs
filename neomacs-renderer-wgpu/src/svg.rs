//! SVG measurement and rasterization behind one platform backend.
//!
//! The renderer must derive the natural size and pixels from the same SVG
//! implementation.  Otherwise a dimensionless document can be measured with
//! one set of layout rules and painted with another.

pub(crate) struct DecodedSvg {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) rgba: Vec<u8>,
}

pub(crate) fn query_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    backend::query_dimensions(data)
}

pub(crate) fn decode(
    data: &[u8],
    max_width: u32,
    max_height: u32,
    max_texture_size: u32,
) -> Option<DecodedSvg> {
    backend::decode(data, max_width, max_height, max_texture_size)
}

fn constrained_dimensions(
    natural_width: f64,
    natural_height: f64,
    max_width: u32,
    max_height: u32,
    max_texture_size: u32,
) -> Option<(u32, u32)> {
    if !natural_width.is_finite()
        || !natural_height.is_finite()
        || natural_width <= 0.0
        || natural_height <= 0.0
    {
        return None;
    }

    let mut width = natural_width.ceil() as u32;
    let mut height = natural_height.ceil() as u32;
    let width_limit = if max_width > 0 {
        max_width.min(max_texture_size)
    } else {
        max_texture_size
    };
    let height_limit = if max_height > 0 {
        max_height.min(max_texture_size)
    } else {
        max_texture_size
    };

    if width > width_limit {
        height = (f64::from(height) * f64::from(width_limit) / f64::from(width)) as u32;
        width = width_limit;
    }
    if height > height_limit {
        width = (f64::from(width) * f64::from(height_limit) / f64::from(height)) as u32;
        height = height_limit;
    }

    Some((width.max(1), height.max(1)))
}

#[cfg(target_os = "linux")]
mod backend {
    use cairo::{Format, ImageSurface};
    use librsvg_rebind::prelude::*;
    use librsvg_rebind::{Handle, Length, Rectangle, Unit};

    use super::{DecodedSvg, constrained_dimensions};

    const DEFAULT_DPI: f64 = 96.0;

    pub(super) fn query_dimensions(data: &[u8]) -> Option<(u32, u32)> {
        let handle = Handle::from_data(data).ok()?;
        let (width, height) = natural_dimensions(&handle)?;
        Some((width.ceil() as u32, height.ceil() as u32))
    }

    pub(super) fn decode(
        data: &[u8],
        max_width: u32,
        max_height: u32,
        max_texture_size: u32,
    ) -> Option<DecodedSvg> {
        let handle = Handle::from_data(data).ok()?;
        let (natural_width, natural_height) = natural_dimensions(&handle)?;
        let (width, height) = constrained_dimensions(
            natural_width,
            natural_height,
            max_width,
            max_height,
            max_texture_size,
        )?;

        let mut surface = ImageSurface::create(Format::ARgb32, width as i32, height as i32).ok()?;
        let context = cairo::Context::new(&surface).ok()?;
        let viewport = Rectangle::new(0.0, 0.0, f64::from(width), f64::from(height));
        handle.render_document(&context, &viewport).ok()?;
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

    fn natural_dimensions(handle: &Handle) -> Option<(f64, f64)> {
        if let Some((width, height)) = handle.intrinsic_size_in_pixels()
            && valid_dimensions(width, height)
        {
            return Some((width, height));
        }

        let (intrinsic_width, intrinsic_height, view_box) = handle.intrinsic_dimensions();
        let explicit_width = absolute_length_in_pixels(&intrinsic_width);
        let explicit_height = absolute_length_in_pixels(&intrinsic_height);
        let view_box = view_box.filter(|rect| valid_dimensions(rect.width(), rect.height()));

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
        let viewport = Rectangle::new(0.0, 0.0, f64::from(u32::MAX), f64::from(u32::MAX));
        let (ink, _) = handle.geometry_for_layer(None, &viewport).ok()?;
        let width = ink.x() + ink.width();
        let height = ink.y() + ink.height();
        valid_dimensions(width, height).then_some((width, height))
    }

    fn absolute_length_in_pixels(length: &Length) -> Option<f64> {
        let value = length.length();
        match length.unit() {
            Unit::Px => Some(value),
            Unit::In => Some(value * DEFAULT_DPI),
            Unit::Cm => Some(value * DEFAULT_DPI / 2.54),
            Unit::Mm => Some(value * DEFAULT_DPI / 25.4),
            Unit::Pt => Some(value * DEFAULT_DPI / 72.0),
            Unit::Pc => Some(value * DEFAULT_DPI / 6.0),
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
}

#[cfg(not(target_os = "linux"))]
mod backend {
    use std::sync::{Arc, OnceLock};

    use resvg::usvg::fontdb;

    use super::{DecodedSvg, constrained_dimensions};

    static SVG_FONTDB: OnceLock<Arc<fontdb::Database>> = OnceLock::new();

    pub(super) fn query_dimensions(data: &[u8]) -> Option<(u32, u32)> {
        let tree = parse(data)?;
        let size = tree.size();
        Some((size.width().ceil() as u32, size.height().ceil() as u32))
    }

    pub(super) fn decode(
        data: &[u8],
        max_width: u32,
        max_height: u32,
        max_texture_size: u32,
    ) -> Option<DecodedSvg> {
        let tree = parse(data)?;
        let size = tree.size();
        let natural_width = f64::from(size.width());
        let natural_height = f64::from(size.height());
        let (width, height) = constrained_dimensions(
            natural_width,
            natural_height,
            max_width,
            max_height,
            max_texture_size,
        )?;

        let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)?;
        let transform = resvg::tiny_skia::Transform::from_scale(
            width as f32 / size.width(),
            height as f32 / size.height(),
        );
        resvg::render(&tree, transform, &mut pixmap.as_mut());

        let mut rgba = pixmap.take();
        for pixel in rgba.chunks_exact_mut(4) {
            let alpha = f32::from(pixel[3]) / 255.0;
            if alpha > 0.0 && alpha < 1.0 {
                pixel[0] = (f32::from(pixel[0]) / alpha).min(255.0) as u8;
                pixel[1] = (f32::from(pixel[1]) / alpha).min(255.0) as u8;
                pixel[2] = (f32::from(pixel[2]) / alpha).min(255.0) as u8;
            }
        }

        Some(DecodedSvg {
            width,
            height,
            rgba,
        })
    }

    fn parse(data: &[u8]) -> Option<resvg::usvg::Tree> {
        let mut options = resvg::usvg::Options::default();
        options.fontdb = font_database();
        resvg::usvg::Tree::from_data(data, &options).ok()
    }

    fn font_database() -> Arc<fontdb::Database> {
        SVG_FONTDB
            .get_or_init(|| {
                let mut database = fontdb::Database::new();
                database.load_system_fonts();
                if database.is_empty() {
                    tracing::warn!(
                        "SVG fontdb: no system fonts found; SVG text elements will not render"
                    );
                } else {
                    tracing::info!(
                        faces = database.len(),
                        "loaded system font faces for SVG rendering"
                    );
                }
                Arc::new(database)
            })
            .clone()
    }
}

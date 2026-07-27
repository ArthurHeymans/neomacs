//! Immutable image geometry shared by layout, async decoding, and rendering.

/// One resolved image realization for one frame presentation.
///
/// `layout_scale` maps GNU image pixels to Emacs logical pixels.
/// `device_scale` maps those logical pixels to physical texture pixels.  The
/// values travel together so pending layout, decoded metadata, and GPU upload
/// cannot consult different scale-factor snapshots.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ImageRealization {
    layout_scale_bits: u32,
    device_scale_bits: u32,
}

impl ImageRealization {
    #[must_use]
    pub fn new(layout_scale: f32, device_scale: f32) -> Self {
        let layout_scale = if layout_scale.is_finite() && layout_scale >= 0.0 {
            layout_scale
        } else {
            1.0
        };
        let device_scale = if device_scale.is_finite() && device_scale > 0.0 {
            device_scale
        } else {
            1.0
        };
        Self {
            // Normalize signed zero so equal numeric realizations have one
            // cache identity.
            layout_scale_bits: if layout_scale == 0.0 {
                0.0_f32.to_bits()
            } else {
                layout_scale.to_bits()
            },
            device_scale_bits: device_scale.to_bits(),
        }
    }

    #[must_use]
    pub fn layout_scale(self) -> f32 {
        f32::from_bits(self.layout_scale_bits)
    }

    #[must_use]
    pub fn device_scale(self) -> f32 {
        f32::from_bits(self.device_scale_bits)
    }

    /// Convert a GNU image dimension to integer logical layout pixels.
    #[must_use]
    pub fn layout_dimension(self, dimension: u32) -> u32 {
        ((f64::from(dimension) * f64::from(self.layout_scale()))
            .round()
            .max(1.0)) as u32
    }

    /// Convert an integer logical extent to physical texture pixels.
    #[must_use]
    pub fn raster_dimension(self, layout_dimension: u32) -> u32 {
        ((f64::from(layout_dimension) * f64::from(self.device_scale()))
            .ceil()
            .max(1.0)) as u32
    }
}

impl Default for ImageRealization {
    fn default() -> Self {
        Self::new(1.0, 1.0)
    }
}

/// How large an image should be drawn, as asked for by its spec.
///
/// This is GNU's `compute_image_size` input set (src/image.c:2750). The four
/// keys are NOT interchangeable, which is easy to get wrong:
///
/// - `:width` / `:height` are TARGETS. They are themselves multiplied by the
///   scale, and each one overrides its `:max-` counterpart.
/// - `:max-width` / `:max-height` are CLAMPS, applied only after the target (or
///   the scaled native size) is known, and always preserving the aspect ratio.
///
/// Collapsing `:width` into `:max-width` loses both the target semantics and
/// the aspect ratio — it is what made `(image-size '(image :max-width 20 ...))`
/// answer `(20 . 4096)` where GNU answers `(20 . 10)`.
///
/// The size cannot be resolved until the native size is known, i.e. after
/// decoding, so this travels to the decoder rather than being applied up front.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ImageSizeSpec {
    width: Option<u32>,
    height: Option<u32>,
    max_width: Option<u32>,
    max_height: Option<u32>,
}

impl ImageSizeSpec {
    #[must_use]
    pub fn with_width(mut self, width: Option<u32>) -> Self {
        self.width = width;
        self
    }

    #[must_use]
    pub fn with_height(mut self, height: Option<u32>) -> Self {
        self.height = height;
        self
    }

    #[must_use]
    pub fn with_max_width(mut self, max_width: Option<u32>) -> Self {
        self.max_width = max_width;
        self
    }

    #[must_use]
    pub fn with_max_height(mut self, max_height: Option<u32>) -> Self {
        self.max_height = max_height;
        self
    }

    /// The size a `native_width` x `native_height` image should be drawn at.
    ///
    /// Mirrors GNU `compute_image_size` (src/image.c:2750) step for step.
    #[must_use]
    pub fn desired(self, native_width: u32, native_height: u32, scale: f64) -> (u32, u32) {
        let native_width = native_width.max(1);
        let native_height = native_height.max(1);

        // ":width overrides :max-width" and likewise for height
        // (src/image.c:2767, 2777).
        let (target_width, max_width) = match self.width {
            Some(width) => (Some(scale_size(width, 1, scale)), None),
            None => (None, self.max_width),
        };
        let (target_height, max_height) = match self.height {
            Some(height) => (Some(scale_size(height, 1, scale)), None),
            None => (None, self.max_height),
        };

        let (mut width, mut height) = match (target_width, target_height) {
            // Both given: GNU skips the aspect-preserving work entirely.
            (Some(width), Some(height)) => return (width.max(1), height.max(1)),
            (Some(width), None) => (width, ratio(width, native_width, native_height)),
            (None, Some(height)) => (ratio(height, native_height, native_width), height),
            (None, None) => (
                scale_size(native_width, 1, scale),
                scale_size(native_height, 1, scale),
            ),
        };

        // Clamps, each preserving the aspect ratio (src/image.c:2798-2810).
        if let Some(max) = max_width.filter(|max| *max < width) {
            width = max;
            height = ratio(width, native_width, native_height);
        }
        if let Some(max) = max_height.filter(|max| *max < height) {
            height = max;
            width = ratio(height, native_height, native_width);
        }

        (width.max(1), height.max(1))
    }
}

/// GNU `scale_image_size` (src/image.c:2700): `size * divisor_reciprocal`,
/// expressed as `size * multiplier / divisor` with saturating conversion.
fn scale_size(size: u32, divisor: u32, multiplier: f64) -> u32 {
    let scaled = f64::from(size) * multiplier / f64::from(divisor.max(1));
    if scaled.is_finite() && scaled >= 1.0 {
        scaled.round() as u32
    } else {
        1
    }
}

/// Keep the aspect ratio: `size * to / from`.
fn ratio(size: u32, from: u32, to: u32) -> u32 {
    scale_size(size, from, f64::from(to))
}

#[cfg(test)]
mod tests {
    use super::{ImageRealization, ImageSizeSpec};

    /// Every expectation below was measured from GNU Emacs 31 on a 40x20 PNG
    /// with `image-scaling-factor` pinned to 1, so the numbers are observed
    /// rather than derived from reading `compute_image_size`.
    const NATIVE: (u32, u32) = (40, 20);

    fn desired(spec: ImageSizeSpec, scale: f64) -> (u32, u32) {
        spec.desired(NATIVE.0, NATIVE.1, scale)
    }

    #[test]
    fn native_size_survives_when_nothing_is_requested() {
        assert_eq!(desired(ImageSizeSpec::default(), 1.0), (40, 20));
    }

    #[test]
    fn scale_multiplies_both_axes() {
        assert_eq!(desired(ImageSizeSpec::default(), 2.0), (80, 40));
    }

    #[test]
    fn width_is_a_target_and_keeps_the_aspect_ratio() {
        // GNU: `:width 80` => (80 . 40) — the height follows from the ratio.
        assert_eq!(
            desired(ImageSizeSpec::default().with_width(Some(80)), 1.0),
            (80, 40)
        );
    }

    #[test]
    fn height_is_a_target_and_keeps_the_aspect_ratio() {
        // GNU: `:height 40` => (80 . 40).
        assert_eq!(
            desired(ImageSizeSpec::default().with_height(Some(40)), 1.0),
            (80, 40)
        );
    }

    #[test]
    fn max_width_clamps_and_keeps_the_aspect_ratio() {
        // GNU: `:max-width 20` => (20 . 10), NOT (20 . <unbounded>).
        assert_eq!(
            desired(ImageSizeSpec::default().with_max_width(Some(20)), 1.0),
            (20, 10)
        );
    }

    #[test]
    fn width_overrides_max_width() {
        // GNU: `:width 80 :max-width 20` => (80 . 40). ":width overrides
        // :max-width" (src/image.c:2767) — conflating the two keys is what
        // made neomacs answer (20 . 4096).
        assert_eq!(
            desired(
                ImageSizeSpec::default()
                    .with_width(Some(80))
                    .with_max_width(Some(20)),
                1.0
            ),
            (80, 40)
        );
    }

    #[test]
    fn explicit_width_and_height_skip_the_aspect_computation() {
        assert_eq!(
            desired(
                ImageSizeSpec::default()
                    .with_width(Some(11))
                    .with_height(Some(99)),
                1.0
            ),
            (11, 99)
        );
    }

    #[test]
    fn targets_are_themselves_scaled() {
        // GNU multiplies :width/:height by the scale (src/image.c:2766).
        assert_eq!(
            desired(ImageSizeSpec::default().with_width(Some(40)), 2.0),
            (80, 40)
        );
    }

    #[test]
    fn fractional_realization_has_one_layout_and_raster_rounding_policy() {
        let realization = ImageRealization::new(1.3 / 1.75, 1.75);

        assert_eq!(realization.layout_dimension(24), 18);
        assert_eq!(realization.raster_dimension(18), 32);
    }
}

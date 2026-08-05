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

/// What a spec asks for along one axis.
///
/// GNU resolves `:width` vs `:max-width` by precedence — ":width overrides
/// :max-width" (src/image.c:2767) — which means the two can never both apply.
/// Making that a sum type retires the bug this replaces: the old code kept one
/// `max_width` field that BOTH keys wrote into, so a target silently became a
/// clamp and the aspect ratio was computed against the wrong number.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AxisSize {
    /// Nothing requested: the native extent, scaled.
    #[default]
    Native,
    /// `:width` / `:height` — an exact target, itself multiplied by the scale.
    Exact(u32),
    /// `:max-width` / `:max-height` — an upper bound; the other axis follows to
    /// preserve the aspect ratio.
    AtMost(u32),
}

impl AxisSize {
    /// Apply GNU's precedence once, at construction: a target wins over a clamp.
    #[must_use]
    pub fn resolve(target: Option<u32>, at_most: Option<u32>) -> Self {
        match (target, at_most) {
            (Some(target), _) => Self::Exact(target),
            (None, Some(at_most)) => Self::AtMost(at_most),
            (None, None) => Self::Native,
        }
    }

    fn target(self, scale: f64) -> Option<u32> {
        match self {
            // GNU scales the target too (src/image.c:2766).
            Self::Exact(size) => Some(scale_size(size, 1, scale)),
            _ => None,
        }
    }

    /// The extent this axis pins, if any. `Native` pins nothing — the answer
    /// is not knowable until the image is decoded.
    #[must_use]
    pub fn pinned(self) -> Option<u32> {
        match self {
            Self::Exact(size) | Self::AtMost(size) => Some(size),
            Self::Native => None,
        }
    }

    fn at_most(self) -> Option<u32> {
        match self {
            Self::AtMost(size) => Some(size),
            _ => None,
        }
    }
}

/// A quarter-turn rotation, the only kind native transforms perform.
///
/// GNU reduces `:rotation` modulo 360 and then rotates only on an exact
/// multiple of 90 (src/image.c:2927, 3144-3203); every other angle, and any
/// non-number, leaves the image upright. Modelling the reduced angle instead of
/// carrying raw degrees means the decoder cannot be handed an angle it has no
/// branch for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ImageRotation {
    #[default]
    None,
    Quarter,
    Half,
    ThreeQuarter,
}

impl ImageRotation {
    /// GNU's `compute_image_rotation` followed by its 90-degree dispatch.
    #[must_use]
    pub fn from_degrees(degrees: f64) -> Self {
        if !degrees.is_finite() {
            return Self::None;
        }
        // Emacs `mod` takes the sign of the divisor, so -90 reduces to 270.
        let reduced = degrees.rem_euclid(360.0);
        match reduced {
            90.0 => Self::Quarter,
            180.0 => Self::Half,
            270.0 => Self::ThreeQuarter,
            _ => Self::None,
        }
    }

    /// Whether the rotation exchanges width and height (src/image.c:3171).
    #[must_use]
    pub fn swaps_axes(self) -> bool {
        matches!(self, Self::Quarter | Self::ThreeQuarter)
    }

    /// Apply GNU's axis exchange to an already-sized extent.
    #[must_use]
    pub fn orient(self, width: u32, height: u32) -> (u32, u32) {
        if self.swaps_axes() {
            (height, width)
        } else {
            (width, height)
        }
    }
}

/// How large an image should be drawn, as asked for by its spec.
///
/// This is GNU's `compute_image_size` input set (src/image.c:2750). The size
/// cannot be resolved until the native size is known, i.e. after decoding, so
/// this travels to the decoder rather than being applied up front.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ImageSizeSpec {
    width: AxisSize,
    height: AxisSize,
}

impl ImageSizeSpec {
    #[must_use]
    pub fn new(width: AxisSize, height: AxisSize) -> Self {
        Self { width, height }
    }

    /// Size to reserve for an image that has not been decoded yet.
    ///
    /// `None` when neither axis is pinned: the native size is the answer then,
    /// and it is not known until decoding finishes.
    #[must_use]
    pub fn placeholder_extent(self) -> Option<(u32, u32)> {
        match (self.width.pinned(), self.height.pinned()) {
            (Some(width), Some(height)) => Some((width, height)),
            // One axis pinned: the other follows the native aspect ratio, which
            // is still unknown, so fall back to a square of the known extent.
            (Some(width), None) => Some((width, width)),
            (None, Some(height)) => Some((height, height)),
            (None, None) => None,
        }
    }

    /// The size a `native_width` x `native_height` image should be drawn at.
    ///
    /// Mirrors GNU `compute_image_size` (src/image.c:2750) step for step.
    #[must_use]
    pub fn desired(self, native_width: u32, native_height: u32, scale: f64) -> (u32, u32) {
        let native_width = native_width.max(1);
        let native_height = native_height.max(1);

        let (mut width, mut height) = match (self.width.target(scale), self.height.target(scale)) {
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
        if let Some(max) = self.width.at_most().filter(|max| *max < width) {
            width = max;
            height = ratio(width, native_width, native_height);
        }
        if let Some(max) = self.height.at_most().filter(|max| *max < height) {
            height = max;
            width = ratio(height, native_height, native_width);
        }

        (width.max(1), height.max(1))
    }
}

/// GNU `scale_image_size` (src/image.c:2700): `size * multiplier / divisor`.
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
    use super::{AxisSize, ImageRealization, ImageRotation, ImageSizeSpec};

    /// Every expectation below was measured from GNU Emacs 31 on a 40x20 PNG
    /// with `image-scaling-factor` pinned to 1, so the numbers are observed
    /// rather than derived from reading `compute_image_size`.
    const NATIVE: (u32, u32) = (40, 20);

    fn desired(spec: ImageSizeSpec, scale: f64) -> (u32, u32) {
        spec.desired(NATIVE.0, NATIVE.1, scale)
    }

    /// Measured from GNU Emacs 31 on the same 40x20 PNG.
    #[test]
    fn rotation_reduces_modulo_360_and_only_turns_on_multiples_of_90() {
        use ImageRotation as R;
        for (degrees, expected) in [
            (0.0, R::None),
            (90.0, R::Quarter),
            (180.0, R::Half),
            (270.0, R::ThreeQuarter),
            (360.0, R::None),
            (450.0, R::Quarter),
            // Emacs `mod` takes the divisor's sign: -90 reduces to 270.
            (-90.0, R::ThreeQuarter),
            // Not a multiple of 90: GNU leaves the image upright.
            (45.0, R::None),
            (f64::NAN, R::None),
        ] {
            assert_eq!(R::from_degrees(degrees), expected, "rotation {degrees}");
        }
    }

    #[test]
    fn quarter_turns_exchange_the_axes() {
        // GNU: 40x20 with `:rotation 90` reports (20 . 40).
        assert_eq!(ImageRotation::Quarter.orient(40, 20), (20, 40));
        assert_eq!(ImageRotation::ThreeQuarter.orient(40, 20), (20, 40));
        assert_eq!(ImageRotation::Half.orient(40, 20), (40, 20));
        assert_eq!(ImageRotation::None.orient(40, 20), (40, 20));
    }

    #[test]
    fn sizing_happens_before_rotation() {
        // GNU: `:rotation 90 :width 80` on 40x20 reports (40 . 80) — `:width`
        // sizes the upright image (80x40), then the turn swaps the axes.
        let sized = ImageSizeSpec::new(AxisSize::Exact(80), AxisSize::Native).desired(40, 20, 1.0);
        assert_eq!(sized, (80, 40));
        assert_eq!(ImageRotation::Quarter.orient(sized.0, sized.1), (40, 80));
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
            desired(
                ImageSizeSpec::new(AxisSize::Exact(80), AxisSize::Native),
                1.0
            ),
            (80, 40)
        );
    }

    #[test]
    fn height_is_a_target_and_keeps_the_aspect_ratio() {
        // GNU: `:height 40` => (80 . 40).
        assert_eq!(
            desired(
                ImageSizeSpec::new(AxisSize::Native, AxisSize::Exact(40)),
                1.0
            ),
            (80, 40)
        );
    }

    #[test]
    fn max_width_clamps_and_keeps_the_aspect_ratio() {
        // GNU: `:max-width 20` => (20 . 10), NOT (20 . <unbounded>).
        assert_eq!(
            desired(
                ImageSizeSpec::new(AxisSize::AtMost(20), AxisSize::Native),
                1.0
            ),
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
                // GNU precedence resolved at construction: the target wins.
                ImageSizeSpec::new(AxisSize::resolve(Some(80), Some(20)), AxisSize::Native),
                1.0
            ),
            (80, 40)
        );
    }

    #[test]
    fn explicit_width_and_height_skip_the_aspect_computation() {
        assert_eq!(
            desired(
                ImageSizeSpec::new(AxisSize::Exact(11), AxisSize::Exact(99)),
                1.0
            ),
            (11, 99)
        );
    }

    #[test]
    fn targets_are_themselves_scaled() {
        // GNU multiplies :width/:height by the scale (src/image.c:2766).
        assert_eq!(
            desired(
                ImageSizeSpec::new(AxisSize::Exact(40), AxisSize::Native),
                2.0
            ),
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

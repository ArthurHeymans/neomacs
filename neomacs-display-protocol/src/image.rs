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

#[cfg(test)]
mod tests {
    use super::ImageRealization;

    #[test]
    fn fractional_realization_has_one_layout_and_raster_rounding_policy() {
        let realization = ImageRealization::new(1.3 / 1.75, 1.75);

        assert_eq!(realization.layout_dimension(24), 18);
        assert_eq!(realization.raster_dimension(18), 32);
    }
}

//! Nonblocking image lookup used by redisplay.
//!
//! Image decoding and renderer upload happen outside the evaluator thread.
//! Callers receive a complete state immediately and never infer readiness
//! from optional metadata.

use crate::heap_types::LispString;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ImageResolveSource {
    File(LispString),
    Data(Vec<u8>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ImageResolveRequest {
    pub source: ImageResolveSource,
    pub max_width: u32,
    pub max_height: u32,
    pub fg_color: u32,
    pub bg_color: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedImageMetadata {
    pub width: u32,
    pub height: u32,
    /// GNU's decoded four-corner background guess (0x00RRGGBB).
    pub background: u32,
    /// GNU's decoded four-corner mask classification.
    pub background_transparent: bool,
}

/// Decoded image whose intrinsic metadata is available for layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadyImage {
    pub image_id: u32,
    pub metadata: ResolvedImageMetadata,
}

/// Stable renderer identity and layout slot for an image lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImagePlacement {
    image_id: u32,
    width: u32,
    height: u32,
}

impl ImagePlacement {
    #[must_use]
    pub const fn new(image_id: u32, width: u32, height: u32) -> Self {
        Self {
            image_id,
            width,
            height,
        }
    }

    #[must_use]
    pub const fn image_id(self) -> u32 {
        self.image_id
    }

    #[must_use]
    pub const fn width(self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(self) -> u32 {
        self.height
    }
}

/// Stable placeholder geometry while an image is decoded asynchronously.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingImage {
    placement: ImagePlacement,
}

/// Stable failed state retaining the slot that was allocated while pending.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FailedImage {
    placement: ImagePlacement,
    pub error: String,
}

impl PendingImage {
    #[must_use]
    pub const fn new(image_id: u32, width: u32, height: u32) -> Self {
        Self {
            placement: ImagePlacement::new(image_id, width, height),
        }
    }

    #[must_use]
    pub const fn placement(&self) -> ImagePlacement {
        self.placement
    }

    #[must_use]
    pub fn failed(self, error: String) -> FailedImage {
        FailedImage {
            placement: self.placement,
            error,
        }
    }
}

impl FailedImage {
    #[must_use]
    pub const fn placement(&self) -> ImagePlacement {
        self.placement
    }
}

/// Result of a nonblocking image catalog lookup.
///
/// Every non-ready state retains stable placement geometry, so asynchronous
/// completion or failure never changes a published frame in place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageLookup {
    Ready(ReadyImage),
    Pending(PendingImage),
    Failed(FailedImage),
}

impl ImageLookup {
    /// Return the stable renderer identity and dimensions represented by this
    /// state. Ready images use decoded dimensions; pending and failed images
    /// retain their placeholder slot.
    #[must_use]
    pub const fn placement(&self) -> ImagePlacement {
        match self {
            Self::Ready(image) => {
                ImagePlacement::new(image.image_id, image.metadata.width, image.metadata.height)
            }
            Self::Pending(image) => image.placement(),
            Self::Failed(image) => image.placement(),
        }
    }

    #[must_use]
    pub const fn ready_metadata(&self) -> Option<&ResolvedImageMetadata> {
        match self {
            Self::Ready(image) => Some(&image.metadata),
            Self::Pending(_) | Self::Failed(_) => None,
        }
    }
}

/// Catalog seam used by redisplay to schedule or inspect image work.
pub trait ImageCatalog {
    /// Return the current state immediately. A cache miss schedules decoding
    /// and returns [`ImageLookup::Pending`]. Implementations must not wait for
    /// renderer queue capacity, metadata locks, file I/O, decode, or upload.
    fn lookup(&self, request: ImageResolveRequest) -> ImageLookup;
}

use std::fmt;
use std::num::NonZeroU32;

use crate::Frontend;

/// Non-zero pixel dimensions of the video presentation exercised by the
/// physical-display benchmark.
///
/// A native-display GUI cannot be sized hermetically by the off-screen GUI
/// adapter.  The benchmark therefore treats the GUI dimensions as its video
/// presentation contract and verifies that the real window can contain it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct NativeVideoPresentationTarget {
    width: NonZeroU32,
    height: NonZeroU32,
}

impl NativeVideoPresentationTarget {
    pub(crate) fn from_frontend(frontend: Frontend) -> Result<Self, InvalidPresentationTarget> {
        let Frontend::Gui { width, height } = frontend else {
            return Err(InvalidPresentationTarget::NotGui);
        };
        Ok(Self {
            width: NonZeroU32::new(width).ok_or(InvalidPresentationTarget::ZeroWidth)?,
            height: NonZeroU32::new(height).ok_or(InvalidPresentationTarget::ZeroHeight)?,
        })
    }

    pub(crate) const fn width(self) -> u32 {
        self.width.get()
    }

    pub(crate) const fn height(self) -> u32 {
        self.height.get()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InvalidPresentationTarget {
    NotGui,
    ZeroWidth,
    ZeroHeight,
}

impl fmt::Display for InvalidPresentationTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NotGui => "sustained native-video performance requires the GUI frontend",
            Self::ZeroWidth => "native-video presentation width must be non-zero",
            Self::ZeroHeight => "native-video presentation height must be non-zero",
        })
    }
}

impl std::error::Error for InvalidPresentationTarget {}

#[cfg(test)]
#[path = "native_video_test.rs"]
mod tests;

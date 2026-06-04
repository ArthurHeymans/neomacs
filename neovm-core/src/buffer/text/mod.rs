pub(in crate::buffer) mod backend;
mod edit;
mod kind;
#[cfg(test)]
mod layout;
mod metrics;

pub use edit::{TextEditRange, TextExtent, TextInsertion};
pub use kind::BufferTextBackendKind;
pub(crate) use kind::ImplementedBufferTextBackendKind;
#[cfg(test)]
pub use layout::{GapDebugLayout, TextBackendDebugLayout};
pub use metrics::TextMetrics;

#[inline]
pub(crate) fn emacs_char_count_bytes(bytes: &[u8], multibyte: bool) -> usize {
    if multibyte {
        crate::emacs_core::emacs_char::chars_in_multibyte(bytes)
    } else {
        bytes.len()
    }
}

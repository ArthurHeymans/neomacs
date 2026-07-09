//! WPE WebKit backend for headless browser rendering.
//!
//! This module provides WPE WebKit integration for embedding web content
//! in Emacs buffers using the WPE Platform API for GPU-accelerated rendering.
//!
//! Architecture:
//! - WPE Platform API: Modern display/view/buffer abstraction
//! - wpe-webkit: WebKit engine (GObject API)
//! - dma-buf: Zero-copy GPU buffer sharing

// The WPE submodules are FFI-heavy: `sys` is bindgen-generated and `backend`/`view`
// wrap raw WPE/GObject C calls whose `unsafe fn` bodies call into C without an inner
// `unsafe {}` block. Scoped here (feature-gated module) instead of crate-wide.
#![allow(unsafe_op_in_unsafe_fn)]

#[cfg(feature = "wpe-webkit")]
pub(crate) mod sys;

#[cfg(feature = "wpe-webkit")]
mod platform;

#[cfg(feature = "wpe-webkit")]
mod backend;

#[cfg(feature = "wpe-webkit")]
mod view;

#[cfg(feature = "wpe-webkit")]
mod dmabuf;

#[cfg(feature = "wpe-webkit")]
mod view_cache;

#[cfg(feature = "wpe-webkit")]
pub use backend::WpeBackend;

#[cfg(feature = "wpe-webkit")]
pub use view_cache::WebKitViewCache;

#[cfg(feature = "wpe-webkit")]
pub use view::{
    DmaBufData, LoadCallback, NewWindowCallback, RawPixelData, WpeViewState, WpeWebView,
    set_load_callback, set_new_window_callback,
};

#[cfg(feature = "wpe-webkit")]
pub use dmabuf::{DmaBufExporter, ExportedDmaBuf};

#[cfg(feature = "wpe-webkit")]
pub use platform::{WpePlatformDisplay, WpePlatformView};

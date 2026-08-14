//! WGPU renderer primitives shared by display backends.

// Renderer entry points and GPU-pipeline builders take many positional
// parameters (geometry, colors, atlas/pipeline handles); folding them into
// structs is a separate refactor, so this bulk category is allowed crate-wide.
#![allow(clippy::too_many_arguments)]

pub mod external_buffer;
pub mod frame_post;
pub mod glyph_atlas;
pub mod image_cache;
pub mod media_budget;
pub mod overlay_state;
pub mod renderer;
pub mod shader_surface;
pub mod shader_surface_cache;
mod svg;
pub mod vertex;
pub mod xbm;
pub mod xpm;

#[cfg(feature = "video")]
pub mod video_cache;

#[cfg(all(feature = "wpe-webkit", target_os = "linux"))]
pub mod webkit_cache;

#[cfg(all(feature = "video-dmabuf", target_os = "linux"))]
pub mod va_dmabuf_export;

#[cfg(all(
    any(feature = "video-dmabuf", feature = "wpe-webkit"),
    target_os = "linux"
))]
pub mod vulkan_dmabuf;

#[cfg(target_os = "linux")]
pub use external_buffer::DmaBufBuffer;
pub use external_buffer::{BufferFormat, ExternalBuffer, PlatformBuffer, SharedMemoryBuffer};
pub use glyph_atlas::{
    ComposedGlyphKey, GlyphAtlasHandle, GlyphKey, RasterizeResult, WgpuGlyphAtlas, allocator,
    pages, types,
};
pub use image_cache::{
    CachedImage, ImageCache, ImageDecodeOutcome, ImageDimensions, ImageMetadata, ImageState,
};
pub use overlay_state::{MenuPanel, PopupMenuState, TooltipState};
pub use renderer::{
    FrameRowDamage, FrameSampleTime, RendererFrameEffects, RowDamageInfo, RowReuseStats,
    WgpuRenderer, WindowRowDamage,
};
pub use shader_surface::{
    SURFACE_USER_UNIFORM_SLOTS, SurfaceUniformInit, compose_surface_wgsl, validate_surface_wgsl,
};
pub use shader_surface_cache::{MAX_SURFACE_SIZE, ShaderSurfaceCache};
pub use vertex::{GlyphVertex, RectVertex, RoundedRectVertex, TextureVertex, Uniforms};
#[cfg(feature = "video")]
pub use video_cache::{CachedVideo, DecodedFrame, VideoCache, VideoState};
#[cfg(all(feature = "wpe-webkit", target_os = "linux"))]
pub use webkit_cache::{CachedWebKitView, WgpuWebKitCache};

/// Re-exported effect configuration module for renderer internals and callers.
pub mod effect_config {
    pub use neomacs_display_protocol::effect_config::*;
}

/// Read GPU power preference from `NEOMACS_GPU`.
pub fn gpu_power_preference() -> wgpu::PowerPreference {
    match std::env::var("NEOMACS_GPU").as_deref() {
        Ok("low") | Ok("integrated") => wgpu::PowerPreference::LowPower,
        Ok("high") | Ok("discrete") => wgpu::PowerPreference::HighPerformance,
        _ => wgpu::PowerPreference::HighPerformance,
    }
}

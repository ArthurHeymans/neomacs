//! Grouped GPU resources owned by `WgpuRenderer`: render pipelines, the
//! stencil clip targets, texture/media caches, and per-frame vertex arenas.

use super::super::image_cache::ImageCache;
#[cfg(feature = "video")]
use super::super::video_cache::VideoCache;
#[cfg(feature = "wpe-webkit")]
use super::super::webkit_cache::WgpuWebKitCache;
use super::dynamic_buffer::FrameVertexArena;
use crate::vertex::{GlyphVertex, SubpixelGlyphVertex};

/// All render pipelines. The `stencil_*` variants are identical to their base
/// counterparts except for stencil state; they draw only where the stencil
/// buffer was written (child frame rounded-corner clipping).
pub(crate) struct Pipelines {
    pub(crate) rect: wgpu::RenderPipeline,
    pub(crate) rounded_rect: wgpu::RenderPipeline,
    pub(crate) corner_mask: wgpu::RenderPipeline,
    pub(crate) glyph: wgpu::RenderPipeline,
    pub(crate) subpixel_glyph: wgpu::RenderPipeline,
    pub(crate) image: wgpu::RenderPipeline,
    pub(crate) opaque_image: wgpu::RenderPipeline,
    pub(crate) stencil_rect: wgpu::RenderPipeline,
    pub(crate) stencil_rounded_rect: wgpu::RenderPipeline,
    pub(crate) stencil_glyph: wgpu::RenderPipeline,
    pub(crate) stencil_subpixel_glyph: wgpu::RenderPipeline,
    pub(crate) stencil_image: wgpu::RenderPipeline,
    pub(crate) stencil_opaque_image: wgpu::RenderPipeline,
    pub(crate) stencil_write: wgpu::RenderPipeline,
}

/// Stencil texture/view used to clip child frames to rounded corners.
/// Recreated on resize.
pub(crate) struct StencilTargets {
    pub(crate) texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
}

/// Texture/media caches.
pub(crate) struct RenderCaches {
    pub(crate) image: ImageCache,
    #[cfg(feature = "video")]
    pub(crate) video: VideoCache,
    #[cfg(feature = "wpe-webkit")]
    pub(crate) webkit: WgpuWebKitCache,
}

/// Per-frame reusable vertex upload arenas.
pub(crate) struct VertexArenas {
    pub(crate) glyph: FrameVertexArena<GlyphVertex>,
    pub(crate) subpixel: FrameVertexArena<SubpixelGlyphVertex>,
    pub(crate) image: FrameVertexArena<GlyphVertex>,
}

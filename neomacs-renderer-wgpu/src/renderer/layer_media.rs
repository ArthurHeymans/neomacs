//! Inline media draw phases of `render_frame_glyphs` (z-order step 7):
//! images, videos, and WebKit xwidget views.

use neomacs_display_protocol::frame_glyphs::FrameGlyph;

use super::super::vertex::{GlyphVertex, RectVertex};
use super::WgpuRenderer;
#[cfg(feature = "video")]
use super::frame_pass::FrameParams;
use super::frame_pass::FramePassCtx;

/// A textured quad gathered for a batched arena draw: the cache id to bind
/// plus its six vertices. All quads of a phase upload as one arena region;
/// each draws its own range so the draw sequence matches the gather order.
pub(super) struct MediaQuad<Id> {
    pub(super) id: Id,
    pub(super) vertices: [GlyphVertex; 6],
}

/// Untinted (white) textured quad spanning the full u range and the given
/// (possibly clip-trimmed) v range.
// Default features use this only from test and feature-gated video/WebKit paths.
#[allow(dead_code)]
pub(super) fn textured_quad_vertices(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    tex_v_min: f32,
    tex_v_max: f32,
) -> [GlyphVertex; 6] {
    textured_quad_vertices_uv(x, y, width, height, 0.0, 1.0, tex_v_min, tex_v_max)
}

pub(super) fn textured_quad_vertices_uv(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    tex_u_min: f32,
    tex_u_max: f32,
    tex_v_min: f32,
    tex_v_max: f32,
) -> [GlyphVertex; 6] {
    let white = [1.0, 1.0, 1.0, 1.0];
    [
        GlyphVertex {
            position: [x, y],
            tex_coords: [tex_u_min, tex_v_min],
            color: white,
        },
        GlyphVertex {
            position: [x + width, y],
            tex_coords: [tex_u_max, tex_v_min],
            color: white,
        },
        GlyphVertex {
            position: [x + width, y + height],
            tex_coords: [tex_u_max, tex_v_max],
            color: white,
        },
        GlyphVertex {
            position: [x, y],
            tex_coords: [tex_u_min, tex_v_min],
            color: white,
        },
        GlyphVertex {
            position: [x + width, y + height],
            tex_coords: [tex_u_max, tex_v_max],
            color: white,
        },
        GlyphVertex {
            position: [x, y + height],
            tex_coords: [tex_u_min, tex_v_max],
            color: white,
        },
    ]
}

impl WgpuRenderer {
    /// Draw inline images on top of text.
    pub(super) fn draw_inline_images(&mut self, ctx: &mut FramePassCtx<'_, '_>) {
        let frame_glyphs = ctx.params.frame_glyphs;
        // Gather quads for images with a ready texture (same skip logic the
        // per-quad draw used), then upload once and draw per-image ranges.
        let mut quads = Vec::new();
        let mut relief_vertices: Vec<RectVertex> = Vec::new();

        for (glyph_index, glyph) in frame_glyphs.glyphs.iter().enumerate() {
            if let FrameGlyph::Image {
                image_id,
                source_rect,
                x,
                y,
                width,
                height,
                clip_rect,
                ..
            } = glyph
            {
                let effective_clip = *clip_rect;
                let (
                    draw_x,
                    draw_y,
                    clipped_width,
                    clipped_height,
                    tex_u_min,
                    tex_u_max,
                    tex_v_min,
                    tex_v_max,
                ) = if let Some(clip) = &effective_clip {
                    let mut x0 = *x;
                    let mut y0 = *y;
                    let mut w0 = *width;
                    let mut h0 = *height;
                    let mut u0 = 0.0_f32;
                    let mut u1 = 1.0_f32;
                    let mut v0 = 0.0_f32;
                    let mut v1 = 1.0_f32;
                    let left = clip.x;
                    let right = clip.x + clip.width;
                    let top = clip.y;
                    let bottom = clip.y + clip.height;
                    if x0 < left {
                        let cut = left - x0;
                        if cut >= w0 {
                            continue;
                        }
                        x0 = left;
                        w0 -= cut;
                        if *width > 0.0 {
                            u0 += cut / *width;
                        }
                    }
                    if x0 + w0 > right {
                        let cut = (x0 + w0) - right;
                        if cut >= w0 {
                            continue;
                        }
                        w0 -= cut;
                        if *width > 0.0 {
                            u1 -= cut / *width;
                        }
                    }
                    if y0 < top {
                        let cut = top - y0;
                        if cut >= h0 {
                            continue;
                        }
                        y0 = top;
                        h0 -= cut;
                        if *height > 0.0 {
                            v0 += cut / *height;
                        }
                    }
                    if y0 + h0 > bottom {
                        let cut = (y0 + h0) - bottom;
                        if cut >= h0 {
                            continue;
                        }
                        h0 -= cut;
                        if *height > 0.0 {
                            v1 -= cut / *height;
                        }
                    }
                    (x0, y0, w0, h0, u0, u1, v0, v1)
                } else {
                    (*x, *y, *width, *height, 0.0, 1.0, 0.0, 1.0)
                };

                // Skip if fully clipped
                if clipped_width <= 0.0 || clipped_height <= 0.0 {
                    continue;
                }
                let (tex_u_min, tex_v_min) = source_rect.map_uv(tex_u_min, tex_v_min);
                let (tex_u_max, tex_v_max) = source_rect.map_uv(tex_u_max, tex_v_max);

                tracing::debug!(
                    "Rendering image {} at ({}, {}) size {}x{} (clipped to {})",
                    image_id,
                    x,
                    y,
                    width,
                    height,
                    clipped_height
                );
                // Check if image texture is ready
                if self.caches.image.get(image_id.get()).is_some() {
                    self.media_budget
                        .touch(crate::media_budget::MediaType::Image, image_id.get());
                    // Create vertices for image quad (white color = no tinting)
                    quads.push(MediaQuad {
                        id: image_id.get(),
                        vertices: textured_quad_vertices_uv(
                            draw_x,
                            draw_y,
                            clipped_width,
                            clipped_height,
                            tex_u_min,
                            tex_u_max,
                            tex_v_min,
                            tex_v_max,
                        ),
                    });
                    if let Some(override_paint) =
                        ctx.params.pointer_override.image_override(glyph_index)
                        && let neomacs_display_protocol::PointerDrawMode::ImageRelief(relief) =
                            override_paint.mode()
                    {
                        let relief_clip = ctx
                            .params
                            .pointer_override
                            .image_clip(glyph_index, clip_rect.as_ref());
                        super::pointer_override::append_clipped_relief(
                            &mut relief_vertices,
                            *x,
                            *y,
                            *width,
                            *height,
                            relief,
                            relief_clip.as_ref(),
                        );
                    }
                }
            }
        }

        let all_vertices: Vec<GlyphVertex> = quads
            .iter()
            .flat_map(|quad| quad.vertices.iter().copied())
            .collect();
        let upload = self
            .arenas
            .image
            .upload(&self.device, &self.queue, &all_vertices);

        // Pipeline + uniforms are set even with zero images: the inline-video
        // phase that follows inherits this pipeline state.
        let render_pass = &mut ctx.pass;
        render_pass.set_pipeline(&self.pipelines.image);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        let Some(upload) = upload else {
            return;
        };
        render_pass.set_vertex_buffer(0, upload.buffer_slice());
        for (i, quad) in quads.iter().enumerate() {
            if let Some(cached) = self.caches.image.get(quad.id) {
                render_pass.set_bind_group(1, &cached.bind_group, &[]);
                render_pass.draw((i * 6) as u32..(i * 6 + 6) as u32, 0..1);
            }
        }
        self.draw_rect_vertex_layer(&mut ctx.pass, &relief_vertices);
        // Feature-gated video rendering intentionally inherits the image
        // pipeline from this phase, so restore it after relief edges.
        ctx.pass.set_pipeline(&self.pipelines.image);
        ctx.pass.set_bind_group(0, &self.uniform_bind_group, &[]);
    }

    /// Apply video loop_count and autoplay before rendering.
    #[cfg(feature = "video")]
    pub(super) fn prepare_inline_videos(&mut self, params: &FrameParams<'_>) {
        let frame_glyphs = params.frame_glyphs;
        // Apply video loop_count and autoplay before rendering
        for glyph in &frame_glyphs.glyphs {
            if let FrameGlyph::Video {
                video_id,
                loop_count,
                autoplay,
                ..
            } = glyph
            {
                if *loop_count != 0 {
                    self.caches.video.set_loop(video_id.get(), *loop_count);
                }
                if *autoplay {
                    let state = self.caches.video.get_state(video_id.get());
                    if matches!(
                        state,
                        Some(super::super::VideoState::Stopped)
                            | Some(super::super::VideoState::Loading)
                    ) {
                        self.caches.video.play(video_id.get());
                    }
                }
            }
        }
    }

    /// Draw inline videos (inherits the image pipeline set by the inline
    /// image phase).
    #[cfg(feature = "video")]
    pub(super) fn draw_inline_videos(&mut self, ctx: &mut FramePassCtx<'_, '_>) {
        let frame_glyphs = ctx.params.frame_glyphs;
        // Gather quads for videos with a ready texture, then upload once and
        // draw per-video ranges.
        let mut quads = Vec::new();
        for glyph in &frame_glyphs.glyphs {
            if let FrameGlyph::Video {
                video_id,
                x,
                y,
                width,
                height,
                clip_rect,
                ..
            } = glyph
            {
                let (draw_y, clipped_height, tex_v_min, tex_v_max) = if let Some(clip) = clip_rect {
                    let mut y0 = *y;
                    let mut h0 = *height;
                    let mut v0 = 0.0_f32;
                    let mut v1 = 1.0_f32;
                    let top = clip.y;
                    let bottom = clip.y + clip.height;
                    if y0 < top {
                        let cut = top - y0;
                        if cut >= h0 {
                            continue;
                        }
                        y0 = top;
                        h0 -= cut;
                        if *height > 0.0 {
                            v0 += cut / *height;
                        }
                    }
                    if y0 + h0 > bottom {
                        let cut = (y0 + h0) - bottom;
                        if cut >= h0 {
                            continue;
                        }
                        h0 -= cut;
                        if *height > 0.0 {
                            v1 -= cut / *height;
                        }
                    }
                    (y0, h0, v0, v1)
                } else {
                    (*y, *height, 0.0, 1.0)
                };

                // Skip if fully clipped
                if clipped_height <= 0.0 {
                    continue;
                }

                // Check if video texture is ready
                if let Some(cached) = self.caches.video.get(video_id.get()) {
                    self.media_budget
                        .touch(crate::media_budget::MediaType::Video, video_id.get());
                    tracing::trace!(
                        "Rendering video {} at ({}, {}) size {}x{} (clipped to {}), frame_count={}",
                        video_id,
                        x,
                        y,
                        width,
                        height,
                        clipped_height,
                        cached.frame_count
                    );
                    if cached.bind_group.is_some() {
                        // Create vertices for video quad (white color = no tinting)
                        quads.push(MediaQuad {
                            id: video_id.get(),
                            vertices: textured_quad_vertices(
                                *x,
                                draw_y,
                                *width,
                                clipped_height,
                                tex_v_min,
                                tex_v_max,
                            ),
                        });
                    } else {
                        tracing::warn!("Video {} has no bind_group!", video_id);
                    }
                } else {
                    tracing::warn!("Video {} not found in cache!", video_id);
                }
            }
        }

        let all_vertices: Vec<GlyphVertex> = quads
            .iter()
            .flat_map(|quad| quad.vertices.iter().copied())
            .collect();
        let Some(upload) = self
            .arenas
            .image
            .upload(&self.device, &self.queue, &all_vertices)
        else {
            return;
        };

        let render_pass = &mut ctx.pass;
        render_pass.set_vertex_buffer(0, upload.buffer_slice());
        for (i, quad) in quads.iter().enumerate() {
            if let Some(cached) = self.caches.video.get(quad.id)
                && let Some(ref bind_group) = cached.bind_group
            {
                render_pass.set_bind_group(1, bind_group, &[]);
                render_pass.draw((i * 6) as u32..(i * 6 + 6) as u32, 0..1);
            }
        }
    }

    /// Draw inline WebKit views (opaque pipeline: DMA-BUF XRGB has alpha=0).
    #[cfg(feature = "wpe-webkit")]
    pub(super) fn draw_inline_webkit_views(&mut self, ctx: &mut FramePassCtx<'_, '_>) {
        let frame_glyphs = ctx.params.frame_glyphs;
        // Draw inline webkit views (use opaque pipeline — DMA-BUF XRGB has alpha=0)
        {
            let mut quads = Vec::new();
            for glyph in &frame_glyphs.glyphs {
                if let FrameGlyph::Xwidget {
                    xwidget_id,
                    x,
                    y,
                    width,
                    height,
                    clip_rect,
                    ..
                } = glyph
                {
                    let (draw_y, clipped_height, tex_v_min, tex_v_max) =
                        if let Some(clip) = clip_rect {
                            let mut y0 = *y;
                            let mut h0 = *height;
                            let mut v0 = 0.0_f32;
                            let mut v1 = 1.0_f32;
                            let top = clip.y;
                            let bottom = clip.y + clip.height;
                            if y0 < top {
                                let cut = top - y0;
                                if cut >= h0 {
                                    continue;
                                }
                                y0 = top;
                                h0 -= cut;
                                if *height > 0.0 {
                                    v0 += cut / *height;
                                }
                            }
                            if y0 + h0 > bottom {
                                let cut = (y0 + h0) - bottom;
                                if cut >= h0 {
                                    continue;
                                }
                                h0 -= cut;
                                if *height > 0.0 {
                                    v1 -= cut / *height;
                                }
                            }
                            (y0, h0, v0, v1)
                        } else {
                            (*y, *height, 0.0, 1.0)
                        };

                    // Skip if fully clipped
                    if clipped_height <= 0.0 {
                        continue;
                    }

                    // An inline xwidget's id IS its webkit view id.
                    let view_id = neomacs_display_protocol::types::WebKitId::new(xwidget_id.get());
                    // Check if webkit texture is ready
                    if self.caches.webkit.get(view_id).is_some() {
                        self.media_budget
                            .touch(crate::media_budget::MediaType::WebKit, view_id.get());
                        tracing::debug!(
                            "Rendering webkit {} at ({}, {}) size {}x{} (clipped to {})",
                            xwidget_id,
                            x,
                            y,
                            width,
                            height,
                            clipped_height
                        );
                        // Create vertices for webkit quad (white color = no tinting)
                        quads.push(MediaQuad {
                            id: view_id,
                            vertices: textured_quad_vertices(
                                *x,
                                draw_y,
                                *width,
                                clipped_height,
                                tex_v_min,
                                tex_v_max,
                            ),
                        });
                    } else {
                        tracing::debug!("WebKit xwidget {} not found in cache", xwidget_id);
                    }
                }
            }

            let all_vertices: Vec<GlyphVertex> = quads
                .iter()
                .flat_map(|quad| quad.vertices.iter().copied())
                .collect();
            let upload = self
                .arenas
                .image
                .upload(&self.device, &self.queue, &all_vertices);

            let render_pass = &mut ctx.pass;
            render_pass.set_pipeline(&self.pipelines.opaque_image);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            let Some(upload) = upload else {
                return;
            };
            render_pass.set_vertex_buffer(0, upload.buffer_slice());
            for (i, quad) in quads.iter().enumerate() {
                if let Some(cached) = self.caches.webkit.get(quad.id) {
                    render_pass.set_bind_group(1, &cached.bind_group, &[]);
                    render_pass.draw((i * 6) as u32..(i * 6 + 6) as u32, 0..1);
                }
            }
        }
    }

    /// Draw inline shader surfaces (image pipeline, alpha blended). Also
    /// stamps each composited surface as recently drawn so its animation
    /// demand and iTime clock stay live only while visible, and routes the
    /// pointer's hover position into the `iMouse` uniform of the surface
    /// under it.
    pub(super) fn draw_inline_surfaces(&mut self, ctx: &mut FramePassCtx<'_, '_>) {
        let frame_glyphs = ctx.params.frame_glyphs;
        let mut quads = Vec::new();
        for glyph in &frame_glyphs.glyphs {
            if let FrameGlyph::Surface {
                surface_id,
                x,
                y,
                width,
                height,
                clip_rect,
                ..
            } = glyph
            {
                let (draw_x, draw_y, clipped_width, clipped_height, u0, u1, v0, v1) =
                    if let Some(clip) = clip_rect {
                        let mut x0 = *x;
                        let mut y0 = *y;
                        let mut w0 = *width;
                        let mut h0 = *height;
                        let mut u0 = 0.0_f32;
                        let mut u1 = 1.0_f32;
                        let mut v0 = 0.0_f32;
                        let mut v1 = 1.0_f32;
                        let left = clip.x;
                        let right = clip.x + clip.width;
                        let top = clip.y;
                        let bottom = clip.y + clip.height;
                        if x0 < left {
                            let cut = left - x0;
                            if cut >= w0 {
                                continue;
                            }
                            x0 = left;
                            w0 -= cut;
                            if *width > 0.0 {
                                u0 += cut / *width;
                            }
                        }
                        if x0 + w0 > right {
                            let cut = (x0 + w0) - right;
                            if cut >= w0 {
                                continue;
                            }
                            w0 -= cut;
                            if *width > 0.0 {
                                u1 -= cut / *width;
                            }
                        }
                        if y0 < top {
                            let cut = top - y0;
                            if cut >= h0 {
                                continue;
                            }
                            y0 = top;
                            h0 -= cut;
                            if *height > 0.0 {
                                v0 += cut / *height;
                            }
                        }
                        if y0 + h0 > bottom {
                            let cut = (y0 + h0) - bottom;
                            if cut >= h0 {
                                continue;
                            }
                            h0 -= cut;
                            if *height > 0.0 {
                                v1 -= cut / *height;
                            }
                        }
                        (x0, y0, w0, h0, u0, u1, v0, v1)
                    } else {
                        (*x, *y, *width, *height, 0.0, 1.0, 0.0, 1.0)
                    };

                if clipped_width <= 0.0 || clipped_height <= 0.0 {
                    continue;
                }

                if self.caches.surface.get(surface_id.get()).is_some() {
                    self.caches.surface.mark_drawn(surface_id.get());
                    self.media_budget
                        .touch(crate::media_budget::MediaType::Surface, surface_id.get());
                    // Hover-only iMouse: while the pointer is inside the
                    // glyph rect (logical px), stream its normalized position
                    // into the surface's uniforms (picked up by the next
                    // offscreen pass). Outside the rect nothing is written,
                    // so iMouse persists at the last hover position.
                    let (mx, my) = ctx.params.mouse_pos;
                    if mx >= *x && mx < *x + *width && my >= *y && my < *y + *height {
                        self.caches.surface.set_mouse_uv(
                            surface_id.get(),
                            (mx - *x) / *width,
                            (my - *y) / *height,
                        );
                    }
                    quads.push(MediaQuad {
                        id: surface_id.get(),
                        vertices: textured_quad_vertices_uv(
                            draw_x,
                            draw_y,
                            clipped_width,
                            clipped_height,
                            u0,
                            u1,
                            v0,
                            v1,
                        ),
                    });
                } else {
                    tracing::warn!("shader surface {} not found in cache", surface_id);
                }
            }
        }

        let all_vertices: Vec<GlyphVertex> = quads
            .iter()
            .flat_map(|quad| quad.vertices.iter().copied())
            .collect();
        let Some(upload) = self
            .arenas
            .image
            .upload(&self.device, &self.queue, &all_vertices)
        else {
            return;
        };

        let render_pass = &mut ctx.pass;
        render_pass.set_pipeline(&self.pipelines.image);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, upload.buffer_slice());
        for (i, quad) in quads.iter().enumerate() {
            if let Some(cached) = self.caches.surface.get(quad.id) {
                render_pass.set_bind_group(1, &cached.composite_bind_group, &[]);
                render_pass.draw((i * 6) as u32..(i * 6 + 6) as u32, 0..1);
            }
        }
    }
}

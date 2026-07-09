//! Inline media draw phases of `render_frame_glyphs` (z-order step 7):
//! images, videos, and WebKit xwidget views.

use wgpu::util::DeviceExt;

use neomacs_display_protocol::frame_glyphs::FrameGlyph;

use super::super::vertex::GlyphVertex;
use super::WgpuRenderer;
#[cfg(feature = "video")]
use super::frame_pass::FrameParams;
use super::frame_pass::FramePassCtx;

impl WgpuRenderer {
    /// Draw inline images on top of text.
    pub(super) fn draw_inline_images(&self, ctx: &mut FramePassCtx<'_, '_>) {
        let render_pass = &mut ctx.pass;
        let frame_glyphs = ctx.params.frame_glyphs;
        // Draw inline images
        render_pass.set_pipeline(&self.pipelines.image);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);

        for glyph in &frame_glyphs.glyphs {
            if let FrameGlyph::Image {
                image_id,
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
                if let Some(cached) = self.caches.image.get(image_id.get()) {
                    // Create vertices for image quad (white color = no tinting)
                    let vertices = [
                        GlyphVertex {
                            position: [*x, draw_y],
                            tex_coords: [0.0, tex_v_min],
                            color: [1.0, 1.0, 1.0, 1.0],
                        },
                        GlyphVertex {
                            position: [*x + *width, draw_y],
                            tex_coords: [1.0, tex_v_min],
                            color: [1.0, 1.0, 1.0, 1.0],
                        },
                        GlyphVertex {
                            position: [*x + *width, draw_y + clipped_height],
                            tex_coords: [1.0, tex_v_max],
                            color: [1.0, 1.0, 1.0, 1.0],
                        },
                        GlyphVertex {
                            position: [*x, draw_y],
                            tex_coords: [0.0, tex_v_min],
                            color: [1.0, 1.0, 1.0, 1.0],
                        },
                        GlyphVertex {
                            position: [*x + *width, draw_y + clipped_height],
                            tex_coords: [1.0, tex_v_max],
                            color: [1.0, 1.0, 1.0, 1.0],
                        },
                        GlyphVertex {
                            position: [*x, draw_y + clipped_height],
                            tex_coords: [0.0, tex_v_max],
                            color: [1.0, 1.0, 1.0, 1.0],
                        },
                    ];

                    let image_buffer =
                        self.device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("Image Vertex Buffer"),
                                contents: bytemuck::cast_slice(&vertices),
                                usage: wgpu::BufferUsages::VERTEX,
                            });

                    render_pass.set_bind_group(1, &cached.bind_group, &[]);
                    render_pass.set_vertex_buffer(0, image_buffer.slice(..));
                    render_pass.draw(0..6, 0..1);
                }
            }
        }
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

    /// Draw inline videos.
    #[cfg(feature = "video")]
    pub(super) fn draw_inline_videos(&self, ctx: &mut FramePassCtx<'_, '_>) {
        let render_pass = &mut ctx.pass;
        let frame_glyphs = ctx.params.frame_glyphs;
        // Draw inline videos
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
                    if let Some(ref bind_group) = cached.bind_group {
                        // Create vertices for video quad (white color = no tinting)
                        let vertices = [
                            GlyphVertex {
                                position: [*x, draw_y],
                                tex_coords: [0.0, tex_v_min],
                                color: [1.0, 1.0, 1.0, 1.0],
                            },
                            GlyphVertex {
                                position: [*x + *width, draw_y],
                                tex_coords: [1.0, tex_v_min],
                                color: [1.0, 1.0, 1.0, 1.0],
                            },
                            GlyphVertex {
                                position: [*x + *width, draw_y + clipped_height],
                                tex_coords: [1.0, tex_v_max],
                                color: [1.0, 1.0, 1.0, 1.0],
                            },
                            GlyphVertex {
                                position: [*x, draw_y],
                                tex_coords: [0.0, tex_v_min],
                                color: [1.0, 1.0, 1.0, 1.0],
                            },
                            GlyphVertex {
                                position: [*x + *width, draw_y + clipped_height],
                                tex_coords: [1.0, tex_v_max],
                                color: [1.0, 1.0, 1.0, 1.0],
                            },
                            GlyphVertex {
                                position: [*x, draw_y + clipped_height],
                                tex_coords: [0.0, tex_v_max],
                                color: [1.0, 1.0, 1.0, 1.0],
                            },
                        ];

                        let video_buffer =
                            self.device
                                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                    label: Some("Video Vertex Buffer"),
                                    contents: bytemuck::cast_slice(&vertices),
                                    usage: wgpu::BufferUsages::VERTEX,
                                });

                        render_pass.set_bind_group(1, bind_group, &[]);
                        render_pass.set_vertex_buffer(0, video_buffer.slice(..));
                        render_pass.draw(0..6, 0..1);
                    } else {
                        tracing::warn!("Video {} has no bind_group!", video_id);
                    }
                } else {
                    tracing::warn!("Video {} not found in cache!", video_id);
                }
            }
        }
    }

    /// Draw inline WebKit views (opaque pipeline: DMA-BUF XRGB has alpha=0).
    #[cfg(feature = "wpe-webkit")]
    pub(super) fn draw_inline_webkit_views(&self, ctx: &mut FramePassCtx<'_, '_>) {
        let render_pass = &mut ctx.pass;
        let frame_glyphs = ctx.params.frame_glyphs;
        // Draw inline webkit views (use opaque pipeline — DMA-BUF XRGB has alpha=0)
        {
            render_pass.set_pipeline(&self.pipelines.opaque_image);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);

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

                    // Check if webkit texture is ready
                    if let Some(cached) = self.caches.webkit.get(*xwidget_id) {
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
                        let vertices = [
                            GlyphVertex {
                                position: [*x, draw_y],
                                tex_coords: [0.0, tex_v_min],
                                color: [1.0, 1.0, 1.0, 1.0],
                            },
                            GlyphVertex {
                                position: [*x + *width, draw_y],
                                tex_coords: [1.0, tex_v_min],
                                color: [1.0, 1.0, 1.0, 1.0],
                            },
                            GlyphVertex {
                                position: [*x + *width, draw_y + clipped_height],
                                tex_coords: [1.0, tex_v_max],
                                color: [1.0, 1.0, 1.0, 1.0],
                            },
                            GlyphVertex {
                                position: [*x, draw_y],
                                tex_coords: [0.0, tex_v_min],
                                color: [1.0, 1.0, 1.0, 1.0],
                            },
                            GlyphVertex {
                                position: [*x + *width, draw_y + clipped_height],
                                tex_coords: [1.0, tex_v_max],
                                color: [1.0, 1.0, 1.0, 1.0],
                            },
                            GlyphVertex {
                                position: [*x, draw_y + clipped_height],
                                tex_coords: [0.0, tex_v_max],
                                color: [1.0, 1.0, 1.0, 1.0],
                            },
                        ];

                        let webkit_buffer =
                            self.device
                                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                    label: Some("WebKit Vertex Buffer"),
                                    contents: bytemuck::cast_slice(&vertices),
                                    usage: wgpu::BufferUsages::VERTEX,
                                });

                        render_pass.set_bind_group(1, &cached.bind_group, &[]);
                        render_pass.set_vertex_buffer(0, webkit_buffer.slice(..));
                        render_pass.draw(0..6, 0..1);
                    } else {
                        tracing::debug!("WebKit xwidget {} not found in cache", xwidget_id);
                    }
                }
            }
        }
    }
}

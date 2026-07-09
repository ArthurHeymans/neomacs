//! Media methods for WgpuRenderer.

use super::super::image_cache::ImageCache;
#[cfg(any(feature = "video", feature = "wpe-webkit"))]
use super::super::vertex::GlyphVertex;
use super::WgpuRenderer;

impl WgpuRenderer {
    /// Load image from file path (async - returns immediately)
    /// Returns image ID, actual texture loads in background
    pub fn load_image_file(
        &mut self,
        path: &str,
        max_width: u32,
        max_height: u32,
        fg_color: u32,
        bg_color: u32,
    ) -> u32 {
        self.caches
            .image
            .load_file(path, max_width, max_height, fg_color, bg_color)
    }

    /// Load image from file path with a pre-allocated ID (for threaded mode)
    pub fn load_image_file_with_id(
        &mut self,
        id: u32,
        path: &str,
        max_width: u32,
        max_height: u32,
        fg_color: u32,
        bg_color: u32,
    ) {
        self.caches
            .image
            .load_file_with_id(id, path, max_width, max_height, fg_color, bg_color)
    }

    /// Load image from data (async - returns immediately)
    pub fn load_image_data(
        &mut self,
        data: &[u8],
        max_width: u32,
        max_height: u32,
        fg_color: u32,
        bg_color: u32,
    ) -> u32 {
        self.caches
            .image
            .load_data(data, max_width, max_height, fg_color, bg_color)
    }

    /// Load image from data with pre-allocated ID (for threaded mode)
    pub fn load_image_data_with_id(
        &mut self,
        id: u32,
        data: &[u8],
        max_width: u32,
        max_height: u32,
        fg_color: u32,
        bg_color: u32,
    ) {
        self.caches
            .image
            .load_data_with_id(id, data, max_width, max_height, fg_color, bg_color)
    }

    /// Load image from raw ARGB32 pixel data
    pub fn load_image_argb32(&mut self, data: &[u8], width: u32, height: u32, stride: u32) -> u32 {
        self.caches
            .image
            .load_raw_argb32(data, width, height, stride, 0, 0)
    }

    /// Load image from raw RGB24 pixel data
    pub fn load_image_rgb24(&mut self, data: &[u8], width: u32, height: u32, stride: u32) -> u32 {
        self.caches
            .image
            .load_raw_rgb24(data, width, height, stride, 0, 0)
    }

    /// Load image from raw ARGB32 pixel data with pre-allocated ID (for threaded mode)
    pub fn load_image_argb32_with_id(
        &mut self,
        id: u32,
        data: &[u8],
        width: u32,
        height: u32,
        stride: u32,
    ) {
        self.caches
            .image
            .load_raw_argb32_with_id(id, data, width, height, stride)
    }

    /// Load image from raw RGB24 pixel data with pre-allocated ID (for threaded mode)
    pub fn load_image_rgb24_with_id(
        &mut self,
        id: u32,
        data: &[u8],
        width: u32,
        height: u32,
        stride: u32,
    ) {
        self.caches
            .image
            .load_raw_rgb24_with_id(id, data, width, height, stride)
    }

    /// Query image file dimensions (fast - reads header only, does not block)
    pub fn query_image_file_size(path: &str) -> Option<(u32, u32)> {
        ImageCache::query_file_dimensions(path).map(|d| (d.width, d.height))
    }

    /// Query image data dimensions (fast - reads header only)
    pub fn query_image_data_size(data: &[u8]) -> Option<(u32, u32)> {
        ImageCache::query_data_dimensions(data).map(|d| (d.width, d.height))
    }

    /// Get image dimensions (works for pending and loaded images)
    pub fn get_image_size(&self, id: u32) -> Option<(u32, u32)> {
        self.caches
            .image
            .get_dimensions(id)
            .map(|d| (d.width, d.height))
    }

    /// Check if image is ready for rendering
    pub fn is_image_ready(&self, id: u32) -> bool {
        self.caches.image.is_ready(id)
    }

    /// Free an image from cache
    pub fn free_image(&mut self, id: u32) {
        self.caches.image.free(id)
    }

    /// Process pending decoded images (call each frame before rendering)
    pub fn process_pending_images(&mut self) {
        self.caches.image.process_pending(&self.device, &self.queue);
    }

    /// Load video from file path (async - returns immediately)
    /// Returns video ID, frames decode in background
    #[cfg(feature = "video")]
    pub fn load_video_file(&mut self, path: &str) -> u32 {
        self.caches.video.load_file(path)
    }

    /// Load video from file path with a pre-allocated ID.
    #[cfg(feature = "video")]
    pub fn load_video_file_with_id(
        &mut self,
        id: u32,
        path: &str,
        loop_count: i32,
        autoplay: bool,
    ) {
        self.caches
            .video
            .load_file_with_id(id, path, loop_count, autoplay);
    }

    /// Load video from URI with a pre-allocated ID.
    #[cfg(feature = "video")]
    pub fn load_video_uri_with_id(&mut self, id: u32, uri: &str, loop_count: i32, autoplay: bool) {
        self.caches
            .video
            .load_uri_with_id(id, uri, loop_count, autoplay);
    }

    /// Get video dimensions
    #[cfg(feature = "video")]
    pub fn get_video_size(&self, id: u32) -> Option<(u32, u32)> {
        self.caches.video.get_dimensions(id)
    }

    /// Get video state
    #[cfg(feature = "video")]
    pub fn get_video_state(&self, id: u32) -> Option<super::super::video_cache::VideoState> {
        self.caches.video.get_state(id)
    }

    /// Play video
    #[cfg(feature = "video")]
    pub fn video_play(&mut self, id: u32) {
        self.caches.video.play(id)
    }

    /// Pause video
    #[cfg(feature = "video")]
    pub fn video_pause(&mut self, id: u32) {
        self.caches.video.pause(id)
    }

    /// Stop video
    #[cfg(feature = "video")]
    pub fn video_stop(&mut self, id: u32) {
        self.caches.video.stop(id)
    }

    /// Set video loop count (-1 for infinite)
    #[cfg(feature = "video")]
    pub fn video_set_loop(&mut self, id: u32, count: i32) {
        self.caches.video.set_loop(id, count)
    }

    /// Free a video from cache
    #[cfg(feature = "video")]
    pub fn free_video(&mut self, id: u32) {
        self.caches.video.remove(id)
    }

    /// Process pending decoded video frames (call each frame before rendering)
    #[cfg(feature = "video")]
    pub fn process_pending_videos(&mut self) {
        tracing::debug!("process_pending_videos called");
        // Use image_cache's bind_group_layout and sampler to ensure video bind groups
        // are compatible with the shared image/video rendering pipeline
        let layout = self.caches.image.bind_group_layout();
        let sampler = self.caches.image.sampler();
        self.caches
            .video
            .process_pending(&self.device, &self.queue, layout, sampler);
    }

    /// Check if any video is currently playing
    #[cfg(feature = "video")]
    pub fn has_playing_videos(&self) -> bool {
        self.caches.video.has_playing_videos()
    }

    /// Get cached video for rendering
    #[cfg(feature = "video")]
    pub fn get_video(&self, id: u32) -> Option<&super::super::video_cache::CachedVideo> {
        self.caches.video.get(id)
    }

    /// Render floating videos from the scene.
    ///
    /// This renders video frames at fixed screen positions (not inline with text).
    #[cfg(feature = "video")]
    pub fn render_floating_videos(
        &mut self,
        view: &wgpu::TextureView,
        floating_videos: &[neomacs_display_protocol::scene::FloatingVideo],
    ) {
        use super::layer_media::{MediaQuad, textured_quad_vertices};

        if floating_videos.is_empty() {
            return;
        }

        let mut quads = Vec::new();
        for fv in floating_videos {
            tracing::debug!(
                "Rendering floating video {} at ({}, {}) size {}x{}",
                fv.video_id,
                fv.x,
                fv.y,
                fv.width,
                fv.height
            );

            if let Some(cached) = self.caches.video.get(fv.video_id.get()) {
                if cached.bind_group.is_some() {
                    quads.push(MediaQuad {
                        id: fv.video_id.get(),
                        vertices: textured_quad_vertices(fv.x, fv.y, fv.width, fv.height, 0.0, 1.0),
                    });
                } else {
                    tracing::debug!("Video {} has no bind_group yet", fv.video_id);
                }
            } else {
                tracing::debug!("Video {} not found in cache", fv.video_id);
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

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Floating Video Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Floating Video Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Don't clear - render on top
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.pipelines.image);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);

            if let Some(ref upload) = upload {
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
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Update a webkit view in the cache from a DMA-BUF buffer.
    /// Returns true if successful.
    #[cfg(feature = "wpe-webkit")]
    pub fn update_webkit_view_dmabuf(
        &mut self,
        view_id: u32,
        buffer: super::super::external_buffer::DmaBufBuffer,
    ) -> bool {
        self.caches.webkit.update_view(
            neomacs_display_protocol::types::WebKitId::new(view_id),
            buffer,
            &self.device,
            &self.queue,
        )
    }

    /// Update a webkit view in the cache from pixel data.
    /// Returns true if successful.
    #[cfg(feature = "wpe-webkit")]
    pub fn update_webkit_view_pixels(
        &mut self,
        view_id: u32,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> bool {
        self.caches.webkit.update_view_from_pixels(
            neomacs_display_protocol::types::WebKitId::new(view_id),
            width,
            height,
            pixels,
            &self.device,
            &self.queue,
        )
    }

    /// Remove a webkit view from the cache.
    #[cfg(feature = "wpe-webkit")]
    pub fn remove_webkit_view(&mut self, view_id: u32) {
        self.caches
            .webkit
            .remove(neomacs_display_protocol::types::WebKitId::new(view_id));
    }

    /// Process pending webkit frames from WPE views.
    /// NOTE: In threaded mode, frame processing is done in render_thread.rs
    /// which calls update_webkit_view_dmabuf/update_webkit_view_pixels directly.
    /// This method is kept for API compatibility but is a no-op.
    #[cfg(feature = "wpe-webkit")]
    pub fn process_webkit_frames(&mut self) {
        // In threaded mode, frame processing happens in render_thread.rs
        // The render thread calls update_webkit_view_dmabuf/update_webkit_view_pixels directly
    }

    /// Render a WebKit view texture at the given bounds.
    ///
    /// This method renders the WebKit view content (from a wgpu texture)
    /// to the screen at the specified rectangle.
    ///
    /// # Arguments
    /// * `_encoder` - The command encoder to use for rendering
    /// * `_view` - The output texture view to render to
    /// * `_webkit_bind_group` - The bind group containing the WebKit texture
    /// * `_bounds` - The rectangle where the WebKit view should be rendered
    #[cfg(feature = "wpe-webkit")]
    pub fn render_webkit_view(
        &mut self,
        _encoder: &mut wgpu::CommandEncoder,
        _view: &wgpu::TextureView,
        _webkit_bind_group: &wgpu::BindGroup,
        _bounds: neomacs_display_protocol::types::Rect,
    ) {
        // TODO: Implement texture rendering
    }

    /// Render floating webkit views to the screen.
    /// This draws the cached webkit textures at their specified positions.
    #[cfg(feature = "wpe-webkit")]
    pub fn render_floating_webkits(
        &mut self,
        view: &wgpu::TextureView,
        floating_webkits: &[neomacs_display_protocol::scene::FloatingWebKit],
    ) {
        use super::layer_media::{MediaQuad, textured_quad_vertices};

        if floating_webkits.is_empty() {
            return;
        }

        let mut quads = Vec::new();
        for fw in floating_webkits {
            tracing::debug!(
                "Rendering floating webkit {} at ({}, {}) size {}x{}",
                fw.webkit_id,
                fw.x,
                fw.y,
                fw.width,
                fw.height
            );

            if self.caches.webkit.get(fw.webkit_id).is_some() {
                quads.push(MediaQuad {
                    id: fw.webkit_id,
                    vertices: textured_quad_vertices(fw.x, fw.y, fw.width, fw.height, 0.0, 1.0),
                });
            } else {
                tracing::debug!("WebKit {} not found in cache", fw.webkit_id);
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

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Floating WebKit Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Floating WebKit Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Preserve existing content
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.pipelines.opaque_image);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);

            if let Some(ref upload) = upload {
                render_pass.set_vertex_buffer(0, upload.buffer_slice());
                for (i, quad) in quads.iter().enumerate() {
                    if let Some(cached) = self.caches.webkit.get(quad.id) {
                        render_pass.set_bind_group(1, &cached.bind_group, &[]);
                        render_pass.draw((i * 6) as u32..(i * 6 + 6) as u32, 0..1);
                    }
                }
            }
        }

        self.queue.submit(Some(encoder.finish()));
    }
}

use super::RenderApp;
#[cfg(feature = "video")]
use super::frame_sched::{NativeWindowId, PacingAction};
#[cfg(feature = "neo-term")]
use super::frame_windows::GuiFrameRenderState;
#[cfg(feature = "neo-term")]
use crate::core::face::{BoxType, Face, FaceAttributes, UnderlineStyle};
#[cfg(feature = "neo-term")]
use crate::core::frame_glyphs::{DisplaySlotId, FrameGlyph, FrameGlyphBuffer, GlyphRowRole};
#[cfg(feature = "neo-term")]
use crate::core::types::DisplayWindowId;
#[cfg(feature = "neo-term")]
use crate::core::types::{Color, FaceId, Px, Rect};
#[cfg(any(feature = "neo-term", feature = "webview"))]
use crate::thread_comm::InputEvent;
#[cfg(feature = "neo-term")]
use neomacs_display_protocol::font::ResolvedFont;
#[cfg(feature = "neo-term")]
use std::collections::HashMap;

#[cfg(feature = "neo-term")]
#[derive(Clone, Copy)]
struct TerminalPaintTarget {
    window_id: DisplayWindowId,
    row_role: GlyphRowRole,
    clip_rect: Option<Rect>,
}

#[cfg(feature = "neo-term")]
impl TerminalPaintTarget {
    const DETACHED_TEXT: Self = Self {
        window_id: DisplayWindowId::new(0),
        row_role: GlyphRowRole::Text,
        clip_rect: None,
    };

    const FLOATING: Self = Self {
        window_id: DisplayWindowId::new(0),
        row_role: GlyphRowRole::ModeLine,
        clip_rect: None,
    };
}

#[cfg(all(feature = "webview", target_os = "linux"))]
use neomacs_renderer_wgpu::WgpuRenderer;

fn publish_image_cache_event(
    shared: &super::SharedImageRenderState,
    event: neomacs_renderer_wgpu::ImageCacheEvent,
) -> crate::thread_comm::ImageStateEvent {
    let (event, terminal) = match event {
        neomacs_renderer_wgpu::ImageCacheEvent::Ready { load, metadata } => {
            let metadata = neovm_core::emacs_core::image_catalog::ResolvedImageMetadata {
                layout: metadata.layout,
                reported: metadata.reported,
                background: metadata.background,
                background_transparent: metadata.background_transparent,
                mask: metadata.mask,
                embedded: metadata.embedded,
            };
            (
                crate::thread_comm::ImageStateEvent::DecodeCompleted(load),
                Some(super::ImageDecodeTerminal::Ready(metadata)),
            )
        }
        neomacs_renderer_wgpu::ImageCacheEvent::Failed { load, error } => (
            crate::thread_comm::ImageStateEvent::DecodeCompleted(load),
            Some(super::ImageDecodeTerminal::Failed(error)),
        ),
        neomacs_renderer_wgpu::ImageCacheEvent::Evicted { image } => {
            (crate::thread_comm::ImageStateEvent::Evicted(image), None)
        }
    };

    if let Some(terminal) = terminal {
        let crate::thread_comm::ImageStateEvent::DecodeCompleted(load) = event else {
            unreachable!("only decode completion publishes terminal metadata")
        };
        shared.publish_terminal(load, terminal);
    } else {
        shared.clear_image_terminals(event.image());
    }
    event
}

impl RenderApp {
    pub(super) fn publish_image_cache_usage(&self) {
        let usage = self
            .renderer
            .as_ref()
            .map(neomacs_renderer_wgpu::WgpuRenderer::image_cache_usage)
            .unwrap_or_default();
        self.image_metadata.publish_cache_usage(usage);
    }

    #[cfg(feature = "neo-term")]
    fn frame_default_resolved_font(frame: &FrameGlyphBuffer) -> Option<&ResolvedFont> {
        frame
            .faces
            .get(&FaceId::new(0))
            .and_then(|face| face.default_resolved_font_id)
            .and_then(|font_id| frame.fonts.get(&font_id))
    }

    #[cfg(feature = "neo-term")]
    fn expanded_terminal_glyphs_for_frame(
        frame: &FrameGlyphBuffer,
        terminal_contents: &HashMap<crate::terminal::TerminalId, crate::terminal::TerminalContent>,
    ) -> (Vec<FrameGlyph>, HashMap<FaceId, Face>) {
        let cell_w = frame.char_width;
        let cell_h = frame.char_height;
        let font_size = frame.font_pixel_size;
        let ascent = cell_h * 0.8;
        let default_font = Self::frame_default_resolved_font(frame);
        let mut extra_glyphs = Vec::new();
        let mut extra_faces = HashMap::new();

        for glyph in &frame.glyphs {
            let FrameGlyph::Terminal {
                terminal_id,
                x,
                y,
                width,
                height,
            } = glyph
            else {
                continue;
            };
            let Some(terminal_id) = crate::terminal::TerminalId::new(*terminal_id) else {
                continue;
            };
            let Some(content) = terminal_contents.get(&terminal_id) else {
                continue;
            };

            extra_glyphs.push(FrameGlyph::Stretch {
                window_id: neomacs_display_protocol::types::DisplayWindowId::new(0),
                row_role: GlyphRowRole::Text,
                clip_rect: None,
                slot_id: DisplaySlotId::from_pixels(
                    DisplayWindowId::new(0),
                    Px(*x),
                    Px(*y),
                    Px(cell_w),
                    Px(cell_h),
                ),
                bidi_level: 0,
                x: *x,
                y: *y,
                width: *width,
                height: *height,
                bg: content.default_bg,
                face_id: FaceId::new(0),
                box_vertical_edges: Default::default(),
            });

            Self::expand_terminal_cells(
                content,
                *x,
                *y,
                cell_w,
                cell_h,
                ascent,
                font_size,
                default_font,
                TerminalPaintTarget::DETACHED_TEXT,
                1.0,
                &mut extra_glyphs,
                &mut extra_faces,
            );
        }

        (extra_glyphs, extra_faces)
    }

    #[cfg(feature = "neo-term")]
    fn expand_terminal_glyphs_for_render_state(
        render: &mut GuiFrameRenderState,
        terminal_contents: &HashMap<crate::terminal::TerminalId, crate::terminal::TerminalContent>,
        terminal_targets: &HashMap<
            crate::terminal::TerminalId,
            crate::terminal::TerminalDisplayTarget,
        >,
    ) {
        let Some(frame) = render.compositor.current_frame.as_ref() else {
            return;
        };
        let (mut extra_glyphs, mut extra_faces) =
            Self::expanded_terminal_glyphs_for_frame(frame, terminal_contents);
        let (window_glyphs, window_faces) =
            Self::expanded_window_terminals_for_frame(frame, terminal_contents, terminal_targets);
        extra_glyphs.extend(window_glyphs);
        extra_faces.extend(window_faces);
        render.extend_current_frame_glyphs_and_faces(extra_glyphs, extra_faces);
    }

    #[cfg(feature = "neo-term")]
    fn window_text_body(info: &crate::core::frame_glyphs::WindowInfo) -> Rect {
        match info.geometry {
            neomacs_display_protocol::PresentedWindowGeometry::Complete { regions, .. } => {
                regions.text_body
            }
            neomacs_display_protocol::PresentedWindowGeometry::Skipped { .. } => Rect::new(
                info.bounds.x,
                info.bounds.y + info.tab_line_height + info.header_line_height,
                info.bounds.width,
                (info.bounds.height
                    - info.tab_line_height
                    - info.header_line_height
                    - info.mode_line_height)
                    .max(0.0),
            ),
        }
    }

    #[cfg(feature = "neo-term")]
    fn expanded_window_terminals_for_frame(
        frame: &FrameGlyphBuffer,
        terminal_contents: &HashMap<crate::terminal::TerminalId, crate::terminal::TerminalContent>,
        terminal_targets: &HashMap<
            crate::terminal::TerminalId,
            crate::terminal::TerminalDisplayTarget,
        >,
    ) -> (Vec<FrameGlyph>, HashMap<FaceId, Face>) {
        let mut glyphs = Vec::new();
        let mut faces = HashMap::new();
        let cell_w = frame.char_width;
        let cell_h = frame.char_height;
        let ascent = cell_h * 0.8;
        let default_font = Self::frame_default_resolved_font(frame);

        for (id, target) in terminal_targets {
            let crate::terminal::TerminalDisplayTarget::Window { buffer } = target else {
                continue;
            };
            let Some(content) = terminal_contents.get(id) else {
                continue;
            };
            for info in frame
                .window_infos
                .iter()
                .filter(|info| info.buffer_id == buffer.0 && !info.is_minibuffer)
            {
                let body = Self::window_text_body(info);
                let paint = TerminalPaintTarget {
                    window_id: info.window_id,
                    row_role: GlyphRowRole::Text,
                    clip_rect: Some(body),
                };
                glyphs.push(FrameGlyph::Stretch {
                    window_id: paint.window_id,
                    row_role: paint.row_role,
                    clip_rect: paint.clip_rect,
                    slot_id: DisplaySlotId::from_pixels(
                        paint.window_id,
                        Px(body.x),
                        Px(body.y),
                        Px(cell_w),
                        Px(cell_h),
                    ),
                    bidi_level: 0,
                    x: body.x,
                    y: body.y,
                    width: body.width,
                    height: body.height,
                    bg: content.default_bg,
                    face_id: FaceId::new(0),
                    box_vertical_edges: Default::default(),
                });
                Self::expand_terminal_cells(
                    content,
                    body.x,
                    body.y,
                    cell_w,
                    cell_h,
                    ascent,
                    frame.font_pixel_size,
                    default_font,
                    paint,
                    1.0,
                    &mut glyphs,
                    &mut faces,
                );
            }
        }
        (glyphs, faces)
    }

    #[cfg(feature = "webview")]
    pub(super) fn pump_glib(&mut self) {
        let Some(system) = self.webview_system.as_mut() else {
            return;
        };
        system.service();
        for event in system.drain_events() {
            self.comms.send_input(InputEvent::WebView(event));
        }
    }

    #[cfg(not(feature = "webview"))]
    pub(super) fn pump_glib(&mut self) {}

    /// Process webkit frames and import to wgpu textures
    #[cfg(all(feature = "webview", target_os = "linux"))]
    pub(super) fn process_webkit_frames(&mut self) {
        use neomacs_renderer_wgpu::DmaBufBuffer;
        use std::os::fd::OwnedFd;

        let (Some(renderer), Some(system)) = (self.renderer.as_mut(), self.webview_system.as_mut())
        else {
            return;
        };
        let view_ids = system.view_ids();

        let try_upload_dmabuf = |renderer: &mut WgpuRenderer,
                                 view_id: neomacs_webview::WebViewId,
                                 dmabuf: neomacs_webview::DmaBufFrame|
         -> bool {
            // Never turn producer synchronization into a render-thread stall.
            // A future Vulkan semaphore-import path can consume the fence on
            // the GPU; until then an unready experimental DMA-BUF frame is
            // skipped and WPE remains free to produce its replacement.
            match dmabuf.wait_until_ready(std::time::Duration::ZERO) {
                Ok(neomacs_webview::DmaBufReadiness::Ready) => {}
                Ok(neomacs_webview::DmaBufReadiness::TimedOut) => {
                    tracing::warn!(
                        "WPE rendering fence is not ready for webview {}; skipping frame",
                        view_id.get()
                    );
                    return false;
                }
                Err(error) => {
                    tracing::warn!(
                        "WPE rendering fence failed for webview {}: {}; skipping frame",
                        view_id.get(),
                        error
                    );
                    return false;
                }
            }
            let width = dmabuf.width();
            let height = dmabuf.height();
            let fourcc = dmabuf.fourcc();
            let modifier = dmabuf.modifier();
            let planes = dmabuf.planes();
            let num_planes = planes.len().min(4) as u32;
            let mut fds: [Option<OwnedFd>; 4] = [None, None, None, None];
            let mut strides = [0u32; 4];
            let mut offsets = [0u32; 4];

            for (index, plane) in planes.iter().take(4).enumerate() {
                strides[index] = plane.stride();
                offsets[index] = plane.offset();
                let Ok(file) = plane.file().try_clone() else {
                    tracing::warn!(
                        "failed to duplicate DMA-BUF plane for webview {}; skipping frame",
                        view_id.get()
                    );
                    return false;
                };
                fds[index] = Some(file.into());
            }

            let buffer = DmaBufBuffer::new(
                fds, strides, offsets, num_planes, width, height, fourcc, modifier,
            );

            // Ownership of the complete frame crosses into the renderer. Its
            // opaque WPE lease cannot be released until the exact GPU copy
            // submission retires on the renderer's retirement worker.
            renderer.update_webview_dmabuf(view_id, buffer, dmabuf)
        };

        for view_id in view_ids {
            let Some(frame) = system.take_frame(view_id) else {
                continue;
            };
            match frame {
                neomacs_webview::WebViewFrame::DmaBuf(frame) => {
                    if try_upload_dmabuf(renderer, view_id, frame) {
                        tracing::debug!("imported DMA-BUF for webview {}", view_id.get());
                    }
                }
                neomacs_webview::WebViewFrame::Pixels(frame) => {
                    if renderer.update_webview_pixels(
                        view_id,
                        frame.width(),
                        frame.height(),
                        frame.pixels(),
                    ) {
                        tracing::debug!("uploaded pixels for webview {}", view_id.get());
                    }
                }
            }
        }
    }

    #[cfg(not(all(feature = "webview", target_os = "linux")))]
    pub(super) fn process_webkit_frames(&mut self) {}

    /// Service native decoder events once. A due frame is mapped through the
    /// presentation-owned video index to exactly the native windows whose
    /// accepted root/child snapshots reference it. The scheduler consumes
    /// those one-shot targets; the decoder's next PTS becomes a coordinator-
    /// owned service wake rather than parallel lifecycle state.
    #[cfg(feature = "video")]
    pub(super) fn process_video_frames(&mut self, now: std::time::Instant) {
        tracing::trace!("process_video_frames called");
        let presented: std::collections::HashSet<_> = self
            .frame_windows
            .windows
            .iter()
            .filter(|(key, _)| {
                let native_id = match key {
                    super::frame_windows::FrameKey::Pending => 0,
                    super::frame_windows::FrameKey::Adopted(id) => *id,
                };
                self.frame_coordinator
                    .is_eligible(super::frame_sched::NativeWindowId(native_id))
            })
            .flat_map(|(_, window)| window.render.presented_video_ids())
            .collect();
        let Some(renderer) = self.renderer.as_mut() else {
            self.frame_coordinator
                .reconcile_video_service_deadline(None);
            return;
        };
        let service = renderer.process_pending_videos_at(now, &presented).clone();
        self.frame_coordinator
            .reconcile_video_service_deadline(service.next_deadline);
        for ready in service.ready_frames {
            for (key, window_state) in &self.frame_windows.windows {
                let native_id = match key {
                    super::frame_windows::FrameKey::Pending => 0,
                    super::frame_windows::FrameKey::Adopted(id) => *id,
                };
                let native_window = NativeWindowId(native_id);
                if !self.frame_coordinator.is_eligible(native_window) {
                    continue;
                }
                if window_state.render.presents_video(ready.id) {
                    let action = self
                        .frame_coordinator
                        .submit_ready_video_frame(native_window, now);
                    if action == PacingAction::RequestRedraw {
                        window_state.request_redraw();
                    }
                }
            }
        }
    }

    #[cfg(not(feature = "video"))]
    pub(super) fn process_video_frames(&mut self, _now: std::time::Instant) {}

    /// Render pending shader-surface passes (call each frame before the main
    /// pass samples the surface textures).
    pub(super) fn process_shader_surfaces(&mut self) {
        if let Some(ref mut renderer) = self.renderer {
            renderer.process_shader_surfaces();
        }
    }

    /// Check if any animated shader surface was composited recently (needs
    /// continuous rendering while visible).
    pub(super) fn has_active_shader_surfaces(&self) -> bool {
        self.renderer
            .as_ref()
            .is_some_and(|r| r.has_active_shader_surfaces())
    }

    /// The cadence the shader-surface demand should run at (max of active
    /// `:fps` caps, else the display rate); see
    /// `WgpuRenderer::shader_surface_demand_rate`.
    pub(super) fn shader_surface_demand_rate(&self, display_rate: u32) -> u32 {
        self.renderer
            .as_ref()
            .map_or(display_rate, |r| r.shader_surface_demand_rate(display_rate))
    }

    /// Check if any WebKit view needs redraw
    #[cfg(feature = "webview")]
    pub(super) fn has_webkit_needing_redraw(&self) -> bool {
        self.webview_system
            .as_ref()
            .is_some_and(neomacs_webview::WebViewSystem::has_pending_frame)
    }

    #[cfg(not(feature = "webview"))]
    pub(super) fn has_webkit_needing_redraw(&self) -> bool {
        false
    }

    /// Check if any terminal has pending content from PTY reader threads.
    #[cfg(feature = "neo-term")]
    pub(super) fn has_terminal_activity(&self) -> bool {
        for view in self.terminal_manager.terminals.values() {
            if view.event_proxy.peek_wakeup() || view.dirty {
                return true;
            }
        }
        false
    }

    #[cfg(not(feature = "neo-term"))]
    pub(super) fn has_terminal_activity(&self) -> bool {
        false
    }

    /// Process pending image uploads (decode → GPU texture)
    pub(super) fn process_pending_images(&mut self) {
        let events = self
            .renderer
            .as_mut()
            .map(neomacs_renderer_wgpu::WgpuRenderer::process_pending_images)
            .unwrap_or_default();
        // Publish residency before wakeups: an evaluator reacting to the
        // completion event must already observe the corresponding exact cache
        // size, never the previous presentation's snapshot.
        self.publish_image_cache_usage();
        for event in events {
            let event = publish_image_cache_event(&self.image_metadata, event);
            self.comms
                .send_input(crate::thread_comm::InputEvent::ImageStateChanged { event });
        }
    }

    /// Publish the complete set of image identities which can still be drawn.
    /// Accepted presentations, deferred child presentations, and renderer-owned
    /// toolbar chrome all participate in the same lifetime fence.
    pub(super) fn synchronize_image_residency(&mut self) {
        let mut retained = self.frame_windows.retained_images();
        for pending in self.pending_child_frames.values() {
            retained.extend(pending.referenced_images().iter());
        }
        retained.extend(self.toolbar.icon_textures.values().copied());
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.synchronize_retained_images(retained);
        }
        self.publish_image_cache_usage();
    }

    pub(super) fn has_pending_images(&self) -> bool {
        self.renderer
            .as_ref()
            .is_some_and(|renderer| renderer.has_pending_images())
    }

    /// Update terminal content and expand Terminal glyphs into renderable cells.
    #[cfg(feature = "neo-term")]
    pub(super) fn update_terminals(&mut self) {
        use crate::terminal::TerminalDisplayTarget;

        // Get frame font metrics for terminal cell sizing.
        // These come from FRAME_COLUMN_WIDTH / FRAME_LINE_HEIGHT / FRAME_FONT->pixel_size.
        let (cell_w, cell_h, font_size, default_font) = if let Some(frame) = self
            .frame_windows
            .primary_window()
            .and_then(|ws| ws.render.compositor.current_frame.as_ref())
        {
            (
                frame.char_width,
                frame.char_height,
                frame.font_pixel_size,
                Self::frame_default_resolved_font(frame).cloned(),
            )
        } else {
            (8.0, 16.0, 14.0, None)
        };
        let ascent = cell_h * 0.8;

        // A window terminal follows a visible window displaying its owning
        // buffer. Prefer the selected one when the same buffer is shown more
        // than once; one PTY has one authoritative grid size.
        let mut window_layouts = HashMap::new();
        self.frame_windows
            .for_each_top_level_window(|window_state| {
                let Some(frame) = window_state.render.compositor.current_frame.as_ref() else {
                    return;
                };
                for info in frame.window_infos.iter().filter(|info| !info.is_minibuffer) {
                    let layout = (
                        info.selected,
                        Self::window_text_body(info),
                        frame.char_width,
                        frame.char_height,
                    );
                    window_layouts
                        .entry(info.buffer_id)
                        .and_modify(|current: &mut (bool, Rect, f32, f32)| {
                            if !current.0 && info.selected {
                                *current = layout;
                            }
                        })
                        .or_insert(layout);
                }
            });
        for id in self.terminal_manager.ids() {
            let Some(view) = self.terminal_manager.get_mut(id) else {
                continue;
            };
            let TerminalDisplayTarget::Window { buffer } = view.target else {
                continue;
            };
            let Some((_, body, owner_cell_w, owner_cell_h)) =
                window_layouts.get(&buffer.0).copied()
            else {
                continue;
            };
            let target_cols = (body.width / owner_cell_w).floor() as u16;
            let target_rows = (body.height / owner_cell_h).floor() as u16;
            let Some(size) = crate::terminal::TerminalGridSize::new(target_cols, target_rows)
            else {
                continue;
            };
            if view.content().is_some_and(|content| {
                content.cols as u16 != target_cols || content.rows as u16 != target_rows
            }) {
                view.resize(size);
            }
        }

        // Update all terminal content (check for PTY data)
        self.terminal_manager.update_all();

        // Check for exited terminals and notify Emacs
        for id in self.terminal_manager.ids() {
            if let Some(view) = self.terminal_manager.get_mut(id)
                && view.event_proxy.is_exited()
                && !view.exit_notified
            {
                view.exit_notified = true;
                self.comms.send_input(InputEvent::TerminalExited { id });
            }
        }
        for id in self.terminal_manager.ids() {
            if let Some(title) = self
                .terminal_manager
                .get(id)
                .and_then(|view| view.event_proxy.take_title())
            {
                self.comms
                    .send_input(InputEvent::TerminalTitleChanged { id, title });
            }
        }

        let terminal_contents: HashMap<_, _> = self
            .terminal_manager
            .ids()
            .into_iter()
            .filter_map(|id| {
                self.terminal_manager
                    .get(id)
                    .and_then(|view| view.content().map(|content| (id, content.clone())))
            })
            .collect();
        let terminal_targets: HashMap<_, _> = self
            .terminal_manager
            .ids()
            .into_iter()
            .filter_map(|id| self.terminal_manager.get(id).map(|view| (id, view.target)))
            .collect();

        self.frame_windows
            .for_each_top_level_window_mut(|window_state| {
                Self::expand_terminal_glyphs_for_render_state(
                    &mut window_state.render,
                    &terminal_contents,
                    &terminal_targets,
                );
            });

        // Render floating terminals
        let mut float_glyphs = Vec::new();
        let mut float_faces = HashMap::new();
        for id in self.terminal_manager.ids() {
            if let Some(view) = self.terminal_manager.get(id) {
                if view.target != TerminalDisplayTarget::Floating {
                    continue;
                }
                if let Some(content) = view.content() {
                    let x = view.float_x;
                    let y = view.float_y;
                    let width = content.cols as f32 * cell_w;
                    let height = content.rows as f32 * cell_h;

                    let mut bg = content.default_bg;
                    bg.a = view.float_opacity;
                    float_glyphs.push(FrameGlyph::Stretch {
                        window_id: neomacs_display_protocol::types::DisplayWindowId::new(0),
                        row_role: GlyphRowRole::ModeLine,
                        clip_rect: None,
                        slot_id: DisplaySlotId::from_pixels(
                            DisplayWindowId::new(0),
                            Px(x),
                            Px(y),
                            Px(cell_w),
                            Px(cell_h),
                        ),
                        bidi_level: 0,
                        x,
                        y,
                        width,
                        height,
                        bg,
                        face_id: FaceId::new(0),
                        box_vertical_edges: Default::default(),
                    });

                    Self::expand_terminal_cells(
                        content,
                        x,
                        y,
                        cell_w,
                        cell_h,
                        ascent,
                        font_size,
                        default_font.as_ref(),
                        TerminalPaintTarget::FLOATING,
                        view.float_opacity,
                        &mut float_glyphs,
                        &mut float_faces,
                    );
                }
            }
        }

        if let Some(primary_frame) = self
            .frame_windows
            .primary_window_mut()
            .map(|ws| &mut ws.render)
        {
            primary_frame.extend_current_frame_glyphs_and_faces(float_glyphs, float_faces);
        }
    }

    /// Expand terminal content cells into FrameGlyph entries.
    ///
    /// Terminal cells carry their own per-cell colors and SGR flags rather than
    /// a GNU face. Since `FrameGlyph::Char` resolves its visual attributes from
    /// the frame face table by `face_id`, each distinct (fg, bold, italic,
    /// underline, strikeout) combination is interned as a synthesized `Face` in
    /// `faces`, and the glyph references it. The synthesized face uses a
    /// transparent background so no per-character background is painted (the
    /// per-cell stretch above and the terminal's default-background stretch
    /// supply the background, exactly as when `Char.bg` was `None`). Because
    /// terminal cell geometry uses the frame's default font metrics, synthesized
    /// faces also inherit that font's exact render identity.
    #[cfg(feature = "neo-term")]
    fn expand_terminal_cells(
        content: &crate::terminal::content::TerminalContent,
        origin_x: f32,
        origin_y: f32,
        cell_w: f32,
        cell_h: f32,
        ascent: f32,
        font_size: f32,
        default_font: Option<&ResolvedFont>,
        paint: TerminalPaintTarget,
        opacity: f32,
        out: &mut Vec<FrameGlyph>,
        faces: &mut HashMap<FaceId, Face>,
    ) {
        use rio_vt::crosswords::style::StyleFlags as CellFlags;

        for cell in &content.cells {
            let cx = origin_x + cell.col as f32 * cell_w;
            let cy = origin_y + cell.row as f32 * cell_h;

            if cell.bg != content.default_bg {
                let mut bg = cell.bg;
                bg.a *= opacity;
                out.push(FrameGlyph::Stretch {
                    window_id: paint.window_id,
                    row_role: paint.row_role,
                    clip_rect: paint.clip_rect,
                    slot_id: DisplaySlotId::from_pixels(
                        paint.window_id,
                        Px(cx),
                        Px(cy),
                        Px(cell_w),
                        Px(cell_h),
                    ),
                    bidi_level: 0,
                    x: cx,
                    y: cy,
                    width: cell_w,
                    height: cell_h,
                    bg,
                    face_id: FaceId::new(0),
                    box_vertical_edges: Default::default(),
                });
            }

            if cell.c != ' ' && cell.c != '\0' {
                let mut fg = cell.fg;
                fg.a *= opacity;
                let bold = cell.flags.contains(CellFlags::BOLD);
                let italic = cell.flags.contains(CellFlags::ITALIC);
                let underline = cell.flags.contains(CellFlags::UNDERLINE);
                let strikeout = cell.flags.contains(CellFlags::STRIKEOUT);
                let face_id = terminal_cell_face_id(fg, bold, italic, underline, strikeout);
                faces.entry(face_id).or_insert_with(|| {
                    terminal_cell_face(
                        face_id,
                        fg,
                        bold,
                        italic,
                        underline,
                        strikeout,
                        font_size,
                        default_font,
                    )
                });
                out.push(FrameGlyph::Char {
                    window_id: paint.window_id,
                    row_role: paint.row_role,
                    clip_rect: paint.clip_rect,
                    slot_id: DisplaySlotId::from_pixels(
                        paint.window_id,
                        Px(cx),
                        Px(cy),
                        Px(cell_w),
                        Px(cell_h),
                    ),
                    bidi_level: 0,
                    char: cell.c,
                    composed: None,
                    x: cx,
                    y: cy,
                    baseline: cy + ascent,
                    width: cell_w,
                    height: cell_h,
                    ascent,
                    face_id,
                    box_vertical_edges: Default::default(),
                });
            }
        }

        // Terminal cursor
        if content.cursor.visible {
            let cx = origin_x + content.cursor.col as f32 * cell_w;
            let cy = origin_y + content.cursor.row as f32 * cell_h;
            let mut fg = content.default_fg;
            fg.a *= opacity;
            out.push(FrameGlyph::Border {
                window_id: paint.window_id,
                row_role: paint.row_role,
                clip_rect: paint.clip_rect,
                x: cx,
                y: cy,
                width: cell_w,
                height: cell_h,
                color: fg,
            });
        }
    }
}

/// Base for synthesized terminal-cell face ids. Kept far above any real GNU
/// face id so terminal faces never collide with faces published by layout.
#[cfg(feature = "neo-term")]
const TERMINAL_FACE_ID_BASE: u32 = 0xF000_0000;

/// Deterministic face id for a terminal cell's visual style.
///
/// Encodes the 8-bit-per-channel foreground plus the four SGR flags into the
/// low 28 bits below [`TERMINAL_FACE_ID_BASE`]. Identical styles map to the
/// same id, so equally styled cells share one synthesized face and one glyph
/// atlas cache entry; distinct colors/flags never collide.
#[cfg(feature = "neo-term")]
fn terminal_cell_face_id(
    fg: Color,
    bold: bool,
    italic: bool,
    underline: bool,
    strike: bool,
) -> FaceId {
    let to_u8 = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u32;
    let rgb = (to_u8(fg.r) << 16) | (to_u8(fg.g) << 8) | to_u8(fg.b);
    let flags =
        (bold as u32) | ((italic as u32) << 1) | ((underline as u32) << 2) | ((strike as u32) << 3);
    FaceId::new(TERMINAL_FACE_ID_BASE | ((rgb << 4) | flags))
}

/// Synthesize the `Face` for a terminal cell so that
/// [`FrameGlyphBuffer::resolved_face`] returns exactly the colors and
/// decorations the cell glyph used to inline: foreground from the cell,
/// transparent background (no per-character fill), bold via font weight 700,
/// italic/underline/strike-through via attributes.
#[cfg(feature = "neo-term")]
fn terminal_cell_face(
    face_id: FaceId,
    fg: Color,
    bold: bool,
    italic: bool,
    underline: bool,
    strike: bool,
    font_size: f32,
    default_font: Option<&ResolvedFont>,
) -> Face {
    let mut attrs = FaceAttributes::empty();
    if bold {
        attrs |= FaceAttributes::BOLD;
    }
    if italic {
        attrs |= FaceAttributes::ITALIC;
    }
    if underline {
        attrs |= FaceAttributes::UNDERLINE;
    }
    if strike {
        attrs |= FaceAttributes::STRIKE_THROUGH;
    }
    let underline_style = if underline {
        UnderlineStyle::from_gnu_code(1).unwrap_or_default()
    } else {
        UnderlineStyle::None
    };
    Face {
        id: face_id,
        foreground: fg,
        background: Color::TRANSPARENT,
        terminal_foreground: None,
        terminal_background: None,
        use_default_foreground: false,
        use_default_background: false,
        underline_color: None,
        terminal_underline_color: None,
        overline_color: None,
        strike_through_color: None,
        box_color: None,
        font_family: default_font
            .map(|font| font.family.clone())
            .unwrap_or_else(|| "monospace".to_string()),
        font_size,
        font_weight: if bold { 700 } else { 400 },
        attributes: attrs,
        underline_style,
        box_type: BoxType::None,
        box_line_width: Default::default(),
        box_corner_radius: 0,
        box_border_style: neomacs_display_protocol::face::BoxBorderStyle::Solid,
        box_border_speed: 1.0,
        box_color2: None,
        font_file_path: default_font.and_then(|font| font.identity.file_path.clone()),
        font_ascent: default_font.map_or(0, |font| font.ascent_px.round() as i32),
        font_descent: default_font.map_or(0, |font| font.descent_px.round() as i32),
        underline_position: 1,
        underline_thickness: 1,
        background_gradient: None,
        lisp_name: None,
        default_resolved_font_id: default_font.map(|font| font.id),
        stipple: None,
        underline_placement: neomacs_display_protocol::face::UnderlinePosition::default(),
        fontset_base_family: Some(
            default_font
                .map(|font| font.family.clone())
                .unwrap_or_else(|| "monospace".to_string()),
        ),
    }
}

#[cfg(test)]
mod image_cache_event_tests {
    use super::*;
    use neomacs_display_protocol::{
        ImageEmbeddedMetadata, ImageFrameDelay, ImageId, ImageLoadAttempt, ImageLoadToken,
        ImageStateEvent,
    };
    use std::sync::Arc;

    #[test]
    fn renderer_eviction_removes_published_residency_metadata() {
        let shared: super::super::SharedImageRenderState =
            Arc::new(super::super::ImageRenderState::default());
        let metadata = neomacs_renderer_wgpu::ImageMetadata {
            layout: neomacs_display_protocol::ImageLayoutExtent::new(48, 48),
            reported: neomacs_display_protocol::ImageReportedExtent::new(48, 48),
            background: 0,
            background_transparent: false,
            mask: neomacs_display_protocol::ImageMaskKind::None,
            embedded: ImageEmbeddedMetadata::animation(
                3,
                ImageFrameDelay::milliseconds(75, 1).expect("valid delay"),
            ),
        };
        let expected_embedded = metadata.embedded.clone();

        let image = ImageId::new(91);
        let load = ImageLoadToken::new(image, ImageLoadAttempt::new(1).unwrap());
        let event = publish_image_cache_event(
            &shared,
            neomacs_renderer_wgpu::ImageCacheEvent::Ready { load, metadata },
        );
        assert_eq!(event, ImageStateEvent::DecodeCompleted(load));
        let super::super::ImageDecodeTerminal::Ready(ready) =
            shared.terminal(load).expect("published decoder metadata")
        else {
            panic!("ready renderer event must publish ready metadata");
        };
        assert_eq!(ready.embedded, expected_embedded);

        let event = publish_image_cache_event(
            &shared,
            neomacs_renderer_wgpu::ImageCacheEvent::Evicted { image },
        );
        assert_eq!(event, ImageStateEvent::Evicted(image));
        assert!(shared.terminal(load).is_none());
    }
}

#[cfg(test)]
#[cfg(feature = "neo-term")]
mod tests {
    use super::*;
    use crate::core::frame_glyphs::FrameGlyphBuffer;
    use crate::core::types::Color;
    use crate::terminal::content::{RenderCell, RenderCursor, TerminalContent};
    use rio_vt::crosswords::style::StyleFlags as CellFlags;

    #[test]
    fn terminal_glyph_expansion_uses_frame_metrics() {
        let mut frame = FrameGlyphBuffer::with_size(120.0, 80.0);
        frame.char_width = 10.0;
        frame.char_height = 20.0;
        frame.font_pixel_size = 18.0;
        frame.glyphs.push(FrameGlyph::Terminal {
            terminal_id: 7,
            x: 30.0,
            y: 40.0,
            width: 50.0,
            height: 20.0,
        });
        let mut contents = HashMap::new();
        contents.insert(
            crate::terminal::TerminalId::new(7).expect("nonzero terminal id"),
            TerminalContent {
                cells: vec![RenderCell {
                    col: 1,
                    row: 0,
                    c: 'x',
                    fg: Color::WHITE,
                    bg: Color::BLACK,
                    flags: CellFlags::empty(),
                }],
                cols: 2,
                rows: 1,
                cursor: RenderCursor {
                    col: 0,
                    row: 0,
                    visible: false,
                },
                default_bg: Color::BLACK,
                default_fg: Color::WHITE,
            },
        );

        let (glyphs, faces) = RenderApp::expanded_terminal_glyphs_for_frame(&frame, &contents);

        assert!(matches!(
            glyphs.first(),
            Some(FrameGlyph::Stretch {
                x: 30.0,
                y: 40.0,
                width: 50.0,
                height: 20.0,
                ..
            })
        ));
        // Geometry stays on the glyph; the font size now lives on the
        // synthesized face referenced by the glyph's face_id.
        let Some(FrameGlyph::Char {
            char: ch,
            x,
            y,
            width,
            height,
            face_id,
            ..
        }) = glyphs.get(1)
        else {
            panic!("expected a Char glyph at index 1");
        };
        assert_eq!(*ch, 'x');
        assert_eq!(*x, 40.0);
        assert_eq!(*y, 40.0);
        assert_eq!(*width, 10.0);
        assert_eq!(*height, 20.0);
        assert_eq!(faces.get(face_id).expect("terminal face").font_size, 18.0);
    }

    #[test]
    fn terminal_glyph_expansion_inherits_frame_font_identity() {
        use neomacs_display_protocol::font::{
            FontReplay, FontResolutionSource, FontSlantKind, ResolvedFont, ResolvedFontAdvance,
            ResolvedFontId, ResolvedFontIdentity,
        };

        let mut frame = FrameGlyphBuffer::with_size(120.0, 80.0);
        frame.char_width = 10.0;
        frame.char_height = 20.0;
        frame.font_pixel_size = 18.0;
        let font_id = ResolvedFontId(42);
        let font = ResolvedFont {
            id: font_id,
            identity: ResolvedFontIdentity::from_file(
                "/tmp/terminal-font.ttf",
                0,
                Some("TerminalFont".to_string()),
            ),
            replay: FontReplay::Swash,
            family: "Terminal Font".to_string(),
            full_name: Some("Terminal Font Regular".to_string()),
            postscript_name: Some("TerminalFont".to_string()),
            weight: 400,
            slant: FontSlantKind::Normal,
            width: 5,
            pixel_size: 18.0,
            ascent_px: 14.0,
            descent_px: 4.0,
            space_advance_px: 10.0,
            glyph_advance: ResolvedFontAdvance::fixed_cell(10.0),
            source: FontResolutionSource::FacePrimary,
        };
        let mut default_face = Face::new(FaceId::new(0));
        default_face.default_resolved_font_id = Some(font_id);
        frame.faces.insert(default_face.id, default_face);
        frame.fonts.insert(font_id, font);
        frame.glyphs.push(FrameGlyph::Terminal {
            terminal_id: 7,
            x: 30.0,
            y: 40.0,
            width: 50.0,
            height: 20.0,
        });
        let contents = HashMap::from([(
            crate::terminal::TerminalId::new(7).expect("nonzero terminal id"),
            TerminalContent {
                cells: vec![RenderCell {
                    col: 0,
                    row: 0,
                    c: 'x',
                    fg: Color::WHITE,
                    bg: Color::BLACK,
                    flags: CellFlags::empty(),
                }],
                cols: 1,
                rows: 1,
                cursor: RenderCursor {
                    col: 0,
                    row: 0,
                    visible: false,
                },
                default_bg: Color::BLACK,
                default_fg: Color::WHITE,
            },
        )]);

        let (glyphs, faces) = RenderApp::expanded_terminal_glyphs_for_frame(&frame, &contents);
        let face_id = glyphs
            .iter()
            .find_map(|glyph| match glyph {
                FrameGlyph::Char { face_id, .. } => Some(*face_id),
                _ => None,
            })
            .expect("terminal char face");
        let face = faces.get(&face_id).expect("synthesized terminal face");

        assert_eq!(face.default_resolved_font_id, Some(font_id));
        assert_eq!(face.font_family, "Terminal Font");
        assert_eq!(
            face.font_file_path.as_deref(),
            Some("/tmp/terminal-font.ttf")
        );
        assert_eq!((face.font_ascent, face.font_descent), (14, 4));
    }

    #[test]
    fn terminal_glyph_expansion_ignores_missing_terminal_content() {
        let mut frame = FrameGlyphBuffer::with_size(120.0, 80.0);
        frame.glyphs.push(FrameGlyph::Terminal {
            terminal_id: 7,
            x: 30.0,
            y: 40.0,
            width: 50.0,
            height: 20.0,
        });
        let contents = HashMap::new();

        let (glyphs, faces) = RenderApp::expanded_terminal_glyphs_for_frame(&frame, &contents);

        assert!(glyphs.is_empty());
        assert!(faces.is_empty());
    }

    #[test]
    fn window_terminal_is_clipped_to_windows_displaying_its_owner_buffer() {
        let mut frame = FrameGlyphBuffer::with_size(300.0, 200.0);
        frame.char_width = 10.0;
        frame.char_height = 20.0;
        frame.font_pixel_size = 18.0;
        frame.add_window_info(
            DisplayWindowId::new(31),
            9,
            1,
            1,
            1,
            20.0,
            30.0,
            100.0,
            80.0,
            20.0,
            0.0,
            0.0,
            true,
            false,
            20.0,
            "*neo-term-1*".to_owned(),
            String::new(),
            false,
        );
        frame.add_window_info(
            DisplayWindowId::new(32),
            10,
            1,
            1,
            1,
            130.0,
            30.0,
            100.0,
            80.0,
            20.0,
            0.0,
            0.0,
            false,
            false,
            20.0,
            "*scratch*".to_owned(),
            String::new(),
            false,
        );
        let id = crate::terminal::TerminalId::new(7).unwrap();
        let contents = HashMap::from([(
            id,
            TerminalContent {
                cells: Vec::new(),
                cols: 10,
                rows: 3,
                cursor: RenderCursor {
                    col: 0,
                    row: 0,
                    visible: false,
                },
                default_bg: Color::BLACK,
                default_fg: Color::WHITE,
            },
        )]);
        let targets = HashMap::from([(
            id,
            crate::terminal::TerminalDisplayTarget::Window {
                buffer: neovm_core::buffer::BufferId(9),
            },
        )]);

        let (glyphs, _) =
            RenderApp::expanded_window_terminals_for_frame(&frame, &contents, &targets);

        assert_eq!(glyphs.len(), 1);
        assert!(matches!(
            glyphs[0],
            FrameGlyph::Stretch {
                window_id,
                clip_rect: Some(Rect {
                    x: 20.0,
                    y: 30.0,
                    width: 100.0,
                    height: 60.0,
                }),
                x: 20.0,
                y: 30.0,
                width: 100.0,
                height: 60.0,
                ..
            } if window_id == DisplayWindowId::new(31)
        ));
    }
}

//! Asset and embedded-content render commands.

use super::RenderApp;
use crate::backend::wgpu::media_budget::MediaType;
use crate::thread_comm::{AssetCommand, MediaSource};

#[cfg(feature = "wpe-webkit")]
use crate::backend::wpe::WpeWebView;

fn clear_image_terminal(shared: &super::SharedImageMetadata, id: u32) {
    let (lock, cvar) = &**shared;
    match lock.lock() {
        Ok(mut images) => {
            images.remove(&id);
        }
        Err(poisoned) => {
            poisoned.into_inner().remove(&id);
        }
    }
    cvar.notify_all();
}

impl RenderApp {
    #[cfg(feature = "wpe-webkit")]
    fn remove_primary_floating_webkit(&mut self, id: u32) -> bool {
        if let Some(primary_frame) = self
            .frame_windows
            .primary_window_mut()
            .map(|ws| &mut ws.render)
        {
            primary_frame.remove_floating_webkit(id)
        } else {
            false
        }
    }

    // WebKit command fields (button/state/modifiers/keycode/script/frame id) are
    // consumed only inside the `wpe-webkit` cfg blocks below.
    #[cfg_attr(not(feature = "wpe-webkit"), allow(unused_variables))]
    pub(super) fn handle_asset(&mut self, cmd: AssetCommand) {
        match cmd {
            AssetCommand::ImageLoadFile {
                id,
                path,
                max_width,
                max_height,
                realization,
                fg_color,
                bg_color,
            } => {
                clear_image_terminal(&self.image_metadata, id);
                tracing::info!(
                    "Loading image {}: {} (max {}x{})",
                    id,
                    path,
                    max_width,
                    max_height
                );
                if let Some(ref mut renderer) = self.renderer {
                    renderer.load_image_file_with_id(
                        id,
                        &path,
                        max_width,
                        max_height,
                        realization,
                        fg_color,
                        bg_color,
                    );
                } else {
                    tracing::warn!("Renderer not initialized, cannot load image {}", id);
                }
            }
            AssetCommand::ImageLoadData {
                id,
                data,
                max_width,
                max_height,
                realization,
                fg_color,
                bg_color,
            } => {
                clear_image_terminal(&self.image_metadata, id);
                tracing::info!(
                    "Loading image data {}: {} bytes (max {}x{})",
                    id,
                    data.len(),
                    max_width,
                    max_height
                );
                if let Some(ref mut renderer) = self.renderer {
                    renderer.load_image_data_with_id(
                        id,
                        &data,
                        max_width,
                        max_height,
                        realization,
                        fg_color,
                        bg_color,
                    );
                } else {
                    tracing::warn!("Renderer not initialized, cannot load image data {}", id);
                }
            }
            AssetCommand::ImageLoadArgb32 {
                id,
                data,
                width,
                height,
                stride,
            } => {
                clear_image_terminal(&self.image_metadata, id);
                tracing::debug!(
                    "Loading ARGB32 image {}: {}x{} stride={}",
                    id,
                    width,
                    height,
                    stride
                );
                if let Some(ref mut renderer) = self.renderer {
                    renderer.load_image_argb32_with_id(id, &data, width, height, stride);
                }
            }
            AssetCommand::ImageLoadRgb24 {
                id,
                data,
                width,
                height,
                stride,
            } => {
                clear_image_terminal(&self.image_metadata, id);
                tracing::debug!(
                    "Loading RGB24 image {}: {}x{} stride={}",
                    id,
                    width,
                    height,
                    stride
                );
                if let Some(ref mut renderer) = self.renderer {
                    renderer.load_image_rgb24_with_id(id, &data, width, height, stride);
                }
            }
            AssetCommand::ImageFree { id } => {
                clear_image_terminal(&self.image_metadata, id);
                tracing::debug!("Freeing image {}", id);
                if let Some(ref mut renderer) = self.renderer {
                    renderer.free_image(id);
                }
            }
            AssetCommand::WebKitCreate { id, width, height } => {
                tracing::info!("Creating WebKit view: id={}, {}x{}", id, width, height);
                #[cfg(feature = "wpe-webkit")]
                if let Some(ref backend) = self.wpe_backend {
                    if let Some(platform_display) = backend.platform_display() {
                        match WpeWebView::new(id, platform_display, width, height) {
                            Ok(view) => {
                                self.webkit_views.insert(id, view);
                                tracing::info!("WebKit view {} created successfully", id);
                            }
                            Err(e) => {
                                tracing::error!("Failed to create WebKit view {}: {:?}", id, e)
                            }
                        }
                    } else {
                        tracing::error!("WPE platform display not available");
                    }
                } else {
                    tracing::warn!("WPE backend not initialized, cannot create WebKit view");
                }
            }
            AssetCommand::WebKitLoadUri { id, url } => {
                tracing::info!("Loading URL in WebKit view {}: {}", id, url);
                #[cfg(feature = "wpe-webkit")]
                if let Some(view) = self.webkit_views.get_mut(&id) {
                    if let Err(e) = view.load_uri(&url) {
                        tracing::error!("Failed to load URL in view {}: {:?}", id, e);
                    }
                } else {
                    tracing::warn!("WebKit view {} not found", id);
                }
            }
            AssetCommand::WebKitResize { id, width, height } => {
                tracing::debug!("Resizing WebKit view {}: {}x{}", id, width, height);
                #[cfg(feature = "wpe-webkit")]
                if let Some(view) = self.webkit_views.get_mut(&id) {
                    view.resize(width, height);
                }
            }
            AssetCommand::WebKitDestroy { id } => {
                tracing::info!("Destroying WebKit view {}", id);
                #[cfg(feature = "wpe-webkit")]
                {
                    self.webkit_views.remove(&id);
                    if let Some(primary_frame) = self
                        .frame_windows
                        .primary_window_mut()
                        .map(|ws| &mut ws.render)
                    {
                        primary_frame
                            .floating_webkits
                            .retain(|w| w.webkit_id.get() != id);
                    }
                    self.frame_windows
                        .destroy_floating_webkit_from_top_level_windows(id);
                    let _ = self.remove_primary_floating_webkit(id);
                    if let Some(ref mut renderer) = self.renderer {
                        renderer.remove_webkit_view(id);
                    }
                    self.frame_windows.mark_top_level_dirty();
                }
            }
            AssetCommand::WebKitClick { id, x, y, button } => {
                tracing::debug!(
                    "WebKit click view {} at ({}, {}), button {}",
                    id,
                    x,
                    y,
                    button
                );
                #[cfg(feature = "wpe-webkit")]
                if let Some(view) = self.webkit_views.get(&id) {
                    view.click(x, y, button);
                }
            }
            AssetCommand::WebKitPointerEvent {
                id,
                event_type,
                x,
                y,
                button,
                state,
                modifiers,
            } => {
                tracing::trace!(
                    "WebKit pointer event view {} type {} at ({}, {})",
                    id,
                    event_type,
                    x,
                    y
                );
                #[cfg(feature = "wpe-webkit")]
                if let Some(view) = self.webkit_views.get(&id) {
                    view.send_pointer_event(event_type, x, y, button, state, modifiers);
                }
            }
            AssetCommand::WebKitScroll {
                id,
                x,
                y,
                delta_x,
                delta_y,
            } => {
                tracing::debug!(
                    "WebKit scroll view {} at ({}, {}), delta ({}, {})",
                    id,
                    x,
                    y,
                    delta_x,
                    delta_y
                );
                #[cfg(feature = "wpe-webkit")]
                if let Some(view) = self.webkit_views.get(&id) {
                    view.scroll(x, y, delta_x, delta_y);
                }
            }
            AssetCommand::WebKitKeyEvent {
                id,
                keyval,
                keycode,
                pressed,
                modifiers,
            } => {
                tracing::debug!(
                    "WebKit key event view {} keyval {} pressed {}",
                    id,
                    keyval,
                    pressed
                );
                #[cfg(feature = "wpe-webkit")]
                if let Some(view) = self.webkit_views.get(&id) {
                    view.send_keyboard_event(keyval, keycode, pressed, modifiers);
                }
            }
            AssetCommand::WebKitGoBack { id } => {
                tracing::info!("WebKit go back view {}", id);
                #[cfg(feature = "wpe-webkit")]
                if let Some(view) = self.webkit_views.get_mut(&id) {
                    let _ = view.go_back();
                }
            }
            AssetCommand::WebKitGoForward { id } => {
                tracing::info!("WebKit go forward view {}", id);
                #[cfg(feature = "wpe-webkit")]
                if let Some(view) = self.webkit_views.get_mut(&id) {
                    let _ = view.go_forward();
                }
            }
            AssetCommand::WebKitReload { id } => {
                tracing::info!("WebKit reload view {}", id);
                #[cfg(feature = "wpe-webkit")]
                if let Some(view) = self.webkit_views.get_mut(&id) {
                    let _ = view.reload();
                }
            }
            AssetCommand::WebKitExecuteJavaScript { id, script } => {
                tracing::debug!("WebKit execute JS view {}", id);
                #[cfg(feature = "wpe-webkit")]
                if let Some(view) = self.webkit_views.get(&id) {
                    let _ = view.execute_javascript(&script);
                }
            }
            AssetCommand::WebKitSetFloating {
                frame,
                id,
                x,
                y,
                width,
                height,
            } => {
                let emacs_frame_id = frame.raw_id();
                tracing::info!(
                    "WebKit set floating: id={} at ({},{}) {}x{}",
                    id,
                    x,
                    y,
                    width,
                    height
                );
                #[cfg(feature = "wpe-webkit")]
                {
                    let overlay = crate::core::scene::FloatingWebKit {
                        webkit_id: crate::core::types::WebKitId::new(id),
                        x,
                        y,
                        width,
                        height,
                    };
                    if let Some(primary_frame) = self
                        .frame_windows
                        .primary_window_mut()
                        .map(|ws| &mut ws.render)
                    {
                        primary_frame
                            .floating_webkits
                            .retain(|w| w.webkit_id.get() != id);
                    }
                    self.frame_windows
                        .remove_floating_webkit_from_top_level_windows(id);
                    let _ = self.remove_primary_floating_webkit(id);
                    if let Some(window_state) = self.frame_windows.get_mut(emacs_frame_id) {
                        window_state.render.push_floating_webkit(overlay);
                    } else if self.frame_windows.is_primary_frame_id(emacs_frame_id) {
                        if let Some(primary_frame) = self
                            .frame_windows
                            .primary_window_mut()
                            .map(|ws| &mut ws.render)
                        {
                            primary_frame.push_floating_webkit(overlay);
                        }
                    } else {
                        tracing::warn!(
                            "WebKitSetFloating requested for unknown frame_id=0x{:x}",
                            emacs_frame_id
                        );
                    }
                }
            }
            AssetCommand::WebKitRemoveFloating { frame, id } => {
                let emacs_frame_id = frame.raw_id();
                tracing::info!("WebKit remove floating: id={}", id);
                #[cfg(feature = "wpe-webkit")]
                {
                    if let Some(primary_frame) = self
                        .frame_windows
                        .primary_window_mut()
                        .map(|ws| &mut ws.render)
                    {
                        primary_frame
                            .floating_webkits
                            .retain(|w| w.webkit_id.get() != id);
                    }
                    self.frame_windows
                        .remove_floating_webkit_from_top_level_windows(id);
                    let removed_primary = self.remove_primary_floating_webkit(id);
                    if let Some(window_state) = self.frame_windows.get_mut(emacs_frame_id) {
                        window_state.render.mark_dirty();
                    } else if self.frame_windows.is_primary_frame_id(emacs_frame_id)
                        && !removed_primary
                    {
                        tracing::debug!(
                            "WebKitRemoveFloating requested for primary frame without matching overlay"
                        );
                    } else {
                        tracing::warn!(
                            "WebKitRemoveFloating requested for unknown frame_id=0x{:x}",
                            emacs_frame_id
                        );
                    }
                }
            }
            AssetCommand::VideoCreate {
                id,
                source,
                loop_count,
                autoplay,
            } => {
                tracing::info!("Loading video {}: {}", id, source.as_str());
                #[cfg(feature = "video")]
                if let Some(ref mut renderer) = self.renderer {
                    match source {
                        MediaSource::File(path) => {
                            renderer.load_video_file_with_id(id, &path, loop_count, autoplay);
                        }
                        MediaSource::Uri(uri) => {
                            renderer.load_video_uri_with_id(id, &uri, loop_count, autoplay);
                        }
                    }
                    tracing::info!("Video loaded with requested id {}", id);
                }
            }
            AssetCommand::VideoPlay { id } => {
                tracing::debug!("Playing video {}", id);
                #[cfg(feature = "video")]
                if let Some(ref mut renderer) = self.renderer {
                    renderer.video_play(id);
                }
            }
            AssetCommand::VideoPause { id } => {
                tracing::debug!("Pausing video {}", id);
                #[cfg(feature = "video")]
                if let Some(ref mut renderer) = self.renderer {
                    renderer.video_pause(id);
                }
            }
            AssetCommand::VideoDestroy { id } => {
                tracing::info!("Destroying video {}", id);
                #[cfg(feature = "video")]
                if let Some(ref mut renderer) = self.renderer {
                    renderer.video_stop(id);
                }
            }
            AssetCommand::SurfaceCreate {
                id,
                source,
                width,
                height,
                animate,
                recreatable,
            } => {
                if let Some(ref mut renderer) = self.renderer {
                    let result = match &source {
                        crate::thread_comm::SurfaceSource::Shader {
                            language,
                            source,
                            uniforms,
                            channel0,
                        } => renderer.create_shader_surface(
                            id, *language, source, uniforms, width, height, animate, *channel0,
                        ),
                        crate::thread_comm::SurfaceSource::Pixels { data } => {
                            renderer.create_pixel_surface(id, data, width, height)
                        }
                    };
                    match result {
                        Ok(()) => {
                            // Account the surface's texture against the shared
                            // media budget. Logical w*h*4: shader surfaces
                            // actually allocate at physical resolution (scale
                            // factor squared larger) — refine alongside
                            // touch-on-draw LRU.
                            self.media_budget.register(
                                MediaType::Surface,
                                id,
                                (width as usize) * (height as usize) * 4,
                            );
                            self.surface_recreatable.insert(id, recreatable);

                            // Eviction driver. The new surface is already
                            // registered above, so the overshoot is fully in
                            // `current_memory` and the honest argument is
                            // new_size = 0: get_eviction_candidates(n) walks
                            // entries in (MediaType, last_access, id) order —
                            // Image < Video < WebKit < Surface, LRU-first
                            // within a type — and returns the prefix whose
                            // byte sum covers (current + n) - max. Only
                            // recreatable shader surfaces may actually be
                            // freed (the declarative resolver recreates them
                            // on the next redisplay walk; everything else in
                            // the list is skipped), so evict the first
                            // eligible candidate and re-query until under
                            // budget or out of victims. Never evict the
                            // surface just created.
                            while self.media_budget.is_over_budget() {
                                let victim = self
                                    .media_budget
                                    .get_eviction_candidates(0)
                                    .into_iter()
                                    .find(|&(kind, victim_id)| {
                                        kind == MediaType::Surface
                                            && victim_id != id
                                            && self
                                                .surface_recreatable
                                                .get(&victim_id)
                                                .copied()
                                                .unwrap_or(false)
                                    });
                                let Some((_, victim_id)) = victim else {
                                    tracing::debug!(
                                        "media budget over limit ({}/{} bytes) with no \
                                         recreatable shader surface to evict",
                                        self.media_budget.current_usage(),
                                        self.media_budget.max_limit()
                                    );
                                    break;
                                };
                                renderer.free_surface(victim_id);
                                self.media_budget.unregister(MediaType::Surface, victim_id);
                                self.surface_recreatable.remove(&victim_id);
                                tracing::info!(
                                    "evicting shader surface {victim_id} (over media budget)"
                                );
                            }
                        }
                        Err(err) => {
                            tracing::warn!("shader surface {id} create failed: {err}");
                        }
                    }
                } else {
                    tracing::warn!("Renderer not initialized, cannot create surface {id}");
                }
            }
            AssetCommand::SurfaceSetUniform { id, name, value } => {
                if let Some(ref mut renderer) = self.renderer {
                    renderer.set_surface_uniform(id, &name, value);
                }
            }
            AssetCommand::SurfaceFree { id } => {
                self.media_budget.unregister(MediaType::Surface, id);
                self.surface_recreatable.remove(&id);
                if let Some(ref mut renderer) = self.renderer {
                    renderer.free_surface(id);
                }
            }
            AssetCommand::FrameShaderSet { composed } => {
                if let Some(ref mut renderer) = self.renderer {
                    match composed {
                        Some((source, language)) => {
                            if let Err(err) = renderer.set_frame_post(language, &source) {
                                tracing::warn!("frame shader install failed: {err}");
                            }
                        }
                        None => renderer.clear_frame_post(),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod image_terminal_tests {
    use super::*;
    use neovm_core::emacs_core::image_catalog::ResolvedImageMetadata;
    use std::collections::HashMap;
    use std::sync::{Arc, Condvar, Mutex};

    #[test]
    fn free_or_reload_clears_shared_image_terminal() {
        let shared: super::super::SharedImageMetadata =
            Arc::new((Mutex::new(HashMap::new()), Condvar::new()));
        let (lock, _) = &*shared;
        lock.lock().unwrap().insert(
            9,
            super::super::ImageDecodeTerminal::Failed("old failure".to_owned()),
        );

        clear_image_terminal(&shared, 9);
        assert!(!lock.lock().unwrap().contains_key(&9));

        lock.lock().unwrap().insert(
            9,
            super::super::ImageDecodeTerminal::Ready(ResolvedImageMetadata {
                width: 2,
                height: 3,
                background: 0,
                background_transparent: true,
            }),
        );
        clear_image_terminal(&shared, 9);
        assert!(!lock.lock().unwrap().contains_key(&9));

        lock.lock().unwrap().insert(
            9,
            super::super::ImageDecodeTerminal::Ready(ResolvedImageMetadata {
                width: 5,
                height: 8,
                background: 0x12_34_56,
                background_transparent: false,
            }),
        );
        assert!(matches!(
            lock.lock().unwrap().get(&9),
            Some(super::super::ImageDecodeTerminal::Ready(_))
        ));
    }
}

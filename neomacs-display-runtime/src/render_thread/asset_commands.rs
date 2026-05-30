//! Asset and embedded-content render commands.

use super::RenderApp;
use crate::thread_comm::{AssetCommand, InputEvent};

#[cfg(feature = "wpe-webkit")]
use crate::backend::wpe::WpeWebView;

impl RenderApp {
    #[cfg(feature = "wpe-webkit")]
    fn remove_primary_floating_webkit(&mut self, id: u32) -> bool {
        if let Some(primary_frame) = self.frame_windows.primary_window_mut().map(|ws| &mut ws.render) {
            primary_frame.remove_floating_webkit(id)
        } else {
            false
        }
    }

    pub(super) fn handle_asset(&mut self, cmd: AssetCommand) {
        match cmd {
            AssetCommand::ImageLoadFile {
                id,
                path,
                max_width,
                max_height,
                fg_color,
                bg_color,
            } => {
                tracing::info!(
                    "Loading image {}: {} (max {}x{})",
                    id,
                    path,
                    max_width,
                    max_height
                );
                if let Some(ref mut renderer) = self.renderer {
                    renderer.load_image_file_with_id(
                        id, &path, max_width, max_height, fg_color, bg_color,
                    );
                    if let Some((w, h)) = renderer.get_image_size(id) {
                        let (lock, cvar) = &*self.image_dimensions;
                        if let Ok(mut dims) = lock.lock() {
                            dims.insert(id, (w, h));
                            cvar.notify_all();
                        }
                        self.comms.send_input(InputEvent::ImageDimensionsReady {
                            id,
                            width: w,
                            height: h,
                        });
                        tracing::debug!("Sent ImageDimensionsReady for image {}: {}x{}", id, w, h);
                    }
                } else {
                    tracing::warn!("Renderer not initialized, cannot load image {}", id);
                }
            }
            AssetCommand::ImageLoadData {
                id,
                data,
                max_width,
                max_height,
                fg_color,
                bg_color,
            } => {
                tracing::info!(
                    "Loading image data {}: {} bytes (max {}x{})",
                    id,
                    data.len(),
                    max_width,
                    max_height
                );
                if let Some(ref mut renderer) = self.renderer {
                    renderer.load_image_data_with_id(
                        id, &data, max_width, max_height, fg_color, bg_color,
                    );
                    if let Some((w, h)) = renderer.get_image_size(id) {
                        let (lock, cvar) = &*self.image_dimensions;
                        if let Ok(mut dims) = lock.lock() {
                            dims.insert(id, (w, h));
                            cvar.notify_all();
                        }
                        self.comms.send_input(InputEvent::ImageDimensionsReady {
                            id,
                            width: w,
                            height: h,
                        });
                        tracing::debug!(
                            "Sent ImageDimensionsReady for image data {}: {}x{}",
                            id,
                            w,
                            h
                        );
                    }
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
                tracing::debug!(
                    "Loading ARGB32 image {}: {}x{} stride={}",
                    id,
                    width,
                    height,
                    stride
                );
                if let Some(ref mut renderer) = self.renderer {
                    renderer.load_image_argb32_with_id(id, &data, width, height, stride);
                    if let Some((w, h)) = renderer.get_image_size(id) {
                        let (lock, cvar) = &*self.image_dimensions;
                        if let Ok(mut dims) = lock.lock() {
                            dims.insert(id, (w, h));
                            cvar.notify_all();
                        }
                    }
                }
            }
            AssetCommand::ImageLoadRgb24 {
                id,
                data,
                width,
                height,
                stride,
            } => {
                tracing::debug!(
                    "Loading RGB24 image {}: {}x{} stride={}",
                    id,
                    width,
                    height,
                    stride
                );
                if let Some(ref mut renderer) = self.renderer {
                    renderer.load_image_rgb24_with_id(id, &data, width, height, stride);
                    if let Some((w, h)) = renderer.get_image_size(id) {
                        let (lock, cvar) = &*self.image_dimensions;
                        if let Ok(mut dims) = lock.lock() {
                            dims.insert(id, (w, h));
                            cvar.notify_all();
                        }
                    }
                }
            }
            AssetCommand::ImageFree { id } => {
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
                    if let Some(primary_frame) = self.frame_windows.primary_window_mut().map(|ws| &mut ws.render) {
                        primary_frame.floating_webkits.retain(|w| w.webkit_id != id);
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
                        webkit_id: id,
                        x,
                        y,
                        width,
                        height,
                    };
                    if let Some(primary_frame) = self.frame_windows.primary_window_mut().map(|ws| &mut ws.render) {
                        primary_frame.floating_webkits.retain(|w| w.webkit_id != id);
                    }
                    self.frame_windows
                        .remove_floating_webkit_from_top_level_windows(id);
                    let _ = self.remove_primary_floating_webkit(id);
                    if let Some(window_state) = self.frame_windows.get_mut(emacs_frame_id) {
                        window_state.render.push_floating_webkit(overlay);
                    } else if self.frame_windows.is_primary_frame_id(emacs_frame_id) {
                        if let Some(primary_frame) = self.frame_windows.primary_window_mut().map(|ws| &mut ws.render) {
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
                    if let Some(primary_frame) = self.frame_windows.primary_window_mut().map(|ws| &mut ws.render) {
                        primary_frame.floating_webkits.retain(|w| w.webkit_id != id);
                    }
                    self.frame_windows
                        .remove_floating_webkit_from_top_level_windows(id);
                    let removed_primary =
                        self.remove_primary_floating_webkit(id);
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
                path,
                loop_count,
                autoplay,
            } => {
                tracing::info!("Loading video {}: {}", id, path);
                #[cfg(feature = "video")]
                if let Some(ref mut renderer) = self.renderer {
                    renderer.load_video_file_with_id(id, &path, loop_count, autoplay);
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
        }
    }
}

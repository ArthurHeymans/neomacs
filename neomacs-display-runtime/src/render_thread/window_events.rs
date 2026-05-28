use super::RenderApp;
use super::frame_windows::GuiFrameWindowState;
use super::state::{effective_window_scale_factor, emacs_pixels_from_window_size};
use crate::backend::wgpu::{
    NEOMACS_CTRL_MASK, NEOMACS_META_MASK, NEOMACS_SHIFT_MASK, NEOMACS_SUPER_MASK,
};
use crate::thread_comm::InputEvent;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::WindowId;

impl RenderApp {
    fn handle_frame_window_popup_key(
        window_state: &mut GuiFrameWindowState,
        logical_key: &Key,
    ) -> Option<InputEvent> {
        match logical_key.as_ref() {
            Key::Named(NamedKey::Escape) => {
                window_state.render.popup_menu = None;
                window_state.render.chrome_interaction.menu_bar_active = None;
                window_state
                    .render
                    .chrome_interaction
                    .compact_bar_menu_active = None;
                window_state.render.frame_dirty = true;
                Some(InputEvent::MenuSelection { index: -1 })
            }
            Key::Named(NamedKey::ArrowDown) => {
                let menu = window_state.render.popup_menu.as_mut()?;
                if menu.move_hover(1) {
                    window_state.render.frame_dirty = true;
                }
                None
            }
            Key::Named(NamedKey::ArrowUp) => {
                let menu = window_state.render.popup_menu.as_mut()?;
                if menu.move_hover(-1) {
                    window_state.render.frame_dirty = true;
                }
                None
            }
            Key::Named(NamedKey::Enter) => {
                let menu = window_state.render.popup_menu.as_mut()?;
                let panel = menu.active_panel();
                let hi = panel.hover_index;
                if hi >= 0 && (hi as usize) < panel.item_indices.len() {
                    let global_idx = panel.item_indices[hi as usize];
                    if menu.all_items[global_idx].submenu {
                        if menu.open_submenu() {
                            window_state.render.frame_dirty = true;
                        }
                        None
                    } else {
                        window_state.render.popup_menu = None;
                        window_state.render.chrome_interaction.menu_bar_active = None;
                        window_state
                            .render
                            .chrome_interaction
                            .compact_bar_menu_active = None;
                        window_state.render.frame_dirty = true;
                        Some(InputEvent::MenuSelection {
                            index: global_idx as i32,
                        })
                    }
                } else {
                    window_state.render.popup_menu = None;
                    window_state.render.chrome_interaction.menu_bar_active = None;
                    window_state
                        .render
                        .chrome_interaction
                        .compact_bar_menu_active = None;
                    window_state.render.frame_dirty = true;
                    Some(InputEvent::MenuSelection { index: -1 })
                }
            }
            Key::Named(NamedKey::ArrowRight) => {
                let menu = window_state.render.popup_menu.as_mut()?;
                if menu.open_submenu() {
                    window_state.render.frame_dirty = true;
                }
                None
            }
            Key::Named(NamedKey::ArrowLeft) => {
                let menu = window_state.render.popup_menu.as_mut()?;
                if menu.close_submenu() {
                    window_state.render.frame_dirty = true;
                }
                None
            }
            Key::Named(NamedKey::Home) => {
                let menu = window_state.render.popup_menu.as_mut()?;
                menu.active_panel_mut().hover_index = -1;
                if menu.move_hover(1) {
                    window_state.render.frame_dirty = true;
                }
                None
            }
            Key::Named(NamedKey::End) => {
                let menu = window_state.render.popup_menu.as_mut()?;
                let len = menu.active_panel().item_indices.len() as i32;
                menu.active_panel_mut().hover_index = len;
                if menu.move_hover(-1) {
                    window_state.render.frame_dirty = true;
                }
                None
            }
            _ => None,
        }
    }

    fn emacs_frame_for_window_event(&self, window_id: WindowId) -> u64 {
        self.frame_windows
            .event_frame_for_winit(window_id)
            .unwrap_or(0)
    }

    fn record_typing_speed_keypress(&mut self, window_id: WindowId) {
        if !self.effects.typing_speed.enabled {
            return;
        }
        let now = std::time::Instant::now();
        if let Some(window_state) = self.frame_windows.get_by_winit_mut(window_id) {
            window_state.render.typing_speed.key_press_times.push(now);
            window_state.render.frame_dirty = true;
        }
    }

    pub(super) fn record_idle_dim_activity(&mut self, window_id: WindowId) {
        if !self.effects.idle_dim.enabled {
            return;
        }
        let now = std::time::Instant::now();
        if let Some(window_state) = self.frame_windows.get_by_winit_mut(window_id) {
            window_state.render.idle_dim.last_activity_time = now;
            window_state.render.frame_dirty = true;
        }
    }

    pub(super) fn handle_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                tracing::info!("Window close requested");
                let is_primary = self.frame_windows.is_primary_winit(window_id);
                let emacs_fid = self.emacs_frame_for_window_event(window_id);
                self.comms.send_input(InputEvent::WindowClose {
                    emacs_frame_id: emacs_fid,
                });
                if is_primary {
                    event_loop.exit();
                } else {
                    self.frame_windows.request_destroy(emacs_fid);
                }
            }

            WindowEvent::Resized(size) => {
                tracing::info!("WindowEvent::Resized: {}x{}", size.width, size.height);

                let emacs_fid = self.emacs_frame_for_window_event(window_id);
                let is_primary = self.frame_windows.is_primary_winit(window_id);
                let mut sent_resize = false;
                if let Some(device) = self.gpu.as_ref().map(|gpu| gpu.device.clone()) {
                    if let Some(ws) = self.frame_windows.get_by_winit_mut(window_id) {
                        ws.handle_resize(&device, size.width, size.height);
                        if is_primary {
                            if let Some(renderer) = &mut self.renderer {
                                renderer.resize(size.width, size.height);
                            }
                            if self.effects.resize_padding.enabled
                                && let Some(renderer) = self.renderer.as_ref()
                            {
                                renderer.trigger_transient_resize_padding(
                                    &mut ws.render.renderer_effects,
                                    std::time::Instant::now(),
                                );
                            }
                            ws.render.frame_dirty = true;
                        }
                        let (emacs_w, emacs_h) = emacs_pixels_from_window_size(
                            size.width,
                            size.height,
                            ws.native.scale_factor,
                        );
                        self.comms.send_input(InputEvent::WindowResize {
                            width: emacs_w,
                            height: emacs_h,
                            emacs_frame_id: emacs_fid,
                        });
                        sent_resize = true;
                    }
                }
                if is_primary && !sent_resize {
                    self.primary_native_fallback.width = size.width;
                    self.primary_native_fallback.height = size.height;
                    if let Some(renderer) = &mut self.renderer {
                        renderer.resize(size.width, size.height);
                    }
                    self.mark_primary_dirty();
                    let (emacs_w, emacs_h) = emacs_pixels_from_window_size(
                        size.width,
                        size.height,
                        self.primary_native_fallback.scale_factor,
                    );
                    self.comms.send_input(InputEvent::WindowResize {
                        width: emacs_w,
                        height: emacs_h,
                        emacs_frame_id: emacs_fid,
                    });
                }
            }

            WindowEvent::Focused(focused) => {
                let is_primary = self.frame_windows.is_primary_winit(window_id);
                let emacs_fid = self.emacs_frame_for_window_event(window_id);
                self.comms.send_input(InputEvent::WindowFocus {
                    focused,
                    emacs_frame_id: emacs_fid,
                });
            }

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key,
                        state,
                        text,
                        physical_key,
                        ..
                    },
                ..
            } => {
                if state == ElementState::Pressed {
                    tracing::debug!(
                        "KeyboardInput: logical_key={:?} physical_key={:?} text={:?} mods={} ime={}",
                        logical_key,
                        physical_key,
                        text,
                        self.modifiers,
                        self.primary_ime_preedit_active()
                    );
                }
                let is_primary = self.frame_windows.is_primary_winit(window_id);
                let ime_preedit_active = self.frame_windows.get_by_winit(window_id).map_or_else(
                    || is_primary && self.primary_ime_preedit_active(),
                    |ws| ws.render.ime_preedit_active,
                );
                let handled_managed_popup = state == ElementState::Pressed
                    && self
                        .frame_windows
                        .get_by_winit(window_id)
                        .is_some_and(|ws| ws.render.popup_menu.is_some());
                if handled_managed_popup {
                    let event = self
                        .frame_windows
                        .get_by_winit_mut(window_id)
                        .and_then(|ws| Self::handle_frame_window_popup_key(ws, &logical_key));
                    if let Some(event) = event {
                        self.comms.send_input(event);
                    }
                } else if self.primary_popup_menu().is_some() && state == ElementState::Pressed {
                    match logical_key.as_ref() {
                        Key::Named(NamedKey::Escape) => {
                            self.comms
                                .send_input(InputEvent::MenuSelection { index: -1 });
                            self.set_primary_popup_menu(None);
                            self.with_primary_chrome_interaction_mut(|chrome| {
                                chrome.menu_bar_active = None;
                            });
                            self.mark_primary_dirty();
                        }
                        Key::Named(NamedKey::ArrowDown) => {
                            if let Some(menu) = self.primary_popup_menu_mut() {
                                if menu.move_hover(1) {
                                    self.mark_primary_dirty();
                                }
                            }
                        }
                        Key::Named(NamedKey::ArrowUp) => {
                            if let Some(menu) = self.primary_popup_menu_mut() {
                                if menu.move_hover(-1) {
                                    self.mark_primary_dirty();
                                }
                            }
                        }
                        Key::Named(NamedKey::Enter) => {
                            if let Some(menu) = self.primary_popup_menu_mut() {
                                let panel = menu.active_panel();
                                let hi = panel.hover_index;
                                if hi >= 0 && (hi as usize) < panel.item_indices.len() {
                                    let global_idx = panel.item_indices[hi as usize];
                                    if menu.all_items[global_idx].submenu {
                                        if menu.open_submenu() {
                                            self.mark_primary_dirty();
                                        }
                                    } else {
                                        self.comms.send_input(InputEvent::MenuSelection {
                                            index: global_idx as i32,
                                        });
                                        self.set_primary_popup_menu(None);
                                        self.with_primary_chrome_interaction_mut(|chrome| {
                                            chrome.menu_bar_active = None;
                                        });
                                        self.mark_primary_dirty();
                                    }
                                } else {
                                    self.comms
                                        .send_input(InputEvent::MenuSelection { index: -1 });
                                    self.set_primary_popup_menu(None);
                                    self.with_primary_chrome_interaction_mut(|chrome| {
                                        chrome.menu_bar_active = None;
                                    });
                                    self.mark_primary_dirty();
                                }
                            }
                        }
                        Key::Named(NamedKey::ArrowRight) => {
                            if let Some(menu) = self.primary_popup_menu_mut() {
                                if menu.open_submenu() {
                                    self.mark_primary_dirty();
                                }
                            }
                        }
                        Key::Named(NamedKey::ArrowLeft) => {
                            if let Some(menu) = self.primary_popup_menu_mut() {
                                if menu.close_submenu() {
                                    self.mark_primary_dirty();
                                }
                            }
                        }
                        Key::Named(NamedKey::Home) => {
                            if let Some(menu) = self.primary_popup_menu_mut() {
                                menu.active_panel_mut().hover_index = -1;
                                if menu.move_hover(1) {
                                    self.mark_primary_dirty();
                                }
                            }
                        }
                        Key::Named(NamedKey::End) => {
                            if let Some(menu) = self.primary_popup_menu_mut() {
                                let len = menu.active_panel().item_indices.len() as i32;
                                menu.active_panel_mut().hover_index = len;
                                if menu.move_hover(-1) {
                                    self.mark_primary_dirty();
                                }
                            }
                        }
                        _ => {}
                    }
                } else if ime_preedit_active {
                    tracing::debug!(
                        "IME preedit active, suppressing KeyboardInput: {:?}",
                        logical_key
                    );
                } else {
                    let mut handled_via_text = false;
                    if state == ElementState::Pressed
                        && Self::should_use_committed_text(&logical_key)
                    {
                        if let Some(ref txt) = text {
                            let s = txt.as_str();
                            if let Some(control_keysym) = Self::translate_control_text(s) {
                                tracing::debug!(
                                    "KeyboardInput control text path: text={:?} keysym=0x{:04x} mods=0x{:x}",
                                    s,
                                    control_keysym,
                                    self.modifiers
                                );
                                self.comms.send_input(InputEvent::Key {
                                    keysym: control_keysym,
                                    modifiers: self.modifiers,
                                    pressed: true,
                                    emacs_frame_id: self.emacs_frame_for_window_event(window_id),
                                });
                                self.record_idle_dim_activity(window_id);
                                self.record_typing_speed_keypress(window_id);
                                handled_via_text = true;
                            } else if let Some(keysyms) =
                                Self::translate_committed_text(s, self.modifiers)
                            {
                                tracing::debug!(
                                    "KeyboardInput committed text path: text={:?} keysyms={:?} mods=0x{:x}",
                                    s,
                                    keysyms,
                                    self.modifiers
                                );
                                for keysym in keysyms {
                                    tracing::debug!(
                                        "Queueing text key event: keysym=0x{:04x} mods=0x{:x}",
                                        keysym,
                                        self.modifiers
                                    );
                                    self.comms.send_input(InputEvent::Key {
                                        keysym,
                                        modifiers: self.modifiers,
                                        pressed: true,
                                        emacs_frame_id: self
                                            .emacs_frame_for_window_event(window_id),
                                    });
                                    self.record_idle_dim_activity(window_id);
                                    self.record_typing_speed_keypress(window_id);
                                }
                                handled_via_text = true;
                            }
                        }
                    }
                    if !handled_via_text {
                        let mut keysym = Self::translate_key(&logical_key);
                        if keysym == 0 && self.modifiers != 0 {
                            use winit::keyboard::KeyCode;
                            use winit::keyboard::PhysicalKey;
                            keysym = match physical_key {
                                PhysicalKey::Code(KeyCode::Space) => 0x20,
                                _ => 0,
                            };
                        }
                        if keysym != 0 {
                            tracing::debug!(
                                "KeyboardInput translated path: logical_key={:?} physical_key={:?} keysym=0x{:04x} mods=0x{:x} pressed={}",
                                logical_key,
                                physical_key,
                                keysym,
                                self.modifiers,
                                state == ElementState::Pressed
                            );
                            if state == ElementState::Pressed {
                                if let Some(window_state) =
                                    self.frame_windows.get_by_winit_mut(window_id)
                                {
                                    window_state.set_mouse_hidden_for_typing(true);
                                }
                            }
                            if state == ElementState::Pressed {
                                self.record_typing_speed_keypress(window_id);
                            }
                            if self.effects.idle_dim.enabled {
                                self.record_idle_dim_activity(window_id);
                            }
                            self.comms.send_input(InputEvent::Key {
                                keysym,
                                modifiers: self.modifiers,
                                pressed: state == ElementState::Pressed,
                                emacs_frame_id: self.emacs_frame_for_window_event(window_id),
                            });
                        } else if state == ElementState::Pressed {
                            tracing::debug!(
                                "KeyboardInput dropped after translation: logical_key={:?} physical_key={:?} text={:?} mods=0x{:x}",
                                logical_key,
                                physical_key,
                                text,
                                self.modifiers
                            );
                        }
                    }
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                self.handle_mouse_input(window_id, state, button);
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.handle_cursor_moved(window_id, position);
            }

            WindowEvent::MouseWheel { delta, .. } => {
                self.handle_mouse_wheel(window_id, delta);
            }

            WindowEvent::RedrawRequested => {
                if let Some(emacs_fid) = self.frame_windows.event_frame_for_winit(window_id) {
                    if let Some(window_state) = self.frame_windows.get_mut(emacs_fid) {
                        window_state.render.frame_dirty = false;
                    }
                    self.render_frame_window(emacs_fid);
                }
            }

            WindowEvent::ModifiersChanged(mods) => {
                let old_modifiers = self.modifiers;
                let state = mods.state();
                self.modifiers = 0;
                if state.shift_key() {
                    self.modifiers |= NEOMACS_SHIFT_MASK;
                }
                if state.control_key() {
                    self.modifiers |= NEOMACS_CTRL_MASK;
                }
                if state.alt_key() {
                    self.modifiers |= NEOMACS_META_MASK;
                }
                if state.super_key() {
                    self.modifiers |= NEOMACS_SUPER_MASK;
                }
                tracing::debug!(
                    "ModifiersChanged: old=0x{:x} new=0x{:x} shift={} ctrl={} alt={} super={}",
                    old_modifiers,
                    self.modifiers,
                    state.shift_key(),
                    state.control_key(),
                    state.alt_key(),
                    state.super_key()
                );
                if self.modifiers != old_modifiers {
                    self.record_idle_dim_activity(window_id);
                }
            }

            WindowEvent::Ime(ime_event) => match ime_event {
                winit::event::Ime::Enabled => {
                    if let Some(window_state) = self.frame_windows.get_by_winit_mut(window_id) {
                        window_state.native.ime_enabled = true;
                        window_state.native.last_ime_cursor_area = None;
                        if let Some(target) = window_state.render.cursor.target_cloned() {
                            Self::update_frame_window_ime_cursor_area_if_needed(
                                window_state,
                                &target,
                            );
                        }
                    } else if self.frame_windows.is_primary_winit(window_id) {
                        self.set_primary_ime_enabled(true);
                        self.reset_primary_ime_cursor_area();
                        if let Some(target) = self.primary_cursor().target_cloned() {
                            self.update_ime_cursor_area_if_needed(&target);
                        }
                    }
                    tracing::info!("IME enabled");
                }
                winit::event::Ime::Disabled => {
                    if let Some(window_state) = self.frame_windows.get_by_winit_mut(window_id) {
                        window_state.native.ime_enabled = false;
                        window_state.render.ime_preedit_active = false;
                        window_state.render.ime_preedit_text.clear();
                        window_state.native.last_ime_cursor_area = None;
                        window_state.render.frame_dirty = true;
                    } else if self.frame_windows.is_primary_winit(window_id) {
                        self.set_primary_ime_enabled(false);
                        self.clear_primary_ime_preedit();
                        self.reset_primary_ime_cursor_area();
                    }
                    tracing::info!("IME disabled");
                }
                winit::event::Ime::Commit(text) => {
                    tracing::debug!("IME Commit: '{}'", text);
                    if let Some(window_state) = self.frame_windows.get_by_winit_mut(window_id) {
                        window_state.render.ime_preedit_active = false;
                        window_state.render.ime_preedit_text.clear();
                        window_state.render.frame_dirty = true;
                    } else if self.frame_windows.is_primary_winit(window_id) {
                        self.clear_primary_ime_preedit();
                    }
                    for ch in text.chars() {
                        let keysym = ch as u32;
                        if keysym != 0 {
                            self.comms.send_input(InputEvent::Key {
                                keysym,
                                modifiers: 0,
                                pressed: true,
                                emacs_frame_id: self.emacs_frame_for_window_event(window_id),
                            });
                            self.record_idle_dim_activity(window_id);
                            self.record_typing_speed_keypress(window_id);
                        }
                    }
                }
                winit::event::Ime::Preedit(text, cursor_range) => {
                    tracing::debug!("IME Preedit: '{}' cursor: {:?}", text, cursor_range);
                    if let Some(window_state) = self.frame_windows.get_by_winit_mut(window_id) {
                        window_state.render.ime_preedit_active = !text.is_empty();
                        window_state.render.ime_preedit_text = text.clone();
                        if let Some(target) = window_state.render.cursor.target_cloned() {
                            Self::update_frame_window_ime_cursor_area_if_needed(
                                window_state,
                                &target,
                            );
                        }
                        window_state.render.frame_dirty = true;
                    } else if self.frame_windows.is_primary_winit(window_id) {
                        self.set_primary_ime_preedit(text.clone());
                        if let Some(target) = self.primary_cursor().target_cloned() {
                            self.update_ime_cursor_area_if_needed(&target);
                        }
                    }
                }
            },

            WindowEvent::DroppedFile(path) => {
                if let Some(path_str) = path.to_str() {
                    tracing::info!("File dropped: {}", path_str);
                    let mouse_pos = self
                        .frame_windows
                        .get_by_winit(window_id)
                        .map_or((0.0, 0.0), |window_state| window_state.render.mouse_pos);
                    self.comms.send_input(InputEvent::FileDrop {
                        paths: vec![path_str.to_string()],
                        x: mouse_pos.0,
                        y: mouse_pos.1,
                    });
                }
            }

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let effective_scale = effective_window_scale_factor(scale_factor);
                let is_primary = self.frame_windows.is_primary_winit(window_id);
                if let Some(ws) = self.frame_windows.get_by_winit_mut(window_id) {
                    tracing::info!(
                        "Scale factor changed for frame 0x{:x}: previous_effective={} raw={} effective={}",
                        ws.render.emacs_frame_id,
                        ws.native.scale_factor,
                        scale_factor,
                        effective_scale
                    );
                    ws.set_scale_factor(scale_factor);
                    if is_primary {
                        if let Some(ref mut renderer) = self.renderer {
                            renderer.set_scale_factor(effective_scale as f32);
                        }
                    }
                } else if is_primary {
                    tracing::info!(
                        "Scale factor changed: previous_effective={} raw={} effective={}",
                        self.primary_native_fallback.scale_factor,
                        scale_factor,
                        effective_scale
                    );
                    self.primary_native_fallback.scale_factor = effective_scale;
                }
            }

            _ => {}
        }
    }
}

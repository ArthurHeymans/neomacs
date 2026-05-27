use super::RenderApp;
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
    fn emacs_frame_for_window_event(&self, window_id: WindowId) -> u64 {
        self.frame_windows
            .emacs_frame_for_winit(window_id)
            .unwrap_or_else(|| {
                if self.frame_windows.is_primary_winit(window_id) {
                    self.frame_windows.primary_event_frame_id()
                } else {
                    0
                }
            })
    }

    fn record_typing_speed_keypress(&mut self, window_id: WindowId) {
        if !self.effects.typing_speed.enabled {
            return;
        }
        let now = std::time::Instant::now();
        if self.frame_windows.is_primary_winit(window_id) {
            if let Some(primary_frame) = self.primary_frame.as_mut() {
                primary_frame.typing_speed.key_press_times.push(now);
            }
            self.mark_primary_dirty();
        } else if let Some(window_state) = self.frame_windows.get_by_winit_mut(window_id) {
            window_state.render.typing_speed.key_press_times.push(now);
            window_state.render.frame_dirty = true;
        }
    }

    pub(super) fn record_idle_dim_activity(&mut self, window_id: WindowId) {
        if !self.effects.idle_dim.enabled {
            return;
        }
        let now = std::time::Instant::now();
        if self.frame_windows.is_primary_winit(window_id) {
            if let Some(primary_frame) = self.primary_frame.as_mut() {
                primary_frame.idle_dim.last_activity_time = now;
            }
            self.mark_primary_dirty();
        } else if let Some(window_state) = self.frame_windows.get_by_winit_mut(window_id) {
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

                let is_primary = self.frame_windows.is_primary_winit(window_id);
                let emacs_fid = self.emacs_frame_for_window_event(window_id);
                if is_primary {
                    self.handle_resize(size.width, size.height);
                    let (emacs_w, emacs_h) =
                        emacs_pixels_from_window_size(size.width, size.height, self.scale_factor);
                    tracing::info!(
                        "Sending WindowResize event to Emacs: {}x{}",
                        emacs_w,
                        emacs_h
                    );
                    self.comms.send_input(InputEvent::WindowResize {
                        width: emacs_w,
                        height: emacs_h,
                        emacs_frame_id: emacs_fid,
                    });
                } else if let Some(device) = self.gpu.as_ref().map(|gpu| gpu.device.clone()) {
                    if let Some(ws) = self.frame_windows.get_mut(emacs_fid) {
                        ws.handle_resize(&device, size.width, size.height);
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
                    }
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
                        self.ime_preedit_active
                    );
                }
                let is_primary = self.frame_windows.is_primary_winit(window_id);
                let secondary_ime_preedit_active = self
                    .frame_windows
                    .get_by_winit(window_id)
                    .is_some_and(|ws| ws.render.ime_preedit_active);
                if self.primary_popup_menu().is_some() && state == ElementState::Pressed {
                    match logical_key.as_ref() {
                        Key::Named(NamedKey::Escape) => {
                            self.comms
                                .send_input(InputEvent::MenuSelection { index: -1 });
                            self.set_primary_popup_menu(None);
                            self.chrome_interaction.menu_bar_active = None;
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
                                        self.chrome_interaction.menu_bar_active = None;
                                        self.mark_primary_dirty();
                                    }
                                } else {
                                    self.comms
                                        .send_input(InputEvent::MenuSelection { index: -1 });
                                    self.set_primary_popup_menu(None);
                                    self.chrome_interaction.menu_bar_active = None;
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
                } else if (is_primary && self.ime_preedit_active)
                    || (!is_primary && secondary_ime_preedit_active)
                {
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
                                if is_primary {
                                    if !self.mouse_hidden_for_typing
                                        && let Some(ref window) = self.window
                                    {
                                        window.set_cursor_visible(false);
                                        self.mouse_hidden_for_typing = true;
                                    }
                                } else if let Some(window_state) =
                                    self.frame_windows.get_by_winit_mut(window_id)
                                    && !window_state.native.mouse_hidden_for_typing
                                {
                                    window_state.native.window.set_cursor_visible(false);
                                    window_state.native.mouse_hidden_for_typing = true;
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
                if self.frame_windows.is_primary_winit(window_id) {
                    self.set_primary_dirty(false);
                    self.render();
                } else if let Some(emacs_fid) = self.frame_windows.emacs_frame_for_winit(window_id)
                {
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
                    if self.frame_windows.is_primary_winit(window_id) {
                        self.ime_enabled = true;
                        self.last_ime_cursor_area = None;
                        if let Some(target) = self.primary_cursor().target_cloned() {
                            self.update_ime_cursor_area_if_needed(&target);
                        }
                    } else if let Some(window_state) =
                        self.frame_windows.get_by_winit_mut(window_id)
                    {
                        window_state.native.ime_enabled = true;
                        window_state.native.last_ime_cursor_area = None;
                        if let Some(target) = window_state.render.cursor.target_cloned() {
                            Self::update_frame_window_ime_cursor_area_if_needed(
                                window_state,
                                &target,
                            );
                        }
                    }
                    tracing::info!("IME enabled");
                }
                winit::event::Ime::Disabled => {
                    if self.frame_windows.is_primary_winit(window_id) {
                        self.ime_enabled = false;
                        self.ime_preedit_active = false;
                        self.ime_preedit_text.clear();
                        self.last_ime_cursor_area = None;
                        self.mark_primary_dirty();
                    } else if let Some(window_state) =
                        self.frame_windows.get_by_winit_mut(window_id)
                    {
                        window_state.native.ime_enabled = false;
                        window_state.render.ime_preedit_active = false;
                        window_state.render.ime_preedit_text.clear();
                        window_state.native.last_ime_cursor_area = None;
                        window_state.render.frame_dirty = true;
                    }
                    tracing::info!("IME disabled");
                }
                winit::event::Ime::Commit(text) => {
                    tracing::debug!("IME Commit: '{}'", text);
                    if self.frame_windows.is_primary_winit(window_id) {
                        self.ime_preedit_active = false;
                        self.ime_preedit_text.clear();
                        self.mark_primary_dirty();
                    } else if let Some(window_state) =
                        self.frame_windows.get_by_winit_mut(window_id)
                    {
                        window_state.render.ime_preedit_active = false;
                        window_state.render.ime_preedit_text.clear();
                        window_state.render.frame_dirty = true;
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
                    if self.frame_windows.is_primary_winit(window_id) {
                        self.ime_preedit_active = !text.is_empty();
                        self.ime_preedit_text = text.clone();
                        if let Some(target) = self.primary_cursor().target_cloned() {
                            self.update_ime_cursor_area_if_needed(&target);
                        }
                        self.mark_primary_dirty();
                    } else if let Some(window_state) =
                        self.frame_windows.get_by_winit_mut(window_id)
                    {
                        window_state.render.ime_preedit_active = !text.is_empty();
                        window_state.render.ime_preedit_text = text.clone();
                        if let Some(target) = window_state.render.cursor.target_cloned() {
                            Self::update_frame_window_ime_cursor_area_if_needed(
                                window_state,
                                &target,
                            );
                        }
                        window_state.render.frame_dirty = true;
                    }
                }
            },

            WindowEvent::DroppedFile(path) => {
                if let Some(path_str) = path.to_str() {
                    tracing::info!("File dropped: {}", path_str);
                    let mouse_pos = if self.frame_windows.is_primary_winit(window_id) {
                        self.primary_mouse_pos()
                    } else {
                        self.frame_windows
                            .get_by_winit(window_id)
                            .map_or((0.0, 0.0), |window_state| window_state.render.mouse_pos)
                    };
                    self.comms.send_input(InputEvent::FileDrop {
                        paths: vec![path_str.to_string()],
                        x: mouse_pos.0,
                        y: mouse_pos.1,
                    });
                }
            }

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let effective_scale = effective_window_scale_factor(scale_factor);
                if !self.frame_windows.is_primary_winit(window_id) {
                    if let Some(ws) = self.frame_windows.get_by_winit_mut(window_id) {
                        tracing::info!(
                            "Scale factor changed for frame 0x{:x}: previous_effective={} raw={} effective={}",
                            ws.render.emacs_frame_id,
                            ws.native.scale_factor,
                            scale_factor,
                            effective_scale
                        );
                        ws.set_scale_factor(scale_factor);
                    }
                    return;
                }
                tracing::info!(
                    "Scale factor changed: previous_effective={} raw={} effective={}",
                    self.scale_factor,
                    scale_factor,
                    effective_scale
                );
                self.scale_factor = effective_scale;
                if let Some(ref mut renderer) = self.renderer {
                    renderer.set_scale_factor(effective_scale as f32);
                }
                if let Some(primary_frame) = self.primary_frame.as_mut() {
                    primary_frame
                        .glyph_atlas
                        .set_scale_factor(effective_scale as f32);
                }
                self.mark_primary_dirty();
            }

            _ => {}
        }
    }
}

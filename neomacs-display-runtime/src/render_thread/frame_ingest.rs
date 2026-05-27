//! Frame ingestion and cursor target extraction.

use super::RenderApp;
use super::frame_windows::GuiFrameWindowState;
use crate::render_thread::cursor::CursorTarget;
use neomacs_display_protocol::glyph_matrix::{
    GuiCompactBarState, GuiMenuBarState, GuiToolBarState,
};
use std::collections::HashSet;
use winit::dpi::{PhysicalPosition, PhysicalSize};

impl RenderApp {
    fn ingest_frame_window_root_frame(
        window_state: &mut GuiFrameWindowState,
        frame: crate::core::frame_glyphs::FrameGlyphBuffer,
        menu_bar: Option<GuiMenuBarState>,
        tool_bar: Option<GuiToolBarState>,
        compact_bar: Option<GuiCompactBarState>,
        cursor_config: &crate::render_thread::cursor::CursorState,
    ) {
        if menu_bar.is_none() {
            window_state.render.chrome_interaction.clear_menu_bar();
        }
        if tool_bar.is_none() {
            window_state.render.chrome_interaction.clear_toolbar();
        }
        if compact_bar.is_none() {
            window_state.render.chrome_interaction.clear_compact_bar();
        }
        if frame.tab_bar.is_none() {
            window_state.render.chrome_interaction.clear_tab_bar();
        }

        window_state.render.cursor.reset_blink();

        window_state.render.menu_bar = menu_bar;
        window_state.render.tool_bar = tool_bar;
        window_state.render.compact_bar = compact_bar;
        window_state.render.current_frame = Some(frame);
        Self::sync_frame_window_cursor(window_state, cursor_config);
        window_state.render.frame_dirty = true;
    }

    fn sync_frame_window_cursor(
        window_state: &mut GuiFrameWindowState,
        cursor_config: &crate::render_thread::cursor::CursorState,
    ) {
        let mut active_cursor = window_state
            .render
            .current_frame
            .as_ref()
            .and_then(|frame| {
                crate::render_thread::frame_windows::GuiFrameWindowManager::cursor_target_for_frame(
                    window_state.render.emacs_frame_id,
                    frame,
                )
            });

        if active_cursor.is_none() {
            for (_, entry) in &window_state.render.child_frames.frames {
                if let Some(cursor) = entry.frame.phys_cursor.as_ref() {
                    active_cursor = Some(CursorTarget {
                        window_id: cursor.window_id,
                        x: cursor.x,
                        y: cursor.y,
                        width: cursor.width,
                        height: cursor.height,
                        style: cursor.style,
                        color: cursor.color,
                        frame_id: entry.frame_id,
                    });
                    break;
                }
            }
        }

        window_state.render.cursor.copy_config_from(cursor_config);
        if let Some(new_target) = active_cursor {
            let (_, target_moved) = window_state.render.cursor.set_target(new_target);
            if target_moved {
                window_state.render.frame_dirty = true;
            }
        } else {
            window_state.render.cursor.clear_target();
            window_state.native.last_ime_cursor_area = None;
            window_state.render.ime_preedit_active = false;
            window_state.render.ime_preedit_text.clear();
            window_state
                .native
                .window
                .set_ime_cursor_area(PhysicalPosition::new(0.0, 0.0), PhysicalSize::new(1.0, 1.0));
        }
    }

    /// Get latest frame from Emacs (non-blocking).
    pub(super) fn poll_frame(&mut self) {
        self.primary_child_frames_mut().tick();
        while let Ok(display_state) = self.comms.frame_rx.try_recv() {
            let frame_id = display_state.frame_id;
            let parent_id = display_state.parent_id;
            let gui_menu_bar = display_state.gui_menu_bar.clone();
            let gui_tool_bar = display_state.gui_tool_bar.clone();
            let gui_compact_bar = display_state.gui_compact_bar.clone();

            // Materialize FrameDisplayState → FrameGlyphBuffer for the
            // existing rendering code.  The layout engine populates
            // the grid and non-grid items; materialize() converts the
            // grid into pixel-positioned glyphs and appends non-grid items.
            let frame = display_state.materialize();

            // ── Observation point: inspect what will be rendered ──
            // Set NEOMACS_DUMP_FRAME_GLYPHS=1 to dump every glyph.
            if std::env::var("NEOMACS_DUMP_FRAME_GLYPHS").as_deref() == Ok("1") {
                let mut char_count = 0usize;
                let mut bg_count = 0usize;
                let mut border_count = 0usize;
                let mut scrollbar_count = 0usize;
                let mut image_count = 0usize;
                let mut stretch_count = 0usize;
                let mut video_count = 0usize;
                let mut webkit_count = 0usize;
                let mut other_count = 0usize;
                for g in &frame.glyphs {
                    match g {
                        crate::core::frame_glyphs::FrameGlyph::Char { .. } => char_count += 1,
                        crate::core::frame_glyphs::FrameGlyph::Background { .. } => bg_count += 1,
                        crate::core::frame_glyphs::FrameGlyph::Border { .. } => border_count += 1,
                        crate::core::frame_glyphs::FrameGlyph::ScrollBar { .. } => {
                            scrollbar_count += 1
                        }
                        crate::core::frame_glyphs::FrameGlyph::Image { .. } => image_count += 1,
                        crate::core::frame_glyphs::FrameGlyph::Stretch { .. } => stretch_count += 1,
                        crate::core::frame_glyphs::FrameGlyph::Video { .. } => video_count += 1,
                        crate::core::frame_glyphs::FrameGlyph::WebKit { .. } => webkit_count += 1,
                        _ => other_count += 1,
                    }
                }
                let cursor_count = frame.window_cursors.len();
                tracing::info!(
                    "poll_frame: frame_id={} parent_id={} size={:.0}x{:.0} char={:.1}x{:.1} \
                     glyphs={} (char={} bg={} border={} stretch={} scrollbar={} image={} video={} webkit={} other={}) \
                     windows={} window_cursors={} phys_cursor={} faces={}",
                    frame_id,
                    parent_id,
                    frame.width,
                    frame.height,
                    frame.char_width,
                    frame.char_height,
                    frame.glyphs.len(),
                    char_count,
                    bg_count,
                    border_count,
                    stretch_count,
                    scrollbar_count,
                    image_count,
                    video_count,
                    webkit_count,
                    other_count,
                    frame.window_infos.len(),
                    cursor_count,
                    if frame.phys_cursor.is_some() {
                        "yes"
                    } else {
                        "no"
                    },
                    frame.faces.len(),
                );
                if let Some(cursor) = frame.phys_cursor.as_ref() {
                    tracing::info!(
                        "phys_cursor: window_id={} charpos={} row={} col={} slot=(window_id={},row={},col={}) \
                         rect=({:.2},{:.2}) {:.2}x{:.2} ascent={:.2} style={:?} color={:?} cursor_fg={:?}",
                        cursor.window_id,
                        cursor.charpos,
                        cursor.row,
                        cursor.col,
                        cursor.slot_id.window_id,
                        cursor.slot_id.row,
                        cursor.slot_id.col,
                        cursor.x,
                        cursor.y,
                        cursor.width,
                        cursor.height,
                        cursor.ascent,
                        cursor.style,
                        cursor.color,
                        cursor.cursor_fg,
                    );
                    match frame.slot_glyph(cursor.slot_id) {
                        Some(slot_glyph) => {
                            tracing::info!("phys_cursor_slot_glyph: {:?}", slot_glyph)
                        }
                        None => tracing::warn!(
                            "phys_cursor_slot_glyph: missing slot=(window_id={},row={},col={})",
                            cursor.slot_id.window_id,
                            cursor.slot_id.row,
                            cursor.slot_id.col,
                        ),
                    }
                    if let Some(effects) = frame.phys_cursor_effects() {
                        tracing::info!("phys_cursor_effects: {:?}", effects);
                    }
                } else {
                    tracing::info!("phys_cursor: none");
                }
                if !frame.window_cursors.is_empty() {
                    let all_window_cursors: String = frame
                        .window_cursors
                        .iter()
                        .enumerate()
                        .fold(String::new(), |acc, (i, cursor)| {
                            acc + &format!(
                                "  window_cursor[{}]: window_id={} slot=(window_id={},row={},col={}) \
                                 rect=({:.2},{:.2}) {:.2}x{:.2} style={:?} color={:?}\n",
                                i,
                                cursor.window_id,
                                cursor.slot_id.window_id,
                                cursor.slot_id.row,
                                cursor.slot_id.col,
                                cursor.x,
                                cursor.y,
                                cursor.width,
                                cursor.height,
                                cursor.style,
                                cursor.color,
                            )
                        });
                    tracing::info!("window_cursors:\n{}", all_window_cursors);
                }
                let all_glyphs: String =
                    frame
                        .glyphs
                        .iter()
                        .enumerate()
                        .fold(String::new(), |acc, (i, g)| {
                            let slot = g.slot_id();
                            acc + &format!(
                                "  glyph[{}][r={},c={}]: {:?}\n",
                                i,
                                slot.map_or(0, |s| s.row),
                                slot.map_or(0, |s| s.col),
                                g
                            )
                        });
                tracing::info!("all_glyphs:\n{}", all_glyphs);
            }

            if frame_id != 0 && parent_id == 0 {
                if let Some(window_state) = self.frame_windows.get_mut(frame_id) {
                    Self::ingest_frame_window_root_frame(
                        window_state,
                        frame,
                        gui_menu_bar,
                        gui_tool_bar,
                        gui_compact_bar,
                        &self.cursor_defaults,
                    );
                    if let Some(target) = window_state.render.cursor.target_cloned() {
                        Self::update_frame_window_ime_cursor_area_if_needed(window_state, &target);
                    }
                    continue;
                }
            }
            if parent_id != 0 {
                if let Some(window_state) = self.frame_windows.get_mut(parent_id) {
                    window_state.render.child_frames.update_frame(frame);
                    Self::sync_frame_window_cursor(window_state, &self.cursor_defaults);
                    window_state.render.frame_dirty = true;
                    if let Some(target) = window_state.render.cursor.target_cloned() {
                        Self::update_frame_window_ime_cursor_area_if_needed(window_state, &target);
                    }
                    continue;
                }
            }

            if frame_id != 0 && parent_id == 0 && self.frame_windows.windows.contains_key(&frame_id)
            {
                self.frame_windows
                    .route_frame(frame, gui_menu_bar, gui_tool_bar, gui_compact_bar);
                continue;
            }
            if parent_id != 0 && self.frame_windows.windows.contains_key(&parent_id) {
                self.frame_windows.route_frame(frame, None, None, None);
                continue;
            }

            if parent_id != 0 {
                self.primary_child_frames_mut().update_frame(frame);
            } else {
                self.set_primary_current_frame(Some(frame));
                if let Some(menu_bar) = gui_menu_bar {
                    self.menu_bar = Some(menu_bar);
                } else {
                    self.menu_bar = None;
                    self.chrome_interaction.clear_menu_bar();
                }
                if let Some(tool_bar) = gui_tool_bar {
                    self.sync_toolbar_visual_config_from_height(tool_bar.height);
                    self.ensure_toolbar_icon_textures(&tool_bar.items);
                    self.tool_bar = Some(tool_bar);
                } else {
                    self.tool_bar = None;
                    self.chrome_interaction.clear_toolbar();
                }
                if let Some(compact_bar) = gui_compact_bar {
                    self.sync_toolbar_visual_config_from_height(compact_bar.height);
                    self.ensure_toolbar_icon_textures(&compact_bar.tool_items);
                    self.compact_bar = Some(compact_bar);
                } else {
                    self.compact_bar = None;
                    self.chrome_interaction.clear_compact_bar();
                }
                self.tab_bar = self
                    .primary_current_frame()
                    .and_then(|frame| frame.tab_bar.clone());
                if self.tab_bar.is_none() {
                    self.chrome_interaction.clear_tab_bar();
                }
                if let Some(cursor) = self.primary_cursor_mut() {
                    cursor.reset_blink();
                }
            }
            self.mark_primary_dirty();
        }

        let mut active_cursor: Option<CursorTarget> =
            self.primary_current_frame().and_then(|frame| {
                frame.phys_cursor.as_ref().map(|cursor| CursorTarget {
                    window_id: cursor.window_id,
                    x: cursor.x,
                    y: cursor.y,
                    width: cursor.width,
                    height: cursor.height,
                    style: cursor.style,
                    color: cursor.color,
                    frame_id: 0,
                })
            });

        if active_cursor.is_none() {
            for (_, entry) in &self.primary_child_frames().frames {
                if let Some(cursor) = entry.frame.phys_cursor.as_ref() {
                    active_cursor = Some(CursorTarget {
                        window_id: cursor.window_id,
                        x: cursor.x,
                        y: cursor.y,
                        width: cursor.width,
                        height: cursor.height,
                        style: cursor.style,
                        color: cursor.color,
                        frame_id: entry.frame_id,
                    });
                    break;
                }
            }
        }

        if let Some(new_target) = active_cursor {
            let (had_target, target_moved, old_cursor_rect) =
                if let Some(cursor) = self.primary_cursor_mut() {
                    let old_cursor_rect = (
                        cursor.current_x,
                        cursor.current_y,
                        cursor.current_w,
                        cursor.current_h,
                    );
                    let (had_target, target_moved) = cursor.set_target(new_target.clone());
                    (had_target, target_moved, old_cursor_rect)
                } else {
                    (false, false, (0.0, 0.0, 0.0, 0.0))
                };

            if target_moved && had_target && self.effects.typing_ripple.enabled {
                if let (Some(renderer), Some(primary_frame)) =
                    (self.renderer.as_ref(), self.primary_frame.as_mut())
                {
                    let cx = new_target.x + new_target.width / 2.0;
                    let cy = new_target.y + new_target.height / 2.0;
                    renderer.spawn_transient_ripple(&mut primary_frame.renderer_effects, cx, cy);
                }
            }

            if target_moved && had_target && self.effects.cursor_trail_fade.enabled {
                if let (Some(renderer), Some(primary_frame)) =
                    (self.renderer.as_ref(), self.primary_frame.as_mut())
                {
                    renderer.record_transient_cursor_trail(
                        &mut primary_frame.renderer_effects,
                        old_cursor_rect.0,
                        old_cursor_rect.1,
                        old_cursor_rect.2,
                        old_cursor_rect.3,
                    );
                }
            }

            self.update_ime_cursor_area_if_needed(&new_target);
        } else {
            if let Some(cursor) = self.primary_cursor_mut() {
                cursor.clear_target();
            }
            self.last_ime_cursor_area = None;
            self.clear_primary_ime_preedit();
            if let Some(window) = self.window.as_ref() {
                window.set_ime_cursor_area(
                    PhysicalPosition::new(0.0, 0.0),
                    PhysicalSize::new(1.0, 1.0),
                );
            }
        }

        let mut live_visual_cursor_ids = HashSet::new();
        let window_cursors = self
            .primary_current_frame()
            .map(|frame| frame.window_cursors.clone())
            .unwrap_or_default();
        for cursor in &window_cursors {
            if cursor.window_id >= 0 {
                continue;
            }
            live_visual_cursor_ids.insert(cursor.window_id);
            let target = CursorTarget {
                window_id: cursor.window_id,
                x: cursor.x,
                y: cursor.y,
                width: cursor.width,
                height: cursor.height,
                style: cursor.style,
                color: cursor.color,
                frame_id: 0,
            };
            let Some(primary_frame) = self.primary_frame.as_mut() else {
                continue;
            };
            let state = primary_frame
                .visual_cursors
                .entry(cursor.window_id)
                .or_default();
            state.anim_enabled = self.cursor_defaults.anim_enabled;
            state.anim_speed = self.cursor_defaults.anim_speed;
            state.anim_style = self.cursor_defaults.anim_style;
            state.anim_duration = self.cursor_defaults.anim_duration;
            state.trail_size = self.cursor_defaults.trail_size;
            state.size_transition_enabled = self.cursor_defaults.size_transition_enabled;
            state.size_transition_duration = self.cursor_defaults.size_transition_duration;
            let (_had_target, target_moved) = state.set_target(target);
            if target_moved {
                self.mark_primary_dirty();
            }
        }
        if let Some(primary_frame) = self.primary_frame.as_mut() {
            primary_frame
                .visual_cursors
                .retain(|id, _| live_visual_cursor_ids.contains(id));
        }
    }
}

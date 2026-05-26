//! Frame ingestion and cursor target extraction.

use super::RenderApp;
use crate::render_thread::cursor::CursorTarget;
use std::collections::HashSet;

impl RenderApp {
    /// Get latest frame from Emacs (non-blocking).
    pub(super) fn poll_frame(&mut self) {
        self.child_frames.tick();
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
                self.child_frames.update_frame(frame);
            } else {
                self.current_frame = Some(frame);
                if let Some(menu_bar) = gui_menu_bar {
                    self.menu_bar_items = menu_bar.items;
                    self.menu_bar_height = menu_bar.height;
                    self.menu_bar_fg = menu_bar.fg;
                    self.menu_bar_bg = menu_bar.bg;
                } else {
                    self.menu_bar_items.clear();
                    self.menu_bar_height = 0.0;
                    self.menu_bar_hovered = None;
                    self.menu_bar_active = None;
                }
                if let Some(tool_bar) = gui_tool_bar {
                    self.sync_toolbar_visual_config_from_height(tool_bar.height);
                    self.ensure_toolbar_icon_textures(&tool_bar.items);
                    self.toolbar_items = tool_bar.items;
                    self.toolbar_height = tool_bar.height;
                    self.toolbar_fg = tool_bar.fg;
                    self.toolbar_bg = tool_bar.bg;
                } else {
                    self.toolbar_items.clear();
                    self.toolbar_height = 0.0;
                    self.toolbar_hovered = None;
                    self.toolbar_pressed = None;
                }
                if let Some(compact_bar) = gui_compact_bar {
                    self.sync_toolbar_visual_config_from_height(compact_bar.height);
                    self.ensure_toolbar_icon_textures(&compact_bar.tool_items);
                    self.compact_bar_menu_items = compact_bar.menu_items;
                    self.compact_bar_tool_items = compact_bar.tool_items;
                    self.compact_bar_height = compact_bar.height;
                    self.compact_bar_menu_fg = compact_bar.menu_fg;
                    self.compact_bar_menu_bg = compact_bar.menu_bg;
                    self.compact_bar_tool_fg = compact_bar.tool_fg;
                    self.compact_bar_tool_bg = compact_bar.tool_bg;
                } else {
                    self.compact_bar_menu_items.clear();
                    self.compact_bar_tool_items.clear();
                    self.compact_bar_height = 0.0;
                    self.compact_bar_menu_hovered = None;
                    self.compact_bar_menu_active = None;
                    self.compact_bar_tool_hovered = None;
                    self.compact_bar_tool_pressed = None;
                }
                if let Some(tab_bar) = self
                    .current_frame
                    .as_ref()
                    .and_then(|frame| frame.tab_bar.as_ref())
                {
                    self.tab_bar_items = tab_bar.items.clone();
                    self.tab_bar_y = tab_bar.y;
                    self.tab_bar_height = tab_bar.height;
                } else {
                    self.tab_bar_items.clear();
                    self.tab_bar_y = 0.0;
                    self.tab_bar_height = 0.0;
                    self.tab_bar_hovered = None;
                    self.tab_bar_pressed = None;
                }
                self.cursor.reset_blink();
            }
            self.frame_dirty = true;
        }

        let mut active_cursor: Option<CursorTarget> =
            self.current_frame.as_ref().and_then(|frame| {
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
            for (_, entry) in &self.child_frames.frames {
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
            let (had_target, target_moved) = self.cursor.set_target(new_target.clone());

            if target_moved && had_target && self.effects.typing_ripple.enabled {
                if let Some(renderer) = self.renderer.as_mut() {
                    let cx = new_target.x + new_target.width / 2.0;
                    let cy = new_target.y + new_target.height / 2.0;
                    renderer.spawn_ripple(cx, cy);
                }
            }

            if target_moved && had_target && self.effects.cursor_trail_fade.enabled {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.record_cursor_trail(
                        self.cursor.current_x,
                        self.cursor.current_y,
                        self.cursor.current_w,
                        self.cursor.current_h,
                    );
                }
            }

            self.update_ime_cursor_area_if_needed(&new_target);
        }

        let mut live_visual_cursor_ids = HashSet::new();
        if let Some(frame) = self.current_frame.as_ref() {
            for cursor in &frame.window_cursors {
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
                let state = self.visual_cursors.entry(cursor.window_id).or_default();
                state.anim_enabled = self.cursor.anim_enabled;
                state.anim_speed = self.cursor.anim_speed;
                state.anim_style = self.cursor.anim_style;
                state.anim_duration = self.cursor.anim_duration;
                state.trail_size = self.cursor.trail_size;
                state.size_transition_enabled = self.cursor.size_transition_enabled;
                state.size_transition_duration = self.cursor.size_transition_duration;
                let (_had_target, target_moved) = state.set_target(target);
                if target_moved {
                    self.frame_dirty = true;
                }
            }
        }
        self.visual_cursors
            .retain(|id, _| live_visual_cursor_ids.contains(id));
    }
}

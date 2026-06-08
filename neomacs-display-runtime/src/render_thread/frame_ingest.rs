//! Frame ingestion and cursor target extraction.

use super::RenderApp;
use super::frame_windows::{GuiFrameRenderState, GuiFrameWindowState};
use crate::render_thread::cursor::CursorTarget;
use neomacs_display_protocol::glyph_matrix::{
    GuiCompactBarState, GuiMenuBarState, GuiToolBarState,
};

#[derive(Clone, Copy)]
struct CursorConfigSnapshot {
    blink_enabled: bool,
    blink_interval: std::time::Duration,
    anim_enabled: bool,
    anim_speed: f32,
    anim_style: crate::core::types::CursorAnimStyle,
    anim_duration: f32,
    trail_size: f32,
    size_transition_enabled: bool,
    size_transition_duration: f32,
}

impl CursorConfigSnapshot {
    fn from_cursor(cursor: &crate::render_thread::cursor::CursorState) -> Self {
        Self {
            blink_enabled: cursor.blink_enabled,
            blink_interval: cursor.blink_interval,
            anim_enabled: cursor.anim_enabled,
            anim_speed: cursor.anim_speed,
            anim_style: cursor.anim_style,
            anim_duration: cursor.anim_duration,
            trail_size: cursor.trail_size,
            size_transition_enabled: cursor.size_transition_enabled,
            size_transition_duration: cursor.size_transition_duration,
        }
    }

    fn apply_to(&self, cursor: &mut crate::render_thread::cursor::CursorState) {
        cursor.copy_config_from_values(
            self.blink_enabled,
            self.blink_interval,
            self.anim_enabled,
            self.anim_speed,
            self.anim_style,
            self.anim_duration,
            self.trail_size,
            self.size_transition_enabled,
            self.size_transition_duration,
        );
    }
}

struct CursorSyncOutcome {
    target: CursorTarget,
    had_target: bool,
    target_moved: bool,
    old_cursor_rect: (f32, f32, f32, f32),
}

impl RenderApp {
    fn ingest_frame_window_root_frame(
        window_state: &mut GuiFrameWindowState,
        frame: crate::core::frame_glyphs::FrameGlyphBuffer,
        menu_bar: Option<GuiMenuBarState>,
        tool_bar: Option<GuiToolBarState>,
        compact_bar: Option<GuiCompactBarState>,
        cursor_config: CursorConfigSnapshot,
    ) -> Option<CursorSyncOutcome> {
        if menu_bar.is_none() {
            window_state.render.chrome.interaction.clear_menu_bar();
        }
        if tool_bar.is_none() {
            window_state.render.chrome.interaction.clear_toolbar();
        }
        if compact_bar.is_none() {
            window_state.render.chrome.interaction.clear_compact_bar();
        }
        if frame.tab_bar.is_none() {
            window_state.render.chrome.interaction.clear_tab_bar();
        }

        let cursor_sync = Self::ingest_top_level_render_frame(
            &mut window_state.render,
            frame,
            menu_bar,
            tool_bar,
            compact_bar,
            cursor_config,
        );
        cursor_sync
    }

    fn ingest_top_level_render_frame(
        render: &mut GuiFrameRenderState,
        frame: crate::core::frame_glyphs::FrameGlyphBuffer,
        menu_bar: Option<GuiMenuBarState>,
        tool_bar: Option<GuiToolBarState>,
        compact_bar: Option<GuiCompactBarState>,
        cursor_config: CursorConfigSnapshot,
    ) -> Option<CursorSyncOutcome> {
        render.cursor.reset_blink();
        render.set_menu_bar(menu_bar);
        render.set_tool_bar(tool_bar);
        render.set_compact_bar(compact_bar);
        render.set_current_frame(Some(frame));
        let cursor_sync = Self::sync_render_cursor(render, cursor_config);
        render.sync_visual_cursors_from_current_frame(|cursor| cursor_config.apply_to(cursor));
        render.mark_dirty();
        cursor_sync
    }

    fn sync_render_cursor(
        render: &mut GuiFrameRenderState,
        cursor_config: CursorConfigSnapshot,
    ) -> Option<CursorSyncOutcome> {
        let mut active_cursor = render.compositor.current_frame.as_ref().and_then(|frame| {
            crate::render_thread::frame_windows::GuiFrameWindowManager::cursor_target_for_frame(
                render.emacs_frame_id,
                frame,
            )
        });

        if active_cursor.is_none() {
            for (_, entry) in &render.compositor.child_frames.frames {
                if let Some(cursor) = entry.frame.phys_cursor.as_ref() {
                    // Slide to the slot glyph's cell (where the static cursor
                    // draws), not the grid-approximate PhysCursor::x; see
                    // cursor_target_for_frame.
                    let x = entry
                        .frame
                        .slot_glyph(cursor.slot_id)
                        .and_then(|glyph| glyph.cell_x())
                        .unwrap_or(cursor.x);
                    active_cursor = Some(CursorTarget {
                        window_id: cursor.window_id,
                        x,
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

        cursor_config.apply_to(&mut render.cursor);
        if let Some(new_target) = active_cursor {
            let old_cursor_rect = (
                render.cursor.current_x,
                render.cursor.current_y,
                render.cursor.current_w,
                render.cursor.current_h,
            );
            let (had_target, target_moved) = render.cursor.set_target(new_target.clone());
            if target_moved {
                render.mark_dirty();
            }
            Some(CursorSyncOutcome {
                target: new_target,
                had_target,
                target_moved,
                old_cursor_rect,
            })
        } else {
            render.cursor.clear_target();
            render.clear_ime_preedit();
            None
        }
    }

    fn sync_frame_window_cursor(
        window_state: &mut GuiFrameWindowState,
        cursor_config: CursorConfigSnapshot,
    ) -> Option<CursorSyncOutcome> {
        let cursor_sync = Self::sync_render_cursor(&mut window_state.render, cursor_config);
        if window_state.render.cursor.target_cloned().is_none() {
            window_state.reset_ime_cursor_area();
        }
        cursor_sync
    }

    fn update_top_level_cursor_effects(
        renderer: Option<&neomacs_renderer_wgpu::WgpuRenderer>,
        render: &mut GuiFrameRenderState,
        new_target: &CursorTarget,
        had_target: bool,
        target_moved: bool,
        old_cursor_rect: (f32, f32, f32, f32),
        typing_ripple_enabled: bool,
        cursor_trail_fade_enabled: bool,
    ) {
        if target_moved
            && had_target
            && typing_ripple_enabled
            && let Some(renderer) = renderer
        {
            let cx = new_target.x + new_target.width / 2.0;
            let cy = new_target.y + new_target.height / 2.0;
            renderer.spawn_transient_ripple(&mut render.compositor.renderer_effects, cx, cy);
        }

        if target_moved
            && had_target
            && cursor_trail_fade_enabled
            && let Some(renderer) = renderer
        {
            renderer.record_transient_cursor_trail(
                &mut render.compositor.renderer_effects,
                old_cursor_rect.0,
                old_cursor_rect.1,
                old_cursor_rect.2,
                old_cursor_rect.3,
            );
        }
    }

    fn update_frame_window_cursor_side_effects(
        renderer: Option<&neomacs_renderer_wgpu::WgpuRenderer>,
        window_state: &mut GuiFrameWindowState,
        cursor_sync: CursorSyncOutcome,
        typing_ripple_enabled: bool,
        cursor_trail_fade_enabled: bool,
        update_transient_effects: bool,
    ) {
        if update_transient_effects {
            Self::update_top_level_cursor_effects(
                renderer,
                &mut window_state.render,
                &cursor_sync.target,
                cursor_sync.had_target,
                cursor_sync.target_moved,
                cursor_sync.old_cursor_rect,
                typing_ripple_enabled,
                cursor_trail_fade_enabled,
            );
        }
        Self::update_frame_window_ime_cursor_area_if_needed(window_state, &cursor_sync.target);
    }

    fn sync_gui_toolbar_assets(&mut self, tool_bar: Option<&GuiToolBarState>) {
        if let Some(tool_bar) = tool_bar {
            self.sync_toolbar_visual_config_from_height(tool_bar.height);
            self.ensure_toolbar_icon_textures(&tool_bar.items);
        }
    }

    fn sync_gui_compact_bar_assets(&mut self, compact_bar: Option<&GuiCompactBarState>) {
        if let Some(compact_bar) = compact_bar {
            self.sync_toolbar_visual_config_from_height(compact_bar.height);
            self.ensure_toolbar_icon_textures(&compact_bar.tool_items);
        }
    }

    /// Get latest frame from Emacs (non-blocking).
    pub(super) fn poll_frame(&mut self) {
        self.frame_windows.tick_top_level_child_frames();
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
                        crate::core::frame_glyphs::FrameGlyph::Xwidget { .. } => webkit_count += 1,
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

            if parent_id == 0 {
                let routed_to_managed = self.frame_windows.get(frame_id).is_some();
                let routed_to_primary_fallback = self.frame_windows.is_primary_frame_id(frame_id);
                if routed_to_managed {
                    self.sync_gui_toolbar_assets(gui_tool_bar.as_ref());
                    self.sync_gui_compact_bar_assets(gui_compact_bar.as_ref());
                }

                let update_transient_effects = routed_to_primary_fallback;
                let typing_ripple_enabled = self.effects.typing_ripple.enabled;
                let cursor_trail_fade_enabled = self.effects.cursor_trail_fade.enabled;
                let renderer = self.renderer.as_ref();
                if let Some(window_state) = self.frame_windows.get_mut(frame_id) {
                    let cursor_config = CursorConfigSnapshot::from_cursor(&self.cursor_defaults);
                    let cursor_sync = Self::ingest_frame_window_root_frame(
                        window_state,
                        frame,
                        gui_menu_bar,
                        gui_tool_bar,
                        gui_compact_bar,
                        cursor_config,
                    );
                    if let Some(cursor_sync) = cursor_sync {
                        Self::update_frame_window_cursor_side_effects(
                            renderer,
                            window_state,
                            cursor_sync,
                            typing_ripple_enabled,
                            cursor_trail_fade_enabled,
                            update_transient_effects,
                        );
                    } else {
                        window_state.reset_ime_cursor_area();
                    }
                    continue;
                }
            }

            if parent_id != 0 {
                let update_transient_effects = self.frame_windows.is_primary_frame_id(parent_id);
                let typing_ripple_enabled = self.effects.typing_ripple.enabled;
                let cursor_trail_fade_enabled = self.effects.cursor_trail_fade.enabled;
                let renderer = self.renderer.as_ref();
                if let Some(window_state) = self.frame_windows.get_mut(parent_id) {
                    let cursor_config = CursorConfigSnapshot::from_cursor(&self.cursor_defaults);
                    window_state
                        .render
                        .compositor
                        .child_frames
                        .update_frame(frame);
                    let cursor_sync = Self::sync_frame_window_cursor(window_state, cursor_config);
                    window_state.render.mark_dirty();
                    if let Some(cursor_sync) = cursor_sync {
                        Self::update_frame_window_cursor_side_effects(
                            renderer,
                            window_state,
                            cursor_sync,
                            typing_ripple_enabled,
                            cursor_trail_fade_enabled,
                            update_transient_effects,
                        );
                    }
                    continue;
                }
            }

            if parent_id != 0 && self.frame_windows.is_primary_frame_id(parent_id) {
                if let Some(ws) = self.frame_windows.primary_window_mut() {
                    ws.render.update_child_frame(frame)
                };
            } else if parent_id == 0 && self.frame_windows.is_primary_frame_id(frame_id) {
                if gui_menu_bar.is_none() {
                    if let Some(ws) = self.frame_windows.primary_window_mut() {
                        ws.render
                            .with_chrome_interaction_mut(|chrome| chrome.clear_menu_bar())
                    } else {
                        false
                    };
                }
                if let Some(tool_bar) = gui_tool_bar.as_ref() {
                    self.sync_toolbar_visual_config_from_height(tool_bar.height);
                    self.ensure_toolbar_icon_textures(&tool_bar.items);
                } else if gui_tool_bar.is_none() {
                    if let Some(ws) = self.frame_windows.primary_window_mut() {
                        ws.render
                            .with_chrome_interaction_mut(|chrome| chrome.clear_toolbar())
                    } else {
                        false
                    };
                }
                if let Some(compact_bar) = gui_compact_bar.as_ref() {
                    self.sync_toolbar_visual_config_from_height(compact_bar.height);
                    self.ensure_toolbar_icon_textures(&compact_bar.tool_items);
                } else if gui_compact_bar.is_none() {
                    if let Some(ws) = self.frame_windows.primary_window_mut() {
                        ws.render
                            .with_chrome_interaction_mut(|chrome| chrome.clear_compact_bar())
                    } else {
                        false
                    };
                }
                if self
                    .frame_windows
                    .primary_window()
                    .and_then(|ws| ws.render.compositor.current_frame.as_ref())
                    .and_then(|frame| frame.tab_bar.as_ref())
                    .is_none()
                {
                    if let Some(ws) = self.frame_windows.primary_window_mut() {
                        ws.render
                            .with_chrome_interaction_mut(|chrome| chrome.clear_tab_bar())
                    } else {
                        false
                    };
                }
                let cursor_config = CursorConfigSnapshot::from_cursor(&self.cursor_defaults);
                if let Some(primary_frame) = self
                    .frame_windows
                    .primary_window_mut()
                    .map(|ws| &mut ws.render)
                {
                    Self::ingest_top_level_render_frame(
                        primary_frame,
                        frame,
                        gui_menu_bar,
                        gui_tool_bar,
                        gui_compact_bar,
                        cursor_config,
                    );
                }
            }
        }

        let cursor_config = CursorConfigSnapshot::from_cursor(&self.cursor_defaults);
        if let Some(primary_frame) = self
            .frame_windows
            .primary_window_mut()
            .map(|ws| &mut ws.render)
        {
            let cursor_sync = Self::sync_render_cursor(primary_frame, cursor_config);
            primary_frame
                .sync_visual_cursors_from_current_frame(|cursor| cursor_config.apply_to(cursor));
            if let Some(cursor_sync) = cursor_sync {
                self.update_ime_cursor_area_if_needed(&cursor_sync.target);
            } else {
                if let Some(window_state) = self.frame_windows.primary_window_mut() {
                    window_state.reset_ime_cursor_area()
                };
                if let Some(ws) = self.frame_windows.primary_window_mut() {
                    ws.render.clear_ime_preedit()
                };
            }
        }
    }
}

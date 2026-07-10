//! Frame ingestion and cursor target extraction.

use super::RenderApp;
use super::frame_windows::{GuiFrameRenderState, GuiFrameWindowState};
use crate::core::types::DisplayFrameId;
use crate::render_thread::cursor::{CursorConfigSnapshot, CursorTarget};
use neomacs_display_protocol::glyph_matrix::{
    GuiCompactBarState, GuiMenuBarState, GuiToolBarState,
};

struct CursorSyncOutcome {
    target: CursorTarget,
    had_target: bool,
    target_moved: bool,
    old_cursor_rect: (f32, f32, f32, f32),
}

impl RenderApp {
    #[allow(clippy::too_many_arguments)]
    fn ingest_frame_window_root_frame(
        window_state: &mut GuiFrameWindowState,
        frame: crate::core::frame_glyphs::FrameGlyphBuffer,
        row_damage: neomacs_renderer_wgpu::FrameRowDamage,
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

        Self::ingest_top_level_render_frame(
            &mut window_state.render,
            frame,
            row_damage,
            menu_bar,
            tool_bar,
            compact_bar,
            cursor_config,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn ingest_top_level_render_frame(
        render: &mut GuiFrameRenderState,
        frame: crate::core::frame_glyphs::FrameGlyphBuffer,
        row_damage: neomacs_renderer_wgpu::FrameRowDamage,
        menu_bar: Option<GuiMenuBarState>,
        tool_bar: Option<GuiToolBarState>,
        compact_bar: Option<GuiCompactBarState>,
        cursor_config: CursorConfigSnapshot,
    ) -> Option<CursorSyncOutcome> {
        render.cursor.reset_blink();
        render.set_menu_bar(menu_bar);
        render.set_tool_bar(tool_bar);
        render.set_compact_bar(compact_bar);
        render.set_current_frame(Some(frame), Some(row_damage));
        let cursor_sync = Self::sync_render_cursor(render, cursor_config);
        render.sync_visual_cursors_from_current_frame(|cursor| cursor.apply_config(cursor_config));
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
            for entry in render.compositor.child_frames.frames.values() {
                if let Some(cursor) = entry.frame.active_cursor() {
                    // Resolve through the shared cursor_draw_rect (where the
                    // static cursor draws), not the grid-approximate cursor
                    // geometry; see cursor_target_for_frame.
                    let (x, y, width, height) = entry.frame.cursor_draw_rect(
                        cursor.slot_id,
                        cursor.style,
                        cursor.ascent,
                        (cursor.x, cursor.y, cursor.width, cursor.height),
                    );
                    active_cursor = Some(CursorTarget {
                        window_id: cursor.window_id.get(),
                        x,
                        y,
                        width,
                        height,
                        style: cursor.style,
                        frame_id: entry.frame_id,
                    });
                    break;
                }
            }
        }

        render.cursor.apply_config(cursor_config);
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
            // Row-damage summary for the renderer's vertex reuse. Built from
            // exactly this display_state (the one `frame` was materialized
            // from) so damage and glyphs can never describe different frames.
            let row_damage =
                neomacs_renderer_wgpu::FrameRowDamage::from_display_state(&display_state);

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
                // Role-aware breakdown: reconstruct chrome-row text + Y so we
                // can tell whether tab-line/header-line glyphs are emitted at
                // all (and where) vs. silently dropped.
                {
                    use crate::core::frame_glyphs::FrameGlyph;
                    let mut per_role: std::collections::HashMap<String, (usize, f32, String)> =
                        std::collections::HashMap::new();
                    for g in &frame.glyphs {
                        if let FrameGlyph::Char {
                            row_role,
                            char: ch,
                            y,
                            window_id,
                            ..
                        } = g
                        {
                            let key = format!("{:?}/win{}", row_role, window_id);
                            let e = per_role.entry(key).or_insert((0, *y, String::new()));
                            e.0 += 1;
                            e.1 = *y;
                            if e.2.len() < 60 {
                                e.2.push(*ch);
                            }
                        }
                    }
                    let mut tabline_total = 0usize;
                    for (role, (n, _, _)) in &per_role {
                        if role.starts_with("TabLine") {
                            tabline_total += n;
                        }
                    }
                    let mut keys: Vec<_> = per_role.keys().cloned().collect();
                    keys.sort();
                    for k in keys {
                        let (n, y, text) = &per_role[&k];
                        tracing::info!("DUMP_ROLE {k}: {n} chars y={y:.1} text=[{text}]");
                    }
                    tracing::info!("DUMP_ROLE tabline_char_total={tabline_total}");
                }
                let cursor_count = frame.window_cursors.len();
                let active_cursor_count = frame.window_cursors.iter().filter(|c| c.active).count();
                tracing::info!(
                    "poll_frame: frame_id={} parent_id={} size={:.0}x{:.0} char={:.1}x{:.1} \
                     glyphs={} (char={} bg={} border={} stretch={} scrollbar={} image={} video={} webkit={} other={}) \
                     windows={} window_cursors={} active_cursors={} faces={}",
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
                    active_cursor_count,
                    frame.faces.len(),
                );
                if let Some(cursor) = frame.active_cursor() {
                    tracing::info!(
                        "active_cursor: window_id={} slot=(window_id={},row={},col={}) \
                         rect=({:.2},{:.2}) {:.2}x{:.2} ascent={:.2} style={:?} color={:?} cursor_fg={:?}",
                        cursor.window_id.get(),
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
                            tracing::info!("active_cursor_slot_glyph: {:?}", slot_glyph)
                        }
                        None => tracing::warn!(
                            "active_cursor_slot_glyph: missing slot=(window_id={},row={},col={})",
                            cursor.slot_id.window_id,
                            cursor.slot_id.row,
                            cursor.slot_id.col,
                        ),
                    }
                    if let Some(effects) = frame.phys_cursor_effects() {
                        tracing::info!("active_cursor_effects: {:?}", effects);
                    }
                } else {
                    tracing::info!("active_cursor: none");
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
                                cursor.window_id.get(),
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

            if parent_id == DisplayFrameId::new(0) {
                let routed_to_managed = self.frame_windows.get(frame_id.get()).is_some();
                let routed_to_primary_fallback =
                    self.frame_windows.is_primary_frame_id(frame_id.get());
                if routed_to_managed {
                    self.sync_gui_toolbar_assets(gui_tool_bar.as_ref());
                    self.sync_gui_compact_bar_assets(gui_compact_bar.as_ref());
                }

                let update_transient_effects = routed_to_primary_fallback;
                let typing_ripple_enabled = self.effects.typing_ripple.enabled;
                let cursor_trail_fade_enabled = self.effects.cursor_trail_fade.enabled;
                let renderer = self.renderer.as_ref();
                if let Some(window_state) = self.frame_windows.get_mut(frame_id.get()) {
                    let cursor_config = self.cursor_defaults.config_snapshot();
                    let cursor_sync = Self::ingest_frame_window_root_frame(
                        window_state,
                        frame,
                        row_damage,
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

            if parent_id != DisplayFrameId::new(0) {
                tracing::debug!(
                    frame_id = frame_id.get(),
                    parent_frame_id = parent_id.get(),
                    width = frame.width,
                    height = frame.height,
                    parent_x = frame.parent_x,
                    parent_y = frame.parent_y,
                    glyphs = frame.glyphs.len(),
                    "child_frame_lifecycle: render_thread_child_frame_state_received"
                );
                let update_transient_effects =
                    self.frame_windows.is_primary_frame_id(parent_id.get());
                let typing_ripple_enabled = self.effects.typing_ripple.enabled;
                let cursor_trail_fade_enabled = self.effects.cursor_trail_fade.enabled;
                let renderer = self.renderer.as_ref();
                if let Some(window_state) = self.frame_windows.get_mut(parent_id.get()) {
                    let cursor_config = self.cursor_defaults.config_snapshot();
                    if window_state.render.update_child_frame(frame) {
                        let cursor_sync =
                            Self::sync_frame_window_cursor(window_state, cursor_config);
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
                    }
                    continue;
                }
            }

            if parent_id != DisplayFrameId::new(0)
                && self.frame_windows.is_primary_frame_id(parent_id.get())
            {
                if let Some(ws) = self.frame_windows.primary_window_mut() {
                    ws.render.update_child_frame(frame);
                };
            } else if parent_id == DisplayFrameId::new(0)
                && self.frame_windows.is_primary_frame_id(frame_id.get())
            {
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
                let cursor_config = self.cursor_defaults.config_snapshot();
                if let Some(primary_frame) = self
                    .frame_windows
                    .primary_window_mut()
                    .map(|ws| &mut ws.render)
                {
                    Self::ingest_top_level_render_frame(
                        primary_frame,
                        frame,
                        row_damage,
                        gui_menu_bar,
                        gui_tool_bar,
                        gui_compact_bar,
                        cursor_config,
                    );
                }
            }
        }

        let cursor_config = self.cursor_defaults.config_snapshot();
        if let Some(primary_frame) = self
            .frame_windows
            .primary_window_mut()
            .map(|ws| &mut ws.render)
        {
            let cursor_sync = Self::sync_render_cursor(primary_frame, cursor_config);
            primary_frame.sync_visual_cursors_from_current_frame(|cursor| {
                cursor.apply_config(cursor_config)
            });
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

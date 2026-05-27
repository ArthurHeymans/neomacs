use super::frame_windows::{GuiFrameNativeWindowState, GuiFrameRenderState, GuiFrameWindowState};
use super::state::{FpsCounter, TypingSpeedState};
use super::transitions::{
    detect_frame_transitions, ensure_frame_offscreen_textures, render_frame_transitions,
};
use super::{RenderApp, surface_readback};
use neomacs_renderer_wgpu::WgpuRenderer;

impl RenderApp {
    fn update_typing_speed_state(state: &mut TypingSpeedState) -> bool {
        let now = std::time::Instant::now();
        let window_secs = 5.0_f64;
        state
            .key_press_times
            .retain(|t| now.duration_since(*t).as_secs_f64() < window_secs);
        let count = state.key_press_times.len() as f64;
        let target_wpm = if count > 1.0 {
            let span = now.duration_since(state.key_press_times[0]).as_secs_f64();
            if span > 0.1 {
                (count / span) * 60.0 / 5.0
            } else {
                0.0
            }
        } else {
            0.0
        };
        state.displayed_wpm += (target_wpm as f32 - state.displayed_wpm) * 0.15;
        if state.displayed_wpm < 0.5 {
            state.displayed_wpm = 0.0;
        }
        state.displayed_wpm > 0.5 || !state.key_press_times.is_empty()
    }

    fn render_frame_common_overlays(
        renderer: &mut WgpuRenderer,
        surface_view: &wgpu::TextureView,
        frame: &crate::core::frame_glyphs::FrameGlyphBuffer,
        glyph_atlas: &mut neomacs_renderer_wgpu::WgpuGlyphAtlas,
        width: u32,
        height: u32,
        scroll_indicators_enabled: bool,
    ) {
        if renderer.effects.breadcrumb.enabled {
            renderer.render_breadcrumbs(surface_view, frame, glyph_atlas);
        }

        if scroll_indicators_enabled {
            renderer.render_scroll_indicators(surface_view, &frame.window_infos, width, height);
        }

        if renderer.effects.window_watermark.enabled {
            renderer.render_window_watermarks(surface_view, frame, glyph_atlas);
        }
    }

    fn render_frame_visual_bell_overlay(
        renderer: &WgpuRenderer,
        surface_view: &wgpu::TextureView,
        visual_bell_start: &mut Option<std::time::Instant>,
        frame_dirty: &mut bool,
        width: u32,
        height: u32,
    ) {
        if let Some(start) = *visual_bell_start {
            let elapsed = start.elapsed().as_secs_f32();
            let duration = 0.15;
            if elapsed < duration {
                let alpha = (1.0 - elapsed / duration) * 0.3;
                renderer.render_visual_bell(surface_view, width, height, alpha);
                *frame_dirty = true;
            } else {
                *visual_bell_start = None;
            }
        }
    }

    fn render_frame_fps_overlay(
        renderer: &WgpuRenderer,
        surface_view: &wgpu::TextureView,
        glyph_atlas: &mut neomacs_renderer_wgpu::WgpuGlyphAtlas,
        fps: &mut FpsCounter,
        glyph_count: usize,
        window_count: usize,
        transition_count: usize,
        width: u32,
        height: u32,
    ) -> bool {
        if !fps.enabled {
            return false;
        }

        let frame_time = fps.render_start.elapsed().as_secs_f32() * 1000.0;
        fps.frame_time_ms = fps.frame_time_ms * 0.9 + frame_time * 0.1;
        let stats_lines = vec![
            format!("{:.0} FPS | {:.1}ms", fps.display_value, fps.frame_time_ms),
            format!(
                "{}g {}w {}t  {}x{}",
                glyph_count, window_count, transition_count, width, height
            ),
        ];
        renderer.render_fps_overlay(surface_view, &stats_lines, glyph_atlas, width, height);
        true
    }

    fn render_frame_typing_speed_overlay(
        renderer: &WgpuRenderer,
        surface_view: &wgpu::TextureView,
        frame: &crate::core::frame_glyphs::FrameGlyphBuffer,
        glyph_atlas: &mut neomacs_renderer_wgpu::WgpuGlyphAtlas,
        typing_speed: &mut TypingSpeedState,
        frame_dirty: &mut bool,
    ) {
        let keep_redrawing = Self::update_typing_speed_state(typing_speed);
        renderer.render_typing_speed(surface_view, frame, glyph_atlas, typing_speed.displayed_wpm);
        if keep_redrawing {
            *frame_dirty = true;
        }
    }

    fn render_secondary_frame_window(
        renderer: &mut WgpuRenderer,
        faces: &std::collections::HashMap<u32, crate::core::face::Face>,
        window_state: &mut GuiFrameWindowState,
        bg_gradient: Option<((f32, f32, f32), (f32, f32, f32))>,
        child_frame_corner_radius: f32,
        child_frame_shadow_enabled: bool,
        child_frame_shadow_layers: u32,
        child_frame_shadow_offset: f32,
        child_frame_shadow_opacity: f32,
        scroll_indicators_enabled: bool,
    ) {
        let GuiFrameWindowState { native, render } = window_state;
        Self::update_fps_counter(&mut render.fps);
        let Some(frame_for_decision) = render.current_frame_clone() else {
            return;
        };
        let mut frame = frame_for_decision.clone();
        let animated_cursor = render.cursor.animated_cursor();
        let root_animated_cursor =
            animated_cursor.filter(|cursor| cursor.frame_id == render.emacs_frame_id);
        if let Some(cursor) = frame.phys_cursor.as_mut()
            && root_animated_cursor.is_some_and(|target| target.window_id == cursor.window_id)
        {
            cursor.x = render.cursor.current_x;
            cursor.y = render.cursor.current_y;
            cursor.width = render.cursor.current_w;
            cursor.height = render.cursor.current_h;
        }

        let need_offscreen = render.transitions.policy.needs_offscreen()
            || frame_for_decision.effect_hints.iter().any(|hint| {
                matches!(
                    hint,
                    crate::core::frame_glyphs::WindowEffectHint::ThemeTransition { .. }
                )
            });

        let output = match native.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(output)
            | wgpu::CurrentSurfaceTexture::Suboptimal(output) => output,
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                native
                    .surface
                    .configure(renderer.device(), &native.surface_config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                tracing::warn!(
                    "Surface validation error for frame 0x{:x}",
                    render.emacs_frame_id
                );
                return;
            }
        };

        let Some(drained_frame) = render.take_current_frame_for_render() else {
            return;
        };
        frame = drained_frame;

        let surface_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let old_scale_factor = renderer.scale_factor();
        let old_width = renderer.width();
        let old_height = renderer.height();
        renderer.set_scale_factor(native.scale_factor as f32);
        renderer.resize(native.width, native.height);
        let cursor_visible = render.cursor.blink_on;

        if need_offscreen {
            render.transitions.current_is_a = !render.transitions.current_is_a;
            ensure_frame_offscreen_textures(
                renderer,
                &mut render.transitions,
                native.width,
                native.height,
            );

            let current_view = if render.transitions.current_is_a {
                render
                    .transitions
                    .offscreen_a
                    .as_ref()
                    .map(|(_, view, _)| view as *const wgpu::TextureView)
            } else {
                render
                    .transitions
                    .offscreen_b
                    .as_ref()
                    .map(|(_, view, _)| view as *const wgpu::TextureView)
            };

            if let Some(current_view) = current_view {
                Self::render_secondary_frame_contents(
                    renderer,
                    faces,
                    native,
                    render,
                    unsafe { &*current_view },
                    &frame,
                    cursor_visible,
                    root_animated_cursor,
                    animated_cursor,
                    bg_gradient,
                    false,
                    child_frame_corner_radius,
                    child_frame_shadow_enabled,
                    child_frame_shadow_layers,
                    child_frame_shadow_offset,
                    child_frame_shadow_opacity,
                    scroll_indicators_enabled,
                );
            }

            let current_bg = if render.transitions.current_is_a {
                render
                    .transitions
                    .offscreen_a
                    .as_ref()
                    .map(|(_, _, bg)| bg as *const wgpu::BindGroup)
            } else {
                render
                    .transitions
                    .offscreen_b
                    .as_ref()
                    .map(|(_, _, bg)| bg as *const wgpu::BindGroup)
            };

            renderer.with_frame_effects(&mut render.renderer_effects, |renderer| {
                detect_frame_transitions(
                    renderer,
                    &mut render.transitions,
                    &renderer.effects.clone(),
                    &mut frame,
                    &mut render.frame_dirty,
                    native.width,
                    native.height,
                );
            });
            if render.renderer_effects.is_active() {
                render.frame_dirty = true;
            }

            if let Some(current_bg) = current_bg {
                renderer.blit_texture_to_view(
                    unsafe { &*current_bg },
                    &surface_view,
                    native.width,
                    native.height,
                );
            }
            render_frame_transitions(
                renderer,
                &mut render.transitions,
                &surface_view,
                native.width,
                native.height,
            );
            if render.transitions.has_active() {
                render.frame_dirty = true;
            }
            Self::render_secondary_frame_overlays(
                renderer,
                faces,
                native,
                render,
                &surface_view,
                &frame,
                cursor_visible,
                animated_cursor,
                child_frame_corner_radius,
                child_frame_shadow_enabled,
                child_frame_shadow_layers,
                child_frame_shadow_offset,
                child_frame_shadow_opacity,
                scroll_indicators_enabled,
            );
        } else {
            Self::render_secondary_frame_contents(
                renderer,
                faces,
                native,
                render,
                &surface_view,
                &frame,
                cursor_visible,
                root_animated_cursor,
                animated_cursor,
                bg_gradient,
                true,
                child_frame_corner_radius,
                child_frame_shadow_enabled,
                child_frame_shadow_layers,
                child_frame_shadow_offset,
                child_frame_shadow_opacity,
                scroll_indicators_enabled,
            );
            renderer.with_frame_effects(&mut render.renderer_effects, |renderer| {
                detect_frame_transitions(
                    renderer,
                    &mut render.transitions,
                    &renderer.effects.clone(),
                    &mut frame,
                    &mut render.frame_dirty,
                    native.width,
                    native.height,
                );
            });
            if render.renderer_effects.is_active() {
                render.frame_dirty = true;
            }
            if render.transitions.has_active() {
                render.frame_dirty = true;
            }
        }

        output.present();
        renderer.set_scale_factor(old_scale_factor);
        renderer.resize(old_width, old_height);
    }

    fn render_secondary_frame_overlays(
        renderer: &mut WgpuRenderer,
        faces: &std::collections::HashMap<u32, crate::core::face::Face>,
        native: &GuiFrameNativeWindowState,
        render: &mut GuiFrameRenderState,
        surface_view: &wgpu::TextureView,
        frame: &crate::core::frame_glyphs::FrameGlyphBuffer,
        cursor_visible: bool,
        animated_cursor: Option<crate::core::types::AnimatedCursor>,
        child_frame_corner_radius: f32,
        child_frame_shadow_enabled: bool,
        child_frame_shadow_layers: u32,
        child_frame_shadow_offset: f32,
        child_frame_shadow_opacity: f32,
        scroll_indicators_enabled: bool,
    ) {
        renderer.with_frame_effects(&mut render.renderer_effects, |renderer| {
            for &child_id in render.child_frames.sorted_for_rendering() {
                if let Some(child_entry) = render.child_frames.frames.get(&child_id) {
                    renderer.render_child_frame(
                        surface_view,
                        &child_entry.frame,
                        child_entry.abs_x,
                        child_entry.abs_y,
                        &mut render.glyph_atlas,
                        faces,
                        native.width,
                        native.height,
                        cursor_visible,
                        animated_cursor.filter(|ac| ac.frame_id == child_id),
                        child_frame_corner_radius,
                        child_frame_shadow_enabled,
                        child_frame_shadow_layers,
                        child_frame_shadow_offset,
                        child_frame_shadow_opacity,
                    );
                }
            }
        });
        if render.renderer_effects.is_active() {
            render.frame_dirty = true;
        }

        #[cfg(feature = "wpe-webkit")]
        if !render.floating_webkits.is_empty() {
            renderer.render_floating_webkits(surface_view, &render.floating_webkits);
        }

        renderer.with_frame_effects(&mut render.renderer_effects, |renderer| {
            Self::render_frame_common_overlays(
                renderer,
                surface_view,
                frame,
                &mut render.glyph_atlas,
                native.width,
                native.height,
                scroll_indicators_enabled,
            );
        });
        if render.renderer_effects.is_active() {
            render.frame_dirty = true;
        }

        if !native.chrome.decorations_enabled
            && !native.chrome.is_fullscreen
            && native.chrome.titlebar_height > 0.0
        {
            renderer.render_custom_titlebar(
                surface_view,
                &native.chrome.title,
                native.chrome.titlebar_height,
                native.chrome.titlebar_hover,
                Some((frame.background.r, frame.background.g, frame.background.b)),
                &mut render.glyph_atlas,
                native.width,
                native.height,
            );
        }

        if let Some(menu_bar) = render.menu_bar.as_ref() {
            if menu_bar.height > 0.0 && !menu_bar.items.is_empty() {
                renderer.render_menu_bar(
                    surface_view,
                    &menu_bar.items,
                    menu_bar.height,
                    menu_bar.fg,
                    menu_bar.bg,
                    render.chrome_interaction.menu_bar_hovered,
                    render.chrome_interaction.menu_bar_active,
                    &mut render.glyph_atlas,
                    native.width,
                    native.height,
                );
            }
        }

        if let Some(tool_bar) = render.tool_bar.as_ref() {
            if tool_bar.height > 0.0 && !tool_bar.items.is_empty() {
                renderer.render_toolbar(
                    surface_view,
                    &tool_bar.items,
                    frame
                        .tab_bar
                        .as_ref()
                        .filter(|tab_bar| tab_bar.height > 0.0)
                        .map(|tab_bar| tab_bar.y + tab_bar.height)
                        .unwrap_or_else(|| {
                            let menu_height = render
                                .menu_bar
                                .as_ref()
                                .map_or(0.0, |menu_bar| menu_bar.height);
                            let compact_height = render
                                .compact_bar
                                .as_ref()
                                .map_or(0.0, |compact_bar| compact_bar.height);
                            menu_height + compact_height
                        }),
                    tool_bar.height,
                    tool_bar.fg,
                    tool_bar.bg,
                    &std::collections::HashMap::new(),
                    render.chrome_interaction.toolbar_hovered,
                    render.chrome_interaction.toolbar_pressed,
                    24,
                    5,
                    native.width,
                    native.height,
                );
            }
        }

        if let Some(compact_bar) = render.compact_bar.as_ref() {
            if compact_bar.height > 0.0
                && (!compact_bar.menu_items.is_empty() || !compact_bar.tool_items.is_empty())
            {
                renderer.render_compact_bar(
                    surface_view,
                    &compact_bar.menu_items,
                    &compact_bar.tool_items,
                    compact_bar.height,
                    compact_bar.menu_fg,
                    compact_bar.menu_bg,
                    compact_bar.tool_fg,
                    compact_bar.tool_bg,
                    &std::collections::HashMap::new(),
                    render.chrome_interaction.compact_bar_menu_hovered,
                    render.chrome_interaction.compact_bar_menu_active,
                    render.chrome_interaction.compact_bar_tool_hovered,
                    render.chrome_interaction.compact_bar_tool_pressed,
                    24,
                    5,
                    &mut render.glyph_atlas,
                    native.width,
                    native.height,
                );
            }
        }

        if let Some(menu) = render.popup_menu.as_ref() {
            renderer.render_popup_menu(
                surface_view,
                menu,
                &mut render.glyph_atlas,
                native.width,
                native.height,
            );
        }

        if let Some(tooltip) = render.tooltip.as_ref() {
            renderer.render_tooltip(
                surface_view,
                tooltip,
                &mut render.glyph_atlas,
                native.width,
                native.height,
            );
        }

        if render.ime_preedit_active && !render.ime_preedit_text.is_empty() {
            if let Some(target) = render.cursor.target_cloned() {
                let (offset_x, offset_y) = if target.frame_id != render.emacs_frame_id {
                    render
                        .child_frames
                        .frames
                        .get(&target.frame_id)
                        .map(|entry| (entry.abs_x, entry.abs_y))
                        .unwrap_or((0.0, 0.0))
                } else {
                    (0.0, 0.0)
                };
                renderer.render_ime_preedit(
                    surface_view,
                    &render.ime_preedit_text,
                    target.x + offset_x,
                    target.y + offset_y,
                    target.height,
                    &mut render.glyph_atlas,
                    native.width,
                    native.height,
                );
            }
        }

        Self::render_frame_visual_bell_overlay(
            renderer,
            surface_view,
            &mut render.visual_bell_start,
            &mut render.frame_dirty,
            native.width,
            native.height,
        );

        if !native.chrome.decorations_enabled
            && !native.chrome.is_fullscreen
            && native.chrome.corner_radius > 0.0
        {
            renderer.render_corner_mask(
                surface_view,
                native.chrome.corner_radius,
                native.width,
                native.height,
            );
        }

        if Self::render_frame_fps_overlay(
            renderer,
            surface_view,
            &mut render.glyph_atlas,
            &mut render.fps,
            frame.glyphs.len(),
            frame.window_infos.len(),
            render.transitions.crossfades.len() + render.transitions.scroll_slides.len(),
            native.width,
            native.height,
        ) {
            render.frame_dirty = true;
        }

        if renderer.effects.typing_speed.enabled {
            Self::render_frame_typing_speed_overlay(
                renderer,
                surface_view,
                frame,
                &mut render.glyph_atlas,
                &mut render.typing_speed,
                &mut render.frame_dirty,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_secondary_frame_contents(
        renderer: &mut WgpuRenderer,
        faces: &std::collections::HashMap<u32, crate::core::face::Face>,
        native: &GuiFrameNativeWindowState,
        render: &mut GuiFrameRenderState,
        surface_view: &wgpu::TextureView,
        frame: &crate::core::frame_glyphs::FrameGlyphBuffer,
        cursor_visible: bool,
        root_animated_cursor: Option<crate::core::types::AnimatedCursor>,
        animated_cursor: Option<crate::core::types::AnimatedCursor>,
        bg_gradient: Option<((f32, f32, f32), (f32, f32, f32))>,
        include_overlays: bool,
        child_frame_corner_radius: f32,
        child_frame_shadow_enabled: bool,
        child_frame_shadow_layers: u32,
        child_frame_shadow_offset: f32,
        child_frame_shadow_opacity: f32,
        scroll_indicators_enabled: bool,
    ) {
        renderer.with_frame_effects(&mut render.renderer_effects, |renderer| {
            renderer.set_idle_dim_alpha(render.idle_dim.current_alpha);
            renderer.render_frame_glyphs(
                surface_view,
                frame,
                &mut render.glyph_atlas,
                faces,
                native.width,
                native.height,
                cursor_visible,
                root_animated_cursor,
                render.mouse_pos,
                bg_gradient,
            );
        });
        let renderer_effects_still_active = render.renderer_effects.is_active();

        if !include_overlays {
            render.frame_dirty = renderer_effects_still_active;
            return;
        }

        Self::render_secondary_frame_overlays(
            renderer,
            faces,
            native,
            render,
            surface_view,
            frame,
            cursor_visible,
            animated_cursor,
            child_frame_corner_radius,
            child_frame_shadow_enabled,
            child_frame_shadow_layers,
            child_frame_shadow_offset,
            child_frame_shadow_opacity,
            scroll_indicators_enabled,
        );
        if renderer_effects_still_active {
            render.frame_dirty = true;
        }
    }

    pub(super) fn render_frame_window(&mut self, emacs_frame_id: u64) {
        self.prepare_frame_state_for_render();

        let bg_gradient = if self.effects.bg_gradient.enabled {
            Some((
                self.effects.bg_gradient.top,
                self.effects.bg_gradient.bottom,
            ))
        } else {
            None
        };

        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        let Some(window_state) = self.frame_windows.get_mut(emacs_frame_id) else {
            return;
        };
        window_state.render.transitions.policy = self.transitions.policy;

        Self::render_secondary_frame_window(
            renderer,
            &self.faces,
            window_state,
            bg_gradient,
            self.child_frame_corner_radius,
            self.child_frame_shadow_enabled,
            self.child_frame_shadow_layers,
            self.child_frame_shadow_offset,
            self.child_frame_shadow_opacity,
            self.scroll_indicators_enabled,
        );
    }

    pub(super) fn render(&mut self) {
        // Early return checks
        if self.current_frame.is_none()
            || self.surface.is_none()
            || self.renderer.is_none()
            || self.glyph_atlas.is_none()
        {
            return;
        }

        self.prepare_frame_state_for_render();
        Self::update_fps_counter(&mut self.fps);

        // Get surface texture
        let Some(surface) = self.surface.as_ref() else {
            return;
        };
        let output = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(output)
            | wgpu::CurrentSurfaceTexture::Suboptimal(output) => output,
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                // Reconfigure surface
                let (w, h) = (self.width, self.height);
                self.handle_resize(w, h);
                return;
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                tracing::warn!("Surface validation error");
                return;
            }
        };

        let surface_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Build animated cursor override if applicable
        let animated_cursor = self.cursor.animated_cursor();
        let root_animated_cursor = animated_cursor.filter(|cursor| cursor.frame_id == 0);

        // Build background gradient option
        let bg_gradient = if self.effects.bg_gradient.enabled {
            Some((
                self.effects.bg_gradient.top,
                self.effects.bg_gradient.bottom,
            ))
        } else {
            None
        };

        // Check if we need offscreen rendering (for transitions)
        let need_offscreen = self.transitions.policy.needs_offscreen();

        if need_offscreen {
            // Swap: previous ← current
            self.transitions.current_is_a = !self.transitions.current_is_a;

            // Ensure offscreen textures exist
            self.ensure_offscreen_textures();

            // Render frame to current offscreen texture
            if let Some((current_view, _)) = self
                .current_offscreen_view_and_bg()
                .map(|(v, bg)| (v as *const wgpu::TextureView, bg))
            {
                let frame = self.current_frame.as_ref().expect("checked in render");
                let renderer = self.renderer.as_mut().expect("checked in render");
                let glyph_atlas = self.glyph_atlas.as_mut().expect("checked in render");

                // SAFETY: current_view is valid for the duration of this block
                renderer.set_idle_dim_alpha(self.idle_dim.current_alpha);
                renderer.render_frame_glyphs(
                    unsafe { &*current_view },
                    frame,
                    glyph_atlas,
                    &self.faces,
                    self.width,
                    self.height,
                    self.cursor.blink_on,
                    root_animated_cursor,
                    self.mouse_pos,
                    bg_gradient,
                );
            }

            // Detect transitions (compare window_infos)
            self.detect_transitions();

            // Blit current offscreen to surface
            if let Some((_, current_bg)) = self
                .current_offscreen_view_and_bg()
                .map(|(v, bg)| (v, bg as *const wgpu::BindGroup))
            {
                let renderer = self.renderer.as_ref().expect("checked in render");
                renderer.blit_texture_to_view(
                    unsafe { &*current_bg },
                    &surface_view,
                    self.width,
                    self.height,
                );
            }

            // Composite active transitions on top
            self.render_transitions(&surface_view);
        } else {
            // Simple path: render directly to surface
            let frame = self.current_frame.as_ref().expect("checked in render");
            let renderer = self.renderer.as_mut().expect("checked in render");
            let glyph_atlas = self.glyph_atlas.as_mut().expect("checked in render");

            renderer.set_idle_dim_alpha(self.idle_dim.current_alpha);
            renderer.render_frame_glyphs(
                &surface_view,
                frame,
                glyph_atlas,
                &self.faces,
                self.width,
                self.height,
                self.cursor.blink_on,
                root_animated_cursor,
                self.mouse_pos,
                bg_gradient,
            );
        }

        // Render child frames as floating overlays on top of the parent frame
        if !self.child_frames.is_empty() {
            for &child_id in self.child_frames.sorted_for_rendering() {
                if let Some(child_entry) = self.child_frames.frames.get(&child_id) {
                    if let (Some(renderer), Some(glyph_atlas)) =
                        (&self.renderer, &mut self.glyph_atlas)
                    {
                        // Pass animated cursor only if it belongs to this child frame
                        let child_anim = animated_cursor.filter(|ac| ac.frame_id == child_id);
                        renderer.render_child_frame(
                            &surface_view,
                            &child_entry.frame,
                            child_entry.abs_x,
                            child_entry.abs_y,
                            glyph_atlas,
                            &self.faces,
                            self.width,
                            self.height,
                            self.cursor.blink_on,
                            child_anim,
                            self.child_frame_corner_radius,
                            self.child_frame_shadow_enabled,
                            self.child_frame_shadow_layers,
                            self.child_frame_shadow_offset,
                            self.child_frame_shadow_opacity,
                        );
                    }
                }
            }
        }

        // Render floating WebKit overlays above frame contents but below GUI chrome.
        #[cfg(feature = "wpe-webkit")]
        if !self.floating_webkits.is_empty() {
            if let Some(ref renderer) = self.renderer {
                renderer.render_floating_webkits(&surface_view, &self.floating_webkits);
            }
        }

        if let (Some(renderer), Some(glyph_atlas), Some(frame)) = (
            &mut self.renderer,
            &mut self.glyph_atlas,
            &self.current_frame,
        ) {
            Self::render_frame_common_overlays(
                renderer,
                &surface_view,
                frame,
                glyph_atlas,
                self.width,
                self.height,
                self.scroll_indicators_enabled,
            );
        }

        // Render custom title bar when decorations are disabled (not in fullscreen)
        tracing::trace!(
            "CSD state: decorations_enabled={} is_fullscreen={} titlebar_height={}",
            self.chrome.decorations_enabled,
            self.chrome.is_fullscreen,
            self.chrome.titlebar_height
        );
        if !self.chrome.decorations_enabled
            && !self.chrome.is_fullscreen
            && self.chrome.titlebar_height > 0.0
        {
            if let (Some(renderer), Some(glyph_atlas)) = (&self.renderer, &mut self.glyph_atlas) {
                let frame_bg = self
                    .current_frame
                    .as_ref()
                    .map(|f| (f.background.r, f.background.g, f.background.b));
                renderer.render_custom_titlebar(
                    &surface_view,
                    &self.chrome.title,
                    self.chrome.titlebar_height,
                    self.chrome.titlebar_hover,
                    frame_bg,
                    glyph_atlas,
                    self.width,
                    self.height,
                );
            }
        }

        // Render menu bar overlay
        if self.menu_bar_height > 0.0 && !self.menu_bar_items.is_empty() {
            if let (Some(renderer), Some(glyph_atlas)) = (&self.renderer, &mut self.glyph_atlas) {
                renderer.render_menu_bar(
                    &surface_view,
                    &self.menu_bar_items,
                    self.menu_bar_height,
                    self.menu_bar_fg,
                    self.menu_bar_bg,
                    self.chrome_interaction.menu_bar_hovered,
                    self.chrome_interaction.menu_bar_active,
                    glyph_atlas,
                    self.width,
                    self.height,
                );
            }
        }

        // Tab bar is now rendered via the layout engine's status-line pipeline
        // (GlyphRowRole::TabBar) — no separate overlay needed.

        // Render toolbar overlay
        if self.toolbar_height > 0.0 && !self.toolbar_items.is_empty() {
            if let Some(ref renderer) = self.renderer {
                renderer.render_toolbar(
                    &surface_view,
                    &self.toolbar_items,
                    self.toolbar_y_origin(),
                    self.toolbar_height,
                    self.toolbar_fg,
                    self.toolbar_bg,
                    &self.toolbar_icon_textures,
                    self.chrome_interaction.toolbar_hovered,
                    self.chrome_interaction.toolbar_pressed,
                    self.toolbar_icon_size,
                    self.toolbar_padding,
                    self.width,
                    self.height,
                );
            }
        }

        if self.compact_bar_height > 0.0
            && (!self.compact_bar_menu_items.is_empty() || !self.compact_bar_tool_items.is_empty())
        {
            if let (Some(renderer), Some(glyph_atlas)) = (&self.renderer, &mut self.glyph_atlas) {
                renderer.render_compact_bar(
                    &surface_view,
                    &self.compact_bar_menu_items,
                    &self.compact_bar_tool_items,
                    self.compact_bar_height,
                    self.compact_bar_menu_fg,
                    self.compact_bar_menu_bg,
                    self.compact_bar_tool_fg,
                    self.compact_bar_tool_bg,
                    &self.toolbar_icon_textures,
                    self.chrome_interaction.compact_bar_menu_hovered,
                    self.chrome_interaction.compact_bar_menu_active,
                    self.chrome_interaction.compact_bar_tool_hovered,
                    self.chrome_interaction.compact_bar_tool_pressed,
                    self.toolbar_icon_size,
                    self.toolbar_padding,
                    glyph_atlas,
                    self.width,
                    self.height,
                );
            }
        }

        // Render popup menu overlay (topmost layer)
        if let Some(ref menu) = self.popup_menu {
            if let (Some(renderer), Some(glyph_atlas)) = (&self.renderer, &mut self.glyph_atlas) {
                renderer.render_popup_menu(
                    &surface_view,
                    menu,
                    glyph_atlas,
                    self.width,
                    self.height,
                );
            }
        }

        // Render tooltip overlay (above everything including popup menu)
        if let Some(ref tip) = self.tooltip {
            if let (Some(renderer), Some(glyph_atlas)) = (&self.renderer, &mut self.glyph_atlas) {
                renderer.render_tooltip(&surface_view, tip, glyph_atlas, self.width, self.height);
            }
        }

        // Render IME preedit text overlay at cursor position
        if self.ime_preedit_active && !self.ime_preedit_text.is_empty() {
            if let (Some(renderer), Some(glyph_atlas), Some(target)) = (
                &self.renderer,
                &mut self.glyph_atlas,
                self.cursor.target_cloned(),
            ) {
                let (offset_x, offset_y) = if target.frame_id != 0 {
                    self.child_frames
                        .frames
                        .get(&target.frame_id)
                        .map(|entry| (entry.abs_x, entry.abs_y))
                        .unwrap_or((0.0, 0.0))
                } else {
                    (0.0, 0.0)
                };
                renderer.render_ime_preedit(
                    &surface_view,
                    &self.ime_preedit_text,
                    target.x + offset_x,
                    target.y + offset_y,
                    target.height,
                    glyph_atlas,
                    self.width,
                    self.height,
                );
            }
        }

        if let Some(ref renderer) = self.renderer {
            Self::render_frame_visual_bell_overlay(
                renderer,
                &surface_view,
                &mut self.visual_bell_start,
                &mut self.frame_dirty,
                self.width,
                self.height,
            );
        }

        if let (Some(renderer), Some(glyph_atlas)) = (&self.renderer, &mut self.glyph_atlas) {
            Self::render_frame_fps_overlay(
                renderer,
                &surface_view,
                glyph_atlas,
                &mut self.fps,
                self.current_frame.as_ref().map_or(0, |f| f.glyphs.len()),
                self.current_frame
                    .as_ref()
                    .map_or(0, |f| f.window_infos.len()),
                self.transitions.crossfades.len() + self.transitions.scroll_slides.len(),
                self.width,
                self.height,
            );
        }

        if self.effects.typing_speed.enabled {
            if let (Some(renderer), Some(glyph_atlas), Some(frame)) =
                (&self.renderer, &mut self.glyph_atlas, &self.current_frame)
            {
                Self::render_frame_typing_speed_overlay(
                    renderer,
                    &surface_view,
                    frame,
                    glyph_atlas,
                    &mut self.typing_speed,
                    &mut self.frame_dirty,
                );
            }
        }

        // Render corner mask for rounded window corners (borderless only, not fullscreen)
        if !self.chrome.decorations_enabled
            && !self.chrome.is_fullscreen
            && self.chrome.corner_radius > 0.0
        {
            if let Some(ref renderer) = self.renderer {
                renderer.render_corner_mask(
                    &surface_view,
                    self.chrome.corner_radius,
                    self.width,
                    self.height,
                );
            }
        }

        if let (Some(renderer), Some(frame)) = (&self.renderer, &self.current_frame) {
            surface_readback::maybe_log_first_frame_surface_readback(
                &mut self.debug_first_frame_readback_pending,
                &output.texture,
                renderer,
                frame,
                self.width,
                self.height,
            );
            surface_readback::maybe_log_debug_surface_readback(
                &mut self.debug_surface_readback_frames_remaining,
                &output.texture,
                renderer,
                frame,
                self.width,
                self.height,
            );
        }

        // Present the frame
        output.present();
    }
}

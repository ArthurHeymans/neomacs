// Several render entry points carry the recurring `bg_gradient` RGB-pair tuple
// parameter, which mirrors the renderer-wgpu API surface; a local type alias
// would not be reused, so the type-complexity lint is allowed module-wide.
#![allow(clippy::type_complexity)]

use super::child_frames::ChildFrameManager;
use super::cursor::CursorTarget;
use super::frame_windows::{
    FrameLifecycle, GuiFrameNativeWindowState, GuiFrameRenderState, GuiFrameWindowState,
};
use super::state::{
    ChildFrameStyle, FpsCounter, GuiChromeInteractionState, ToolbarResources, TypingSpeedState,
    WindowChrome,
};
use super::transitions::{
    detect_frame_transitions, ensure_frame_offscreen_textures, render_frame_transitions,
};
use super::{RenderApp, surface_readback};
use crate::core::types::DisplayFrameId;
use crate::thread_comm::{MenuBarItem, ToolBarItem};
use neomacs_display_protocol::frame_chrome::{FrameChromeContent, FrameRect, PositionedChromeItem};
use neomacs_renderer_wgpu::{PopupMenuState, TooltipState, WgpuGlyphAtlas, WgpuRenderer};

/// Flatten a protocol [`Color`] into the legacy `(r, g, b)` tuple the
/// renderer's chrome-overlay draw fns still take. Alpha is dropped: GUI
/// chrome colors are opaque sRGB. Follow-up: migrate the overlay draw
/// fns themselves to `Color` and delete this.
fn color_rgb_tuple(color: neomacs_display_protocol::types::Color) -> (f32, f32, f32) {
    (color.r, color.g, color.b)
}

pub(super) fn frame_chrome_toolbar_bounds(
    frame: &crate::core::frame_glyphs::FrameGlyphBuffer,
) -> Option<FrameRect> {
    frame
        .frame_chrome
        .band(neomacs_display_protocol::frame_chrome::FrameChromeKind::ToolBar)
        .map(|band| band.bounds())
}

struct GuiFrameMenuBarOverlay<'a> {
    items: &'a [PositionedChromeItem<MenuBarItem>],
    bounds: FrameRect,
    fg: (f32, f32, f32),
    bg: (f32, f32, f32),
}

struct GuiFrameToolBarOverlay<'a> {
    items: &'a [PositionedChromeItem<ToolBarItem>],
    bounds: FrameRect,
    fg: (f32, f32, f32),
    bg: (f32, f32, f32),
    toolbar: &'a ToolbarResources,
    icon_size: u32,
    padding: u32,
}

struct GuiFrameCompactBarOverlay<'a> {
    menu_items: &'a [PositionedChromeItem<MenuBarItem>],
    tool_items: &'a [PositionedChromeItem<ToolBarItem>],
    bounds: FrameRect,
    menu_fg: (f32, f32, f32),
    menu_bg: (f32, f32, f32),
    tool_fg: (f32, f32, f32),
    tool_bg: (f32, f32, f32),
    toolbar: &'a ToolbarResources,
    icon_size: u32,
    padding: u32,
}

struct GuiFrameImeOverlay<'a> {
    text: &'a str,
    x: f32,
    y: f32,
    height: f32,
}

struct GuiFrameChromeOverlays<'a> {
    native_chrome: &'a WindowChrome,
    titlebar_background: Option<(f32, f32, f32)>,
    chrome_interaction: GuiChromeInteractionState,
    menu_bar: Option<GuiFrameMenuBarOverlay<'a>>,
    tool_bar: Option<GuiFrameToolBarOverlay<'a>>,
    compact_bar: Option<GuiFrameCompactBarOverlay<'a>>,
    popup_menu: Option<&'a PopupMenuState>,
    tooltip: Option<&'a TooltipState>,
    ime_preedit: Option<GuiFrameImeOverlay<'a>>,
}

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
        glyph_atlas: &mut WgpuGlyphAtlas,
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

    fn frame_ime_preedit_overlay<'a>(
        active: bool,
        text: &'a str,
        target: Option<CursorTarget>,
        root_frame_id: u64,
        child_frames: &ChildFrameManager,
    ) -> Option<GuiFrameImeOverlay<'a>> {
        if !active || text.is_empty() {
            return None;
        }

        let target = target?;
        let (offset_x, offset_y) = if target.frame_id != root_frame_id {
            child_frames
                .frames
                .get(&target.frame_id)
                .map(|entry| (entry.abs_x, entry.abs_y))
                .unwrap_or((0.0, 0.0))
        } else {
            (0.0, 0.0)
        };

        Some(GuiFrameImeOverlay {
            text,
            x: target.x + offset_x,
            y: target.y + offset_y,
            height: target.height,
        })
    }

    fn render_frame_chrome_overlays(
        renderer: &mut WgpuRenderer,
        surface_view: &wgpu::TextureView,
        glyph_atlas: &mut WgpuGlyphAtlas,
        overlays: GuiFrameChromeOverlays<'_>,
        width: u32,
        height: u32,
    ) {
        if !overlays.native_chrome.decorations_enabled
            && !overlays.native_chrome.is_fullscreen
            && overlays.native_chrome.titlebar_height > 0.0
        {
            renderer.render_custom_titlebar(
                surface_view,
                &overlays.native_chrome.title,
                overlays.native_chrome.titlebar_height,
                overlays.native_chrome.titlebar_hover,
                overlays.titlebar_background,
                glyph_atlas,
                width,
                height,
            );
        }

        if let Some(menu_bar) = overlays.menu_bar {
            renderer.render_menu_bar(
                surface_view,
                menu_bar.items,
                menu_bar.bounds,
                menu_bar.fg,
                menu_bar.bg,
                overlays.chrome_interaction.menu_bar_hovered,
                overlays.chrome_interaction.menu_bar_active,
                glyph_atlas,
                width,
                height,
            );
        }

        if let Some(tool_bar) = overlays.tool_bar {
            renderer.render_toolbar(
                surface_view,
                tool_bar.items,
                tool_bar.bounds,
                tool_bar.fg,
                tool_bar.bg,
                &tool_bar.toolbar.icon_textures,
                overlays.chrome_interaction.toolbar_hovered,
                overlays.chrome_interaction.toolbar_pressed,
                tool_bar.icon_size,
                tool_bar.padding,
                width,
                height,
            );
        }

        if let Some(compact_bar) = overlays.compact_bar {
            renderer.render_compact_bar(
                surface_view,
                compact_bar.menu_items,
                compact_bar.tool_items,
                compact_bar.bounds,
                compact_bar.menu_fg,
                compact_bar.menu_bg,
                compact_bar.tool_fg,
                compact_bar.tool_bg,
                &compact_bar.toolbar.icon_textures,
                overlays.chrome_interaction.compact_bar_menu_hovered,
                overlays.chrome_interaction.compact_bar_menu_active,
                overlays.chrome_interaction.compact_bar_tool_hovered,
                overlays.chrome_interaction.compact_bar_tool_pressed,
                compact_bar.icon_size,
                compact_bar.padding,
                glyph_atlas,
                width,
                height,
            );
        }

        if let Some(menu) = overlays.popup_menu {
            renderer.render_popup_menu(surface_view, menu, glyph_atlas, width, height);
        }

        if let Some(tooltip) = overlays.tooltip {
            renderer.render_tooltip(surface_view, tooltip, glyph_atlas, width, height);
        }

        if let Some(preedit) = overlays.ime_preedit {
            renderer.render_ime_preedit(
                surface_view,
                preedit.text,
                preedit.x,
                preedit.y,
                preedit.height,
                glyph_atlas,
                width,
                height,
            );
        }
    }

    fn render_frame_corner_mask(
        renderer: &mut WgpuRenderer,
        surface_view: &wgpu::TextureView,
        chrome: &WindowChrome,
        width: u32,
        height: u32,
    ) {
        if !chrome.decorations_enabled && !chrome.is_fullscreen && chrome.corner_radius > 0.0 {
            renderer.render_corner_mask(surface_view, chrome.corner_radius, width, height);
        }
    }

    fn render_frame_visual_bell_overlay(
        renderer: &mut WgpuRenderer,
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
        renderer: &mut WgpuRenderer,
        surface_view: &wgpu::TextureView,
        glyph_atlas: &mut WgpuGlyphAtlas,
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
        renderer: &mut WgpuRenderer,
        surface_view: &wgpu::TextureView,
        frame: &crate::core::frame_glyphs::FrameGlyphBuffer,
        glyph_atlas: &mut WgpuGlyphAtlas,
        typing_speed: &mut TypingSpeedState,
        frame_dirty: &mut bool,
    ) {
        let keep_redrawing = Self::update_typing_speed_state(typing_speed);
        renderer.render_typing_speed(surface_view, frame, glyph_atlas, typing_speed.displayed_wpm);
        if keep_redrawing {
            *frame_dirty = true;
        }
    }

    fn render_frame_window_contents_to_surface(
        renderer: &mut WgpuRenderer,
        window_state: &mut GuiFrameWindowState,
        bg_gradient: Option<((f32, f32, f32), (f32, f32, f32))>,
        child_frame_style: &ChildFrameStyle,
        scroll_indicators_enabled: bool,
        toolbar: &ToolbarResources,
        extra_line_spacing: f32,
        extra_letter_spacing: f32,
    ) -> Option<(
        wgpu::SurfaceTexture,
        crate::core::frame_glyphs::FrameGlyphBuffer,
    )> {
        Self::render_frame_window_contents_to_acquired_surface(
            renderer,
            window_state,
            bg_gradient,
            child_frame_style,
            scroll_indicators_enabled,
            toolbar,
            extra_line_spacing,
            extra_letter_spacing,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_frame_window_contents_to_acquired_surface(
        renderer: &mut WgpuRenderer,
        window_state: &mut GuiFrameWindowState,
        bg_gradient: Option<((f32, f32, f32), (f32, f32, f32))>,
        child_frame_style: &ChildFrameStyle,
        scroll_indicators_enabled: bool,
        toolbar: &ToolbarResources,
        extra_line_spacing: f32,
        extra_letter_spacing: f32,
        output: Option<wgpu::SurfaceTexture>,
    ) -> Option<(
        wgpu::SurfaceTexture,
        crate::core::frame_glyphs::FrameGlyphBuffer,
    )> {
        let render = &mut window_state.render;
        let native = match &mut window_state.lifecycle {
            FrameLifecycle::Active { native, .. } => native,
            _ => return None,
        };
        Self::update_fps_counter(&mut render.overlays.fps);
        let frame_for_decision = render.current_frame_clone()?;
        let mut frame = frame_for_decision.clone();
        if extra_line_spacing != 0.0 || extra_letter_spacing != 0.0 {
            Self::apply_extra_spacing(
                &mut frame.glyphs,
                &mut frame.window_cursors,
                extra_line_spacing,
                extra_letter_spacing,
            );
        }
        let animated_cursor = render.cursor.animated_cursor();
        let root_animated_cursor = animated_cursor
            .filter(|cursor| cursor.frame_id == DisplayFrameId::new(render.emacs_frame_id));
        // The slide animation is composed at draw time: emit_cursor_visual reads
        // the interpolated rect from animated_cursor for the active window's
        // cursor. The frame's stored cursor geometry is no longer mutated here,
        // so the materialized frame stays a pure function of the layout snapshot.

        let need_offscreen = render.compositor.transitions.policy.needs_offscreen()
            || frame_for_decision.effect_hints.iter().any(|hint| {
                matches!(
                    hint,
                    crate::core::frame_glyphs::WindowEffectHint::ThemeTransition { .. }
                )
            });

        let output = if let Some(output) = output {
            output
        } else {
            match native.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(output)
                | wgpu::CurrentSurfaceTexture::Suboptimal(output) => output,
                wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                    tracing::info!(
                        "Skipping redraw for frame 0x{:x}: surface lost or outdated",
                        render.emacs_frame_id
                    );
                    return None;
                }
                wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                    return None;
                }
                wgpu::CurrentSurfaceTexture::Validation => {
                    tracing::warn!(
                        "Surface validation error for frame 0x{:x}",
                        render.emacs_frame_id
                    );
                    return None;
                }
            }
        };

        let drained_frame = render.take_current_frame_for_render()?;
        render.begin_presentable_render();
        frame = drained_frame;
        if extra_line_spacing != 0.0 || extra_letter_spacing != 0.0 {
            Self::apply_extra_spacing(
                &mut frame.glyphs,
                &mut frame.window_cursors,
                extra_line_spacing,
                extra_letter_spacing,
            );
        }

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
            render.compositor.transitions.current_is_a =
                !render.compositor.transitions.current_is_a;
            ensure_frame_offscreen_textures(
                renderer,
                &mut render.compositor.transitions,
                native.width,
                native.height,
            );

            let current_view = if render.compositor.transitions.current_is_a {
                render
                    .compositor
                    .transitions
                    .offscreen_a
                    .as_ref()
                    .map(|(_, view, _)| view.clone())
            } else {
                render
                    .compositor
                    .transitions
                    .offscreen_b
                    .as_ref()
                    .map(|(_, view, _)| view.clone())
            };

            if let Some(current_view) = current_view {
                Self::render_frame_window_contents(
                    renderer,
                    native,
                    render,
                    &current_view,
                    &frame,
                    cursor_visible,
                    root_animated_cursor,
                    animated_cursor,
                    bg_gradient,
                    false,
                    child_frame_style,
                    scroll_indicators_enabled,
                    toolbar,
                );
            }

            let current_bg = if render.compositor.transitions.current_is_a {
                render
                    .compositor
                    .transitions
                    .offscreen_a
                    .as_ref()
                    .map(|(_, _, bg)| bg.clone())
            } else {
                render
                    .compositor
                    .transitions
                    .offscreen_b
                    .as_ref()
                    .map(|(_, _, bg)| bg.clone())
            };

            renderer.with_frame_effects(&mut render.compositor.renderer_effects, |renderer| {
                detect_frame_transitions(
                    renderer,
                    &mut render.compositor.transitions,
                    &renderer.effects.clone(),
                    &mut frame,
                    &mut render.compositor.dirty,
                    native.width,
                    native.height,
                );
            });
            if render.compositor.renderer_effects.needs_redraw() {
                render.mark_dirty();
            }

            if let Some(current_bg) = current_bg {
                renderer.blit_texture_to_view(
                    &current_bg,
                    &surface_view,
                    native.width,
                    native.height,
                );
            }
            render_frame_transitions(
                renderer,
                &mut render.compositor.transitions,
                &surface_view,
                native.width,
                native.height,
            );
            if render.compositor.transitions.has_active() {
                render.mark_dirty();
            }
            Self::render_frame_window_overlays_with_toolbar_resources(
                renderer,
                native,
                render,
                &surface_view,
                &frame,
                cursor_visible,
                animated_cursor,
                child_frame_style,
                scroll_indicators_enabled,
                toolbar,
            );
        } else {
            Self::render_frame_window_contents(
                renderer,
                native,
                render,
                &surface_view,
                &frame,
                cursor_visible,
                root_animated_cursor,
                animated_cursor,
                bg_gradient,
                true,
                child_frame_style,
                scroll_indicators_enabled,
                toolbar,
            );
            renderer.with_frame_effects(&mut render.compositor.renderer_effects, |renderer| {
                detect_frame_transitions(
                    renderer,
                    &mut render.compositor.transitions,
                    &renderer.effects.clone(),
                    &mut frame,
                    &mut render.compositor.dirty,
                    native.width,
                    native.height,
                );
            });
            render.mark_active_visuals_dirty();
        }

        renderer.set_scale_factor(old_scale_factor);
        renderer.resize(old_width, old_height);
        Some((output, frame))
    }

    #[allow(clippy::too_many_arguments)]
    fn render_frame_window_overlays_with_toolbar_resources(
        renderer: &mut WgpuRenderer,
        native: &GuiFrameNativeWindowState,
        render: &mut GuiFrameRenderState,
        surface_view: &wgpu::TextureView,
        frame: &crate::core::frame_glyphs::FrameGlyphBuffer,
        cursor_visible: bool,
        animated_cursor: Option<crate::core::types::AnimatedCursor>,
        child_frame_style: &ChildFrameStyle,
        scroll_indicators_enabled: bool,
        toolbar: &ToolbarResources,
    ) {
        Self::render_frame_content_overlays(
            renderer,
            native,
            render,
            surface_view,
            frame,
            cursor_visible,
            animated_cursor,
            child_frame_style,
            scroll_indicators_enabled,
        );

        let menu_bar = frame.frame_chrome.bands().iter().find_map(|band| {
            let FrameChromeContent::MenuBar(content) = band.content() else {
                return None;
            };
            Some((band.bounds(), content))
        });
        let tool_bar_content = frame.frame_chrome.bands().iter().find_map(|band| {
            let FrameChromeContent::ToolBar(content) = band.content() else {
                return None;
            };
            Some(content)
        });
        let tool_bar = frame_chrome_toolbar_bounds(frame).zip(tool_bar_content);
        let compact_bar = frame.frame_chrome.bands().iter().find_map(|band| {
            let FrameChromeContent::CompactBar(content) = band.content() else {
                return None;
            };
            Some((band.bounds(), content))
        });
        Self::render_frame_chrome_overlays(
            renderer,
            surface_view,
            render.compositor.glyph_atlas.as_mut().unwrap(),
            GuiFrameChromeOverlays {
                native_chrome: &native.chrome,
                titlebar_background: Some((
                    frame.background.r,
                    frame.background.g,
                    frame.background.b,
                )),
                chrome_interaction: render.chrome.interaction,
                menu_bar: menu_bar.map(|(bounds, menu_bar)| GuiFrameMenuBarOverlay {
                    items: menu_bar.items(),
                    bounds,
                    fg: color_rgb_tuple(menu_bar.foreground()),
                    bg: color_rgb_tuple(menu_bar.background()),
                }),
                tool_bar: tool_bar.map(|(bounds, tool_bar)| GuiFrameToolBarOverlay {
                    items: tool_bar.items(),
                    bounds,
                    fg: color_rgb_tuple(tool_bar.foreground()),
                    bg: color_rgb_tuple(tool_bar.background()),
                    toolbar,
                    icon_size: tool_bar.icon_size(),
                    padding: tool_bar.padding(),
                }),
                compact_bar: compact_bar.map(|(bounds, compact_bar)| GuiFrameCompactBarOverlay {
                    menu_items: compact_bar.menu_items(),
                    tool_items: compact_bar.tool_items(),
                    bounds,
                    menu_fg: color_rgb_tuple(compact_bar.menu_foreground()),
                    menu_bg: color_rgb_tuple(compact_bar.menu_background()),
                    tool_fg: color_rgb_tuple(compact_bar.tool_foreground()),
                    tool_bg: color_rgb_tuple(compact_bar.tool_background()),
                    toolbar,
                    icon_size: compact_bar.icon_size(),
                    padding: compact_bar.padding(),
                }),
                popup_menu: render.overlays.popup_menu.as_ref(),
                tooltip: render.overlays.tooltip.as_ref(),
                ime_preedit: Self::frame_ime_preedit_overlay(
                    render.overlays.ime_preedit_active,
                    &render.overlays.ime_preedit_text,
                    render.cursor.target_cloned(),
                    render.emacs_frame_id,
                    &render.compositor.child_frames,
                ),
            },
            native.width,
            native.height,
        );

        Self::render_frame_visual_bell_overlay(
            renderer,
            surface_view,
            &mut render.overlays.visual_bell_start,
            &mut render.compositor.dirty,
            native.width,
            native.height,
        );

        Self::render_frame_corner_mask(
            renderer,
            surface_view,
            &native.chrome,
            native.width,
            native.height,
        );

        if Self::render_frame_fps_overlay(
            renderer,
            surface_view,
            render.compositor.glyph_atlas.as_mut().unwrap(),
            &mut render.overlays.fps,
            frame.glyphs.len(),
            frame.window_infos.len(),
            render.compositor.transitions.crossfades.len()
                + render.compositor.transitions.scroll_slides.len(),
            native.width,
            native.height,
        ) {
            render.mark_dirty();
        }

        if renderer.effects.typing_speed.enabled {
            Self::render_frame_typing_speed_overlay(
                renderer,
                surface_view,
                frame,
                render.compositor.glyph_atlas.as_mut().unwrap(),
                &mut render.overlays.typing_speed,
                &mut render.compositor.dirty,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_frame_root_glyphs(
        renderer: &mut WgpuRenderer,
        native: &GuiFrameNativeWindowState,
        render: &mut GuiFrameRenderState,
        surface_view: &wgpu::TextureView,
        frame: &crate::core::frame_glyphs::FrameGlyphBuffer,
        cursor_visible: bool,
        root_animated_cursor: Option<crate::core::types::AnimatedCursor>,
        bg_gradient: Option<((f32, f32, f32), (f32, f32, f32))>,
    ) {
        super::frame_stats::count(&super::frame_stats::ROOT_GLYPH_PASSES);
        if let Some(atlas) = render.compositor.glyph_atlas.as_mut() {
            atlas.set_current_frame_fonts(&frame.fonts, &frame.char_fonts, &frame.shaped_clusters);
        }
        renderer.with_frame_effects(&mut render.compositor.renderer_effects, |renderer| {
            renderer.set_idle_dim_alpha(render.overlays.idle_dim.current_alpha);
            renderer.render_frame_glyphs(
                surface_view,
                frame,
                render.compositor.glyph_atlas.as_mut().unwrap(),
                native.width,
                native.height,
                cursor_visible,
                root_animated_cursor,
                render.mouse_pos,
                bg_gradient,
                render.compositor.current_row_damage.as_ref(),
            );
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn render_frame_content_overlays(
        renderer: &mut WgpuRenderer,
        native: &GuiFrameNativeWindowState,
        render: &mut GuiFrameRenderState,
        surface_view: &wgpu::TextureView,
        frame: &crate::core::frame_glyphs::FrameGlyphBuffer,
        cursor_visible: bool,
        animated_cursor: Option<crate::core::types::AnimatedCursor>,
        child_frame_style: &ChildFrameStyle,
        scroll_indicators_enabled: bool,
    ) {
        renderer.with_frame_effects(&mut render.compositor.renderer_effects, |renderer| {
            for &child_id in render.compositor.child_frames.sorted_for_rendering() {
                if let Some(child_entry) = render.compositor.child_frames.frames.get(&child_id) {
                    if let Some(atlas) = render.compositor.glyph_atlas.as_mut() {
                        atlas.set_current_frame_fonts(
                            &child_entry.frame.fonts,
                            &child_entry.frame.char_fonts,
                            &child_entry.frame.shaped_clusters,
                        );
                    }
                    tracing::debug!(
                        parent_frame_id = render.emacs_frame_id,
                        frame_id = child_id,
                        x = child_entry.abs_x,
                        y = child_entry.abs_y,
                        width = child_entry.frame.width,
                        height = child_entry.frame.height,
                        glyphs = child_entry.frame.glyphs.len(),
                        "child_frame_lifecycle: render_child_frame_start"
                    );
                    renderer.render_child_frame(
                        surface_view,
                        &child_entry.frame,
                        child_entry.abs_x,
                        child_entry.abs_y,
                        render.compositor.glyph_atlas.as_mut().unwrap(),
                        native.width,
                        native.height,
                        cursor_visible,
                        animated_cursor.filter(|ac| ac.frame_id == DisplayFrameId::new(child_id)),
                        child_frame_style.corner_radius,
                        child_frame_style.shadow_enabled,
                        child_frame_style.shadow_layers,
                        child_frame_style.shadow_offset,
                        child_frame_style.shadow_opacity,
                    );
                    tracing::debug!(
                        parent_frame_id = render.emacs_frame_id,
                        frame_id = child_id,
                        "child_frame_lifecycle: render_child_frame_done"
                    );
                }
            }
        });
        if render.compositor.renderer_effects.needs_redraw() {
            render.mark_dirty();
        }

        if let Some(atlas) = render.compositor.glyph_atlas.as_mut() {
            atlas.set_current_frame_fonts(&frame.fonts, &frame.char_fonts, &frame.shaped_clusters);
        }

        #[cfg(feature = "wpe-webkit")]
        if !render.floating_webkits.is_empty() {
            renderer.render_floating_webkits(surface_view, &render.floating_webkits);
        }

        renderer.with_frame_effects(&mut render.compositor.renderer_effects, |renderer| {
            Self::render_frame_common_overlays(
                renderer,
                surface_view,
                frame,
                render.compositor.glyph_atlas.as_mut().unwrap(),
                native.width,
                native.height,
                scroll_indicators_enabled,
            );
        });
        if render.compositor.renderer_effects.needs_redraw() {
            render.mark_dirty();
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_frame_window_contents(
        renderer: &mut WgpuRenderer,
        native: &GuiFrameNativeWindowState,
        render: &mut GuiFrameRenderState,
        surface_view: &wgpu::TextureView,
        frame: &crate::core::frame_glyphs::FrameGlyphBuffer,
        cursor_visible: bool,
        root_animated_cursor: Option<crate::core::types::AnimatedCursor>,
        animated_cursor: Option<crate::core::types::AnimatedCursor>,
        bg_gradient: Option<((f32, f32, f32), (f32, f32, f32))>,
        include_overlays: bool,
        child_frame_style: &ChildFrameStyle,
        scroll_indicators_enabled: bool,
        toolbar: &ToolbarResources,
    ) {
        Self::render_frame_root_glyphs(
            renderer,
            native,
            render,
            surface_view,
            frame,
            cursor_visible,
            root_animated_cursor,
            bg_gradient,
        );
        let renderer_effects_still_active = render.compositor.renderer_effects.needs_redraw();

        if !include_overlays {
            render.set_dirty(renderer_effects_still_active);
            return;
        }

        Self::render_frame_window_overlays_with_toolbar_resources(
            renderer,
            native,
            render,
            surface_view,
            frame,
            cursor_visible,
            animated_cursor,
            child_frame_style,
            scroll_indicators_enabled,
            toolbar,
        );
        if renderer_effects_still_active {
            render.mark_dirty();
        }
    }

    pub(super) fn render_frame_window(&mut self, emacs_frame_id: u64) {
        if self.lifecycle_flags.shutdown_requested {
            return;
        }
        self.prepare_frame_state_for_render();

        let bg_gradient = if self.effects.bg_gradient.enabled {
            Some((
                self.effects.bg_gradient.top,
                self.effects.bg_gradient.bottom,
            ))
        } else {
            None
        };

        let is_primary_frame = self.frame_windows.is_primary_frame_id(emacs_frame_id);
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        let Some(window_state) = self.frame_windows.get_mut(emacs_frame_id) else {
            return;
        };
        window_state.render.compositor.transitions.policy = self.transition_policy;

        if let Some((output, frame)) = Self::render_frame_window_contents_to_surface(
            renderer,
            window_state,
            bg_gradient,
            &self.child_frame_style,
            self.scroll_indicators_enabled,
            &self.toolbar,
            self.extra_line_spacing,
            self.extra_letter_spacing,
        ) {
            if is_primary_frame {
                let (w, h) = self
                    .frame_windows
                    .get(emacs_frame_id)
                    .map(|ws| ws.native_size())
                    .unwrap_or((0, 0));
                surface_readback::maybe_log_first_frame_surface_readback(
                    &mut self.debug_first_frame_readback_pending,
                    &output.texture,
                    renderer,
                    &frame,
                    w,
                    h,
                );
                surface_readback::maybe_log_debug_surface_readback(
                    &mut self.debug_surface_readback_frames_remaining,
                    &output.texture,
                    renderer,
                    &frame,
                    w,
                    h,
                );
            }
            let (child_frame_ids, removed_child_frame_ids) = self
                .frame_windows
                .get_mut(emacs_frame_id)
                .map(|window_state| {
                    let child_frame_ids = window_state
                        .render
                        .compositor
                        .child_frames
                        .sorted_for_rendering()
                        .to_vec();
                    let removed_child_frame_ids = std::mem::take(
                        &mut window_state
                            .render
                            .compositor
                            .pending_child_frame_removals_to_present,
                    );
                    (child_frame_ids, removed_child_frame_ids)
                })
                .unwrap_or_default();
            if !child_frame_ids.is_empty() || !removed_child_frame_ids.is_empty() {
                tracing::debug!(
                    parent_frame_id = emacs_frame_id,
                    child_frame_ids = ?child_frame_ids,
                    removed_child_frame_ids = ?removed_child_frame_ids,
                    "child_frame_lifecycle: present_begin"
                );
            }
            output.present();
            super::frame_stats::note_present(std::time::Instant::now());
            if !child_frame_ids.is_empty() || !removed_child_frame_ids.is_empty() {
                tracing::debug!(
                    parent_frame_id = emacs_frame_id,
                    child_frame_ids = ?child_frame_ids,
                    removed_child_frame_ids = ?removed_child_frame_ids,
                    "child_frame_lifecycle: present_done"
                );
            }
        }
    }
}

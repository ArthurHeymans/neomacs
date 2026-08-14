//! Window transition state (crossfade and scroll animations).

use crate::core::frame_glyphs::{
    FrameGlyphBuffer, WindowEffectHint, WindowTransitionHint, WindowTransitionKind,
};
use crate::core::types::Rect;
use neomacs_display_protocol::{ScrollEasing, ScrollEffect, TransitionPolicy};
use neomacs_renderer_wgpu::{FrameSampleTime, WgpuRenderer};
use std::collections::HashMap;

/// State for an active crossfade transition
pub(super) struct CrossfadeTransition {
    pub(super) started: std::time::Instant,
    pub(super) duration: std::time::Duration,
    pub(super) bounds: Rect,
    pub(super) effect: ScrollEffect,
    pub(super) easing: ScrollEasing,
    // Snapshot handles retained for the transition's lifetime; sampling during the
    // crossfade goes through `old_bind_group`, so these are never read directly.
    #[allow(dead_code)]
    pub(super) old_texture: wgpu::Texture,
    #[allow(dead_code)]
    pub(super) old_view: wgpu::TextureView,
    pub(super) old_bind_group: wgpu::BindGroup,
}

/// State for an active scroll slide transition
pub(super) struct ScrollTransition {
    pub(super) started: std::time::Instant,
    pub(super) duration: std::time::Duration,
    pub(super) bounds: Rect,
    pub(super) direction: i32, // +1 = scroll down (content up), -1 = scroll up
    /// Pixel distance to slide (clamped to bounds.height).
    /// For a 1-line scroll this equals char_height, not the full window.
    pub(super) scroll_distance: f32,
    pub(super) effect: ScrollEffect,
    pub(super) easing: ScrollEasing,
    // Snapshot handles retained for the transition's lifetime; sampling during the
    // scroll slide goes through `old_bind_group`, so these are never read directly.
    #[allow(dead_code)]
    pub(super) old_texture: wgpu::Texture,
    #[allow(dead_code)]
    pub(super) old_view: wgpu::TextureView,
    pub(super) old_bind_group: wgpu::BindGroup,
}

/// Window transition state (crossfade and scroll animations).
///
/// Groups configuration, double-buffer textures, and active transition maps.
pub(crate) struct TransitionState {
    // Configuration
    pub(super) policy: TransitionPolicy,

    // Double-buffer offscreen textures
    pub(super) offscreen_a: Option<(wgpu::Texture, wgpu::TextureView, wgpu::BindGroup)>,
    pub(super) offscreen_b: Option<(wgpu::Texture, wgpu::TextureView, wgpu::BindGroup)>,
    pub(super) current_is_a: bool,

    // Active transitions
    pub(super) crossfades: HashMap<i64, CrossfadeTransition>,
    pub(super) scroll_slides: HashMap<i64, ScrollTransition>,
}

impl Default for TransitionState {
    fn default() -> Self {
        Self {
            policy: TransitionPolicy::default(),
            offscreen_a: None,
            offscreen_b: None,
            current_is_a: true,
            crossfades: HashMap::new(),
            scroll_slides: HashMap::new(),
        }
    }
}

impl TransitionState {
    pub(super) fn apply_policy(&mut self, policy: TransitionPolicy) {
        if !policy.crossfade.enabled {
            self.crossfades.clear();
        }
        if !policy.scroll.enabled {
            self.scroll_slides.clear();
        }
        self.policy = policy;
    }

    /// Check if any transitions are currently active
    pub(super) fn has_active(&self) -> bool {
        !self.crossfades.is_empty() || !self.scroll_slides.is_empty()
    }
}

fn current_offscreen_view_and_bg(
    transitions: &TransitionState,
) -> Option<(&wgpu::TextureView, &wgpu::BindGroup)> {
    let (_, view, bg) = if transitions.current_is_a {
        transitions.offscreen_a.as_ref()?
    } else {
        transitions.offscreen_b.as_ref()?
    };
    Some((view, bg))
}

fn previous_offscreen(
    transitions: &TransitionState,
) -> Option<(&wgpu::Texture, &wgpu::TextureView, &wgpu::BindGroup)> {
    let (tex, view, bg) = if transitions.current_is_a {
        transitions.offscreen_b.as_ref()?
    } else {
        transitions.offscreen_a.as_ref()?
    };
    Some((tex, view, bg))
}

fn snapshot_prev_texture(
    renderer: &WgpuRenderer,
    transitions: &TransitionState,
    width: u32,
    height: u32,
) -> Option<(wgpu::Texture, wgpu::TextureView, wgpu::BindGroup)> {
    let (prev_tex, _, _) = previous_offscreen(transitions)?;

    let (snap, snap_view) = renderer.create_offscreen_texture(width, height);

    let mut encoder = renderer
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Snapshot Copy Encoder"),
        });
    encoder.copy_texture_to_texture(
        wgpu::TexelCopyTextureInfo {
            texture: prev_tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyTextureInfo {
            texture: &snap,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    renderer.queue().submit(std::iter::once(encoder.finish()));

    let snap_bg = renderer.create_texture_bind_group(&snap_view);
    Some((snap, snap_view, snap_bg))
}

fn apply_transition_hint(
    renderer: &WgpuRenderer,
    transitions: &mut TransitionState,
    hint: &WindowTransitionHint,
    now: std::time::Instant,
    width: u32,
    height: u32,
) {
    match hint.kind {
        WindowTransitionKind::Crossfade => {
            if !transitions.policy.crossfade.enabled {
                return;
            }

            transitions.crossfades.remove(&hint.window_id.get());
            transitions.scroll_slides.remove(&hint.window_id.get());

            if let Some((tex, view, bg)) =
                snapshot_prev_texture(renderer, transitions, width, height)
            {
                let effect = hint.effect.unwrap_or(transitions.policy.crossfade.effect);
                let easing = hint.easing.unwrap_or(transitions.policy.crossfade.easing);
                tracing::debug!(
                    "Starting crossfade for window {} (effect={:?}, easing={:?})",
                    hint.window_id.get(),
                    effect,
                    easing
                );
                transitions.crossfades.insert(
                    hint.window_id.get(),
                    CrossfadeTransition {
                        started: now,
                        duration: transitions.policy.crossfade.duration,
                        bounds: hint.bounds,
                        effect,
                        easing,
                        old_texture: tex,
                        old_view: view,
                        old_bind_group: bg,
                    },
                );
            }
        }
        WindowTransitionKind::ScrollSlide {
            direction,
            scroll_distance,
        } => {
            if !transitions.policy.scroll.enabled {
                return;
            }
            if hint.bounds.height < 50.0 {
                return;
            }

            transitions.crossfades.remove(&hint.window_id.get());
            transitions.scroll_slides.remove(&hint.window_id.get());

            let dir = if direction >= 0 { 1 } else { -1 };
            let scroll_px = scroll_distance.max(0.0).min(hint.bounds.height);
            if let Some((tex, view, bg)) =
                snapshot_prev_texture(renderer, transitions, width, height)
            {
                let effect = hint.effect.unwrap_or(transitions.policy.scroll.effect);
                let easing = hint.easing.unwrap_or(transitions.policy.scroll.easing);
                tracing::debug!(
                    "Starting scroll slide for window {} (dir={}, effect={:?}, easing={:?}, scroll_px={})",
                    hint.window_id.get(),
                    dir,
                    effect,
                    easing,
                    scroll_px
                );
                transitions.scroll_slides.insert(
                    hint.window_id.get(),
                    ScrollTransition {
                        started: now,
                        duration: transitions.policy.scroll.duration,
                        bounds: hint.bounds,
                        direction: dir,
                        scroll_distance: scroll_px,
                        effect,
                        easing,
                        old_texture: tex,
                        old_view: view,
                        old_bind_group: bg,
                    },
                );
            }
        }
    }
}

fn apply_effect_hint(
    renderer: &mut WgpuRenderer,
    transitions: &mut TransitionState,
    effects: &neomacs_display_protocol::EffectsConfig,
    hint: &WindowEffectHint,
    now: std::time::Instant,
    frame_dirty: &mut bool,
    width: u32,
    height: u32,
) {
    match hint {
        WindowEffectHint::TextFadeIn { window_id, bounds } => {
            if effects.text_fade_in.enabled {
                renderer.trigger_text_fade_in(window_id.get(), *bounds, now);
            }
        }
        WindowEffectHint::ScrollLineSpacing {
            window_id,
            bounds,
            direction,
        } => {
            if effects.scroll_line_spacing.enabled {
                renderer.trigger_scroll_line_spacing(window_id.get(), *bounds, *direction, now);
            }
        }
        WindowEffectHint::ScrollMomentum {
            window_id,
            bounds,
            direction,
        } => {
            if effects.scroll_momentum.enabled {
                renderer.trigger_scroll_momentum(window_id.get(), *bounds, *direction, now);
            }
        }
        WindowEffectHint::ScrollVelocityFade {
            window_id,
            bounds,
            delta,
        } => {
            if effects.scroll_velocity_fade.enabled {
                renderer.trigger_scroll_velocity_fade(window_id.get(), *bounds, *delta, now);
            }
        }
        WindowEffectHint::LineAnimation {
            bounds,
            edit_y,
            offset,
            ..
        } => {
            if effects.line_animation.enabled {
                renderer.start_line_animation(
                    *bounds,
                    *edit_y,
                    *offset,
                    effects.line_animation.duration_ms,
                );
            }
        }
        WindowEffectHint::WindowSwitchFade { window_id, bounds } => {
            if effects.window_switch_fade.enabled {
                renderer.start_window_fade(window_id.get(), *bounds);
                *frame_dirty = true;
            }
        }
        WindowEffectHint::ThemeTransition { bounds } => {
            if !effects.theme_transition.enabled {
                return;
            }
            if transitions.crossfades.contains_key(&-1) {
                return;
            }
            if let Some((tex, view, bg_group)) =
                snapshot_prev_texture(renderer, transitions, width, height)
            {
                tracing::debug!("Starting theme transition crossfade (effect hint)");
                transitions.crossfades.insert(
                    -1,
                    CrossfadeTransition {
                        started: now,
                        duration: effects.theme_transition.duration,
                        bounds: *bounds,
                        effect: transitions.policy.crossfade.effect,
                        easing: transitions.policy.crossfade.easing,
                        old_texture: tex,
                        old_view: view,
                        old_bind_group: bg_group,
                    },
                );
            }
        }
    }
}

pub(super) fn detect_frame_transitions(
    renderer: &mut WgpuRenderer,
    transitions: &mut TransitionState,
    effects: &neomacs_display_protocol::EffectsConfig,
    frame: &mut FrameGlyphBuffer,
    frame_dirty: &mut bool,
    width: u32,
    height: u32,
) {
    let (transition_hints, effect_hints) = frame.take_runtime_hints();
    let now = std::time::Instant::now();

    for hint in &transition_hints {
        apply_transition_hint(renderer, transitions, hint, now, width, height);
    }
    for hint in &effect_hints {
        apply_effect_hint(
            renderer,
            transitions,
            effects,
            hint,
            now,
            frame_dirty,
            width,
            height,
        );
    }
}

pub(super) fn ensure_frame_offscreen_textures(
    renderer: &WgpuRenderer,
    transitions: &mut TransitionState,
    width: u32,
    height: u32,
) {
    if transitions.offscreen_a.is_some() && transitions.offscreen_b.is_some() {
        return;
    }
    if transitions.offscreen_a.is_none() {
        let (tex, view) = renderer.create_offscreen_texture(width, height);
        let bg = renderer.create_texture_bind_group(&view);
        transitions.offscreen_a = Some((tex, view, bg));
    }
    if transitions.offscreen_b.is_none() {
        let (tex, view) = renderer.create_offscreen_texture(width, height);
        let bg = renderer.create_texture_bind_group(&view);
        transitions.offscreen_b = Some((tex, view, bg));
    }
}

pub(super) fn clear_frame_transition_textures(transitions: &mut TransitionState) {
    transitions.offscreen_a = None;
    transitions.offscreen_b = None;
    transitions.crossfades.clear();
    transitions.scroll_slides.clear();
}

pub(super) fn render_frame_transitions(
    renderer: &mut WgpuRenderer,
    transitions: &mut TransitionState,
    surface_view: &wgpu::TextureView,
    width: u32,
    height: u32,
    sample_time: FrameSampleTime,
) {
    let now = sample_time.as_instant();
    let current_bg = match current_offscreen_view_and_bg(transitions) {
        Some((_, bg)) => bg.clone(),
        None => return,
    };

    let mut completed_crossfades = Vec::new();
    for (&wid, transition) in &transitions.crossfades {
        let elapsed = now.duration_since(transition.started);
        let raw_t = (elapsed.as_secs_f32() / transition.duration.as_secs_f32()).min(1.0);
        let elapsed_secs = elapsed.as_secs_f32();

        renderer.render_scroll_effect(
            surface_view,
            &transition.old_bind_group,
            &current_bg,
            raw_t,
            elapsed_secs,
            1,
            &transition.bounds,
            transition.bounds.height,
            transition.effect,
            transition.easing,
            width,
            height,
        );

        if raw_t >= 1.0 {
            completed_crossfades.push(wid);
        }
    }
    for wid in completed_crossfades {
        transitions.crossfades.remove(&wid);
    }

    let mut completed_scrolls = Vec::new();
    for (&wid, transition) in &transitions.scroll_slides {
        let elapsed = now.duration_since(transition.started);
        let raw_t = (elapsed.as_secs_f32() / transition.duration.as_secs_f32()).min(1.0);
        let elapsed_secs = elapsed.as_secs_f32();

        renderer.render_scroll_effect(
            surface_view,
            &transition.old_bind_group,
            &current_bg,
            raw_t,
            elapsed_secs,
            transition.direction,
            &transition.bounds,
            transition.scroll_distance,
            transition.effect,
            transition.easing,
            width,
            height,
        );

        if raw_t >= 1.0 {
            completed_scrolls.push(wid);
        }
    }
    for wid in completed_scrolls {
        transitions.scroll_slides.remove(&wid);
    }
}

// ==========================================================================
// Tests
// ==========================================================================

#[cfg(test)]
#[path = "transitions_test.rs"]
mod tests;

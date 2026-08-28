//! Renderer-owned state for snapshot-based window transitions.

use crate::core::frame_glyphs::{
    FrameGlyphBuffer, WindowEffectHint, WindowTransitionHint, WindowTransitionKind,
};
use neomacs_display_protocol::{
    DirectionlessTransitionEffect, ResolvedTransitionEffect, TransitionPlan, TransitionPolicy,
};
use neomacs_renderer_wgpu::WgpuRenderer;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransitionSource {
    Buffer,
    Scroll,
    Theme,
}

/// Renderer-owned state for any snapshot transition.
pub(super) struct ActiveTransition {
    source: TransitionSource,
    pub(super) started: std::time::Instant,
    pub(super) plan: TransitionPlan,
    // Snapshot handles retained for the transition's lifetime; sampling goes
    // through `old_bind_group`, so these are never read directly.
    #[allow(dead_code)]
    pub(super) old_texture: wgpu::Texture,
    #[allow(dead_code)]
    pub(super) old_view: wgpu::TextureView,
    pub(super) old_bind_group: wgpu::BindGroup,
}

/// Window transition state.
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
    pub(super) active: HashMap<i64, ActiveTransition>,
}

impl Default for TransitionState {
    fn default() -> Self {
        Self {
            policy: TransitionPolicy::default(),
            offscreen_a: None,
            offscreen_b: None,
            current_is_a: true,
            active: HashMap::new(),
        }
    }
}

impl TransitionState {
    pub(super) fn apply_policy(&mut self, policy: TransitionPolicy) {
        self.active.retain(|_, transition| match transition.source {
            TransitionSource::Buffer => policy.buffer.enabled,
            TransitionSource::Scroll => policy.scroll.enabled,
            TransitionSource::Theme => true,
        });
        self.policy = policy;
    }

    /// Check if any transitions are currently active
    pub(super) fn has_active(&self) -> bool {
        !self.active.is_empty()
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
    let (source, plan) = match hint.kind {
        WindowTransitionKind::ContentReplaced { intent } => (
            TransitionSource::Buffer,
            transitions.policy.buffer_plan(hint.bounds, intent),
        ),
        WindowTransitionKind::ViewportScrolled {
            direction,
            scroll_distance,
        } => {
            if hint.bounds.height < 50.0 {
                return;
            }
            (
                TransitionSource::Scroll,
                transitions
                    .policy
                    .scroll_plan(hint.bounds, direction, scroll_distance),
            )
        }
    };

    let window_id = hint.window_id.get();
    let Some(plan) = plan else {
        return;
    };
    transitions.active.remove(&window_id);
    start_transition(
        renderer,
        transitions,
        window_id,
        source,
        plan,
        now,
        width,
        height,
    );
}

#[allow(clippy::too_many_arguments)]
fn start_transition(
    renderer: &WgpuRenderer,
    transitions: &mut TransitionState,
    transition_id: i64,
    source: TransitionSource,
    plan: TransitionPlan,
    now: std::time::Instant,
    width: u32,
    height: u32,
) {
    let Some((tex, view, bg)) = snapshot_prev_texture(renderer, transitions, width, height) else {
        return;
    };
    tracing::debug!(
        ?source,
        ?plan.effect,
        ?plan.easing,
        transition_id,
        "starting window transition"
    );
    transitions.active.insert(
        transition_id,
        ActiveTransition {
            source,
            started: now,
            plan,
            old_texture: tex,
            old_view: view,
            old_bind_group: bg,
        },
    );
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
            if transitions.active.contains_key(&-1) {
                return;
            }
            let plan = TransitionPlan {
                duration: effects.theme_transition.duration,
                easing: effects.theme_transition.easing,
                bounds: *bounds,
                effect: ResolvedTransitionEffect::Directionless(
                    DirectionlessTransitionEffect::Crossfade,
                ),
            };
            start_transition(
                renderer,
                transitions,
                -1,
                TransitionSource::Theme,
                plan,
                now,
                width,
                height,
            );
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
    transitions.active.clear();
}

pub(super) fn render_frame_transitions(
    renderer: &mut WgpuRenderer,
    transitions: &mut TransitionState,
    surface_view: &wgpu::TextureView,
    width: u32,
    height: u32,
) {
    let now = std::time::Instant::now();
    let current_bg = match current_offscreen_view_and_bg(transitions) {
        Some((_, bg)) => bg.clone(),
        None => return,
    };

    let mut completed = Vec::new();
    for (&transition_id, transition) in &transitions.active {
        let elapsed = now.duration_since(transition.started);
        let raw_t = (elapsed.as_secs_f32() / transition.plan.duration.as_secs_f32()).min(1.0);
        let elapsed_secs = elapsed.as_secs_f32();

        renderer.render_transition_effect(
            surface_view,
            &transition.old_bind_group,
            &current_bg,
            raw_t,
            elapsed_secs,
            &transition.plan.bounds,
            transition.plan.effect,
            transition.plan.easing,
            width,
            height,
        );

        if raw_t >= 1.0 {
            completed.push(transition_id);
        }
    }
    for transition_id in completed {
        transitions.active.remove(&transition_id);
    }
}

// ==========================================================================
// Tests
// ==========================================================================

#[cfg(test)]
#[path = "transitions_test.rs"]
mod tests;

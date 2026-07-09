//! Cursor effect commands shared by the frontend host and render runtime.

use crate::effect_config::EffectsConfig;

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum CursorEffectArg {
    Nil,
    Bool(bool),
    Number(f64),
    String(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CursorEffectCommand {
    pub name: String,
    pub args: Vec<CursorEffectArg>,
}

impl CursorEffectCommand {
    pub fn new(name: impl Into<String>, args: Vec<CursorEffectArg>) -> Self {
        Self {
            name: name.into(),
            args,
        }
    }

    pub fn apply_to(&self, effects: &mut EffectsConfig) {
        apply_cursor_effect_config(effects, &self.name, &self.args);
    }
}

fn enabled(args: &[CursorEffectArg]) -> bool {
    !matches!(
        args.first(),
        None | Some(CursorEffectArg::Nil) | Some(CursorEffectArg::Bool(false))
    )
}

fn number(args: &[CursorEffectArg], index: usize, default: f32) -> f32 {
    match args.get(index) {
        Some(CursorEffectArg::Number(value)) => *value as f32,
        _ => default,
    }
}

fn u32_arg(args: &[CursorEffectArg], index: usize, default: u32) -> u32 {
    number(args, index, default as f32).round().max(0.0) as u32
}

fn usize_arg(args: &[CursorEffectArg], index: usize, default: usize) -> usize {
    number(args, index, default as f32).round().max(0.0) as usize
}

fn percent(args: &[CursorEffectArg], index: usize, default: f32) -> f32 {
    match args.get(index) {
        Some(CursorEffectArg::Number(value)) => (*value as f32 / 100.0).clamp(0.0, 1.0),
        _ => default,
    }
}

fn speed_percent(args: &[CursorEffectArg], index: usize, default: f32) -> f32 {
    match args.get(index) {
        Some(CursorEffectArg::Number(value)) => (*value as f32 / 100.0).max(0.001),
        _ => default,
    }
}

fn color(args: &[CursorEffectArg], index: usize, default: (f32, f32, f32)) -> (f32, f32, f32) {
    let Some(CursorEffectArg::String(text)) = args.get(index) else {
        return default;
    };
    let text = text.strip_prefix('#').unwrap_or(text);
    if text.len() != 6 {
        return default;
    }
    let Ok(r) = u8::from_str_radix(&text[0..2], 16) else {
        return default;
    };
    let Ok(g) = u8::from_str_radix(&text[2..4], 16) else {
        return default;
    };
    let Ok(b) = u8::from_str_radix(&text[4..6], 16) else {
        return default;
    };
    (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

fn apply_cursor_effect_config(effects: &mut EffectsConfig, name: &str, args: &[CursorEffectArg]) {
    let enabled = enabled(args);
    match name {
        "glow" => {
            effects.cursor_glow.enabled = enabled;
            effects.cursor_glow.color = color(args, 1, effects.cursor_glow.color);
            effects.cursor_glow.radius = number(args, 2, effects.cursor_glow.radius);
        }
        "pulse" => {
            effects.cursor_pulse.enabled = enabled;
            effects.cursor_pulse.speed = speed_percent(args, 1, effects.cursor_pulse.speed);
        }
        "color-cycle" => {
            effects.cursor_color_cycle.enabled = enabled;
            effects.cursor_color_cycle.speed =
                speed_percent(args, 1, effects.cursor_color_cycle.speed);
            effects.cursor_color_cycle.saturation =
                percent(args, 2, effects.cursor_color_cycle.saturation);
            effects.cursor_color_cycle.lightness =
                percent(args, 3, effects.cursor_color_cycle.lightness);
        }
        "shadow" => {
            effects.cursor_shadow.enabled = enabled;
            effects.cursor_shadow.offset_x = number(args, 1, effects.cursor_shadow.offset_x);
            effects.cursor_shadow.offset_y = number(args, 2, effects.cursor_shadow.offset_y);
            effects.cursor_shadow.opacity = percent(args, 3, effects.cursor_shadow.opacity);
        }
        "wake" => {
            effects.cursor_wake.enabled = enabled;
            effects.cursor_wake.duration_ms = u32_arg(args, 1, effects.cursor_wake.duration_ms);
            effects.cursor_wake.scale = speed_percent(args, 2, effects.cursor_wake.scale);
        }
        "error-pulse" => {
            effects.cursor_error_pulse.enabled = enabled;
            effects.cursor_error_pulse.color = color(args, 1, effects.cursor_error_pulse.color);
            effects.cursor_error_pulse.duration_ms =
                u32_arg(args, 2, effects.cursor_error_pulse.duration_ms);
        }
        "crosshair" => {
            effects.cursor_crosshair.enabled = enabled;
            effects.cursor_crosshair.color = color(args, 1, effects.cursor_crosshair.color);
            effects.cursor_crosshair.opacity = percent(args, 2, effects.cursor_crosshair.opacity);
        }
        "magnetism" => {
            effects.cursor_magnetism.enabled = enabled;
            effects.cursor_magnetism.color = color(args, 1, effects.cursor_magnetism.color);
            effects.cursor_magnetism.ring_count =
                u32_arg(args, 2, effects.cursor_magnetism.ring_count);
            effects.cursor_magnetism.duration_ms =
                u32_arg(args, 3, effects.cursor_magnetism.duration_ms);
            effects.cursor_magnetism.opacity = percent(args, 4, effects.cursor_magnetism.opacity);
        }
        "comet" => {
            effects.cursor_comet.enabled = enabled;
            effects.cursor_comet.trail_length = u32_arg(args, 1, effects.cursor_comet.trail_length);
            effects.cursor_comet.fade_ms = u32_arg(args, 2, effects.cursor_comet.fade_ms);
            effects.cursor_comet.color = color(args, 3, effects.cursor_comet.color);
            effects.cursor_comet.opacity = percent(args, 4, effects.cursor_comet.opacity);
        }
        "spotlight" => {
            effects.cursor_spotlight.enabled = enabled;
            effects.cursor_spotlight.radius = number(args, 1, effects.cursor_spotlight.radius);
            effects.cursor_spotlight.intensity =
                percent(args, 2, effects.cursor_spotlight.intensity);
            effects.cursor_spotlight.color = color(args, 3, effects.cursor_spotlight.color);
        }
        "particles" => {
            effects.cursor_particles.enabled = enabled;
            effects.cursor_particles.color = color(args, 1, effects.cursor_particles.color);
            effects.cursor_particles.count = u32_arg(args, 2, effects.cursor_particles.count);
            effects.cursor_particles.lifetime_ms =
                u32_arg(args, 3, effects.cursor_particles.lifetime_ms);
            effects.cursor_particles.gravity = number(args, 4, effects.cursor_particles.gravity);
        }
        "trail-fade" => {
            effects.cursor_trail_fade.enabled = enabled;
            effects.cursor_trail_fade.length = usize_arg(args, 1, effects.cursor_trail_fade.length);
            effects.cursor_trail_fade.ms = u32_arg(args, 2, effects.cursor_trail_fade.ms);
        }
        "elastic-snap" => {
            effects.cursor_elastic_snap.enabled = enabled;
            effects.cursor_elastic_snap.overshoot =
                percent(args, 1, effects.cursor_elastic_snap.overshoot);
            effects.cursor_elastic_snap.duration_ms =
                u32_arg(args, 2, effects.cursor_elastic_snap.duration_ms);
        }
        "ghost" => {
            effects.cursor_ghost.enabled = enabled;
            effects.cursor_ghost.color = color(args, 1, effects.cursor_ghost.color);
            effects.cursor_ghost.fade_ms = u32_arg(args, 2, effects.cursor_ghost.fade_ms);
            effects.cursor_ghost.opacity = percent(args, 3, effects.cursor_ghost.opacity);
        }
        "ripple-wave" => {
            effects.cursor_ripple_wave.enabled = enabled;
            effects.cursor_ripple_wave.color = color(args, 1, effects.cursor_ripple_wave.color);
            effects.cursor_ripple_wave.max_radius =
                number(args, 2, effects.cursor_ripple_wave.max_radius);
            effects.cursor_ripple_wave.duration_ms =
                u32_arg(args, 3, effects.cursor_ripple_wave.duration_ms);
            effects.cursor_ripple_wave.opacity =
                percent(args, 4, effects.cursor_ripple_wave.opacity);
        }
        "lighthouse" => effects.cursor_lighthouse.enabled = enabled,
        "sonar-ping" => effects.cursor_sonar_ping.enabled = enabled,
        "orbit-particles" => effects.cursor_orbit_particles.enabled = enabled,
        "heartbeat" => effects.cursor_heartbeat.enabled = enabled,
        "metronome" => effects.cursor_metronome.enabled = enabled,
        "radar" => effects.cursor_radar.enabled = enabled,
        "ripple-ring" => effects.cursor_ripple_ring.enabled = enabled,
        "scope" => effects.cursor_scope.enabled = enabled,
        "shockwave" => effects.cursor_shockwave.enabled = enabled,
        "gravity-well" => effects.cursor_gravity_well.enabled = enabled,
        "water-drop" => effects.cursor_water_drop.enabled = enabled,
        "pixel-dust" => effects.cursor_pixel_dust.enabled = enabled,
        "candle-flame" => effects.cursor_candle_flame.enabled = enabled,
        "moth-flame" => effects.cursor_moth_flame.enabled = enabled,
        "sparkler" => effects.cursor_sparkler.enabled = enabled,
        "plasma-ball" => effects.cursor_plasma_ball.enabled = enabled,
        "quill-pen" => effects.cursor_quill_pen.enabled = enabled,
        "aurora-borealis" => effects.cursor_aurora_borealis.enabled = enabled,
        "feather" => effects.cursor_feather.enabled = enabled,
        "stardust" => effects.cursor_stardust.enabled = enabled,
        "compass-needle" => effects.cursor_compass_needle.enabled = enabled,
        "galaxy" => effects.cursor_galaxy.enabled = enabled,
        "prism" => effects.cursor_prism.enabled = enabled,
        "moth" => effects.cursor_moth.enabled = enabled,
        "flame" => effects.cursor_flame.enabled = enabled,
        "crystal" => effects.cursor_crystal.enabled = enabled,
        "lightning" => effects.cursor_lightning.enabled = enabled,
        "snowflake" => effects.cursor_snowflake.enabled = enabled,
        "firework" => effects.cursor_firework.enabled = enabled,
        "tornado" => effects.cursor_tornado.enabled = enabled,
        "portal" => effects.cursor_portal.enabled = enabled,
        "bubble" => effects.cursor_bubble.enabled = enabled,
        "sparkle-burst" => effects.cursor_sparkle_burst.enabled = enabled,
        "compass" => effects.cursor_compass.enabled = enabled,
        "dna-helix" => effects.cursor_dna_helix.enabled = enabled,
        "pendulum" => effects.cursor_pendulum.enabled = enabled,
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glow_command_updates_color_radius_and_enabled_state() {
        let mut effects = EffectsConfig::default();

        CursorEffectCommand::new(
            "glow",
            vec![
                CursorEffectArg::Bool(true),
                CursorEffectArg::String("#66CCFF".to_owned()),
                CursorEffectArg::Number(48.0),
            ],
        )
        .apply_to(&mut effects);

        assert!(effects.cursor_glow.enabled);
        assert_eq!(effects.cursor_glow.color, (0.4, 0.8, 1.0));
        assert_eq!(effects.cursor_glow.radius, 48.0);
    }

    #[test]
    fn nil_first_arg_disables_effect_without_resetting_config() {
        let mut effects = EffectsConfig::default();
        effects.cursor_comet.enabled = true;
        effects.cursor_comet.trail_length = 12;

        CursorEffectCommand::new("comet", vec![CursorEffectArg::Nil]).apply_to(&mut effects);

        assert!(!effects.cursor_comet.enabled);
        assert_eq!(effects.cursor_comet.trail_length, 12);
    }

    #[test]
    fn color_cycle_maps_lisp_percent_values_to_unit_interval() {
        let mut effects = EffectsConfig::default();

        CursorEffectCommand::new(
            "color-cycle",
            vec![
                CursorEffectArg::Bool(true),
                CursorEffectArg::Number(90.0),
                CursorEffectArg::Number(90.0),
                CursorEffectArg::Number(60.0),
            ],
        )
        .apply_to(&mut effects);

        assert!(effects.cursor_color_cycle.enabled);
        assert_eq!(effects.cursor_color_cycle.speed, 0.9);
        assert_eq!(effects.cursor_color_cycle.saturation, 0.9);
        assert_eq!(effects.cursor_color_cycle.lightness, 0.6);
    }
}

//! Cursor effect commands shared by the frontend host and render runtime.

use crate::effect_config::EffectsConfig;
use std::fmt;

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
    pub kind: CursorEffectKind,
    pub args: Vec<CursorEffectArg>,
}

macro_rules! cursor_effect_kinds {
    ($($variant:ident => $name:literal),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum CursorEffectKind {
            $($variant),+
        }

        impl CursorEffectKind {
            pub fn from_name(name: &str) -> Option<Self> {
                match name {
                    $($name => Some(Self::$variant),)+
                    _ => None,
                }
            }

            pub fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $name,)+
                }
            }
        }
    };
}

cursor_effect_kinds! {
    Glow => "glow",
    Pulse => "pulse",
    ColorCycle => "color-cycle",
    Shadow => "shadow",
    Wake => "wake",
    ErrorPulse => "error-pulse",
    Crosshair => "crosshair",
    Magnetism => "magnetism",
    Comet => "comet",
    Spotlight => "spotlight",
    Particles => "particles",
    TrailFade => "trail-fade",
    ElasticSnap => "elastic-snap",
    Ghost => "ghost",
    RippleWave => "ripple-wave",
    Lighthouse => "lighthouse",
    SonarPing => "sonar-ping",
    OrbitParticles => "orbit-particles",
    Heartbeat => "heartbeat",
    Metronome => "metronome",
    Radar => "radar",
    RippleRing => "ripple-ring",
    Scope => "scope",
    Shockwave => "shockwave",
    GravityWell => "gravity-well",
    WaterDrop => "water-drop",
    PixelDust => "pixel-dust",
    CandleFlame => "candle-flame",
    MothFlame => "moth-flame",
    Sparkler => "sparkler",
    PlasmaBall => "plasma-ball",
    QuillPen => "quill-pen",
    AuroraBorealis => "aurora-borealis",
    Feather => "feather",
    Stardust => "stardust",
    CompassNeedle => "compass-needle",
    Galaxy => "galaxy",
    Prism => "prism",
    Moth => "moth",
    Flame => "flame",
    Crystal => "crystal",
    Lightning => "lightning",
    Snowflake => "snowflake",
    Firework => "firework",
    Tornado => "tornado",
    Portal => "portal",
    Bubble => "bubble",
    SparkleBurst => "sparkle-burst",
    Compass => "compass",
    DnaHelix => "dna-helix",
    Pendulum => "pendulum",
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownCursorEffectName {
    name: String,
}

impl UnknownCursorEffectName {
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for UnknownCursorEffectName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown cursor effect {:?}", self.name)
    }
}

impl std::error::Error for UnknownCursorEffectName {}

impl CursorEffectCommand {
    pub fn try_new(
        name: impl Into<String>,
        args: Vec<CursorEffectArg>,
    ) -> Result<Self, UnknownCursorEffectName> {
        let name = name.into();
        let Some(kind) = CursorEffectKind::from_name(&name) else {
            return Err(UnknownCursorEffectName { name });
        };
        Ok(Self { kind, args })
    }

    pub fn apply_to(&self, effects: &mut EffectsConfig) {
        apply_cursor_effect_config(effects, self.kind, &self.args);
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

fn apply_cursor_effect_config(
    effects: &mut EffectsConfig,
    kind: CursorEffectKind,
    args: &[CursorEffectArg],
) {
    use CursorEffectKind::*;

    let enabled = enabled(args);
    match kind {
        Glow => {
            effects.cursor_glow.enabled = enabled;
            effects.cursor_glow.color = color(args, 1, effects.cursor_glow.color);
            effects.cursor_glow.radius = number(args, 2, effects.cursor_glow.radius);
        }
        Pulse => {
            effects.cursor_pulse.enabled = enabled;
            effects.cursor_pulse.speed = speed_percent(args, 1, effects.cursor_pulse.speed);
        }
        ColorCycle => {
            effects.cursor_color_cycle.enabled = enabled;
            effects.cursor_color_cycle.speed =
                speed_percent(args, 1, effects.cursor_color_cycle.speed);
            effects.cursor_color_cycle.saturation =
                percent(args, 2, effects.cursor_color_cycle.saturation);
            effects.cursor_color_cycle.lightness =
                percent(args, 3, effects.cursor_color_cycle.lightness);
        }
        Shadow => {
            effects.cursor_shadow.enabled = enabled;
            effects.cursor_shadow.offset_x = number(args, 1, effects.cursor_shadow.offset_x);
            effects.cursor_shadow.offset_y = number(args, 2, effects.cursor_shadow.offset_y);
            effects.cursor_shadow.opacity = percent(args, 3, effects.cursor_shadow.opacity);
        }
        Wake => {
            effects.cursor_wake.enabled = enabled;
            effects.cursor_wake.duration_ms = u32_arg(args, 1, effects.cursor_wake.duration_ms);
            effects.cursor_wake.scale = speed_percent(args, 2, effects.cursor_wake.scale);
        }
        ErrorPulse => {
            effects.cursor_error_pulse.enabled = enabled;
            effects.cursor_error_pulse.color = color(args, 1, effects.cursor_error_pulse.color);
            effects.cursor_error_pulse.duration_ms =
                u32_arg(args, 2, effects.cursor_error_pulse.duration_ms);
        }
        Crosshair => {
            effects.cursor_crosshair.enabled = enabled;
            effects.cursor_crosshair.color = color(args, 1, effects.cursor_crosshair.color);
            effects.cursor_crosshair.opacity = percent(args, 2, effects.cursor_crosshair.opacity);
        }
        Magnetism => {
            effects.cursor_magnetism.enabled = enabled;
            effects.cursor_magnetism.color = color(args, 1, effects.cursor_magnetism.color);
            effects.cursor_magnetism.ring_count =
                u32_arg(args, 2, effects.cursor_magnetism.ring_count);
            effects.cursor_magnetism.duration_ms =
                u32_arg(args, 3, effects.cursor_magnetism.duration_ms);
            effects.cursor_magnetism.opacity = percent(args, 4, effects.cursor_magnetism.opacity);
        }
        Comet => {
            effects.cursor_comet.enabled = enabled;
            effects.cursor_comet.trail_length = u32_arg(args, 1, effects.cursor_comet.trail_length);
            effects.cursor_comet.fade_ms = u32_arg(args, 2, effects.cursor_comet.fade_ms);
            effects.cursor_comet.color = color(args, 3, effects.cursor_comet.color);
            effects.cursor_comet.opacity = percent(args, 4, effects.cursor_comet.opacity);
        }
        Spotlight => {
            effects.cursor_spotlight.enabled = enabled;
            effects.cursor_spotlight.radius = number(args, 1, effects.cursor_spotlight.radius);
            effects.cursor_spotlight.intensity =
                percent(args, 2, effects.cursor_spotlight.intensity);
            effects.cursor_spotlight.color = color(args, 3, effects.cursor_spotlight.color);
        }
        Particles => {
            effects.cursor_particles.enabled = enabled;
            effects.cursor_particles.color = color(args, 1, effects.cursor_particles.color);
            effects.cursor_particles.count = u32_arg(args, 2, effects.cursor_particles.count);
            effects.cursor_particles.lifetime_ms =
                u32_arg(args, 3, effects.cursor_particles.lifetime_ms);
            effects.cursor_particles.gravity = number(args, 4, effects.cursor_particles.gravity);
        }
        TrailFade => {
            effects.cursor_trail_fade.enabled = enabled;
            effects.cursor_trail_fade.length = usize_arg(args, 1, effects.cursor_trail_fade.length);
            effects.cursor_trail_fade.ms = u32_arg(args, 2, effects.cursor_trail_fade.ms);
        }
        ElasticSnap => {
            effects.cursor_elastic_snap.enabled = enabled;
            effects.cursor_elastic_snap.overshoot =
                percent(args, 1, effects.cursor_elastic_snap.overshoot);
            effects.cursor_elastic_snap.duration_ms =
                u32_arg(args, 2, effects.cursor_elastic_snap.duration_ms);
        }
        Ghost => {
            effects.cursor_ghost.enabled = enabled;
            effects.cursor_ghost.color = color(args, 1, effects.cursor_ghost.color);
            effects.cursor_ghost.fade_ms = u32_arg(args, 2, effects.cursor_ghost.fade_ms);
            effects.cursor_ghost.opacity = percent(args, 3, effects.cursor_ghost.opacity);
        }
        RippleWave => {
            effects.cursor_ripple_wave.enabled = enabled;
            effects.cursor_ripple_wave.color = color(args, 1, effects.cursor_ripple_wave.color);
            effects.cursor_ripple_wave.max_radius =
                number(args, 2, effects.cursor_ripple_wave.max_radius);
            effects.cursor_ripple_wave.duration_ms =
                u32_arg(args, 3, effects.cursor_ripple_wave.duration_ms);
            effects.cursor_ripple_wave.opacity =
                percent(args, 4, effects.cursor_ripple_wave.opacity);
        }
        Lighthouse => effects.cursor_lighthouse.enabled = enabled,
        SonarPing => effects.cursor_sonar_ping.enabled = enabled,
        OrbitParticles => effects.cursor_orbit_particles.enabled = enabled,
        Heartbeat => effects.cursor_heartbeat.enabled = enabled,
        Metronome => effects.cursor_metronome.enabled = enabled,
        Radar => effects.cursor_radar.enabled = enabled,
        RippleRing => effects.cursor_ripple_ring.enabled = enabled,
        Scope => effects.cursor_scope.enabled = enabled,
        Shockwave => effects.cursor_shockwave.enabled = enabled,
        GravityWell => effects.cursor_gravity_well.enabled = enabled,
        WaterDrop => effects.cursor_water_drop.enabled = enabled,
        PixelDust => effects.cursor_pixel_dust.enabled = enabled,
        CandleFlame => effects.cursor_candle_flame.enabled = enabled,
        MothFlame => effects.cursor_moth_flame.enabled = enabled,
        Sparkler => effects.cursor_sparkler.enabled = enabled,
        PlasmaBall => effects.cursor_plasma_ball.enabled = enabled,
        QuillPen => effects.cursor_quill_pen.enabled = enabled,
        AuroraBorealis => effects.cursor_aurora_borealis.enabled = enabled,
        Feather => effects.cursor_feather.enabled = enabled,
        Stardust => effects.cursor_stardust.enabled = enabled,
        CompassNeedle => effects.cursor_compass_needle.enabled = enabled,
        Galaxy => effects.cursor_galaxy.enabled = enabled,
        Prism => effects.cursor_prism.enabled = enabled,
        Moth => effects.cursor_moth.enabled = enabled,
        Flame => effects.cursor_flame.enabled = enabled,
        Crystal => effects.cursor_crystal.enabled = enabled,
        Lightning => effects.cursor_lightning.enabled = enabled,
        Snowflake => effects.cursor_snowflake.enabled = enabled,
        Firework => effects.cursor_firework.enabled = enabled,
        Tornado => effects.cursor_tornado.enabled = enabled,
        Portal => effects.cursor_portal.enabled = enabled,
        Bubble => effects.cursor_bubble.enabled = enabled,
        SparkleBurst => effects.cursor_sparkle_burst.enabled = enabled,
        Compass => effects.cursor_compass.enabled = enabled,
        DnaHelix => effects.cursor_dna_helix.enabled = enabled,
        Pendulum => effects.cursor_pendulum.enabled = enabled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glow_command_updates_color_radius_and_enabled_state() {
        let mut effects = EffectsConfig::default();

        CursorEffectCommand::try_new(
            "glow",
            vec![
                CursorEffectArg::Bool(true),
                CursorEffectArg::String("#66CCFF".to_owned()),
                CursorEffectArg::Number(48.0),
            ],
        )
        .unwrap()
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

        CursorEffectCommand::try_new("comet", vec![CursorEffectArg::Nil])
            .unwrap()
            .apply_to(&mut effects);

        assert!(!effects.cursor_comet.enabled);
        assert_eq!(effects.cursor_comet.trail_length, 12);
    }

    #[test]
    fn color_cycle_maps_lisp_percent_values_to_unit_interval() {
        let mut effects = EffectsConfig::default();

        CursorEffectCommand::try_new(
            "color-cycle",
            vec![
                CursorEffectArg::Bool(true),
                CursorEffectArg::Number(90.0),
                CursorEffectArg::Number(90.0),
                CursorEffectArg::Number(60.0),
            ],
        )
        .unwrap()
        .apply_to(&mut effects);

        assert!(effects.cursor_color_cycle.enabled);
        assert_eq!(effects.cursor_color_cycle.speed, 0.9);
        assert_eq!(effects.cursor_color_cycle.saturation, 0.9);
        assert_eq!(effects.cursor_color_cycle.lightness, 0.6);
    }

    #[test]
    fn unknown_effect_name_is_rejected() {
        let error =
            CursorEffectCommand::try_new("glwo", vec![CursorEffectArg::Bool(true)]).unwrap_err();

        assert_eq!(error.name(), "glwo");
        assert_eq!(error.to_string(), "unknown cursor effect \"glwo\"");
    }

    #[test]
    fn effect_kind_round_trips_its_protocol_name() {
        assert_eq!(
            CursorEffectKind::from_name("ripple-wave"),
            Some(CursorEffectKind::RippleWave)
        );
        assert_eq!(CursorEffectKind::RippleWave.name(), "ripple-wave");
    }
}

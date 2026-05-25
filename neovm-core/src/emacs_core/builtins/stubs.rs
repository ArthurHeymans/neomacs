use super::*;
use crate::buffer::BufferManager;
use crate::emacs_core::display;
use crate::emacs_core::fontset;
use crate::emacs_core::value::{ValueKind, VecLikeType};
use crate::window::{FrameManager, WindowId};
#[cfg(not(target_os = "linux"))]
use arboard::Clipboard;
#[cfg(target_os = "linux")]
use arboard::{Clipboard, GetExtLinux, LinuxClipboardKind, SetExtLinux};
use notify::Watcher;

// =========================================================================
// fontset.c gap-fill stubs
// =========================================================================

pub(crate) fn builtin_fontset_list_all(args: Vec<Value>) -> EvalResult {
    expect_args("fontset-list-all", &args, 0)?;
    Ok(super::symbols::fontset_list_value())
}

// =========================================================================
// inotify.c / file-notification — cross-platform via `notify` crate
// =========================================================================

pub(crate) fn builtin_inotify_watch_list(args: Vec<Value>) -> EvalResult {
    expect_args("inotify-watch-list", &args, 0)?;
    FILE_NOTIFY_STATE.with(|slot| {
        let state = slot.borrow();
        let list: Vec<Value> = state
            .watches
            .iter()
            .map(|w| Value::cons(Value::fixnum(w.id), Value::string(&w.path)))
            .collect();
        Ok(Value::list(list))
    })
}

pub(crate) fn builtin_inotify_allocated_p(args: Vec<Value>) -> EvalResult {
    expect_args("inotify-allocated-p", &args, 0)?;
    FILE_NOTIFY_STATE.with(|slot| {
        let state = slot.borrow();
        Ok(Value::bool_val(state.watcher.is_some()))
    })
}

// =========================================================================
// dbusbind.c gap-fill stubs
// =========================================================================

pub(crate) fn builtin_dbus_make_inhibitor_lock(
    ctx: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_range_args("dbus-make-inhibitor-lock", &args, 2, 3)?;
    // GNU dbusbind.c:Fdbus_make_inhibitor_lock performs CHECK_STRING on
    // WHAT and WHY before any D-Bus side effect.
    let what_text = expect_string(&args[0])?;
    let why_text = expect_string(&args[1])?;
    let block = args.get(2).copied().unwrap_or(Value::NIL);
    let normalized_block = if block.is_nil() { Value::NIL } else { Value::T };
    let mode = if block.is_nil() { "delay" } else { "block" };

    let what = args[0];
    let why = args[1];
    let triple = Value::list(vec![what, why, normalized_block]);
    if let Some(registered) =
        dbus_rassoc_registered_triple(ctx.dbus_registered_inhibitor_locks, triple, ctx)
    {
        return Ok(registered.cons_car());
    }

    let root_scope = ctx.save_specpdl_roots();
    ctx.push_specpdl_root(what);
    ctx.push_specpdl_root(why);
    ctx.push_specpdl_root(triple);
    let lock = ctx.apply(
        Value::symbol("dbus-call-method"),
        vec![
            Value::keyword("system"),
            Value::string("org.freedesktop.login1"),
            Value::string("/org/freedesktop/login1"),
            Value::string("org.freedesktop.login1.Manager"),
            Value::string("Inhibit"),
            Value::string(what_text),
            Value::string("Emacs"),
            Value::string(why_text),
            Value::string(mode),
        ],
    );
    let lock = match lock {
        Ok(lock) => lock,
        Err(err) => {
            ctx.restore_specpdl_roots(root_scope);
            return Err(err);
        }
    };
    ctx.push_specpdl_root(lock);
    let entry = Value::cons(lock, triple);
    ctx.dbus_registered_inhibitor_locks = Value::cons(entry, ctx.dbus_registered_inhibitor_locks);
    ctx.restore_specpdl_roots(root_scope);
    Ok(lock)
}

pub(crate) fn builtin_dbus_close_inhibitor_lock(
    ctx: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("dbus-close-inhibitor-lock", &args, 1)?;
    // GNU dbusbind.c:Fdbus_close_inhibitor_lock starts with CHECK_FIXNUM.
    let lock = expect_fixnum(&args[0])?;
    let Some((_registered, updated)) =
        dbus_delete_registered_lock(ctx.dbus_registered_inhibitor_locks, args[0])
    else {
        return Ok(Value::NIL);
    };

    ctx.dbus_registered_inhibitor_locks = updated;
    #[cfg(unix)]
    {
        let result = unsafe { libc::close(lock as libc::c_int) };
        return Ok(if result == 0 { Value::T } else { Value::NIL });
    }
    #[cfg(not(unix))]
    Ok(Value::NIL)
}

pub(crate) fn builtin_dbus_registered_inhibitor_locks(
    ctx: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("dbus-registered-inhibitor-locks", &args, 0)?;
    Ok(dbus_copy_registered_inhibitor_locks(
        ctx.dbus_registered_inhibitor_locks,
    ))
}

fn dbus_rassoc_registered_triple(
    mut alist: Value,
    triple: Value,
    ctx: &crate::emacs_core::eval::Context,
) -> Option<Value> {
    while alist.is_cons() {
        let entry = alist.cons_car();
        if entry.is_cons()
            && crate::emacs_core::value::equal_value_swp(
                &entry.cons_cdr(),
                &triple,
                0,
                ctx.symbols_with_pos_enabled,
            )
        {
            return Some(entry);
        }
        alist = alist.cons_cdr();
    }
    None
}

fn dbus_delete_registered_lock(alist: Value, lock: Value) -> Option<(Value, Value)> {
    let mut cursor = alist;
    let mut result = Value::NIL;
    let mut removed = None;

    while cursor.is_cons() {
        let entry = cursor.cons_car();
        if removed.is_none() && entry.is_cons() && entry.cons_car().bits() == lock.bits() {
            removed = Some(entry);
        } else {
            result = Value::cons(entry, result);
        }
        cursor = cursor.cons_cdr();
    }

    removed.map(|entry| {
        (
            entry,
            crate::emacs_core::builtins::builtin_nreverse(vec![result]).unwrap_or(Value::NIL),
        )
    })
}

fn dbus_copy_registered_inhibitor_locks(mut alist: Value) -> Value {
    let mut result = Value::NIL;
    while alist.is_cons() {
        let entry = alist.cons_car();
        let copy =
            crate::emacs_core::builtins::builtin_copy_sequence(vec![entry]).unwrap_or(Value::NIL);
        result = Value::cons(copy, result);
        alist = alist.cons_cdr();
    }
    crate::emacs_core::builtins::builtin_nreverse(vec![result]).unwrap_or(Value::NIL)
}

// =========================================================================
// term.c gap-fill stubs
// =========================================================================

pub(crate) fn builtin_tty_frame_at(args: Vec<Value>) -> EvalResult {
    expect_args("tty-frame-at", &args, 2)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_tty_frame_geometry(args: Vec<Value>) -> EvalResult {
    expect_range_args("tty-frame-geometry", &args, 0, 1)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_tty_frame_edges(args: Vec<Value>) -> EvalResult {
    expect_range_args("tty-frame-edges", &args, 0, 2)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_tty_frame_list_z_order(args: Vec<Value>) -> EvalResult {
    expect_range_args("tty-frame-list-z-order", &args, 0, 1)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_tty_frame_restack(args: Vec<Value>) -> EvalResult {
    expect_range_args("tty-frame-restack", &args, 2, 3)?;
    Err(signal(
        "error",
        vec![Value::string("tty-frame-restack is not implemented")],
    ))
}

fn tty_display_dimension(
    ctx: &mut crate::emacs_core::eval::Context,
    name: &str,
    args: &[Value],
) -> Result<(i64, i64), Flow> {
    expect_range_args(name, args, 0, 1)?;

    let frame_id = match args.first().map(|value| value.kind()) {
        Some(ValueKind::Veclike(VecLikeType::Frame)) => {
            crate::window::FrameId(args[0].as_frame_id().unwrap())
        }
        _ => crate::emacs_core::window_cmds::ensure_selected_frame_id(ctx),
    };

    let Some(frame) = ctx.frames.get(frame_id) else {
        return Err(signal(
            "wrong-type-argument",
            vec![
                Value::symbol("framep"),
                args.first().copied().unwrap_or(Value::NIL),
            ],
        ));
    };

    if frame.initial {
        return Ok((80, 25));
    }

    Ok((i64::from(frame.columns()), i64::from(frame.lines())))
}

pub(crate) fn builtin_tty_display_pixel_width(
    ctx: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let (width, _) = tty_display_dimension(ctx, "tty-display-pixel-width", &args)?;
    Ok(Value::fixnum(width))
}

pub(crate) fn builtin_tty_display_pixel_height(
    ctx: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let (_, height) = tty_display_dimension(ctx, "tty-display-pixel-height", &args)?;
    Ok(Value::fixnum(height))
}

// =========================================================================
// lcms.c stubs (no lcms in NeoVM)
// =========================================================================

pub(crate) fn builtin_lcms2_available_p(args: Vec<Value>) -> EvalResult {
    expect_args("lcms2-available-p", &args, 0)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_lcms_cie_de2000(args: Vec<Value>) -> EvalResult {
    expect_range_args("lcms-cie-de2000", &args, 2, 5)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_lcms_xyz_to_jch(args: Vec<Value>) -> EvalResult {
    expect_range_args("lcms-xyz->jch", &args, 1, 3)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_lcms_jch_to_xyz(args: Vec<Value>) -> EvalResult {
    expect_range_args("lcms-jch->xyz", &args, 1, 3)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_lcms_jch_to_jab(args: Vec<Value>) -> EvalResult {
    expect_range_args("lcms-jch->jab", &args, 1, 3)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_lcms_jab_to_jch(args: Vec<Value>) -> EvalResult {
    expect_range_args("lcms-jab->jch", &args, 1, 3)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_lcms_cam02_ucs(args: Vec<Value>) -> EvalResult {
    expect_range_args("lcms-cam02-ucs", &args, 2, 4)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_lcms_temp_to_white_point(args: Vec<Value>) -> EvalResult {
    expect_args("lcms-temp->white-point", &args, 1)?;
    Ok(Value::NIL)
}

// =========================================================================
// neomacsfns.c gap-fill stubs
// =========================================================================

#[derive(Clone, Debug, PartialEq)]
pub struct NeomacsMonitorInfo {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub scale: f64,
    pub width_mm: i32,
    pub height_mm: i32,
    pub name: Option<String>,
}

pub fn set_neomacs_monitor_info(monitors: Vec<NeomacsMonitorInfo>) {
    NEOMACS_MONITORS.with(|slot| *slot.borrow_mut() = monitors);
}

pub fn neomacs_monitor_info_snapshot() -> Vec<NeomacsMonitorInfo> {
    NEOMACS_MONITORS.with(|slot| slot.borrow().clone())
}

fn set_cached_clipboard_text(text: Option<String>) {
    NEOMACS_CLIPBOARD_TEXT.with(|slot| *slot.borrow_mut() = text);
}

fn cached_clipboard_text() -> Option<String> {
    NEOMACS_CLIPBOARD_TEXT.with(|slot| slot.borrow().clone())
}

fn set_cached_primary_selection_text(text: Option<String>) {
    NEOMACS_PRIMARY_SELECTION_TEXT.with(|slot| *slot.borrow_mut() = text);
}

fn cached_primary_selection_text() -> Option<String> {
    NEOMACS_PRIMARY_SELECTION_TEXT.with(|slot| slot.borrow().clone())
}

fn set_system_clipboard_text(text: &str) -> bool {
    Clipboard::new()
        .and_then(|mut clipboard| clipboard.set_text(text.to_owned()))
        .is_ok()
}

fn get_system_clipboard_text() -> Option<String> {
    Clipboard::new()
        .ok()
        .and_then(|mut clipboard| clipboard.get_text().ok())
}

#[cfg(target_os = "linux")]
fn set_system_primary_selection_text(text: &str) -> bool {
    Clipboard::new()
        .and_then(|mut clipboard| {
            clipboard
                .set()
                .clipboard(LinuxClipboardKind::Primary)
                .text(text.to_owned())
        })
        .is_ok()
}

#[cfg(not(target_os = "linux"))]
fn set_system_primary_selection_text(_text: &str) -> bool {
    false
}

#[cfg(target_os = "linux")]
fn get_system_primary_selection_text() -> Option<String> {
    Clipboard::new().ok().and_then(|mut clipboard| {
        clipboard
            .get()
            .clipboard(LinuxClipboardKind::Primary)
            .text()
            .ok()
    })
}

#[cfg(not(target_os = "linux"))]
fn get_system_primary_selection_text() -> Option<String> {
    None
}

fn monitor_geometry_value(monitor: &NeomacsMonitorInfo) -> Value {
    Value::list(vec![
        Value::fixnum(monitor.x as i64),
        Value::fixnum(monitor.y as i64),
        Value::fixnum(monitor.width as i64),
        Value::fixnum(monitor.height as i64),
    ])
}

fn monitor_mm_size_value(monitor: &NeomacsMonitorInfo) -> Value {
    Value::list(vec![
        Value::fixnum(monitor.width_mm as i64),
        Value::fixnum(monitor.height_mm as i64),
    ])
}

fn monitor_alist_value(monitor: &NeomacsMonitorInfo, frames: Value) -> Value {
    Value::list(vec![
        Value::cons(Value::symbol("geometry"), monitor_geometry_value(monitor)),
        Value::cons(Value::symbol("workarea"), monitor_geometry_value(monitor)),
        Value::cons(Value::symbol("mm-size"), monitor_mm_size_value(monitor)),
        Value::cons(Value::symbol("frames"), frames),
        Value::cons(
            Value::symbol("scale-factor"),
            Value::make_float(monitor.scale),
        ),
        Value::cons(
            Value::symbol("name"),
            monitor
                .name
                .as_deref()
                .map(Value::string)
                .unwrap_or(Value::NIL),
        ),
        Value::cons(Value::symbol("source"), Value::string("Neomacs")),
    ])
}

pub(crate) fn builtin_neomacs_frame_geometry(args: Vec<Value>) -> EvalResult {
    expect_range_args("neomacs-frame-geometry", &args, 0, 1)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_neomacs_frame_edges(args: Vec<Value>) -> EvalResult {
    expect_range_args("neomacs-frame-edges", &args, 0, 2)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_neomacs_mouse_absolute_pixel_position(args: Vec<Value>) -> EvalResult {
    expect_args("neomacs-mouse-absolute-pixel-position", &args, 0)?;
    Ok(Value::cons(Value::fixnum(0), Value::fixnum(0)))
}

pub(crate) fn builtin_neomacs_set_mouse_absolute_pixel_position(args: Vec<Value>) -> EvalResult {
    expect_args("neomacs-set-mouse-absolute-pixel-position", &args, 2)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_neomacs_set_cursor_blink(
    eval: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_range_args("neomacs-set-cursor-blink", &args, 1, 2)?;
    let enabled = !args[0].is_nil();
    let interval_seconds = match args.get(1) {
        Some(value) if !value.is_nil() => expect_number(value)?,
        _ => 0.5,
    };
    let interval_ms = (interval_seconds.max(0.001) * 1000.0).round() as u32;
    if let Some(host) = eval.display_host.as_mut() {
        host.set_cursor_blink(enabled, interval_ms)
            .map_err(|message| {
                signal(
                    "error",
                    vec![Value::string(format!(
                        "neomacs-set-cursor-blink: {message}"
                    ))],
                )
            })?;
    }
    Ok(Value::NIL)
}

fn cursor_effect_arg(value: &Value) -> Result<crate::emacs_core::eval::CursorEffectArg, Flow> {
    if value.is_nil() {
        return Ok(crate::emacs_core::eval::CursorEffectArg::Nil);
    }
    if *value == Value::symbol("t") {
        return Ok(crate::emacs_core::eval::CursorEffectArg::Bool(true));
    }
    if let Some(text) = value.as_utf8_str() {
        return Ok(crate::emacs_core::eval::CursorEffectArg::String(
            text.to_owned(),
        ));
    }
    Ok(crate::emacs_core::eval::CursorEffectArg::Number(
        expect_number(value)?,
    ))
}

pub(crate) fn builtin_neomacs_set_cursor_animation(
    eval: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_range_args("neomacs-set-cursor-animation", &args, 1, 2)?;
    let enabled = !args[0].is_nil();
    let speed = match args.get(1) {
        Some(value) if !value.is_nil() => (expect_number(value)? as f32 / 100.0).max(0.001),
        _ => 2.4,
    };
    if let Some(host) = eval.display_host.as_mut() {
        host.set_cursor_animation(enabled, speed)
            .map_err(|message| {
                signal(
                    "error",
                    vec![Value::string(format!(
                        "neomacs-set-cursor-animation: {message}"
                    ))],
                )
            })?;
    }
    Ok(Value::NIL)
}

fn builtin_neomacs_set_cursor_effect(
    eval: &mut crate::emacs_core::eval::Context,
    subr_name: &str,
    effect_name: &str,
    args: Vec<Value>,
) -> EvalResult {
    expect_range_args(subr_name, &args, 1, 6)?;
    let effect_args = args
        .iter()
        .map(cursor_effect_arg)
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(host) = eval.display_host.as_mut() {
        host.set_cursor_effect(effect_name, effect_args)
            .map_err(|message| {
                signal(
                    "error",
                    vec![Value::string(format!("{subr_name}: {message}"))],
                )
            })?;
    }
    Ok(Value::NIL)
}

macro_rules! cursor_effect_builtin {
    ($fn_name:ident, $subr_name:literal, $effect_name:literal) => {
        pub(crate) fn $fn_name(
            eval: &mut crate::emacs_core::eval::Context,
            args: Vec<Value>,
        ) -> EvalResult {
            builtin_neomacs_set_cursor_effect(eval, $subr_name, $effect_name, args)
        }
    };
}

cursor_effect_builtin!(
    builtin_neomacs_set_cursor_glow,
    "neomacs-set-cursor-glow",
    "glow"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_pulse,
    "neomacs-set-cursor-pulse",
    "pulse"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_color_cycle,
    "neomacs-set-cursor-color-cycle",
    "color-cycle"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_shadow,
    "neomacs-set-cursor-shadow",
    "shadow"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_wake,
    "neomacs-set-cursor-wake",
    "wake"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_error_pulse,
    "neomacs-set-cursor-error-pulse",
    "error-pulse"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_crosshair,
    "neomacs-set-cursor-crosshair",
    "crosshair"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_magnetism,
    "neomacs-set-cursor-magnetism",
    "magnetism"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_comet,
    "neomacs-set-cursor-comet",
    "comet"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_spotlight,
    "neomacs-set-cursor-spotlight",
    "spotlight"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_particles,
    "neomacs-set-cursor-particles",
    "particles"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_trail_fade,
    "neomacs-set-cursor-trail-fade",
    "trail-fade"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_size_transition,
    "neomacs-set-cursor-size-transition",
    "size-transition"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_elastic_snap,
    "neomacs-set-cursor-elastic-snap",
    "elastic-snap"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_ghost,
    "neomacs-set-cursor-ghost",
    "ghost"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_ripple_wave,
    "neomacs-set-cursor-ripple-wave",
    "ripple-wave"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_lighthouse,
    "neomacs-set-cursor-lighthouse",
    "lighthouse"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_sonar_ping,
    "neomacs-set-cursor-sonar-ping",
    "sonar-ping"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_orbit_particles,
    "neomacs-set-cursor-orbit-particles",
    "orbit-particles"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_heartbeat,
    "neomacs-set-cursor-heartbeat",
    "heartbeat"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_metronome,
    "neomacs-set-cursor-metronome",
    "metronome"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_radar,
    "neomacs-set-cursor-radar",
    "radar"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_ripple_ring,
    "neomacs-set-cursor-ripple-ring",
    "ripple-ring"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_scope,
    "neomacs-set-cursor-scope",
    "scope"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_shockwave,
    "neomacs-set-cursor-shockwave",
    "shockwave"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_gravity_well,
    "neomacs-set-cursor-gravity-well",
    "gravity-well"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_water_drop,
    "neomacs-set-cursor-water-drop",
    "water-drop"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_pixel_dust,
    "neomacs-set-cursor-pixel-dust",
    "pixel-dust"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_candle_flame,
    "neomacs-set-cursor-candle-flame",
    "candle-flame"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_moth_flame,
    "neomacs-set-cursor-moth-flame",
    "moth-flame"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_sparkler,
    "neomacs-set-cursor-sparkler",
    "sparkler"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_plasma_ball,
    "neomacs-set-cursor-plasma-ball",
    "plasma-ball"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_quill_pen,
    "neomacs-set-cursor-quill-pen",
    "quill-pen"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_aurora_borealis,
    "neomacs-set-cursor-aurora-borealis",
    "aurora-borealis"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_feather,
    "neomacs-set-cursor-feather",
    "feather"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_stardust,
    "neomacs-set-cursor-stardust",
    "stardust"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_compass_needle,
    "neomacs-set-cursor-compass-needle",
    "compass-needle"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_galaxy,
    "neomacs-set-cursor-galaxy",
    "galaxy"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_prism,
    "neomacs-set-cursor-prism",
    "prism"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_moth,
    "neomacs-set-cursor-moth",
    "moth"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_flame,
    "neomacs-set-cursor-flame",
    "flame"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_crystal,
    "neomacs-set-cursor-crystal",
    "crystal"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_lightning,
    "neomacs-set-cursor-lightning",
    "lightning"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_snowflake,
    "neomacs-set-cursor-snowflake",
    "snowflake"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_firework,
    "neomacs-set-cursor-firework",
    "firework"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_tornado,
    "neomacs-set-cursor-tornado",
    "tornado"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_portal,
    "neomacs-set-cursor-portal",
    "portal"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_bubble,
    "neomacs-set-cursor-bubble",
    "bubble"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_sparkle_burst,
    "neomacs-set-cursor-sparkle-burst",
    "sparkle-burst"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_compass,
    "neomacs-set-cursor-compass",
    "compass"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_dna_helix,
    "neomacs-set-cursor-dna-helix",
    "dna-helix"
);
cursor_effect_builtin!(
    builtin_neomacs_set_cursor_pendulum,
    "neomacs-set-cursor-pendulum",
    "pendulum"
);

pub(crate) fn builtin_neomacs_display_monitor_attributes_list(
    eval: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_range_args("neomacs-display-monitor-attributes-list", &args, 0, 1)?;
    let frames = eval
        .frames
        .frame_list()
        .into_iter()
        .map(|fid| Value::make_frame(fid.0))
        .collect::<Vec<_>>();
    let monitor_values = neomacs_monitor_info_snapshot();
    if monitor_values.is_empty() {
        return Ok(Value::NIL);
    }

    let mut alists = Vec::with_capacity(monitor_values.len());
    for (index, monitor) in monitor_values.iter().enumerate() {
        let frame_list = if index == 0 {
            Value::list(frames.clone())
        } else {
            Value::NIL
        };
        alists.push(monitor_alist_value(monitor, frame_list));
    }
    Ok(Value::list(alists))
}

pub(crate) fn builtin_x_scroll_bar_foreground(args: Vec<Value>) -> EvalResult {
    expect_args("x-scroll-bar-foreground", &args, 1)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_x_scroll_bar_background(args: Vec<Value>) -> EvalResult {
    expect_args("x-scroll-bar-background", &args, 1)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_neomacs_clipboard_set(args: Vec<Value>) -> EvalResult {
    expect_args("neomacs-clipboard-set", &args, 1)?;
    let text = match args[0].kind() {
        ValueKind::Nil => None,
        ValueKind::String => Some(
            args[0]
                .as_runtime_string_owned()
                .expect("ValueKind::String must carry LispString payload"),
        ),
        _ => Some(format!("{}", args[0])),
    };
    set_cached_clipboard_text(text.clone());
    if let Some(text) = text {
        let _ = set_system_clipboard_text(&text);
    }
    Ok(Value::NIL)
}

pub(crate) fn builtin_neomacs_clipboard_get(args: Vec<Value>) -> EvalResult {
    expect_args("neomacs-clipboard-get", &args, 0)?;
    Ok(get_system_clipboard_text()
        .or_else(cached_clipboard_text)
        .map(Value::string)
        .unwrap_or(Value::NIL))
}

pub(crate) fn builtin_neomacs_primary_selection_set(args: Vec<Value>) -> EvalResult {
    expect_args("neomacs-primary-selection-set", &args, 1)?;
    let text = match args[0].kind() {
        ValueKind::Nil => None,
        ValueKind::String => Some(
            args[0]
                .as_runtime_string_owned()
                .expect("ValueKind::String must carry LispString payload"),
        ),
        _ => Some(format!("{}", args[0])),
    };
    set_cached_primary_selection_text(text.clone());
    if let Some(text) = text {
        let _ = set_system_primary_selection_text(&text);
    }
    Ok(Value::NIL)
}

pub(crate) fn builtin_neomacs_primary_selection_get(args: Vec<Value>) -> EvalResult {
    expect_args("neomacs-primary-selection-get", &args, 0)?;
    Ok(get_system_primary_selection_text()
        .or_else(cached_primary_selection_text)
        .map(Value::string)
        .unwrap_or(Value::NIL))
}

pub(crate) fn builtin_neomacs_core_backend(args: Vec<Value>) -> EvalResult {
    expect_args("neomacs-core-backend", &args, 0)?;
    Ok(Value::string("rust"))
}

pub(super) fn reset_stubs_thread_locals() {
    super::super::sqlite::reset_sqlite_thread_locals();
    NEOMACS_CLIPBOARD_TEXT.with(|slot| *slot.borrow_mut() = None);
    NEOMACS_PRIMARY_SELECTION_TEXT.with(|slot| *slot.borrow_mut() = None);
    NEOMACS_MONITORS.with(|slot| slot.borrow_mut().clear());
    FILE_NOTIFY_STATE.with(|slot| *slot.borrow_mut() = FileNotifyState::default());
}

thread_local! {
    static NEOMACS_CLIPBOARD_TEXT: RefCell<Option<String>> = const { RefCell::new(None) };
    static NEOMACS_PRIMARY_SELECTION_TEXT: RefCell<Option<String>> = const { RefCell::new(None) };
    static NEOMACS_MONITORS: RefCell<Vec<NeomacsMonitorInfo>> = const { RefCell::new(Vec::new()) };
}

/// Resolve a Lisp window designator to a `WindowId`.
///
/// Mirrors GNU's `decode_any_window` for the new_pixel / new_total
/// / new_normal accessor family. A bare integer is interpreted as a
/// raw window id (matching the long-standing test fixtures), and a
/// real window value is unwrapped via `as_window_id`.
fn window_designator_to_id(value: &Value) -> Option<crate::window::WindowId> {
    if let Some(wid) = value.as_window_id() {
        return Some(crate::window::WindowId(wid));
    }
    match value.kind() {
        ValueKind::Fixnum(id) if id >= 0 => Some(crate::window::WindowId(id as u64)),
        _ => None,
    }
}

pub(super) fn window_new_normal_value(
    eval: &super::eval::Context,
    window: Option<&Value>,
) -> Value {
    let Some(id) = window.and_then(window_designator_to_id) else {
        return Value::NIL;
    };
    eval.frames.window_new_normal(id)
}

pub(super) fn set_window_new_normal_value(
    eval: &mut super::eval::Context,
    window: &Value,
    value: Value,
) -> Value {
    if let Some(id) = window_designator_to_id(window) {
        eval.frames.set_window_new_normal(id, value);
    }
    value
}

pub(super) fn window_new_pixel_value(eval: &super::eval::Context, window: Option<&Value>) -> Value {
    let Some(id) = window.and_then(window_designator_to_id) else {
        return Value::fixnum(0);
    };
    Value::fixnum(eval.frames.window_new_pixel(id).unwrap_or(0))
}

pub(super) fn set_window_new_pixel_value(
    eval: &mut super::eval::Context,
    window: &Value,
    size: i64,
    add: bool,
) -> Value {
    let Some(id) = window_designator_to_id(window) else {
        return Value::fixnum(size);
    };
    Value::fixnum(eval.frames.set_window_new_pixel(id, size, add))
}

pub(super) fn window_new_total_value(eval: &super::eval::Context, window: Option<&Value>) -> Value {
    let Some(id) = window.and_then(window_designator_to_id) else {
        return Value::fixnum(0);
    };
    Value::fixnum(eval.frames.window_new_total(id).unwrap_or(0))
}

pub(super) fn set_window_new_total_value(
    eval: &mut super::eval::Context,
    window: &Value,
    size: i64,
    add: bool,
) -> Value {
    let Some(id) = window_designator_to_id(window) else {
        return Value::fixnum(size);
    };
    Value::fixnum(eval.frames.set_window_new_total(id, size, add))
}

fn fillarray_character_code_from_value(value: &Value) -> Result<u32, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n)
            if (0..=crate::emacs_core::emacs_char::MAX_CHAR as i64).contains(&n) =>
        {
            Ok(n as u32)
        }
        _ => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("characterp"), *value],
        )),
    }
}

pub(crate) fn builtin_fillarray(args: Vec<Value>) -> EvalResult {
    const BOOL_VECTOR_SIZE_SLOT: usize = 1;
    const BOOL_VECTOR_BITS_START: usize = 2;

    expect_args("fillarray", &args, 2)?;
    match args[0].kind() {
        ValueKind::Veclike(VecLikeType::CharTable) => {
            super::chartable::fill_char_table_from_fillarray(&args[0], args[1])?;
            Ok(args[0])
        }
        ValueKind::Veclike(VecLikeType::Vector) => {
            let is_bool_vector = super::chartable::is_bool_vector(&args[0]);
            let is_char_table = !is_bool_vector && super::chartable::is_char_table(&args[0]);
            if is_bool_vector {
                let fill_bit = if args[1].is_nil() { 0 } else { 1 };
                let v = args[0].as_vector_data().unwrap();
                let logical_len = match v.get(BOOL_VECTOR_SIZE_SLOT).map(|val| val.kind()) {
                    Some(ValueKind::Fixnum(n)) if n > 0 => n as usize,
                    _ => 0,
                };
                let available_bits = v.len().saturating_sub(BOOL_VECTOR_BITS_START);
                let bit_count = logical_len.min(available_bits);
                let mut vec = v.clone();
                for bit in vec.iter_mut().skip(BOOL_VECTOR_BITS_START).take(bit_count) {
                    *bit = Value::fixnum(fill_bit);
                }
                let _ = args[0].replace_vector_data(vec);
                return Ok(args[0]);
            }
            if is_char_table {
                super::chartable::fill_char_table_from_fillarray(&args[0], args[1])?;
                return Ok(args[0]);
            }
            let fill_len = args[0].as_vector_data().map_or(0, |vec| vec.len());
            let _ = args[0].replace_vector_data(vec![args[1]; fill_len]);
            Ok(args[0])
        }
        ValueKind::String => {
            let fill = fillarray_character_code_from_value(&args[1])?;
            let string = args[0].as_lisp_string().expect("string");
            let len = string.schars();
            let size_byte = string.sbytes();
            if len == 0 {
                return Ok(args[0]);
            }

            let mut fill_bytes = [0u8; crate::emacs_core::emacs_char::MAX_MULTIBYTE_LENGTH];
            let fill_len = if string.is_multibyte() {
                let mut buf = [0u8; crate::emacs_core::emacs_char::MAX_MULTIBYTE_LENGTH];
                let written = crate::emacs_core::emacs_char::char_string(fill, &mut buf);
                fill_bytes[..written].copy_from_slice(&buf[..written]);
                written
            } else {
                fill_bytes[0] = fill as u8;
                1
            };

            let new_size_byte = len.checked_mul(fill_len).ok_or_else(|| {
                signal(
                    "error",
                    vec![Value::string("Attempt to change byte length of a string")],
                )
            })?;
            if new_size_byte != size_byte {
                return Err(signal(
                    "error",
                    vec![Value::string("Attempt to change byte length of a string")],
                ));
            }

            let _ = args[0].with_lisp_string_mut(|lisp_str| {
                lisp_str.mutate_bytes(|bytes| {
                    if fill_len == 1 && len == size_byte {
                        bytes.fill(fill_bytes[0]);
                    } else {
                        for (idx, byte) in bytes.iter_mut().enumerate() {
                            *byte = fill_bytes[idx % fill_len];
                        }
                    }
                });
            });
            Ok(args[0])
        }
        _ => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("arrayp"), args[0]],
        )),
    }
}

pub(crate) fn builtin_define_fringe_bitmap(args: Vec<Value>) -> EvalResult {
    expect_range_args("define-fringe-bitmap", &args, 2, 5)?;
    if args[0].as_symbol_name().is_none() {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("symbolp"), args[0]],
        ));
    }
    if !matches!(
        args[1].kind(),
        ValueKind::Veclike(VecLikeType::Vector) | ValueKind::String
    ) {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("arrayp"), args[1]],
        ));
    }

    if let Some(height) = args.get(2) {
        if !height.is_nil() {
            let _ = expect_fixnum(height)?;
        }
    }
    if let Some(width) = args.get(3) {
        if !width.is_nil() {
            let _ = expect_fixnum(width)?;
        }
    }
    // GNU fringe.c: ALIGN can be a symbol (top, bottom, center) or a
    // list of alignment flags like (top repeat). Accept any non-nil value.
    // The actual fringe rendering is a stub; just validate minimally.

    Ok(args[0])
}

pub(crate) fn builtin_destroy_fringe_bitmap(args: Vec<Value>) -> EvalResult {
    expect_args("destroy-fringe-bitmap", &args, 1)?;
    if args[0].as_symbol_name().is_none() {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("symbolp"), args[0]],
        ));
    }
    Ok(Value::NIL)
}

pub(crate) fn builtin_display_line_is_continued_p(args: Vec<Value>) -> EvalResult {
    expect_args("display--line-is-continued-p", &args, 0)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_display_update_for_mouse_movement(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("display--update-for-mouse-movement", &args, 3)?;
    let fid = super::window_cmds::resolve_frame_id_in_state(
        &mut eval.frames,
        &mut eval.buffers,
        Some(&args[0]),
        "frame-live-p",
    )?;
    let x = expect_fixnum(&args[1])?;
    let y = expect_fixnum(&args[2])?;
    eval.note_mouse_move_for_frame(Some(fid), x, y);
    Ok(Value::NIL)
}

pub(crate) fn builtin_external_debugging_output(
    eval: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("external-debugging-output", &args, 1)?;
    let ch = expect_fixnum(&args[0])?;
    if ch < 0 {
        return Err(signal(
            "error",
            vec![Value::string("Invalid character: f03fffff")],
        ));
    }
    let character = char::from_u32(ch as u32).ok_or_else(|| {
        signal(
            "error",
            vec![Value::string(format!("Invalid character: {ch:x}"))],
        )
    })?;
    let mut encoded = [0; 4];
    let bytes = character.encode_utf8(&mut encoded).as_bytes();
    if let Some(file) = eval.debugging_output_file.as_mut() {
        use std::io::Write;
        file.write_all(bytes)
            .and_then(|_| file.flush())
            .map_err(|err| signal("file-error", vec![Value::string(err.to_string())]))?;
    } else {
        use std::io::Write;
        std::io::stderr()
            .write_all(bytes)
            .and_then(|_| std::io::stderr().flush())
            .map_err(|err| signal("file-error", vec![Value::string(err.to_string())]))?;
    }
    Ok(args[0])
}

pub(crate) fn builtin_internal_labeled_narrow_to_region(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_internal_labeled_narrow_to_region_in_buffers(&mut eval.buffers, args)
}

pub(crate) fn builtin_internal_labeled_narrow_to_region_in_buffers(
    buffers: &mut BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("internal--labeled-narrow-to-region", &args, 3)?;
    let start = super::buffers::expect_integer_or_marker_in_buffers(buffers, &args[0])?;
    let end = super::buffers::expect_integer_or_marker_in_buffers(buffers, &args[1])?;
    let label = args[2];
    let current_id = buffers
        .current_buffer_id()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    let (byte_start, byte_end) = super::buffers::normalize_narrow_region_in_buffers(
        buffers, current_id, start, end, args[0], args[1],
    )?;
    let _ = buffers.internal_labeled_narrow_to_region(current_id, byte_start, byte_end, label);
    Ok(Value::NIL)
}

pub(crate) fn builtin_internal_labeled_widen(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_internal_labeled_widen_in_buffers(&mut eval.buffers, args)
}

pub(crate) fn builtin_internal_labeled_widen_in_buffers(
    buffers: &mut BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("internal--labeled-widen", &args, 1)?;
    let current_id = buffers
        .current_buffer_id()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    let _ = buffers.internal_labeled_widen(current_id, &args[0]);
    Ok(Value::NIL)
}

pub(crate) fn builtin_internal_obarray_buckets(args: Vec<Value>) -> EvalResult {
    expect_args("internal--obarray-buckets", &args, 1)?;
    let obarray_val = expect_obarray_vector_id(&args[0])?;
    let buckets = super::symbols::obarray_buckets(obarray_val).unwrap_or_default();
    Ok(Value::list(buckets))
}

pub(crate) fn builtin_handle_save_session(args: Vec<Value>) -> EvalResult {
    expect_args("handle-save-session", &args, 1)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_handle_switch_frame(args: Vec<Value>) -> EvalResult {
    expect_args("handle-switch-frame", &args, 1)?;
    let frame = match args[0].kind() {
        ValueKind::Veclike(VecLikeType::Frame) => args[0],
        ValueKind::Cons => {
            let pair_car = args[0].cons_car();
            let pair_cdr = args[0].cons_cdr();
            match pair_car.as_symbol_name() {
                Some("switch-frame") => {
                    let cdr = pair_cdr;
                    match cdr.kind() {
                        ValueKind::Cons => cdr.cons_car(),
                        _ => {
                            return Err(signal(
                                "wrong-type-argument",
                                vec![Value::symbol("framep"), args[0]],
                            ));
                        }
                    }
                }
                _ => {
                    return Err(signal(
                        "wrong-type-argument",
                        vec![Value::symbol("framep"), args[0]],
                    ));
                }
            }
        }
        _ => {
            return Err(signal(
                "wrong-type-argument",
                vec![Value::symbol("framep"), args[0]],
            ));
        }
    };
    if !frame.is_frame() {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("framep"), frame],
        ));
    }
    Ok(Value::NIL)
}

pub(crate) fn builtin_gpm_mouse_start(args: Vec<Value>) -> EvalResult {
    expect_args("gpm-mouse-start", &args, 0)?;
    Err(signal(
        "error",
        vec![Value::string(
            "Gpm-mouse only works in the GNU/Linux console",
        )],
    ))
}

pub(crate) fn builtin_gpm_mouse_stop(args: Vec<Value>) -> EvalResult {
    expect_args("gpm-mouse-stop", &args, 0)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_help_describe_vector(args: Vec<Value>) -> EvalResult {
    expect_args("help--describe-vector", &args, 7)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_init_image_library(args: Vec<Value>) -> EvalResult {
    expect_args("init-image-library", &args, 1)?;
    let available = args[0]
        .as_symbol_name()
        .is_some_and(super::super::image::is_supported_image_type);
    Ok(Value::bool_val(available))
}

pub(crate) fn builtin_describe_buffer_bindings(args: Vec<Value>) -> EvalResult {
    expect_range_args("describe-buffer-bindings", &args, 1, 3)?;
    if !args[0].is_buffer() {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("bufferp"), args[0]],
        ));
    }
    if let Some(prefixes) = args.get(1) {
        if !prefixes.is_nil()
            && !(prefixes.is_cons()
                || prefixes.is_vector()
                || prefixes.is_string()
                || prefixes.is_nil())
        {
            return Err(signal(
                "wrong-type-argument",
                vec![Value::symbol("sequencep"), *prefixes],
            ));
        }
    }
    Ok(Value::NIL)
}

pub(crate) fn builtin_describe_vector(
    eval: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_range_args("describe-vector", &args, 1, 2)?;
    let is_char_table = super::chartable::is_char_table(&args[0]);
    if !is_char_table && !matches!(args[0].kind(), ValueKind::Veclike(VecLikeType::Vector)) {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("vector-or-char-table-p"), args[0]],
        ));
    }
    let formatter = args
        .get(1)
        .copied()
        .filter(|value| !value.is_nil())
        .unwrap_or_else(|| Value::symbol("princ"));

    if is_char_table {
        let mut first = true;
        for (key, value) in describe_vector_char_table_entries(&args[0])? {
            describe_vector_insert_entry(eval, formatter, key, value, &mut first)?;
        }
    } else if let Some(items) = args[0].as_vector_data() {
        let mut first = true;
        for (index, value) in items.iter().enumerate() {
            if value.is_nil() {
                continue;
            }
            let key = Value::fixnum(index as i64);
            describe_vector_insert_entry(eval, formatter, key, *value, &mut first)?;
        }
    }

    Ok(Value::NIL)
}

fn describe_vector_insert_entry(
    eval: &mut crate::emacs_core::eval::Context,
    formatter: Value,
    key: Value,
    value: Value,
    first: &mut bool,
) -> EvalResult {
    if *first {
        super::buffers::builtin_insert(eval, vec![Value::string("\n")])?;
        *first = false;
    }

    let key_text = describe_vector_key_name(key);
    super::buffers::builtin_insert(eval, vec![Value::string(&key_text)])?;

    // GNU keymap.c:describe_vector_princ indents to column 16 with minimum
    // one separating column before calling the element describer.
    let key_width = key_text.chars().count();
    let spaces = if key_width < 16 { 16 - key_width } else { 1 };
    super::buffers::builtin_insert(eval, vec![Value::string(" ".repeat(spaces))])?;
    eval.apply(formatter, vec![value])?;
    super::buffers::builtin_insert(eval, vec![Value::string("\n")])?;
    Ok(Value::NIL)
}

fn describe_vector_char_table_entries(table: &Value) -> Result<Vec<(Value, Value)>, Flow> {
    let entries = super::chartable::char_table_local_entries(table)?;
    let mut slots = vec![Value::NIL; 256];
    for (key, value) in entries {
        match key.kind() {
            ValueKind::Fixnum(ch) if (0..=255).contains(&ch) => {
                slots[ch as usize] = value;
            }
            ValueKind::Cons => {
                let start = key.cons_car().as_fixnum().unwrap_or(0).clamp(0, 255);
                let end = key
                    .cons_cdr()
                    .as_fixnum()
                    .unwrap_or(start)
                    .clamp(start, 255);
                for ch in start..=end {
                    slots[ch as usize] = value;
                }
            }
            _ => {}
        }
    }

    let mut runs = Vec::new();
    let mut start = 0usize;
    while start < slots.len() {
        if slots[start].is_nil() {
            start += 1;
            continue;
        }
        let value = slots[start];
        let mut end = start;
        while end + 1 < slots.len() && slots[end + 1] == value {
            end += 1;
        }
        let key = if start == end {
            Value::fixnum(start as i64)
        } else {
            Value::cons(Value::fixnum(start as i64), Value::fixnum(end as i64))
        };
        runs.push((key, value));
        start = end + 1;
    }
    Ok(runs)
}

fn describe_vector_key_name(key: Value) -> String {
    if key.is_cons() {
        let start = key.cons_car().as_fixnum().unwrap_or(0);
        let end = key.cons_cdr().as_fixnum().unwrap_or(start);
        format!(
            "{} .. {}",
            describe_vector_char_name(start),
            describe_vector_char_name(end)
        )
    } else {
        describe_vector_char_name(key.as_fixnum().unwrap_or(0))
    }
}

fn describe_vector_char_name(code: i64) -> String {
    match code {
        0 => "C-@".to_string(),
        1..=8 => format!(
            "C-{}",
            char::from_u32((code as u32) + b'a' as u32 - 1).unwrap()
        ),
        9 => "TAB".to_string(),
        10 => "C-j".to_string(),
        11 => "C-k".to_string(),
        12 => "C-l".to_string(),
        13 => "RET".to_string(),
        14..=26 => format!(
            "C-{}",
            char::from_u32((code as u32) + b'a' as u32 - 1).unwrap()
        ),
        27 => "ESC".to_string(),
        28 => "C-\\".to_string(),
        29 => "C-]".to_string(),
        30 => "C-^".to_string(),
        31 => "C-_".to_string(),
        32 => "SPC".to_string(),
        127 => "DEL".to_string(),
        _ => char::from_u32(code as u32)
            .map(|ch| ch.to_string())
            .unwrap_or_else(|| code.to_string()),
    }
}

pub(crate) fn builtin_frame_set_was_invisible(args: Vec<Value>) -> EvalResult {
    expect_args("frame--set-was-invisible", &args, 2)?;
    expect_frame_live_or_nil(&args[0])?;
    Ok(args[1])
}

pub(crate) fn builtin_frame_after_make_frame(args: Vec<Value>) -> EvalResult {
    expect_args("frame-after-make-frame", &args, 2)?;
    expect_frame_live_or_nil(&args[0])?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_frame_ancestor_p(args: Vec<Value>) -> EvalResult {
    expect_args("frame-ancestor-p", &args, 2)?;
    expect_frame_live_or_nil(&args[0])?;
    expect_frame_live_or_nil(&args[1])?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_frame_bottom_divider_width(args: Vec<Value>) -> EvalResult {
    expect_range_args("frame-bottom-divider-width", &args, 0, 1)?;
    if let Some(frame) = args.first() {
        expect_frame_live_or_nil(frame)?;
    }
    Ok(Value::fixnum(0))
}

pub(crate) fn builtin_frame_child_frame_border_width(args: Vec<Value>) -> EvalResult {
    expect_range_args("frame-child-frame-border-width", &args, 0, 1)?;
    if let Some(frame) = args.first() {
        expect_frame_live_or_nil(frame)?;
    }
    Ok(Value::fixnum(0))
}

pub(crate) fn builtin_frame_focus(args: Vec<Value>) -> EvalResult {
    expect_range_args("frame-focus", &args, 0, 1)?;
    if let Some(frame) = args.first() {
        expect_frame_live_or_nil(frame)?;
    }
    Ok(Value::NIL)
}

pub(crate) fn builtin_frame_font_cache(args: Vec<Value>) -> EvalResult {
    expect_range_args("frame-font-cache", &args, 0, 1)?;
    if let Some(frame) = args.first() {
        expect_frame_live_or_nil(frame)?;
    }
    Ok(Value::NIL)
}

pub(crate) fn builtin_frame_fringe_width(args: Vec<Value>) -> EvalResult {
    expect_range_args("frame-fringe-width", &args, 0, 1)?;
    if let Some(frame) = args.first() {
        expect_frame_live_or_nil(frame)?;
    }
    Ok(Value::fixnum(0))
}

pub(crate) fn builtin_frame_internal_border_width(args: Vec<Value>) -> EvalResult {
    expect_range_args("frame-internal-border-width", &args, 0, 1)?;
    if let Some(frame) = args.first() {
        expect_frame_live_or_nil(frame)?;
    }
    Ok(Value::fixnum(0))
}

pub(crate) fn builtin_frame_or_buffer_changed_p(args: Vec<Value>) -> EvalResult {
    expect_range_args("frame-or-buffer-changed-p", &args, 0, 1)?;
    let Some(symbol) = args.first() else {
        return Ok(Value::T);
    };
    if symbol.is_nil() {
        return Ok(Value::NIL);
    }
    if symbol.as_symbol_name().is_none() {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("symbolp"), *symbol],
        ));
    }
    Err(signal("void-variable", vec![*symbol]))
}

pub(crate) fn builtin_frame_parent(args: Vec<Value>) -> EvalResult {
    expect_range_args("frame-parent", &args, 0, 1)?;
    if let Some(frame) = args.first() {
        expect_frame_live_or_nil(frame)?;
    }
    Ok(Value::NIL)
}

pub(crate) fn builtin_frame_pointer_visible_p(args: Vec<Value>) -> EvalResult {
    expect_range_args("frame-pointer-visible-p", &args, 0, 1)?;
    if let Some(frame) = args.first() {
        expect_frame_live_or_nil(frame)?;
    }
    Ok(Value::T)
}

pub(crate) fn builtin_frame_right_divider_width(args: Vec<Value>) -> EvalResult {
    expect_range_args("frame-right-divider-width", &args, 0, 1)?;
    if let Some(frame) = args.first() {
        expect_frame_live_or_nil(frame)?;
    }
    Ok(Value::fixnum(0))
}

pub(crate) fn builtin_frame_scale_factor(args: Vec<Value>) -> EvalResult {
    expect_range_args("frame-scale-factor", &args, 0, 1)?;
    if let Some(frame) = args.first() {
        expect_frame_live_or_nil(frame)?;
    }
    Ok(Value::make_float(1.0))
}

pub(crate) fn builtin_frame_scroll_bar_height(args: Vec<Value>) -> EvalResult {
    expect_range_args("frame-scroll-bar-height", &args, 0, 1)?;
    if let Some(frame) = args.first() {
        expect_frame_live_or_nil(frame)?;
    }
    Ok(Value::fixnum(0))
}

pub(crate) fn builtin_frame_scroll_bar_width(args: Vec<Value>) -> EvalResult {
    expect_range_args("frame-scroll-bar-width", &args, 0, 1)?;
    if let Some(frame) = args.first() {
        expect_frame_live_or_nil(frame)?;
    }
    Ok(Value::fixnum(0))
}

pub(crate) fn builtin_frame_window_state_change(args: Vec<Value>) -> EvalResult {
    expect_range_args("frame-window-state-change", &args, 0, 1)?;
    if let Some(frame) = args.first() {
        expect_frame_live_or_nil(frame)?;
    }
    Ok(Value::NIL)
}

// --- frame.c missing builtins ---

/// Eval-dependent variant: defaults to selected frame.
pub(crate) fn builtin_frame_id(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_range_args("frame-id", &args, 0, 1)?;
    let fid = super::window_cmds::resolve_frame_id_in_state(
        &mut eval.frames,
        &mut eval.buffers,
        args.first(),
        "frame-live-p",
    )?;
    let public_id = if fid.0 >= crate::window::FRAME_ID_BASE {
        fid.0 - crate::window::FRAME_ID_BASE + 1
    } else {
        fid.0
    };
    Ok(Value::fixnum(public_id as i64))
}

/// Eval-dependent variant: defaults to selected frame.
pub(crate) fn builtin_frame_root_frame(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_range_args("frame-root-frame", &args, 0, 1)?;
    let fid = super::window_cmds::resolve_frame_id_in_state(
        &mut eval.frames,
        &mut eval.buffers,
        args.first(),
        "frame-live-p",
    )?;
    let root = eval.frames.root_frame_id(fid).unwrap_or(fid);
    Ok(Value::make_frame(root.0))
}

/// `(set-frame-size-and-position-pixelwise FRAME WIDTH HEIGHT LEFT TOP &optional GRAVITY)`
/// — combined resize+move stub, returns nil.
pub(crate) fn builtin_set_frame_size_and_position_pixelwise(args: Vec<Value>) -> EvalResult {
    expect_range_args("set-frame-size-and-position-pixelwise", &args, 5, 6)?;
    expect_frame_live_or_nil(&args[0])?;
    Ok(Value::NIL)
}

/// `(mouse-position-in-root-frame)` — stub, returns nil.
pub(crate) fn builtin_mouse_position_in_root_frame(args: Vec<Value>) -> EvalResult {
    expect_args("mouse-position-in-root-frame", &args, 0)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_fringe_bitmaps_at_pos(args: Vec<Value>) -> EvalResult {
    expect_range_args("fringe-bitmaps-at-pos", &args, 0, 2)?;
    if let Some(pos) = args.first() {
        if !pos.is_nil() {
            let _ = expect_integer_or_marker(pos)?;
        }
    }
    if let Some(window) = args.get(1) {
        if !window.is_nil() && !window.is_window() {
            return Err(signal(
                "wrong-type-argument",
                vec![Value::symbol("window-live-p"), *window],
            ));
        }
    }
    Ok(Value::NIL)
}

pub(crate) fn builtin_gap_position(args: Vec<Value>) -> EvalResult {
    expect_args("gap-position", &args, 0)?;
    Ok(Value::fixnum(1))
}

pub(crate) fn builtin_gap_size(args: Vec<Value>) -> EvalResult {
    expect_args("gap-size", &args, 0)?;
    Ok(Value::fixnum(2001))
}

pub(crate) fn builtin_garbage_collect_maybe(args: Vec<Value>) -> EvalResult {
    expect_args("garbage-collect-maybe", &args, 1)?;
    let Some(n) = args[0].as_fixnum() else {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("wholenump"), args[0]],
        ));
    };
    if n < 0 {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("wholenump"), Value::fixnum(n)],
        ));
    }
    Ok(Value::NIL)
}

pub(crate) fn builtin_garbage_collect_heapsize(args: Vec<Value>) -> EvalResult {
    expect_args("garbage-collect-heapsize", &args, 0)?;
    Ok(Value::fixnum(0))
}

pub(crate) fn builtin_get_unicode_property_internal(args: Vec<Value>) -> EvalResult {
    expect_args("get-unicode-property-internal", &args, 2)?;
    Err(signal(
        "wrong-type-argument",
        vec![Value::symbol("char-table-p"), args[0]],
    ))
}

pub(crate) fn builtin_gnutls_available_p(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-available-p", &args, 0)?;
    Ok(Value::list(vec![Value::symbol("gnutls")]))
}

pub(crate) fn builtin_gnutls_ciphers(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-ciphers", &args, 0)?;
    Ok(Value::list(vec![Value::symbol("AES-256-GCM")]))
}

pub(crate) fn builtin_gnutls_digests(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-digests", &args, 0)?;
    Ok(Value::list(vec![Value::symbol("SHA256")]))
}

pub(crate) fn builtin_gnutls_macs(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-macs", &args, 0)?;
    Ok(Value::list(vec![Value::symbol("AEAD")]))
}

pub(crate) fn builtin_gnutls_errorp(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-errorp", &args, 1)?;
    Ok(Value::T)
}

pub(crate) fn builtin_gnutls_error_string(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-error-string", &args, 1)?;
    match args[0].kind() {
        ValueKind::Fixnum(0) => Ok(Value::string("Success.")),
        ValueKind::Nil => Ok(Value::string("Symbol has no numeric gnutls-code property")),
        _ => Ok(Value::string("Unknown TLS error")),
    }
}

pub(crate) fn builtin_gnutls_error_fatalp(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-error-fatalp", &args, 1)?;
    if args[0].is_nil() {
        return Err(signal(
            "error",
            vec![Value::string("Symbol has no numeric gnutls-code property")],
        ));
    }
    Ok(Value::NIL)
}

fn expect_processp(value: &Value) -> Result<(), Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) if n >= 0 => Ok(()),
        _ => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), *value],
        )),
    }
}

pub(crate) fn builtin_gnutls_peer_status_warning_describe(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-peer-status-warning-describe", &args, 1)?;
    if args[0].is_nil() {
        return Ok(Value::NIL);
    }
    if args[0].as_symbol_name().is_none() {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("symbolp"), args[0]],
        ));
    }
    Ok(Value::NIL)
}

pub(crate) fn builtin_gnutls_asynchronous_parameters(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-asynchronous-parameters", &args, 2)?;
    expect_processp(&args[0])?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_gnutls_bye(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-bye", &args, 2)?;
    expect_processp(&args[0])?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_gnutls_deinit(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-deinit", &args, 1)?;
    expect_processp(&args[0])?;
    Ok(Value::T)
}

pub(crate) fn builtin_gnutls_format_certificate(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-format-certificate", &args, 1)?;
    let _ = expect_strict_string(&args[0])?;
    Ok(Value::string("Certificate"))
}

pub(crate) fn builtin_gnutls_get_initstage(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-get-initstage", &args, 1)?;
    expect_processp(&args[0])?;
    Ok(Value::fixnum(0))
}

pub(crate) fn builtin_gnutls_hash_digest(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-hash-digest", &args, 2)?;
    if args[0].is_nil() {
        return Err(signal(
            "error",
            vec![
                Value::string("GnuTLS digest-method is invalid or not found"),
                Value::NIL,
            ],
        ));
    }
    if args[0].as_symbol_name().is_none() {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("symbolp"), args[0]],
        ));
    }
    let _ = expect_strict_string(&args[1])?;
    Ok(Value::string("digest"))
}

pub(crate) fn builtin_gnutls_hash_mac(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-hash-mac", &args, 3)?;
    if args[0].is_nil() {
        return Err(signal(
            "error",
            vec![
                Value::string("GnuTLS MAC-method is invalid or not found"),
                Value::NIL,
            ],
        ));
    }
    if args[0].as_symbol_name().is_none() {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("symbolp"), args[0]],
        ));
    }
    let _ = expect_strict_string(&args[1])?;
    let _ = expect_strict_string(&args[2])?;
    Ok(Value::string("mac"))
}

pub(crate) fn builtin_gnutls_peer_status(args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-peer-status", &args, 1)?;
    expect_processp(&args[0])?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_gnutls_symmetric_decrypt(args: Vec<Value>) -> EvalResult {
    expect_range_args("gnutls-symmetric-decrypt", &args, 4, 5)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_gnutls_symmetric_encrypt(args: Vec<Value>) -> EvalResult {
    expect_range_args("gnutls-symmetric-encrypt", &args, 4, 5)?;
    Ok(Value::NIL)
}

pub(super) const FACE_ATTRIBUTES_VECTOR_LEN: usize = 20;

pub(crate) fn builtin_font_get_system_font(args: Vec<Value>) -> EvalResult {
    expect_args("font-get-system-font", &args, 0)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_font_get_system_normal_font(args: Vec<Value>) -> EvalResult {
    expect_args("font-get-system-normal-font", &args, 0)?;
    Ok(Value::NIL)
}

fn expect_characterp_from_int(value: &Value) -> Result<char, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) if n >= 0 => char::from_u32(n as u32).ok_or_else(|| {
            signal(
                "wrong-type-argument",
                vec![Value::symbol("characterp"), *value],
            )
        }),
        _ => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("characterp"), *value],
        )),
    }
}

fn is_font_object(value: &Value) -> bool {
    match value.kind() {
        ValueKind::Veclike(VecLikeType::Vector) => {
            let items = value.as_vector_data().unwrap();
            items
                .first()
                .and_then(|value| value.as_symbol_name())
                .is_some_and(|name| name == "font-object" || name == ":font-object")
        }
        _ => false,
    }
}

fn is_font_spec(value: &Value) -> bool {
    match value.kind() {
        ValueKind::Veclike(VecLikeType::Vector) => {
            let items = value.as_vector_data().unwrap();
            items
                .first()
                .and_then(|value| value.as_symbol_name())
                .is_some_and(|name| name == "font-spec" || name == ":font-spec")
        }
        _ => false,
    }
}

fn unspecified_face_attributes_vector() -> Value {
    Value::vector(vec![
        Value::symbol("unspecified");
        FACE_ATTRIBUTES_VECTOR_LEN
    ])
}

pub(crate) fn builtin_face_attributes_as_vector(args: Vec<Value>) -> EvalResult {
    expect_args("face-attributes-as-vector", &args, 1)?;
    Ok(unspecified_face_attributes_vector())
}

fn expect_window_live_or_nil_in_state(frames: &FrameManager, value: &Value) -> Result<(), Flow> {
    if value.is_nil() {
        return Ok(());
    }
    let live = if let Some(wid) = value.as_window_id() {
        frames.is_live_window_id(WindowId(wid))
    } else {
        match value.kind() {
            ValueKind::Fixnum(id) if id >= 0 => frames.is_live_window_id(WindowId(id as u64)),
            _ => false,
        }
    };
    if live {
        Ok(())
    } else {
        Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("window-live-p"), *value],
        ))
    }
}

pub(crate) fn builtin_font_face_attributes(args: Vec<Value>) -> EvalResult {
    expect_range_args("font-face-attributes", &args, 1, 2)?;
    if !is_font_object(&args[0]) {
        return Err(signal("error", vec![Value::string("Invalid font object")]));
    }
    Ok(unspecified_face_attributes_vector())
}

pub(crate) fn builtin_font_get_glyphs(args: Vec<Value>) -> EvalResult {
    expect_range_args("font-get-glyphs", &args, 3, 4)?;
    if !is_font_object(&args[0]) {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("font-object"), args[0]],
        ));
    }
    let _ = expect_fixnum(&args[1])?;
    let _ = expect_fixnum(&args[2])?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_font_has_char_p(args: Vec<Value>) -> EvalResult {
    expect_range_args("font-has-char-p", &args, 2, 3)?;
    if !is_font_object(&args[0]) && !is_font_spec(&args[0]) {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("font"), args[0]],
        ));
    }
    let _ = expect_characterp_from_int(&args[1])?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_font_match_p(args: Vec<Value>) -> EvalResult {
    expect_args("font-match-p", &args, 2)?;
    if !is_font_spec(&args[0]) {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("font-spec"), args[0]],
        ));
    }
    if !is_font_spec(&args[1]) {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("font-spec"), args[1]],
        ));
    }
    Ok(Value::NIL)
}

pub(crate) fn builtin_font_shape_gstring(args: Vec<Value>) -> EvalResult {
    expect_args("font-shape-gstring", &args, 2)?;
    if !matches!(args[0].kind(), ValueKind::Veclike(VecLikeType::Vector)) {
        return Err(signal(
            "error",
            vec![Value::string("Invalid glyph-string: ")],
        ));
    }
    let _ = expect_fixnum(&args[1])?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_font_variation_glyphs(args: Vec<Value>) -> EvalResult {
    expect_args("font-variation-glyphs", &args, 2)?;
    if !is_font_object(&args[0]) {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("font-object"), args[0]],
        ));
    }
    let _ = expect_characterp_from_int(&args[1])?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_fontset_font(args: Vec<Value>) -> EvalResult {
    expect_range_args("fontset-font", &args, 2, 3)?;
    let ch = expect_characterp_from_int(&args[1])?;
    fontset::fontset_font(
        &args[0],
        ch,
        args.get(2).is_some_and(|value| !value.is_nil()),
    )
}

pub(crate) fn builtin_fontset_info(args: Vec<Value>) -> EvalResult {
    expect_range_args("fontset-info", &args, 1, 2)?;
    Err(signal(
        "error",
        vec![Value::string(
            "Window system is not in use or not initialized",
        )],
    ))
}

pub(crate) fn builtin_fontset_list(args: Vec<Value>) -> EvalResult {
    expect_args("fontset-list", &args, 0)?;
    Ok(super::symbols::fontset_list_value())
}

fn expect_window_live_or_nil(value: &Value) -> Result<(), Flow> {
    if value.is_nil() || value.is_window() {
        Ok(())
    } else {
        Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("window-live-p"), *value],
        ))
    }
}

pub(super) fn expect_window_valid_or_nil(value: &Value) -> Result<(), Flow> {
    if value.is_nil() || value.is_window() {
        Ok(())
    } else {
        Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("window-valid-p"), *value],
        ))
    }
}

fn expect_frame_live_or_nil(value: &Value) -> Result<(), Flow> {
    if value.is_nil() || value.is_frame() {
        Ok(())
    } else {
        Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("frame-live-p"), *value],
        ))
    }
}

pub(crate) fn builtin_window_bottom_divider_width(args: Vec<Value>) -> EvalResult {
    expect_range_args("window-bottom-divider-width", &args, 0, 1)?;
    if let Some(window) = args.first() {
        expect_window_live_or_nil(window)?;
    }
    Ok(Value::fixnum(0))
}

/// `(window-lines-pixel-dimensions &optional WINDOW FIRST LAST BODY INVERSE NO-RESTRICT)`
///
/// GNU `src/window.c::Fwindow_lines_pixel_dimensions` walks the
/// window's display matrix and returns a list of
/// `(width . height)` pairs (one per glyph row) plus the
/// total height. neomacs's display matrix lives in the layout
/// engine, not in `neovm-core`, so this builtin cannot read it
/// directly without going through the renderer round trip.
///
/// Window audit Low 13 in `drafts/window-system-audit.md`:
/// returning `nil` is the GNU-documented "no information
/// available" answer (the same value GNU uses on a TTY frame
/// before any redisplay), so callers that probe with
/// `(or (window-lines-pixel-dimensions ...) ...)` get the
/// expected fallback. Building real glyph-row data requires
/// piping the matrix builder snapshot back into neovm-core,
/// which is part of the cursor audit Finding 11
/// (`display_and_set_cursor` collapse) restructuring.
pub(crate) fn builtin_window_lines_pixel_dimensions(args: Vec<Value>) -> EvalResult {
    expect_range_args("window-lines-pixel-dimensions", &args, 0, 6)?;
    if let Some(window) = args.first() {
        expect_window_live_or_nil(window)?;
    }
    Ok(Value::NIL)
}

pub(crate) fn builtin_window_new_normal(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_range_args("window-new-normal", &args, 0, 1)?;
    if let Some(window) = args.first() {
        expect_window_valid_or_nil(window)?;
    }
    Ok(window_new_normal_value(eval, args.first()))
}

pub(crate) fn builtin_window_new_pixel(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_range_args("window-new-pixel", &args, 0, 1)?;
    if let Some(window) = args.first() {
        expect_window_valid_or_nil(window)?;
    }
    Ok(window_new_pixel_value(eval, args.first()))
}

pub(crate) fn builtin_window_new_total(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_range_args("window-new-total", &args, 0, 1)?;
    if let Some(window) = args.first() {
        expect_window_valid_or_nil(window)?;
    }
    Ok(window_new_total_value(eval, args.first()))
}

pub(crate) fn builtin_window_old_body_pixel_height(args: Vec<Value>) -> EvalResult {
    expect_range_args("window-old-body-pixel-height", &args, 0, 1)?;
    if let Some(window) = args.first() {
        expect_window_live_or_nil(window)?;
    }
    Ok(Value::fixnum(0))
}

pub(crate) fn builtin_window_old_body_pixel_width(args: Vec<Value>) -> EvalResult {
    expect_range_args("window-old-body-pixel-width", &args, 0, 1)?;
    if let Some(window) = args.first() {
        expect_window_live_or_nil(window)?;
    }
    Ok(Value::fixnum(0))
}

pub(crate) fn builtin_window_old_pixel_height(args: Vec<Value>) -> EvalResult {
    expect_range_args("window-old-pixel-height", &args, 0, 1)?;
    if let Some(window) = args.first() {
        expect_window_valid_or_nil(window)?;
    }
    Ok(Value::fixnum(0))
}

pub(crate) fn builtin_window_old_pixel_width(args: Vec<Value>) -> EvalResult {
    expect_range_args("window-old-pixel-width", &args, 0, 1)?;
    if let Some(window) = args.first() {
        expect_window_valid_or_nil(window)?;
    }
    Ok(Value::fixnum(0))
}

pub(crate) fn builtin_window_right_divider_width(args: Vec<Value>) -> EvalResult {
    expect_range_args("window-right-divider-width", &args, 0, 1)?;
    if let Some(window) = args.first() {
        expect_window_live_or_nil(window)?;
    }
    Ok(Value::fixnum(0))
}

pub(crate) fn builtin_window_scroll_bar_height(args: Vec<Value>) -> EvalResult {
    expect_range_args("window-scroll-bar-height", &args, 0, 1)?;
    if let Some(window) = args.first() {
        expect_window_live_or_nil(window)?;
    }
    Ok(Value::fixnum(0))
}

pub(crate) fn builtin_window_scroll_bar_width(args: Vec<Value>) -> EvalResult {
    expect_range_args("window-scroll-bar-width", &args, 0, 1)?;
    if let Some(window) = args.first() {
        expect_window_live_or_nil(window)?;
    }
    Ok(Value::fixnum(0))
}

thread_local! {
    static FILE_NOTIFY_STATE: RefCell<FileNotifyState> = RefCell::new(FileNotifyState::default());
}

pub(crate) const INOTIFY_FEATURE_AVAILABLE: bool = true;

#[derive(Debug)]
struct FileWatch {
    id: i64,
    path: String,
}

#[derive(Debug)]
struct FileNotifyState {
    watcher: Option<notify::RecommendedWatcher>,
    _rx: Option<std::sync::mpsc::Receiver<Result<notify::Event, notify::Error>>>,
    watches: Vec<FileWatch>,
    next_id: i64,
}

impl Default for FileNotifyState {
    fn default() -> Self {
        Self {
            watcher: None,
            _rx: None,
            watches: Vec::new(),
            next_id: 0,
        }
    }
}

impl FileNotifyState {
    fn ensure_watcher(&mut self) -> Result<(), Flow> {
        if self.watcher.is_some() {
            return Ok(());
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let watcher = notify::RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                let _ = tx.send(res);
            },
            notify::Config::default(),
        )
        .map_err(|e| {
            file_notify_error("File watching is not available", Some(e.to_string()), None)
        })?;
        self.watcher = Some(watcher);
        self._rx = Some(rx);
        Ok(())
    }

    fn allocate_id(&mut self) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

fn file_notify_error(message: &str, detail: Option<String>, object: Option<Value>) -> Flow {
    let mut tail = match object {
        Some(object) if object.is_cons() => object,
        Some(object) if !object.is_nil() => Value::list(vec![object]),
        _ => Value::NIL,
    };
    if let Some(detail) = detail {
        tail = Value::cons(Value::string(&detail), tail);
    }
    let raw_data = Value::cons(Value::string(message), tail);
    crate::emacs_core::error::signal_with_data("file-notify-error", raw_data)
}

fn inotify_unknown_aspect_error(aspect: Value) -> Flow {
    file_notify_error(
        "Unknown aspect",
        Some("Invalid argument".to_string()),
        Some(aspect),
    )
}

fn inotify_invalid_descriptor_error(descriptor: Value, detail: &str) -> Flow {
    file_notify_error(
        "Invalid descriptor ",
        Some(detail.to_string()),
        Some(descriptor),
    )
}

fn inotify_aspect_symbol_valid(name: &str) -> bool {
    matches!(
        name,
        "access"
            | "attrib"
            | "close-write"
            | "close-nowrite"
            | "create"
            | "delete"
            | "delete-self"
            | "modify"
            | "move-self"
            | "moved-from"
            | "moved-to"
            | "open"
            | "move"
            | "close"
            | "dont-follow"
            | "onlydir"
            | "ignored"
            | "unmount"
            | "all-events"
            | "t"
    )
}

fn validate_inotify_aspect(aspect: Value) -> Result<(), Flow> {
    if aspect.is_nil() {
        return Ok(());
    }
    if let Some(name) = aspect.as_symbol_name() {
        return if inotify_aspect_symbol_valid(name) {
            Ok(())
        } else {
            Err(inotify_unknown_aspect_error(aspect))
        };
    }
    if !aspect.is_cons() {
        return Err(inotify_unknown_aspect_error(aspect));
    }

    let mut rest = aspect;
    while rest.is_cons() {
        let item = rest.cons_car();
        let Some(name) = item.as_symbol_name() else {
            return Err(inotify_unknown_aspect_error(item));
        };
        if !inotify_aspect_symbol_valid(name) {
            return Err(inotify_unknown_aspect_error(item));
        }
        rest = rest.cons_cdr();
    }
    if !rest.is_nil() {
        return Err(inotify_unknown_aspect_error(rest));
    }
    Ok(())
}

fn extract_valid_watch_id(value: Value) -> Option<i64> {
    if !value.is_cons() {
        return None;
    }
    let id = value.cons_car().as_int()?;
    let generation = value.cons_cdr().as_int()?;
    if id >= 0 && generation >= 0 {
        Some(id)
    } else {
        None
    }
}

pub(crate) fn builtin_inotify_valid_p(args: Vec<Value>) -> EvalResult {
    expect_args("inotify-valid-p", &args, 1)?;
    let Some(id) = extract_valid_watch_id(args[0]) else {
        return Ok(Value::NIL);
    };
    FILE_NOTIFY_STATE.with(|slot| {
        let state = slot.borrow();
        Ok(Value::bool_val(state.watches.iter().any(|w| w.id == id)))
    })
}

pub(crate) fn builtin_inotify_add_watch(args: Vec<Value>) -> EvalResult {
    expect_args("inotify-add-watch", &args, 3)?;
    let filename = expect_strict_string(&args[0])?;
    validate_inotify_aspect(args[1])?;

    FILE_NOTIFY_STATE.with(|slot| {
        let mut state = slot.borrow_mut();
        state.ensure_watcher()?;

        let path = std::path::Path::new(&filename);
        if !path.exists() {
            return Err(file_notify_error(
                "Could not add watch for file",
                Some("No such file or directory".to_string()),
                Some(args[0]),
            ));
        }
        if let Some(ref mut watcher) = state.watcher {
            watcher
                .watch(path, notify::RecursiveMode::NonRecursive)
                .map_err(|e| {
                    file_notify_error(
                        "Could not add watch for file",
                        Some(e.to_string()),
                        Some(args[0]),
                    )
                })?;
        }

        let id = state.allocate_id();
        state.watches.push(FileWatch {
            id,
            path: filename.clone(),
        });

        Ok(Value::cons(Value::fixnum(id), Value::fixnum(0)))
    })
}

pub(crate) fn builtin_inotify_rm_watch(args: Vec<Value>) -> EvalResult {
    expect_args("inotify-rm-watch", &args, 1)?;

    let detail = if args[0].is_cons() {
        "Invalid argument"
    } else {
        "No such file or directory"
    };
    let Some(id) = extract_valid_watch_id(args[0]) else {
        return Err(inotify_invalid_descriptor_error(args[0], detail));
    };

    FILE_NOTIFY_STATE.with(|slot| {
        let mut state = slot.borrow_mut();
        let Some(pos) = state.watches.iter().position(|w| w.id == id) else {
            return Ok(Value::T);
        };

        let removed = state.watches.remove(pos);
        if let Some(ref mut watcher) = state.watcher {
            let path = std::path::Path::new(&removed.path);
            let _ = watcher.unwatch(path);
        }

        if state.watches.is_empty() {
            state.watcher = None;
            state._rx = None;
        }

        Ok(Value::T)
    })
}

// =========================================================================
// eval.c gap-fill stubs
// =========================================================================

/// GNU eval.c:838 — return SYMBOL's toplevel buffer-local value in BUFFER.
///
/// "Toplevel" means outside any let binding.  This pure stub returns nil;
/// a full implementation needs eval access (buffer manager + dynamic stack)
/// and is dispatched via the eval-backed path in builtins/mod.rs.
pub(crate) fn builtin_buffer_local_toplevel_value(args: Vec<Value>) -> EvalResult {
    expect_range_args("buffer-local-toplevel-value", &args, 1, 2)?;
    Ok(Value::NIL)
}

/// GNU eval.c:857 — set SYMBOL's toplevel buffer-local value in BUFFER.
pub(crate) fn builtin_set_buffer_local_toplevel_value(args: Vec<Value>) -> EvalResult {
    expect_range_args("set-buffer-local-toplevel-value", &args, 2, 3)?;
    Ok(args[1])
}

pub(crate) fn builtin_debugger_trap(args: Vec<Value>) -> EvalResult {
    expect_args("debugger-trap", &args, 0)?;
    Ok(Value::NIL)
}

// =========================================================================
// coding.c gap-fill stubs
// =========================================================================

/// GNU coding.c:10362 — internal-decode-string-utf-8.
///
/// These are test/benchmark functions (inside ENABLE_UTF_8_CONVERTER_TEST
/// in GNU).  NeoVM stores all strings as UTF-8 natively, so decode is a
/// pass-through.  We validate arguments per GNU to return nil on bad input.
pub(crate) fn builtin_internal_decode_string_utf_8(args: Vec<Value>) -> EvalResult {
    expect_args("internal-decode-string-utf-8", &args, 7)?;
    // GNU returns nil if STRING is not a string.
    if args[0].as_utf8_str().is_none() {
        return Ok(Value::NIL);
    }
    // GNU: CHECK_FIXNUM(count)
    if !args[6].is_fixnum() {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("fixnump"), args[6]],
        ));
    }
    // NeoVM is UTF-8 natively; return the input string unchanged.
    Ok(args[0])
}

/// GNU coding.c:10306 — internal-encode-string-utf-8.
///
/// Same rationale as decode: NeoVM strings are already UTF-8.
pub(crate) fn builtin_internal_encode_string_utf_8(args: Vec<Value>) -> EvalResult {
    expect_args("internal-encode-string-utf-8", &args, 7)?;
    if args[0].as_utf8_str().is_none() {
        return Ok(Value::NIL);
    }
    if !args[6].is_fixnum() {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("fixnump"), args[6]],
        ));
    }
    Ok(args[0])
}

// =========================================================================
// buffer.c gap-fill stubs
// =========================================================================

pub(crate) fn builtin_overlay_tree(args: Vec<Value>) -> EvalResult {
    expect_range_args("overlay-tree", &args, 0, 1)?;
    Ok(Value::NIL)
}

// =========================================================================
// =========================================================================
// thread.c gap-fill stubs
// =========================================================================

pub(crate) fn builtin_thread_buffer_disposition(args: Vec<Value>) -> EvalResult {
    expect_args("thread-buffer-disposition", &args, 1)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_thread_set_buffer_disposition(args: Vec<Value>) -> EvalResult {
    expect_args("thread-set-buffer-disposition", &args, 2)?;
    // Stub: ignore the set
    Ok(Value::NIL)
}

// =========================================================================
// window.c gap-fill stubs
// =========================================================================

pub(crate) fn builtin_window_discard_buffer_from_window(args: Vec<Value>) -> EvalResult {
    expect_range_args("window-discard-buffer-from-window", &args, 2, 3)?;
    Ok(Value::NIL)
}

// `window-cursor-info` is implemented in
// `neovm-core/src/emacs_core/window_cmds/mod.rs::builtin_window_cursor_info`
// (cursor audit Finding 2). The placeholder that lived here used to
// return `nil` unconditionally.

pub(crate) fn builtin_combine_windows(args: Vec<Value>) -> EvalResult {
    expect_args("combine-windows", &args, 2)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_uncombine_window(args: Vec<Value>) -> EvalResult {
    expect_args("uncombine-window", &args, 1)?;
    Ok(Value::NIL)
}

// =========================================================================
// frame.c gap-fill stubs
// =========================================================================

pub(crate) fn builtin_frame_windows_min_size(args: Vec<Value>) -> EvalResult {
    expect_args("frame-windows-min-size", &args, 4)?;
    Ok(Value::fixnum(0))
}

// =========================================================================
// xdisp.c gap-fill stubs
// =========================================================================

pub(crate) fn builtin_remember_mouse_glyph(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("remember-mouse-glyph", &args, 3)?;
    if !args[0].is_nil() && !display::live_frame_designator_p(eval, &args[0]) {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("frame-live-p"), args[0]],
        ));
    }
    if !display::display_window_system_symbol_eval(eval, Some(&args[0]))?
        .is_some_and(display::gui_window_system_active_value)
    {
        return Err(signal(
            "error",
            vec![Value::string("Window system frame should be used")],
        ));
    }
    let _x = expect_fixnum(&args[1])?;
    let _y = expect_fixnum(&args[2])?;
    Ok(Value::NIL)
}

// =========================================================================
// image.c gap-fill stubs
// =========================================================================

pub(crate) fn builtin_lookup_image(args: Vec<Value>) -> EvalResult {
    expect_args("lookup-image", &args, 1)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_imagemagick_types(args: Vec<Value>) -> EvalResult {
    expect_args("imagemagick-types", &args, 0)?;
    Ok(Value::NIL)
}

// =========================================================================
// font.c gap-fill stubs
// =========================================================================

pub(crate) fn builtin_font_drive_otf(args: Vec<Value>) -> EvalResult {
    expect_args("font-drive-otf", &args, 6)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_font_otf_alternates(args: Vec<Value>) -> EvalResult {
    expect_args("font-otf-alternates", &args, 3)?;
    Ok(Value::NIL)
}

// =========================================================================
// emacs.c / version.c gap-fill stubs for loadup.el
// =========================================================================

pub(crate) fn builtin_emacs_repository_get_version(args: Vec<Value>) -> EvalResult {
    expect_args("emacs-repository-get-version", &args, 0)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_emacs_repository_get_branch(args: Vec<Value>) -> EvalResult {
    expect_args("emacs-repository-get-branch", &args, 0)?;
    Ok(Value::NIL)
}

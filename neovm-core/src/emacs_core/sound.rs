//! Sound playback support, matching GNU Emacs's sound.c.
//!
//! Provides real implementation for:
//! - `play-sound-internal` — audio playback via `rodio` crate (when `sound` feature is enabled)
//!
//! When the `sound` feature is disabled, `play-sound-internal` signals an error
//! matching GNU Emacs behavior when compiled without sound support.

use super::error::{EvalResult, Flow, signal};
use super::value::*;

// ---------------------------------------------------------------------------
// GNU Emacs sound spec parsing
// ---------------------------------------------------------------------------
//
// SOUND must be: (sound :file "path" :data "bytes" :volume N :device "dev")
// The leading `sound` symbol is required.
// Either :file or :data must be a string. Volume is 0-100 (int) or 0.0-1.0 (float).
// ---------------------------------------------------------------------------

struct SoundSpec {
    file: Option<String>,
    #[cfg(feature = "sound")]
    data: Option<Vec<u8>>,
    #[cfg(not(feature = "sound"))]
    has_data: bool,
    volume: f32,
}

fn parse_sound_spec(sound: Value) -> Result<SoundSpec, Flow> {
    let elements = super::value::list_to_vec(&sound).unwrap_or_default();
    if elements.is_empty() {
        return Err(signal(
            "error",
            vec![Value::string("Invalid sound specification")],
        ));
    }

    match elements[0].kind() {
        ValueKind::Symbol(s) => {
            if crate::emacs_core::intern::resolve_sym(s) != "sound" {
                return Err(signal(
                    "error",
                    vec![Value::string("Invalid sound specification")],
                ));
            }
        }
        _ => {
            return Err(signal(
                "error",
                vec![Value::string("Invalid sound specification")],
            ));
        }
    }

    let plist_val = if elements.len() > 1 {
        Value::list(elements[1..].to_vec())
    } else {
        Value::NIL
    };

    let file_val = super::plist::plist_get(plist_val, &Value::symbol(":file"));
    let data_val = super::plist::plist_get(plist_val, &Value::symbol(":data"));
    let volume_val = super::plist::plist_get(plist_val, &Value::symbol(":volume"));

    let file = match file_val {
        Some(v) if !v.is_nil() => match v.kind() {
            ValueKind::String => Some(
                v.as_lisp_string()
                    .unwrap()
                    .as_utf8_str()
                    .unwrap_or_default()
                    .to_string(),
            ),
            _ => {
                return Err(signal(
                    "error",
                    vec![Value::string("Invalid sound specification")],
                ));
            }
        },
        _ => None,
    };

    #[cfg(feature = "sound")]
    let data = match data_val {
        Some(v) if !v.is_nil() => match v.kind() {
            ValueKind::String => {
                let ls = v.as_lisp_string().unwrap();
                Some(ls.as_bytes().to_vec())
            }
            _ => {
                return Err(signal(
                    "error",
                    vec![Value::string("Invalid sound specification")],
                ));
            }
        },
        _ => None,
    };

    #[cfg(not(feature = "sound"))]
    let has_data = match data_val {
        Some(v) if !v.is_nil() => {
            // Just validate that data was provided (a string).
            match v.kind() {
                ValueKind::String => true,
                _ => {
                    return Err(signal(
                        "error",
                        vec![Value::string("Invalid sound specification")],
                    ));
                }
            }
        }
        _ => false,
    };

    let no_file = file.is_none();
    #[cfg(feature = "sound")]
    let no_data = data.is_none();
    #[cfg(not(feature = "sound"))]
    let no_data = !has_data;

    if no_file && no_data {
        return Err(signal(
            "error",
            vec![Value::string("Invalid sound specification")],
        ));
    }

    let volume = match volume_val {
        Some(v) if !v.is_nil() => match v.kind() {
            ValueKind::Fixnum(n) => {
                if !(0..=100).contains(&n) {
                    return Err(signal(
                        "error",
                        vec![Value::string("Invalid sound specification")],
                    ));
                }
                n as f32 / 100.0
            }
            ValueKind::Float => {
                let fv = v.xfloat();
                if !(0.0..=1.0).contains(&fv) {
                    return Err(signal(
                        "error",
                        vec![Value::string("Invalid sound specification")],
                    ));
                }
                fv as f32
            }
            _ => {
                return Err(signal(
                    "error",
                    vec![Value::string("Invalid sound specification")],
                ));
            }
        },
        _ => 1.0,
    };

    Ok(SoundSpec {
        file,
        #[cfg(feature = "sound")]
        data,
        #[cfg(not(feature = "sound"))]
        has_data,
        volume,
    })
}

// ---------------------------------------------------------------------------
// Playback via rodio (feature-gated)
// ---------------------------------------------------------------------------

#[cfg(feature = "sound")]
fn play_sound_file(path: &str, volume: f32) -> Result<(), Flow> {
    let file = std::fs::File::open(path).map_err(|e| {
        signal(
            "file-error",
            vec![
                Value::string(&format!("Cannot open sound file: {e}")),
                Value::string(path),
            ],
        )
    })?;

    let stream = rodio::OutputStream::try_default().map_err(|e| {
        signal(
            "error",
            vec![Value::string(&format!("No audio device: {e}"))],
        )
    })?;
    let (_stream, stream_handle) = stream;

    let sink = rodio::Sink::try_new(&stream_handle).map_err(|e| {
        signal(
            "error",
            vec![Value::string(&format!("Audio sink error: {e}"))],
        )
    })?;

    sink.set_volume(volume);
    sink.append(
        rodio::Decoder::new(std::io::BufReader::new(file)).map_err(|e| {
            signal(
                "error",
                vec![Value::string(&format!("Cannot decode sound: {e}"))],
            )
        })?,
    );

    sink.sleep_until_end();
    drop(sink);
    Ok(())
}

#[cfg(feature = "sound")]
fn play_sound_data(data: &[u8], volume: f32) -> Result<(), Flow> {
    use std::io::Cursor;

    let stream = rodio::OutputStream::try_default().map_err(|e| {
        signal(
            "error",
            vec![Value::string(&format!("No audio device: {e}"))],
        )
    })?;
    let (_stream, stream_handle) = stream;

    let sink = rodio::Sink::try_new(&stream_handle).map_err(|e| {
        signal(
            "error",
            vec![Value::string(&format!("Audio sink error: {e}"))],
        )
    })?;

    sink.set_volume(volume);
    sink.append(
        rodio::Decoder::new(Cursor::new(data.to_vec())).map_err(|e| {
            signal(
                "error",
                vec![Value::string(&format!("Cannot decode sound: {e}"))],
            )
        })?,
    );

    sink.sleep_until_end();
    drop(sink);
    Ok(())
}

// ---------------------------------------------------------------------------
// Builtin function
// ---------------------------------------------------------------------------

/// (play-sound-internal SOUND)
#[cfg(feature = "sound")]
pub(crate) fn builtin_play_sound_internal(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args("play-sound-internal", &args, 1)?;

    let spec = parse_sound_spec(args[0])?;

    if let Some(ref path) = spec.file {
        play_sound_file(path, spec.volume)?;
    } else if let Some(ref data) = spec.data {
        play_sound_data(data, spec.volume)?;
    }

    Ok(Value::NIL)
}

/// (play-sound-internal SOUND) — stub when sound feature is disabled.
#[cfg(not(feature = "sound"))]
pub(crate) fn builtin_play_sound_internal(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args("play-sound-internal", &args, 1)?;

    let _spec = parse_sound_spec(args[0])?;

    Err(signal(
        "error",
        vec![Value::string("Sound support not available")],
    ))
}

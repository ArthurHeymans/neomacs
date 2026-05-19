//! Zlib decompression support, matching GNU Emacs's decompress.c.
//!
//! Provides:
//! - `zlib-available-p`
//! - `zlib-decompress-region`

use std::io::Read;

use super::editfns::{
    buffer_read_only_active_in_state, current_buffer_byte_span_char_len, signal_after_change,
    signal_before_change,
};
use super::error::{EvalResult, Flow, signal};
use super::fns::{
    read_buffer_region_bytes_in_manager, replace_buffer_region_lisp_string_in_manager,
};
use super::value::*;
use crate::emacs_core::value::ValueKind;
use crate::heap_types::LispString;
use flate2::{Decompress, FlushDecompress, Status};

/// Resolve a Lisp integer-or-marker to an i64 position value.
fn expect_integer_or_marker(
    buffers: &crate::buffer::BufferManager,
    value: &Value,
) -> Result<i64, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) => Ok(n),
        _ if value.is_marker() => {
            super::marker::marker_position_as_int_with_buffers(buffers, value)
        }
        _ => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("integer-or-marker-p"), *value],
        )),
    }
}

/// (zlib-available-p)
/// Return t if zlib decompression is available.
pub(crate) fn builtin_zlib_available_p(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args("zlib-available-p", &args, 0)?;
    Ok(Value::T)
}

/// (zlib-decompress-region START END &optional ALLOW-PARTIAL)
///
/// Decompress gzip- or zlib-compressed region, replacing text in-place.
/// Must be called in a unibyte buffer.
/// Returns t on success, the number of unconsumed bytes on partial success
/// (when ALLOW-PARTIAL is non-nil), or nil on failure.
pub(crate) fn builtin_zlib_decompress_region(
    ctx: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::builtins::expect_min_args("zlib-decompress-region", &args, 2)?;
    super::builtins::expect_max_args("zlib-decompress-region", &args, 3)?;
    let allow_partial = args.get(2).is_some_and(|v| v.is_truthy());

    let Some(buf) = ctx.buffers.current_buffer() else {
        return Ok(Value::NIL);
    };

    let start = expect_integer_or_marker(&ctx.buffers, &args[0])?;
    let end = expect_integer_or_marker(&ctx.buffers, &args[1])?;

    // GNU `Fzlib_decompress_region` calls `validate_region` before the
    // unibyte-buffer check.
    let point_min = buf.point_min_char() as i64 + 1;
    let point_max = buf.point_max_char() as i64 + 1;
    if start < point_min || start > point_max || end < point_min || end > point_max {
        return Err(signal(
            "args-out-of-range",
            vec![Value::make_buffer(buf.id), args[0], args[1]],
        ));
    }

    // Check unibyte — GNU signals error in multibyte buffers.
    if buf.get_multibyte() {
        return Err(signal(
            "error",
            vec![Value::string(
                "This function can be called only in unibyte buffers",
            )],
        ));
    }

    // Check read-only.
    if buffer_read_only_active_in_state(&ctx.obarray, &[], buf) {
        return Err(signal("buffer-read-only", vec![Value::make_buffer(buf.id)]));
    }

    let (from, to) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    let from_byte = buf.lisp_pos_to_accessible_byte(from);
    let to_byte = buf.lisp_pos_to_accessible_byte(to);

    let buffer_id = buf.id;
    let compressed =
        read_buffer_region_bytes_in_manager(&ctx.buffers, buffer_id, from_byte, to_byte)?;

    // Try gzip first (most common for Emacs .gz files), then fall back to zlib.
    // GNU uses inflateInit2 with MAX_WBITS + 32 which auto-detects format.
    let decompressed = decompress_auto(&compressed, allow_partial);

    match decompressed {
        Some((data, remaining)) if remaining == 0 => {
            let replacement = LispString::from_emacs_bytes(data);
            let old_len = current_buffer_byte_span_char_len(ctx, from_byte, to_byte);
            let new_len = replacement.sbytes();
            signal_before_change(ctx, from_byte, to_byte)?;
            replace_buffer_region_lisp_string_in_manager(
                &mut ctx.buffers,
                buffer_id,
                from_byte,
                to_byte,
                &replacement,
            )?;
            signal_after_change(ctx, from_byte, from_byte + new_len, old_len)?;
            Ok(Value::T)
        }
        Some((data, remaining)) if allow_partial => {
            let replacement = LispString::from_emacs_bytes(data);
            let old_len = current_buffer_byte_span_char_len(ctx, from_byte, to_byte);
            let new_len = replacement.sbytes();
            signal_before_change(ctx, from_byte, to_byte)?;
            replace_buffer_region_lisp_string_in_manager(
                &mut ctx.buffers,
                buffer_id,
                from_byte,
                to_byte,
                &replacement,
            )?;
            signal_after_change(ctx, from_byte, from_byte + new_len, old_len)?;
            Ok(Value::fixnum(remaining as i64))
        }
        Some(_) => unreachable!("non-partial successful decompression handled above"),
        None if allow_partial => Ok(Value::fixnum((to_byte - from_byte) as i64)),
        None => {
            // Failure without allow-partial — leave region unchanged, return nil.
            Ok(Value::NIL)
        }
    }
}

/// Auto-detect compression format and decompress.
/// Tries gzip first (most common in Emacs), then raw zlib.
fn decompress_auto(compressed: &[u8], allow_partial: bool) -> Option<(Vec<u8>, usize)> {
    if let Some(result) = decompress_streaming_auto(compressed, allow_partial) {
        return Some(result);
    }
    // Gzip magic number: 0x1f 0x8b
    if compressed.len() >= 2 && compressed[0] == 0x1f && compressed[1] == 0x8b {
        if let Ok(data) = decompress_gzip(compressed) {
            return Some((data, 0));
        }
    }
    // Try zlib format.
    decompress_zlib(compressed).ok().map(|data| (data, 0))
}

fn decompress_streaming_auto(compressed: &[u8], allow_partial: bool) -> Option<(Vec<u8>, usize)> {
    if compressed.len() >= 2 && compressed[0] == 0x1f && compressed[1] == 0x8b {
        return None;
    }
    let mut decoder = Decompress::new(true);
    let mut output = Vec::new();
    loop {
        let before_in = decoder.total_in();
        let before_out = decoder.total_out();
        output.reserve(16 * 1024);
        match decoder.decompress_vec(compressed, &mut output, FlushDecompress::None) {
            Ok(Status::StreamEnd) => {
                return Some((output, 0));
            }
            Ok(Status::Ok) | Ok(Status::BufError) => {
                if decoder.total_in() == before_in && decoder.total_out() == before_out {
                    if allow_partial && !output.is_empty() {
                        let remaining =
                            compressed.len().saturating_sub(decoder.total_in() as usize);
                        return Some((output, remaining));
                    }
                    return None;
                }
            }
            Err(_) => {
                if allow_partial && !output.is_empty() {
                    let remaining = compressed.len().saturating_sub(decoder.total_in() as usize);
                    return Some((output, remaining));
                }
                return None;
            }
        }
    }
}

fn decompress_gzip(compressed: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut decoder = flate2::read::MultiGzDecoder::new(compressed);
    let mut output = Vec::new();
    decoder.read_to_end(&mut output)?;
    Ok(output)
}

fn decompress_zlib(compressed: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut decoder = flate2::read::ZlibDecoder::new(compressed);
    let mut output = Vec::new();
    decoder.read_to_end(&mut output)?;
    Ok(output)
}

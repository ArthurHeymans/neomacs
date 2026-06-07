//! Zlib decompression support, matching GNU Emacs's decompress.c.
//!
//! Provides:
//! - `zlib-available-p`
//! - `zlib-decompress-region`

use std::io::Read;

use super::editfns::{
    buffer_read_only_active_in_state, signal_after_text_change, signal_before_text_change,
    text_change_for_lisp_string_replacement_in_manager,
};
use super::error::{EvalResult, Flow, signal};
use super::fns::{
    read_buffer_region_bytes_in_manager, replace_buffer_region_lisp_string_in_manager,
};
use super::value::*;
use crate::heap_types::LispString;
use flate2::{Decompress, FlushDecompress, Status};

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

    let region = super::position::LispRegionArgs::from_values(&ctx.buffers, args[0], args[1])?;

    // GNU `Fzlib_decompress_region` calls `validate_region` before the
    // unibyte-buffer check.
    let byte_range = region.accessible_byte_range(buf)?;

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

    let from_byte = byte_range.start().get();
    let to_byte = byte_range.end().get();

    let buffer_id = buf.id;
    let compressed = read_buffer_region_bytes_in_manager(&ctx.buffers, buffer_id, byte_range)?;

    // Try gzip first (most common for Emacs .gz files), then fall back to zlib.
    // GNU uses inflateInit2 with MAX_WBITS + 32 which auto-detects format.
    let decompressed = decompress_auto(&compressed, allow_partial);

    match decompressed {
        Some((data, remaining)) if remaining == 0 => {
            let replacement = LispString::from_emacs_bytes(data);
            let change = text_change_for_lisp_string_replacement_in_manager(
                &ctx.buffers,
                buffer_id,
                byte_range,
                &replacement,
            )?;
            signal_before_text_change(ctx, change)?;
            replace_buffer_region_lisp_string_in_manager(
                &mut ctx.buffers,
                buffer_id,
                change.old_range(),
                &replacement,
            )?;
            signal_after_text_change(ctx, change)?;
            Ok(Value::T)
        }
        Some((data, remaining)) if allow_partial => {
            let replacement = LispString::from_emacs_bytes(data);
            let change = text_change_for_lisp_string_replacement_in_manager(
                &ctx.buffers,
                buffer_id,
                byte_range,
                &replacement,
            )?;
            signal_before_text_change(ctx, change)?;
            replace_buffer_region_lisp_string_in_manager(
                &mut ctx.buffers,
                buffer_id,
                change.old_range(),
                &replacement,
            )?;
            signal_after_text_change(ctx, change)?;
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

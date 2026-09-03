//! Platform-neutral Lisp error construction shared by native adapters.

use super::*;
use std::path::Path;

pub(super) fn file_name_to_lisp(ctx: &crate::emacs_core::eval::Context, path: &Path) -> Value {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;

        return Value::heap_string(crate::emacs_core::fileio::decode_file_name_lisp(
            ctx,
            path.as_os_str().as_bytes(),
        ));
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;

        let bytes = path
            .as_os_str()
            .encode_wide()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        return Value::heap_string(crate::encoding::decode_bytes_to_lisp_string(
            &bytes,
            "utf-16le",
            ctx.eol_conversion(),
        ));
    }
}

pub(crate) fn file_notify_error(
    message: &str,
    detail: Option<String>,
    object: Option<Value>,
) -> Flow {
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

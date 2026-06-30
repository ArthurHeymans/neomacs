//! Frame and startup bootstrap variables.
use crate::emacs_core::value::Value;

pub fn register_bootstrap_vars(obarray: &mut crate::emacs_core::symbol::Obarray) {
    obarray.set_symbol_value("default-frame-alist", Value::NIL);
    // GNU frame.c exposes this as a built-in variable. GUI builds default to a
    // concrete side instead of leaving scroll-bar.el to trip over an unbound var.
    obarray.set_symbol_value("default-frame-scroll-bars", Value::symbol("right"));
    obarray.set_symbol_value("initial-frame-alist", Value::NIL);
    obarray.set_symbol_value("initial-window-system", Value::NIL);
    obarray.set_symbol_value("window-system", Value::NIL);
    obarray.set_symbol_value("handle-args-function", Value::symbol("command-line-1"));
    obarray.set_symbol_value("handle-args-function-alist", Value::NIL);
    obarray.set_symbol_value("inhibit-x-resources", Value::NIL);
    obarray.set_symbol_value("resize-mini-windows", Value::symbol("grow-only"));
    // GNU `syms_of_xdisp` (xdisp.c:38639-38647) assigns BOTH frame-title-format
    // and icon-title-format the same structured default: `(multiple-frames "%b"
    // ("" "%b - GNU Emacs at " system-name))`, where the inner tail's last
    // element is the `system-name` symbol (resolved at title-render time).
    let icon_title_name_format = Value::list(vec![
        Value::string(""),
        Value::string("%b - GNU Emacs at "),
        Value::symbol("system-name"),
    ]);
    let title_format = Value::list(vec![
        Value::symbol("multiple-frames"),
        Value::string("%b"),
        icon_title_name_format,
    ]);
    obarray.set_symbol_value("frame-title-format", title_format);
    obarray.set_symbol_value("icon-title-format", title_format);
    obarray.set_symbol_value("frame-resize-pixelwise", Value::NIL);
    // GNU frame.c DEFVAR_BOOL (Emacs 31.1), default t: `delete-frame' selects
    // the most recently used frame (vs. the oldest visible one). Exposed here
    // so cus-start.el does not signal "built-in variable ... not bound".
    obarray.set_symbol_value("after-delete-frame-select-mru-frame", Value::T);
    obarray.set_symbol_value("focus-follows-mouse", Value::NIL);
    obarray.set_symbol_value("frame-inhibit-implied-resize", Value::NIL);
    obarray.set_symbol_value("terminal-frame", Value::NIL);
    obarray.set_symbol_value("frameset-filter-alist", Value::NIL);
    obarray.set_symbol_value("frameset-session-filter-alist", Value::NIL);
}

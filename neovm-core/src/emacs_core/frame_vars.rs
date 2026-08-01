//! Frame and startup bootstrap variables.
use crate::emacs_core::value::Value;

pub fn register_bootstrap_vars(obarray: &mut crate::emacs_core::symbol::Obarray) {
    obarray.set_symbol_value("default-frame-alist", Value::NIL);
    // GNU frame.c exposes this as a built-in variable. GUI builds default to a
    // concrete side instead of leaving scroll-bar.el to trip over an unbound var.
    obarray.set_symbol_value("default-frame-scroll-bars", Value::symbol("right"));
    obarray.set_symbol_value("initial-frame-alist", Value::NIL);
    obarray.set_symbol_value("initial-window-system", Value::NIL);
    // GNU graphical builds load term/common-win.el during loadup, which binds
    // this public display variable even for batch sessions.  Neomacs defers
    // its side-effectful GUI terminal layer until GUI startup, so preserve the
    // stable frame-variable surface here instead.
    obarray.define_special_variable("x-display-name", Value::NIL);
    // GNU `DEFVAR_KBOARD` both installs the forwarded value and declares the
    // symbol special.  Neomacs models the selected-frame value separately,
    // but Lisp bindings must retain the same dynamic-scope contract.
    obarray.define_special_variable("window-system", Value::NIL);
    obarray.set_symbol_value("handle-args-function", Value::symbol("command-line-1"));
    obarray.set_symbol_value("handle-args-function-alist", Value::NIL);
    obarray.set_symbol_value("inhibit-x-resources", Value::NIL);
    obarray.set_symbol_value("resize-mini-windows", Value::symbol("grow-only"));
    // GNU `syms_of_xdisp` (xdisp.c:38639-38647) assigns BOTH frame-title-format
    // and icon-title-format the same structured default: `(multiple-frames "%b"
    // ("" "%b - GNU Emacs at " system-name))`, where the inner tail's last
    // element is the `system-name` symbol (resolved at title-render time).
    //
    // Neomacs is NOT GNU Emacs: this is a DELIBERATE product-branding
    // divergence. The title bar must advertise "NEO Emacs", never "GNU Emacs".
    // We keep the structure (the `multiple-frames` form plus the trailing
    // `system-name` symbol) byte-for-byte identical to GNU and change only the
    // product name inside the literal. The oracle parity probe still locks that
    // structure: the shared normalizer canonicalizes the product name to
    // `[EMACS-PRODUCT]` on both engines, so the intentional brand difference is
    // ignored while every other part stays a parity assertion (see
    // neovm-oracle-tests/src/divergence/combos/strict/modeline_lnum_fringe_windowtree.rs
    // and the EMACS-PRODUCT rule in neovm-oracle-tests/src/common.rs).
    let icon_title_name_format = Value::list(vec![
        Value::string(""),
        Value::string("%b - NEO Emacs at "),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emacs_core::eval::Context;

    #[test]
    fn graphical_backend_display_name_is_bound_in_batch_like_gnu() {
        crate::test_utils::init_test_tracing();
        let eval = Context::new();

        assert_eq!(
            eval.obarray().symbol_value("x-display-name").copied(),
            Some(Value::NIL)
        );
        assert!(eval.obarray().is_special("x-display-name"));
    }
}

//! Buffer-related bootstrap variables.
use crate::emacs_core::value::Value;

pub fn register_bootstrap_vars(obarray: &mut crate::emacs_core::symbol::Obarray) {
    obarray.set_symbol_value("kill-buffer-query-functions", Value::NIL);
    obarray.set_symbol_value("kill-buffer-hook", Value::NIL);
    obarray.set_symbol_value("buffer-list-update-hook", Value::NIL);
    obarray.set_symbol_value("change-major-mode-hook", Value::NIL);
    obarray.set_symbol_value("after-change-major-mode-hook", Value::NIL);
    obarray.set_symbol_value("first-change-hook", Value::NIL);
    obarray.set_symbol_value("before-change-functions", Value::NIL);
    obarray.set_symbol_value("after-change-functions", Value::NIL);
    obarray.set_symbol_value("combine-after-change-calls", Value::NIL);
    // GNU `src/buffer.c` defines this with DEFVAR_LISP and initializes it to nil.
    obarray.set_symbol_value("inhibit-read-only", Value::NIL);
    obarray.set_symbol_value("inhibit-modification-hooks", Value::NIL);
    for name in [
        "first-change-hook",
        "before-change-functions",
        "after-change-functions",
        "combine-after-change-calls",
        "inhibit-read-only",
        "inhibit-modification-hooks",
        // GNU declares these hook variables with DEFVAR_LISP (buffer.c) or
        // `defvar' (subr.el/simple.el), which makes them special.  Without the
        // special mark, a `let'-binding of the hook under lexical-binding (as
        // the oracle harness uses via `eval FORM t') creates an invisible
        // lexical binding, so `run-hooks' reads the global value and the hook
        // never fires.  Marking them special restores GNU's dynamic-binding
        // semantics for `let'-bound hooks.
        "kill-buffer-query-functions",
        "kill-buffer-hook",
        "buffer-list-update-hook",
        "change-major-mode-hook",
        "after-change-major-mode-hook",
    ] {
        obarray.make_special(name);
    }
    // GNU Emacs marks this property in `syms_of_buffer` so changing major
    // modes does not discard a buffer's cleanup hook.
    obarray
        .put_property("kill-buffer-hook", "permanent-local", Value::T)
        .expect("bootstrap symbol properties must be writable");
    for name in [
        "inhibit-field-text-motion",
        "buffer-access-fontify-functions",
        "buffer-access-fontified-property",
    ] {
        obarray.set_symbol_value(name, Value::NIL);
        obarray.make_special(name);
    }
    obarray.set_symbol_value("buffer-file-coding-system", Value::NIL);
    obarray.set_symbol_value("buffer-file-format", Value::NIL);
    obarray.set_symbol_value("buffer-saved-size", Value::fixnum(0));
    obarray.set_symbol_value(
        "buffer-auto-save-file-format",
        Value::list(vec![Value::symbol("t")]),
    );
    obarray.set_symbol_value("buffer-stale-function", Value::NIL);
    obarray.set_symbol_value("buffer-undo-list", Value::NIL);
    obarray.set_symbol_value("buffer-display-table", Value::NIL);
    obarray.set_symbol_value("enable-multibyte-characters", Value::T);
    // `default-enable-multibyte-characters' was removed in Emacs 31 (only a
    // comment in mule-cmds.el mentions it); GNU leaves it unbound, so neomacs
    // must not seed it.
    obarray.set_symbol_value("find-file-hook", Value::NIL);
    obarray.set_symbol_value("find-file-not-found-functions", Value::NIL);
    obarray.set_symbol_value("major-mode", Value::symbol("fundamental-mode"));
    obarray.set_symbol_value("mode-name", Value::string("Fundamental"));
    obarray.set_symbol_value("local-abbrev-table", Value::NIL);
    obarray.set_symbol_value("fill-column", Value::fixnum(70));
    obarray.set_symbol_value("left-margin", Value::fixnum(0));
    // tab-width is set by init_indent_vars() with special=true
    obarray.set_symbol_value("ctl-arrow", Value::T);
    obarray.set_symbol_value("truncate-lines", Value::NIL);
    obarray.set_symbol_value("word-wrap", Value::NIL);
    obarray.set_symbol_value("word-wrap-by-category", Value::NIL);
    // GNU `xdisp.c` defines these as nil, buffer-local display variables.
    obarray.set_symbol_value("wrap-prefix", Value::NIL);
    obarray.set_symbol_value("line-prefix", Value::NIL);
    obarray.set_symbol_value("selective-display", Value::NIL);
    obarray.set_symbol_value("selective-display-ellipses", Value::T);
    obarray.set_symbol_value("indicate-empty-lines", Value::NIL);
    obarray.set_symbol_value("indicate-buffer-boundaries", Value::NIL);
    obarray.set_symbol_value("fringe-indicator-alist", Value::NIL);
    obarray.set_symbol_value("fringe-cursor-alist", Value::NIL);
    obarray.set_symbol_value("scroll-up-aggressively", Value::NIL);
    obarray.set_symbol_value("scroll-down-aggressively", Value::NIL);
    obarray.set_symbol_value("auto-fill-function", Value::NIL);
    obarray.set_symbol_value("buffer-display-count", Value::fixnum(0));
    obarray.set_symbol_value("buffer-display-time", Value::NIL);
}

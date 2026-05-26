//! Windows bootstrap symbols.
//!
//! GNU Emacs registers the MS-Windows Lisp surface from `syms_of_w32*`
//! functions before `lisp/loadup.el` loads `term/w32-win.el`,
//! `term/w32-nt.el`, and `w32-fns.el`.  Keep the same shape here:
//! platform variables are registered only for Windows builds, while the Lisp
//! files can remain direct GNU sources.

use super::intern::intern;
use super::symbol::Obarray;
use super::value::Value;

pub(crate) fn register_bootstrap_symbols(obarray: &mut Obarray) {
    register_w32term_symbols(obarray);
    register_w32proc_symbols(obarray);
    register_w32fns_symbols(obarray);
    register_w32font_symbols(obarray);
    register_w32dwrite_symbols(obarray);
    register_w32console_symbols(obarray);
    register_windows_dynamic_library_versions(obarray);
}

fn register_w32term_symbols(obarray: &mut Obarray) {
    for name in [
        "vendor-specific-keysyms",
        "added",
        "removed",
        "modified",
        "renamed-from",
        "renamed-to",
        "application",
        "winlogo",
        "x-use-underline-position-properties",
        "x-underline-at-descent-line",
    ] {
        defsym(obarray, name);
    }

    defvar_lisp(obarray, "x-wait-for-event-timeout", Value::make_float(0.1));
    defvar_int(obarray, "w32-num-mouse-buttons", 2);
    defvar_lisp(obarray, "w32-swap-mouse-buttons", Value::NIL);
    defvar_lisp(obarray, "w32-grab-focus-on-raise", Value::T);
    defvar_lisp(obarray, "w32-capslock-is-shiftlock", Value::NIL);
    defvar_lisp(obarray, "w32-recognize-altgr", Value::T);
    defvar_bool(obarray, "w32-use-visible-system-caret", false);
    defvar_bool(obarray, "x-use-underline-position-properties", false);
    defvar_bool(obarray, "x-underline-at-descent-line", false);
    defvar_lisp(obarray, "x-toolkit-scroll-bars", Value::T);
    defvar_bool(obarray, "w32-unicode-filenames", true);
    defvar_bool(obarray, "w32-use-native-image-API", false);
    defvar_bool(obarray, "w32-yes-no-dialog-show-cancel", true);
    defvar_bool(obarray, "w32-add-wrapped-menu-bar-lines", true);
}

fn register_w32proc_symbols(obarray: &mut Obarray) {
    for name in ["high", "low", "cygwin", "msys", "w32-native"] {
        defsym(obarray, name);
    }

    defvar_lisp(obarray, "w32-quote-process-args", Value::T);
    defvar_lisp(obarray, "w32-start-process-show-window", Value::NIL);
    defvar_lisp(obarray, "w32-start-process-share-console", Value::NIL);
    defvar_lisp(obarray, "w32-start-process-inherit-error-mode", Value::T);
    defvar_int(obarray, "w32-pipe-read-delay", 0);
    defvar_int(obarray, "w32-pipe-buffer-size", 0);
    defvar_lisp(obarray, "w32-downcase-file-names", Value::NIL);
    defvar_lisp(
        obarray,
        "w32-get-true-file-attributes",
        Value::symbol("local"),
    );
    defvar_lisp(obarray, "w32-collate-ignore-punctuation", Value::NIL);
}

fn register_w32fns_symbols(obarray: &mut Obarray) {
    for name in [
        "undefined-color",
        "cancel-timer",
        "hyper",
        "super",
        "meta",
        "alt",
        "ctrl",
        "control",
        "shift",
        "font-parameter",
        "geometry",
        "workarea",
        "mm-size",
        "frames",
        "tip-frame",
        "assq-delete-all",
        "unicode-sip",
        "read-file-name-internal",
        ":icon",
        ":tip",
        ":level",
        "info",
        "warning",
        ":title",
        ":body",
        "HKCR",
        "HKCU",
        "HKLM",
        "HKU",
        "HKCC",
        "gnutls",
        "libxml2",
        "serif",
        "zlib",
        "lcms2",
        "json",
        "not-useful",
        "pseudo-color",
        "static-gray",
        "static-color",
        "true-color",
        "asterisk",
        "exclamation",
        "question",
        "ok",
        "silent",
        "data-directory",
        "run-at-time",
        "x-hide-tip",
        "capslock",
        "kp-numlock",
        "scroll",
        "informational",
        "critical",
    ] {
        defsym(obarray, name);
    }

    obarray
        .put_property(
            "undefined-color",
            "error-conditions",
            Value::list(vec![
                Value::symbol("undefined-color"),
                Value::symbol("error"),
            ]),
        )
        .expect("undefined-color error-conditions property must be valid");
    obarray
        .put_property(
            "undefined-color",
            "error-message",
            Value::string("Undefined color"),
        )
        .expect("undefined-color error-message property must be valid");

    defvar_lisp(obarray, "w32-color-map", Value::NIL);
    defvar_lisp(obarray, "w32-pass-alt-to-system", Value::NIL);
    defvar_lisp(obarray, "w32-alt-is-meta", Value::T);
    defvar_int(obarray, "w32-quit-key", 0);
    defvar_lisp(obarray, "w32-pass-lwindow-to-system", Value::T);
    defvar_lisp(obarray, "w32-pass-rwindow-to-system", Value::T);
    defvar_lisp(obarray, "w32-phantom-key-code", Value::fixnum(255));
    defvar_lisp(obarray, "w32-enable-num-lock", Value::T);
    defvar_lisp(obarray, "w32-enable-caps-lock", Value::T);
    defvar_lisp(obarray, "w32-scroll-lock-modifier", Value::NIL);
    defvar_lisp(obarray, "w32-lwindow-modifier", Value::NIL);
    defvar_lisp(obarray, "w32-rwindow-modifier", Value::NIL);
    defvar_lisp(obarray, "w32-apps-modifier", Value::NIL);
    defvar_bool(obarray, "w32-enable-synthesized-fonts", false);
    defvar_lisp(obarray, "w32-enable-palette", Value::T);
    defvar_int(
        obarray,
        "w32-mouse-button-tolerance",
        w32_mouse_button_tolerance_default(),
    );
    defvar_int(obarray, "w32-mouse-move-interval", 0);
    defvar_bool(obarray, "w32-pass-extra-mouse-buttons-to-system", false);
    defvar_bool(obarray, "w32-pass-multimedia-buttons-to-system", true);
    defvar_lisp(obarray, "x-cursor-fore-pixel", Value::NIL);
    defvar_lisp(obarray, "x-max-tooltip-size", Value::NIL);
    defvar_lisp(obarray, "x-no-window-manager", Value::NIL);
    defvar_lisp(obarray, "x-pixel-size-width-font-regexp", Value::NIL);
    defvar_bool(obarray, "w32-strict-painting", true);
    defvar_bool(obarray, "w32-use-fallback-wm-chars-method", false);
    defvar_bool(obarray, "w32-disable-new-uniscribe-apis", false);
    defvar_lisp(obarray, "w32-tooltip-extra-pixels", Value::T);
    defvar_bool(obarray, "w32-disable-abort-dialog", false);
    defvar_bool(obarray, "w32-ignore-modifiers-on-IME-input", true);
    defvar_int(obarray, "w32-ansi-code-page", w32_ansi_code_page());
    defvar_int(
        obarray,
        "w32-multibyte-code-page",
        w32_multibyte_code_page(),
    );
    defvar_bool(obarray, "w32-disable-double-buffering", false);
    defvar_bool(obarray, "w32-follow-system-dark-mode", true);
}

fn register_w32font_symbols(obarray: &mut Obarray) {
    defvar_lisp(obarray, "w32-charset-info-alist", Value::NIL);
    for name in [
        "w32-charset-ansi",
        "w32-charset-symbol",
        "w32-charset-default",
        "w32-charset-shiftjis",
        "w32-charset-hangeul",
        "w32-charset-chinesebig5",
        "w32-charset-gb2312",
        "w32-charset-oem",
        "w32-charset-johab",
        "w32-charset-easteurope",
        "w32-charset-turkish",
        "w32-charset-baltic",
        "w32-charset-russian",
        "w32-charset-arabic",
        "w32-charset-greek",
        "w32-charset-hebrew",
        "w32-charset-vietnamese",
        "w32-charset-thai",
        "w32-charset-mac",
        "w32-non-USB-fonts",
    ] {
        defsym(obarray, name);
    }
}

fn register_w32dwrite_symbols(obarray: &mut Obarray) {
    defvar_bool(obarray, "w32-inhibit-dwrite", false);
}

fn register_w32console_symbols(obarray: &mut Obarray) {
    defvar_bool(obarray, "w32-use-full-screen-buffer", false);
}

fn register_windows_dynamic_library_versions(obarray: &mut Obarray) {
    defvar_int(obarray, "libpng-version", -1);
    defvar_int(obarray, "libgif-version", -1);
    defvar_int(obarray, "libjpeg-version", -1);
    defvar_int(obarray, "libgnutls-version", -1);
    defvar_int(
        obarray,
        "tree-sitter--library-abi",
        tree_sitter::LANGUAGE_VERSION as i64,
    );
}

fn defsym(obarray: &mut Obarray, name: &str) {
    let id = intern(name);
    obarray.ensure_interned_global_id(id);
}

fn defvar_lisp(obarray: &mut Obarray, name: &str, value: Value) {
    obarray.set_symbol_value(name, value);
    obarray.make_special(name);
}

fn defvar_int(obarray: &mut Obarray, name: &str, value: i64) {
    defvar_lisp(obarray, name, Value::fixnum(value));
}

fn defvar_bool(obarray: &mut Obarray, name: &str, value: bool) {
    defvar_lisp(obarray, name, if value { Value::T } else { Value::NIL });
}

#[cfg(windows)]
fn w32_mouse_button_tolerance_default() -> i64 {
    unsafe { windows_sys::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime() as i64 / 2 }
}

#[cfg(not(windows))]
fn w32_mouse_button_tolerance_default() -> i64 {
    250
}

#[cfg(windows)]
fn w32_ansi_code_page() -> i64 {
    unsafe { windows_sys::Win32::Globalization::GetACP() as i64 }
}

#[cfg(not(windows))]
fn w32_ansi_code_page() -> i64 {
    0
}

#[cfg(windows)]
fn w32_multibyte_code_page() -> i64 {
    unsafe extern "C" {
        fn _getmbcp() -> std::ffi::c_int;
    }

    unsafe { _getmbcp() as i64 }
}

#[cfg(not(windows))]
fn w32_multibyte_code_page() -> i64 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_symbol_registration_matches_gnu_bootstrap_shape() {
        let mut obarray = Obarray::new();
        register_bootstrap_symbols(&mut obarray);

        assert_eq!(
            obarray.symbol_value("w32-quote-process-args").copied(),
            Some(Value::T)
        );
        assert_eq!(
            obarray
                .symbol_value("w32-get-true-file-attributes")
                .copied(),
            Some(Value::symbol("local"))
        );
        assert_eq!(
            obarray.symbol_value("x-toolkit-scroll-bars").copied(),
            Some(Value::T)
        );
        assert_eq!(
            obarray.symbol_value("tree-sitter--library-abi").copied(),
            Some(Value::fixnum(tree_sitter::LANGUAGE_VERSION as i64))
        );
        assert!(obarray.is_special("w32-quote-process-args"));
        assert!(obarray.is_special("tree-sitter--library-abi"));
        assert!(obarray.intern_soft("w32-charset-shiftjis").is_some());
    }

    #[test]
    fn undefined_color_has_gnu_error_properties() {
        let mut obarray = Obarray::new();
        register_bootstrap_symbols(&mut obarray);

        assert_eq!(
            obarray.get_property("undefined-color", "error-message"),
            Some(Value::string("Undefined color"))
        );
        assert_eq!(
            obarray.get_property("undefined-color", "error-conditions"),
            Some(Value::list(vec![
                Value::symbol("undefined-color"),
                Value::symbol("error"),
            ]))
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn non_windows_context_does_not_leak_w32_surface() {
        let eval = super::super::eval::Context::new();
        let features = eval
            .obarray()
            .symbol_value("features")
            .copied()
            .expect("features should be bound");
        let feature_list =
            super::super::value::list_to_vec(&features).expect("features should be a list");

        assert!(
            eval.obarray()
                .intern_soft("w32-quote-process-args")
                .is_none()
        );
        assert!(!feature_list.contains(&Value::symbol("w32")));
    }
}

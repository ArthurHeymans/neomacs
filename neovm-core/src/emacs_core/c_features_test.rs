//! Standing check for the `Fprovide` half of a build option.
//!
//! Ledger 189 measured what one `#ifdef` decides in GNU -- "the C variable
//! surface, the `Fprovide`, and which `loadup.el` branch runs" -- for the
//! window system.  Ledger 190 measured the subr surface.  This file owns the
//! `Fprovide` itself, for **every** feature GNU's `src/*.c` provides, because
//! `(featurep 'X)` is what GNU's own Lisp asks to decide whether a capability
//! is there, and a `t` this build cannot back is worse than an error: the
//! caller believes it.
//!
//! GNU's `lisp/net/tramp-gvfs.el:123` is the shape:
//!
//! ```elisp
//! (defconst tramp-gvfs-enabled
//!   (ignore-errors
//!     (and (featurep 'dbusbind)
//!          (tramp-compat-funcall 'dbus-get-unique-name :session)
//!          ...)))
//! ```
//!
//! -- the same "`featurep`/`fboundp` as the build test" ledger 190 found at
//! `lisp/t-mouse.el:49`, and `lisp/net/dbus.el` repeats it at seven sites as
//! `(or (featurep 'dbusbind) (signal 'dbus-error (list "Emacs not compiled
//! with dbus support")))`.  A build that answers `t` there does not gain a
//! capability; it loses GNU's own detection AND GNU's own honest error.
//!
//! Ledger 192.

use crate::test_utils::runtime_startup_eval_one;

/// GNU's `syms_of_dbusbind` is the whole of `src/dbusbind.c` behind one
/// `#ifdef HAVE_DBUS` (`src/dbusbind.c:21`, `:2179`), called from
/// `src/emacs.c:2477-2479` behind the same one.  A build without the option
/// therefore has none of the six subrs, none of the nine `DEFVAR`s, no
/// `dbus-error` conditions (`src/dbusbind.c:2011-2016`) and none of
/// `keyboard.c`'s four `#ifdef HAVE_DBUS` sites.
///
/// This port links no libdbus and has no D-Bus transport, so it is in that
/// configuration and every one of those answers must be the absent one.
#[test]
fn without_a_dbus_transport_the_whole_dbusbind_surface_is_absent() {
    crate::test_utils::init_test_tracing();
    let result = runtime_startup_eval_one(
        "(list
           (featurep 'dbusbind)
           (mapcar #'fboundp '(dbus--init-bus dbus-get-unique-name
                               dbus-message-internal dbus--fd-open
                               dbus--fd-close dbus--registered-fds))
           (mapcar #'boundp '(dbus-compiled-version dbus-runtime-version
                              dbus-message-type-invalid
                              dbus-message-type-method-call
                              dbus-message-type-method-return
                              dbus-message-type-error
                              dbus-message-type-signal
                              dbus-registered-objects-table
                              dbus-debug))
           (get 'dbus-error 'error-conditions)
           (lookup-key special-event-map [dbus-event])
           (memq 'dbus-event while-no-input-ignore-events))",
    );
    assert_eq!(
        result,
        "OK (nil (nil nil nil nil nil nil) \
         (nil nil nil nil nil nil nil nil nil) nil nil nil)",
        "GNU without HAVE_DBUS declares none of this; a value invented here is \
         believed by every `(featurep 'dbusbind)' caller in GNU's own Lisp"
    );
}

/// The anti-vacuity half: the same probe run against features this build
/// really does have must answer `t`, or the test above is passing because
/// `runtime_startup_eval_one` returned nothing useful.
#[test]
fn the_features_this_build_really_has_still_answer_t() {
    crate::test_utils::init_test_tracing();
    let result = runtime_startup_eval_one(
        "(list (featurep 'emacs)
               (featurep 'multi-tty)
               (featurep 'make-network-process)
               (featurep 'tty-child-frames)
               (and (featurep 'threads) (fboundp 'make-thread))
               (and (featurep 'inotify) (fboundp 'inotify-add-watch)))",
    );
    assert_eq!(result, "OK (t t t t t t)");
}

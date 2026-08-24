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

use crate::emacs_core::c_features::{GnuGuard, HereDecision, gnu_c_features};
use crate::test_utils::runtime_startup_eval_one;

/// Every `Fprovide` GNU's `src/*.c` makes, and the seed, sorted.
///
/// Measured on GNU 31.0.90 (mirror `0ee48ac4df2`) with
///
/// ```text
/// grep -rhn 'Fprovide (' src/*.c | sed 's/.*Fprovide (//;s/,.*//' | sort -u
/// ```
///
/// -- 33 call sites, 26 distinct names -- plus `emacs`, which is not an
/// `Fprovide` at all but `Vfeatures = list1 (Qemacs)` at `src/fns.c:6820`.
/// A name here with no row in [`gnu_c_features`] is a feature nobody decided
/// about, which is the hole ledger 192 found `dbusbind` sitting in.
const GNU_C_PROVIDES: &[&str] = &[
    "android",
    "cairo",
    "dbusbind",
    "dynamic-setting",
    "emacs",
    "font-render-setting",
    "gfilenotify",
    "gtk",
    "haiku",
    "inotify",
    "kqueue",
    "lcms2",
    "make-network-process",
    "motif",
    "move-toolbar",
    "multi-tty",
    "native-compile",
    "pgtk",
    "system-font-setting",
    "threads",
    "tty-child-frames",
    "w32",
    "w32notify",
    "x",
    "x-toolkit",
    "xinput2",
    "xwidget-internal",
];

/// The table has a row for every one of them, and no row for anything else.
///
/// This is the sweep ledger 192 was asked for, kept: a feature GNU provides
/// and this table has no opinion about cannot be audited, and a row for a name
/// GNU never provides is this port inventing a feature.
#[test]
fn the_table_covers_exactly_the_features_gnus_c_provides() {
    crate::test_utils::init_test_tracing();
    let mut rows: Vec<&str> = gnu_c_features().iter().map(|f| f.name).collect();
    rows.sort_unstable();
    assert_eq!(rows, GNU_C_PROVIDES);
}

/// No feature is advertised without something behind it.
///
/// GNU cannot advertise a capability it did not build, because one `#ifdef`
/// compiles both the `Fprovide` and the implementation it stands for.  Here the
/// two are separate pieces of Rust, so the rule is a test: a provided row must
/// carry a citation of the code that answers for it.
#[test]
fn every_provided_feature_names_what_backs_it() {
    crate::test_utils::init_test_tracing();
    let mut provided = 0usize;
    for row in gnu_c_features() {
        if !row.here.provided() {
            continue;
        }
        provided += 1;
        match row.here {
            HereDecision::UnconditionalInGnu => assert_eq!(
                row.gnu_guard,
                GnuGuard::Unconditional,
                "{} is provided here on the grounds that GNU provides it in \
                 every build, but its own row says GNU guards it",
                row.name
            ),
            HereDecision::Implemented { by } => assert!(
                by.len() > 20 && by.contains(".rs"),
                "{} claims an implementation but does not cite one: {by:?}",
                row.name
            ),
            HereDecision::DetectedAtBuildTime { cfg, .. } => assert!(
                cfg.contains("build.rs"),
                "{} claims a build-time probe but does not cite one: {cfg:?}",
                row.name
            ),
            HereDecision::NotBuilt { .. } => unreachable!("filtered above"),
        }
    }
    assert!(
        provided >= 5,
        "only {provided} features provided; the filter is eating rows"
    );
}

/// Every row this build does NOT provide says why, and the reason is about a
/// capability rather than about the list.
#[test]
fn every_absent_feature_says_why() {
    crate::test_utils::init_test_tracing();
    for row in gnu_c_features() {
        if let HereDecision::NotBuilt { because } = row.here {
            assert!(
                because.len() > 30,
                "{} is absent with no reason worth reading: {because:?}",
                row.name
            );
        }
    }
}

/// Every row cites a `file:line` in GNU's `src/`.
#[test]
fn every_row_cites_gnus_own_site() {
    crate::test_utils::init_test_tracing();
    for row in gnu_c_features() {
        assert!(
            row.gnu_site.starts_with("src/") && row.gnu_site.contains(".c:"),
            "{} cites {:?}, which is not a src/*.c line",
            row.name,
            row.gnu_site
        );
    }
}

/// `dbusbind` is absent, and the row says the reason is the missing transport.
///
/// The regression pin for ledger 192: putting the name back requires editing
/// this row, and the only variants that provide it demand a citation.
#[test]
fn dbusbind_is_absent_and_its_row_names_the_missing_transport() {
    crate::test_utils::init_test_tracing();
    let row = gnu_c_features()
        .into_iter()
        .find(|f| f.name == "dbusbind")
        .expect("the table has a dbusbind row");
    assert_eq!(row.gnu_guard, GnuGuard::BuildOption("HAVE_DBUS"));
    assert!(!row.here.provided());
    let HereDecision::NotBuilt { because } = row.here else {
        panic!("dbusbind is provided again: {:?}", row.here);
    };
    assert!(because.contains("no D-Bus transport"), "{because:?}");
}

/// The derived list is exactly the one this build had before ledger 192, less
/// `dbusbind`, in GNU's own order.
///
/// `features` reads newest-provided first (`src/fns.c:3751` conses), so the
/// order is a fact about `main`'s `syms_of_*` sequence and is observable from
/// Lisp.  This pins that the table's row order reproduces it.
#[test]
fn the_derived_list_keeps_gnus_relative_order() {
    crate::test_utils::init_test_tracing();
    let names = crate::emacs_core::c_features::initial_feature_names();
    let mut expected = vec!["threads"];
    if cfg!(target_os = "linux") {
        expected.push("inotify");
    }
    if cfg!(neomacs_have_lcms2) {
        expected.push("lcms2");
    }
    expected.extend([
        "multi-tty",
        "make-network-process",
        "tty-child-frames",
        "emacs",
    ]);
    assert_eq!(names, expected);
}

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

//! The `lisp/cus-start.el` names whose C declaration belongs to a platform,
//! and whether GNU binds each one in a build like this one.
//!
//! `cus-start.el` lists every variable GNU's C layer can define, across all the
//! window systems and operating systems GNU builds for.  When one is not bound
//! it consults a `native-p` test -- `dos-` needs `(eq system-type 'ms-dos)`,
//! `ns-` needs `(featurep 'ns)`, `imagemagick` needs
//! `(fboundp 'imagemagick-types)`, and so on -- and only signals "built-in
//! variable `%S' not bound" when that test says this build should have had it
//! (`lisp/cus-start.el:893-951`).  So a build without MS-DOS support does not
//! need `dos-hyper-key` to exist; it needs it to NOT exist.
//!
//! Neomacs used to seed all of these to nil in one loop, which is the wrong
//! shape twice over.  Binding a name GNU leaves unbound is invented existence:
//! measured under GNU Emacs 31.0.90 on GNU/Linux, `-Q --batch`,
//! `(boundp 'dos-hyper-key)` is `nil` and `(boundp 'imagemagick-render-type)`
//! is `nil`, while Neomacs answered `t` to both.  And a name GNU DOES bind
//! here is not a stub at all -- it has a real declaration with a real default,
//! and belongs wherever its `syms_of_*` counterpart is registered.
//!
//! [`GnuBinding`] is that measurement, as a required field: a row cannot be
//! added without answering "does GNU bind this in a build like this one?", and
//! only the `BoundHere` rows are seeded.  The `UnboundHere` rows stay in the
//! table because deleting them would lose the answer -- the next author would
//! see `cus-start.el` mention `ns-antialias-text` and seed it again.

use super::symbol::Obarray;
use super::value::Value;

/// Whether GNU Emacs binds a `cus-start.el` platform variable in a build like
/// this one -- GNU/Linux, X, GTK, no MS-DOS, no NS, no Haiku, no w32, no
/// xwidgets, no ImageMagick.
///
/// Measured name by name under GNU Emacs 31.0.90, `-Q --batch`, not derived
/// from the `#ifdef`s: 7 of these 32 are bound, 25 are not.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum GnuBinding {
    /// GNU binds it here.  The nil seed keeps `cus-start.el` quiet during
    /// loadup for the ones Neomacs has no real declaration for yet; where
    /// Neomacs does declare it, that declaration is what the variable ends up
    /// holding, because it runs after this table.
    BoundHere,
    /// GNU leaves it unbound here, because the `DEFVAR` sits in a file this
    /// build does not compile.  Seeding it would make `boundp` disagree with
    /// GNU, and `cus-start.el`'s `native-p` test is what keeps the absence
    /// from erroring.
    UnboundHere,
}

/// One `cus-start.el` platform name.
#[derive(Copy, Clone, Debug)]
pub struct CusStartPlatformVariable {
    pub name: &'static str,
    pub gnu: GnuBinding,
}

const fn bound(name: &'static str) -> CusStartPlatformVariable {
    CusStartPlatformVariable {
        name,
        gnu: GnuBinding::BoundHere,
    }
}

const fn unbound(name: &'static str) -> CusStartPlatformVariable {
    CusStartPlatformVariable {
        name,
        gnu: GnuBinding::UnboundHere,
    }
}

/// The trailing comment on each row is the GNU C file the `DEFVAR` is in, and
/// for the bound ones the default measured under GNU.
pub static CUS_START_PLATFORM_VARIABLES: &[CusStartPlatformVariable] = &[
    // ---- Bound under GNU/Linux + X + GTK (7) ----
    bound("window-combination-limit"),  // window.c, => window-size
    bound("void-text-area-pointer"),    // xdisp.c, => arrow
    bound("x-bitmap-file-path"),        // xfns.c, => ("/usr/include/X11/bitmaps")
    bound("x-gtk-use-system-tooltips"), // gtkutil.c/x-win.el alias, => t
    bound("x-scroll-event-delta-factor"), // xterm.c, => 1.0
    bound("x-auto-preserve-selections"), // xselect.c, => (CLIPBOARD PRIMARY)
    bound("vertical-centering-font-regexp"), // fontset.c, => a regexp
    // ---- Unbound under GNU/Linux (25) ----
    // src/image.c, guarded by HAVE_IMAGEMAGICK; `cus-start.el' asks
    // `(fboundp 'imagemagick-types)'.
    unbound("imagemagick-render-type"),
    // GNU never declares a VARIABLE called `xwidget-internal' at all -- it is
    // the feature name `syms_of_xwidget' provides -- and `cus-start.el' probes
    // it with `boundp', so the answer is nil in every GNU build.
    // `xwidget-webkit-disable-javascript' is deliberately not a row here:
    // Neomacs ships a real xwidget layer (`xwidget.rs'), which is a build
    // difference from this GNU rather than an invented seed, so that module
    // declares it alongside `xwidget-list' and `xwidget-view-list'.
    unbound("xwidget-internal"),
    // src/nsterm.m, src/nsfns.m -- `(featurep 'ns)'.
    unbound("ns-control-modifier"),
    unbound("ns-right-control-modifier"),
    unbound("ns-command-modifier"),
    unbound("ns-right-command-modifier"),
    unbound("ns-alternate-modifier"),
    unbound("ns-right-alternate-modifier"),
    unbound("ns-function-modifier"),
    unbound("ns-antialias-text"),
    unbound("ns-auto-hide-menu-bar"),
    unbound("ns-confirm-quit"),
    unbound("ns-use-native-fullscreen"),
    unbound("ns-use-fullscreen-animation"),
    unbound("ns-use-srgb-colorspace"),
    unbound("ns-scroll-event-delta-factor"),
    unbound("ns-click-through"),
    // src/w32*.c -- `(eq system-type 'windows-nt)'.
    unbound("w32-follow-system-dark-mode"),
    // src/msdos.c, src/dosfns.c -- `(eq system-type 'ms-dos)'.
    unbound("dos-display-scancodes"),
    unbound("dos-hyper-key"),
    unbound("dos-super-key"),
    unbound("dos-keypad-mode"),
    unbound("dos-unsupported-char-glyph"),
    // src/haikuterm.c, src/haikufns.c -- `(featurep 'haiku)'.
    unbound("haiku-debug-on-fatal-error"),
    unbound("haiku-use-system-tooltips"),
];

/// Seed the names GNU binds here, and only those.
pub fn register_bootstrap_vars(obarray: &mut Obarray) {
    for var in CUS_START_PLATFORM_VARIABLES {
        if var.gnu == GnuBinding::BoundHere {
            obarray.set_symbol_value(var.name, Value::NIL);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Measured under GNU Emacs 31.0.90 on GNU/Linux, `-Q --batch`: of the 32
    /// platform names Neomacs used to seed, GNU binds 7 and leaves 25 unbound.
    #[test]
    fn table_matches_gnu_measurement() {
        assert_eq!(CUS_START_PLATFORM_VARIABLES.len(), 32);
        assert_eq!(
            CUS_START_PLATFORM_VARIABLES
                .iter()
                .filter(|v| v.gnu == GnuBinding::BoundHere)
                .count(),
            7
        );
    }

    #[test]
    fn table_has_no_duplicate_rows() {
        let mut names: Vec<&str> = CUS_START_PLATFORM_VARIABLES
            .iter()
            .map(|v| v.name)
            .collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before, "duplicate cus-start platform row");
    }
}

//! Standing check: which Rust subrs does the preloaded Lisp shadow?
//!
//! A name registered by `defsubr` and then `defun`ed again by a file
//! `loadup.el` preloads is a Rust reimplementation of Lisp we already ship.
//! The subr never answers once the `.el` is loaded, so it drifts silently --
//! DIVERGENCES.md 131 (`start-process`, "drifted twice") and 146
//! (`primitive-undo`, whose `(t . MODTIME)` arm was wrong in both directions)
//! are the two this project has paid for.  The standing rule is "don't reimplement elisp
//! in Rust -- load the .el", and this test is that rule made checkable.
//!
//! GNU has exactly ONE such shadow, and it announces itself:
//! `Sframe_windows_min_size` (src/frame.c:494-502) is prefixed
//! "Placeholder used by temacs -nw before window.el is loaded" and returns a
//! constant 0.  Every other name below has no C implementation in GNU at all
//! -- `grep 'DEFUN ("NAME"' src/*.c` finds nothing -- so ours is invention.
//!
//! Adding to this list is not forbidden, but it must be deliberate: a new
//! entry means a Rust function nothing will ever call, and it must be
//! justified in the entry's comment the way GNU justifies its one.
//!
//! `bootstrap_kill_ring_commands_are_not_rust_subrs`
//! (`neovm-core/src/emacs_core/kill_ring_test.rs:46`) is the same check
//! written out by hand for one area; this one is that check with the name
//! list replaced by a scan, so a name nobody thought to list still trips it.

use crate::emacs_core::eval::lookup_global_subr_entry;
use crate::emacs_core::intern::intern;
use crate::emacs_core::value::{ValueKind, VecLikeType};

/// Registered Rust subrs whose function cell, after `loadup.el`, holds
/// something other than a subr.
///
/// GNU source for each name was checked with `grep 'DEFUN ("NAME"' src/*.c`
/// against emacs-mirror 31.0.90 (0ee48ac4df2).
const SHADOWED_BY_PRELOADED_LISP: &[&str] = &[
    // -- GNU ships a C placeholder here ON PURPOSE, and window.el overrides
    // it (src/frame.c:494-502).  This is the only justified entry.
    "frame-windows-min-size",
    // -- Window and frame geometry.  GNU has none of these in C; they are
    // window.el / frame.el / faces.el functions built on the primitives that
    // ARE in C (`window-edges' on `window-pixel-edges' and so on).
    "balance-windows",              // lisp/window.el:6222
    "color-defined-p",              // lisp/faces.el:1923
    "color-values",                 // lisp/faces.el:1940
    "delete-other-windows",         // lisp/window.el:4453
    "delete-window",                // lisp/window.el:4318
    "display-buffer",               // lisp/window.el:8166
    "display-color-cells",          // lisp/frame.el:2966
    "enlarge-window",               // lisp/window.el:3714
    "fit-window-to-buffer",         // lisp/window.el:10307
    "make-frame",                   // lisp/frame.el:1019
    "pop-to-buffer",                // lisp/window.el:9403
    "select-frame-set-input-focus", // lisp/frame.el:1262
    "shrink-window",                // lisp/window.el:3759
    "switch-to-buffer",             // lisp/window.el:9558
    "window-absolute-pixel-edges",  // lisp/window.el:3937
    "window-edges",                 // lisp/window.el:3839
    "window-pixel-edges",           // lisp/window.el:3922
    "window-tree",                  // lisp/window.el:3999
    // -- Type predicates.  One-line `defun's over primitives that are in C.
    "booleanp",          // lisp/subr.el:4775
    "char-uppercase-p",  // lisp/simple.el:6683
    "integer-or-null-p", // lisp/subr.el:4809
    "list-of-strings-p", // lisp/subr.el:4768
    "macrop",            // lisp/subr.el:4793
    "string-or-null-p",  // lisp/subr.el:4762
    // -- Names GNU creates with `defalias', so the shadowing cell is a
    // SYMBOL, not a function: `symbol-function' answers differently here
    // whichever way round it is.
    "move-marker", // lisp/subr.el:2280 -> set-marker
    "not",         // lisp/subr.el:71   -> null
    "string<",     // lisp/subr.el:2278 -> string-lessp
    "string=",     // lisp/subr.el:2277 -> string-equal
    "string>",     // lisp/subr.el:2279 -> string-greaterp
    // -- Process launchers.  All four are thin Lisp wrappers over
    // `make-process', which IS in C (src/process.c:1767).  DIVERGENCES.md 131
    // and 146: the Rust ones answer only in unit tests.
    "start-file-process",               // lisp/simple.el:5249
    "start-file-process-shell-command", // lisp/subr.el:5076
    "start-process",                    // lisp/subr.el:3466
    "start-process-shell-command",      // lisp/subr.el:5063
    // -- Undo.  `syms_of_undo' (src/undo.c:423-490) has exactly one `defsubr',
    // `&Sundo_boundary' (:435).  `primitive-undo' was deleted for this reason
    // (DIVERGENCES.md 146); these two are the same class, not yet done.
    "buffer-disable-undo", // lisp/simple.el:3591
    "undo",                // lisp/simple.el:3466
    // -- Everything else.
    "emacs-repository-get-branch",   // lisp/version.el:231
    "emacs-repository-get-version",  // lisp/version.el:183
    "global-set-key",                // lisp/subr.el:1545
    "ignore",                        // lisp/subr.el:501
    "local-set-key",                 // lisp/subr.el:1569
    "make-auto-save-file-name",      // lisp/files.el:7699
    "memory-limit",                  // lisp/subr.el:3574
    "read-number",                   // lisp/subr.el:3725
    "set-buffer-file-coding-system", // lisp/international/mule.el:1302
    "string-greaterp",               // lisp/subr.el:6283
    "string-match-p",                // lisp/subr.el:5941 (a `defsubst')
    "symbol-file",                   // lisp/subr.el:3351
    "transient-mark-mode",           // lisp/simple.el:7614 (`define-minor-mode')
];

#[test]
fn rust_subrs_shadowed_by_preloaded_lisp_match_the_reviewed_list() {
    crate::test_utils::init_test_tracing();

    let eval = crate::test_utils::runtime_startup_context();
    let mut names: Vec<String> = eval
        .obarray
        .all_symbols()
        .into_iter()
        .map(str::to_string)
        .collect();
    names.sort();
    names.dedup();

    let mut shadowed: Vec<String> = Vec::new();
    for name in names {
        // Registered as a Rust subr...
        if lookup_global_subr_entry(intern(&name)).is_none() {
            continue;
        }
        // ...but the loaded runtime's function cell is no longer that subr.
        let Some(cell) = eval.obarray.symbol_function(&name) else {
            continue;
        };
        if !matches!(
            cell.kind(),
            ValueKind::Subr(_) | ValueKind::Veclike(VecLikeType::Subr)
        ) {
            shadowed.push(name);
        }
    }

    let mut expected: Vec<String> = SHADOWED_BY_PRELOADED_LISP
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    expected.sort();

    assert_eq!(
        shadowed, expected,
        "\nRust subrs shadowed by preloaded Lisp changed.\n\
         A NEW name means a Rust reimplementation of a `.el' we ship: prefer \
         deleting the subr (see DIVERGENCES.md 146).  A name that DISAPPEARED \
         means the subr was deleted or the `.el' stopped defining it -- update \
         the list either way.\n",
    );

    // `primitive-undo' is the deletion DIVERGENCES.md 146 records: GNU has no
    // C version, so neither do we, and the name must not come back.
    assert!(
        lookup_global_subr_entry(intern("primitive-undo")).is_none(),
        "primitive-undo must have no Rust subr: GNU implements it in \
         lisp/simple.el:3645 and nowhere in src/",
    );
}

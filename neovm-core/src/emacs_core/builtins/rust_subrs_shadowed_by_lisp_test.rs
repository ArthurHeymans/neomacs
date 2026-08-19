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
                                    // -- The six type predicates and the five `defalias' names that used to
                                    // sit here are GONE: their Rust subrs were deleted (DIVERGENCES.md 148),
                                    // so the count fell from 49 to 38.  See
                                    // `lisp_only_predicates_and_aliases_test.rs' for the per-name statement.
                                    // -- The four process launchers that used to sit here are GONE: their
                                    // Rust subrs were deleted (DIVERGENCES.md 149).  All four are Lisp over
                                    // `make-process', which IS in C (src/process.c:1767); see
                                    // `process_launchers_are_lisp_only_test.rs' for the per-name statement.
                                    // -- Undo: NOTHING is left here.  `syms_of_undo' (src/undo.c:423-490) has
                                    // exactly one `defsubr', `&Sundo_boundary' (:435), and we register that
                                    // one and no more.  `primitive-undo' went in DIVERGENCES.md 146; `undo'
                                    // and `buffer-disable-undo' went in 150, which also deleted the third
                                    // replay loop the `undo' subr reached (`BufferManager::undo_buffer').
                                    // `buffer-enable-undo' is NOT in that group and never was: GNU DEFUNs it
                                    // at src/buffer.c:1829, so it is a subr here too and does not appear on
                                    // this list.  See `lisp_only_undo_commands_test.rs'.
                                    // -- "Everything else": the thirteen names from six files that were left
                                    // when the groups with a theme had been taken.  They are GONE
                                    // (DIVERGENCES.md 152), so the count fell from 32 to 19.  Two of them had
                                    // a C NEIGHBOUR that had to survive the deletion and does:
                                    // `string-match-p' is a `defsubst' over `string-match' (src/search.c:442),
                                    // and `transient-mark-mode' the COMMAND is `define-minor-mode' while
                                    // `transient-mark-mode' the VARIABLE is DEFVAR_LISP (src/buffer.c:5835).
                                    // See `lisp_only_misc_names_test.rs' for the per-name statement.
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

    // The names GNU implements in Lisp and nowhere in src/, deleted by
    // DIVERGENCES.md 146, 150 and 152.  None may come back.
    for (name, gnu_source) in [
        ("primitive-undo", "lisp/simple.el:3645"),
        ("undo", "lisp/simple.el:3466"),
        ("buffer-disable-undo", "lisp/simple.el:3591"),
        ("emacs-repository-get-branch", "lisp/version.el:231"),
        ("emacs-repository-get-version", "lisp/version.el:183"),
        ("global-set-key", "lisp/subr.el:1545"),
        ("ignore", "lisp/subr.el:501"),
        ("local-set-key", "lisp/subr.el:1569"),
        ("make-auto-save-file-name", "lisp/files.el:7699"),
        ("memory-limit", "lisp/subr.el:3574"),
        ("read-number", "lisp/subr.el:3725"),
        (
            "set-buffer-file-coding-system",
            "lisp/international/mule.el:1302",
        ),
        ("string-greaterp", "lisp/subr.el:6283"),
        ("string-match-p", "lisp/subr.el:5941"),
        ("symbol-file", "lisp/subr.el:3351"),
        ("transient-mark-mode", "lisp/simple.el:7614"),
    ] {
        assert!(
            lookup_global_subr_entry(intern(name)).is_none(),
            "{name} must have no Rust subr: GNU implements it in {gnu_source} \
             and nowhere in src/",
        );
    }
    // ...and the ones GNU DOES implement in C, right next to them, which must
    // stay registered: deleting the pair above is not a licence to delete
    // these.
    for (name, gnu_source) in [
        ("undo-boundary", "src/undo.c:251"),
        ("buffer-enable-undo", "src/buffer.c:1829"),
        // 152's near misses: the C primitive each deleted Lisp name is
        // written over, or the C half of a split name.
        ("string-match", "src/search.c:442"),
        ("string-lessp", "src/fns.c:557"),
        ("define-key", "src/keymap.c"),
        ("current-global-map", "src/keymap.c"),
        ("current-local-map", "src/keymap.c"),
        ("do-auto-save", "src/fileio.c"),
        ("process-attributes", "src/process.c"),
        ("read-from-minibuffer", "src/minibuf.c"),
    ] {
        assert!(
            lookup_global_subr_entry(intern(name)).is_some(),
            "{name} IS a C DEFUN in GNU ({gnu_source}) and must stay a subr",
        );
    }

    // `transient-mark-mode' is the split name 152 had to be careful with: the
    // COMMAND is lisp/simple.el:7614 and is gone, but the VARIABLE is
    // DEFVAR_LISP at src/buffer.c:5835 and must still be bound.
    assert!(
        eval.obarray.symbol_value("transient-mark-mode").is_some(),
        "transient-mark-mode the VARIABLE is DEFVAR_LISP in GNU \
         (src/buffer.c:5835); deleting the Lisp COMMAND must not remove it",
    );

    // The list must shrink, not just stay reviewed.  This pins the arithmetic
    // so a re-added subr cannot be absorbed by editing the list alone.
    assert_eq!(
        SHADOWED_BY_PRELOADED_LISP.len(),
        19,
        "the reviewed shadow list is 19 names after DIVERGENCES.md 152 \
         (50 before 146, 49 after it, 38 after 148, 34 after 149, 32 after \
         150, 19 after 152).  What is left is one justified C placeholder \
         (`frame-windows-min-size') and the eighteen window/frame/face names.",
    );
}

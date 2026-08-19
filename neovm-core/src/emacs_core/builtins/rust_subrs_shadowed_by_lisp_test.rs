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
//! After DIVERGENCES.md 154 the list is two names long: GNU's own placeholder,
//! and `display-color-cells', which is a DEBT -- our `faces.el' load reaches it
//! before `frame.el' defines it, which GNU's cannot.  Adding to the list is not
//! forbidden, but it must be deliberate, so `ShadowJustification' makes an
//! entry say WHICH of those two things it is and cite the evidence: a
//! placeholder cites the GNU `src/' line and the `.el' that overrides it, a debt
//! cites the `.el' that owns the name and the caller that has to move first.  A
//! name cannot be parked here with no justification, which is how the list
//! reached fifty.
//!
//! `bootstrap_kill_ring_commands_are_not_rust_subrs`
//! (`neovm-core/src/emacs_core/kill_ring_test.rs:46`) is the same check
//! written out by hand for one area; this one is that check with the name
//! list replaced by a scan, so a name nobody thought to list still trips it.

use crate::emacs_core::eval::lookup_global_subr_entry;
use crate::emacs_core::intern::intern;
use crate::emacs_core::value::{ValueKind, VecLikeType};

/// Why a name is allowed to sit on the shadow list.
///
/// The two reasons are different in KIND, and keeping them apart is the point
/// of the type.  GNU has exactly one shadow and it is a deliberate placeholder
/// with a comment saying so; anything else on this list is a Rust function
/// nothing will ever call, kept only because something reaches the name before
/// its `.el` loads -- which is a bug with a location, not a design.  A bare
/// `&[&str]` could not tell those apart, and that is how the list reached fifty.
enum ShadowJustification {
    /// GNU ships the same C placeholder ON PURPOSE, and says why in `src/`.
    GnuShipsTheSamePlaceholder {
        /// The GNU `src/` line that registers the placeholder, and its reason.
        gnu_c_placeholder: &'static str,
        /// The `.el` line that overrides it, in GNU and here.
        gnu_lisp_override: &'static str,
    },
    /// NOT justified.  GNU has no C version at all, so GNU's bootstrap cannot
    /// reach this name before its `.el` loads -- and ours does.  The entry is a
    /// debt: it records the Rust-side caller that has to move before the subr
    /// can be deleted like the rest of its group.
    UnjustifiedBootstrapCaller {
        /// The `.el` line that owns the name in GNU.
        gnu_lisp_definition: &'static str,
        /// The bootstrap-window caller that keeps the subr alive, and the
        /// measurement that found it.
        why_it_cannot_go_yet: &'static str,
    },
}

/// A registered Rust subr whose function cell, after `loadup.el`, holds
/// something other than a subr.
struct ReviewedShadow {
    /// The name whose function cell the preloads overwrite.
    name: &'static str,
    /// Why it is still here.
    justification: ShadowJustification,
}

/// Registered Rust subrs whose function cell, after `loadup.el`, holds
/// something other than a subr.
///
/// GNU source for each name was checked with `grep 'DEFUN ("NAME"' src/*.c`
/// against emacs-mirror 31.0.90 (0ee48ac4df2).
const SHADOWED_BY_PRELOADED_LISP: &[ReviewedShadow] = &[
    // The one justified entry: GNU ships this C placeholder ON PURPOSE and
    // window.el overrides it.
    ReviewedShadow {
        name: "frame-windows-min-size",
        justification: ShadowJustification::GnuShipsTheSamePlaceholder {
            gnu_c_placeholder: "src/frame.c:494-502, prefixed \"Placeholder used \
                 by temacs -nw before window.el is loaded\", returns a constant 0",
            gnu_lisp_override: "lisp/window.el:1899 (`frame-windows-min-size')",
        },
    },
    // The one DEBT: eighteenth of DIVERGENCES.md 154's group, and the only one
    // of the eighteen that could not go.
    ReviewedShadow {
        name: "display-color-cells",
        justification: ShadowJustification::UnjustifiedBootstrapCaller {
            gnu_lisp_definition: "lisp/frame.el:2966 -- and `loadup.el' loads \
                 `frame' at :255, ninety-five files after `faces' at :160, so \
                 the name is VOID in GNU while faces.el loads.  GNU bootstraps, \
                 which proves GNU's faces.el load never asks for it.",
            why_it_cannot_go_yet: "our faces.el load DOES ask: \
                 `custom-declare-face show-paren-match' (lisp/faces.el:3161) -> \
                 face-spec-set -> face-spec-recalc -> face-spec-choose -> \
                 face-spec-set-match-display on the clause \
                 `((background dark) (min-colors 4))' -> `display-color-cells' \
                 (lisp/faces.el:1588).  The `background' conjunct matches only \
                 because `ensure_selected_frame_id_in_state_with_policy' \
                 (window_cmds/mod.rs) seeds `background-mode' = `dark', which \
                 GNU computes later in `frame-set-background-mode'.  Deleting \
                 the subr costs 1124 tests -- every test that boots a runtime. \
                 See `display_color_cells_is_the_one_that_could_not_go_yet' in \
                 lisp_only_window_frame_names_test.rs.",
        },
    },
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
    // -- SEVENTEEN of the eighteen window/frame/face geometry names are GONE
    // (DIVERGENCES.md 154), so the count fell from 19 to 2.  146 ranked them
    // last and riskiest because the display stack is downstream; measured, all
    // eighteen already answered from window.el, frame.el and faces.el, and each
    // one's C neighbour -- from `delete-window-internal' to `xw-color-values'
    // to `x-create-frame' -- stays registered.  The eighteenth,
    // `display-color-cells', is the entry above.  See
    // `lisp_only_window_frame_names_test.rs' for the per-name statement.
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
        .map(|entry| entry.name.to_string())
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
        // DIVERGENCES.md 154's eighteen.
        ("balance-windows", "lisp/window.el:6222"),
        ("color-defined-p", "lisp/faces.el:1923"),
        ("color-values", "lisp/faces.el:1940"),
        ("delete-other-windows", "lisp/window.el:4453"),
        ("delete-window", "lisp/window.el:4318"),
        ("display-buffer", "lisp/window.el:8166"),
        ("enlarge-window", "lisp/window.el:3714"),
        ("fit-window-to-buffer", "lisp/window.el:10307"),
        ("make-frame", "lisp/frame.el:1019"),
        ("pop-to-buffer", "lisp/window.el:9403"),
        ("select-frame-set-input-focus", "lisp/frame.el:1262"),
        ("shrink-window", "lisp/window.el:3759"),
        ("switch-to-buffer", "lisp/window.el:9558"),
        ("window-absolute-pixel-edges", "lisp/window.el:3937"),
        ("window-edges", "lisp/window.el:3839"),
        ("window-pixel-edges", "lisp/window.el:3922"),
        ("window-tree", "lisp/window.el:3999"),
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
        // 154's near misses: the C primitive each deleted window/frame/face
        // name is written over.  Six of them are one character away from the
        // Lisp name that went.
        ("delete-window-internal", "src/window.c:5684"),
        ("delete-other-windows-internal", "src/window.c:3463"),
        ("window-pixel-left", "src/window.c:1001"),
        ("window-body-width", "src/window.c:1140"),
        ("window-resize-apply", "src/window.c:4957"),
        ("frame-root-window", "src/window.c:350"),
        ("set-window-buffer", "src/window.c:4428"),
        ("select-window", "src/window.c:616"),
        ("xw-color-defined-p", "src/xfns.c:5581"),
        ("xw-color-values", "src/xfns.c:5597"),
        ("x-display-color-cells", "src/xfns.c:5714"),
        ("tty-display-color-cells", "src/term.c:2226"),
        ("x-create-frame", "src/xfns.c:4916"),
        ("make-terminal-frame", "src/frame.c:1736"),
        ("select-frame", "src/frame.c:2097"),
        ("raise-frame", "src/frame.c:3667"),
        ("x-focus-frame", "src/frame.c:3756"),
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
        2,
        "the reviewed shadow list is TWO names after DIVERGENCES.md 154 \
         (50 before 146, 49 after it, 38 after 148, 34 after 149, 32 after \
         150, 19 after 152, 2 after 154).  One is the C placeholder GNU itself \
         ships (`frame-windows-min-size', src/frame.c:494-502); the other is \
         `display-color-cells', a DEBT with a named cause.  A third entry \
         means a Rust reimplementation of a `.el' we ship -- delete the subr \
         instead.",
    );

    // Exactly one entry may be the justified kind, and it is GNU's.
    let justified = SHADOWED_BY_PRELOADED_LISP
        .iter()
        .filter(|entry| {
            matches!(
                entry.justification,
                ShadowJustification::GnuShipsTheSamePlaceholder { .. }
            )
        })
        .count();
    assert_eq!(
        justified, 1,
        "GNU has exactly one deliberate shadow, so at most one entry here may \
         claim `GnuShipsTheSamePlaceholder'",
    );

    // ...and every entry must carry its citations.  This is what the enum
    // exists for: a name cannot be parked here with no justification, which is
    // how the list reached fifty, and a debt cannot be filed as a design.
    for entry in SHADOWED_BY_PRELOADED_LISP {
        match &entry.justification {
            ShadowJustification::GnuShipsTheSamePlaceholder {
                gnu_c_placeholder,
                gnu_lisp_override,
            } => {
                assert!(
                    gnu_c_placeholder.contains("src/"),
                    "{}: a justified entry must name the GNU src/ line that \
                     ships the same C placeholder, the way src/frame.c:494-502 \
                     does",
                    entry.name,
                );
                assert!(
                    gnu_lisp_override.contains("lisp/"),
                    "{}: a justified entry must name the .el line that \
                     overrides the placeholder",
                    entry.name,
                );
            }
            ShadowJustification::UnjustifiedBootstrapCaller {
                gnu_lisp_definition,
                why_it_cannot_go_yet,
            } => {
                assert!(
                    gnu_lisp_definition.contains("lisp/"),
                    "{}: a debt entry must name the .el line GNU implements it \
                     in",
                    entry.name,
                );
                assert!(
                    why_it_cannot_go_yet.contains(".rs") || why_it_cannot_go_yet.contains(".el"),
                    "{}: a debt entry must name the caller that keeps the subr \
                     alive, by file",
                    entry.name,
                );
            }
        }
    }
}

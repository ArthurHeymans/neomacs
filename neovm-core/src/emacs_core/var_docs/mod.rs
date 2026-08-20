//! Documentation strings for built-in (DEFVAR_*) variables.
//!
//! Phase A7-A10 of the substitute-command-keys-audit-v5 R5 plan.
//! Companion to `subr_docs/' (which holds DEFUN docs). Each entry
//! is a `(name, doc)' pair lifted verbatim from a GNU Emacs
//! `DEFVAR_LISP("name", Vsymbol, doc: /* TEXT */)' block (or
//! DEFVAR_INT/BOOL/KBOARD/PER_BUFFER variant) in `src/*.c'.
//!
//! ## Architecture
//!
//! - `gnu_table.rs' is **auto-generated** by
//!   `scripts/extract_gnu_defvar_docs.py' from upstream GNU's
//!   `src/*.c'. To refresh, run the script against an updated GNU
//!   mirror.
//! - `lookup(name)' does a linear scan over the table. Lookups
//!   happen only on `(documentation-property 'foo
//!   'variable-documentation)' queries, which are user-initiated
//!   and rare. Linear scan is fine; ~820 entries today.
//!
//! ## Why grave-quoted strings (not curly)
//!
//! Same reason as `subr_docs/': GNU's `DEFVAR_* doc:' text uses
//! ASCII grave accents (`` ` `` and `'`). `substitute-command-keys'
//! converts them to curly quotes at display time per the user's
//! `text-quoting-style'. Pre-substituting here would lock in
//! `'curve' regardless of preference.
//!
//! ## Lookup precedence
//!
//! `documentation_property_plan' consults sources in this order:
//!   1. Symbol's `variable-documentation' plist property (set by
//!      Lisp `defvar' or by `Snarf-documentation' in GNU's case)
//!   2. `STARTUP_VARIABLE_DOC_STUBS' / `_STRING_PROPERTIES' (the
//!      legacy hand-typed tables, shrinking in Phase A10)
//!   3. `var_docs::lookup(name)' (this module — covers all
//!      upstream GNU DEFVAR_* variables)
//!   4. nil (no doc available)

pub(crate) mod gnu_table;

/// GNU's `SKIP` marker is not documentation, and this is where that becomes
/// unrepresentable rather than merely absent.
///
/// A variable that several window-system files declare keeps its text in one
/// of them and a placeholder in the rest -- `x-pointer-shape` is `DEFVAR_LISP`
/// in `src/xfns.c:10327`, `src/w32fns.c:11809`, `src/haikufns.c:3284` and
/// `src/androidfns.c:3587`, three of which read
/// `doc: /* SKIP: real doc in xfns.c.  */` -- so the string is maintained once.
/// 170 `DEFVAR` blocks across GNU's `src/*.c` carry it, and
/// `Fsnarf_documentation` refuses every one:
///
/// ```text
/// /* Ignore docs that start with SKIP.  These mark
///    placeholders where the real doc is elsewhere.  */
/// if ((!NILP (Fboundp (sym)) || !NILP (Fmemq (sym, delayed_init)))
///     && strncmp (end, "\nSKIP", 5))
///   Fput (sym, Qvariable_documentation, make_fixnum (pos + end + 1 - buf));
/// ```
///
/// (`src/doc.c:600-608`.)  So no GNU build shows one to a user.
/// [`gnu_table`] is generated from ALL of `src/*.c` and used to keep the
/// alphabetically first copy of a duplicated name, which handed 35 variables a
/// placeholder instead of their text; the generator now drops a `SKIP` block
/// so the next file's real copy wins.  The check below is what keeps that
/// true: a regenerated table carrying a placeholder does not compile.
const fn doc_is_a_skip_placeholder(doc: &str) -> bool {
    let bytes = doc.as_bytes();
    bytes.len() >= 4 && bytes[0] == b'S' && bytes[1] == b'K' && bytes[2] == b'I' && bytes[3] == b'P'
}

const _: () = {
    let mut index = 0;
    while index < gnu_table::GNU_VAR_DOCS.len() {
        assert!(
            !doc_is_a_skip_placeholder(gnu_table::GNU_VAR_DOCS[index].1),
            "GNU_VAR_DOCS holds a SKIP placeholder; GNU never installs one \
             (src/doc.c:600-608). Re-run scripts/extract_gnu_defvar_docs.py."
        );
        index += 1;
    }
};

/// Look up the doc string for a built-in variable by name.
/// Returns `None` if no entry exists. The returned `&'static str`
/// points into `.rodata`.
///
/// O(n) linear scan over `gnu_table::GNU_VAR_DOCS`. Called only on
/// documentation-query paths, never from `eval`/`funcall`/dispatch.
#[inline]
pub(crate) fn lookup(name: &str) -> Option<&'static str> {
    // `gnu_table' is generated from ALL of GNU's `src/*.c', the platform files
    // included, but a `DEFVAR_*' attaches the doc string and binds the symbol
    // in the same statement -- so a variable whose C file this build does not
    // compile has no `variable-documentation' either.  Measured under GNU
    // 31.0.90, `-Q --batch': `(documentation-property 'dos-hyper-key
    // 'variable-documentation)' is nil, and so is
    // `(boundp 'dos-hyper-key)'.  The one table that already answers "does any
    // build reachable from here declare this name" answers this too.
    if crate::emacs_core::cus_start_platform_vars::is_name_gnu_leaves_unbound_here(name) {
        return None;
    }
    gnu_table::GNU_VAR_DOCS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, doc)| *doc)
}

#!/usr/bin/env python3
"""Extract GNU Emacs DEFVAR doc: text into a Rust source file.

Walks every `.c` file in GNU Emacs's `src/` directory looking for
DEFVAR declarations of the form:

    DEFVAR_LISP ("name", Vsymbol,
                 doc: /* DOCSTRING TEXT
    POSSIBLY MULTI-LINE */);

    DEFVAR_INT ("name", c_var,
                doc: /* DOCSTRING */);

    DEFVAR_BOOL ("name", c_var,
                 doc: /* DOCSTRING */);

    DEFVAR_KBOARD ("name", kboard_field,
                   doc: /* DOCSTRING */);

For each match, emits a `(name, doc)` tuple to a Rust file:

    pub(crate) static GNU_VAR_DOCS: &[(&str, &str)] = &[
        ("name", "DOCSTRING TEXT\\nPOSSIBLY MULTI-LINE"),
        ...
    ];

Output is sorted alphabetically by name. Unlike DEFUN docs, DEFVAR
docs do NOT get a `(fn ARGS)` suffix appended.

Usage:
    scripts/extract_gnu_defvar_docs.py \\
        --gnu-src /path/to/emacs-mirror/emacs/src \\
        --output  neovm-core/src/emacs_core/var_docs/gnu_table.rs
"""

import argparse
import os
import re
import sys
from pathlib import Path


# Match the start of a DEFVAR_* declaration.  DEFVAR_PER_BUFFER can use
# expressions such as `&BVAR (current_buffer, fill_column)' for the C storage
# argument, so don't try to parse the argument list here.  We only need the
# Lisp variable name plus a bounded search for the following `doc: /* ... */`.
DEFVAR_HEAD = re.compile(
    r'\bDEFVAR_(LISP|BOOL|INT|KBOARD|LISP_NOPRO|PER_BUFFER)\s*\(\s*"([^"]+)"\s*,'
)

# `make-docfile' skips ALL whitespace between the `doc' keyword's colon and the
# opening `/*', newlines included:
#
#     if (c == ':')
#       {
#         doc_keyword = true;
#         do
#           c = getc (infile);
#         while (c_isspace (c));
#       }
#     bool comment = c == '/' && (c = getc (infile)) == '*';
#
# (`lib-src/make-docfile.c:1148-1157'.)  Searching for the literal `"doc: /*"'
# instead silently dropped nine `DEFVAR' blocks from the generated table:
# `inhibit-message' (`src/xdisp.c:38216', two spaces),
# `xft-font-ascent-descent-override' (`src/xftfont.c:800', two spaces),
# `macroexp--dynvars' (`src/lread.c:5931', three spaces), and the six
# `treesit-*' names whose `/*' is on the NEXT line (`src/treesit.c:5355'
# onward).  A dropped row is not a visible failure -- the name simply has no
# documentation -- which is why this went unnoticed until entry 173 diffed the
# generated table's name set against `src/*.c'.
DOC_MARKER = re.compile(r"doc:\s*/\*")


def find_doc_block(text: str, start: int) -> tuple[str | None, int]:
    """Find the `doc: /* ... */)` block starting at or after `start`.
    Returns (doc_text, end_offset) or (None, start) on failure.
    """
    match = DOC_MARKER.search(text, start)
    if match is None:
        return None, start
    doc_marker = match.start()
    # Bound the search: don't cross another DEFVAR boundary.
    next_defvar = text.find("DEFVAR_", start + 1)
    if next_defvar != -1 and next_defvar < doc_marker:
        return None, start
    body_start = match.end()
    body_end = text.find("*/", body_start)
    if body_end == -1:
        return None, start
    body = unescape_doc_comment(text[body_start:body_end])
    # Strip leading/trailing single space (matches make-docfile.c).
    if body.startswith(" "):
        body = body[1:]
    if body.endswith(" "):
        body = body[:-1]
    return body.rstrip(), body_end + 2


def unescape_doc_comment(text: str) -> str:
    r"""Mirror GNU make-docfile's `read_c_string_or_comment` escape handling.

    For `doc: /* ... */` comments, make-docfile consumes a backslash and emits
    the following character verbatim except for `\n`, `\t`, and escaped
    newlines.  In particular, C source `\\[command]` becomes Emacs doc text
    `\[command]` so `substitute-command-keys` can replace it.
    """
    out: list[str] = []
    i = 0
    while i < len(text):
        ch = text[i]
        i += 1
        if ch != "\\" or i >= len(text):
            out.append(ch)
            continue

        escaped = text[i]
        i += 1
        if escaped in "\n\r":
            continue
        if escaped == "n":
            out.append("\n")
        elif escaped == "t":
            out.append("\t")
        else:
            out.append(escaped)
    return "".join(out)


def is_skip_placeholder(doc: str) -> bool:
    """GNU's own test, verbatim: a DOC entry that starts with `SKIP` is not
    documentation.

    A variable that several window-system files declare -- `x-pointer-shape`
    is in `xfns.c`, `w32fns.c`, `haikufns.c` and `androidfns.c` -- carries the
    real text in exactly one of them and
    `doc: /* SKIP: real doc in xfns.c.  */` in the rest, so the string never
    has to be maintained four times.  `Fsnarf_documentation` refuses to install
    such an entry -- `strncmp (end, "\\nSKIP", 5)` guarding the
    `Fput (sym, Qvariable_documentation, ...)`, under the comment "Ignore docs
    that start with SKIP.  These mark placeholders where the real doc is
    elsewhere." (`src/doc.c:600-608`).  So no GNU build ever shows one to a
    user, and a table row holding one is a generator bug rather than a value.

    170 of GNU's `src/*.c` DEFVAR blocks carry the marker.
    """
    return doc.startswith("SKIP")


def extract_defvars(src: str) -> tuple[list[tuple[str, str]], list[str]]:
    """Extract `(name, doc)` pairs from a single C source file.

    Declarations whose doc text is a `SKIP` placeholder contribute nothing:
    the caller must be free to take the next file's copy, which is where the
    real text lives.

    Also returns the names of `DEFVAR` heads for which no `doc:` block could be
    found at all.  Every `DEFVAR_*` in GNU's `src/*.c` has one -- `make-docfile`
    would otherwise emit no `^_V` record for it -- so a non-empty second element
    means the scanner is out of step with `make-docfile`, which is exactly the
    failure entry 173 found and which is silent in the generated table.
    """
    results = []
    undocumented: list[str] = []
    pos = 0
    while True:
        m = DEFVAR_HEAD.search(src, pos)
        if not m:
            break
        name = m.group(2)
        doc, doc_end = find_doc_block(src, m.end())
        if doc is not None:
            if not is_skip_placeholder(doc):
                results.append((name, doc))
            pos = doc_end
        else:
            undocumented.append(name)
            pos = m.end()
    return results, undocumented


def rust_string_literal(s: str) -> str:
    """Format a Rust string literal that handles `\\`, `"`, and newlines.
    Uses raw string syntax `r#"..."#` when possible to preserve grave
    quotes verbatim, falling back to escaped form if the raw delimiter
    appears in the body."""
    if '"#' not in s and not any(ord(c) < 32 and c != "\n" and c != "\t" for c in s):
        for hashes in ["#", "##", "###"]:
            if f'"{hashes}' not in s:
                return f'r{hashes}"{s}"{hashes}'
    escaped = (
        s.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("\n", "\\n")
        .replace("\t", "\\t")
    )
    return f'"{escaped}"'


def emit_rust(entries: list[tuple[str, str]], output: Path) -> None:
    entries_sorted = sorted(entries, key=lambda kv: kv[0])
    lines = [
        "// AUTO-GENERATED by scripts/extract_gnu_defvar_docs.py — DO NOT EDIT.",
        "//",
        "// Source: GNU Emacs `src/*.c` DEFVAR_{LISP,INT,BOOL,KBOARD,...} doc: text.",
        "// Re-run the extractor against an updated GNU mirror to refresh.",
        "//",
        "// Each entry is `(name, raw_grave_quoted_doc)' lifted verbatim",
        "// from the corresponding DEFVAR block. Strings preserve GNU's",
        "// grave-quote convention so that `substitute-command-keys' can",
        "// convert them per the user's `text-quoting-style' at display",
        "// time.",
        "//",
        "// Variables don't have an `(fn ARGS)' suffix -- only DEFUNs do.",
        "",
        "#[rustfmt::skip]",
        "pub(crate) static GNU_VAR_DOCS: &[(&str, &str)] = &[",
    ]
    for name, doc in entries_sorted:
        name_lit = rust_string_literal(name)
        doc_lit = rust_string_literal(doc)
        lines.append(f"    ({name_lit}, {doc_lit}),")
    lines.append("];")
    lines.append("")
    output.write_text("\n".join(lines))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gnu-src", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    if not args.gnu_src.is_dir():
        print(f"error: {args.gnu_src} is not a directory", file=sys.stderr)
        return 1

    all_entries: list[tuple[str, str]] = []
    first_site: dict[str, tuple[str, str]] = {}
    undocumented_heads: list[str] = []
    conflicts = 0
    for c_file in sorted(args.gnu_src.glob("*.c")):
        try:
            src = c_file.read_text(encoding="utf-8", errors="replace")
        except OSError as e:
            print(f"warning: cannot read {c_file}: {e}", file=sys.stderr)
            continue
        entries, undocumented = extract_defvars(src)
        for name in undocumented:
            undocumented_heads.append(f"{c_file.name}:{name}")
        for name, doc in entries:
            if name in first_site:
                where, kept = first_site[name]
                # After the SKIP filter, a second REAL doc for the same name is
                # the case GNU's convention is meant to prevent -- it keeps the
                # text in one file, e.g. `xterm.c`, and marks every other copy
                # SKIP (`src/doc.c:585-594`).  Where two real copies genuinely
                # differ, which one this build would show depends on which file
                # it compiles, and this table cannot know that; say so out loud
                # rather than picking one silently.
                if doc != kept:
                    conflicts += 1
                    print(
                        f"note: '{name}' has DIFFERING real doc text in "
                        f"{where} and {c_file.name}; keeping {where}",
                        file=sys.stderr,
                    )
                continue
            first_site[name] = (c_file.name, doc)
            all_entries.append((name, doc))

    if undocumented_heads:
        # Refuse to write a table that is quietly smaller than GNU's source.
        # A `DEFVAR_*` whose `doc:` block this scanner cannot find produces no
        # row at all, and a missing row looks exactly like "GNU has no doc for
        # that name" from every direction except this one.
        print(
            f"error: {len(undocumented_heads)} DEFVAR head(s) with no doc: block; "
            f"the scanner is out of step with make-docfile "
            f"(lib-src/make-docfile.c:1148-1157):",
            file=sys.stderr,
        )
        for head in undocumented_heads:
            print(f"  {head}", file=sys.stderr)
        return 1

    args.output.parent.mkdir(parents=True, exist_ok=True)
    emit_rust(all_entries, args.output)
    print(
        f"extracted {len(all_entries)} DEFVAR docs from "
        f"{args.gnu_src} -> {args.output} "
        f"({conflicts} name(s) with differing real text in two files)",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())

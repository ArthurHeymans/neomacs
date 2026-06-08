# Display Unification GNU Oracles

This document records the GNU Emacs references and quick commands used while
unifying Neomacs buffer/chrome display rows. The key point is that GNU's
implementation is not cleanly typed, but it does route buffer and string display
through the same redisplay iterator/glyph-production concepts.

## Source References

Use `/home/exec/Projects/github.com/emacs-mirror/emacs/src/xdisp.c` as the
primary reference.

- `handle_display_prop` handles `display` properties for both string and buffer
  iterator states.
- `get_next_display_element` fills the redisplay iterator with the next source
  element.
- `PRODUCE_GLYPHS` turns the iterator element into glyph row output.
- `display_string` is used by mode-line-like string rendering and calls
  `get_next_display_element` plus `PRODUCE_GLYPHS`.
- `display_tab_bar_line` also calls `get_next_display_element` plus
  `PRODUCE_GLYPHS` for non-toolkit tab-bar rows.

Useful search:

```bash
rg -n "handle_display_prop|display_string|display_mode_element|display_tab_bar_line|get_next_display_element|PRODUCE_GLYPHS" /home/exec/Projects/github.com/emacs-mirror/emacs/src/xdisp.c
```

## Batch Oracles

These commands check stable Lisp-level inputs. Batch mode does not expose the
final GUI/TTY glyph rows, so use these as input/property checks, not as full
visual redisplay checks.

Character width:

```bash
/home/exec/.local/bin/emacs -Q --batch --eval '(prin1 (list (char-width ?A) (char-width ?中) (char-width ?א)))'
```

String text properties:

```bash
/home/exec/.local/bin/emacs -Q --batch --eval '(let ((s (propertize "AB" '"'"'face '"'"'(:foreground "red")))) (prin1 (text-properties-at 1 s)))'
```

Display-space property shape:

```bash
/home/exec/.local/bin/emacs -Q --batch --eval '(let ((s (propertize " " '"'"'display '"'"'(space :align-to 4)))) (prin1 (get-text-property 0 '"'"'display s)))'
```

Mode-line string evaluation preserves propertized strings before redisplay:

```bash
/home/exec/.local/bin/emacs -Q --batch --eval '(let ((mode-line-format (propertize "AB" '"'"'face '"'"'(:foreground "red")))) (prin1 (format-mode-line mode-line-format)))'
```

## Neomacs Characterization Coverage

The first refactor slice adds lock tests for:

- propertized tab-bar row text
- mode-line `display` space expansion
- current tab-line CJK behavior
- current tab-line ZWJ emoji behavior
- current tab-line RTL row normalization
- current buffer-row CJK/emoji behavior

Some of these intentionally document the current split: chrome rows preserve
string faces and some display-space behavior, but do not yet use the main buffer
wide-char and cluster machinery. Later phases should update those tests when the
shared `DisplayRowBuilder` becomes the authoritative path for both sources.

## Worktree Verification Note

Project-local git worktrees do not automatically copy ignored generated Lisp
artifacts or existing bootstrap pdumps from the main checkout. A fresh worktree
can therefore try to bootstrap from a partial source-only runtime and fail before
layout tests execute. For Phase 0 verification, run the layout slice with the
complete runtime root used by main:

```bash
NEOMACS_RUNTIME_ROOT=/home/exec/Projects/github.com/eval-exec/neomacs \
  cargo nextest run -p neomacs-layout-engine \
  -E 'test(~display_row) | test(~display_status_line) | test(~layout_frame_rust)'
```

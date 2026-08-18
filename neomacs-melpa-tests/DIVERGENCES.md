# GNU Emacs / Neomacs divergences surfaced by the MELPA parity suites

Every entry below is a place where a MELPA parity workflow gets a different
result from `emacs` and from `target/release/neomacs`. The workflows are left
**failing on purpose** with GNU Emacs's behaviour recorded as the expectation,
per `tmp/neomacs-melpa-tests-standards.md`: "Do not hide a genuine divergence by
weakening the expectation, adding `#[ignore]`, or bypassing the public route."

Each entry gives a package-free reduction, so a fix can be developed and
verified without installing any MELPA package. Run one with:

```sh
emacs -Q --batch --eval '<form>'
./target/release/neomacs -Q --batch --eval '<form>'
```

**Every reduction below was executed in both editors and reproduces as
written** (checked 2026-07-28 against GNU Emacs 31.0.50 and
`target/release/neomacs`), with two marked exceptions: entry 9 needs two files
on disk rather than one form, and entry 15 is an intermittent segfault not
reduced below its package. The script that runs the rest is
`tmp/verify-divergences.sh`.

Triaging by severity? Read these two first. Entry 47 is **data loss** --
`replace-buffer-contents` cannot be undone, so a user who undoes an LSP edit,
a format-on-save or a `revert-buffer` gets back corrupted text and cannot
recover the original. Entry 15 is a **memory fault**, not a behavioural
difference.

Reproduce a failing suite with:

```sh
RUST_LOG=warn NEOMACS_BIN="$PWD/target/release/neomacs" TMPDIR="$PWD/tmp" \
  cargo nextest run -p neomacs-melpa-tests -E 'test(~parity_tests::<pkg>::)' --no-fail-fast
```

---

**The status column is a snapshot, not a record.** Re-run an entry's reduction
in both editors before trusting its status: the 2026-08-05 column was found
STALE on nine of the ten entries it listed as diverging when they were re-run
on 2026-08-08 (see "Verification status 2026-08-09" below). A status here dates
only from the sweep that wrote it.

## Verification status 2026-08-09

The ten entries the 2026-08-05 sweep left as still-diverging were re-run
side by side against GNU Emacs and a fresh `target/release/neomacs`
(evidence and per-entry output: `tmp/p7-reverify.md`). **Nine of the ten no
longer reproduce**: 2, 6, 9, 15, 23, 26, 33, 37 and 42 now give identical
output in both editors, so the 2026-08-05 column was stale for all of them.
Entry 15 (the SIGSEGV) did not reproduce in 44 runs of its package; that is
weaker evidence than a behavioural reduction, because an intermittent memory
fault can be latent rather than gone.

The tenth, **entry 18** (an error inside a process filter or sentinel is
swallowed), was genuinely live and is now FIXED in this tree: filter and
sentinel errors route through the shared command-error reporter with GNU's
context strings, and batch exits 255 with the diagnostic on stderr, matching
GNU byte for byte on all three reductions.

Not re-verified and still the real open signal: the eleven failing parity
suites from 2026-08-05, which have never been mapped to ledger entries. At
least ace_link's help-buffer link offset looks like a divergence this ledger
does not contain.

## The eleven failing suites, mapped 2026-08-09

Each of the eleven was run in isolation and diagnosed. They did **not** collapse
into a few shared causes: with one exception every failure was its own
single-feature gap, and only one was a harness defect.

| suite | verdict |
|---|---|
| auctex_latexmk | already green; nothing was wrong with it |
| ace_link | HARNESS DEFECT, fixed 5dd14bf22 -- it failed on *both* editors, the signature of a stale expectation. Avy label positions were recorded as buffer offsets, and two workflows build a buffer quoting the sandbox root, so every offset past it carried the path length. Now recorded as line and column |
| counsel | entry 49, FIXED 7ac897fee -- `command-history` recorded raw argument values; GNU's `quotify_arg`, `varies` and `fix_command` were all missing |
| dumb_jump | entry 50, FIXED 5f926d612 -- `error-message-string` kept text properties GNU strips everywhere but the `(error STRING)` fast path |
| evil_numbers | entry 51, FIXED b20ec53b9 -- `replace-match` refused any match data recorded as string-sourced, which is what `set-match-data` from plain integers produces |
| vi_tilde_fringe | entry 52, FIXED 392a9f90c -- `define-fringe-bitmap` never registered the symbol in `fringe-bitmaps` |
| rainbow_delimiters | entry 44, FIXED a5ebb0d7a -- the sexp scanner read the `syntax-table` text property raw instead of through GNU's `textget`, so a syntax supplied via `category` (the CC Mode `c-use-category` mechanism) was invisible to it |
| helm_descbinds | entry 45 (`describe-bindings` omits global bindings and the function-key map) |
| swiper | entry 46 (query-replace replaces nothing; not reduced below the package) |
| lsp_mode | entry 47, **DATA LOSS**, FIXED -- `replace-buffer-contents` could not be undone; a zero-length deletion was not recorded, so two insertions coalesced. Reduced to pure ASCII; **not a multibyte bug at all** |
| elisp_slime_nav | entry 48 (an error message loses a text property; not reduced below the package) |

Every genuine divergence has a numbered entry with its reduction, kept after
the fix lands as the regression record.

Two of the fixes were much broader than the suite that caught them.
`replace-match` after `set-match-data` with integers had been failing for
*every* subexpression -- any package that computes bounds itself and then
replaces was affected -- and the `command-history` gap meant no
`repeat-complex-command` entry replayed the way GNU records it. The third,
entry 47, was mis-framed by its own symptom: the suite shows corruption around
an emoji, yet the reduction is pure ASCII and the trigger is the shape of a
diff, so nothing built on `replace-buffer-contents` could be undone. It is now
fixed, in the undo recorder rather than anywhere near the diff.

## Verification status 2026-08-05

All single-form reductions re-run against GNU Emacs and a fresh
target/release/neomacs (commit 601ef8d38 era). **33 of 43 entries no longer
reproduce** -- the two editors now give identical output for their reductions
-- and are presumed fixed by the July/August work (batch macro lifecycle,
TTY separator width, filelock GNU state machine, face pipeline, and friends).
**Still diverging: 2, 6, 9, 18, 23, 26, 33, 37, 42**, plus 15 (the SIGSEGV,
not reducible to one form) which needs a fresh check against its package.
Raw per-entry outputs from the sweep were captured under tmp/divsweep/.

The full parity suite the same day: 600/611 passed; the 11 failing suites
(ace_link, auctex_latexmk, counsel, dumb_jump, elisp_slime_nav, evil_numbers,
helm_descbinds, lsp_mode, rainbow_delimiters, swiper, vi_tilde_fringe) need
mapping to the surviving entries -- at least one (ace_link: help-buffer link
positions offset by 8 with identical help text) looks like a NEW divergence
not in this ledger.

| # | entry | status |
|---|-------|--------|
| 1 | A minibuffer prompt inside a keyboard macro reads stdin | PARITY (stale?) |
| 2 | `read-char` / `read-event` / `read-char-exclusive` ignore the macro | FIXED (2026-08-08, tmp/p7-reverify.md) |
| 3 | `[t]` default keymap bindings are never dispatched | PARITY (stale?) |
| 4 | `(throw 'exit …)` with no catch silently ends batch evaluation | PARITY (stale?) |
| 5 | `write-region` writes `?` for every legacy coding system | PARITY (stale?) |
| 6 | `directory-files` returns undecoded bytes | FIXED (2026-08-08, tmp/p7-reverify.md) |
| 7 | `completing-read` forwards 7 arguments instead of 8 | PARITY (stale?) |
| 8 | `string-search` rejects an explicit nil START | PARITY (stale?) |
| 9 | Chained autoloads stop after one hop | FIXED (2026-08-08, tmp/p7-reverify.md) |
| 10 | Face aliases are not followed | PARITY (stale?) |
| 11 | `completion-in-region-mode-map` is empty | PARITY (stale?) |
| 12 | `x-popup-menu` rejects the documented `POSITION` value `t` | PARITY (stale?) |
| 13 | `buffer-list` differs in order *and* contents | PARITY (stale?) |
| 14 | `delete-other-windows` does not move the surviving window | PARITY (stale?) |
| 15 | SIGSEGV in `%S` printing of a string nested in conses | FIXED? (not reproduced in 44 runs, 2026-08-08 -- intermittent, may be latent) |
| 16 | `real-last-command` is set one command-loop iteration too late | PARITY (stale?) |
| 17 | An undefined face reference is never reported | PARITY (stale?) |
| 18 | An error signalled inside a process filter is swallowed | FIXED (2026-08-09, this tree -- filter/sentinel reporting + batch exit 255) |
| 19 | Word boundaries at a script change are not honoured | PARITY (stale?) |
| 20 | `*Messages*` does not replace a progress line with its "...done" | PARITY (stale?) |
| 21 | A refused connection reports a different error — synchronously only | PARITY (stale?) |
| 22 | `format` reverses the plist of a propertized string used as the FORMAT | PARITY (stale?) |
| 23 | `write-file` leaves a stray lock file behind | FIXED (2026-08-08, tmp/p7-reverify.md) |
| 24 | The `default` face ignores a display-conditional theme setting | PARITY (stale?) |
| 25 | An interpreted lambda's parameter destroys a built-in buffer-local | PARITY (stale?) |
| 26 | No lock file is created for a modified visited buffer | FIXED (2026-08-08, tmp/p7-reverify.md) |
| 27 | `get-buffer-window` does not prefer the selected window | PARITY (stale?) |
| 28 | An error signalled in `pre-command-hook` is not reported | PARITY (stale?) |
| 29 | `function-key-map` holds a malformed translation for keypad digits | PARITY (stale?) |
| 30 | `*Messages*` keeps a `...` progress line that GNU replaces | PARITY (stale?) |
| 31 | `self-insert-command` never expands an abbrev | PARITY (stale?) |
| 32 | Backward `forward-comment` ignores a comment with a two-character ender | PARITY (stale?) |
| 33 | `call-interactively` on a non-interactive autoload never resolves it | FIXED (2026-08-08, tmp/p7-reverify.md) |
| 34 | A regexp using a syntax class does not trigger `syntax-propertize` | PARITY (stale?) |
| 35 | A `:family` set beside a colour on `default` is discarded by GNU only | PARITY (stale?) |
| 36 | Process output does not relocate markers at the process mark | PARITY (stale?) |
| 37 | Killing a windowed buffer leaves the current buffer out of sync | FIXED (2026-08-08, tmp/p7-reverify.md) |
| 38 | A quantifier after the `` \` `` anchor is not treated as literal | PARITY (stale?) |
| 39 | `easy-menu-add-item` drops a submenu that carries any property | PARITY (stale?) |
| 40 | `window-body-width` keeps the tty vertical-bar column | PARITY (stale?) |
| 41 | `ceiling` and friends accept a marker as the divisor | PARITY (stale?) |
| 42 | A tree-sitter font-lock setting without its language slot fails to compile | FIXED (2026-08-08, tmp/p7-reverify.md) |
| 43 | `autoload` does not record its definition in `load-history` | PARITY (stale?) |


---

## 1. A minibuffer prompt inside a keyboard macro reads stdin

GNU's `read_minibuf` (src/minibuf.c) only diverts to `read_minibuf_noninteractive`
when `(noninteractive || daemon) && NILP (Vexecuting_kbd_macro)`. A running macro
keeps the real minibuffer, and its keys come from the macro. Neomacs's
`read_minibuf`-family readers gate only on "is there an input receiver", so batch
always takes the stdin path and the prompt hits EOF.

```elisp
(defalias 'my-cmd (lambda () (interactive) (setq result (read-string "p: "))))
(global-set-key (kbd "C-c t") 'my-cmd)
(execute-kbd-macro (kbd "C-c t z z RET"))
;; GNU     => result is "zz"
;; Neomacs => (end-of-file "Error reading from stdin")
```

`execute-kbd-macro` itself works in both, and `executing-kbd-macro` is non-nil
inside the macro in both.

Affects: `aangit` (6), `abc-mode` (1), `academic-phrases` (1),
`ace-jump-helm-line` (1). Any package that prompts, and every helm session —
helm reads its pattern with `read-from-minibuffer`, so no helm session can be
driven in Neomacs batch at all.

`ac-helm`'s six failures are *triggered* here but do not all land as
`end-of-file`. With stdin at `/dev/null` they arrive as an intermittent core
dump (twice), `End of buffer` (twice), `Quit` (once) and silent termination
(once — that one is divergence 4); with a pipe on stdin the modes shift again.
Treat "helm cannot be driven" as the trigger and read entries 4 and 15 for the
rest.

## 2. `read-char` / `read-event` / `read-char-exclusive` ignore the macro

They return nil **and leave the key in the macro**, so it is then dispatched as
an ordinary command (typically self-inserting). `read-key-sequence` is correct,
and therefore so is `read-key`.

```elisp
(defun probe (character)
  (interactive (list (read-char "Query Char:")))
  (setq result (list character (key-description (this-command-keys)))))
(global-set-key (kbd "C-c t") 'probe)
(execute-kbd-macro (kbd "C-c t x"))
;; GNU     => (120 "C-c t x")
;; Neomacs => (nil "C-c t")
```

Affects: `ace-jump-mode` (7), `ace-jump-zap` (8). Any package prompting with
`read-char` — `ace-jump-char-mode` signals `(wrong-type-argument
number-or-marker-p nil)`, and word mode silently mislabels, jumps unasked, and
types the query char into the buffer.

## 3. `[t]` default keymap bindings are never dispatched

`define-key` stores the binding and `lookup-key` finds it in both editors; only
dispatch skips it, falling through to the global map. Same for
`overriding-local-map`.

```elisp
(defun probe-fallback () (interactive) (setq probe-result 'fallback))
(let ((buffer (generate-new-buffer "*probe*")))
  (set-window-buffer (selected-window) buffer)
  (set-buffer buffer)
  (let ((map (make-sparse-keymap)))
    (define-key map [t] 'probe-fallback)
    (use-local-map map)
    (execute-kbd-macro (kbd "RET"))
    (list (lookup-key map [t]) probe-result (buffer-size))))
;; GNU     => (probe-fallback fallback 0)
;; Neomacs => (probe-fallback nil      1)
```

Affects: `ace-mc` (5). Any catch-all binding used to terminate a key loop —
ace-jump, avy, hydra, transient.

## 4. `(throw 'exit …)` with no catch silently ends batch evaluation

GNU signals `no-catch`. Neomacs unwinds past `condition-case` to an implicit
top-level `exit` catch and exits 0, so the rest of the script never runs. Only
the tag `exit` behaves this way. This both truncates a batch script and reports
success, so it can hide unrelated failures.

```elisp
(progn (princ (format "%S" (condition-case e (throw 'exit 7)
                             (error (list 'caught e)))))
       (princ " then"))
;; GNU     => (caught (no-catch exit 7)) then
;; Neomacs => no output, exit status 0
```

Verified side by side with another tag: `(throw 'my-tag 7)` gives
`(caught (no-catch my-tag 7))` in both editors and execution continues. So this
is not a `throw` or `condition-case` bug — it points at an implicit top-level
`catch 'exit` in Neomacs's batch evaluation.

Found via `helm-exit-minibuffer` in `ace-jump-helm-line`, and confirmed
independently from `ac-helm`, where it is the sole cause of the one workflow
that produces no output at all rather than a mismatch.

## 5. `write-region` writes `?` for every legacy coding system

`encode-coding-string` is correct in both editors; only the file-writing path is
lossy. Saving a Shift_JIS, EUC-JP, GBK or ISO-2022-JP file destroys its content.

```elisp
(with-temp-buffer
  (insert "あい\n")
  (let ((coding-system-for-write 'japanese-shift-jis))
    (write-region (point-min) (point-max) "/tmp/x" nil 'silent)))
;; bytes on disk — GNU => (130 160 130 162 10)   Neomacs => (63 63 10)
```

utf-8 and utf-8-unix are written correctly.

**latin-1 depends on what you write.** For genuine latin-1 *text* it round trips
correctly in both editors — an address book written with
`bookmark-file-coding-system` set to latin-1 keeps its accented names, with real
single latin-1 bytes and no `?` replacements. For **arbitrary binary** the two
editors disagree in the opposite direction from the shift_jis case: GNU performs
a real conversion while Neomacs passes the bytes through untouched.

```elisp
;; payload (unibyte-string 216 205 183 128), written then read back binary
;; japanese-shift-jis   GNU => (216 32 128)        Neomacs => (63 63 63 63)
;; latin-1              GNU => (216 32 128)        Neomacs => (216 205 183 128)
;; binary               byte-exact in both
```

For ciphertext Neomacs's pass-through is arguably the more useful behaviour and
GNU is the reference, but either way **a file written by one editor cannot be
read by the other**. Affects: `aa-edit-mode` (1), `aes` (1).

## 6. `directory-files` returns undecoded bytes

File names are not decoded through `file-name-coding-system`, so a non-ASCII
name comes back as raw bytes and sorts differently.
`directory-files-recursively` inherits this.

```elisp
(directory-files DIR)  ; DIR contains "Lösung.pdf"
;; GNU     => ("." ".." "Lösung.pdf" …)          (multibyte-string-p => t)
;; Neomacs => ("." ".." "L\303\266sung.pdf" …)   (multibyte-string-p => nil)
```

Affects: `abgaben` (1), `asdf-vm` (1).

`asdf-vm` reaches it by a completely different route, which is worth
recording: `asdf-vm-plugin.el:73` lists installed plugins with
`(directory-files plugins-directory t …)`, so a plugin whose directory name
is non-ASCII comes back as raw bytes — `("ruby" "standalone" "資料")` in GNU
against `("ruby" "standalone" "������������")` in Neomacs. The *second*
list in the same value is correct because it is built from a parsed alist
rather than from the filesystem, which is the signature to look for.

## 7. `completing-read` forwards 7 arguments instead of 8

GNU's `Fcompleting_read` always calls `completing-read-function` with all eight
parameters (src/minibuf.c), padding omitted optionals with nil. Neomacs forwards
only what the caller supplied, so a handler declared with the full eight-parameter
arglist signals `wrong-number-of-arguments`.

```elisp
(let ((completing-read-function (lambda (&rest args) (length args))))
  (completing-read "P: " (list "a" "b") nil t nil nil "a"))
;; GNU => 8    Neomacs => 7
```

Affects: `abgaben` (2).

## 8. `string-search` rejects an explicit nil START

GNU's `Fstring_search` treats nil as 0. An omitted optional passed explicitly as
nil must behave as omitted.

```elisp
(string-search " " "a b c" nil)
;; GNU => 1    Neomacs => (wrong-type-argument fixnump nil)
```

Blast radius beyond the package: `info.el`'s `Info-follow-reference` calls it
that way with an uninitialised loop variable, so **following any `*note` cross
reference in Info fails**, with or without any package. Info *menu* items are
unaffected. Affects: `ace-link` (1).

## 9. Chained autoloads stop after one hop

When an `-autoloads.el` re-declares an autoload that another package already
declared for a different file, Neomacs leaves the symbol's function cell pointing
at the inner autoload object and the second hop is never taken.

```elisp
;; chain-autoloads.el contains (autoload 'chain-run "chain" nil t)
;; chain.el defines chain-run
(autoload 'chain-run "chain-autoloads" nil t)
(funcall 'chain-run)
;; GNU     => runs
;; Neomacs => (wrong-type-argument symbolp (autoload "chain" nil t nil))
```

Affects: `abs-mode` (1).

## 10. Face aliases are not followed

`define-obsolete-face-alias` names fail every face operation, because face lookup
does not follow the `face-alias` symbol property.

```elisp
(defface neomacs-probe-real-face '((t :background "red")) "Probe.")
(define-obsolete-face-alias 'neomacs-probe-alias-face 'neomacs-probe-real-face "1.0")
(list (facep 'neomacs-probe-alias-face)
      (get 'neomacs-probe-alias-face 'face-alias)
      (face-attribute 'neomacs-probe-alias-face :background nil t))
;; GNU     => (<face vector>  neomacs-probe-real-face  "red")
;; Neomacs => (nil            neomacs-probe-real-face  (error "Invalid face" …))
```

**The `face-alias` property is set identically in both editors** — so
`define-obsolete-face-alias` is not implicated and the divergence is entirely in
*lookup*. Worth stating, because "aliases are not followed" reads as though the
alias might not be getting created.

`facep` is the cheapest probe: one call, no frame, no defface comparison, and it
returns nil rather than signalling, so it can sit inside a larger report without
a `condition-case`. `face-differs-from-default-p` shows the same thing by
signalling.

Affects: `abridge-diff` (1), via `smerge-refine`'s use of the obsolete
`smerge-refined-change` alias; `amaranth-dark-theme` (1), via flymake's
`flymake-errline` alias. Every `define-obsolete-face-alias` in the Emacs
tree is affected.

## 11. `completion-in-region-mode-map` is empty

GNU populates it with five bindings; Neomacs leaves it empty, so no completion
navigation key works after `completion-at-point`, and the `*Completions*` help
header degrades from "Type M-RET on a completion to select it" to
"Type M-x minibuffer-choose-completion …".

```elisp
(let (out)
  (map-keymap (lambda (k v) (push (cons (key-description (vector k)) v) out))
              completion-in-region-mode-map)
  (nreverse out))
;; GNU     => (("M-<down>" . minibuffer-next-completion) ("M-<up>" . …)
;;             ("RET" menu-item "" minibuffer-choose-completion :filter …)
;;             ("TAB" . completion-at-point) ("ESC" keymap …))
;; Neomacs => nil
```

`minibuffer-local-completion-map` has `M-RET` in both, so only the
completion-in-region map is affected. Affects: `accent` (1).

## 12. `x-popup-menu` rejects the documented `POSITION` value `t`

```elisp
(x-popup-menu t '("Title" ("Pane" ("Item" . value))))
;; GNU     => nil
;; Neomacs => (wrong-type-argument listp t)
```

Affects: `ace-popup-menu` (1).

## 13. `buffer-list` differs in order *and* contents

Two distinct problems in one primitive.

**Order.** GNU and Neomacs return the buffers in a different order, so anything
rendering a buffer menu (bs, ibuffer, ace-jump-buffer) shows a different list,
and any labelling of that list differs with it. Affects: `ace-jump-buffer` (4).

**Contents.** In batch, Neomacs exposes internal echo-area buffers that GNU does
not. Enough to push a "fewer than N buffers open" check past its threshold.

```elisp
(execute-kbd-macro (kbd "C-x C-b"))
(sort (mapcar #'buffer-name (buffer-list)) #'string<)
;; GNU     => (" *Minibuf-0*" " *load*" "*Buffer List*" "*Messages*" "*scratch*")
;; Neomacs => (" *Echo Area 0*" " *Echo Area 1*" " *Minibuf-0*" " *load*"
;;             "*Buffer List*" "*Messages*" "*scratch*")
```

Affects: `achievements`.

## 14. `delete-other-windows` does not move the surviving window

After `C-x 1` from a window that is not at the frame's left edge, GNU relocates
the surviving window to the frame origin and gives it the frame's width; Neomacs
leaves its left edge where it was and grows it, so the right edge ends up past
the frame.

```elisp
(let ((right (split-window-right)))
  (select-window right)
  (delete-other-windows)
  (list (window-edges) (frame-width)))
;; GNU     => ((0 1 80 25) 80)     ; left edge 0, right edge == frame width
;; Neomacs => ((40 0 120 24) 80)   ; left edge still 40, right edge 40 past it
```

Affects: `ace-window` (1).

## 15. SIGSEGV in `%S` printing of a string nested in conses

The most severe entry in this file: a memory fault, not a signal. **It is not a
helm bug** — helm is only how it was reached. Neomacs segfaults inside its own
`format`/`%S` printer while walking a cons tree.

**Signal:** SIGSEGV (11), core dumped. No Rust panic, no `RUST_BACKTRACE`
output, so a genuine memory fault rather than an assertion.

**Rate:** 8/20, 6/20 and 10/20 across three configurations — roughly 30–40%.

**Stack** (`coredumpctl debug`; full trace kept at `tmp/crash/bt.txt`):

```
#0  neovm_core::emacs_core::string_escape::format_lisp_string_bytes_inner_emacs
#1  neovm_core::emacs_core::error::format_string_bytes_in_state
#2  neovm_core::emacs_core::error::format_value_bytes_in_state_with_options
#3  neovm_core::emacs_core::error::format_cons_bytes_in_state
#4..#8   … the same two frames recursing through the cons tree …
#9  …::builtins::strings::builtin_format_message_slice::{{closure}}::{{closure}}
#10 …::builtins::strings::do_format
#12 …::builtins::strings::builtin_format_slice
```

It dies `%S`-printing a **string nested inside conses** — in the harness's own
`(format "RESULT %S" …)`, *after* helm has already failed and returned.
`format_string_bytes_in_state` (`neovm-core/src/emacs_core/error.rs:983`) does
`value.as_lisp_string().expect("checked string")` and hands `ls.as_bytes()` to
`neovm-core/src/emacs_core/string_escape.rs:634`.

**Hypothesis, not confirmed:** a segfault in a leaf string-escape while
recursively walking a cons tree, intermittently, looks like a stale
`LispString` — a `Value` collected or moved during the walk. That matches this
repo's documented hazard that the exact GC does not scan the Rust stack, so a
`Value` held in a Rust local across a safepoint needs `push_specpdl_root`. GC
landing inside the format walk would explain the intermittency exactly.

**Ruled out** (20–25 iterations each, all clean, all under the same keyboard
macro): a bare `helm :sources …`; `with-helm-show-completion` around a helm
session without auto-complete; auto-complete armed and started followed by a
bare helm session; the same followed by a plain `read-from-minibuffer`;
auto-complete armed and started alone; helm driven by `unread-command-events`
instead of a keyboard macro; ac-helm's own
`helm-source-auto-complete-candidates` with plain and with propertized
candidates; and the same plus buffer/helm-buffer cleanup. So neither the
keyboard macro nor `with-helm-show-completion` is required. What the clean
variants share is returning something *small*; the crashing ones `%S` a large
structure containing the captured error object.

**Reductions tried that do NOT reproduce**, recorded so nobody repeats them:
formatting propertized strings in a loop under allocation pressure, and `%S` on
strings holding raw non-UTF-8 bytes (`\310\311`, `\377\376`, lone continuation
bytes, broken UTF-8) — those print byte-identically in both editors.

**Not reduced below the package.** The smallest reproducer is still "run
`ac-complete-with-helm` under a keyboard macro and `%S` the resulting
structure", at 30–40%. Scripts in `tmp/crash/`: `raw.sh` reproduces, `run.sh`
takes probe/iterations/stdin-mode and tallies exit codes.

Affects: `ac-helm` (2 of its 6 failures, when stdin is `/dev/null`).

## 16. `real-last-command` is set one command-loop iteration too late

`last-command` is correct throughout. `real-last-command` is *not* "never
updated" — it is not updated in time for a command to read it. Both editors
agree once the macro returns; they differ only during the command-loop
iteration, which is exactly the window `pre-command-hook` and
`post-command-hook` analytics live in.

```elisp
(defun probe-cmd () (interactive) (setq inside (list last-command real-last-command)))
(global-set-key (kbd "C-c r") 'probe-cmd)
(execute-kbd-macro (kbd "a C-c r"))
;; read inside the command — GNU (self-insert-command self-insert-command)
;;                          NEO (self-insert-command nil)
;; read after the macro    — both (probe-cmd probe-cmd)
```

GNU sets it from the previous iteration's `real-this-command` each time round
the command loop (src/keyboard.c:1354 and 1580).

The same seen from a hook:

```elisp
(add-hook 'pre-command-hook
          (lambda () (push (list this-command real-last-command last-command) events)))
(execute-kbd-macro (kbd "C-x C-b C-h e C-x o"))
;; GNU     => (view-echo-area-messages list-buffers list-buffers)
;;            (other-window view-echo-area-messages view-echo-area-messages)
;; Neomacs => (view-echo-area-messages nil list-buffers)
;;            (other-window nil view-echo-area-messages)
```

Pointer for the fixer: the assignment exists in `command_loop_1`
(`neovm-core/src/emacs_core/eval.rs:6996`) but snapshots `this-command`, which
is already nil at that point, instead of the previous `real-this-command`.

Blast radius beyond the package: keyfreq's `pre-command-hook` records exactly
this variable, so command counting is dead. Anything doing "what did the user
just run" analytics — keyfreq, command-log-mode, repeat.el-style logic — is
affected. Affects: `achievements` (5).

## 17. An undefined face reference is never reported

The measurement agrees; only the diagnostic is missing. GNU logs from
`merge_face_ref` (src/xfaces.c:3022, via `add_to_log`) whenever a face name
cannot be resolved.

```elisp
(let ((buffer (get-buffer-create "*probe*")))
  (set-window-buffer (selected-window) buffer)
  (set-buffer buffer)
  (insert (propertize "value" 'face 'no-such-face-here) "\n")
  (list (window-text-pixel-size)
        (with-current-buffer "*Messages*" (buffer-string))))
;; GNU     => ((5 . 1) "Invalid face reference: no-such-face-here [3 times]\n")
;; Neomacs => ((5 . 1) "")
```

Distinct from entry 10: nothing here is an alias, the face simply does not
exist. Reached through `fit-window-to-buffer`, which measures with
`window-text-pixel-size`. Affects: `ack-menu` (2) — mag-menu draws its argument
values with `'face 'widget-field`, undefined in both editors, and the two
workflows agree on menu text, argv, working directory, results buffer and text
properties, failing on the message list alone.

## 18. An error signalled inside a process filter is swallowed

**FIXED 2026-08-09.** Both editors now print `error in process filter: filter
boom` on stderr and exit 255, leaving `AFTER` unprinted; sentinels match too.
The hole was `command-error-default-function` being a stub, so the reporting
chain ended in nothing. Kept here with its reduction as the regression record.

```elisp
(let ((process (start-process "probe" (get-buffer-create "*out*")
                              "sh" "-c" "printf hello")))
  (set-process-filter process (lambda (_p _o) (error "filter boom")))
  (while (process-live-p process) (accept-process-output process 0.05))
  (accept-process-output nil 0.05))
(princ "AFTER\n")
;; GNU     => "error in process filter: filter boom", exit 255, AFTER not printed
;; Neomacs => no diagnostic at all, AFTER printed, exit 0
```

Both editors drop the filter's output; only GNU reports the failure, fatally in
batch.

**Not assertable by this harness**: batch GNU dies before the oracle can print
its marker, so no parity workflow can express it. It is recorded here and in the
`ack-menu` commit body deliberately — do not "fix" its absence by writing a test.
Surfaced by `ack-menu`, whose SGR parser reads `ansi-color-drop-regexp`, deleted
from ansi-color.el in Emacs 26.1 (commit 35ed01dfb3f), so every search it runs
raises inside the filter on a current Emacs.

## 19. Word boundaries at a script change are not honoured

GNU ends a word where the script changes — kanji/latin, hiragana/katakana,
kana/latin (src/category.c, `word-separating-categories`). Neomacs treats the
whole run as one word, so `\<` and `\b` never match inside mixed-script text and
word motion runs to the end of it.

```elisp
(list (string-match "\\<my" "変数名myVariable")
      (string-match "\\bmy" "変数名myVariable")
      (with-temp-buffer (insert "変数名myVariable") (goto-char (point-min)) (forward-word 1) (point))
      (with-temp-buffer (insert "myVariable変数名") (goto-char (point-min)) (forward-word 1) (point))
      (with-temp-buffer (insert "ひらがなカタカナ") (goto-char (point-min)) (forward-word 1) (point))
      (with-temp-buffer (insert "かなkana")        (goto-char (point-min)) (forward-word 1) (point)))
;; GNU     => (3 3 4 11 5 3)
;; Neomacs => (nil nil 14 14 9 7)
```

Widest blast radius of anything in this file: `M-f`/`M-b`, `forward-word` and
`backward-word`, every `\<`/`\b` regexp, and any word-based motion, kill or
completion in text that mixes scripts — all Japanese editing, and any identifier
embedded in CJK prose.

This is the same root cause as the long-standing `word-boundary` oracle cluster
(~40 tests); the reduction above is its user-facing form.

Affects: `ac-mozc` (1) — `ac-source-ascii-words-in-same-mode-buffers` offers
`("myVariable" "myFunction")` in GNU and only `("myFunction")` in Neomacs,
because `変数名myVariable` is indexed as one word while `ac-mozc-partial-match`
searches for `\<my`.

## 20. `*Messages*` does not replace a progress line with its "...done"

GNU overwrites the previous log entry when the new message is the old one with
`done` appended; Neomacs appends a second line.

```elisp
(message "Working...")
(message "Working...done")
(with-current-buffer "*Messages*" (buffer-string))
;; GNU     => "Working...done\n"
;; Neomacs => "Working...\nWorking...done\n"
```

Affects: `ac-mozc` (1) — mozc.el reports "…Starting mozc-helper-process…" then
"…done". Hits any package using the `"..."` / `"...done"` progress idiom, which
is most of them.

## 21. A refused connection reports a different error — synchronously only

Two differences in one reduction: Neomacs's message is Rust's
`std::io::Error` Display text, which appends the raw errno as
`" (os error 111)"`, where GNU uses the plain `strerror` string; and Neomacs
drops the connection parameters GNU appends to the error data.

```elisp
(condition-case e (make-network-process :name "probe" :host "127.0.0.1" :service 1)
  (error e))
;; GNU     => (file-error "make client process failed" "Connection refused"
;;             :name "probe" :host "127.0.0.1" :service 1)
;; Neomacs => (file-error "make client process failed" "Connection refused (os error 111)")
```

Other errno-derived file errors agree in both editors, so this is specific to
the network-process path rather than to error formatting generally.

**The url transport is not affected, and a suite testing an HTTP package should
not expect a red test here.** `url-retrieve` reaches a closed port through its
own asynchronous connection rather than through a signal out of
`make-network-process`, and the status plist it hands the callback is
byte-identical in both editors — the shared trailing newline in
`"failed with code 111\n"` included:

```elisp
(url-retrieve "http://127.0.0.1:1/probe" (lambda (status) ...) nil t)
;; GNU and Neomacs alike =>
;;   (:error (error connection-failed "failed with code 111\n"
;;            :host "127.0.0.1" :service 1))
```

So the entry is confined to the error object raised by a *synchronous*
`make-network-process`. A package that only ever connects through url will not
witness it. Established by `anaconda-mode`, which closed its listener mid-session
and got matching output in both editors. Affects: `adafruit-wisdom` (1).

## 22. `format` reverses the plist of a propertized string used as the FORMAT

Specific to the propertized string being the **FORMAT** argument. Passing one as
a *value* reverses in both editors, and so does `concat` — those are not
divergences, and a suite hunting this through a package will not find it there.

```elisp
(let ((p (propertize "x" 'alpha 1 'beta 2 'gamma 3)))
  (list (text-properties-at 0 (format p))          ; p as the FORMAT
        (text-properties-at 0 (format "%s-tail" p)) ; p as an argument
        (text-properties-at 0 (concat p "-tail"))))
;; property order in the source is (alpha beta gamma)
;;              as FORMAT        as argument      concat
;; GNU     =>   (alpha beta gamma)  (gamma beta alpha)  (gamma beta alpha)
;; Neomacs =>   (gamma beta alpha)  (gamma beta alpha)  (gamma beta alpha)
```

GNU *preserves* order only in the FORMAT position; everything else reverses in
both.

Reached in practice through any error message built with `format` from a
propertized template — `ac-octave` sees it in inferior-octave's
"No inferior octave process running. Type M-x run-octave", where GNU keeps
`(font-lock-face help-key-binding face help-key-binding)` and Neomacs yields
`(face help-key-binding font-lock-face help-key-binding)`.

Affects: `ac-octave` (1).

## 23. `write-file` leaves a stray lock file behind

The lock survives the save *and* `kill-buffer`, while `buffer-modified-p` is
nil — so Neomacs believes the buffer is clean but the lock stays on disk.

```elisp
(with-current-buffer (get-buffer-create " *scratch-write*")
  (insert "data\n")
  (write-file "<dir>/saved.txt"))
(directory-files "<dir>")
;; GNU     => ("." ".." "saved.txt")
;; Neomacs => ("." ".#saved.txt" ".." "saved.txt")
```

`write-region` does not do this in either editor, and GNU's own lock for a
modified visited buffer is released on save, so it is specific to the
`write-file` path.

Blast radius: `bookmark-save` writes through `write-file`, so a bookmark
directory accumulates `.#` files that make other Emacs instances believe the
data is being edited elsewhere. Anything else built on bookmark.el, and any
package calling `write-file`, is affected. Affects: `addressbook-bookmark` (1).

Read with entry 26, which is its converse: an ordinary modified visited buffer
takes no lock at all. Locking is wrong in both directions.

## 24. The `default` face ignores a display-conditional theme setting

`default` is not special in general — it is display-*conditional* settings for
`default` that are dropped. All three cases, verified:

| `default` setting | GNU | Neomacs |
|---|---|---|
| unconditional `((t …))` | applied | applied |
| nil display clause `((nil …))` | applied | applied |
| conditional `((((class color)) …))` | applied | **dropped** |

Ordinary faces take the conditional form fine; only `default` does not.
Both editors agree the clause matches (`face-spec-set-match-display` returns
`(color)`) and both store an identical spec.

```elisp
(set-frame-parameter nil 'display-type 'color)
(deftheme probe)
(custom-theme-set-faces 'probe
  '(default ((((class color)) (:background "gray20"))))
  '(font-lock-keyword-face ((((class color)) (:foreground "gray70")))))
(enable-theme 'probe)
(list (face-attribute 'default :background nil t)
      (face-attribute 'font-lock-keyword-face :foreground nil t))
;; GNU     => ("gray20" "gray70")
;; Neomacs => ("unspecified-bg" "gray70")
```

In a real colour terminal this would mean every such theme's background and
foreground stay at the terminal's while everything else gets themed. That
consequence is **inferred, not observed** — batch reports a `mono` display, so
the reduction sets `display-type` by hand. No suite witnesses this: it needs a
theme whose `default` clause carries no `min-colors`, and `adwaita-dark-theme`'s
does. Recorded from a verified reduction rather than a failing test — though two
of the three rows above are now covered by *passing* tests: `abyss-theme` for the
unconditional form and `alect-themes` for the nil-display form.

## 25. An interpreted lambda's parameter destroys a built-in buffer-local

Calling an **interpreted list** lambda whose parameter names a built-in
buffer-local variable leaves that variable void — **process-wide and
permanently**, not just in the current buffer.

```elisp
(with-temp-buffer
  (funcall (list 'lambda (list 'mode-name) "ok") "Ag")
  mode-name)
;; GNU     => "Fundamental"
;; Neomacs => (void-variable mode-name)
```

```elisp
(progn
  (with-temp-buffer (funcall (list 'lambda (list 'mode-name) "ok") "Ag"))
  (list (with-temp-buffer mode-name) (default-value 'mode-name)))
;; GNU     => ("Fundamental" nil)
;; Neomacs => (void-variable mode-name)      ; every buffer, for the rest of the session
```

**Both conditions are required.**

*An interpreted list lambda.* These three reproduce — `(list 'lambda …)`,
`` `(lambda (mode-name) …) `` and `'(lambda (mode-name) …)`. A real closure does
**not**:

```elisp
(with-temp-buffer (funcall (lambda (mode-name) "ok") "Ag") mode-name)
;; "Fundamental" in both editors
```

Under lexical binding a bare `lambda` is a closure, so only a list built by
`quote`/backquote/`list` hits it. `ag.el` has no lexical-binding cookie and
builds exactly `` `(lambda (mode-name) ,(ag/buffer-name …)) ``.

*The parameter must shadow a built-in buffer-local.* A user variable is fine:

```elisp
(defvar-local pv 'default-val)
(with-temp-buffer (setq-local pv 'loc) (funcall (list 'lambda (list 'pv) "ok") "x") pv)
;; 'loc in both editors
```

`major-mode` is damaged the same way, but GNU signals `wrong-type-argument` on
that input, so `mode-name` is the clean witness.

Likely mechanism, consistent with a neighbouring verified difference —
`(default-value 'mode-name)` is nil in GNU and `"Fundamental"` here, so Neomacs
appears to hold the value in the default cell with no per-buffer binding, and
unwinding the dynamic bind clears the only cell there is.

Blast radius: any package passing an interpreted list lambda whose parameter
names a built-in buffer-local. The `compilation-start` name-function idiom is
the common one and is not exotic — `ag.el` reaches it on **every search**, after
which `mode-name` is void everywhere. Affects: `ag` (5).

## 26. No lock file is created for a modified visited buffer

The converse of entry 23, and the two together say Neomacs's file locking is
inverted: it omits the lock GNU takes, and leaves behind one GNU never makes.

```elisp
(let ((buf (find-file-noselect FILE)))          ; FILE exists, create-lockfiles is t
  (with-current-buffer buf
    (goto-char (point-max))
    (insert "edited\n")
    (list (directory-files DIR)                 ; while modified
          (progn (save-buffer) (directory-files DIR)))))
;; GNU     while modified => ("." ".#secret.txt" ".." "secret.txt")
;;         after save     => ("." ".." "secret.txt" "secret.txt~")
;; Neomacs while modified => ("." ".." "secret.txt")          ← no lock, ever
;;         after save     => ("." ".." "secret.txt" "secret.txt~")
```

`create-lockfiles` is t and `buffer-modified-p` is t in both. The consequence is
the one locking exists to prevent: two Emacs instances editing the same file see
nothing to warn about. Affects: `agenix` (1).

Read with entry 23: a `write-file` leaves a stale `.#` lock that is never
removed, while an ordinary modified buffer takes no lock at all.

## 27. `get-buffer-window` does not prefer the selected window

When a buffer is showing in more than one window and the selected window is not
the frame's first, GNU returns the *selected* window and Neomacs returns another
one. GNU's `window_loop` `GET_BUFFER_WINDOW` case returns the selected window
specifically.

```elisp
(let ((shared (generate-new-buffer "*shared*")))
  (with-current-buffer shared (insert "one\ntwo\nthree\nfour\n"))
  (set-window-buffer (selected-window) shared)
  (let ((second (split-window)))
    (set-window-buffer second shared)
    (select-window second)
    (eq (get-buffer-window shared) (selected-window))))
;; GNU => t    Neomacs => nil
```

`window-list` reports the same order and the same selected window in both
editors, so only the lookup differs. Note the obvious constructions all *agree* —
`select-window` on the second window is what exposes it, which is why this took
five attempts to reduce.

User-visible effect in `all-ext`: `all-next-error` looks the `*All*` window up
this way and steps from that window's point, so after jumping to a match and
returning — which leaves the collection showing in two windows — `next-error`
continues from the wrong window's position and visits the wrong match (the first
collected line in GNU, the third here). Any package that shows a buffer twice and
then acts "on the window the user is in" is affected. Affects: `all-ext` (1).

## 28. An error signalled in `pre-command-hook` is not reported

Both editors remove the failing hook; only GNU says why. Third member of the
silent-diagnostic family, with entries 17 and 18 — but unlike the process-filter
one this is assertable, because GNU logs and continues rather than dying.

```elisp
(add-hook 'pre-command-hook (lambda () (error "hook boom")) nil t)
(execute-kbd-macro "x")
(list (buffer-string) (copy-sequence pre-command-hook)
      (with-current-buffer "*Messages*" (buffer-string)))
;; GNU     => "x", (t), "Error in pre-command-hook (…): (error \"hook boom\")"
;; Neomacs => "x", (t), ""
```

A mode whose hook has just been disabled by an error explains itself in GNU and
goes quiet here. Affects: `alt-codes` (1).

## 29. `function-key-map` holds a malformed translation for keypad digits

The value must be a key vector; Neomacs wraps the character in a list, so the
translation never fires.

```elisp
(lookup-key function-key-map [kp-6])
;; GNU     => [54]
;; Neomacs => [(54)]
```

`(key-binding (kbd "M-6"))` is `digit-argument` in both, so the binding is right
and only the translation is wrong: `M-<kp-6>` reaches no binding at all
("M-6 is undefined") where GNU dispatches `digit-argument`. Anything reached
through a keypad key — numeric prefixes, `kp-add`, calc, any user binding on the
keypad — is affected. Affects: `alt-codes` (1).

## 30. `*Messages*` keeps a `...` progress line that GNU replaces

GNU deletes the previous log line when the new message diverges from it *after* a
`...` in the previous line — the near-universal `"Working..."` → `"Working...done"`
progress idiom collapses to a single line. Neomacs logs both.

```elisp
(message "Analysing project for completion...")
(message "Analysing project for completion...done")
(with-current-buffer "*Messages*" (buffer-string))
;; GNU     => "Analysing project for completion...done\n"
;; Neomacs => "Analysing project for completion...\nAnalysing project for completion...done\n"
```

**Not general prefix collapsing**, which is what makes it easy to misdiagnose.
The full characterisation, from an independent reduction that agrees with this
one:

| previous → new | GNU | Neomacs |
|---|---|---|
| `Foo...` → `Foo...done` | one line | two lines |
| `Foo...` → `Foo...50%` → `Foo...done` | one line | three lines |
| `Foo` → `Foobar` (plain prefix) | two lines | two lines |
| `Foo...` → `Bar...done` | two lines | two lines |
| `Foo...` → `Foo...` (identical) | `Foo... [2 times]` | `Foo... [2 times]` |
| `Foo...\n` → `Foo...done` | two lines | two lines |

Only the `...` continuation differs, and the multi-step row is the one that
shows the cost: a progress report with N steps leaves N lines in Neomacs where
GNU leaves one. The mechanism is `message_log_check_duplicate`
(GNU `src/xdisp.c:12501`), which walks the two lines together tracking a
`seen_dots` flag set once the previous line has had `...` behind it, and on the
first differing byte returns that flag; a nonzero return makes `message_dolog`
`del_range_both` the previous line.

Neomacs has the *exact*-duplicate half of the same function — `"Same"` twice
gives `"Same [2 times]"` in both editors — so what is missing is the `seen_dots`
branch rather than the duplicate machinery.

Blast radius is wide: `...`/`...done` is the standard Emacs progress idiom, so
any workflow that asserts `*Messages*` content around a long operation sees an
extra line. Affects: `anakondo` (1).

---

## 31. `self-insert-command` never expands an abbrev

`abbrev-mode` is on, the abbrev is in the buffer's local table, and
`expand-abbrev` called directly expands it — but typing the character that
should trigger the expansion just inserts it. GNU expands the word before
point whenever the self-inserted character is not a word constituent, in
`internal_self_insert` (`src/cmds.c`), which calls `Fexpand_abbrev` before
inserting.

```elisp
(define-abbrev-table 'probe-abbrev-table '(("se" "SEQUENCE")))
(with-temp-buffer
  (setq local-abbrev-table probe-abbrev-table)
  (abbrev-mode 1)
  (insert "se")
  (let ((last-command-event ?\s)) (call-interactively 'self-insert-command))
  (buffer-string))
;; GNU     => "SEQUENCE "
;; Neomacs => "se "
```

Not a keyboard-macro artefact and not a display artefact: it fails identically
whether the character arrives through `execute-kbd-macro` or through a direct
`call-interactively`, and whether or not the buffer is shown in a window. SPC,
`,` and RET all fail the same way. `(abbrev-expansion "se" probe-abbrev-table)`
is `"SEQUENCE"` in both editors, so the table itself is correct — only the
trigger is missing.

Blast radius is every abbrev, which is the whole point of the feature: abbrev
tables are how major modes offer keyword completion without a completion
framework, and `abbrev-mode` is also what `skeleton`, `expand.el` and many
language modes build on. Affects: `asn1-mode` (1).

## 32. Backward `forward-comment` ignores a comment with a two-character ender

Moving backward over a comment works for `;`-style and `/* */`-style comments,
but not when the comment's *opening* characters also carry the comment-end
flags 3 and 4 — the `--` of ASN.1, Ada, SQL, Lua and Haskell, where `--` both
opens a comment and can close one. When such a comment is in fact terminated by
a newline, Neomacs refuses to move back over it.

```elisp
(let ((table (make-syntax-table)))
  (modify-syntax-entry ?-  ". 1234" table)   ; -- opens a comment and -- closes one
  (modify-syntax-entry ?\n ">" table)        ; a newline closes one too
  (with-temp-buffer
    (set-syntax-table table)
    (insert "AB\n-- C\nDE\n")
    (goto-char 9)                            ; start of "DE", just past the comment
    (list (forward-comment -100) (point))))
;; GNU     => (nil 3)   ; skipped the comment, stopped before the newline at 3
;; Neomacs => (nil 8)   ; stopped on the comment's closing newline
```

The flags are what matters, not the character class: `". 1234"` and `"w 1234"`
both fail, and dropping the ender flags to `". 12"` makes both editors return
`(nil 3)`. Forward motion is correct — `(goto-char 4) (forward-comment 100)`
lands at 9 in both — so only the backward scan is affected. `syntax-ppss`
agrees with GNU at every position in the buffer, including inside the comment,
so the comment is being *parsed* correctly and only `forward-comment`'s
backward walk mishandles it.

The visible damage is indentation. Any SMIE-based mode calls
`forward-comment` with a negative argument from its backward tokenizer, so a
comment line makes the tokenizer stop inside the comment and report a word from
the comment text as the previous token. `smie-indent-keyword` then computes a
virtual indentation at a position inside the comment, `smie-indent-comment-inside`
returns `noindent` from there, and that propagates out through
`smie-indent-calculate`: the comment line and the line after it are left
untouched at column 0. In asn1-mode:

```elisp
;; buffer: "Bestellung DEFINITIONS ::=\nBEGIN\n-- 1.2 Zustaende\nZustand ::= INTEGER\nEND\n"
;; point at the indentation of line 3
(smie-indent-calculate)
;; GNU     => 0
;; Neomacs => noindent
```

Affects: `asn1-mode` (1).

## 33. `call-interactively` on a non-interactive autoload never resolves it

An autoload declared **without** the interactive flag: GNU loads the file (here
reporting that it is missing), Neomacs signals `commandp` against the still
unresolved autoload object.

```elisp
(autoload 'probe-plain "no-such-library-xyz")
(call-interactively 'probe-plain)
;; GNU     => (file-missing "Cannot open load file" … "no-such-library-xyz")
;; Neomacs => (wrong-type-argument commandp probe-plain)
```

**The fix is confined to `call-interactively`'s ordering — `interactive-form`
already agrees.** Everything around it matches:

| probe | GNU | Neomacs |
|---|---|---|
| `(interactive-form 'probe-plain)` | `file-missing` | `file-missing` ✓ |
| `(probe-plain)` — ordinary call | `file-missing` | `file-missing` ✓ |
| `(commandp 'probe-plain)` | nil | nil ✓ |
| `(commandp 'probe-interactive)` | t | t ✓ |
| `(call-interactively 'probe-interactive)` | `file-missing` | `file-missing` ✓ |
| `(call-interactively 'probe-plain)` | `file-missing` | **`commandp`** ✗ |

GNU's `Fcall_interactively` (`src/callint.c:312`) obtains the form *first* —
`Lisp_Object form = calln (Qinteractive_form, function);` — and only then barfs
`wrong_type_argument (Qcommandp, function)` if it is not a cons. Since
`interactive-form` resolves an autoload in **both** editors, Neomacs must be
testing `commandp` before asking for the form. Reordering is the whole fix.

Affects: `airplay` (1) — five of its eight entry points lack an `interactive`
form and take this path. Any `M-x`-reachable autoload stub whose real definition
is loaded on demand.


## 34. A regexp using a syntax class does not trigger `syntax-propertize`

GNU's regexp engine propertizes the buffer before matching a syntax class
(`\s-`, `\sw`, `\s(` …) when `parse-sexp-lookup-properties` is non-nil and a
`syntax-propertize-function` is installed — the search calls
`internal--syntax-propertize`, which is wrapped in `save-match-data`
(`lisp/emacs-lisp/syntax.el:480`). Neomacs's engine matches the syntax class
without propertizing, leaving the work to whatever calls `syntax-ppss` next.

```elisp
(with-temp-buffer
  (setq-local syntax-propertize-function
              (syntax-propertize-rules ("x" (0 (ignore)))))
  (setq-local parse-sexp-lookup-properties t)
  (insert "a b\n")
  (goto-char (point-min))
  (list (re-search-forward "\\s-" nil t) syntax-propertize--done))
;; GNU     => (3 5)     ; the search propertized the whole buffer
;; Neomacs => (3 -1)    ; nothing was propertized
```

**The damage is not the deferral, it is where the deferred work lands.**
`syntax-ppss` reaches `syntax-propertize` on a path that is *not*
`save-match-data`-protected, so the propertization destroys the match data of
an interleaved `string-match`. Code that alternates `string-match` on a string
with `syntax-ppss` on the buffer — a completely ordinary "is this match inside
a comment?" loop — then indexes the string with buffer positions:

```elisp
(with-temp-buffer
  (setq-local syntax-propertize-function
              (syntax-propertize-rules ("x" (0 (ignore)))))
  (setq-local parse-sexp-lookup-properties t)
  (insert "a b c d e f g h i j k l m n o p q r s t u v w x y z\n")
  (goto-char (point-min))
  (re-search-forward "\\s-" nil t)
  (let ((subject "'zeta'"))
    (string-match "'[^']+'" subject 0)
    (syntax-ppss)
    (match-string 0 subject)))
;; GNU     => "'zeta'"
;; Neomacs => (args-out-of-range "'zeta'" 47 48)
```

In GNU the search has already propertized, so the later `syntax-ppss` is a
cache hit and touches nothing. Both editors clobber match data if the
propertization really happens inside `syntax-ppss` — that part is shared, and
GNU only escapes it because the search got there first. So the fix is in the
regexp engine, not in `syntax-ppss`.

Blast radius: any major mode with a `syntax-propertize-function` whose commands
mix string matching with syntactic queries. Affects: `alan-mode` (1) —
`alan-grammar-update-keyword` searches for keyword groups with a `\s-`-bearing
regexp, then loops `string-match` over each group calling `syntax-ppss` to skip
commented-out entries, so `M-x alan-grammar-update-keyword` rewrites the
keywords section in GNU and signals `args-out-of-range` in Neomacs.


## 35. A `:family` set beside a colour on `default` is discarded by GNU only

Specific to the **`default` face**, and to setting a font family *together with*
a colour. Either alone agrees; any other face agrees. GNU resets the family to
`"default"`, Neomacs reports the family the theme asked for.

```elisp
(progn
  (deftheme probe)
  (custom-theme-set-faces 'probe
    '(default ((t (:family "Terminus" :foreground "#ffffff")))))
  (enable-theme 'probe)
  (face-attribute 'default :family nil 'default))
;; GNU => "default"    Neomacs => "Terminus"
```

The three neighbouring cases all agree in both editors, which is what makes the
characterisation narrow rather than "GNU will not report an uninstantiable
font":

| spec on the face | GNU | Neomacs |
|---|---|---|
| `default` with `:family` alone | `"Terminus"` | `"Terminus"` |
| `default` with `:family` + `:foreground` | **`"default"`** | `"Terminus"` |
| `default` with `:family` + `:background` | **`"default"`** | `"Terminus"` |
| `font-lock-keyword-face` with `:family` + `:foreground` | `"Terminus"` | `"Terminus"` |

`:foundry` follows `:family` exactly. `:height` is **not** affected — both
editors report 130 in every row above — so this is not "GNU drops font
attributes on a tty", and a suite that only checked `:height` would miss it.

Consistent with GNU re-realizing the `default` face when a colour is set on it,
recomputing the font from scratch on a frame that has none, while Neomacs keeps
the requested family. Not confirmed against GNU's source.

Blast radius: any package that sets a font family and reads it back —
`default-text-scale`, font-size adjusters, theme validators, and any theme
shipping a font preference, which is most themes that ship one at all. On a tty
neither editor can use the font, so the visible effect is confined to what
`face-attribute` reports; that is still the value such packages branch on.

Affects: `ahungry-theme` (1) — its `default` spec carries `:foreground
"#ffffff"` and `:family "Terminus" :foundry "xos4"` in one `((t ...))` clause,
so the font workflow reads back `"Terminus"`/`"xos4"` here and
`"default"`/`"default"` in GNU. The suite's colour-focused workflows read a
deliberately narrowed attribute list so that only the font workflow witnesses
this.

## 36. Process output does not relocate markers at the process mark

GNU's default process filter inserts output the way `insert-before-markers`
does, so a marker sitting exactly at the process mark is carried to the end of
the freshly inserted text. Neomacs inserts the same bytes and leaves such a
marker behind, pointing at where the output began instead of where it ended.

```elisp
(let* ((buffer (generate-new-buffer " *probe*"))
       (process (start-process "probe" buffer "sh" "-c" "printf world")))
  (with-current-buffer buffer
    (insert "hello")
    (set-marker (process-mark process) (point-max))
    (let ((mark (copy-marker (point-max))))
      (while (accept-process-output process 0.2))
      (list (buffer-string) (marker-position mark)
            (marker-position (process-mark process))))))
;; GNU     => ("helloworld\nProcess probe finished\n" 11 35)
;; Neomacs => ("helloworld\nProcess probe finished\n"  6 35)
```

**The buffer text and the process mark itself agree** — only the third-party
marker differs, which is what makes this quiet. And `insert-before-markers`
called directly is correct in both editors (the marker moves to 11 either way),
so the primitive is not implicated: the divergence is confined to the
insertion the default filter performs.

The markers this strands are the ones comint keeps at the process mark —
`comint-last-input-end`, `comint-last-output-start`. A buffer whose
`comint-last-input-end` points before the echoed output rather than after it
reports the wrong "last input" region, so `comint-get-old-input`, input history
and anything that shows or re-sends the previous command work from a region
that includes output the user never typed.

Affects: `alchemist` (1) — `alchemist-iex--send-command` deliberately uses
`insert-before-markers` and then `move-marker comint-last-input-end (point)`
before handing the line to `comint-send-string`; when IEx echoes it back, GNU
carries that marker past the echo and Neomacs leaves it at 39 where GNU has 77.


## 37. Killing a windowed buffer leaves the current buffer out of sync

After `kill-buffer` on a buffer that is displayed in the selected window, GNU
leaves the current buffer and the selected window's buffer the same. Neomacs
gives the *window* the right replacement and leaves a *different* buffer
current.

```elisp
(save-window-excursion
  (find-file FILE)
  (kill-buffer)
  (list (buffer-name)
        (buffer-name (window-buffer (selected-window)))
        (eq (current-buffer) (window-buffer (selected-window)))))
;; GNU     => ("*scratch*"  "*scratch*" t)
;; Neomacs => ("*Messages*" "*scratch*" nil)
```

**This is not entry 13**, and all three ways it could have been were checked:
`buffer-list` returns the same buffers in the same order in both editors here;
the window agrees; and `kill-buffer` with *no window displaying the victim*
agrees too — both editors land on `*Messages*`. Only the current buffer
diverges, and only when a window displayed the buffer being killed.

The damage is that `default-directory` comes with the current buffer. Any
command that kills a buffer and then resolves a path is working from a
directory the user never chose. In `aidermacs`, `aidermacs-project-root` reads
`default-directory` after the prompt buffer is killed, so a second
`M-x aidermacs-open-prompt-file` creates `.aider.prompt.org` four directories
*above* the project instead of inside it — a file written outside the
repository the user is working in. Affects: `aidermacs` (1).

## 38. A quantifier after the `` \` `` anchor is not treated as literal

GNU treats `*`, `+` and `?` as literal characters wherever they have nothing to
repeat. Neomacs implements that rule at the start of a pattern, after `\(` and
after `\|`, but **not after the `` \` `` string/buffer-start anchor**, where it
applies the quantifier to the anchor itself.

```elisp
(list (string-match "\\`+a\\'" "+a")     ; GNU 0   Neomacs nil
      (string-match "\\`*a\\'" "*a")     ; GNU 0   Neomacs 1
      (string-match "\\`?a\\'" "?a"))    ; GNU 0   Neomacs 1
```

Everything around it agrees, which is what makes the entry narrow:

| pattern | GNU | Neomacs |
|---|---|---|
| `+a` (no anchor) | 0 | 0 ✓ |
| `\(+a\)` | 0 | 0 ✓ |
| `\`\|+\`` after `\|` | 0 | 0 ✓ |
| `\<+a` | 1 | 1 ✓ |
| `\+a` (escaped) | 0 | 0 ✓ |
| `` \`+a `` | 0 | **nil** ✗ |
| `` \`*a `` | 0 | **1** ✗ |

The `*` and `?` rows are the dangerous ones: they do not fail, they match at the
**wrong offset**, because "zero or more start-anchors" succeeds on the empty
string and the rest of the pattern then matches one character late.

Blast radius: anchoring a literal `+`, `*` or `?` at string start is the normal
way to recognise an `emacsclient`-style `+LINE:COL` argument, a diff hunk
marker, or a leading option character. Affects: `ai-code` (1) —
`ai-code-editor-viewport--parse-file-arguments` matches
``"\\`+\\([0-9]+\\)\\(?::\\([0-9]+\\)\\)?\\'"``, so `+12:4 src/api.el` opens
`api.el` at line 12 column 4 in GNU, while Neomacs takes `+12:4` to be a
filename of its own and opens no file at any position.


## 39. `easy-menu-add-item` drops a submenu that carries any property

The whole entry goes — key, label, binding and property together — leaving a
degenerate `(nil (menu-item nil))` where the submenu should be.

```elisp
(progn
  (require 'easymenu)
  (easy-menu-define sub nil "" '("Sub" :help "h" ["I" ignore]))
  (easy-menu-define top nil "" '("Top"))
  (easy-menu-add-item top nil sub)
  (let (out) (map-keymap (lambda (k v) (push (list k v) out)) top) out))
;; GNU     => ((Sub (menu-item "Sub" (keymap "Sub" (I menu-item "I" ignore)) :help "h")))
;; Neomacs => ((nil (menu-item nil)))
```

**It is not `:visible`, and it is not the submenu-keymap path.** The controls
are what make the entry precise, and each was run in both editors:

| submenu carries | GNU | Neomacs |
|---|---|---|
| no property | `(Sub (menu-item "Sub" (keymap …)))` | same |
| `:visible t` | `(Sub (menu-item "Sub" (keymap …)))` | same |
| `:visible foo` | `… :visible foo` | **`(nil (menu-item nil))`** |
| `:enable foo` | `… :enable foo` | **`(nil (menu-item nil))`** |
| `:help "h"` | `… :help "h"` | **`(nil (menu-item nil))`** |

`:visible t` is the decisive control: easymenu optimises a constant-true
`:visible` away, leaving an empty property list, and then the two agree. So the
trigger is the *presence of a surviving property*, not any particular one.

**And the keymap layer is innocent.** Building the identical structure by hand
agrees in both editors, so `menu-item` storage, submenu keymaps and property
retention are all correct and the loss is inside easymenu's splicing:

```elisp
(let ((m (make-sparse-keymap "Top")) (sub (make-sparse-keymap "Sub")))
  (define-key sub [i] (list 'menu-item "I" 'ignore))
  (define-key m [Sub] (list 'menu-item "Sub" sub :help "h"))
  (let (out) (map-keymap (lambda (k v) (push (list k v) out)) m) out))
;; both => ((Sub (menu-item "Sub" (keymap (i menu-item "I" ignore) "Sub") :help "h")))
```

Blast radius is wider than the package that surfaced it: a propertied submenu
spliced with `easy-menu-add-item` is an ordinary way to build a menu, and the
failure is silent — the menu simply has a hole where the submenu should be,
with no error and nothing in `*Messages*`.

Affects: `anju` (6). `anju-context-menu.el:866` defines the Hide/Show submenu
with `easy-menu-define … '("Hide/Show" :visible hs-minor-mode …)` and `:981`
splices it with `easy-menu-add-item`; all six of that package's Neomacs-only
failures are menus built that way — four in `context_menu.rs`, two in
`initialization.rs`.


## 40. `window-body-width` keeps the tty vertical-bar column

On a terminal frame a window that has another window to its right gives up one
column to the vertical bar drawn between them. GNU subtracts that column from
the body width; Neomacs reports the full total width, so every window except
the rightmost is reported one column wider than the text it can hold.

```elisp
(let ((right (split-window-right)))
  (list (window-total-width) (window-body-width)
        (window-total-width right) (window-body-width right)))
;; GNU     => (40 39 40 40)
;; Neomacs => (40 40 40 40)
```

**The split itself is right; only the body measurement is wrong.** The controls
were run in both editors and pin the failure to one term:

| | GNU | Neomacs |
|---|---|---|
| `window-total-width`, every window | `(20 20 40)` | same |
| `window-body-width`, rightmost window | `40` | same |
| `window-body-width`, non-rightmost, three-way split | **`(19 19 40)`** | `(20 20 40)` |
| `window-text-width`, every window | `(20 20 40)` | same |

GNU's rule is `src/window.c:1096` `window_body_width`, which subtracts a
one-column vertical bar when `!FRAME_WINDOW_P (f) && !WINDOW_RIGHTMOST_P (w)
&& !WINDOW_RIGHT_DIVIDER_WIDTH (w)`. Neomacs's
`window_body_width_pixels` (`neovm-core/src/emacs_core/window_cmds/mod.rs:991`)
subtracts scroll bars, fringes and margins by way of
`window_body_horizontal_offsets_pixels`, and has no term for the bar at all.
The condition is `!FRAME_WINDOW_P`, so this is a terminal-frame divergence
only.

The consequence is off-by-one overflow rather than a visible error: any package
that asks how much text fits and then fills the line to that width will run one
column past the text area of a split tty window, and the last character wraps.

Affects: `ascii-table` (2). Both `workflows.rs` tests split an 80-column batch
frame at 50 and read the window back: GNU reports 49, Neomacs 50. Every
decision `ascii-table` itself makes from that number agrees — same layout, same
rendered text, same narrowest-window choice — so the two tests are red on the
measurement alone.


## 41. `ceiling` and friends accept a marker as the divisor

`ceiling`, `floor`, `truncate` and `round` take an optional DIVISOR that GNU
requires to be a number. Neomacs checks `integer-or-marker-p` instead, which
both admits a marker GNU rejects and names the wrong predicate when it does
signal.

```elisp
(list (condition-case e (ceiling 5 (point-marker)) (error e))
      (condition-case e (ceiling 128 "8") (error e)))
;; GNU     => ((wrong-type-argument numberp #<marker at 1 in *scratch*>)
;;             (wrong-type-argument numberp "8"))
;; Neomacs => (5
;;             (wrong-type-argument integer-or-marker-p "8"))
```

**All four members of the family, not just `ceiling`** — a fix that patches one
leaves three live. Run in both editors:

```elisp
(mapcar (lambda (f) (list f (condition-case e (funcall f 5 (point-marker)) (error e))))
        '(ceiling floor truncate round))
;; GNU     => ((ceiling (wrong-type-argument numberp #<marker at 1 in *scratch*>))
;;             (floor    …) (truncate …) (round …))   ; all four signal
;; Neomacs => ((ceiling 5) (floor 5) (truncate 5) (round 5))
```

The marker case is the one that matters: GNU signals, Neomacs silently divides
by the marker's position, so `(ceiling 5 (point-marker))` returns 5 at
point-min and a different answer everywhere else in the buffer.

The single-argument form is unaffected — `(ceiling "8")` signals
`(wrong-type-argument numberp "8")` in both — and a float divisor is accepted
by both, so the divergence is confined to the DIVISOR type check. GNU reaches
it through `rounding_driver` in `src/floatfns.c`, which applies `CHECK_NUMBER`
to the divisor, and `CHECK_NUMBER` does not admit markers.

Affects: `ascii-table` (1).
`formatting.rs:295 ascii_table_table_invalid_radix_control_and_row_counts_signal_exact_errors`
passes `"8"` as codepoints-per-row, which reaches `(ceiling 128 "8")` inside
`ascii-table--table`. That test predates this catalogue entry and was already
red.


## 42. A tree-sitter font-lock setting without its language slot fails to compile

GNU 31 carries the language in a sixth slot of each `treesit-font-lock-settings`
entry: `(QUERY ENABLE FEATURE OVERRIDE nil LANGUAGE)`. A setting rebuilt with
only the first four elements still fontifies in GNU and signals in Neomacs.

> **Precondition — this is the one entry in this file that does not run from a
> bare `emacs -Q --batch`.** It needs the **css tree-sitter grammar**, and
> `(treesit-language-available-p 'css)` is `nil` in a stock session of *both*
> editors. Without it the reduction dies at `(wrong-type-argument
> treesit-parser-p)` before reaching any compile — identically in both, so it
> reads as "does not reproduce" rather than as a missing grammar. Either
> install one with `M-x treesit-install-language-grammar RET css`, or point at
> the suite's pinned copy, which is what the harness itself does:
>
> ```elisp
> (setq treesit-extra-load-path
>       (list "tmp/melpa/tree-sitter-grammar-cache/css/\
> dda5cfc5722c429eaba1c910ca32c2c0c5bb1a3f/home/.emacs.d/tree-sitter"))
> ```
>
> Confirm `(treesit-language-available-p 'css)` is `t` before concluding
> anything about the reduction below.

```elisp
(require 'css-mode)
(with-temp-buffer
  (insert ".a { color: red; }")
  (treesit-parser-create 'css)
  (setq-local treesit-font-lock-settings
              (list (seq-take (car css--treesit-settings) 4)))
  (setq-local treesit-font-lock-feature-list '((comment)))
  (treesit-font-lock-recompute-features)
  (font-lock-ensure))
;; GNU     => fontifies
;; Neomacs => (wrong-type-argument treesit-query-p nil treesit-query-compile)
```

**It is the missing slot, and it is order-dependent.** Controls, all run in both
editors:

| | GNU | Neomacs |
|---|---|---|
| the setting unmodified (6 elements) | fontifies | same |
| `copy-sequence` of it, still 6 elements | fontifies | same |
| truncated to 4, on a cold buffer | fontifies | **signals** |
| truncated to 4, *after* a 6-element setting has been used once | fontifies | fontifies |

So the query object itself is fine — it reports `treesit-query-p` as `t`, and it
even prints its own language (`#s(treesit-compiled-query 1 css …)`) — and
nothing is wrong with the truncated list's identity. What fails is the first
compile of a setting that has no language slot, and once some other setting has
primed whatever Neomacs caches, the same truncated setting goes through.

The blast radius is any package that borrows another mode's tree-sitter rules
and rebuilds them positionally, which is the ordinary idiom for embedding one
language's font-lock in another's mode. The rebuild is written against the
four fields the author knows about and silently drops the rest.

Affects: `astro-ts-mode` (19 of 38 — every test that activates the mode).
`astro-ts-mode--prefix-font-lock-features` rebuilds each borrowed CSS and TSX
setting as `(list (nth 0 setting) (nth 1 setting) (intern …) (nth 3 setting))`,
so 28 of the mode's 33 settings arrive four elements long and
`treesit-major-mode-setup` dies inside `define-derived-mode`. The five native
Astro rules, built by `treesit-font-lock-rules`, are unaffected — setting only
those and fontifying succeeds in Neomacs.

Reductions that did **not** isolate it, recorded so nobody repeats them:
`treesit-range-rules` with `:local t` agrees; all six of the mode's own
`treesit-query-compile` inputs agree; string, sexp and vector query forms agree
(both editors reject a bare vector); and the unmodified `css--treesit-settings`
and `typescript-ts-mode--font-lock-settings` are identical in shape and length
in both editors.

Two smaller observations from the same reduction, neither worth its own entry.
Neomacs's `wrong-type-argument` data from `treesit-query-compile` carries a
third element naming the function where GNU's carries two. And Neomacs's
compiled query prints as a record exposing its language and source pattern
where GNU's prints as the opaque `#<treesit-compiled-query>`.

## 43. `autoload` does not record its definition in `load-history`

`defalias` attaches a `(defun . SYMBOL)` entry to the loading file's
`load-history` record; `autoload`, which GNU implements *as* a `defalias`, does
not in Neomacs. The file's other entries are all present, so the omission is
specific to the autoload.

```elisp
;; a file containing
;;   (autoload 'lh-probe-command "lh-probe-lib" "Docstring." t)
;;   (defvar lh-probe-var 1)
;;   (defun lh-probe-real-fn () nil)
;;   (provide 'lh-probe)
(cdr (assoc (expand-file-name "lh-probe.el") load-history))
;; GNU     => ((defun . lh-probe-command) lh-probe-var
;;             (defun . lh-probe-real-fn) (provide . lh-probe))
;; Neomacs => (lh-probe-var (defun . lh-probe-real-fn) (provide . lh-probe))
```

**It is the `autoload` function, not autoload-ness of the definition.** The
control is the decisive one and was run in both editors -- handing `defalias` an
autoload object directly is recorded by both, so nothing rejects autoloads as
such:

| defined by | GNU | Neomacs |
|---|---|---|
| `(defalias 'x (lambda () nil))` | `(defun . x)` | same |
| `(defalias 'x (list 'autoload …))` | `(defun . x)` | same |
| `(autoload 'x "lib" nil t)` | `(defun . x)` | **absent** |

GNU records it in `defalias` at `src/data.c:970`, which attaches for every
definition except while dumping:

```c
bool autoload = AUTOLOADP (definition);
if (!will_dump_p () || !autoload)
  LOADHIST_ATTACH (Fcons (Qdefun, symbol));
```

and `Fautoload` (`src/eval.c`) ends in `return Fdefalias (function, ...)`, so
the autoload path reaches the same attach. Neomacs's `autoload` does not go
through whatever its `defalias` uses to record the entry.

**The consequence is that unloading leaves the commands behind.**
`unload-feature` undoes definitions by walking these entries, so an autoloaded
command survives the unload and stays in `M-x`, pointing at a file the user
just unloaded:

```elisp
;; a file containing (autoload 'lh-feat-command "lh-feat-lib" nil t)
(load "lh-feat.el") (unload-feature 'lh-feat t) (fboundp 'lh-feat-command)
;; GNU     => nil
;; Neomacs => t
```

`symbol-file` is *not* affected -- it returns nil for an autoloaded symbol in
both editors, so `describe-function` is not where this shows up.

Affects: `auto-dark` (1).
`registry.rs auto_dark_generated_autoload_exposes_only_global_mode_before_activation`
reads the autoload file's `load-history` record and gets
`((provide . auto-dark-autoloads))` where GNU gives
`((defun . auto-dark-mode) (provide . auto-dark-autoloads))`.

## 44. The sexp scanner does not follow a `category` text property -- FIXED

FIXED 2026-08-09 (a5ebb0d7a fix, 6670885b6 perf). The scanner now resolves its
`syntax-table` property through the shared `textget` implementation
(`CharPropertyResolver`, neovm-core/src/emacs_core/textprop.rs), so it agrees
with `get-char-property` on every character by construction. The
`honor_properties` flag became `SyntaxProperties::{Ignore, Honor(resolver)}`,
which makes "honours properties but reads them raw" -- the state this bug
lived in -- unrepresentable. `rainbow_delimiters` now reports GNU's
`c-<-as-paren-syntax` / `c->-as-paren-syntax` and its suite is GREEN.
Pinned by neovm-core/src/emacs_core/syntax_category_property_test.rs (14
tests, every expectation measured on GNU 31.0.90), which also pins two GNU
boundaries the fix must not cross: overlays never reach the scanner, and the
fallbacks apply only where an interval exists (GNU `update_syntax_table`
returns early when `interval_of` finds none). Cost: NET NEGATIVE. On a fontified
160k-char syntax-scan sweep the rung is -4.37% instructions against the
pre-fix baseline, and TTY typing is -0.64%. Routing the scanner through the
resolver initially cost +1.29%; 755689fa4 then gave the byte-addressed
scanners (regexp matcher, `forward-comment`, `backward-prefix-chars`) the
property-run cache GNU's `gl_state` has always given them, which removed a
per-character byte->char conversion they had been paying all along: regexp
searching -26.9%, `forward-sexp` -27.6%, fontification -1.7%. Still slightly
up are loops that call `skip-syntax` (+3.6%) or `forward-comment` (+2.4%) at
every buffer position, where a scan examines about one character and
amortizes neither the resolver snapshot nor a refill's two char->byte
conversions.

The reduction below is kept as the regression record.

`parse-sexp-lookup-properties` makes the scanner honour a `syntax-table` text
property. GNU reaches that property through `textget`
(`textprop.c:lookup_char_property`), which falls back to the `category`
property: when the plist carries `(category SYM)` and `SYM` has a
`syntax-table` property, that value is the character's syntax. Neomacs reads
the `syntax-table` text property directly, so a character whose syntax is
supplied *through* a category is scanned with its plain table syntax.

```elisp
(defconst c-<-as-paren-syntax '(4 . ?>))
(put 'c-<-as-paren-syntax 'syntax-table c-<-as-paren-syntax)
(defconst c->-as-paren-syntax '(5 . ?<))
(put 'c->-as-paren-syntax 'syntax-table c->-as-paren-syntax)
(with-temp-buffer
  (let ((parse-sexp-lookup-properties t))
    (set-syntax-table (make-syntax-table))
    (insert "<()>")
    (put-text-property 1 2 'category 'c-<-as-paren-syntax)
    (put-text-property 4 5 'category 'c->-as-paren-syntax)
    (goto-char (point-min))
    (forward-sexp)
    (point)))
;; GNU     => 5   (the whole `<()>` is one sexp)
;; Neomacs => 2
```

The indirection is not broken everywhere -- `syntax-after` and
`get-char-property` resolve it in both editors, which is what makes this narrow
and easy to mistake for working:

| probe | GNU | Neomacs |
|---|---|---|
| `(get-char-property POS 'syntax-table)` via category | `(4 . 62)` | `(4 . 62)` OK |
| `(syntax-after POS)` via category | `(4 . 62)` | `(4 . 62)` OK |
| `forward-sexp` honouring the same category | 5 | **2** wrong |

Blast radius: this is exactly the feature probe CC Mode runs to set
`c-use-category` (`cc-defs.el:1288-1301`), and that constant is baked in at
byte-compile time. With it nil, `c-mark-<-as-paren` puts a direct
`syntax-table` property where GNU puts a `category`, so every CC Mode buffer
carries different text properties from GNU's for template brackets -- and CC
Mode loses the cheap "toggle every template bracket at once" operation the
indirection exists to provide (`cc-defs.el:1851-1858`).

Affects: `rainbow-delimiters` (1).
`cpp_release_manifest_colors_templates_calls_and_initializer_lists` reports
`:category nil` for each of `<` `<` `>` `>` in
`std::vector<std::pair<int, std::string>>` where GNU reports
`c-<-as-paren-syntax` / `c->-as-paren-syntax`. The delimiter *faces* agree, so
the visible colouring is right today; the property mechanism underneath is not.

A fix belongs in the scanner's property lookup
(`neovm-core/src/emacs_core/syntax.rs`, `effective_syntax_entry_for_char_at_byte`
and the `SyntaxPropRange` run cache), which resolve `syntax-table` with a raw
text-property read. Both sit on the per-character scanning path that carries a
run cache, so the change needs a before/after measurement, not just a
correctness test.


## 45. `describe-bindings` omits global bindings and the function-key map — FIXED

**Fixed on three independent causes**, each with its own root and its own
regression test. The reduction below now answers
`(:lines 2311 :has-global-binding t :has-function-key-map t)`; the residual 82
lines against GNU's 2393 are entry 56 (raw 8-bit key descriptions), not this one.

1. **`accessible-keymaps` did not follow a prefix bound to a SYMBOL.** GNU
   `accessible_keymaps_1` resolves every binding with
   `get_keymap (get_keyelt (cmd, 0), 0, 0)`; we tested the raw binding for
   keymap-ness instead. The global map stores every one of its own prefixes as a
   `define-prefix-command` symbol -- `C-x` as `Control-X-prefix`, `C-c` as
   `mode-specific-command-prefix` -- so the whole global keymap tree was
   invisible and `Global Bindings:` was filled from the root map alone (22
   accessible maps against GNU's 135). The walk now goes through
   `resolve_keymap`, the module's single answer to "does this name a keymap",
   and also implements GNU's meta-ization rule (`ESC s` is reported as `[M-s]`,
   spliced in at its parent's length) and GNU's PREFIX handling (start the walk
   at the map PREFIX reaches rather than enumerating and filtering -- the two
   differ, because GNU refuses to metize the key after a PREFIX ending in ESC).
   `map_keymap_following_parents`, a second ad-hoc spine decoder, is gone in
   favour of `list_keymap_for_each_binding_recursive`.
2. **Four sections were never emitted, and one was shadowed that GNU never
   shadows.** `describe-buffer-bindings` now emits GNU's full list in GNU's
   order: overriding terminal/local map, `keymap` property bindings, minor
   modes, `local-map` property vs major mode (distinguished, as GNU does),
   global, `local-function-key-map`, `input-decode-map`. The sections are built
   as data and emitted in one loop, with a `BindingSectionKind` enum carrying
   the only axis the ten-argument `help--describe-map-tree` calls differ on:
   real keymaps accumulate shadow, translation maps neither shadow nor are
   shadowed. We had been consing `key-translation-map` onto the shadow list,
   which GNU never does.
3. **`help--describe-vector` was a no-op stub.** `describe-map` reaches every
   dense keymap element through it, so every char-table binding was silently
   dropped -- `self-insert-command` across the printable range, the whole of
   `key-translation-map`. Ported from GNU `describe_vector` (keymap.c),
   including the bug#9293 range/shadow rule; GNU's own
   `help--describe-vector/bug-9293-*` tests now pass byte-for-byte.

A fourth bug surfaced while verifying 3 and is fixed with it: `map-keymap`
handed Lisp the char-table walk's SHARED range cons. `map-char-table`
deliberately reuses and mutates one cons (GNU does too), and GNU's
`map_keymap_char_table_item` copies it for exactly that reason -- "make a copy
since map_char_table modifies it in place". We planned all pairs before calling
FUNCTION, so every range key carried the final post-walk `(LAST+1 . MAX_CHAR)`.
That broke `keymap-canonicalize`, which collects range keys during `map-keymap`
and calls `define-key` on them afterwards: `(lookup-key (keymap-canonicalize m)
[?b])` returned nil where GNU returns the binding.

Regression tests: `accessible_keymaps_follows_a_prefix_bound_to_a_symbol_naming_a_keymap`,
`accessible_keymaps_metizes_a_key_following_meta_prefix_char`,
`help_describe_vector_prints_ranges_only_where_shadowing_agrees`,
`map_keymap_copies_char_table_range_keys_before_the_shared_cons_moves`
(neovm-core/src/emacs_core/builtins/tests.rs). `helm-descbinds` is green.

Was:

`describe-buffer-bindings` builds a much shorter report than GNU's. Two whole
sections are wrong: user bindings made in the global map are missing from
`Global Bindings:`, and the `Function key map translations:` section is not
emitted at all.

```elisp
(global-set-key (kbd "C-c g s") 'p9-global-status)
(define-key function-key-map (kbd "<f24>") (kbd "C-c d d"))
(with-temp-buffer
  (describe-bindings)
  (with-current-buffer "*Help*"
    (list :lines (count-lines (point-min) (point-max))
          :has-global-binding
          (and (save-excursion (goto-char (point-min))
                               (search-forward "p9-global-status" nil t)) t)
          :has-function-key-map
          (and (save-excursion (goto-char (point-min))
                               (search-forward "Function key map translations" nil t))
               t))))
;; GNU     => (:lines 2393 :has-global-binding t   :has-function-key-map t)
;; Neomacs => (:lines 1441 :has-global-binding nil :has-function-key-map nil)
```

The `Global Bindings:` *heading* is present in both, so the section is being
started and then filled from the wrong keymap; `Key translations:` is present in
both as well. Only the function-key-map section is absent outright.

Affects: `helm-descbinds` (2).
`real_major_minor_and_global_bindings_preserve_gnu_section_precedence` loses the
entire `("Global Bindings:" ("C-c g s" . …))` section from the Helm source list,
and `function_key_translations_are_exposed_as_a_searchable_binding_section`
returns nil where GNU returns
`(("Function key map translations:" ("<f24>" . "C-c d d…")))`. Helm-Descbinds
parses the `*Help*` buffer, so anything `describe-bindings` does not print is
simply not offered to the user.


## 46. `swiper`'s query-replace replaces nothing — FIXED

**Not a replace bug at all.** `swiper-query-replace` was never reached: `M-q` in
the swiper minibuffer ran the global `fill-paragraph`. Tracing the session shows
neomacs never entering `swiper-query-replace`, never calling
`query-replace-compile-replacement`, and never calling `perform-replace` --
where GNU calls all three.

The cause is in `lookup-key`. GNU `access_keymap_1` searches a keymap embedded
in a composed keymap's spine by recursing into ITSELF -- `access_keymap_1
(submap, idx, t_ok, noinherit, autoload)` -- so the member is searched WITH its
own parent chain, and a prefix it shares with that parent is merged before the
composed map's own parent is considered. neomacs searched the member ONE LEVEL
deep, so every binding a composed map's member INHERITS was invisible.

That is the shape of every ivy/swiper minibuffer: `read-from-minibuffer` is
given `(make-composed-keymap swiper-isearch-map ivy-minibuffer-map)`, and `M-q`
lives in `swiper-map`, the parent of the member `swiper-isearch-map`.

Reduced below the package -- no ivy, no swiper, six keymaps:

```elisp
(let* ((grand (make-sparse-keymap))
       (inner (make-sparse-keymap))
       (outer-parent (make-sparse-keymap))
       composed)
  (define-key grand (kbd "M-q") 'from-grandparent)
  (define-key inner (kbd "M-n") 'from-inner)
  (set-keymap-parent inner grand)
  (define-key outer-parent (kbd "M-o") 'from-outer-parent)
  (setq composed (make-composed-keymap inner outer-parent))
  (list (lookup-key inner (kbd "M-q"))
        (lookup-key composed (kbd "M-q"))))
;; GNU     => (from-grandparent from-grandparent)
;; Neomacs => (from-grandparent nil)
```

All three maps must bind under the SAME prefix (here ESC, via the meta
modifier): the lookup has to merge three submaps rather than take the first, and
it is the member's inherited submap that was dropped.

The scope is much wider than swiper. Composed keymaps are how minibuffer
completion UIs, `set-transient-map`, `internal-push-keymap` and every
`make-composed-keymap` caller layer their bindings, and any member of one that
inherits from another map lost those inherited keys.

Pinned by `lookup_key_follows_the_parent_of_a_composed_keymaps_member`
(neovm-core/src/emacs_core/builtins/tests.rs). The `swiper` suite is green.

Was:

```
Neomacs: :text "ticket-417 state:failed\nticket-418 state:healthy\nticket-419 state:failed\n"
         :point (:line 1 :column 0 :text "ticket-417 state:failed")
GNU:     :text "INC-417 state:retry\nticket-418 state:healthy\nINC-419 state:retry\n"
         :point (:line 3 :column 19 :text "INC-419 state:retry")
```

Affects: `swiper` (1)
`query_replace_renames_selected_incidents_with_captured_identifiers`.


## 47. DATA LOSS: `replace-buffer-contents` cannot be undone — FIXED

**Was: data-loss class, the highest-severity entry in this ledger.** The
user's original text was unrecoverable through undo, and nothing warned them:
the forward edit is always correct, so the damage only appeared when undo was
pressed, by which point the original was gone.

Replacing a region through `replace-region-contents` and undoing the result
gave back text that was neither the original nor the replacement -- individual
characters from the replacement were left behind.

```elisp
(with-temp-buffer
  (buffer-enable-undo)
  (insert "greet")
  (setq buffer-undo-list nil)
  (let ((temp (with-current-buffer (generate-new-buffer " *r*")
                (insert "welcome") (current-buffer))))
    (replace-region-contents (point-min) (point-max) (lambda (&rest _) temp)))
  (undo-boundary)
  (undo-only 1)
  (buffer-string))
;; GNU     => "greet"
;; Neomacs => "grmet"
```

**The multibyte character in the originating suite is a red herring.** The
reduction above is pure ASCII. What decides it is the *shape of the diff*:
`replace-buffer-contents` applies its result as a run of small hunks, and the
bug needs two insertion hunks that end up adjacent in the undo list.

| replacement | GNU | Neomacs |
|---|---|---|
| `abc` -> `xyz` (one hunk) | `abc` | `abc` OK |
| `abcde` -> `aZcYe` (equal lengths) | `abcde` | `abcde` OK |
| `abcde` -> `aZZcYYe` | `abcde` | `abcde` OK |
| `greet` -> `welcome` | `greet` | **`grmet`** wrong |

The undo lists show the mechanism directly:

```
GNU:     (("gr" . 1) (3 . 4) ("" . 4) (4 . 8) ("t" . -5) (6 . 6))
Neomacs: (("gr" . 1) (3 . 8)                  ("t" . -5))
```

GNU records six entries, neomacs three. GNU's `("" . 4)` is a **zero-length
deletion**, and it is load-bearing: it sits between the insertions `(3 . 4)` and
`(4 . 8)` and stops them coalescing. `record_insert` merges a new insertion into
the newest record only when that record is an insertion ending exactly where the
new one begins, so the empty deletion breaks the chain. Neomacs skipped
recording a deletion whose text is empty, the two insertions coalesced into
`(3 . 8)`, and undoing that single wide record deletes a span GNU never deletes
as one piece.

GNU's `record_delete` (`undo.c`) has exactly one early return, for a disabled
undo list; it never tests the string's length.

Blast radius: everything built on `replace-buffer-contents` --
`replace-region-contents`, `revert-buffer` with `preserve-modes`, format-on-save
wrappers, and every LSP client applying workspace edits. The forward direction
is always correct, so the damage only appeared when the user pressed undo, which
made it a data-loss bug rather than a visible one.

Affected: `lsp-mode` (1).
`rename_response_applies_ordered_unicode_edits_as_one_undoable_transaction`
reported `:undo-restored nil` with `fn grmet(name)` / `"Hellk, U+1F600 "` where
GNU reports `:undo-restored t` and the original text, and point 36 against GNU's
49. Every intermediate `:after rename` state matched in both editors.

**FIXED.** Three recording seams had to move together; each one alone leaves the
reduction unchanged.

1. `undo_list_record_delete` / `undo_list_record_insert`
   (`neovm-core/src/buffer/undo.rs`) each carried a zero-length early return
   that GNU's `record_delete` / `record_insert` (`undo.c`) lack -- GNU returns
   early only for a disabled undo list. Both guards are gone, so `("" . POS)`
   and `(POS . POS)` records are conses again.
2. `replace_range` with an empty old range still records. GNU sets
   `deletion` to the empty *string* rather than nil, so `!NILP (deletion)`
   holds and it runs `record_insert (from, inschars)` then
   `record_delete (from, "")`. Neomacs routed that case down an insert-only
   path that never reached the delete recorder, at two layers:
   `replace_buffer_measured_region_lisp_string`
   (`neovm-core/src/buffer/insdel.rs`) and `execute_replace_text_plan`
   (`neovm-core/src/buffer/edit_transaction.rs`). Both now go through
   `execute_replace_insert_only_plan`, which keeps the insertion's marker and
   point mechanics -- GNU's `adjust_markers_for_replace` delegates
   `old_chars == 0` straight to `adjust_markers_for_insert` (insdel.c:351) --
   and adds the empty deletion record.
3. `Freplace_region_contents` (editfns.c) calls `Fundo_boundary` once
   compareseq has succeeded and before it walks the change runs, including
   when the diff is empty. Besides the boundary that conses, this sets
   `point_before_last_command_or_undo`, which is what the first run's
   `record_point` then conses ahead of the first change record. The neomacs
   apply loop now does the same, and the trivial empty-side paths still return
   before it, as in GNU.

Pinned by `replace_region_contents_undo_restores_the_original_text`,
`replace_region_contents_undo_list_shape_matches_gnu` and
`replace_region_contents_records_gnu_undo_boundary_before_the_change_runs`
(`neovm-core/src/emacs_core/builtins/replace_region_contents_test.rs`). The
shape pin is the stronger of the three: it asserts GNU 31.0.90's exact
`buffer-undo-list` for the reduction and for four neighbouring diff shapes,
so a fix that restores the text by accident cannot pass.


## 48. An error message loses a text property that GNU keeps — HALF FIXED

Reduced below the package at last, into TWO independent links. The first is
fixed; the second is filed as entry 57 and still fails the suite.

**Link 1 (FIXED): `format`'s `%s` printed a symbol instead of using its NAME.**
GNU `styled_format` (editfns.c) replaces a SYMBOL argument with `SYMBOL_NAME
(arg)` -- the symbol's actual name STRING object -- before it decides anything
else, so from that point the argument IS a string and its text properties
propagate into the result like any string argument's. We printed the symbol
afresh, which has no property origin.

```elisp
(let* ((name (propertize "p15zz" 'foo 'bar)) (sym (intern name)))
  (list (text-properties-at 0 (format "%s" sym))
        (text-properties-at 0 (format "%S" sym))))
;; GNU          => ((foo bar) nil)
;; Neomacs, was => (nil nil)
```

`%S` carries nothing in GNU either -- it prints a fresh representation -- so the
two conversions must differ, and the test pins both. Pinned by
`format_percent_s_takes_a_symbols_properties_from_its_name_string`
(neovm-core/src/emacs_core/builtins/strings_test.rs). The whole error chain --
`format`, `format-message`, `error`, `error-message-string` -- now matches GNU
for a symbol interned from propertized text.

**Link 2 (OPEN, entry 57): `intern` drops the properties of a MULTIBYTE name.**
The package's symbol comes from buffer text, which is multibyte, and that is the
case still broken. See entry 57 for the reduction.

Because link 2 is still open, the suite still reports `:message-properties nil`.

Was:


`elisp-slime-nav`'s "Don't know how to find X" error carries `(fontified nil)`
on the symbol name in GNU, inherited from the buffer text the symbol was read
from. Neomacs's message carries no properties there.

**Not reduced.** Every step probed separately agrees between the editors:
`find-file-noselect` gives buffer text with `(fontified nil)`,
`thing-at-point 'symbol` keeps it, `format`, `concat` and `format-message`
propagate it, and `error-message-string` on `(error STRING)` returns the string
object itself. So the property is lost somewhere inside `elisp-slime-nav`'s own
path between reading the symbol and signalling, and finding it needs the package
loaded.

Affects: `elisp-slime-nav` (1).
`stale_symbol_failure_returns_to_the_caller_and_records_xref_forward_history`
reports `:message-properties nil` against GNU's `(fontified nil)`. The message
*text* is identical, so only the property is at stake.


## 49. `command-history` records raw argument values

`call-interactively` pushed the resolved arguments onto `command-history`
verbatim. GNU applies three transformations first, and neomacs applied none, so
no entry replayed through `repeat-complex-command` the way GNU records it.

```elisp
(defun d (arg) (interactive "P") arg)
(defun p (pos) (interactive "d") pos)
(defun r (b e) (interactive "r") (list b e))
(defun s (x) (interactive (list 'a-symbol)) x)
(setq command-history nil)
(let ((current-prefix-arg '(4))) (call-interactively 'd t))
(with-temp-buffer (insert "hello") (set-mark 2) (goto-char 4)
                  (call-interactively 'p t) (call-interactively 'r t))
(call-interactively 's t)
(mapcar #'prin1-to-string (reverse command-history))
;; GNU     => ("(d '(4))" "(p (point))" "(r (region-beginning) (region-end))" "(s 'a-symbol)")
;; Neomacs => ("(d (4))"  "(p 4)"       "(r 2 4)"                             "(s a-symbol)")
```

Three separate rules, all missing:

- `quotify_arg` (`callint.c:127`) wraps an argument in `quote` unless it already
  evaluates to itself -- conses, and symbols other than nil and t. Numbers,
  strings and vectors stay bare.
- the `varies` array (`callint.c:447`, read at `:781`) records an argument that
  came from point, the mark or the region as a call to the function that
  produced it, so replaying re-reads the *current* position. Which form an
  argument takes is fixed by its spec letter, not by its value: `R` records the
  `use-region-*` calls even when the region was inactive and both arguments came
  out nil.
- `fix_command` (`callint.c:175`), for a Lisp-form spec only, substitutes the
  replacements a command declares in its `interactive-args` property and then
  drops trailing nil optional arguments, skipping that trim for a `&rest`
  function whose arity has no fixed maximum.

Affects: `counsel` (1).
`command_palette_remaps_m_x_executes_prefix_commands_and_records_history`
reads `command-history` back through `prin1-to-string` and got
`"(neomacs-counsel-test-deploy (4))"` against GNU's
`"(neomacs-counsel-test-deploy '(4))"`.

FIXED in `neovm-core/src/emacs_core/interactive.rs`. Kept as the regression
record.


## 50. `error-message-string` keeps text properties GNU strips

```elisp
(let ((s (propertize "SYM" 'p 1)))
  (mapcar (lambda (data) (error-message-string data))
          (list (list 'error s) (list 'user-error s)
                (list 'error s "extra") (list 'file-error s "more"))))
;; GNU     => (#("SYM" 0 3 (p 1))  "SYM"                "SYM: \"extra\""                "SYM: more")
;; Neomacs => (#("SYM" 0 3 (p 1))  #("SYM" 0 3 (p 1))   #("SYM: \"extra\"" 0 3 (p 1))   #("SYM: more" 0 3 (p 1)))
```

GNU's `Ferror_message_string` (`print.c:1046-1061`) short-circuits an error of
exactly the shape `(error STRING)` and hands back that very string object,
properties and all. Every other shape is rendered by `print_error_message` into
`prin1-to-string-buffer` one character at a time -- a route that carries no
properties -- and read back with `buffer-string`. So the fast path is the only
way a property reaches the caller, and the first column above is the only one
where the two editors agreed.

Affects: `dumb-jump` (1).
`a_missing_definition_reports_the_real_xref_error_without_moving_the_user` built
its `user-error` from buffer text carrying dumb-jump's own `:dumb-jump-ctx`
property, which survived into the reported message where GNU reports plain text.

FIXED in `neovm-core/src/emacs_core/errors.rs`. Kept as the regression record.


## 51. `replace-match` refuses match data that did not come from a buffer

```elisp
(with-temp-buffer
  (insert "aa 01 bb")
  (set-match-data (list 4 6))
  (replace-match "999" t t nil 0)
  (list (buffer-string) (match-beginning 0) (match-end 0)))
;; GNU     => ("aa 999 bb" 4 7)
;; Neomacs => signals (args-out-of-range 0)
```

`set-match-data` called with plain integers records the match as
*string-sourced* in both editors -- GNU sets `last_thing_searched` to `Qt`
unless an element names a buffer or is a marker carrying one
(`search.c:2966`). Neomacs's `replace-match` consulted that flag and refused,
signalling `args-out-of-range` with a bare `0`, an error shape GNU never
produces. It therefore failed for **every** subexpression, and only worked when
a caller happened to pass markers.

GNU's `Freplace_match` (`search.c:2396`) never consults `last_thing_searched`.
A nil STRING argument means the current buffer, full stop. It validates the
subexpression against the accessible portion and reports both endpoints
(`search.c:2418-2427`):

```elisp
(with-temp-buffer (insert "abc") (set-match-data (list 2 99))
                  (replace-match "X" t t nil 0))
;; GNU => (args-out-of-range 2 99)
```

Two further consequences hid behind the refusal. The buffer replacement path
used the same flag to choose between slicing the match and copying the whole
buffer, and the whole-buffer branch misread 1-based buffer positions as 0-based
string offsets. And the post-replacement register adjustment (GNU's
`update_search_regs`) skipped string-sourced data, leaving the registers stale
after the edit -- so once the refusal was removed, a caller replacing in a loop
read the same positions forever and never terminated.

Blast radius: any package that computes a region itself, installs it with
`set-match-data` and calls `replace-match`. That is the normal way to reuse the
replacement machinery without running a search.

Affects: `evil-numbers` (5, all cases), through `shift-number`.

FIXED in `neovm-core/src/emacs_core/builtins/search.rs`,
`.../search.rs` and `.../regex.rs`. Kept as the regression record.


## 52. `define-fringe-bitmap` does not register the symbol in `fringe-bitmaps`

```elisp
(define-fringe-bitmap 'p9 [0 0 24 24 0 0] nil nil 'center)
(list (and (memq 'p9 fringe-bitmaps) t) (get 'p9 'fringe))
;; GNU     => (t 25)
;; Neomacs => (nil 25)
```

The `fringe` property was set but the symbol never joined the list Emacs
documents as "List of fringe bitmap symbols", and `destroy-fringe-bitmap` never
removed one. GNU does both next to the property: a newly named bitmap is consed
onto `Vfringe_bitmaps` under the same "not already known" test that assigns its
index (`fringe.c:1620-1656`), which is why redefining does not list it twice,
and destroy unregisters only indices at or above the standard range
(`fringe.c:1409-1414`), so a standard bitmap keeps both its listing and its
property.

Affects: `vi-tilde-fringe` (1), which checks
`(memq 'vi-tilde-fringe-bitmap fringe-bitmaps)` to confirm registration.

FIXED in `neovm-core/src/emacs_core/builtins/stubs.rs`. Kept as the regression
record.


## Behaviour that is NOT a divergence

Recorded so nobody re-investigates them:

- `ac-capf` snapshots contain `(void-variable arg)`. That is an upstream bug in
  `ac-capf.el` (`(substring arg 0 base-size)` with `arg` never bound, reached by
  any capf with a non-zero completion base size), reproduced byte-identically by
  both editors.
- `ace-link`'s minor-mode dispatch table never matches, because `ace-link`
  applies `bound-and-true-p` to the loop variable's own name; and its org
  invisible-link filter uses `outline-invisible-p`, which does not know about
  modern org-fold. Both are upstream bugs, identical in both editors.
- `read-key` and `read-key-sequence` DO receive keys from a running keyboard
  macro in Neomacs. Divergence 2 is specific to the `read-char` family.
- `achievements` defines `achievements-list-mode-map` with `defvar` after
  `define-derived-mode` has already defined it, so that `defvar` is a no-op and
  the documented `d` binding resolves to `undefined` from the suppressed
  special-mode map. Upstream defect, identical in both editors.
- `agtags`' save-time database update never runs. `agtags--auto-update` builds
  its argument as `(concat "--single-update=" buffer-file-name)` *inside*
  `with-temp-buffer`, where `buffer-file-name` is nil, so GNU GLOBAL is handed a
  bare `--single-update=` with no path. Real GNU GLOBAL 6.6.14 answers that with
  exit 1 and `gtags: path '<cwd>' is out of the project.`, indexing nothing — so
  `agtags-mode`'s headline feature is inert, and an edited file's new
  definitions never become findable however often it is saved. The guard
  `(and agtags-mode buffer-file-name …)` reads the name in the right buffer,
  which is why the hook still fires and still clears the tag history and the
  completion cache. Upstream defect, identical in both editors.
- `agtags-open-file` cannot open a file completed by base name. GNU GLOBAL's
  `-c -P` completes on path *components*, so `global -c -P main` answers
  `main.c` where `global -c -P inc` answers `include/parser.h`; agtags expands
  whatever comes back against the project root, so picking `main.c` visits a
  `<root>/main.c` that does not exist and the user gets an empty buffer.
  Upstream, identical in both editors.
- `alan-mode` never `require`s `thingatpt`, but
  `alan-documentation-include-link-p` calls `thing-at-point-looking-at`, which
  the library does not autoload. In a bare session `C-c '`-adjacent
  documentation-link handling signals `(void-function
  thing-at-point-looking-at)`; a full session survives because
  `thing-at-point` *is* autoloaded and the package's own xref backend calls it,
  which loads the library as a side effect. Upstream defect, identical in both
  editors, and asserted from both sides in the suite.


## 53. A string object's `syntax-table` property never reaches the regexp matcher -- FIXED

FIXED 2026-08-09. `string-match`, `string-match-p` and `posix-string-match` now
set up syntax state for the STRING object the way GNU's
`RE_SETUP_SYNTAX_TABLE_FOR_OBJECT` does: the current buffer still supplies the
base table (GNU calls `SETUP_BUFFER_SYNTAX_TABLE` whatever the object is) and
the current buffer's `parse-sexp-lookup-properties` is still the gate, but the
positional property now comes from the searched string's own intervals,
resolved through the same `CharPropertyResolver` (`textget`) the buffer half
uses -- so a `category`, a `char-property-alias-alist` entry and
`default-text-properties` all resolve on a string exactly as they do in a
buffer. The string run cache (`StringSyntaxPropByteRun`) is the string sibling
of the buffer's `SyntaxPropByteRun`, sharing `PropRunCells` and the
`SyntaxProperties` vocabulary, and converts the matcher's byte offsets to
interval characters with the string's own mapping (GNU
`RE_SYNTAX_TABLE_BYTE_TO_CHAR`).

Pinned by 11 tests in
neovm-core/src/emacs_core/syntax_gnu_parity_regression_test.rs, every
expectation measured on GNU 31.0.90: both reductions below, the
alias-alist form, a property that takes syntax AWAY, the gate off, the gate
read from the current buffer rather than the string's origin, a multibyte
character ahead of the propertized one (the byte-to-character mapping), a
propertyless string, `default-text-properties` reaching an interval that
exists but says nothing about syntax -- and NOT reaching a string with no
intervals at all -- and buffer matching left alone.

Cost: the propertyless string, which is very nearly every `string-match`, is
decided before any of this is built -- a string with no intervals has no
property at any position, so it is tested ahead of the
`parse-sexp-lookup-properties` read and runs the same position-free
per-character path as before. Measured (instructions:u, three interleaved
pairs): TTY typing +0.02% and the entry-44 syntax sweep -0.05%, both noise; a
`string-match`-only microbench +0.14%, whose startup half is unchanged
(551.0M both sides), so the residue is roughly 60 instructions per call on the
seam itself.

Entry points: the three Lisp-visible ones above. The internal
`fast_string_match` analogues (completion, dired, file-name handlers) still
match strings under `DefaultSyntaxLookup`; GNU honours properties there too,
but those call sites do not even use the buffer's syntax table today, which is
a separate and older gap.

The reduction below is kept as the regression record.

Found while fixing entry 44, and orthogonal to it: it is not about the
`category` indirection but about strings carrying properties at all. GNU sets
up its syntax state for a string object (`RE_SETUP_SYNTAX_TABLE_FOR_OBJECT`,
`src/syntax.c:277`), so `string-match` on a propertized string honours the
positional `syntax-table` property exactly as a buffer search does. Neomacs's
`BufferSyntaxLookup` is documented as deliberately position-independent for
string input, so the property is never consulted.

```elisp
(let ((parse-sexp-lookup-properties t)
      (s (concat "a" (propertize "<" 'syntax-table '(4 . ?>)) "b")))
  (string-match "\\s(" s))
;; GNU     => 1
;; Neomacs => nil
```

The `category` form of the same probe diverges identically, and for the same
reason -- the property is not read, so nothing resolves it:

```elisp
(put 'p12-open 'syntax-table '(4 . ?>))
(let ((parse-sexp-lookup-properties t)
      (s (concat "a" (propertize "<" 'category 'p12-open) "b")))
  (string-match "\\s(" s))
;; GNU     => 1
;; Neomacs => nil
```

Not yet attributed to a package suite. The buffer half of this path is fixed
(entry 44 pins `re-search-forward` resolving a category in a buffer); a fix
here means giving the string matcher the same
`SyntaxProperties`/`CharPropertyResolver` seam over the string's own
`TextPropertyTable`.

Found while fixing this, and split out as entry 55: on the BUFFER side
`default-text-properties` does not reach a position whose interval exists but
carries no `syntax-table`. That is a different mechanism -- interval coverage
rather than property resolution -- so it did not travel with this fix.

## 54. An implicit `:stderr` pipe is not `closed` when the owner's sentinel runs -- FIXED

FIXED 2026-08-09. GNU retires a pipe connection in two separate phases, and
neomacs was doing both in one place, or neither.

The fd-scan loop gives a pipe connection its terminal status the moment a read
returns 0 -- `tick++`, `deactivate_process`, and `(exit 0)` if it was still
running (`src/process.c:6072-6080`), which `process-status` reports as `closed`
for a pipe (`src/process.c:1193`). The SENTINEL is not run there: that happens
later in `status_notify` (`src/process.c:7873`), which the fd loop calls only
once it has finished scanning, and which walks the process alist newest-first
-- so the owner of an implicit `:stderr` pipe, created after it, is notified
first and sees the pipe `closed` but still attached.

neomacs had two paths and both diverged. The drain used during a targeted wait
consumed stderr bytes and fell out of its loop on EOF without recording
anything, so the pipe stayed `open`; the wait loop's own stderr-EOF branch went
to the other extreme and ran the pipe's sentinel and removed it from the alist
inline, so the owner's sentinel then found `get-buffer-process` nil. Both now
go through one retire step that sets the status and defers the notification,
and a pipe whose notification is pending yields to its owner's, reproducing
GNU's newest-first order.

This is Lisp-visible because `process-kill-buffer-query-function`
(`lisp/subr.el:3542`) prompts only for a process whose status is one of
`run stop open listen`. Magit's blame sentinel kills the stderr buffer, so on
`open` it prompted; in batch that prompt reads EOF from stdin, the sentinel
signals, and the session exits 255.

```elisp
(let* ((out (generate-new-buffer " *o*"))
       (err (generate-new-buffer " *e*"))
       (done nil) (seen nil))
  (let ((p (make-process :name "s" :buffer out :stderr err
                         :command (list "sh" "-c" "printf 'e' 1>&2; printf 'x'"))))
    (set-process-sentinel
     p (lambda (pr _e)
         (when (memq (process-status pr) '(exit signal))
           (let ((sp (get-buffer-process err)))
             (setq seen (if sp (process-status sp) 'GONE)))
           (setq done t))))
    (while (not done) (accept-process-output nil 0.05))
    seen))
;; GNU     => closed (12/12)
;; Neomacs => GONE 10/12, open 2/12, closed 0/12  (before this fix)
```

The sentinel ORDER diverged for the same reason, and is the same fix:

```elisp
;; owner sentinel and stderr-pipe sentinel, in the order they run
;; GNU     => (owner stderr)
;; Neomacs => (stderr owner)   (before this fix)
```

Pinned by `the_stderr_pipe_is_closed_and_attached_when_the_owner_sentinel_runs`
and `the_stderr_pipe_sentinel_still_runs_after_the_owners` in
neovm-core/src/emacs_core/process_test.rs. The first runs twelve iterations
over both empty and non-empty stderr payloads because the divergence was
timing-dependent; the second exists because retiring the pipe early must not
cost it the notification GNU still delivers.

Discovered as the cause of the `magit_package_batch` flake (blame cases). On
ten serial runs of that suite the neomacs-side failure went from 8/10 to 0/10,
and the instrumented real Magit case now reports GNU's exact line,
`(:kill " *git-stderr*" :proc "git stderr" :status closed :live nil :query t)`,
24/24. Two of those ten runs still fail, but on the GNU BASELINE side -- real
GNU Emacs 31.0.90 also prompts there under load -- which is an environmental
flake in the harness, not a neomacs divergence.

### GNU-side residue, re-measured 2026-08-11 -- still GNU-only, and worse

The "two of ten" above is stale. On the same ten-serial-run protocol, on a
machine loaded by concurrent builds, `magit_package_batch` failed **6 of 10**
runs -- every one of them on the GNU baseline, in
`magit_blame_cycle_style_rewrites_real_blame_details_for_every_visualization`,
and **0 of 10** on the neomacs side. Twenty further runs across two protocol
rounds never produced a neomacs-side failure. The neomacs fix above holds; only
the GNU-side residue remains, and its rate scales with machine load.

The failure is GNU exiting 255 during `RestartProbe`. Magit's process sentinel
kills `" *git-stderr*"` while the stderr pipe process is still live, so
`process-kill-buffer-query-function` asks `Buffer " *git-stderr*" has a running
process; kill it? (yes or no)`; in batch that reads a stdin already at EOF and
dies with `End of file during parsing: Error reading from stdin`.

Three levers were assessed and all three rejected:

- **Raise the `RestartProbe` readiness allowance for the GNU side.** Wrong
  diagnosis. GNU is not slow to become ready -- it aborts immediately on an
  interactive prompt at EOF. No readiness budget changes that.
- **Retry the GNU baseline phase.** Mechanically it would work, and scoped to
  "the GNU phase produced no valid outcome" (never to an outcome *mismatch*) it
  could not hide a neomacs regression. It was still rejected: at a 60% failure
  rate a retry is not papering over a rare blip, it is running the baseline
  until it agrees, and it would keep a loud-but-ignorable log line in front of a
  race nobody then fixes.
- **Bind `kill-buffer-query-functions` to nil in the case.** Tried and
  measured, because it is symmetric across both editors and the case asserts
  blame overlays and styles rather than kill-buffer prompting. It removes the
  prompt -- GNU's stdout comes back clean -- but the rate stays exactly 6 of 10:
  the run then dies as `error in process sentinel: Process git is not active`.
  Reverted, because it changes the symptom without moving the number.

That second failure mode names the real bug. The scenario's own teardown
(`(dolist (process (process-list)) ... delete-process)`) deletes git processes
out from under Magit's still-running sentinels, so the prompt and the dead-
process error are two faces of one race between the teardown and Magit's
cleanup. The fix belongs in the case -- let Magit finish its own reaping, or
wait for the stderr pipe to close before tearing down -- not in a harness-level
timeout or retry.

**The GNU-side residue is CLOSED as of 2026-08-12** (`fe693ec6a`), and closing
it corrected the analysis above in one place. The reason neither lever moved the
number is that the two failure modes are not merely "two faces of one race" in
the loose sense -- they are ordered, and the prompt runs FIRST. Magit's blame
sentinel fires during the scenario body, long before teardown, and in batch the
prompt reads EOF and exits 255 immediately. The session is already dead by the
time the teardown would run. So fixing the teardown alone could not help (it was
never reached), and removing the prompt alone could not either (it only unmasked
the teardown race underneath). Both legs had to go together, which is exactly
what the 6-of-10 plateau was reporting.

Both legs now do, in the case:

- The scenario clears `process-query-on-exit-flag` on the processes it started.
  That is the switch GNU itself provides for this -- `process-kill-buffer-query-
  function` (`lisp/subr.el:3542`) prompts only when the flag is set -- so the
  prompt is answered rather than suppressed, it stays symmetric across both
  editors, and the case can still see prompts it ought to see. This is what the
  rejected `kill-buffer-query-functions` lever should have been.
- Teardown settles before deleting: it waits for the scenario's processes to
  exit AND for the sentinels those exits queued to run, deadline-bounded so a
  genuinely stuck process fails the case instead of hanging it, and only then
  deletes what remains.

That took the rate from 7/10 to 1/10 and exposed a third mode underneath, which
was a pre-existing bug in the case's own waiter rather than a new one. Magit
blames in two phases -- a quickstart process whose sentinel installs a full-file
process, whose sentinel installs the overlays and clears `magit-blame-process`
-- and between the phases the process object is dead while the variable is not
yet nil. Waiting on `process-live-p` could therefore return before a single
overlay existed, tripping the case's own `blame process completed without
deterministic overlays` guard. Cleared-to-nil is the actual completion signal;
liveness had been standing in as a hang guard, and a deadline does that job
without firing on a merely slow run.

Rates, 10x serial under an identical 48-spinner load: 7/10 fail before, 1/10
after the settle, 0/10 after the waiter fix. Ambient load on the host was
falling across the sequence, so the pre-change code was re-run under the later
conditions as a control and still failed at least 2 of 8 (partial run). Idle,
the baseline failed 0/10 -- load is what exposes this, which is why it read as
environmental for so long.

Status: FIXED.

## 55. `default-text-properties` does not reach a buffer position whose interval says nothing — NOT A DIVERGENCE (stale)

**Re-verified 2026-08-10 against GNU 31.0.90 and found already correct**, on the
entry's own reduction and on the whole coverage space around it. This entry was
recorded from the same session that fixed the string side and was never
re-measured against a build carrying that fix; nothing has changed in the
property path since (`git log 67e28525b..` touches it only through the two
commits that closed entry 53).

Both sides now agree everywhere:

| where the unrelated property sits | GNU | neomacs |
|---|---|---|
| on the character being scanned | 2 | 2 |
| on the character before it | 2 | 2 |
| on the character after it | 2 | 2 |
| four characters away | 2 | 2 |
| nowhere in the object | nil | nil |

and the string half of the same matrix answers `(0 0 0 nil)` on both. The last
row is the boundary entry 44 pins, and it holds: where no interval exists
anywhere, neither implementation applies a fallback.

The one thing that was genuinely missing was a test. The buffer side had no
pin at all -- only the string side did -- which is how this could have
regressed silently. The whole matrix above is now pinned by
`an_existing_partition_carries_default_text_properties_to_every_position`
(neovm-core/src/emacs_core/syntax_gnu_parity_regression_test.rs), buffer and
string together, including the no-interval boundary.

Was:

Found while fixing entry 53, and a different mechanism from it: entry 53 was
about which OBJECT the property is read from, this is about which POSITIONS
have an interval to read at all.

GNU's `textget` fallbacks -- `category`, `char-property-alias-alist`,
`default-text-properties` -- apply wherever an interval exists, even one whose
plist says nothing about syntax. GNU's interval tree partitions the whole
object once any property is set anywhere in it, so every position then has an
interval and the fallback applies everywhere. Neomacs's buffer table reports no
interval at a position no property was ever set on, so the fallback is skipped
there.

```elisp
(with-temp-buffer
  (setq-local parse-sexp-lookup-properties t)
  (insert "a<b")
  (put-text-property 2 3 'foo 'bar)
  (let ((default-text-properties '(syntax-table (4 . ?>))))
    (goto-char (point-min))
    (re-search-forward "\\s(" nil t)))
;; GNU     => 2
;; Neomacs => nil
```

The string half of the same probe now agrees with GNU (pinned by
`a_strings_interval_takes_syntax_from_default_text_properties` in
neovm-core/src/emacs_core/syntax_gnu_parity_regression_test.rs), which is what
exposed the buffer asymmetry. Note the boundary entry 44 already pins and this
entry must not cross: where NO interval exists at all -- a string or buffer
with no properties anywhere -- GNU applies no fallback either, because
`update_syntax_table` returns early when `interval_of` finds nothing. The fix
is about coverage of an EXISTING partition, not about synthesizing intervals
for propertyless text.

Not yet attributed to a package suite.


## 56. `key-description` renders a raw 8-bit character as the replacement char -- FIXED

FIXED 2026-08-10. A raw byte now comes back as itself. GNU's
`push_key_description` ends in `CHAR_STRING (c, p)` (keymap.c:2296-2301), and
`char_string` sends the eight-bit range through `CHAR_TO_BYTE8` +
`BYTE8_STRING` (character.c:133-136), so the description holds ONE character
whose code is the raw byte; `Fsingle_key_description` then wraps the buffer with
`make_specified_string (..., multibyte=1)` (keymap.c:2339).

This was a TYPE problem, not a formatting one, which is why it produced a
replacement character rather than a wrong escape. `describe_int_key` and
`describe_single_key_value` returned a Rust `String`, which structurally cannot
hold an eight-bit or non-Unicode Emacs character, so our `emacs_char::char_string`
computed the right bytes and the `to_utf8_lossy` on the next line destroyed them
into U+FFFD. The description pipeline now carries Emacs BYTES to the boundary
and the builtins build a multibyte `LispString` from them, exactly as GNU does;
this is the bug class `EmacsChar` was introduced for (issue #131), so the
encoding step goes through `EmacsChar::char_string` rather than a bare `u32`.
The two internal callers that genuinely want Rust text -- an error message in
`describe_event_sequence` and a `KeyEvent` label for logs -- now decode lossily
at their own call sites, where the loss is visible and harmless, instead of
making the shared builder lossy for everyone.

A second, quieter divergence was measured and fixed with it: GNU's key
descriptions are ALWAYS multibyte, so `(multibyte-string-p (key-description
[?a]))` is `t`, while ours were unibyte for ASCII. `text-char-description` keeps
GNU's own split (`make_string` and unibyte for an ASCII character,
`make_multibyte_string` for anything else, keymap.c:2406-2411), and had the same
U+FFFD bug in its non-ASCII arm.

Measured on GNU 31.0.90 and now matched byte-for-byte: `(key-description
[4194208])` is `"\240"`, length 1, multibyte `t`; `(key-description [4194208
4194303 ?a])` is `"\240 \377 a"`; `(single-key-description 4194208)` and
`(text-char-description 4194208)` are `"\240"`. Genuine multibyte characters are
untouched -- `(key-description [?é])` and `[?中]` still render as themselves --
and `"C-x a"` is unchanged.

The entry's claim that this raw byte was "the whole of the remaining
`describe-bindings` difference (82 lines of 2393)" is WRONG, and checking it end
to end is what showed that. With the encoding fixed, the three
`self-insert-command` rows ARE byte-identical to GNU -- both outputs carry the
same 24 raw bytes and no U+FFFD anywhere -- but the `--batch` diff against GNU
31.0.90 is unchanged at 422 lines. Ignoring tab runs, the real residual is 77
rows GNU emits and we do not, nearly all `touch-screen-*` bindings on
window-part prefixes (`<bottom-divider> <touchscreen-begin>`,
`<right-fringe> <touchscreen-end>`, and so on). Those keys are ~40 columns wide,
which pushes GNU's section into a wider bucket of
`describe-map--align-section`'s 16/24/32 quantization (help.el:1916-1931) and
accounts for the one-tab column shift on every row beneath. So the column is a
CONSEQUENCE of the missing bindings, not of key rendering, and it is a separate
divergence from this entry -- filed rather than fixed here.

A second, genuine `single-key-description` divergence was found while checking
that, and is fixed. GNU treats a cons of two fixnums as an interval from a
map-char-table and renders it `FROM..TO` (keymap.c:2322-2329); it has no
"(MOD . CHAR) modifier event" case at all. We had one, added on the stated
premise that "GNU Emacs treats dotted cons pairs as modifier+character key
events". Measurement says otherwise: `(single-key-description (cons 1 ?x))` is
`"C-a..x"` on GNU and was `"S-x"` here, and `(cons ?a ?z)` is `"a..z"` and was
`"S-s-z"`. It is fixed, and the X11 modifier-bit conversion the false premise
introduced is deleted. In fairness to the honest expectation: this did NOT move
the `describe-bindings` column, which is why the paragraph above says what it
says. Lucid-style event LISTS like `(meta shift up)` are unaffected, since GNU
converts those first and we still do.

Original report follows.

A character in the raw-byte range (`#x3FFF80`..`#x3FFFFF`, the codes Emacs uses
for bytes that decode to nothing) comes back from `key-description` as U+FFFD
instead of as itself.

```elisp
(list (key-description [4194208])
      (mapcar #'identity (key-description [4194208])))
;; GNU     => ("\240" (4194208))
;; Neomacs => ("<?>" (65533))     ; the literal is U+FFFD REPLACEMENT CHARACTER
```

Found while closing entry 45: it is the whole of the remaining
`describe-bindings` difference against GNU (82 lines of 2393). Two rows of the
global map's `self-insert-command` char-table cover raw-byte ranges, and
rendering their endpoints as U+FFFD both prints the wrong characters and
narrows the section's widest key, which shifts `describe-map--align-section`'s
column for every row under it -- so a one-character encoding bug accounts for
hundreds of diff lines.

Not a keymap bug: the corruption is in turning the character into a string, so
anything that prints a raw byte through a key description is affected.

Not yet attributed to a package suite.


## 57. `intern` drops the text properties of a MULTIBYTE name string -- FIXED

FIXED 2026-08-10. `intern` now adopts the string OBJECT it was handed as the
new symbol's name whenever the call CREATES the symbol, for both the global and
a custom obarray, exactly as GNU's `Fintern` -> `intern_driver` ->
`Fmake_symbol (string)` does (lread.c:4705-4708, 4773-4806). `symbol-name`
returns that object, so its text properties, its multibyteness and any later
`aset` on it all survive; an already-interned symbol keeps the name it was
created with, so the argument's properties stay invisible there, as in GNU.

Measured on GNU 31.0.90 rather than assumed, and two of the measurements
changed the fix. First, `(eq s (symbol-name (intern s)))` is `t` in GNU for a
unibyte AND a multibyte name -- GNU shares, it does not copy, so sharing is both
the faithful shape and the free one. Second, GNU does NOT canonicalize an
ascii-only multibyte name:
`(multibyte-string-p (symbol-name (intern (string-to-multibyte "/tmp/HELLO") ob)))`
is `t` for a custom obarray and for the global one alike. Our custom-obarray
path was rewriting such a name to unibyte, and since the reduction above uses
`string-to-multibyte`, that rewrite WAS the property loss. Name IDENTITY is a
separate question that GNU answers on chars and bytes only (`oblookup` never
compares the multibyte flag), which our normalized name atom already reproduces.

That separation is now explicit in the code, because conflating it was also
hiding a latent memory-safety bug. A symbol's name ATOM is process-lifetime and
is what identity, hashing and the `&'static` thread-local name cache use; a
symbol's name OBJECT is an ordinary GC-managed heap string and is what Lisp
observes (`symbol-name`, printing, obarray lookup). `resolve_lisp_string` used
to prefer the name object, which was safe only by accident -- canonical symbols
never had one, so the `&'static` cache never saw a heap pointer. Giving interned
symbols a name object broke that invariant immediately (three tests died on a
`slice::from_raw_parts` UB check against a freed string). `resolve_lisp_string`
is now atom-only and a new `resolve_sym_lisp_name` serves the Lisp-visible name.
`SymbolRegistry::alloc_symbol`'s `Option<SymbolNameValue>` became a named
`NewSymbolName { AtomOnly, LispObject }` so every construction site states what
the name is instead of passing the bare `None` that caused this.

Cost gate (the entry asked for one): no cost. Release fresh-builds of both
sides, `taskset -c 0-15 perf stat -e instructions:u`, 3 interleaved pairs --
`--batch` startup 542,932,328 -> 542,643,463 (-0.053%) and a `cl-macs.el`
byte-compile 3,105,142,573 -> 3,102,882,254 (-0.073%), both marginally negative
and inside run-to-run spread. Structural reason: the reader interns through
`intern_lisp_string(&LispString)` with no Lisp string object in hand, so it stays
on the unchanged `AtomOnly` path; only Lisp-level `intern` calls gain a rooted
name value, and those are not the byte-compile hot path.

`elisp-slime-nav` (the one affected suite, and the remaining half of entry 48)
now passes.

Original report follows.

GNU `Fintern` stores the string it was handed as the new symbol's name, so
`symbol-name` gives that string back with its text properties intact. Neomacs
does this for a unibyte name and loses the properties for a multibyte one.

```elisp
(list (text-properties-at 0 (symbol-name (intern (propertize (string-to-multibyte "p15-mb-alpha") 'fontified nil))))
      (text-properties-at 0 (symbol-name (intern (propertize "p15-ub-alpha" 'fontified nil)))))
;; GNU     => ((fontified nil) (fontified nil))
;; Neomacs => (nil             (fontified nil))
```

`make-symbol` keeps them in BOTH cases, which is the clue: our
`make_uninterned_symbol_with_name_value` stores the exact name VALUE alongside
the symbol, while `intern` stores only the interned name ATOM
(`SymbolRegistry::alloc_symbol` receives `name_value: None`) and the unibyte case
keeps its properties only incidentally. GNU has no such split -- a symbol's name
is always the string object it was created from.

The proposed fix is to make `intern` pass the given string as the new symbol's
name value when it CREATES a symbol, exactly as `make-symbol` does, so both
widths work by construction rather than by accident. It must apply only on
creation: GNU keeps the existing name when the symbol already exists. Note the
cost to weigh first -- that roots one more Lisp string per newly interned symbol
in the registry's GC roots, on the interning path, so it wants a startup and
byte-compile measurement before it lands.

This is the remaining half of entry 48: an `elisp-slime-nav` error message built
with `(error "..." SYM)` should carry the `fontified` property the symbol name
was read from, and does not.

Affects: `elisp-slime-nav` (1).


## 58. The internal `fast_string_match` analogues bypassed the current buffer's syntax state -- FIXED

FIXED 2026-08-10 (task #26, recorded during the entry-53 fix). Entry 53 gave
the Lisp-visible `string-match` family the syntax state GNU gives any matched
object, but the INTERNAL string matchers -- GNU's `fast_string_match_internal`
callers -- still ran under the hardcoded standard classification and never saw
the current buffer at all. GNU arms the same `re_match_object` +
`RE_SETUP_SYNTAX_TABLE_FOR_OBJECT` (`src/syntax.c:277`) machinery for these
too, and `SETUP_BUFFER_SYNTAX_TABLE` takes the base table, the category table
and the `parse-sexp-lookup-properties` gate from the CURRENT BUFFER:

- `completion-regexp-list` filtering in `try-completion`, `all-completions`
  and `test-completion` (`src/minibuf.c:1592` `match_regexps` and the
  `Ftry_completion` candidate loop);
- the same filtering in `file-name-completion` and
  `file-name-all-completions` (`src/dired.c:756`);
- the MATCH argument of `directory-files` and
  `directory-files-and-attributes` (`src/dired.c:311`);
- `find-file-name-handler`'s walk of `file-name-handler-alist`
  (`src/fileio.c:411`).

```elisp
(with-temp-buffer
  (set-syntax-table (copy-syntax-table (standard-syntax-table)))
  (modify-syntax-entry ?z " ")
  (let ((completion-regexp-list '("\\`\\sw+\\'")))
    (all-completions "" '("abc" "zzz"))))
;; GNU 31          => ("abc")   ; buffer-local table decides \sw
;; Neomacs before  => ("abc" "zzz")
```

The same reduction was measured on GNU for every surface above (files abc +
zzz, regexp `\`\sw+\'`: `directory-files` => ("abc"),
`file-name-completion` => "abc", handler alist entry matches "abc" and not
"zzz"), and for the searched string's own `syntax-table` text properties,
which GNU honours here under `parse-sexp-lookup-properties` exactly as in
entry 53 (a `'(6)` property on a candidate removes it from `all-completions`
only when the gate is on).

The fix routes every internal caller through one seam,
`FastStringMatchSyntax` (neovm-core/src/emacs_core/builtins/search.rs): an
owned, `Copy` snapshot of GNU's `fast_string_match_internal` setup -- current
buffer's syntax table, category table, word-boundary state and
`parse-sexp-lookup-properties` gate -- whose only constructor takes the
evaluator, so an internal matcher without buffer state is unrepresentable.
Matching goes through the entry-53 `StringSyntaxLookup` vocabulary, so string
properties resolve through the same `CharPropertyResolver` as everywhere
else; the per-string interval test still precedes any resolver work, keeping
the propertyless case position-free. The old `DefaultSyntaxLookup` wrappers
are now `#[cfg(test)]`, so no production caller can silently bypass again.
The `keep-lines`/`flush-lines` per-line matcher (a string-shaped port of what
GNU does as a buffer search) was routed through the same seam. Known
remaining bypass, out of scope here: the `iterate_string_matches`
cluster behind the `query-replace`/`how-many` Rust ports still compiles under
the default lookup and deserves its own pass (GNU runs those as buffer
searches).

Pinned by 6 tests (minibuffer_test.rs, dired_test.rs, fileio_test.rs), every
expectation measured on GNU 31.

Affects: found by audit during entry 53, not by a package failure; the
exposed surfaces (completion filtering, dired, file handlers) are
package-facing.
## 59. The global map is missing the `touch-screen-*` window-part bindings -- FIXED

`describe-bindings` emits 77 rows that GNU does, and we do not. Nearly all of
them bind a touch-screen event on a window-part prefix:

```elisp
;; GNU has, and neomacs does not:
;;   <bottom-divider> <touchscreen-begin>   touch-screen-translate-touch
;;   <bottom-divider> <touchscreen-end>     touch-screen-translate-touch
;;   <right-fringe> <touchscreen-begin>     touch-screen-translate-touch
;;   ... 77 rows over <left-fringe>, <right-fringe>, <left-margin>,
;;   <right-margin>, <header-line>, <mode-line>, <vertical-line>,
;;   <right-divider>, <bottom-divider>, <tab-line> ...
(length (split-string (with-temp-buffer (describe-bindings) ...) "\n"))
;; GNU     => 2402 lines
;; Neomacs => 2320 lines
```

Found while closing entry 56, which had claimed the raw-byte rendering was the
whole of the remaining `describe-bindings` gap. It is not: with entry 56 fixed
the diff against GNU 31.0.90 is unchanged at 422 `--batch` diff lines, and these
missing rows are what is left.

They also explain the column shift entry 56 predicted, by a different route than
that entry assumed. `describe-map--align-section` quantizes a section's key
column to 16, 24 or 32 based on the widest key in it (help.el:1916-1931). These
touch-screen keys are ~40 columns wide, so GNU's global section lands in the
32-column bucket while ours lands in 24 -- one tab narrower on EVERY row of the
section. So the alignment difference is a consequence of the missing bindings,
and fixing them should close the column and most of the 422 lines at once.

Not a rendering bug: the keys we do emit now render byte-identically to GNU,
including the raw-byte ranges (entry 56).

Not yet attributed to a package suite.

## 60. Aborting a minibuffer signalled plain `quit` instead of `minibuffer-quit` -- FIXED

FIXED 2026-08-10 (task #35), found by the `ido_vertical_mode` suite, whose
entire diff was one atom: `(:signal quit)` where GNU records
`(:signal minibuffer-quit)` after a `C-g` in an ido prompt.

Since Emacs 28 a minibuffer abort signals `minibuffer-quit`, a condition that
inherits from `quit` (`(minibuffer-quit quit)`, `PUT_ERROR` at
`src/data.c:4125`) so existing `quit` handlers still catch it, while packages
that want to tell "the user cancelled the prompt" apart from "the user
interrupted a computation" can dispatch on it. We already defined the
condition correctly and already vendored the Lisp that raises it; what was
missing was the route between them.

GNU never throws `exit` from `abort-minibuffers` itself. `Fabort_minibuffers`
(`src/minibuf.c:472`) resolves the current buffer's minibuffer level, confirms
with `yes-or-no-p` when aborting also discards nested minibuffers, and then
calls the Lisp `minibuffer-quit-recursive-edit` (`lisp/minibuffer.el:3050`),
which throws a *function* to `exit`:

```elisp
(throw 'exit (lambda () (signal 'minibuffer-quit nil)))
```

`recursive_edit_1` (`src/keyboard.c:749-758`) then dispatches on the thrown
value's TYPE, not its truthiness:

| thrown value | GNU does            | raised by                  |
|--------------|---------------------|----------------------------|
| `t`          | `quit ()`           | `abort-recursive-edit`     |
| a string     | `xsignal1 (Qerror,)`| `read_minibuf` cross-window|
| a function   | `call0 (val)`       | minibuffer abort           |
| anything else| returns normally    | `exit-recursive-edit`      |

Neomacs collapsed all of that into one boolean test -- `if value.is_truthy()`
-> signal `quit` -- and `abort-minibuffers` threw `t` directly, skipping the
Lisp entirely. So a thrown thunk (truthy) became a plain `quit`, and a thrown
string would have too.

The fix restores both halves. `CommandLoopExit`
(neovm-core/src/emacs_core/eval.rs) makes GNU's four outcomes an enum the
match must cover, so no future edit can silently fold a new exit kind into
"truthy means quit"; classification follows GNU's order (`t`, then string,
then function). `builtin_abort_minibuffers_ctx`
(neovm-core/src/emacs_core/minibuffer.rs) now mirrors `Fabort_minibuffers`,
including the `this_minibuffer_depth` level lookup, the "Not in a minibuffer"
and "Not in most nested command loop" errors, and the
`Abort N minibuffer levels?` confirmation before quitting several recursive
edits at once. It delegates to the vendored `minibuffer-quit-recursive-edit`
rather than reimplementing the throw in Rust.

One follow-on came with it: GNU guards its stderr-then-`kill-emacs` branch in
`command-error-default-function` with `!is_minibuffer_quit`
(`src/keyboard.c:1064`), so aborting a minibuffer never takes down a session
that has no frame yet. We had no such guard -- harmless only because
`minibuffer-quit` was unreachable, which this entry changes. Both halves are
mirrored now.

Not display-visible: `error-message` is the string `"Quit"` for both
conditions, so the echo area reads identically. GNU's other asymmetry there is
`Fding (Qt)` instead of `discard-input` + `bitch_at_user`, and we implement
neither side of that ding, before or after -- so it is unchanged, and left as
a separate gap.

Pinned by 3 tests: the abort delegation and the inheritance boundary (a
`minibuffer-quit` handler catches it, a `quit` handler still catches it, an
`error` handler must NOT -- it inherits from `quit` only) in vm_test.rs, the
four-way exit classification in eval_test.rs, and the noninteractive guard in
error_test.rs.

Affects: `ido_vertical_mode` (now green). Any package that dispatches on a
cancelled prompt -- the `ido`, `helm` and `consult` families -- was seeing the
wrong condition.

## 61. describe-map-tree does not autoload-and-descend prefix keymaps -- FIXED

The residue left after entry 59: the batch describe-buffer-bindings diff vs
GNU 31.0.90 is 45 lines, all one mechanism. A prefix key bound to an
AUTOLOADED keymap (C-x C-k kmacro-keymap, C-x 6 and <f2> 2C-command,
C-<down-mouse-2> facemenu-menu) is printed by us as the unexpanded prefix
command row, while GNU's describe_map_tree autoloads the keymap and descends
into its sub-bindings, emitting the kmacro/2C/facemenu rows.

Reduction: compare (describe-buffer-bindings (current-buffer)) output around
"C-x C-k" between editors; GNU emits the kmacro sub-rows, we emit one row.
GNU reference: src/keymap.c describe_map_tree + get_keymap's autoload
handling. Evidence transcripts: the entry-59 worktree's tmp/descb-*.txt.

Status: FIXED (4d3fea4b4): accessible-keymaps lists autoload-keymap symbols
without loading; keymapp matches GNU; the keymap--get-keyelt identity stub
(which emptied menu keymap sections) wired to get_keyelt_runtime. The batch
describe-buffer-bindings diff vs GNU 31.0.90 is now ZERO lines.

## 62. `fringe-bitmaps-at-pos` was a stub that always returned nil -- FIXED

`git-gutter-fringe` puts its hunk indicators in the fringe by hanging an
overlay `before-string` carrying a `(left-fringe BITMAP FACE)` display spec on
each changed line, and the package's own tests read them back with
`fringe-bitmaps-at-pos`. Our builtin validated its two arguments and then
returned nil unconditionally, so every row reported no bitmaps:

```elisp
;; after (git-gutter-mode 1) on a file with three hunks, per line:
(fringe-bitmaps-at-pos (point) (selected-window))
;; GNU     => (git-gutter-fr:modified nil nil)   ; and (nil nil nil) elsewhere
;; Neomacs => nil                                ; every row
```

Everything else in the payload already matched GNU byte-exactly -- window and
frame fringe geometry, the three bitmap registrations, the decoded bitmap
vectors and their sha256, the `display` properties, and the resolved faces --
which located the gap precisely at the introspection builtin rather than in
the display pipeline.

Note the shape: GNU answers a three-element list for every row redisplay laid
out, and nil ONLY when no row contains POS. A row with no bitmaps is
`(nil nil nil)`, not nil.

GNU reference: `src/fringe.c` `Ffringe_bitmaps_at_pos`, which reads the
window's CURRENT MATRIX -- it locates the row with `row_containing_pos` and
returns `(left_fringe_bitmap right_fringe_bitmap overlay_arrow_bitmap)`, the
last decoded as `0 => nil`, `< 0 => t`, `> 0 => the bitmap's symbol`.

The fix follows that structure rather than re-deriving bitmaps from text
properties, which would have missed every indicator redisplay puts in the
fringe itself (truncation, continuation, empty-line, overlay arrows).
neovm-core cannot see the layout engine's matrices, so the three fringe slots
now travel out on the per-window `WindowDisplaySnapshot` that already carries
each row's buffer-position span, and the builtin reads them there -- the same
seam `pos-visible-in-window-p` and `window-line-height` already use.

This also corrected the overlay arrow, which the layout engine had been
stamping into the row's `left_fringe_bitmap` (and suppressing when that slot
was taken). GNU keeps `overlay_arrow_bitmap` as a slot of its own and draws it
in addition to the row's left bitmap, so the two are now independent in the
matrix, in the painter, and in what this builtin reports.

Reduction: `cargo nextest run -p neomacs-melpa-tests -E
'test(git_gutter_fringe_real_gui_rows_match_gnu)' -j1` -- was 19 melpa
failures, now 18.

Status: FIXED.

## 63. Editing a file locked by ANOTHER live Emacs proceeds silently -- FIXED

Divergence #26 fixed lock CREATION; the CONTESTED case was still broken in
three stacked ways, found by running a real GNU Emacs holder against the
neomacs release binary:

1. `lock_file_resolved` called `ask-user-about-lock` but swallowed its result
   with `unwrap_or(nil)`. GNU's `calln` propagates: in batch, userlock.el
   signals `(file-locked FILE "USER@HOST (pid PID)" "Cannot resolve lock
   conflict in batch mode")` and the edit is refused. Neomacs edited the
   buffer anyway.
2. Before even reaching the prompt, neomacs ZAPPED the live GNU lock as
   stale: GNU stamps locks with the utmp BOOT_TIME boot time (gnulib
   boot-time.c), which systemd writes seconds after the kernel btime that
   sysinfo reports, and `current_lock_owner` tolerates only 1s of skew.
   The boot-time SOURCE must be utmp.
3. The opponent string passed to `ask-user-about-lock` was the bare user
   name; GNU passes `USER@HOST (pid PID)`. `file-locked-p` conversely
   reports only USER. Unparseable lock contents are EINVAL (no prompt from
   `lock_file`; a file-error from `file-locked-p`/`unlock-file`), and an
   empty lock file is zapped as a buggy-filesystem leftover.

```
; GNU holder: emacs -Q -batch visiting note.txt, buffer modified, lock held
; second GNU: (insert "EDIT") =>
;   (file-locked ".../note.txt" "exec@Matrix (pid 1775960)"
;                "Cannot resolve lock conflict in batch mode"), buffer clean
; neomacs before fix: file-locked-p => nil (live lock zapped!),
;   insert proceeds, buffer modified, lock stolen
```

Fixed on the ask-user-about-lock rung: propagate the signal, type the answer
as GNU's enum (`LockOwner::{None,Current,Other(LockClasher)}`), read boot
time from utmp first, parse from the last `@` with EINVAL strictness.

Affects: any two-editor workflow; found during P8's lock-race diagnosis.

## 64. `completion-ignored-extensions` was never made special, so every `let` of it bound lexically -- FIXED

GNU installs this variable in C: `syms_of_dired` (`src/dired.c:1206`) declares
it with `DEFVAR_LISP` and initializes it to nil, and `lisp/bindings.el:989`
then `setq`s the real 67-element list. We instead seeded a three-element
placeholder from `eval.rs` with a plain `set_symbol_value` and never called
`make_special`.

`DEFVAR_LISP` is what marks a symbol special. Without it, a `let` around the
variable in a lexical-binding file creates a LEXICAL binding, which the
callees that read the value never see:

```elisp
;; inside a lexical-binding form that let-binds the variable to '(".bak")
(special-variable-p 'completion-ignored-extensions)
;; GNU     => t     ; the let is dynamic, callees observe (".bak")
;; Neomacs => nil   ; the let is lexical, callees observe the global list

(completion-pcm--filename-try-filter '("production.bak" "production.toml"))
;; GNU     => ("production.toml")
;; Neomacs => ("production.bak" "production.toml")
```

This was not one package's bug. Every Elisp caller that rebinds the variable
this way -- `hexl.el`, `comint.el`, `term.el` and `files.el` all do -- was
silently completing against the global list instead of its own.

It surfaced as the `vertico` melpa batch, where it was first read as fixture
pollution because the failing case creates a `configs/production.bak` fixture
on purpose. It is not pollution: `MelpaSandbox::new` makes a fresh tempdir per
case and `run_elisp_oracle_batch` gives each editor its own, so both engines
saw an identical three-file directory. Vertico filters ignored extensions
itself -- `vertico--compute` calls `completion-pcm--filename-try-filter` when
the completion category is `file` -- so the case's `let` of `'(".bak")` was
invisible and `production.bak` stayed in the candidate list where GNU dropped
it (`:total 3` vs GNU's `:total 2`).

What isolated it was that every intermediate layer agreed between the engines:
`file-name-all-completions`, `file-name-completion`, the completion metadata
category, `completion-all-completions` under all four completion styles,
`regexp-opt`, `string-match-p`, and even
`completion-pcm--filename-try-filter` called standalone. Only inside the real
batch run did the two diverge, and the discriminator was
`special-variable-p`.

Fixed by mirroring `syms_of_dired`: a `dired::register_bootstrap_vars` that
sets the variable to nil and calls `make_special`, wired into the bootstrap
registration list in `eval.rs`, with the placeholder deleted so
`lisp/bindings.el` supplies the list exactly as upstream. A bare `Context`
now starts it at nil, which is GNU's real pre-`bindings.el` state, and the
running editor's value is byte-identical to GNU's 67-element list.

Pinned by `completion_ignored_extensions_is_special_like_gnu` in
`neovm-core/src/emacs_core/dired_test.rs`, written red first.

### The same class, swept: 113 more variables are special in GNU but not here

This is a bug class, not a one-off. Comparing `special-variable-p` in both
editors over every name that GNU declares with a C `DEFVAR_*` and that we set
without `make_special`, 113 variables are special in real GNU Emacs 31.0.90
and plain in ours. Any lexical-binding `let` of one of these is invisible to
its callees, exactly as above. Several are routinely let-bound in the wild --
`scroll-margin`, `scroll-conservatively`, `resize-mini-windows`,
`max-mini-window-height`, `face-remapping-alist`,
`write-region-annotate-functions`, `after-insert-file-functions`,
`last-nonmenu-event`, `function-key-map`, `key-translation-map`,
`truncate-partial-width-windows`, `inhibit-load-charset-map`.

Reproduce the inventory by evaluating `special-variable-p` for each name in
both editors and diffing; the full list as measured on 2026-08-11:

`after-delete-frame-select-mru-frame`, `after-insert-file-functions`,
`auto-composition-mode`, `auto-fill-chars`, `auto-raise-tab-bar-buttons`,
`auto-raise-tool-bar-buttons`, `auto-resize-tab-bars`,
`auto-resize-tool-bars`, `auto-save-include-big-deletions`,
`auto-save-interval`, `auto-save-timeout`, `auto-window-vscroll`,
`baud-rate`, `case-symbols-as-words`, `char-code-property-alist`,
`charset-map-path`, `current-iso639-language`,
`current-key-remap-sequence`, `current-time-list`, `debug-on-event`,
`default-frame-scroll-bars`, `default-process-coding-system`,
`defining-kbd-macro`, `delete-terminal-functions`,
`display-monitors-changed-functions`, `double-click-fuzz`,
`double-click-time`, `enable-disabled-menus-and-buttons`,
`extra-keyboard-modifiers`, `face-default-stipple`,
`face-filters-always-match`, `face-font-lax-matched-attributes`,
`face-font-rescale-alist`, `face-ignored-fonts`,
`face-near-same-color-threshold`, `face-remapping-alist`,
`frame-inhibit-implied-resize`, `frame-title-format`, `function-key-map`,
`hourglass-delay`, `hscroll-margin`, `hscroll-step`, `iconify-child-frame`,
`icon-title-format`, `image-cache-eviction-delay`, `image-types`,
`inhibit-load-charset-map`, `inhibit-try-cursor-movement`,
`inhibit-x-resources`, `initial-window-system`, `key-translation-map`,
`last-event-device`, `last-event-frame`, `last-kbd-macro`,
`last-nonmenu-event`, `last-repeatable-command`,
`line-number-display-limit-width`, `load-in-progress`, `main-thread`,
`max-image-size`, `maximum-scroll-margin`, `max-mini-window-height`,
`messages-buffer-name`, `message-truncate-lines`, `meta-prefix-char`,
`minibuffer-follows-selected-frame`, `minibuffer-message-timeout`,
`next-screen-context-lines`, `nobreak-char-display`, `num-input-keys`,
`num-nonmacro-input-events`, `overline-margin`,
`overriding-text-conversion-style`, `polling-period`,
`post-select-region-hook`, `pre-redisplay-function`,
`read-buffer-function`, `read-expression-history`,
`redisplay--inhibit-bidi`, `resize-mini-windows`, `resume-tty-functions`,
`saved-region-selection`, `scroll-conservatively`, `scroll-margin`,
`scroll-step`, `set-auto-coding-function`, `special-event-map`,
`suspend-tty-functions`, `system-uses-terminfo`, `tab-bar-border`,
`tab-bar-button-margin`, `tab-bar-button-relief`, `tab-bar-truncate`,
`terminal-frame`, `this-original-command`, `timer-idle-list`, `timer-list`,
`tool-bar-border`, `tool-bar-button-margin`, `tool-bar-button-relief`,
`tool-bar-max-label-size`, `translation-hash-table-vector`,
`truncate-partial-width-windows`, `tty-defined-color-alist`,
`tty-erase-char`, `underline-minimum-offset`, `unicode-category-table`,
`void-text-area-pointer`, `window-point-insertion-type`,
`write-region-annotate-functions`, `write-region-annotations-so-far`,
`write-region-post-annotation-function`, `yes-or-no-prompt`.

Reduction: `cargo nextest run -p neomacs-melpa-tests -E 'test(vertico)'` --
both the parity batch and the TUI case now pass.

Status: FIXED (this variable). One sibling, `tty-erase-char`, was fixed
separately (12f923e21) because it was also carrying a wrong VALUE, not only a
missing declaration -- see entry 66.

### Sweep status: ALL 112 remaining siblings FIXED

Every remaining variable in the list above is now registered through
`Obarray::define_special_variable` at its owning subsystem's bootstrap site
(keyboard/pure.rs, xdisp.rs, frame_vars.rs, window_cmds, xfaces, fileio,
composite, eval.rs), each with GNU's C DEFVAR initial value read from the
`syms_of_*` source, not from recall. The per-variable audit found seven
carrying a neomacs-invented or wrong VALUE on top of the missing declaration:

- `polling-period`: fixnum 2 -> float 2.0 (keyboard.c:13869 `make_float`)
- `debug-on-event`: nil -> `sigusr2` (keyboard.c:14358)
- `maximum-scroll-margin`: fixnum 25 -> float 0.25 (xdisp.c:38541)
- `tool-bar-max-label-size`: 10 -> 14 (DEFAULT_TOOL_BAR_LABEL_SIZE,
  dispextern.h:3494)
- `resize-mini-windows`: bootstrap `grow-only` -> nil (GNU inits nil so
  loadup can run before window.el; lisp/loadup.el:142 then assigns
  `grow-only`, which our loadup also does)
- `redisplay--inhibit-bidi`: a conflicting nil seed deleted; bootstrap is t
  (xdisp.c:39235), loadup.el flips it off after charprop loads
- `frame-inhibit-implied-resize`: nil -> `(tab-bar-lines tool-bar-lines)`
  (frame.c:7636, the own-drawn-tool-bar GUI branch)

Pinned by the five per-subsystem tables in
`neovm-core/src/emacs_core/gnu_defvar_special_test.rs`, which assert
`special-variable-p` AND the GNU default for all 112, plus a let-through-callee
behavioral pin.

## 65. A row going wholly blank was cleared with DCH instead of EL -- FIXED

`magit_log_buffer_file_margin_columns_match_gnu_full_screen` failed 5/5 in the
main checkout and passed 5/5 in a worktree at the same commit. One row, always
row 44, always the full width: GNU `contents=""` against neomacs
`contents=" "`. The test's GNU-side `expect!` pin matched byte-for-byte in both
environments, so GNU was never the varying side.

The raw PTY byte streams named the mechanism. GNU clears the row with `ESC[K`.
Neomacs emitted `ESC[45;1H ESC[8P ESC[45;153H` plus eight literal spaces -- a
DCH(8) and a tail fill. In the terminal model `DCH` shifts *written* blanks in
where `EL` leaves the cells unwritten, which is exactly the `""` vs `" "` the
harness reports, and it costs more bytes than the erase it replaced.

The environment sensitivity was never in the code. The `*Warnings*` buffer
prints `Missing 'lexical-binding' cookie in "<sandbox path>/magit-startup.el"`,
that line wraps at column 160, and the checkout root's path length decides
where. In the main checkout the continuation remnant on row 44 is exactly
`tup.el".` -- eight characters, the 8 in `ESC[8P`. A worktree's longer
`.claude/worktrees/...` path moves the wrap, the remnant stops matching a
shift, the row takes the erase path, and the case passes. Both prior 5/5
observations were correct readings of one path-length-sensitive planner bug.

GNU reference: `update_frame_line` (src/dispnew.c:5960-6240) strips trailing
spaces from BOTH the old (`olen`) and the new (`nlen`) row before it computes
`begmatch`/`endmatch`, so blanks past a row's logical end never count toward
what an insert/delete-char saves. Traced for this case: `nlen` becomes 0,
`osp == nsp == 0`, `begmatch == 0`, `endmatch == 0`, no `delete_glyphs`,
nothing written, and control reaches `just_erase` ->
`clear_end_of_line`. GNU structurally cannot pick DCH here.

Our `detect_row_shift` instead matched over the full PHYSICAL row width, where
a run of default blanks trivially matches any other run of default blanks, so a
wholly-blank desired row matched as a left shift of exactly its old content's
width -- and the shift path is ordered before the erase-to-EOL branch, so `EL`
never got a chance.

Reduction: capture the neomacs PTY stream for the case and look for `ESC[8P` on
the row the diff names; or, without magit, drive the planner directly --
`neomacs-display-runtime` `rif_test.rs`
`a_row_going_wholly_blank_erases_instead_of_shifting` sets a row to
`tup.el".`, renders, sets it to all blanks, and asserts the second render
contains `ESC[K` and not `ESC[8P`.

Status: FIXED (65d419e79). Measure the saving over CONTENT rather than cells:
a shift must preserve a run carrying at least one cell that is not a default
blank (`carries_shiftable_content`), which is what GNU's trailing-space
stripping accomplishes. A space carrying a background is real content and still
shifts, matching GNU's `colored_spaces_p`, so genuine content shifts keep the
ICH/DCH path and the branch order is untouched. neomacs-display-runtime
1005/1005 with every issue-206 shift/erase pin intact; neomacs-tui-tests
905/905. The other five melpa `tui_parity` failures were measured by stashing
the fix and rebuilding -- six before, the same five after -- and are unrelated.

## 66. `tty-erase-char` was frozen at 0 instead of the terminal's ERASE byte -- FIXED

GNU splits this variable across two files. `syms_of_keyboard`
(src/keyboard.c:13925) declares it with `DEFVAR_LISP` under the literal comment
"This variable is set up in sysdep.c", and `init_sys_modes` supplies the value:
`Qnil` to start (src/sysdep.c:1112), then `tty.main.c_cc[VERASE]`
(src/sysdep.c:1130) read from the termios it saved BEFORE touching the terminal
modes.

Neomacs did neither half. `keyboard/pure.rs` seeded a plain `fixnum(0)` with
`set_symbol_value` and never called `make_special`, so the variable was neither
GNU's off-terminal `nil` nor any real terminal's ERASE byte, and a `let` around
it in a lexical-binding file bound lexically -- the same missing-`DEFVAR_LISP`
class as entry 64.

The value is load-bearing rather than informational.
`normal-erase-is-backspace-setup-frame` (lisp/simple.el:11093) enables the mode
on a tty when `(eq tty-erase-char ?\^H)`, and the mode then
`key-translate`s `C-h` to `DEL` (lisp/simple.el:11178) so Backspace deletes a
character instead of opening the help prefix. Frozen at 0 that comparison could
never be true, so on a terminal whose `stty erase` is `^H` neomacs opened help
where GNU deletes.

Reduction: `emacs -nw` and `neomacs -nw` in the same terminal, then compare
`tty-erase-char` via `M-:`. GNU reports the byte `stty -a` calls `erase`;
neomacs reported 0. In batch, GNU reports `nil` and neomacs reported 0.

Why no existing case caught it: the TUI harness's pty reports `^?` (127), so
`normal-erase-is-backspace-mode` stays OFF on both engines and they agree on
every visible behaviour. `find_file_minibuffer_ctrl_h_does_not_delete_previous_character`
pins exactly that agreement. The divergence only becomes visible on a `^H`
terminal, which the suite never constructs.

Discovered while refuting a suspected `C-h C-g` regression (task #42, NOT a
divergence: GNU binds `g` in `help-map`, not `C-g`, so `C-h C-g` is undefined on
both engines and the interactive sequence quits at the command loop).

Status: FIXED (12f923e21). Mirror GNU's split rather than collapsing it:
`keyboard/pure.rs` declares `nil` plus `make_special`; `neomacs-bin`'s
`tty_init.rs` gains `detect_tty_erase_char` (a `tcgetattr` read of
`c_cc[VERASE]` on stdin, `None` when stdin is not a terminal) and
`tty_erase_char_value` mapping `Some(byte)` to a fixnum and `None` to `nil`;
`main.rs` publishes it in the live-tty startup branch, which already runs before
`tty_init_terminal` enters raw mode, so like GNU the byte describes the user's
`stty` setting rather than the modes we impose. Red-first at three levels: the
neovm-core declaration test, the neomacs-bin mapping test, and an end-to-end
pty pin (`tty_erase_char_reports_the_terminals_stty_erase_like_gnu`) that read
the variable through `M-:` and failed on the neomacs side ALONE. neovm-core
8847/8847; neomacs-tui-tests 906/906.

## 67. `normal-erase-is-backspace` latches OFF because of an invented terminal-parameter default -- FIXED

Found by the `^H`-erase pin added with entry 66, which is the first case in the
suite to run on a terminal whose `stty erase` is Backspace rather than `^?`.

On such a terminal GNU deletes a character for the `0x08` the Backspace key
sends; neomacs opens the help prefix instead, leaving the character in place.
Measured through `M-:` on both engines, on the same pty, at the same moment:

```elisp
(list tty-erase-char
      (terminal-parameter nil 'normal-erase-is-backspace)
      (and (char-table-p keyboard-translate-table)
           (aref keyboard-translate-table 8)))
;; GNU     => (8 1 127)
;; Neomacs => (8 0 nil)
```

`tty-erase-char` AGREES -- entry 66 fixed that. What differs is the terminal
parameter: GNU decided 1 (mode on, `C-h` translated to `DEL`), neomacs decided
0 and never revisited it.

The decision is not wrong on its inputs; it was made too early. Every input
`normal-erase-is-backspace-setup-frame` (lisp/simple.el:11093) consults is
IDENTICAL across the two engines when queried at the prompt:

```elisp
(list window-system noninteractive normal-erase-is-backspace
      (eq tty-erase-char 8) (display-symbol-keys-p))
;; GNU     => (nil nil maybe t nil)
;; Neomacs => (nil nil maybe t nil)
```

So on today's state the condition would enable the mode on both. The function
is guarded by `(unless (terminal-parameter nil 'normal-erase-is-backspace)
...)` (lisp/simple.el:11097), and a latched `0` is non-nil, so once the
parameter exists the decision is never made again. Clearing it and re-running
the function makes neomacs match GNU exactly:

```elisp
(progn (set-terminal-parameter nil 'normal-erase-is-backspace nil)
       (normal-erase-is-backspace-setup-frame)
       (list (terminal-parameter nil 'normal-erase-is-backspace)
             (aref keyboard-translate-table 8)))
;; GNU     => (1 127)
;; Neomacs => (1 127)   <- identical once it is allowed to decide again
```

That isolated the decision as the fault. Two defects produced it, and the
second only became visible once the first was fixed.

DEFECT 1 -- an invented default vetoed the decision forever. The decision was
never premature: nothing ran it early. `terminal_parameter_default_value`
(neovm-core terminal/pure.rs) fabricated a compiled-in fallback of `0` for
`normal-erase-is-backspace`, so `(terminal-parameter nil
'normal-erase-is-backspace)` answered non-nil even though NOTHING had ever
stored it, and the `unless` guard at lisp/simple.el:11097 refused to run the
body at all. GNU has no terminal-parameter defaults: `Fterminal_parameter` is
an `assq` over the terminal's alist (src/terminal.c), every entry starts
absent, and the genuine `0`/`1` is written by the minor mode's `:variable`
setter (lisp/simple.el:11144) during `command-line`. That also explains the
earlier negative result: clearing `terminal.params` could not help, because the
default re-supplied `0` on the next read. The startup ORDER was already GNU's
-- neomacs-bin/src/main.rs publishes `tty-erase-char` at evaluator setup,
before the frontend loop evaluates `top-level` -> `command-line` ->
`normal-erase-is-backspace-setup-frame` (lisp/startup.el:1638) -- mirroring
`init_sys_modes` (src/sysdep.c:1130). Deleting the invented default made both
engines report `(8 1 127)` for the ledger probe above.

DEFECT 2 -- nothing consumed `keyboard-translate-table`. With the mode now
correctly on, C-h was still not translated: `key-translate` populated the
table and no Rust code ever read it. GNU applies it in `read_char`
(src/keyboard.c:3149-3163), on FRESHLY read events only -- rereads from
`unread-command-events` and keyboard macros jump past that block to
`reread_for_input_method` (src/keyboard.c:3252). Neomacs now mirrors that in
`translate_fresh_character_event` (neovm-core keyboard.rs), applied at the two
host-input conversion sites (`TtyCharacter` and `KeyPress`) that correspond to
GNU's `kbd_buffer_get_event` output, after the quit-character check (GNU
compares `quit_char` against the raw byte when storing the event) and before
kbd-macro recording (GNU records the translated char).

Fixed by both changes; the reduction
`backspace_on_a_ctrl_h_erase_terminal_deletes_like_gnu` (neomacs-tui-tests) is
now un-`#[ignore]`d and green, with unit pins for the empty default and for
the translation step.

Sibling sweep: one fabricated terminal-parameter default remains in those
helpers, `keyboard-coding-saved-meta-mode` => `(t)`. It is left in place
deliberately -- unlike `normal-erase-is-backspace` no guard keys off its
presence, and it is what an oracle-shaped bare `Context` needs to answer like a
booted GNU session -- but it is the same invented-default family and should be
retired the day the bootstrap stores it for real.

Status: FIXED.

## 68. Editing a buffer whose file changed on disk overwrites it silently -- FIXED

Sibling of entry 63, one step further along the same code path: 63 was the
lock held by another live Emacs, this is the file changed underneath us by
anything at all. Both were the same defect class -- neomacs asked the user
and then discarded the answer.

```elisp
;; visit a file, change it on disk behind the buffer's back, then type
(find-file f)
(write-region "changed\n" nil f nil 'silent)
(set-visited-file-modtime '(0 0))
(condition-case e (insert "X") (error (list 'signalled e)))
;; GNU     => (signalled (error "Cannot resolve conflict in batch mode"))
;;            buffer-modified-p => nil, buffer-string => "hello\n"
;; Neomacs => no-signal
;;            buffer-modified-p => t,   buffer-string => "Xhello\n"
```

Both engines PRINTED the prompt. Only GNU acted on it. `lock_file`
(src/filelock.c:608) calls the threat function with `calln`, so the
`file-supersession` signal -- or userlock.el's batch-mode error
(lisp/userlock.el:184) -- propagates out of `Flock_file`, out of
`prepare_to_modify_buffer_1` (src/insdel.c:2174), and aborts the edit before
any text is inserted. Our call site was `let _ = eval.apply(...)`: prompt
shown, answer dropped, edit proceeds. The buffer then overwrites on save the
file it was warned about, which is the data loss the check exists to prevent.

Reading `filelock.c:592-608` turned up three more gate bugs in the same
twenty lines, each of which independently suppresses the check:

* `create-lockfiles` nil returned early. GNU computes `lfname` only when
  `create_lockfiles` (:593-599) but runs the threat check regardless
  (:601-608) -- the `lock-file` docstring says so in as many words.
* The subject buffer was guessed as "the current buffer, if its
  `buffer-file-name` matches". GNU uses `Fget_truename_buffer (fn)` (:603)
  and hands THAT buffer to `Fverify_visited_file_modtime` (:605).
* `Ffile_exists_p (fn)` (:606) and the "unless this Emacs already owns the
  lock" clause (:607) were absent entirely.

Two supporting primitives had to be made real first, and both had failed
silently for as long as they had existed:

```elisp
(get-truename-buffer buffer-file-truename)
;; GNU     => #<buffer tn-3031593.txt>
;; Neomacs => nil            ; the whole builtin was a stub
```

`Fget_truename_buffer` (src/buffer.c:524-539) walks the live buffer list
comparing `buffer-file-truename` with `Fstring_equal`. Ours ignored its
argument and returned nil, so once `lock_file` was corrected to consult it
the check still never fired. `verify-visited-file-modtime` likewise
type-checked its BUF argument and then read the current buffer anyway, where
GNU does `decode_buffer (buf)` (src/fileio.c:6129).

Last, and only visible in a real session: GNU expands FN in exactly ONE
place, `make_lock_file_name` (src/filelock.c:543). Every other consumer sees
the caller's string verbatim. We expanded at the top of `lock_file`,
`unlock_file`, `file_locked_p`, `sync_modified_buffer_file_lock`,
`lock-buffer` and `unlock-buffer`. That is invisible for an absolute name --
every unit test passed -- and fatal in practice, because `find-file` stores
`buffer-file-truename` abbreviated: the buffer held `"~/Projects/.../f.txt"`
while we asked `get-truename-buffer` for `"/home/exec/Projects/.../f.txt"`,
nothing matched, and the check was skipped again. The batch probe, not the
unit tests, caught this.

Fix: `neovm-core/src/emacs_core/filelock.rs`. The `create-lockfiles` gating
is now a three-state `LockFileTarget` enum (`LockingDisabled`,
`FileExemptFromLocking`, `At(PathBuf)`) rather than `Option<PathBuf>` plus an
early return. GNU's single `lfname` local is nil in two unrelated cases that
behave differently, and collapsing them into one nil is precisely what let
the early return eat the check; naming both states makes that collapse
unrepresentable and the match exhaustive. `verify-visited-file-modtime`'s
helper now returns the decided `BufferId` instead of `Result<(), Flow>`, so
no caller can type-check BUF and then operate on a different buffer again.

Auditing this module for the same defect class turned up a THIRD swallow
site, `make-lock-file-name` itself:

```elisp
(advice-add 'make-lock-file-name :override
            (lambda (&rest _) (error "make-lock-file-name exploded")))
(condition-case e (lock-file f) (error (list 'signalled e)))
;; GNU     => (signalled (error "make-lock-file-name exploded")), no lock file
;; Neomacs => nil, and a lock file appears at .#NAME anyway
```

GNU calls it with `calln` (src/filelock.c:558). We caught the error and fell
back to a hand-rolled `".#NAME"` -- worse than losing the signal, because we
then created a lock at a name the Lisp layer had just refused to produce.
The fallback is now narrowed from "any error" to "the function is not
defined yet", which is the one state GNU has no analogue for: neomacs can
reach this code before files.el is loaded, where GNU bails out under
`will_dump_p` (src/filelock.c:589).

Commits: `5e70c106b` (primitives), `0f905057a` (propagation and gates),
`2dfb58742` (expansion boundary), `3934fa9f1` (make-lock-file-name errors).
Tests:
`supersession_threat_signal_propagates_out_of_lock_file_like_gnu`,
`supersession_threat_aborts_the_first_text_change_like_gnu`,
`supersession_threat_is_checked_even_when_create_lockfiles_is_nil_like_gnu`,
`supersession_threat_is_skipped_when_we_own_the_lock_like_gnu`,
`only_the_lock_file_name_expands_the_filename_like_gnu`,
`make_lock_file_name_errors_propagate_like_gnu`. The batch probe
above now diffs byte-identical against GNU, normalized only for the pid in
the temp file name.

Status: FIXED.

## 69. Lock file-errors carried GNU's elements in the wrong order, and unlock-file signalled -- FIXED

One probe, two divergences. Leave a lock file whose contents do not parse as
`USER@HOST.PID:BOOT`, then ask about it:

```elisp
(file-locked-p f)
;; GNU     => (file-error "Testing file lock" "Invalid argument" "/tmp/lk.txt")
;; Neomacs => (file-error "Testing file lock" "/tmp/lk.txt" "Invalid argument")

(unlock-file f)
;; GNU     => nil, plus  Warning (unlock-file): Unlocking file: Invalid argument, ...
;; Neomacs => (signalled (file-error ...))
```

GNU reports every lock failure with `report_file_errno` (src/filelock.c:648
for "Unlocking file", :776 for "Testing file lock"), and
`get_file_errno_data` (src/fileio.c:264-283) builds
`(SYMBOL ACTION STRERROR . NAME)` -- errno picks the symbol
(`file-missing` / `permission-denied` / `file-already-exists`, which
uniquely omits ACTION), and STRERROR is the bare libc text. We hand-built
`(ACTION FILENAME STRERROR)` and used Rust's `io::Error::to_string()`, which
appends `(os error N)` to a string the user reads. We also tagged
unparseable lock contents as `ErrorKind::InvalidData` rather than the EINVAL
GNU reports, so the errno-keyed classification never saw the real code.

Second half: `Funlock_file` (src/filelock.c:717-720) wraps the native path
in `internal_condition_case_1` for `file-error` and routes the error to
`userlock--handle-unlock-error` (lisp/userlock.el:217), which warns and
returns nil; the file-name-handler path sits deliberately OUTSIDE that
condition case. We propagated the error, so one bad lock file turned an
ordinary unlock into a failure.

Fix: `neovm-core/src/emacs_core/filelock.rs`. The ad-hoc constructor is
deleted, not re-ordered -- filelock now calls the existing faithful port of
`get_file_errno_data` in `fileio.rs`, so one place in the tree decides what
a file-error looks like and this cannot drift again.

Commit: `c267e79e1`. Tests:
`lock_error_data_matches_gnu_report_file_errno_shape`,
`unlock_file_routes_lock_errors_to_the_userlock_handler_like_gnu`. Batch
probe diffs byte-identical against GNU.

Status: FIXED.

## Not a divergence: `inhibit-modification-hooks` and file locking

Recorded because it was investigated and refuted, so nobody re-opens it. The
suspicion was that binding `inhibit-modification-hooks` silently disables
file locking in neomacs but not in GNU. The GNU source says the opposite:
`prepare_to_modify_buffer_1` returns at `src/insdel.c:2167`, and the
`Flock_file` call is at `:2174`, AFTER it. GNU skips locking too. neomacs
does the same thing at the same point (`editfns.rs`, `signal_before_change`).

```elisp
;; visit a file, then insert under each binding
(let ((inhibit-modification-hooks t)) (insert "X"))
;; GNU     => (nil nil)     ; (file-locked-p f) and (file-symlink-p lockfile)
;; Neomacs => (nil nil)
(insert "Y")   ; plain
;; GNU     => (t "exec@Matrix.PID:BOOT")
;; Neomacs => (t "exec@Matrix.PID:BOOT")
```

Byte-identical across all three probed states. No fix, no entry number.

## 70. `package-install-file` locks the file it installs, so a shared cached tar cannot be installed twice at once -- NOT A DIVERGENCE (harness defect), FIXED

Recorded because the signal looks like a neomacs bug and is not one. Two
`ac-html-*` scenarios that share a dependency raced, and the loser died in
batch with `file-locked` on the cached `web-completion-data` tar. GNU behaves
identically here; the defect was ours, in the harness, and the signal was
correct.

Reproduced deterministically -- 3 of 3 cold runs -- by removing the `ready`
markers for `ac-html-angular` and `ac-html-bootstrap` and letting nextest run
them in parallel. The backtrace leaves nothing to infer:

```
Error: file-locked (".../web-completion-data-20160318.848.tar"
                    "melpa-test@Matrix (pid 3819635)"
                    "Cannot resolve lock conflict in batch mode")
  signal(file-locked ...)
  ask-user-about-lock(".../web-completion-data-20160318.848.tar" ...)
  lock-buffer(".../web-completion-data-20160318.848.tar")
  set-visited-file-name(".../web-completion-data-20160318.848.tar" t)
  package-install-file(".../web-completion-data-20160318.848.tar")
```

`package-install-file` VISITS what it installs. It calls
`set-visited-file-name`, and `tar-mode` then touches the modified bit -- a
consequence `package.el` documents in its own comment at
`lisp/emacs-lisp/package.el:2316-2338` -- so Emacs takes a lock beside whatever
path it was handed. Both plans were handed the same path inside
`tmp/melpa/source-package-cache`, so both locked the same tar.

The `file-locked` signal is correct and is NOT suppressed. It became visible
only because the filelock work made `ask-user-about-lock` signal in batch
instead of being swallowed (entries 63 and 68-69); the collision itself
predates that and was previously silent.

This was also not a gap in the existing harness locking, which is worth
recording because it is the wrong place to look. `prepare_cached_source_artifact_with_tools`
already serializes BUILDING an artifact under an `fs4` lock keyed by package,
version, revision and tool revisions, and that lock was working. What was
missing is that INSTALLING from an artifact also writes next to it, so a cache
that is only safe to share for reading was being shared for writing.

Fixed by staging each artifact into a private `install/` directory under the
plan's own cache root and installing from the copy, so the shared artifact
cache is read-only for consumers and only the builder writes it, under its
existing lock. This removes the shared mutable path rather than scheduling
access to it. Locking every artifact in a plan was considered and rejected:
plans overlap partially, so that introduces a cross-plan lock ordering and a
deadlock surface to buy nothing.

Cold-race rate over the `ac-html` family: 3/3 fail before, 0/5 after, with zero
`file-locked` signals and no lock files left behind in the shared cache. Unlike
entry 54's magit race this one needs no load to reproduce -- both processes
visit the same path every time.

The general rule, for anyone adding a scenario: never hand
`package-install-file` (or anything else that visits a file) a path inside a
shared cache. Give it a private copy.

Status: FIXED.

## 71. `vertical-motion` counts a wrap that lands at end of buffer as a screen line moved -- FIXED

Found as a CRASH in the `multi_term` parity suite and reduced to a core motion
primitive. The suite was the messenger; `term.el` was the caller unlucky enough
to turn a wrong integer into a dead session.

The whole bug, with no packages, no processes and no terminal:

```elisp
(with-temp-buffer (insert (make-string 79 ?x)) (goto-char (point-min))
  (vertical-motion 1))
;; GNU     => 0   (point 80)
;; neomacs => 1   (point 80)   (before this fix)
```

Point moves identically in both. Only the RETURN differs. GNU's contract is
explicit (`src/indent.c`, `Fvertical_motion` docstring): it returns "number of
screen lines moved over; that usually equals LINES, but may be closer to zero
if beginning or end of buffer was reached". Filling the body width exactly puts
point at ZV with nothing on a following line, so no screen line was moved over
and the answer is zero. In batch GNU does not even use the display iterator --
`Fvertical_motion` branches on `noninteractive` to `vmotion`, which ends in
`compute_motion (from, from_byte, vpos, ..., ZV, vtarget, ...)`; that stops at
ZV and reports only the lines it genuinely crossed.

The boundary, measured against GNU 31.0.90 at body width 80:

| buffer length | GNU | neomacs before |
|---|---|---|
| 78 | 0 | 0 |
| 79 (fills the width exactly) | **0** | **1** |
| 80 (one char on a continuation line) | 1 | 1 |
| 81, 100, 160 | 1 | 1 |

So exactly one case diverged: the wrap that lands ON end-of-buffer.

HOW IT BECAME A CRASH. `term.el`'s hard-newline path
(`lisp/term.el:3183-3188`) does `term-move-to-column 0`, `term-down 1 t`, then
`(add-text-properties (1- (point)) (point) '(term-line-wrap t rear-nonsticky t))`.
`term-down` accounts with `(setq down (- down (term-vertical-motion down)))` and
only extends the buffer with `(term-insert-char ?\n down)` while `down` remains
non-negative. GNU's 0 leaves work to do and the newline is inserted; our 1
convinced it the move had happened, no newline was added, point stayed at 1, and
`(1- (point))` was 0 -- below `point-min` in a 1-based buffer. That signalled
`args-out-of-range (0 1)` inside the process filter and killed the batch session
with exit 255.

Fixed in `screen_line_motion_target`'s scanner
(`neovm-core/src/emacs_core/builtins/symbols.rs`): the wrap branch now reports
`counts_line: scan < point_max`, because a wrap only begins a screen line when
something is left to put on it. Counting it also claimed a line redisplay never
draws.

### How this was found, because the route generalises

**The `(0 1)` data shape did the first cut.** The signal carried exactly two
integers. That excludes most candidates by arity alone: `Fsubstring` signals
`args_out_of_range_3` with three elements (the string plus both indices), and
`Faref` puts the array first. A bare pair of integers is the text-property
validators' signature, which is what pointed at `add-text-properties` before any
backtrace existed.

**An A/B on identical input killed the leading hypothesis.** The first theory
was that our process filter delivered a short or empty string -- GNU's
`decoding_carryover` (`src/process.c:6243-6254`) holds an incomplete multibyte
tail across reads, and a terminal emulator is the workload most likely to expose
it. Instrumenting the filter in BOTH editors on the same scenario showed they
receive the IDENTICAL 152-byte string, the same process, and the same entry
state (term-width 79, term-height 22, point 1, point-max 1, same window,
window-width 80). GNU completes; we signalled. That refuted the hypothesis with
an observation rather than an argument, and it is retracted here in as many
words: `decoding_carryover` is NOT implicated and was never needed.

**WARNING -- AN ADVICE PROBE PRODUCED A CONFIDENT FALSE NEGATIVE.** Wrapping the
text-property family with `advice-add` and a `condition-case` did NOT fire, even
though `add-text-properties` was the signalling call. Read naively that says
"not a text-property function", which is the opposite of the truth and nearly
sent the diagnosis the wrong way. Only a real Lisp backtrace (custom `debugger`
with `debug-on-signal`) named the call. **Trust the backtrace over advice
coverage**: an instrument that silently fails to cover its target is worse than
no instrument, because it manufactures evidence for a wrong conclusion.

**Reduction was the rest of the value.** The six-case scenario was rebuilt as a
standalone script from the Rust sources, reproducing in seconds and freely
instrumentable, then narrowed to one case, then to one primitive, then to the
three-line form at the top of this entry.

Pinned by `vertical_motion_counts_only_screen_lines_that_are_actually_occupied`
(`neovm-core/src/emacs_core/window_cmds/tests.rs`), which asserts all three of
width-1, width and width+1 in one probe. The width+1 case is deliberate: it
stops the end-of-buffer rule from degenerating into "always report zero at ZV",
which would break every ordinary wrap.

### Two side findings, rechecked and closed 2026-08-12

- `backtrace-to-string` and `backtrace-frames` appeared to disagree in our
  build, but `(backtrace-to-string (backtrace-frames))` signals the identical
  `(wrong-type-argument backtrace-frame (t backtrace-frames nil nil))` on GNU.
  These are intentionally different interfaces: `backtrace-frames` exposes
  raw `(evald fun args flags)` tuples from `mapbacktrace`, while
  `backtrace-to-string` accepts the `backtrace-frame` records constructed by
  `backtrace-get-frames` (`lisp/emacs-lisp/backtrace.el:54-115,893-899`).  The
  documented adapter path produces a frame record and a string on both
  engines.  There is no divergence.
- GNU's `validate_interval_range` (`src/textprop.c:128`) does early-return on
  `(EQ (*begin, *end) && begin != end)`, where pointer identity encodes whether
  the caller requested a range or a point.  Neomacs does not flatten that
  operation kind: range APIs call `validate_{string,buffer}_*_range`, whose
  equal endpoints deliberately return no interval before bounds validation,
  while point APIs call distinct `validate_*_point*` functions that always
  validate bounds.  Invalid empty string/buffer ranges, distinct equal markers,
  an invalid nonempty range, invalid point queries, wrong objects, and wrong
  position types produce a byte-identical nine-case result on both engines.
  The Rust function boundary is the typed equivalent of GNU's pointer choice;
  there is no behavioral divergence to fix.

Status: FIXED.

## 72. `zlib-decompress-region` decoded its output instead of inserting raw bytes -- FIXED

Silent data corruption in a primitive. Every multibyte sequence in a
decompressed payload collapsed to a single truncated byte, with no error
anywhere.

Same gzip input, same buffer:

```elisp
(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert-file-contents-literally "omega.gz")   ; "A<U+03A9>B<U+4F60>C<U+1F600>D\n"
  (zlib-decompress-region (point-min) (point-max))
  (string-to-list (buffer-string)))
;; GNU     => (65 206 169 66 228 189 160 67 240 159 152 128 68 10)   14 bytes
;; neomacs => (65 169 66 96 67 0 68 10)                               8 bytes  (before)
```

The pattern names the cause exactly: each character survives as its codepoint
truncated to one byte. U+03A9 -> 0xA9 (169), U+4F60 -> 0x60 (96), U+1F600 ->
0x00 (NUL). We built the replacement string with a constructor that counts
characters as multibyte text, so a 14-byte payload became an 8-character string
and each character was then narrowed to a byte on its way into the unibyte
buffer.

GNU does not decide anything here. `Fzlib_decompress_region` inserts with
`insert_from_gap (decompressed, decompressed, 0, false)`
(`src/decompress.c:311`) -- the SAME count passed as both nchars and nbytes,
justified by its own comment a few lines up: "This is a unibyte buffer, so
character positions and bytes are the same." The function errors out on a
multibyte buffer before reaching that point, so bytes-as-characters is the only
possible reading.

BLAST RADIUS, which is larger than the parity case that found it: any
in-process gzip decompression of non-ASCII content was wrong, and wrong
QUIETLY -- no signal, no truncation error, just different bytes. It stayed
invisible because `jka-compr` shells out to the gzip binary for
`insert-file-contents`, so ordinary `.gz` file visiting never touched this
path. Only callers that decompress in-process -- `url.el` fetching a
gzip-encoded response, which is how org_cliplink tripped it -- saw the damage.

Fixed at the type level rather than by correcting the call. The inflate output
now leaves `decompress_auto` wrapped in `InflatedBytes`, whose only exit is
`into_unibyte_string` (`neovm-core/src/emacs_core/zlib.rs`). The wrong
constructor is no longer reachable from that value, which matters because the
wrong reading produced no error -- a future caller reaching for a decoding
constructor by habit would have reintroduced exactly this bug just as silently.

Pinned by `zlib_decompress_region_inserts_raw_bytes_not_decoded_characters`
(`neovm-core/src/emacs_core/xml_test.rs`), which asserts the full byte list of
a fixture containing 2-, 3- and 4-byte sequences. The 4-byte case earns its
place: it truncated to NUL, the one corruption that also terminates C-style
consumers.

(Correction, same day: as first committed this entry and the test's own
docstring claimed the 4-byte case was covered when the fixture held only the
2- and 3-byte sequences. The claim was written before the fixture was, and
nothing checked it -- the test passed either way, because it was pinning
whatever the fixture happened to contain. The emoji was added afterwards and
the fixture now genuinely spans all three widths.)

### Not a cluster, and that was checked rather than assumed

This arrived paired with a CRLF-on-save divergence under one "coding-system
cluster" hypothesis. Tested explicitly and FALSE: that one is EOL conversion
being skipped by the signature-prefixed and UTF-16 ENCODERS (`utf-8-dos` and
`latin-1-dos` are correct; `utf-8-with-signature-dos` and `utf-16le-dos` drop
the CR), which shares no code with decompression and is filed separately. This
is the second cluster hypothesis in one day to look obvious and be wrong; the
first was a shared stdin-EOF signature that turned out unique to one suite.

Note for anyone re-running the suites: fixing this does NOT turn org_cliplink
green. That suite has a second, unrelated failure -- a gnutls handshake
rejecting the test server's certificate as UnknownIssuer where GNU accepts it
-- also filed separately.

Status: FIXED.

## 73. EOL conversion was opt-in across the encode paths, so half the coding systems never got CRLF -- FIXED

Writing a file with CRLF worked for some coding systems and silently did not
for others. Same content, same `end_of_line = crlf` rule, different bytes:

```elisp
(string-to-list (encode-coding-string "a\nb" 'utf-8-dos))
;; GNU     => (97 13 10 98)
;; neomacs => (97 13 10 98)                      correct
(string-to-list (encode-coding-string "a\nb" 'utf-8-with-signature-dos))
;; GNU     => (239 187 191 97 13 10 98)
;; neomacs => (239 187 191 97 10 98)             the CR is gone
(string-to-list (encode-coding-string "a\nb" 'utf-16le-dos))
;; GNU     => (97 0 13 0 10 0 98 0)
;; neomacs => (97 0 10 0 98 0)                   the CR is gone
```

The BOM was written correctly. Only the EOL leg dropped, and it dropped
without any signal.

TWO EARLIER DESCRIPTIONS OF THIS BUG WERE WRONG, and both are recorded here
because each would have sent the fix to the wrong place.

The first, from the original backlog note, was "the EOL rule is not reaching
the coding system". Measured and false: `buffer-file-coding-system` really is
`utf-8-with-signature-dos` with `(coding-system-eol-type ...)` = 1. The rule
arrives intact; the encode path ignores it.

The second, recorded in entry 72's "not a cluster" section, was the narrower
"EOL conversion being skipped by the signature-prefixed and UTF-16 ENCODERS".
Also false, and false in the direction that matters: it names two encoders when
the defect is structural. Measured against GNU 31.0.90, `"a\nb"` through
`encode-coding-string`, the honest split was

| already correct | dropped the CR |
|---|---|
| utf-8-dos / -mac | utf-8-with-signature-dos / -mac |
| latin-1-dos / -mac | utf-8-auto-dos |
| raw-text-dos / -mac | utf-16le-dos / -mac, utf-16be-dos, utf-16-dos |
| japanese-iso-8bit-dos, korean-iso-8bit-dos, chinese-iso-8bit-dos | utf-7-dos, utf-7-imap-dos |
| japanese-shift-jis-dos, shift_jis-dos | chinese-hz-dos |
| in-is13194-devanagari-dos, chinese-big5-dos | emacs-mule-dos, iso-2022-jp-dos, iso-2022-7bit-dos |
| | vietnamese-viqr-dos / -mac |

Fourteen coding systems wrong, not two. The statement that survives measurement
is: EOL conversion was OPT-IN across our encode paths, and whether a coding
system got it depended on which branch it happened to fall into.

GNU makes the choice impossible. It applies EOL conversion in NO encoder. It
applies it in `consume_chars` (`src/coding.c:7607`), the single function that
fills the character buffer every encoder then reads (`src/coding.c:7683`):

```c
if (! EQ (eol_type, Qunix))
  { if (c == '\n') { if (EQ (eol_type, Qdos)) *buf++ = '\r'; else c = '\r'; } }
```

By the time `encode_coding_utf_8`, `encode_coding_utf_16`,
`encode_coding_iso_2022` or `encode_coding_raw_text` run, the CR is already in
the charbuf. An encoder cannot skip EOL conversion because it never sees a bare
newline. The buffer-headroom comment just above -- "Compensate for CRLF and
conversion", `src/coding.c:7646` -- is there for exactly this expansion.

Ours was the inverse. EOL conversion was an opt-in call, `encode_eol_bytes` /
`encode_eol_text` in `neovm-core/src/encoding.rs`, made at five scattered
sites. `encode_lisp_string` opened with an early return into the UTF-16 encoder
BEFORE any of them. A second dispatch chain in
`builtin_coding_string_in_context` -- ten arms: utf-7, HZ, emacs-mule,
no-conversion-multibyte, utf-8-signature, CCL, ISO-2022, EUC, Shift_JIS,
charset-list -- reached the call only from inside three of its own encoders
(`encode_via_euc`, `encode_via_sjis`, `encode_via_charset_list`), which is
precisely why EUC and Shift_JIS were right while ISO-2022 and emacs-mule, one
arm away, were not.

Fixed by inverting the structure rather than by teaching the failing arms to
call it. The source text is now expanded ONCE, above every dispatch, by
`expand_source_eol` (and its `&str` twin `expand_source_eol_text`), mirroring
`consume_chars`; all five downstream call sites were deleted in the same
change and `encode_eol_bytes` no longer exists. Teaching two arms to call it
would have restored exactly the opt-in shape that produced the bug and left the
other ten silently at the mercy of the next edit.

Two things keep the new shape from decaying. The single pass also SPENDS the
EOL leg of the coding-system name (`coding_name_with_eol_spent`: `-dos`/`-mac`
becomes `-unix`), so any second pass anywhere downstream is a no-op by
construction rather than by discipline. And `encode_via_euc`, `encode_via_sjis`
and `encode_via_charset_list` no longer take a `coding_system` parameter at
all -- they took it only to do EOL, and without it they cannot be told about
EOL again. Removing the parameter turned the invariant into three compile
errors that had to be answered, which is the point.

One more path was found by measuring rather than by reading: a coding system
whose conversion is an elisp `:pre-write-conversion` hook (vietnamese-viqr)
encoded the hook's OUTPUT through the bare base codec name, `utf-8`, which
carries no EOL leg -- so `vietnamese-viqr-dos` also wrote LF. GNU runs the hook
and then encodes the result through `encode_coding_object`, whose
`consume_chars` still expands the newline, so the leg survives. It is now
carried onto the base codec name in `run_coding_with_conversion_hook`.

THE DECODE SIDE HAD THE SAME DEFECT, and fixing encode alone exposed it. The
editorconfig parity case writes the file and then reads it back; with the CR
finally being written, `utf-8-with-signature-dos` read back as `"café\15\n"`.
Our decode collapsed CR LF in only four of the ten codec arms, and that had
stayed invisible for exactly as long as the encode side never produced a CR to
collapse. GNU is symmetric here too: `decode_coding` (`src/coding.c:7481`) runs
the decoder and then calls `decode_eol` once, outside every `decode_coding_*`.
So the decode pass is now also single, and it is placed AFTER the decoder
rather than before it -- in UTF-16 a CR is the byte pair 0D 00, so collapsing
CR LF in the SOURCE bytes would never find it. The name is spent up front here
too, which matters more on this side than on the encode side: collapsing is
NOT idempotent (`"\r\r\n\n"` collapses twice to `"\n\n"`), so a second pass is
corruption, not a no-op.

The trap in this fix is the reverse of the bug: convert up front and leave the
old call sites in place, and the coding systems that were CORRECT start
emitting CR CR LF. So the pin is ONE assertion over ALL 31 measured rows --
`eol_conversion_applies_to_every_encoder_not_just_some`
(`neovm-core/src/encoding_test.rs`). The rows that were broken prove the fix;
the rows that were already right are what stops it over-applying; the `-unix`
row stops it applying at all. Every expected value was taken by running the
probe under GNU 31.0.90, not derived -- a derived expectation would have
pinned our bug. A second assertion in the same test round-trips all 19 dos/mac
systems back through decode, which is the pin the decode half needed and did
not have.

The integration check is the editorconfig parity case
`charset_and_crlf_rules_control_the_exact_bytes_written_on_save`, which asserts
both halves at once: the bytes on disk (239 187 191 99 97 102 195 169 13 10)
and the text read back ("café\n").

Status: FIXED.

## 74. `call-interactively` leaked nested command identity out of interactive argument acquisition -- FIXED

Evil operators ran the right edit under the wrong command identity.  In four
evil-goggles workflows, GNU exposed the operator consistently while Neomacs
kept the motion used to acquire its range:

```elisp
;; Observed from advice around the operator body while executing "dd".
;; GNU     => (evil-delete evil-delete evil-delete)
;; Neomacs => (next-line  evil-delete evil-delete) ; before
;;             this-command real-this  this-original
```

That asymmetry initially pointed at the command loop, but the command loop had
already selected the right Evil operator.  Evil's Lisp-form interactive spec
then computes an operator range by running a nested motion; that motion changes
the command variables temporarily.  The package-independent reproduction was:

```elisp
(setq this-command 'outer-this
      this-original-command 'outer-original
      real-this-command 'outer-real
      last-command 'outer-last)
(defun command-state (argument)
  (interactive
   (progn
     (setq this-command 'inner-this
           this-original-command 'inner-original
           real-this-command 'inner-real
           last-command 'inner-last)
     (list 7)))
  (list argument this-command this-original-command
        real-this-command last-command))
(list (call-interactively 'command-state)
      this-command this-original-command real-this-command last-command)
;; GNU => ((7 outer-this outer-original outer-real outer-last)
;;         outer-this outer-original outer-real outer-last)
;; Neomacs before => the same shape filled with the four inner-* values.
```

GNU makes the lifetime explicit in `Fcall_interactively`.  It snapshots
`Vthis_command`, `Vthis_original_command`, `Vreal_this_command`, and the
keyboard's `Vlast_command` together before acquiring arguments
(`src/callint.c:281-284`).  After evaluating a Lisp-form spec it restores all
four immediately before `funcall-interactively` (`src/callint.c:340-346`); the
string-spec path does the same (`src/callint.c:796-803`).  Neomacs evaluated
the spec through its shared call planner but had no corresponding saved state,
so the nested motion's last assignment remained visible to the operator body
and its advice.

The fix models that protocol instead of adding four unrelated assignments.
`CallInteractivelyCommandIdentity` captures and GC-roots the four values as one
unit, and every `CallInteractivelyPlan` must own one.  Its invocation target is
private until the plan is consumed by `restore_for_invocation`, which returns a
distinct `RestoredCallInteractivelyInvocation`.  Both the evaluator and the
bytecode VM therefore have to cross the restoration transition before this API
will construct the final `funcall-interactively` arguments; a new planner path
that forgets the snapshot or restores only a subset fails to compile.

The GNU-derived invariant is pinned independently in both engines by
`call_interactively_restores_command_identity_after_form_spec_evaluation` and
`vm_call_interactively_restores_command_identity_before_invocation`.  Each
asserts what the command body sees as well as what remains after it returns.
The unchanged evil-goggles package oracle then passes all four original
workflows with the operator-specific faces and command identities restored.

Status: FIXED.

## 75. Empty strings were not canonical objects, so `eq` guards fell through to prompts -- FIXED

`auto-yasnippet` should reject `aya-persist-snippet` before asking for a name
when no auto-snippet has been defined.  Its interactive form contains this
guard:

```elisp
(if (eq aya-current "")
    (user-error "You don't have an auto-snippet defined")
  (list (read-string "Snippet name: ")))
```

GNU took the `user-error` branch.  Neomacs missed it, entered `read-string`,
and batch execution ended with `(end-of-file "Error reading from stdin")`.
The package code and the dynamically bound value were the same on both sides;
the smallest discriminator was object identity itself:

```elisp
(list (eq "" "")
      (eq "" (make-string 0 ?x))
      (eq "" (substring "x" 0 0))
      (eq "" (concat)))
;; GNU     => (t t t t)
;; Neomacs => (nil nil nil nil)                 before
```

GNU makes this a property of allocation, not of those four callers.
`src/alloc.c:1519-1536` initializes permanent `empty_unibyte_string` and
`empty_multibyte_string` objects.  `make_clear_string`
(`src/alloc.c:2298-2307`) and `make_clear_multibyte_string`
(`src/alloc.c:2327-2339`) return the corresponding singleton at length zero;
the reader reaches the same allocation seam through `make_specified_string`
(`src/lread.c:3039-3145`).  The two storage representations remain distinct:
empty unibyte strings are `eq` to each other, empty multibyte strings are `eq`
to each other, and an empty unibyte string is not `eq` to an empty multibyte
string.

Neomacs instead allocated a fresh arena object on every `alloc_string` call.
One earlier symptom had already been patched locally in `expand-file-name` by
treating two empty filename strings as identical by content.  That workaround
fixed one consumer while leaving every ordinary Lisp `eq` guard broken.

The fix puts the invariant at the shared allocator boundary.  The storage
choice is now an exhaustive `LispStringStorageKind` enum rather than a boolean
threaded through singleton logic.  Each heap owns a typed
`CanonicalEmptyString` lifecycle with three states: `Missing`, `Owned`, and
`Mapped`.  Zero-length allocation returns the existing object for its storage
kind; newly owned objects are permanent GC roots; pdump restoration explicitly
replaces a temporary owned handle with the authoritative mapped object.  The
old `expand-file-name` content exception was then deleted, because its normal
identity check now has GNU's semantics.

The public Lisp regression
`empty_strings_are_canonical_per_storage_kind_like_gnu` was run red first.  It
seeded the multibyte object, forced a garbage collection, and observed
`(nil nil nil t nil nil nil)` where GNU requires
`(t t nil t nil t t)`.  It passes after the allocator change, as does the
existing empty-name `expand-file-name` regression.  The full `neovm-core` gate
passes 8,813 tests, a fresh release build and pdump complete successfully, the
rebuilt dumped runtime matches GNU's constructor/storage-kind identity matrix,
and the original `auto_yasnippet_package_batch` oracle is green.

Status: FIXED.

## 76. `error-message-string` used `prin1` for objects GNU sends through `princ` -- FIXED

An incomplete form in `flycheck-package` correctly signalled `end-of-file`,
but the rendered error data exposed a different buffer representation:

```elisp
(let ((buffer (generate-new-buffer " *error-source*")))
  (unwind-protect
      (error-message-string (list 'end-of-file buffer))
    (kill-buffer buffer)))
;; GNU     => "End of file during parsing:  *error-source*"
;; Neomacs => "End of file during parsing: #<buffer  *error-source*>" ; before
```

The generic printers were already correct: `(format "%s" buffer)` returned
the live buffer name and `(prin1-to-string buffer)` returned the readable
`#<buffer ...>` form in both editors.  The divergence was in how
`error-message-string` chose between them.

GNU's `print_error_message` makes that choice for the whole datum in
`src/print.c:1128-1162`.  Data belonging to a file-error, `end-of-file`, or
`user-error` goes through `Fprinc`; ordinary condition data goes through
`Fprin1`.  Neomacs represented the choice as a `quote_strings` boolean.  That
could remove quotes from a string but could not express `princ` semantics for
a buffer, so all non-string data still passed through the readable printer.

The replacement is the exhaustive `ErrorDatumPrintMode::{Princ, Prin1}`.
Every error branch now selects a complete object-printing protocol, and both
arms reuse the same byte printers as the public Lisp operations.  The
destination is part of the protocol too: GNU prints into its multibyte
`Vprin1_to_string_buffer`, where `Fprinc` octal-escapes a raw unibyte 0xFF as
the four characters `\377`; this differs from `(format "%s" ...)`, which
preserves the raw byte.  A dedicated multibyte-buffer `princ` seam records
that distinction instead of making callers reconstruct it.

The public Lisp regression
`error_message_string_end_of_file_prints_live_buffer_name_like_gnu` failed
first with the package's exact `#<buffer ...>` excess.  While gating the fix,
an older raw-unibyte unit expectation was measured against GNU and found
stale; it now pins GNU's multibyte `\377: foo` result.  All 29
`error-message-string` tests pass, the complete `neovm-core` gate passes 8,814
tests, a fresh release build and pdump succeed, and the original
`flycheck_package_package_batch` oracle is green.

Status: FIXED.

## 77. Directory enumeration discarded the Lisp-expanded path before local I/O -- FIXED

`shrink-path` expands abbreviated fish-style paths with `f-glob`, which reaches
the public `file-expand-wildcards` and `directory-files` primitives.  With a
dynamically rebound `HOME`, Neomacs expanded `~` correctly in isolation but
enumerated the process's original home directory:

```elisp
(let ((old-home (getenv "HOME")))
  (unwind-protect
      (progn
        (setenv "HOME" "/path/to/fixture")
        (directory-files "~/"))
    (setenv "HOME" old-home)))
;; GNU     => ("." ".." "Projects")
;; Neomacs => entries from /home/exec                 ; before
```

An explicit absolute wildcard worked, which ruled out wildcard matching and
isolated the first broken primitive to directory enumeration.  GNU expands
the directory exactly once before both handler lookup and local work in
`Fdirectory_files` (`src/dired.c:379-392`).  Its sibling
`Fdirectory_files_and_attributes` has the identical preamble
(`src/dired.c:421-436`).

Neomacs had a helper that also expanded before handler lookup, but returned
only `Option<Value>`.  `None` meant "no handler result" and discarded the
expanded filename with it.  The caller then resolved the original `~/` again
through a host-environment path helper, bypassing Lisp's dynamic
`process-environment`.  `directory-files-and-attributes` skipped the GNU
preamble entirely and had the same observable bug.

The replacement models the protocol as the exhaustive
`ExpandedFileOperation::{Handled, Local}` enum.  `Local` owns the expanded
filename, so a caller cannot represent "handler absent" without also carrying
the only path local I/O may consume.  `directory-files`,
`directory-files-and-attributes`, and `delete-file` now validate their DEFUN
arguments, cross that typed expansion/dispatch boundary, and pattern-match the
result before proceeding.

The public Lisp regression
`directory_files_expands_tilde_from_lisp_process_environment_like_gnu` failed
first for both directory primitives and now returns the synthetic fixture from
both.  All 18 focused directory tests pass, the complete `neovm-core` gate
passes 8,815 tests with 51 skipped, a fresh release build and pdump succeed,
and the unchanged `shrink_path_package_batch` oracle is green.

Status: FIXED.

## 78. Backward search let beginning assertions inspect at the match stop -- FIXED

`gitattributes-mode-backward-field` searches backward with a pattern ending in
`\<`.  Neomacs accepted the nearest field beginning even though the assertion
was evaluated at the original point, so the command stayed on `markdown`
instead of moving back to `text`:

```elisp
(with-temp-buffer
  (insert "one two three")
  (goto-char 9)
  (re-search-backward " \\<" nil t))
;; GNU     => 4
;; Neomacs => 8                              ; before
```

The sibling boundary pattern `" \\b"` returned 8 in both editors.  That
control ruled out backward candidate ordering and exposed the assertion-level
distinction.

GNU passes the original point as the `re_search_2` match stop in
`src/search.c:1202-1211`.  Its `wordbeg` opcode uses the limit-aware `PREFETCH`
before inspecting the character at the assertion (`src/regex-emacs.c:5007-5048`),
and `symbeg` does the same (`src/regex-emacs.c:5093-5111`).  Word boundaries
deliberately use `PREFETCH_NOLIMIT`, so their lookahead remains legal at the
stop.  Neomacs passed `stop` through search and matching but its beginning
assertion helpers read the full text without consulting it.  Both regex
engines therefore accepted a match GNU rejects.

The fix represents the six zero-width syntax assertions with an exhaustive
`SyntaxAssertion` enum and routes both the backtracking and Pike engines
through one stop-aware evaluator.  Pike epsilon closure now receives the
match stop explicitly.  The type boundary centralizes GNU's intentionally
different policies for beginnings, ends, and boundaries so a future engine
cannot silently choose another one.

The public Lisp regression
`re_search_backward_word_begin_respects_search_origin_like_gnu` failed first
with 8 instead of 4 and also pins `\_<` and the `\b` control.  A lower-level
test forces the same matrix through both regex engines.  All 302 focused regex
tests pass, the complete `neovm-core` gate passes 8,817 tests with 51 skipped,
a fresh release build and Lisp byte-compilation succeed, and the unchanged
`gitattributes_mode_package_batch` oracle is green.

Status: FIXED.

## 79. `line-number-at-pos` counted inaccessible text beyond a narrowed buffer -- FIXED

ESS built its imenu index before narrowing the buffer, leaving the
`healthcheck` entry as a marker beyond the later `point-max`.  The parity test
then converted that marker to a line number while the buffer was narrowed:

```elisp
(with-temp-buffer
  (insert "zero\none\ntwo\nthree\nfour\n")
  (let ((after (copy-marker 20)))
    (narrow-to-region 6 14)
    (line-number-at-pos after)))
;; GNU     => 3
;; Neomacs => 4                              ; before
```

GNU resolves markers and validates explicit fixnums against the full-buffer
`BEG` and `Z` in `Fline_number_at_pos` (`src/fns.c:6698-6717`).  It then clips
the resolved byte position to `[BEGV_BYTE, ZV_BYTE]` for non-absolute counting
before calling `count_lines` (`src/fns.c:6719-6729`).  Neomacs did neither half
of that sequence correctly: it rejected fixnums outside the narrowed bounds,
while markers bypassed that check and were counted unmodified through the
inaccessible tail.  ESS's blank line beyond `ZV` therefore added one.

The fix models the complete origin-and-endpoint policy as the exhaustive
`LineNumberScope::{Accessible, Absolute}` enum.  Each variant constructs the
typed `EmacsByteRange` consumed by newline counting.  Accessible scope can no
longer select `BEGV` as its origin without also clipping the endpoint, and
numeric validation is independently expressed in full-buffer coordinates.

The public Lisp regression
`line_number_at_pos_clips_nonabsolute_positions_to_narrowing_like_gnu` failed
first with `(1 4 :rejected :rejected 1 :rejected)` instead of GNU's
`(1 3 1 3 1 1)`, covering markers on both sides, explicit fixnums, and the
safe absolute-before-`BEGV` control.  All focused line-number tests pass, the
complete `neovm-core` gate passes 8,818 tests with 51 skipped, a fresh release
build, pdump, and all 1,515 Lisp byte-compiles succeed, and the unchanged
`ess_package_batch` oracle is green.

Status: FIXED.

## 80. GnuTLS `:trustfiles` were parsed by Lisp but discarded before TLS verification -- FIXED

`org-cliplink` fetched a title from the package fixture's HTTPS server while
binding `gnutls-trustfiles` to that server's self-signed certificate.  GNU
completed the verified request and selected the exact auth-source entry after
a wildcard entry; Neomacs failed before the HTTP or auth logic ran:

```elisp
(let ((gnutls-trustfiles (list fixture-certificate))
      (network-security-level 'low))
  (url-retrieve-synchronously "https://127.0.0.1:<port>/private"))
;; GNU     => an HTTP response buffer
;; Neomacs => (gnutls-error -1
;;              "TLS handshake: invalid peer certificate: UnknownIssuer") ; before
```

The fixture certificate includes the matching `127.0.0.1` IP subject
alternative name, and the test supplied it as an explicit trust anchor.  The
failure was therefore not a reason to weaken hostname or certificate
verification.

GNU parses `:trustfiles` in `emacs_gnutls_boot` (`src/gnutls.c:2036-2044`),
loads the system trust store (`src/gnutls.c:2124-2138`), and then adds every
listed PEM file with `gnutls_certificate_set_x509_trust_file`
(`src/gnutls.c:2140-2168`).  Neomacs parsed only `:hostname` from the same
property list.  Its Rustls backend always constructed a root store containing
only Mozilla roots, so the Lisp trust-file setting could not reach
verification through any synchronous, deferred, or direct `gnutls-boot`
path.

The replacement makes the policy backend-neutral and closed:
`TlsClientParameters` carries the hostname and an exhaustive
`TlsTrustRoots::{Default, DefaultPlusFiles}` choice.  Every TLS backend call
now requires the complete parameter object, turning any future call site that
drops trust policy into a compile error.  The Rustls implementation begins
with its existing Mozilla roots and adds every PEM certificate from the
explicit files, preserving GNU's additive behavior and all normal peer
verification.

The unchanged `org_cliplink_package_batch` workflow failed first with
`UnknownIssuer` and now completes the HTTPS and auth-source case.  Focused
tests pin ordered trust-file parsing, GNU's invalid-entry error, PEM root-store
augmentation, and backend errors.  All 16 TLS tests pass, the complete
`neovm-core` gate passes 8,821 tests with 51 skipped, a fresh release build
produces a 13,616,018-byte pdump and all 1,651 Lisp byte-compiles, and the exact
package oracle remains green after rebasing onto current `main`.

Status: FIXED.

## 81. TTY row tails lost GNU's logical-face and physical-cell semantics -- FIXED

The `helm-pydoc` workflow compared raw PTY cells after opening Python source,
moving through Helm candidates, and running actions.  Fourteen stage/row
comparisons disagreed: Neomacs wrote ordinary spaces where GNU left erased
cells, while the comment line needed physically written spaces despite its
line-end filler resolving to default SGR attributes.  One action row also put
the split-window divider immediately after short text instead of in the final
matrix column.

GNU keeps three distinctions that the old TTY grid had flattened.  First,
`write_row` trims only trailing `CHAR_GLYPH_SPACE_P` cells and clears the rest
of an unknown row (`src/dispnew.c:5944-6013`); termcap `in` selects the
mutually exclusive must-write-spaces path (`src/term.c:4683`).  Second,
`CHAR_GLYPH_SPACE_P` requires a space in the default logical face
(`src/dispextern.h:634-639`), so a face-filtered filler can look like default
SGR without becoming erasable.  The TTY branch of
`extend_face_to_end_of_line` explicitly fills the remaining text cells and
uses the default face only for an `ends_at_zv` row
(`src/xdisp.c:24681-24758`).  Finally, frame-matrix assembly overwrites the
last window-matrix cell with a vertical-border glyph for a non-rightmost
window (`src/dispnew.c:2585-2601,2664-2674`).

Neomacs instead forced every initially unknown nonempty row through a
full-width write, inferred blank semantics from terminal attributes alone,
and represented erased and written spaces as the same `TtyCell`.  Its
synthetic right-margin area also flowed after the used text, so a short line
moved the divider left.

The fix makes both independent cell dimensions closed and explicit:
`BlankErase::{DefaultFace, Explicit}` records logical erase eligibility and
`CellMaterialization::{Erased, Written}` records the physical terminal state.
Every terminal operation is exhaustively replayed into materialization before
the desired grid becomes current.  `BlankTailMethod::{WriteSpaces,
EraseToEol { back_color_erase }}` combines termcap `in`, the exact encodable
`ce`, and BCE policy so contradictory capability booleans are unrepresentable.
Rasterization now materializes GNU's line-end filler, canonicalizes stable
presentation face IDs at the TTY-attribute boundary, preserves published face
fills, and anchors a synthetic right-border area at the last matrix cell.

The unchanged
`helm_pydoc_real_helm_workflows_match_gnu_terminal_and_filesystem` test failed
first with the raw-cell and divider differences and now passes all stages
against a freshly built release binary.  Focused red/green tests pin unknown
row trimming, termcap `in`, written interior blanks, default-looking
non-default faces, nonzero IDs for resolved default faces, and short-text
divider placement.  The complete display-runtime suite passes 1,015 tests,
display-protocol passes 620, and the frontend passes 229 with one skipped.  A
fresh release build produces a 13,616,082-byte pdump and all 1,651 Lisp byte
compiles.  The broad core release gate passed 8,813 tests with one unrelated
PTY status timing failure; that test passed immediately in isolation.

Status: FIXED.

## 82. Overlay before-strings stayed behind when their anchor word wrapped -- FIXED

With word wrapping enabled, an overlay before-string at the first character of
a word remained on the full row while the buffer word moved to its continuation
row:

```elisp
(insert (concat (make-string 70 ?a) " bbbbbbbbbb\n"))
(overlay-put (make-overlay 72 74) 'before-string "SS")
;; GNU row 1/2     => no S / "SSbbbbbbbbbb"
;; Neomacs before => "SS" / "bbbbbbbbbb"
```

GNU's `display_line` records a word-wrap candidate with `SAVE_IT` before the
first display element after whitespace (`src/xdisp.c:26073-26103`).  That saves
the complete iterator, including its display-string stack.  Restoring the
candidate therefore removes the before-string from the first row and replays it
with its anchor word.  Neomacs recorded candidates only before buffer
characters; the producer-owned overlay-string element had already rendered by
the time the anchor character supplied a checkpoint.

The fix gives every source element one atomic candidate operation over its
glyph checkpoint, display-point metadata, and authoritative byte/character
position, and calls it before appending the overlay strings.  Producer rewind
is now the exhaustive `BufferSourceRewind::{WordWrap, CharacterWrap}` choice:
word wrap clears an insertion marker at or after the restored checkpoint so
the string replays, while character wrap preserves an already-rendered string.
The compiler now forces any future overflow path to choose those semantics.

The end-to-end regression failed first with two S glyphs on the first row and
now pins the GNU row placement.  Controls prove that `may_wrap` still prevents
a candidate inside an unbroken word and that character wrapping emits the
string only once.  The complete layout-engine suite passes 1,946 tests with
three skipped, a fresh release build produces a 13,616,082-byte pdump and all
1,651 Lisp byte-compiles, and the release TUI suite passes all 908 tests.

Status: FIXED.

## 83. String glyph positions and continuation tabs lost their GNU coordinate spaces -- FIXED

Two frozen display-parity cases shared the same structural fault: row assembly
reduced a source coordinate to a bare row-local number.  A string-valued
`display` replacement emitted every glyph with the covered buffer start, so a
three-character replacement produced `[2, 2, 2]` instead of string indices
`[0, 1, 2]`.  Independently, a TAB on a wrapped continuation row was measured
from that screen row's origin instead of from the accumulated physical line.

GNU makes both coordinate spaces explicit.  `struct glyph` stores `charpos`
together with its source object; the number is a buffer position for a buffer
object and a string index for a string object
(`src/dispextern.h:460-483`).  String production installs the string object and
its current index (`src/xdisp.c:9459-9613`), while cursor recovery dispatches on
the object and selects the smallest string index after bidi reordering
(`src/xdisp.c:18589-19079`).  GNU deliberately keeps replacement coverage out
of each glyph to keep the glyph compact (`src/xdisp.c:6759-6821`).  TAB layout
likewise adds `continuation_lines_width` for ordinary continuation-row text,
but measures a wrap-prefix string on the current screen row
(`src/xdisp.c:33815-33885`); wrap transitions update that physical-line width
at the exact restored iterator position (`src/xdisp.c:26345-26435`).

The fix represents glyph origin as the exhaustive
`GlyphProvenance::{Buffer, Str, Redisplay}` enum.  Each row owns a compact side
table of string occurrences containing the logical string identity and, for a
replacement, its exact covered buffer range.  A glyph carries only a niche-sized
row token and string index, preserving the existing 80-byte glyph size bound.
Production writers can no longer accept a bare numeric stamp.  Cursor lookup,
fast cursor paint, word-wrap rollback, JSON transport, routed rows, and
incremental rebasing now consume the typed provenance; string indices never
shift as buffer positions, while occurrence coverage shifts exactly once.
Clipped text remainders advance their typed string origin transactionally, so
a replacement cannot silently become buffer-mapped on the next row.

TAB state is now the closed
`DisplayTabCoordinates::{ScreenLine, ContinuedPhysicalLine}` choice backed by
an integer `TabGridPixel` domain matching GNU.  The buffer walk owns the
physical-line accumulator, resets it only on physical line breaks or
truncation, and subtracts wrap-prefix width from ordinary buffer TABs.  Word
wrap and routed-row fit checkpoints carry the complete typed row position.
Structural row-tail reconciliation may move the renderer pen but cannot erase
the source walk's TAB coordinate space.

The replacement-stamp regression failed first with `[2, 2, 2]`; continuation
TAB, row-tail reconciliation, and clipped-string-remainder tests each failed
at their respective untyped seam before the fixes.  The final protocol suite
passes 624 tests, display-runtime passes 1,015 with one skipped, layout-engine
passes 1,956 with three skipped, and the workspace all-target check succeeds.
A fresh release build produces a 13,616,018-byte pdump and all 1,651 Lisp byte
compiles, and the release TUI suite passes all 908 tests.

Status: FIXED.

## 84. TUI fixtures escaped their owning tests; the idle-wakeup report was stale -- FIXED

Task 29's remaining item contained two independent claims.  Re-measuring the
GUI first refuted the old "8 wakeups/s versus 2 presents/s" snapshot: after
startup settled, the current release produced 53.0 event-loop wakeups/s and
13.2 presents/s over 17.4 seconds.  The active demand was the intentional 12 Hz
cursor-color cycle plus the 2 Hz cursor animation; no expose reason was active,
no deadline needed recovery, and scene commits stayed flat.  Task 48 had
already made a ripe deadline into work before constructing a future wake, so
there was no residual zero-wait spin to fix.

The TUI storage half did reproduce.  One focused `project-find-file` test left
its complete 168 KiB temporary Git repository behind after passing.  Session
HOME and TMPDIR paths had acquired manual best-effort cleanup earlier, but
project, Dired, strict-grid, shell, mode, child-frame, and shared-file fixtures
still returned bare `PathBuf`s.  Once the path escaped its constructor, Rust no
longer represented who owned the directory, so normal test completion had
nothing to drop.

GNU's test contract makes the missing lifetime explicit:
`ert-with-temp-directory` creates the resource inside `unwind-protect` and
recursively deletes it on normal or nonlocal exit
(`lisp/emacs-lisp/ert-x.el:281-376`).  Interactive Org separately removes both
Babel temporary directories from `kill-emacs-hook`
(`lisp/org/ob-core.el:3630-3650`).

The harness now represents session paths as the exhaustive
`SessionDirectory::{Owned, Borrowed}` choice.  `Owned` contains a
`TuiTempDirectory` RAII guard; `Borrowed` contains only the caller's path and
cannot enter harness cleanup.  `TuiTempDirectory` and `TuiTempFile` carry the
same ownership token through every fixture constructor, and no raw
`std::env::temp_dir()` fixture creator remains in the crate.  The failed-first
drop-contract test observed the leaked project tree, then passed with the typed
owner.  All-target compilation succeeds, the complete TUI suite passes 911 of
911 tests, and its isolated TMPDIR has zero descendants after the suite exits.

Status: FIXED.

## 85. Fontconfig enrichment changed GNU entity order, and named instances shared a synthetic ordinal -- FIXED

The real X11 font-selection oracle diverged in 9 of 19 Noto Sans cases.  The
first semantic difference was `semi-light`: GNU selected the variable Light
entity (weight 300), while Neomacs selected a local static Regular face
(weight 400).  Both were distance 50 from the requested CSS weight 350, so the
difference exposed equal-score ordering rather than weight normalization.

GNU `ftfont_list` asks `FcFontList` only for entity metadata and conditionally
adds the charset (`src/ftfont.c:919-940`).  It skips variable-font meta patterns
but preserves the order of every concrete named-instance pattern
(`src/ftfont.c:190-218,1071-1077`).  `font_score` encodes four independent
seven-bit property distances, ordered by `face-font-selection-order`
(`src/font.c:2110-2167,2332-2356`), and `font_sort_entities` scores each entity
independently.  Equal scores never replace the earlier entity
(`src/font.c:2232-2322`).

Neomacs asked the discovery `FcFontList` for `FC_POSTSCRIPT_NAME` and
`FC_FONT_VARIATIONS`.  Fontconfig uses the requested projection while
uniquifying patterns, and those two renderer-enrichment fields changed the raw
order: static Regular became ordinal 0 and variable Light ordinal 10.  A
second abstraction grouped all named instances sharing a family/file, retained
the first instance's ordinal, and replaced its representative with a later
style.  Thus a variable ExtraBold Italic entity could donate its early ordinal
to variable Bold Italic, incorrectly outranking the earlier static Bold Italic
entity.

The fix makes discovery projection the closed
`GnuEntityProjection::{Metadata, MetadataAndCharset}` choice and keeps it
byte-for-byte aligned with GNU's object set.  Renderer identity is enriched
only after shared policy selects one candidate, through `FontBackend`'s
single-winner finalization hook; FreeType then supplies the exact named-instance
PostScript name and variation tuple without perturbing discovery or probing
every candidate.  Candidate grouping is removed.  The shared
`CandidateSelectionScore` contains compatibility plus a `GnuStyleScore` whose
derived Rust ordering declares width, size, weight, then slant; the entity's
own discovery ordinal is an explicit final tie break.  The legacy emergency
fallback consumes the same score type.

Focused real-GUI tests failed first for semi-light and italic identity, and a
synthetic same-variable-file test failed by selecting the donated ordinal.
They now pass, the complete 19-case font oracle is byte-identical to GNU, the
layout-engine suite passes 1,959 tests with three skipped, the X11 GUI suite
passes 20 of 20 tests, and the Wayland smoke passes.  A refreshed release
pdump was used for every real-GUI green.

Status: FIXED.

## 86. Inserted padding does not inherit the surrounding text property -- FIXED

Command Log Mode inserts a propertized key description and then uses
`move-to-column` to pad the command name to a fixed column.  GNU gives the
inserted padding the key description's `:time` property; Neomacs stops the
property at the original last character:

```elisp
(with-temp-buffer
  (insert (propertize "x" :time "STAMP"))
  (move-to-column 5 t)
  (list (buffer-string)
        (mapcar (lambda (position)
                  (get-text-property position :time))
                (number-sequence (point-min) (1- (point-max))))))
;; GNU     => (#("x    " 0 5 (:time "STAMP"))
;;             ("STAMP" "STAMP" "STAMP" "STAMP" "STAMP"))
;; Neomacs => (#("x    " 0 1 (:time "STAMP"))
;;             ("STAMP" nil nil nil nil))
```

The rank-396 `command-log-mode` corpus reaches this through real keyboard
commands and records both merged and unmerged repetitions.  The visible log
text is identical, but every timestamp property run ends before the padding in
Neomacs.

GNU `Findent_to` inserts both tabs and spaces through
`Finsert_char(..., INHERIT=t)` (`src/indent.c:943-982`).  The tab-splitting
branch of `Fmove_to_column` uses the same inheriting insertion, and its
short-line branch delegates back to `Findent_to`
(`src/indent.c:1158-1177`).  Neomacs instead had two direct calls to its raw
buffer insertion method in `move-to-column`; both bypassed the typed property
policy already used by `indent-to`.

The fix puts all indentation text through one
`insert_inheriting_indentation` helper.  Its interface deliberately exposes no
plain-insertion option, while the lower insertion boundary requires the closed
`InsertPiecePropertyMode::InheritAdjoining` enum independently of marker
placement.  Tab splitting calls that helper; short-line padding now follows
GNU's call graph by delegating to `indent-to`, removing its duplicate tab/space
construction.

Separate failed-first GNU-parity regressions cover short-line padding and tab
splitting.  Both now pass, as do all 43 `move-to-column` oracle cases, all 53
focused indentation tests, and the complete `neovm-core` gate (8,823 passed,
51 skipped).  A fresh release binary and pdump pass the live oracle.  The real
Command Log Mode workflow now agrees on every timestamp-property range and
fails only on the independent write annotation recorded in entry 87.

Status: FIXED.

## 87. `write-region` ignores `write-region-annotate-functions` -- FIXED

Command Log Mode saves each logged command with a timestamp prefix by
dynamically binding `write-region-annotate-functions`.  GNU calls the
annotation function; Neomacs writes only the buffer bytes:

```elisp
(defun divergence-annotation (start _end)
  (list (cons start "[STAMP] ")))
(let ((file (make-temp-file "annotation")))
  (unwind-protect
      (with-temp-buffer
        (insert "line\n")
        (let ((write-region-annotate-functions
               '(divergence-annotation)))
          (write-region (point-min) (point-max) file nil 'silent))
        (with-temp-buffer
          (insert-file-contents file)
          (buffer-string)))
    (delete-file file)))
;; GNU     => "[STAMP] line\n"
;; Neomacs => "line\n"
```

The variable is already registered as special in Neomacs; the dynamic binding
is visible.  The missing behavior was in the write path itself.  Neomacs
extracted a buffer region and immediately selected a coding system and encoded
it; no phase consumed the hook, its returned annotations, a hook-selected
replacement buffer, or the post-annotation callback.

GNU removes restrictions for a nil START, builds annotations before coding
selection and destination opening, and lets a callback-selected buffer replace
the source (`src/fileio.c:5584-5627`).  `build_annotations` exposes earlier
results through `write-region-annotations-so-far`, discards them after a buffer
switch, and destructively merges each newer sorted list with the accumulated
list (`src/fileio.c:5880-5960`).  At equal positions, this puts the newer
callback's list first while preserving order inside each list.  `a_write`
intersperses string payloads at 1-based **character** positions, including the
end boundary, through the same coding stream as the source; non-string payloads
are consumed without output (`src/fileio.c:5966-6025`).  Literal string START
values skip annotation collection, but successful writes still run the post
callback for each participating live buffer, newest replacement first and the
original last (`src/fileio.c:5804-5823`).

The fix introduces the exhaustive `WriteRegionSource::{Literal, BufferRegion}`
choice, so a literal cannot accidentally enter hook collection.  A
`PreparedWriteRegion` owns source selection and cleanup.  Annotation positions
use `LispCharPos1` rather than storage offsets, payloads are the closed
`WriteAnnotationPayload::{Text, NoText}` choice, and distinct
`WriteAnnotationBatch`/`WriteAnnotationBatchIndex` types make GNU's two-level
equal-position order explicit and prevent the two coordinates from being
mixed.  The prepared character stream is encoded once, preserving stateful and
multibyte coding behavior across annotation boundaries.

The original live oracle failed first with `"line\n"` instead of
`"[STAMP] line\n"`.  A second failed-first regression caught an initially
over-broad reverse tie break: GNU produced
`<B1><B2><A1><A2>x`, while the first implementation reversed each callback's
own list.  Regressions also cover multibyte character positions, end-boundary
ordering, `write-region-annotations-so-far`, literal-source bypass, replacement
buffer selection, and post-cleanup order.  All 199 focused file-I/O tests pass,
the fresh release build and final pdump succeed, the live oracle is green, and
the rank-396 Command Log Mode workflow now matches GNU completely.

Status: FIXED.

## 88. Shared unique-symbol names multiplied the GC root set -- FIXED

The exhaustive minimax tic-tac-toe oracle took more than 120 seconds under
Neomacs while GNU completed the same probe in 18.3 seconds.  A profile assigned
57.7% of Neomacs samples to `Context::seed_all_context_roots`; GC tracing then
showed one `symbol-name-thread-local` class containing about 1.47 million roots
and being scanned twice per collection.

GNU's `Fmake_symbol` creates a new symbol but stores the exact supplied name
string in it (`src/alloc.c:3662-3708`), and the reader passes that object
directly to `Fmake_symbol` (`src/lread.c:4705-4708`).  Recursive macro
expansion can therefore create many unique symbols that all share one name
object.  Neomacs correctly retained a name per symbol, but its process-global
registry also indexed GC roots by symbol ID, appending the shared string once
for every unique symbol.

The failed-first regression creates two unique symbols with the same exact
name object and observed two roots where GNU's object graph has one.  Its
second control uses equal-content but separately allocated strings: those must
remain two roots.  That distinction rules out a structural `HashSet`, because
`TaggedValue` equality and hashing are value-based for strings.

The fix introduces separate `SymbolNameHeapId` and `SymbolNameObjectId`
newtypes and a `SymbolNameRootIndex` keyed first by heap identity and then by
the name object's tagged bits.  Per-symbol name storage remains unchanged, so
unique-symbol semantics are preserved, while the root index represents exact
object identity once.  The types prevent heap IDs, symbol IDs, and object IDs
from being mixed at compile time.  The long-term model remains GC-managed
uninterned symbol objects; this bounded refactor fixes root cardinality without
coupling that representation migration to the bug.

The regression failed with two roots and now passes both identity controls.
The formerly timing-out Neomacs snapshot completes in 17.7 seconds, live
GNU/Neomacs parity completes in 33.9 seconds, and the complete `neovm-core`
gate passes 8,822 tests with 51 skipped.  Formatting, the fresh release build,
and the release pdump fingerprint gate also pass.

Status: FIXED.

## 89. Category-inherited `rear-nonsticky` did not bound inserted text -- FIXED

Org Brain renders entries as text-property buttons and uses `picture-mode`
column padding to lay out its relationship graph.  The default button category
declares `rear-nonsticky t`, so GNU leaves padding outside the button.  Neomacs
instead copied the button category, action, keymap, and identifier across the
padding, creating extra buttons whose labels consisted of newlines and spaces.

The package workflow exposed the difference in a real rendered map.  This
package-free reduction isolates the insertion rule:

```elisp
(with-temp-buffer
  (put 'padding-boundary 'rear-nonsticky t)
  (insert (propertize "x" 'category 'padding-boundary 'probe t))
  (let ((indent-tabs-mode nil))
    (move-to-column 3 t))
  (list (buffer-substring-no-properties (point-min) (point-max))
        (get-text-property 1 'probe)
        (get-text-property 2 'probe)
        (get-text-property 2 'category)))
;; GNU                => ("x  " t nil nil)
;; Neomacs before fix => ("x  " t t padding-boundary)
```

GNU's `adjust_intervals_for_insertion` obtains `front-sticky` and
`rear-nonsticky` with `textget` (`src/intervals.c:844-853,1034-1037`).
`textget` resolves a missing direct property through the interval's category
symbol (`src/intervals.c:1706-1735`).  Neomacs' insertion merge implemented
the direct-plist and default-nonsticky rules but read these four meta-properties
only from the direct interval map, so it never saw a button category's
nonstickiness.

The fix routes left/right front/rear stickiness through the existing effective
text-property resolver before the merge.  The focused regression covers the
category boundary, while the Org Brain visualization workflow locks the
original multi-line button topology.

Status: FIXED.

## 90. A word-wrap break drew a continuation marker GNU never produces -- FIXED

MWIM binds `C-a`/`C-e` to visual-line movers, so its terminal workflow renders
a `visual-line-mode` buffer whose long lines wrap at word boundaries.  Every
wrapped row in Neomacs ended in a backslash where GNU's row ends in the
wrapped text.

The package-free reduction is the shape of the rows themselves.  With a
ten-column window, `word-wrap` on and `truncate-lines` off:

```elisp
;; buffer contents "aaaa bbbbbb\n"
;; GNU rows              => ("aaaa "        "bbbbbb ")
;; Neomacs before fix    => ("aaaa      \\" "bbbbbb ")

;; buffer contents "aaaa bbbbbbbbbbbb\n"
;; GNU rows              => ("aaaa " "bbbbbbbbbb\\" "bb ")
;; Neomacs before fix    => ("aaaa      \\" "bbbbbbbbbb\\" "bb ")
```

GNU's `display_line` distinguishes the two ways a row can reach the right
edge.  The word-wrap branch (`back_to_wrap`, `src/xdisp.c:26360-26388`)
rewinds to the recorded wrap point, sets `row->continued_p` and calls
`extend_face_to_end_of_line`, but never calls `produce_special_glyphs` with
`IT_CONTINUATION`.  The mid-element branches do: the element that does not fit
at all (`src/xdisp.c:26336-26345`), an over-wide TAB
(`src/xdisp.c:26399-26403`), and the general case (`src/xdisp.c:26421-26432`),
which produces the glyph whenever `!FRAME_WINDOW_P`.

Neomacs drove its right-edge marker off one `Continued` row flag, which
conflates both branches.  The fix carries the break kind through the overflow
transition type (`VisualWrapBreak::AtWordBoundary` vs `MidElement`) and raises
a separate `ContinuedMidElement` flag that the marker pass reads; `Continued`
keeps GNU's `row->continued_p` meaning, so fringe continuation arrows are
unchanged.
## 91. `insert` discarded the arguments it had already converted -- FIXED

The Git Rebase Mode fallback-insert workflow passes a valid prefix, a list
returned by `process-lines`, and a newline to primitive `insert`.  GNU leaves
the valid prefix in the buffer and signals on the list; Neomacs inserted
nothing, so the workflow's buffer, point, `buffer-modified-p` and undo list all
diverged.

```elisp
(with-temp-buffer
  (list (condition-case e (insert "pick " '("x") "\n") (error e))
        (buffer-string)
        (point)
        (buffer-modified-p)))
;; GNU                => ((wrong-type-argument char-or-string-p ("x")) "pick " 6 t)
;; Neomacs before fix => ((wrong-type-argument char-or-string-p ("x")) "" 1 nil)
```

GNU's `general_insert_function` (`src/editfns.c:1307-1345`) converts and
inserts one argument at a time, so `wrong_type_argument` for argument N leaves
arguments 0..N already in the buffer.  It also re-reads the buffer's
multibyteness per argument, because a change hook run by an earlier argument
may have changed it.  Neomacs collected and validated the whole argument vector
first, then inserted the batch.

`insert`, `insert-before-markers`, `insert-and-inherit` and
`insert-before-markers-and-inherit` were four hoisted copies of that batch
body.  They now share one `general_insert_function` mirroring GNU's loop, with
GNU's two behavioral axes kept as the separate `InsertPieceMarkerPlacement` and
`InsertPiecePropertyMode` parameters so no call site can conflate them.  All
four variants insert their valid prefix, as verified in GNU.

Status: FIXED.

## 92. `call-process-region` resolved the program before applying DELETE -- FIXED

The emacs-w3m recovery workflow calls `call-process-region` with DELETE=t and a
renderer that is not on `exec-path`.  GNU has already deleted the region when
the executable lookup fails; Neomacs still held the original HTML, so the two
editors entered the recovery step from different buffer states.

```elisp
(with-temp-buffer
  (insert "hello")
  (list (condition-case e
            (call-process-region (point-min) (point-max)
                                 "neomacs-no-such-program-xyz" t nil)
          (error e))
        (buffer-string)))
;; GNU                => ((file-missing "Searching for program"
;;                         "No such file or directory"
;;                         "neomacs-no-such-program-xyz") "")
;; Neomacs before fix => (... "hello")
```

GNU's `Fcall_process_region` (`src/callproc.c:1099-1147`) validates the region,
writes it to the temp file, performs the DELETE, and only then calls
`call_process`.  `call_process` is where PROGRAM is type-checked
(`src/callproc.c:390`) and searched for on `exec-path`
(`src/callproc.c:447-476`), so everything that can signal about the program,
the working directory, the destination or the argument vector happens after the
region is gone.  Neomacs did all of that first.

The impl now follows GNU's order.  The read-only pre-check stays ahead of the
deletion, where GNU's `Fdelete_region`/`barf_if_buffer_read_only` puts it, and
the write coding system is still resolved before the region text is captured,
matching GNU's `create_temp_file`.  The same holds for START = nil, GNU's
whole-buffer delete branch.

Status: FIXED.

## 93. `minibuffer-default` was never bound during a minibuffer read -- FIXED

All five failing Imenu Anywhere workflows record `minibuffer-default` from
`minibuffer-setup-hook` while `imenu-anywhere` runs its real `completing-read`.
GNU reports the guessed default; Neomacs reported nil in every one of them.

```elisp
(let ((seen nil))
  (let ((minibuffer-setup-hook
         (list (lambda ()
                 (push (copy-tree minibuffer-default) seen)
                 (setq unread-command-events
                       (listify-key-sequence (kbd "RET"))))))
        (executing-kbd-macro t))
    (read-from-minibuffer "P: " nil nil nil nil "zed")
    (read-from-minibuffer "Q: " nil nil nil nil '("one" "two")))
  (list (nreverse seen) minibuffer-default))
;; GNU                => (("zed" ("one" "two")) nil)
;; Neomacs before fix => ((nil nil) nil)
```

GNU's `read_minibuf` binds the variable to the DEFAULT argument at the top of
the read (`src/minibuf.c:591`, `specbind (Qminibuffer_default, defalt)`) and
unwinds it on exit.  Everything that offers the default to the user reads the
variable rather than the argument: `next-history-element`/`M-n`,
`minibuffer-default-add-function`, and packages observing the live minibuffer.
Neomacs threaded DEFAULT through the read but never bound the variable.

The bind now sits above the recursion and entry checks in the one shared VM
runtime read, so no path through `read-from-minibuffer`, `read-string`,
`completing-read` or their callers can skip it -- the same shape as the other
GNU invariants that are applied once above dispatch.

Status: FIXED.

## 94. `eval-buffer` never consulted `load-read-function`, so Edebug never instrumented -- FIXED

All five Undercover coverage workflows report zero coverage.  Undercover
instruments through Edebug, and Edebug never ran: with `edebug-all-defs` bound
to t, defining a function left no `edebug` property and no `edebug-*` call in
the stored body at all.  Instrumentation did not happen differently -- it did
not happen.

```elisp
(require 'edebug)
(with-temp-buffer
  (insert "(defun probe-fn (x) (+ x 1))\n")
  (let ((edebug-all-defs t)) (eval-buffer)))
(list (and (get 'probe-fn 'edebug) t)
      (and (string-match-p "edebug-after" (format "%S" (symbol-function 'probe-fn))) t))
;; GNU                => (t t)      ; and it prints "Edebug: probe-fn"
;; Neomacs before fix => (nil nil)
```

Edebug does not hook `defun`, `eval-defun` or macroexpansion for this.  It
replaces the *reader*: `edebug-install-read-eval-functions` runs
`(add-function :around load-read-function #'edebug--read)`
(`lisp/emacs-lisp/edebug.el:556`), unconditionally when edebug.el loads
(`edebug.el:4632`).  GNU's `readevalloop` reads every top-level form through
that variable --

```c
	  else if (! NILP (Vload_read_function))
	    val = calln (Vload_read_function, readcharfun);   /* src/lread.c:2317 */
```

-- and `edebug--read` instruments only when the stream it is handed is the
current buffer (`edebug.el:457`), which is why `eval-buffer` instruments and
`load` of a file does not: `Feval_buffer` passes the BUFFER as readcharfun
(`src/lread.c:2417`), while loading a file passes `get-file-char`.  Undercover
picks the instrumenting one on purpose, through a `file-name-handler-alist`
entry: `(let ((edebug-all-defs t) ...) (eval-buffer (find-file file)))`.

Neomacs's `load` already consulted the hook; `eval-buffer` and `eval-region`
did not.  They lifted the buffer's text and read it internally, so the hook was
never called and `edebug-all-defs` silently did nothing.

The fix makes our readevalloop consult the hook GNU consults -- no Edebug
behaviour is written in Rust; edebug.el is loaded as Lisp and does the
instrumenting.  Which function reads a form is now one type resolved once from
GNU's precedence (explicit READ-FUNCTION, then `load-read-function`, then the
internal reader), and the two shapes GNU's loop takes over a buffer are a
second type: `eval-buffer` reads from the buffer's own point, `eval-region`
records an excursion and narrows around every read.  Both skip whitespace and
comments before invoking the reader, as GNU does (`src/lread.c:2272-2288`), so
a trailing newline no longer provokes an extra end-of-file read.

Status: FIXED.
## 95. A terminal row skipped the `:extend` fill whenever its background was invisible -- FIXED

The Leuven TUI workflow renders a `diff-mode` buffer.  GNU carries the
`diff-context` face to the end of its row; Neomacs stopped one space after the
text, so the row lost the face's foreground across its whole tail.

GNU's `extend_face_to_end_of_line` has a "nothing would be painted, skip the
fill" early return, and it is guarded by `FRAME_WINDOW_P`
(`src/xdisp.c:24388`).  A terminal frame can never reach it, so control always
falls through to the terminal fill branch (`src/xdisp.c:24679-24809`), which
materializes the row out to the text-area edge whatever the face looks like.
The skip is a window-system optimization only.

That matters precisely when an `:extend` face is invisible against the frame.
GNU defines `diff-context` as `'((t :extend t))`
(`lisp/vc/diff-mode.el:476-479`) and Leuven maps it to `diff-none`
(`etc/themes/leuven-theme.el:341`), whose realized background is the default
`#FFFFFF`.  Comparing backgrounds alone therefore drops a fill GNU still
performs -- and GNU uses it to carry the face's FOREGROUND `#A0A1A7` across the
row.

Neomacs applied that background comparison on both frame types, and applied it
at each caller rather than once: `row_lifecycle.rs` and `finalizer.rs` each
pre-filtered the extend face out of the `LineEndContext` before the line-end
seam ever saw it, and `source_render.rs` repeated the test for the wrap-break
path.  This is the opt-in-invariant shape: a rule every branch had to remember.

The fix moves the decision into the seam as `line_end::extend_fill_runs`, a
total match on `DisplayRowMeasurementMode` (the existing `FRAME_WINDOW_P`
equivalent).  The skip lives only in the `ConcreteFont` arm, so a terminal row
that skipped the fill is no longer representable.  `LineEndContext` gained a
required `frame_background` field, which turned every call site into a compile
error until it stopped filtering and handed the raw face over.  A single
`effective_extend` accessor now feeds the plain fill, the fill-column-indicator
gap/tail and the merged-indicator background, so those three cannot disagree
about whether the highlight is painting.

The unchanged `leuven_theme_real_color_lifecycle_matches_gnu` failed before and
passes after.  Two focused `line_end` tests pin both arms in one test each --
an invisible `:extend` face fills on a terminal row and is skipped on a
window-system row, and a visible one fills on both -- so a fix that filled on
neither or on both would leave half of it green.  `neomacs-layout-engine`
passes 1,966 tests with 3 skipped.  A fresh release build produced a
13,646,938-byte pdump.

Attribution was established by running the whole `tui_parity_tests` set twice
at the same filesystem path, swapping only the binary: pristine gives 7 passed
/ 6 failed, the fix gives 8 passed / 5 failed, and `leuven` is the only test
that moves.  That control matters because this worktree's long path makes five
unrelated terminal tests (`magit`, `mwim`, `gruvbox`, `helm_css_scss`,
`beacon`, plus two `neomacs-tui-tests` cases) fail identically with and without
the change -- they wrap prompts and truncate directory strings that a shorter
checkout path never exercises.  `helm_pydoc`, the pin guarding ledger 81's
blank-erase semantics, stays green.

Magit is NOT fixed by this and is a different defect.  Its remaining diff is
`row 3 cols 44..=120` and `row 4 cols 15..=120`, where GNU has
`contents=" "` and Neomacs has `contents=""` -- rows carrying NO `:extend`
face, whose text-area gap sits before a right window margin.  Row 2, which does
carry an extend face, already matched.  The cells are not missing because
nothing fills them: the TTY backend's rasterizer already materializes that gap.
They are missing because `set_desired_glyph_cell`
(`neomacs-display-runtime/src/backend/tty/rif.rs:1041-1057`) preserves a cell's
existing `Erased` materialization whenever a default-face blank is written over
an identical default-face blank, and `normalize_desired_blank_tails`
(`rif.rs:1476-1490`) only ever converts the row's uniform trailing run to
`Erased`, never promotes an interior cell back to `Written`.  GNU writes those
cells because they are interior -- margin glyphs follow them, so its row
trimming cannot reach them.  Emitting fill glyphs from the layout engine would
not help, since a stretch glyph takes the same `set_desired_glyph_cell` path.
The candidate rule is to make `Erased` hold exactly for cells inside the row's
uniform erasable tail, but `materialization` feeds the row hash
(`rif.rs:1977`) and the wholly-erased fast path (`rif.rs:1329`), so that change
needs its own red/green cycle and is deliberately left unmade here.

Status: FIXED for the `:extend` line fill (Leuven); Magit tracked separately.

## 96. Terminal cells GNU wrote inside a row's changed span were left erased -- FIXED

Magit's log buffer puts a right margin (author and date) at the far end of
every row.  GNU filled the gap between the commit subject and that margin with
spaces; Neomacs left those cells never written, so a raw PTY capture read
`contents=" "` for GNU and `contents=""` for Neomacs across `row 3 cols
44..=120` and `row 4 cols 15..=120`.  Ledger 95 diagnosed the rows as carrying
no `:extend` face and left the fix unmade.

The diagnosis inherited from ledger 95 -- that `set_desired_glyph_cell`
preserves an `Erased` materialization and `normalize_desired_blank_tails`
never promotes an interior cell back to `Written` -- is accurate about how
those cells become `Erased`, but it is not where the divergence is decided.
Reading GNU shows the decision lives one level up, in how a row is put on the
wire.  `write_row` never diffs the interior of what it writes.  When the
current row is blank it takes the `!olen` path and emits ONE run,
`write_glyphs (f, nbody + nsp, nlen - nsp)` (`src/dispnew.c:6062-6079`);
otherwise the insert/delete path emits ONE run,
`write_glyphs (f, nbody + nsp + begmatch, nlen - tem)`
(`src/dispnew.c:6173-6186`).  Either way every cell between the common prefix
and `nlen` is physically written.  `nlen` loses only the trailing
`CHAR_GLYPH_SPACE_P` cells, and only when `write_spaces_p` is false
(`src/dispnew.c:6019-6022`) -- a row whose margin reaches the last column has
no such tail, so its interior gap is content GNU writes.

Neomacs decided both cases by logical cell equality instead.  Its
wholly-erased fast path required `uniform_erasable_tail` to find a trimmable
tail and refused the whole GNU branch when there was none; its multi-span
emitter split `[first_changed, last_changed]` into changed runs and skipped
gaps up to `GOTO_COST_CELLS`, whose comment claimed the byte-cost rule
"strictly dominates" GNU's single span.  It does not: a skipped gap whose
cells are physically erased stays unwritten where GNU has spaces, and that
difference is exactly what a raw cell capture reads.

The fix keeps materialization untouched and corrects the two write decisions.
`desired_row_content_end` names GNU's `nlen` once, so the wholly-erased path
falls back to the full row instead of abandoning GNU's branch, and the span
emitter's gap test now asks whether the terminal already has a glyph there:
byte cost may only skip cells that are already `Written`.  Skipping a written
cell is invisible; skipping an erased one is not.  Nothing else observes the
change -- `row_hash` only accelerates scroll matching and always verifies cell
equality before trusting a match, and the wholly-erased fast path's guard
still reads the same materialization it always did, now with GNU's extent.

Two focused tests pin the two paths.  An erased row gaining content that
reaches its last column previously planned `[WriteRun 0..2, WriteRun 17..20]`
and now plans `[WriteRun 0..20]`; a row with leftover content, which stays out
of the `!olen` path, previously planned `[WriteRun 0..3, WriteRun 17..20]` and
now plans `[WriteRun 0..20]`.  Both failed before the change and pass after.

`magit_log_buffer_file_margin_columns_match_gnu_full_screen` passes against a
freshly built release binary (13,647,778-byte pdump, 1,651 Lisp byte
compiles).  The whole `tui_parity_tests` set is 13 tests run: 9 passed, 4
failed, and the four -- `mwim`, `gruvbox`, `helm_css_scss`, `beacon` -- are
ledger 95's long-worktree-path set minus `magit`, which is the only test that
moved.  `helm_pydoc`, the pin guarding ledger 81's blank-erase semantics,
stays green.  `neomacs-layout-engine` passes 1,966 with 3 skipped;
`neomacs-display-runtime` passes 614 (the `wgpu::transition` and
`render_thread` groups hang for 600s each in this headless worktree with and
without the change, so they are excluded rather than reported as passing);
`neomacs-tui-tests` is 913 tests run: 910 passed, 3 failed, and those three
(`set_visited_file_name_elisp_functions_match_gnu_semantics`,
`dired_copy_current_file_via_c_copies_file_and_updates_listing`,
`keyboard_quit_after_find_file_ctrl_h_returns_to_scratch`) fail identically
with a pristine binary built at the same filesystem path -- they wrap this
worktree's long directory string and disagree with the host `ls` column width.

Status: FIXED.

## 97. `record_insert` coalesced insertions in the wrong direction, so undo deleted untouched text -- FIXED

The Tide format/undo workflow formats a JS region, then undoes it.  GNU's
`undo-only` restores the file byte for byte; Neomacs restored
`export const total=add(1,2)` as `export const total add(1,2)` -- the `=` that
neither editor had touched was gone.

```elisp
(with-current-buffer (get-buffer-create "probe")
  (buffer-enable-undo)
  (setq buffer-undo-list nil)
  (insert "export const total=add(1,2)")
  (undo-boundary)
  (goto-char 20) (insert " ")
  (goto-char 19) (insert " ")
  (let ((records buffer-undo-list))
    (primitive-undo 1 buffer-undo-list)
    (list (buffer-string) (car records) (car (cdr records)))))
;; GNU                => ("export const total=add(1,2)" (19 . 20) (20 . 21))
;; Neomacs before fix => ("export const total add(1,2)" (19 . 21) 28)
```

GNU's `record_insert` (`src/undo.c:98-112`) coalesces a new insertion into the
newest record in exactly one direction: when that record is a `(BEG . END)`
insertion whose END equals the new insertion's BEG.  There is deliberately no
reverse rule.  `primitive-undo` replays the records newest-first, and each
deletion reshapes the buffer that the later records are read against, so two
insertions made back-to-front are two records that undo correctly in sequence.

Neomacs had a second, invented branch that also merged when the new insertion
ENDED where the newest record BEGAN, rewriting `(20 . 21)` and a new insert at
19 into a single `(19 . 21)`.  That record claims positions 19 and 20 are both
newly inserted text; position 20 is the untouched `=`, and undo deleted it.

Back-to-front is the ordinary shape, not an exotic one: `tide-apply-edits`
walks a TypeScript `textChanges` list in reverse precisely so that earlier
positions stay valid while later edits are applied, and every LSP-style client
does the same.  The invented branch is deleted; the remaining merge is GNU's.

Status: FIXED.

## 98. `last-nonmenu-event` outlived the key sequence that read it, so `imenu` prompted -- FIXED

The Tide navigation workflow calls `imenu` interactively after driving two
commands through `execute-kbd-macro`.  GNU builds the index and returns without
a prompt and without moving point; Neomacs read `Index item: ` from the
minibuffer and jumped.

```elisp
(defun probe-cmd () (interactive) nil)
(global-set-key (kbd "C-c C-d") #'probe-cmd)
(execute-kbd-macro (kbd "C-c C-d C-c C-d"))
last-nonmenu-event
;; GNU                => nil
;; Neomacs before fix => 4
```

`imenu-choose-buffer-index` (`lisp/imenu.el:915`) chooses between the mouse menu
and the completing-read prompt with `(listp last-nonmenu-event)`, and `nil` is a
list.  With `imenu-use-popup-menu` at its default `on-mouse` GNU therefore takes
the menu path, which in a batch session selects nothing and leaves point where
it was.  A leftover integer sends Neomacs down the prompt path instead.

The variable is not a durable record of the last key pressed.  GNU's
`read_key_sequence` clears it at the top of every sequence read, at and after
the `replay_sequence:` label (`src/keyboard.c:11038-11054`), and only then
assigns the key it read (`src/keyboard.c:11668-11673`).  The sequence read that
discovers an exhausted keyboard macro therefore leaves it nil, which is what
Lisp observes once `execute-kbd-macro` returns.

Neomacs assigned the key but never cleared it, and its macro-exhausted case
returns before the reader's prologue.  The prologue is now one
`begin_key_sequence_read` called above that dispatch -- accumulator reset,
committed `this-command-keys` clear, and `last-nonmenu-event` clear together --
so neither branch can apply a subset.

Status: FIXED.

## 99. `accept-process-output` returned early on pending input, so a wait inside a keyboard macro did not wait -- FIXED

The Tern documentation workflow presses `C-c C-d` twice in one
`execute-kbd-macro`.  The first press's `post-command-hook` pumps
`accept-process-output` until the analyzer's reply lands, so the second press
sees `tern-last-docs-url` set and opens the URL.  In Neomacs the pump returned
instantly, the hook timed out, and the second press issued a second analyzer
request instead of opening anything.

```elisp
(defvar probe-log nil)
(defun probe-cmd ()
  (interactive)
  (let ((start (float-time)))
    (dotimes (_ 20) (accept-process-output nil 0.01))
    (push (- (float-time) start) probe-log)))
(global-set-key (kbd "C-c C-d") #'probe-cmd)
(execute-kbd-macro (kbd "C-c C-d C-c C-d"))
(nreverse probe-log)
;; GNU                => (0.201 0.202)
;; Neomacs before fix => (0.000 0.201)   ; the first command did not wait
```

The events of a running keyboard macro that have not executed yet are pending
input, which is why only the first of the two commands was affected: by the
second, the macro was exhausted.

GNU's `Faccept_process_output` calls `wait_reading_process_output` with
READ_KBD = 0 (`src/process.c:4957-4959`), and that value is exactly what
suppresses the return-on-input path.  With READ_KBD = 0 the loop calls
`swallow_events` when input is pending and keeps waiting; the `break` next to it
is `#if 0`-ed out under the comment "Exiting when read_kbd doesn't request that
seems wrong, though" (`src/process.c:5930-5937`).  The docstring states the same
contract: "if PROCESS is nil, the function should not be expected to return
before the timeout expires."

Neomacs built the request with a yield-on-command-input keyboard policy.  It now
uses the service-special-input-only policy, which is the typed spelling of
READ_KBD = 0: special events are still serviced, quit still interrupts, and
pending command input no longer completes the wait.  `sit-for` keeps yielding,
because GNU passes a non-zero READ_KBD there.

Status: FIXED.

## 100. A minibuffer read dropped an undo boundary into buffers an earlier command had edited -- FIXED

The Tide cross-file rename workflow renames a symbol, then renames files, each
step reading its argument from the minibuffer.  Every buffer the first rename
edited came out of the later steps with one more undo entry than GNU's -- 12
entries and 1 boundary in `src/main.js` against GNU's 11 and 0, 4 and 1 in
`src/math.js` against 3 and 0.

```elisp
(with-current-buffer (get-buffer-create "probe")
  (buffer-enable-undo)
  (setq buffer-undo-list nil)
  (insert "hello")
  (let ((setup (lambda ()
                 (setq unread-command-events
                       (append (listify-key-sequence (kbd "z RET"))
                               unread-command-events)))))
    (add-hook 'minibuffer-setup-hook setup)
    (unwind-protect (let ((executing-kbd-macro t)) (read-from-minibuffer "P: "))
      (remove-hook 'minibuffer-setup-hook setup)))
  buffer-undo-list)
;; GNU                => ((1 . 6) (t . 0))
;; Neomacs before fix => (nil (1 . 6) (t . 0))
```

`undo-auto--add-boundary` runs once per command-loop iteration and adds a
boundary to every buffer listed in `undo-auto--undoably-changed-buffers`
(`lisp/simple.el:4104-4116`).  A minibuffer read enters a command loop of its
own, so without further care the first minibuffer command would group the
edits of a command that has already finished.

GNU handles that where the recursive loop is entered.  `recursive_edit_1`
(`src/keyboard.c:708-748`) is the one entry both `recursive-edit` and
`read_minibuf` pass through, and it specbinds
`undo-auto--undoably-changed-buffers` to nil before calling `command_loop`,
under the comment "so that changes in the recursive edit will not result in
undo boundaries in buffers changed before we entered there recursive edit"
(Bug #23632).

Neomacs never bound it.

### Which command-loop entry owns the binding (amended 2026-08-15)

The first fix put the binding in `run_exit_wrapped_command_loop`, the function
both `recursive-edit` and the minibuffer command loop go through.  A follow-up
moved it down into `command_loop_1`, next to the `inhibit-redisplay` specbind,
on the reasoning that GNU keeps the two together and they must unwind at the
same boundary.

That reasoning had the right pair but the wrong level, and it regressed undo
for every keyboard macro.  GNU keeps *both* specbinds in `recursive_edit_1`
(`src/keyboard.c:738` and `747`) and puts neither in `command_loop_1` -- and
the distinction is the whole point, because `execute-kbd-macro` runs a command
loop *without* passing through `recursive_edit_1` (`src/macros.c`).  A macro is
supposed to build undo state that outlives it.

Rebinding on every command-loop entry meant each `execute-kbd-macro` got a
fresh nil list and threw it away on return, so the buffers the macro's LAST
command changed never received their boundary:

```elisp
(let ((buffer (generate-new-buffer "*pin*")))
  (set-window-buffer (selected-window) buffer)
  (set-buffer buffer)
  (text-mode)
  (buffer-enable-undo)
  (execute-kbd-macro (string-to-vector "abc"))
  (execute-kbd-macro (string-to-vector "def"))
  buffer-undo-list)
;; GNU                 => ((4 . 7) nil (1 . 4) (t . 0))
;; Neomacs after 08-14 => ((1 . 7) (t . 0))
```

Both halves of that follow: the boundary is missing, and because
`record_insert` coalesces into a newest record that ends where the new
insertion begins (`src/undo.c:98-112`), the two runs then merge into one
record.  The user-visible cost is that `undo`'s "get rid of initial undo
boundary" `undo-more` (`lisp/simple.el:3509-3511`) runs against a real group
instead of a boundary, so one `C-/` takes back *two* command groups.  Typing a
paragraph under `aggressive-fill-paragraph` and pressing `C-/` rewound to 41
characters where GNU rewinds to 62.

The binding is back at the recursive-edit entry in effect: `command_loop_1`
now takes a `CommandLoopEntry` saying which GNU entry point it is running for,
and rebinds `undo-auto--undoably-changed-buffers` only for
`CommandLoopEntry::RecursiveEdit`, never for
`CommandLoopEntry::KeyboardMacro`.  The enum exists because the failure is
silent in the direction that gets tested: recursive edits keep behaving, and
only keyboard macros -- which is how every melpa workflow types -- lose state.
`inhibit-redisplay` stays where it was, so its own pin is untouched.

## 101. `let` refused to bind a keyword to its own value -- FIXED

The Helm Org Rifle occur workflow renders its results buffer and Neomacs
signalled `(setting-constant :text)` where GNU renders.  The signal comes from
`dash`, not from Helm Org Rifle: the render loop destructures each result plist
with `(-let (((plist &as :text text . rest) entry)) ...)`, and `-let` expands a
keyword pattern into a plain `let*` binding of the keyword itself
(`dash--match-symbol`, dash.el), producing
`(let* ((:text (pop src)) (text (pop src)) (rest src)) ...)` where the value
popped off the plist IS `:text`.

```elisp
(list (condition-case e (let ((:text :text)) 1) (error e))
      (condition-case e (let ((:text 5)) 2) (error e))
      (condition-case e (let ((t t)) 3) (error e)))
;; GNU                => (1 (setting-constant :text) (setting-constant t))
;; Neomacs before fix => ((setting-constant :text) (setting-constant :text)
;;                        (setting-constant t))
```

GNU's `let`/`let*` have no constant check of their own.  `Flet`/`Flet_star`
just `specbind`, and the refusal comes from `do_specbind`
(`src/eval.c:3597-3604`) handing a trapped-write symbol to `set_internal`,
whose `SYMBOL_NOWRITE` arm makes one exception:

```c
case SYMBOL_NOWRITE:
  if (NILP (Fkeywordp (symbol)) || !EQ (newval, Fsymbol_value (symbol)))
    xsignal1 (Qsetting_constant, symbol);
  else
    /* Allow setting keywords to their own value.  */
    return;
```

(`src/data.c:1687-1697`; `set_default_internal` repeats it verbatim at
`src/data.c:2039-2049`.)  Neomacs asked only whether the symbol was a
constant, at four separate sites.

`Obarray::classify_constant_write` is now the single place that spells GNU's
rule, returning `Writable`, `KeywordSelfAssign` or `Refused`, and the four
sites that mirror `set_internal` ask it: interpreted `let`, `let*`, the VM's
two assignment paths, and the `set` builtin, which already carried a
hand-rolled copy of the keyword exception and now shares this one.  Sites that
mirror GNU's other constant test -- `make-variable-buffer-local`,
`make-local-variable`, `makunbound`, `fset` -- keep refusing unconditionally,
because GNU's `SYMBOL_CONSTANT_P` there has no keyword exception.

Status: FIXED.

## 102. `end-of-file` was raised without the stream that hit it -- FIXED

Company Statistics loads a truncated history file with plain `load`.  GNU's
error data carries the buffer the reader was reading, which
`load-with-code-conversion` has already killed by the time a handler sees it;
Neomacs raised a bare `(end-of-file)`, so the package's recorded condition
lost both the datum and the rendered message.

This reduction needs one file on disk rather than a single form:

```elisp
(let ((f (expand-file-name "tmp/divergence-98.el")))
  (with-temp-file f (insert "(setq foo '(1 2\n"))
  (condition-case e (load f 'noerror nil 'nosuffix)
    (t (list (car e) (cdr e) (error-message-string e)))))
;; GNU                => (end-of-file (#<killed buffer>)
;;                        "End of file during parsing: #<killed buffer>")
;; Neomacs before fix => (end-of-file nil "End of file during parsing")
```

GNU decides the datum from the STREAM, in one function:

```c
static AVOID
end_of_file_error (source_t *source)
{
  if (from_file_p (source))
    /* Only Fload calls read on a file, and Fload always binds
       load-true-file-name around the call.  */
    xsignal1 (Qend_of_file, Vload_true_file_name);
  else if (from_buffer_p (source))
    xsignal1 (Qend_of_file, source->object);
  else
    xsignal0 (Qend_of_file);
}
```

(`src/lread.c:2121-2132`).  Loading a source file reaches the buffer arm,
because `load-source-file-function` is `load-with-code-conversion`, which reads
the text in a temp buffer through `eval-buffer`.

Neomacs's `eval-buffer` already handed the buffer to its own readevalloop, but
the arm it takes while a load is in progress went through `load.rs`, whose
`read_error_for_load` assembled `end-of-file` data of its own and assembled it
empty.  `ReadSourceObject` now names GNU's three arms and every readevalloop
carries one, so no raise site invents `end-of-file` data: `eval-buffer` during
a load passes the buffer, and `load` reading a file itself -- a `.elc`, or a
source file when `load-source-file-function` is nil -- passes the
`load-true-file-name` it just bound.  A `(read)` inside a loaded form reads the
same stream and reports the same datum.

Status: FIXED.

## 103. `format` lost a format-string property that spanned into a conversion -- FIXED

The ggtags xref workflow reads the face runs of the `*xref*` buffer.  GNU's
runs include `("3" xref-line-number)` for the line number; Neomacs's did not,
and the run was missing rather than misfaced -- `xref-line-number` is an
undefined face in both editors, so this is a text property that never arrived.

```elisp
(format #("%1d:" 0 2 (face xref-line-number) 3 4 (face shadow)) 3)
;; GNU                => #("3:" 0 1 (face xref-line-number) 1 2 (face shadow))
;; Neomacs before fix => #("3:" 1 2 (face shadow))
```

`xref--insert-xrefs` builds that argument itself, by formatting a propertized
format string twice (`lisp/progmodes/xref.el:1160-1184`):

```elisp
(format #("%%%dd:" 0 4 (face xref-line-number) 5 6 (face shadow))
        (1+ (floor (log max-line 10))))
```

whose result the line number is then formatted through.  The second pass's
`xref-line-number` range covers `"%1d"`, so its end falls INSIDE the conversion
specification.

GNU carries a format string's own text properties into the result
(`src/editfns.c:4303-4377`) by walking the format string against a
`discarded[]` table.  The specification's first character is
`discarded[] == 1`, and passing it jumps `translated` over the whole converted
field, so a boundary interior to a specification lands at the field's END.
Neomacs mapped an interior boundary to the field's START, which keeps the
translation monotonic but collapses this range to nothing.

`FormatSourceSpan` now records which of GNU's three shapes produced it.  A
conversion sends an interior boundary to the field end; `%%` sends it to the
start, because its discarded `%` has no field to jump over and GNU drops the
property there too; a literal character has no interior at all.

Status: FIXED.

## 104. An XML entity reference split its text node in three -- FIXED

The org-ref LaTeX/CSL export workflow raised
`(wrong-type-argument listp "edited ")` where GNU completes the export.
`"edited "` is the first half of a CSL locale term:

```elisp
(with-temp-buffer
  (insert "<r><term form=\"verb\">edited &amp; translated by</term></r>")
  (libxml-parse-xml-region (point-min) (point-max)))
;; GNU                => (r nil (term ((form . "verb")) "edited & translated by"))
;; Neomacs before fix => (r nil (term ((form . "verb")) "edited " " translated by"))
```

libxml2 substitutes the predefined entities and character references into the
character data it is accumulating, so the element reaches GNU's `make_dom`
(`src/xml.c:123-160`) as ONE `XML_TEXT_NODE` and comes back as one string
child.  `quick_xml` reports the reference as its own `Event::GeneralRef`
between two `Event::Text`s, and `parse_xml_region` had no arm for it: the
resolved character was dropped and the element came back with three children.

citeproc branches on exactly that count while reading the locale
(`citeproc-term.el`, `citeproc-term--from-xml-frag`):

```elisp
(if (= (length frag) 2)
    (setf (citeproc-term-text term) (cadr frag))
  (setf (citeproc-term-text term) (cl-caddr (cadr frag)))
  ...)
```

so the split sent it down the two-form branch, where `cl-caddr` on the string
`"edited "` signals `(wrong-type-argument listp "edited ")`.

Character data now accumulates across `Text` and `GeneralRef` events and is
emitted as one string when an element boundary, comment, CDATA section or EOF
ends the run.  CDATA stays its own node, because libxml2 keeps it as its own
`XML_CDATA_SECTION_NODE` and GNU turns that into its own string child.  A run
that resolved a reference is never treated as ignorable whitespace, since
`XML_PARSE_NOBLANKS` only drops text that was blank in the source.

Status: FIXED.

## 105. The first-change undo entry recorded a constant instead of the visited-file modtime -- FIXED

The org-ref prefix-completion workflow inserts a citation, undoes it with
`C-_`, and records the document state.  GNU reports `:modified nil` after the
undo; Neomacs reported `:modified t`.  That was the whole difference between
the two editors on that workflow.

```elisp
(progn (setq buffer-file-name (expand-file-name "tmp/divergence-101.txt"))
       (with-temp-file buffer-file-name (insert ""))
       (set-visited-file-modtime)
       (setq buffer-undo-list nil)
       (set-buffer-modified-p nil)
       (insert "hello")
       (let ((entries buffer-undo-list))
         (primitive-undo 1 entries)
         (list :recorded (cdr (assq t entries))
               :after-undo (buffer-modified-p))))
;; GNU                => (:recorded (27263 16044 798808 306000) :after-undo nil)
;; Neomacs before fix => (:recorded 0 :after-undo t)
```

GNU's `record_first_change` (`src/undo.c:209-223`) stores the buffer's
visited-file modification time:

```c
bset_undo_list (current_buffer,
                Fcons (Fcons (Qt, buffer_visited_file_modtime (base_buffer)),
                       BVAR (current_buffer, undo_list)));
```

and that datum is the entry's entire purpose.  `primitive-undo`'s `(t . TIME)`
arm (`lisp/simple.el:3669-3688`) clears the modified flag only when
`(time-equal-p time (visited-file-modtime))`, so that undoing back to a save
the file has since outlived does not claim the buffer is unmodified.  Neomacs
recorded the constant `(t . 0)` for every buffer, so the comparison could never
succeed for a file-visiting buffer.

`Buffer::visited_file_modtime_value` now spells GNU's
`buffer_visited_file_modtime` (`src/fileio.c:6156-6163`) once, and both the
recorder and the `visited-file-modtime` builtin read it -- they have to agree
or `primitive-undo` could never match them.

Residue: GNU's `record_first_change` redirects an indirect buffer to its base
buffer's modtime.  A `Buffer` here cannot reach the buffer manager, so an
indirect buffer still records 0 -- strictly narrower than before, when every
buffer did.
## 106. `end-of-visual-line` stopped one column short on a newline-terminated row -- FIXED

MWIM binds `C-e` to `end-of-visual-line`, so its terminal workflow ends every
visual row of a `visual-line-mode` buffer.  On the LAST visual row of a logical
line Neomacs landed on the row's last character where GNU lands on the newline:
`MWIM-MOVE e=8` is `p=83 col=82` in GNU and was `p=82 col=81` here.  This is the
second defect in the area ledger 90 opened; 90 fixed how a word-wrap break is
DRAWN, this one is the wrap/end POSITION itself.

The package-free reduction is `vertical-motion` with a goal column, in a
24-column `visual-line-mode` window over
`"  alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron\n"`.
Run under GNU in a PTY, every screen line answers `next-screen-line-start - 1`:

```elisp
;; (vertical-motion 0) / (vertical-motion (cons 24 0)) / (vertical-motion 1)
;; GNU               P45  bol=43 eovl=59 vm1=60      ; row closed by a wrap
;; GNU               P80  bol=60 eovl=83 vm1=84      ; row closed by a newline
;; Neomacs before    P45  bol=43 eovl=59 vm1=60
;; Neomacs before    P80  bol=60 eovl=82 vm1=84
```

`end-of-visual-line` is exactly `(vertical-motion (cons (window-width) 0))`
(lisp/simple.el:8546-8558), and this is the interactive display-iterator path,
not the `noninteractive` `vmotion` branch of ledger 71: `Fvertical_motion`
takes the `else` arm at src/indent.c:2287 and reaches the goal column with
`move_it_in_display_line (&it, ZV, first_x + to_x, MOVE_TO_X)`
(src/indent.c:2540).  `move_it_in_display_line_to` (src/xdisp.c:10105) has TWO
ways to stop: at a glyph that reaches the goal x, or -- when the goal is past
everything the row draws -- where the display line itself ENDS, at the newline
it refuses to consume.  A wrap-closed row's end coincides with its last glyph,
so only newline-closed rows exposed the gap.

Neomacs answered the goal column from the redisplay snapshot's `points`, which
carry only DRAWN glyphs; a newline draws none, so the row's end was not a
candidate at all and the walk settled on the last glyph one column back.  The
row's end was already published, correctly, as `DisplayRowSnapshot::end_col` /
`end_x` / `end_buffer_pos` -- for a newline-closed row `end_buffer_pos` is the
newline, one column past the last glyph.  The fix names both exits as one kind,
`RowGoalStop`, and has the goal-column walk range over glyph stops chained with
the row's end stop, so the second exit cannot be forgotten by whichever branch
built the candidate list.

Status: FIXED.

## 107. `recenter` counted buffer lines where GNU counts screen lines -- FIXED

`helm-css-scss--recenter` is `(recenter (/ (window-height) 2))`, so every Helm
candidate move re-anchors the source window.  With one line of the SCSS fixture
folded away by an `invisible` overlay, Neomacs put window-start one line lower
than GNU, and the source pane rendered `  color: red;` at its top row where
GNU renders `.dashboard--compact {`.

The package-free reduction needs no package at all -- a 23-line buffer, line 10
hidden by an `invisible` overlay, point on line 20, run in a PTY under both
editors:

```elisp
;; hidden line 10, point on line 20
;; (vertical-motion -12)  => line 7   GNU and Neomacs agree
;; (recenter 12)          => line 7   GNU
;;                        => line 8   Neomacs before this fix
;; with nothing hidden both answer line 8, so only the hidden line diverged
```

GNU's positive-ARG branch of `Frecenter` runs the display iterator:
`start_display`, `move_it_by_lines (&it, 0)` onto the head of the screen line
holding point, then `move_it_by_lines (&it, -nlines)`
(src/window.c:7395-7407).  That is the same machinery `vertical-motion` uses,
so invisible text is stepped over without consuming one of the ARG lines.

Neomacs had the seam already -- `screen_line_motion_target`, whose own doc
calls itself "the shared display-motion seam used by both `vertical-motion` and
window scrolling" -- and `recenter` simply never opted into it, walking raw
buffer newlines with `prev_newline_emacs_byte` instead.  That is the ledger-95
shape again: a rule every caller has to remember, which GNU applies once, in the
iterator.  `recenter` now resolves its origin and then goes through the seam, so
invisible text, continuation rows and display properties all count the way
redisplay counts them.

Residual, NOT this divergence: `helm_css_scss_public_tui_workflows_match_gnu`
still fails.  The grid assertion this entry is about
(helm_css_scss_test.rs:1125, "filtered real Helm grid differs") now passes, and
the failure moved to `report_expect` at helm_css_scss_test.rs:1500, which pins
GNU's OWN ledger and no longer matches live GNU: at `:stage cancel` GNU reports
`:windows ("tui-fixture.scss")` where the pin says
`("tui-fixture.scss" :other)`.  Neomacs cannot influence that side.  A
`UPDATE_EXPECT=1` run rewrites three GNU pins (1500, 1751, 1804) and then
reaches a further, separately-masked failure in the MULTI-buffer phase
("Neomacs multi action returned did not reach the expected terminal state"), so
the pins were deliberately left untouched rather than refreshed blind.  Two more
layers live behind this one and want their own ledger entries.

Status: FIXED.

## 108. The terminal writer quantized 256-colors with its own heuristic -- FIXED

Under Gruvbox's 256-color LIGHT profile, `font-lock-comment-face`,
`org-verbatim` and `org-document-info-keyword` came out as `38;5;248` where GNU
emits `38;5;145`.

Several tempting explanations were probed under both editors first and all came
back byte-identical: `tty-color-approximate`, `tty-color-desc`,
`tty-color-translate`, the whole 256-entry `tty-color-alist`, and the resolved
face value (`#afafaf`, which those functions map to 145 in BOTH editors), across
a dark -> light -> `disable-theme` lifecycle.  Every one of those probes was
right, and every one of them missed the defect, because none of them is what
draws the glyph.

GNU does not quantize in the writer at all.  `turn_on_face` (src/term.c) emits
the index the realized face already carries, and that index came from
`tty-color-approximate` (lisp/term/tty-colors.el:875-915), which scans the WHOLE
palette for the smallest squared 8-bit RGB distance, skipping candidates on the
gray diagonal when the requested color is 0.065 radians or more off it
(`tty-color-off-gray-diag`, tty-colors.el:866-873).

Neomacs' TTY writer carried a SECOND, independent quantizer: a hand-rolled
`rgb_to_256` that short-circuited any color with `max - min < 8` straight into
the 24-step grayscale ramp, and bucketed the rest through a coarse cube-level
table.  `#afafaf` is an EXACT 6x6x6 cube entry (145), but the gray short-circuit
never looked at the cube and answered 248 (`#a8a8a8`, distance 147).  That is why
the divergence was invisible to every Lisp-level probe: the Lisp palette was
right and only the writer disagreed.

Measured against 41 colors read out of GNU -- `emacs -Q -nw` in a PTY with
COLORTERM unset, `(nth 1 (tty-color-approximate (tty-color-standard-values C)))`
-- the old function was wrong on 13 of them, well beyond the reported face:

```
;;              GNU   before
;; #afafaf      145      248     ; the reported one
;; #ebdbb2      187      223
;; #665c54       95       59
;; #504945      239       59
;; #ffffff       15      231     ; the 16 system colors lead tty-color-alist,
;; #000000        0       16     ; so they win an exact tie
```

`rgb_to_256` is now the search GNU's Lisp does: all 256 candidates in the same
ascending order the alist is consulted (so exact ties resolve identically),
squared 8-bit RGB distance, and GNU's gray-diagonal exclusion.  All 41 GNU
answers are pinned as a test.

Status: FIXED.

## 109. A self-inserted newline never filled the line it just closed -- FIXED

`git-commit-mode` turns on auto-fill and pins `git-commit-fill-column`, so a
typed commit message wraps as you type.  With `fill-column` 24 the fixture types
`"Summary words continue beyond configured boundary\n\n"` and GNU leaves
`"Summary words continue\nbeyond configured\nboundary"`; Neomacs left the last
two words joined as `"beyond configured boundary"`, 26 columns wide.

Auto-fill does not run per character.  `internal_self_insert` (src/cmds.c:477)
consults `auto-fill-chars`, whose default entries are SPACE and NEWLINE, so only
those two characters can fill.  Every space in the fixture fills correctly here;
the newline is the one that did nothing, because of what GNU does around the
call:

```c
      if (c == '\n')
	/* After inserting a newline, move to previous line and fill
	   that.  Must have the newline in place already so filling and
	   justification, if any, know where the end is going to be.  */
	SET_PT_BOTH (PT - 1, PT_BYTE - 1);
      auto_fill_result = call0 (Qinternal_auto_fill);
      /* Test PT < ZV in case the auto-fill-function is strange.  */
      if (c == '\n' && PT < ZV)
	SET_PT_BOTH (PT + 1, PT_BYTE + 1);
```

(src/cmds.c:484-492.)  `do-auto-fill` starts from `(current-column)`
(lisp/simple.el:9081), and after inserting a newline that column is 0, so
without the one-character step back the filler is asked to fill the empty line
the newline just opened instead of the finished line it closed.  Neomacs called
`internal-auto-fill` with no such step, so RET was a no-op for filling and only
SPACE ever wrapped anything.

Reduced to no package at all, in a 24-column `text-mode` buffer typed through
`self-insert-command`:

```elisp
;; GNU              "Summary words continue\nbeyond configured\nboundary\n\n"
;; Neomacs before   "Summary words continue\nbeyond configured boundary\n\n"
```

The fix names the straddle instead of open-coding `ch == '\n'` at both sides of
the call: `NewlineFillStraddle::for_self_inserted_char` decides once, and its
`step` method carries GNU's asymmetry -- the backward step is unconditional, the
forward step is guarded by `PT < ZV` -- so neither half can be applied without
the other.  The SPACE path is pinned alongside it, because it must keep leaving
point after the space.

Status: FIXED.

## 110. A nested comment's inner ender ended the whole comment for font-lock -- FIXED

`scala-mode` marks block comments nestable -- `?/` is `". 124b"` and `?*` is
`". 23n"` (scala-mode-syntax.el:498-499) -- so `/* outer /* nested */ done */`
is one comment.  Neomacs fontified only `/* outer /* nested */`: the
`font-lock-comment-face` run ended at 454 where GNU ends it at 462, dropping
`" done */"`.

`forward-comment` and `parse-partial-sexp`'s comment DEPTH were both already
nesting-aware; the defect was in the third thing the syntax parser owes its
callers, the parse BOUNDARY.  `font-lock-fontify-syntactically-region` walks a
region with `(parse-partial-sexp point end nil nil state 'syntax-table)`, and
that `'syntax-table` argument means "stop as soon as a comment or string starts
or ends".  GNU stops exactly twice per comment because `scan_sexps_forward`
consumes a whole comment, nesting included, in ONE `forw_comment` call
(src/syntax.c:3352) and only the code after that call clears `state->incomment`
and honours `boundary_stop` (src/syntax.c:3370-3374).  Neomacs walked the
comment character by character and treated every same-style ender as a boundary,
so the inner `*/` cut the run even though the parse state it returned correctly
still said depth 1.

Package-free, on scala-mode's syntax table over
`"  /* outer /* nested */ done */\nafter\n"`, driving font-lock's own loop and
recording `(point (nth 4 state) (nth 8 state))` at every stop:

```elisp
;; GNU              ((5 1 3) (32 nil nil) (39 nil nil))
;; Neomacs before   ((5 1 3) (24 1 3) (32 nil nil) (39 nil nil))
```

Stop 2 is the bug: position 24 is just past the inner `*/`, and Neomacs reported
it as a boundary while simultaneously reporting `nth 4` = 1, "still in a
comment" -- a boundary the caller cannot act on.

The fix gives the ender a return value.  `close_comment_level` pops one nesting
level and answers `CommentEnderEffect::NestingLevelClosed` or
`CommentEnderEffect::CommentClosed`; only the latter is a `'syntax-table`
boundary.  Both ender branches (one-character and two-character) route through
it, so the distinction cannot be re-derived differently in one of them.

Two differences remained in this suite after that fix and neither was this bug.
Both are now resolved; what is still red is a third case neither of them named.

The first was the coding system.  The comment run itself became byte-identical
to GNU -- the first divergence in
`activates_unicode_scala_files_with_real_syntax_and_prettify` moved from
character 7347 to character 10847 -- and what sat at 10847 was that `demo.sbt`
and `demo.worksheet.sc`, both pure-ASCII fixtures, had
`buffer-file-coding-system` `utf-8-unix` in GNU and `undecided-unix` here.  A
standalone probe visiting ASCII, Unicode and empty files answered identically
in both editors, so the promotion came from the suite's world rather than from
coding detection.  It comes from the package: `scala-mode-autoloads.el:49`
(and `scala-mode.el:183`) runs

```elisp
(modify-coding-system-alist 'file "\\.\\(scala\\|sbt\\|worksheet\\.sc\\)\\'" 'utf-8)
```

which pushes an entry onto `file-coding-system-alist`.  Package-free in live
GNU, before and after that single call:

```elisp
;; before   ("b.sbt" undecided-unix)
;; after    ("a.sbt" utf-8-unix)  ("a.worksheet.sc" utf-8-unix)
;;          ("a.scalax" undecided-unix)
```

The `.scalax` near-miss the suite also pins stays `undecided-unix` because the
regexp does not match it, which is the same answer both editors already gave.

Neomacs never asked.  GNU decides the read coding system on a four-rung ladder
and stops at the first rung that answers: `coding-system-for-read`
(src/fileio.c:4317-4318), then `set-auto-coding-function` (:4401-4402 for a
non-empty buffer, :5051-5055 for the empty-buffer path), then
`file-coding-system-alist` through `Ffind_operation_coding_system`
(:4411-4420 and :5057-5066, taking `XCAR` of a cons answer), then `undecided`
(:4423-4424).  Neomacs had rungs one, two and four.  Rung three was missing
entirely, so `find-operation-coding-system` was a correct builtin that nothing
on the file-reading path ever called.

The fix names the ladder.  `ReadCodingDecision` carries one variant per rung,
so the decision records WHICH source answered instead of collapsing into a
chain of `Option::or`, and a rung cannot be dropped without deleting a variant.

Building it exposed a second defect underneath.  `auto_coding_system_name`
turned the hook's `nil` -- GNU's "I have not decided" -- into the coding-system
NAME `"nil"`, because `nil` is a symbol and the function only asked whether it
had a symbol name.  That was harmless while the sole consumer filtered the
string `"nil"` back out just before decoding.  It stopped being harmless the
moment a later rung had to ask "did rung two answer?", because rung two then
always claimed it had.  With `set-auto-coding-function` bound to a function
that returns `nil` -- the ordinary case -- the alist rung was unreachable, and
the whole fix was inert until the guard was added.  It now returns "no answer"
for `nil`, matching GNU's `NILP (coding_system)` test at src/fileio.c:4411
and :5057.

The second difference was in
`types_and_reindents_scala_through_one_real_command_loop`, whose
`:undo-recovery :returned` was 225 here and 227 in GNU.  Re-measured on current
HEAD it is gone: the case passes, and it was the undo work in ledgers 97, 100,
105, 109 and 115 that closed it, not anything here.

The suite's one remaining red case is
`organises_imports_warns_and_recovers_through_public_commands`, which neither
residual named and which was already red before this fix.  After `undo`,
Neomacs leaves point at `point-min` where GNU leaves it at the change it undid:
`:undone` point 1 (line 1) here against GNU's 271 (line 13), and the `:twice`
state that follows from it 1 against GNU's 15.  The buffer text agrees at every
step; only point differs.  It wants its own investigation and its own entry.

Status: FIXED.

## 111. An explicit window hscroll was reset to 0 by the next redisplay -- FIXED

`buffer-move` deliberately preserves horizontal scroll across a swap: it reads
`(window-hscroll window)` before and calls `set-window-hscroll` after
(buffer-move.el:106-115).  In the four-pane fixture GNU ends with hscrolls
`2 11 5 8`; Neomacs ended with `0 0 0 0`.  The values were correct in the
`:before` snapshot, so nothing was wrong with `set-window-hscroll` or
`window-hscroll` themselves -- they were correct right up until a redisplay ran,
which is why a batch probe that only sets and reads them reproduces nothing.

GNU's auto-hscroll pass has three triggers (`hscroll_window_tree`,
src/xdisp.c:16755-16786): point inside the left margin while hscrolled, point
inside the right margin on a right-truncated line, and

```c
		  || (hscl
		      && w->hscroll != w->min_hscroll
		      && !cursor_row->truncated_on_left_p)
```

the "moved onto a short line" reset.  `hscl` is `hscrolling_current_line_p (w)`
(src/xdisp.c:3074), which is true only when `auto-hscroll-mode` is the symbol
`current-line`.  Under the default `t` that third trigger is dead code, and an
explicitly set hscroll survives redisplay.  Neomacs' port had the clause -- and
even quoted `hscl` in its comment -- but dropped it from the expression, so any
window whose point sat to the RIGHT of its hscroll (where the other two triggers
stay quiet) was reset to 0 on the first redisplay after the command.

The fix restores the clause and, so it cannot be lost again, replaces the raw
`auto-hscroll-mode` Lisp value with a three-case `AutoHscrollMode`
(`Off` / `CurrentLine` / `AllLines`).  GNU reads that variable for two different
questions -- "is auto hscroll on at all" and "is it `current-line`" -- and only
the first degrades safely to "non-nil"; naming the cases keeps the second from
degrading into the first.  `hscl` is computed from the pre-pass suspend flag,
matching GNU, which evaluates it at src/xdisp.c:16644 before the STEP 4
un-suspend.

Status: FIXED.

## 112. `recursive-edit` returned with the wrong buffer current -- FIXED

Quelpa runs every Git command through `quelpa--run`, which in asynchronous mode
parks in `(recursive-edit)` and lets the process sentinel call
`exit-recursive-edit` (quelpa.el:611-627).  The caller,
`quelpa-build--run-process-match` (quelpa.el:654-661), is a `with-temp-buffer`
that runs the process into that temp buffer and then searches it:

```elisp
  (with-temp-buffer
    (apply 'quelpa-build--run-process dir prog args)
    (goto-char (point-min))
    (re-search-forward regexp)
    (match-string-no-properties 1))
```

Neomacs failed the very first such call with
`Search failed: "git version \(.*\)"`, and the natural reading -- that the
process output never reached the buffer -- is wrong.  Instrumenting the sentinel
shows the output IS in the buffer while the sentinel runs; it is the SEARCH that
is in the wrong place, because `recursive-edit` returned with a different buffer
current.

GNU saves and restores it.  `Frecursive_edit` (src/keyboard.c:811-816) records
the current buffer, but only when it is not the selected window's buffer:

```c
  if (command_loop_level >= 0
      && current_buffer != XBUFFER (XWINDOW (selected_window)->contents))
    buffer = Fcurrent_buffer ();
  else
    buffer = Qnil;
  ...
  record_unwind_protect (recursive_edit_unwind, buffer);
```

and `recursive_edit_unwind` (src/keyboard.c:837-844) does `Fset_buffer (buffer)`
on every exit -- including the `(throw 'exit ...)` that `exit-recursive-edit`
uses.  Neomacs' `recursive-edit` had no such record, so the command loop's own
buffer selection leaked out to the caller.

Reduced to no package, running `sh -c "echo hello"` inside `with-temp-buffer`
with a sentinel that exits the recursive edit:

```elisp
;; in-sentinel is the buffer's text while the sentinel runs;
;; after is (buffer-string) once the wait loop returns.
;; GNU              in-sentinel="hello\n"  after="hello\n"
;; Neomacs before   in-sentinel="hello\n"  after=""
;; sleep-for and accept-process-output waiters were already correct in both.
```

The fix is `RecursiveEditBuffer`, recorded by `recursive-edit` and applied on
unwind.  Its two cases are GNU's two cases: `SelectedWindows`, where the command
loop leaves the right buffer current by itself and GNU deliberately records
nothing, and `Restore(BufferId)`, a buffer only the calling Lisp frame knows
about.  Making the "record nothing" case a named value rather than an absent
`if` is the point: it is the case that made the omission invisible for every
`recursive-edit` entered from a window's own buffer.

Status: FIXED.
## 113. `window-scroll-functions` existed but nothing on the scroll path ever ran it -- FIXED

`beacon` never blinked on a scroll.  After `C-v` GNU reports seven overlays, a
live blink timer and one `beacon-before-blink-hook` call (`o=7 t=t n=1`);
Neomacs reported `o=0 t=nil n=0`.

`beacon` arms itself entirely from the hook:

```elisp
;; beacon.el:441-459
(defun beacon--window-scroll-function (window start-pos)
  (unless (or (and (equal beacon--previous-window-start start-pos)
                   (equal beacon--previous-window window))
              (not beacon-blink-when-window-scrolls))
    (if this-command
        (setq beacon--window-scrolled window)
      (setq beacon--window-scrolled nil)
      (beacon-blink-automated))))
;; beacon.el:479
(add-hook 'window-scroll-functions #'beacon--window-scroll-function)
```

so the blink after `C-v` happens inside redisplay itself: GNU's command loop
clears `this-command` before `read_key_sequence` (src/keyboard.c:1417), the
redisplay inside `read_char` then runs the scroll hook with `this-command` nil,
and `beacon` takes the immediate `beacon-blink-automated` branch.

Neomacs had the hook, the variable and even the builtin
(`run-window-scroll-functions`) -- and called it from exactly one place, the end
of `set-window-buffer` (window_cmds/mod.rs:4754).  Redisplay never called it.
This is ledger 94's shape exactly: the hook exists, nothing consults it on the
path that matters.

GNU calls `run_window_scroll_functions` (src/xdisp.c:19222) from
`redisplay_window` at each site where redisplay commits a window start it did
not simply inherit: the `w->force_start` branch (src/xdisp.c:20728), the
`try_scrolling` result (src/xdisp.c:19645), and the recenter fallback
(src/xdisp.c:21227).  A `try_window` that just works from the start it was
handed reaches none of them, which is why an idle redisplay is silent.

`Context::publish_redisplay_window_positions` is Neomacs's one seam where
redisplay writes the settled start back onto the window -- it already knew both
the old start and whether `force_start` was armed, because it is the code that
clears `force_start`.  It now returns a `WindowStartCommit`
(`Committed` / `Inherited`), and the publication site runs the hook for a
`Committed` start.  The enum is `#[must_use]`, so this cannot decay into a rule
each future publication site has to remember -- the ledger 95 / 107 failure mode.

Two deliberate narrowings, both matching GNU: the preliminary
`resize_mini_window` measurement publication is not that redisplay and stays
silent, and the start is published before the hook runs so a hook that moves the
start wins (GNU re-reads `w->start` after the call).  Unlike GNU we do not
re-lay the window inside the same pass; the next redisplay picks the moved start
up.  The call binds `inhibit-redisplay` and demotes errors, like every other
Lisp seam redisplay already runs.

Status: FIXED.

## 114. Helm's exit-time source resolution diverges in a multi-source session -- FIXED

Unmasked by refreshing the three stale GNU pins in the helm-css-scss
single-buffer suite (see the refresh commit; ledger 107 predicted this layer).
`helm_css_scss_public_tui_workflows_match_gnu` now clears the single-buffer
phase and fails in `helm_css_scss_named_display_adapter_drives_real_multi_buffer_helm`
at "Neomacs multi action returned": after `C-o` (next source), `C-n` and `RET`,
GNU ends up in `component.css` at point 52 and Neomacs stays in
`tui-fixture.scss`.

`helm-css-scss--multi` bakes one action per source, closing over that source's
buffer:

```elisp
;; helm-css-scss.el:697-702
(action ("Goto open brace" . (lambda ($po) (switch-to-buffer ,$buf) ...)))
```

Tracing `switch-to-buffer` through the whole session shows the action DID run in
both, with a different argument: GNU called it with `component.css`, Neomacs
with `tui-fixture.scss`.  So the divergence is which source Helm resolves at
exit, not `switch-to-buffer` (a direct `save-current-buffer` /
`with-current-buffer` + `switch-to-buffer` probe agrees between the two
editors), and not the Helm buffer's text properties: the full
`helm-cur-source` property map over `*Helm Css SCSS multi buffers*` is
byte-identical between GNU and Neomacs

```
((1 nil) (18 "tui-fixture.scss") (52 nil) (53 "tui-fixture.scss") (94 nil)
 (95 "tui-fixture.scss") (158 nil) (159 "tui-fixture.scss") (170 nil)
 (186 "component.css") (196 nil) (197 "component.css") (213 nil)
 (226 "theme.less") (235 nil) (236 "theme.less") (251 nil)
 (265 "IGNORED.CSS") (289 nil))
```

so the remaining suspect is where point / `helm-selection-overlay` sits in the
Helm buffer when Helm saves its source on exit, which `helm-css-scss` reaches
through its own advice on `helm-next-line` and `helm-move--next-line-fn`.  Note
that the plain-text grid assertion before `RET` cannot see this: it compares
text, and the selection differs only in the highlight.

### Mechanism

Point in the Helm buffer, and neither the advice nor the overlay.  A probe that
logged the whole Helm state after every command, plus `before`/`after` notes
around `exit-minibuffer`, `bury-buffer`, `switch-to-prev-buffer`,
`set-window-buffer` and `set-window-configuration`, is byte-identical between
the two editors up to the last command:

```
GNU  post-command helm-next-line :helm-point 197 :helm-window-point 197 :get-current-source "component.css"
Neo  post-command helm-next-line :helm-point 197 :helm-window-point 197 :get-current-source "component.css"
```

and then diverges across ONE unadvisable step -- the C-level window-configuration
restore that `read_minibuf` unwinds on minibuffer exit (`src/minibuf.c:695-698`):

```
GNU  minibuffer-exit-hook :helm-point 197 ... -> before bury-buffer :helm-point 197 :helm-window 197
Neo  minibuffer-exit-hook :helm-point 197 ... -> before bury-buffer :helm-point  53 :helm-window  53
```

53 is the preselected candidate: the position the Helm window had when Helm
entered its minibuffer.  Neomacs restored it, so `helm-execute-selection-action-1`
read `tui-fixture.scss` out of `(get-text-property (point) 'helm-cur-source)`
and ran that source's baked action.

`current-window-configuration` deliberately does not record point in the buffer
that was current when it ran ("does not save the value of point in the current
buffer"), and `Fset_window_configuration` enforces that by recomputing
`old_point` from the LIVE session before touching anything
(`src/window.c:7692-7733`) and writing it back over the window that was selected
at save time, after the saved tree is installed (`src/window.c:7978-7984`):

```c
      /* Arrange *not* to restore point in the buffer that was
	 current when the window configuration was saved.  */
      if (EQ (XWINDOW (data->current_window)->contents, new_current_buffer))
	set_marker_restricted (XWINDOW (data->current_window)->pointm,
			       make_fixnum (old_point),
			       XWINDOW (data->current_window)->contents);
```

Helm hits this exactly: `helm-internal` displays its buffer and does
`(select-window (helm-window))` before `read-from-minibuffer`, so the Helm window
is the saved-selected window and the Helm buffer is the saved current buffer.
Reduced to a batch A/B (`emacs -Q --batch`, same file for both editors):

```elisp
(select-window w)                       ; w shows B, B is current
(setq conf (current-window-configuration))
(select-window other)
(set-window-point w 13) (with-current-buffer b (goto-char 13))
(set-window-configuration conf)
(list (window-point w) (with-current-buffer b (point)))
;; GNU               => (13 13)   live point kept
;; Neomacs before fix => (1 1)    saved point restored
```

Neomacs restored the whole saved window tree, points included, so every window
got its snapshot point back.  The fix computes the same exclusion GNU does, once,
above the restore, and carries it as one value -- `LiveCurrentBufferPoint`
(window + the buffer that window must still show + the live point) -- so no
branch can apply half of it.  Two neomacs unit tests had pinned the pre-GNU
behaviour (`(10 10)` and `[10, 37]` for a `save-window-excursion`-shaped round
trip); GNU answers `(3 3)` and `(10 10)`, and both were corrected against live
GNU rather than kept.

The nearby `helm-css-scss--recenter:` prefix asymmetry in the error line was a
red herring: with the probe attached, both editors raise it from the same frame
(`recenter` <- `helm-css-scss--recenter` <- `helm-css-scss--move-line-action` <-
`ad-Advice-helm-next-line`), with the same current-buffer/window-buffer pair.

Status: FIXED.  `helm_css_scss_public_tui_workflows_match_gnu` passes; the
multi-source report (including `:current-source "component.css"` and the
`post-action` stage at `component.css` point 52) is identical between the two
editors.  Two GNU-side grid pins in the multi test were stale from an older
layout (they recorded a 42/7 window split; the package's own
`helm-css-scss-split-window-function` splits 50/50, which is what live GNU
produces today) and were refreshed in a separate commit -- they had never been
reached before, because the workflow failed earlier.
## 115. The first-change sentinel was recorded before the boundary check that gates the point entry -- FIXED

Tide's edit workflow formats a region, then `undo-only`.  GNU restores point to
where the command started; Neomacs left it at the change.

```elisp
(setq buffer-file-name FILE) (set-visited-file-modtime)
(insert "abcdef") (setq buffer-undo-list nil) (set-buffer-modified-p nil)
(goto-char (point-min)) (undo-boundary)
(save-excursion (goto-char (point-max)) (insert "X"))
buffer-undo-list
;; GNU               => ((7 . 8) 1 (t 27263 42178 675008 795000))
;; Neomacs before fix => ((7 . 8) (t 27263 42729 324050 609000))
```

The `1` is the point entry.  `primitive-undo`'s `((integerp next) (goto-char
next))` arm is the only thing that restores point across an undo, so without it
`undo` leaves point at the change site: the same probe driven through
`undo-only` ends at 31 under Neomacs and at 1 under GNU.

GNU records it in `record_point` (`src/undo.c:47-78`), which every recorder
calls before it conses anything:

```c
  at_boundary = ! CONSP (BVAR (current_buffer, undo_list))
                || NILP (XCAR (BVAR (current_buffer, undo_list)));
  if (MODIFF <= SAVE_MODIFF)
    record_first_change ();
  if (at_boundary && point_before_last_command_or_undo != beg && ...)
    bset_undo_list (..., Fcons (make_fixnum (point_before_last_command_or_undo), ...));
```

The order is the whole content of the function.  `at_boundary` is a fact about
the list as the command found it, so it is read BEFORE `record_first_change`
may cons `(t . TIME)` on top; GNU's own comment on that read says the check "is
currently dependent on being called before record_first_change".

Neomacs had split the three steps: `undo_prepare_change` recorded the
first-change sentinel, and then each recorder (`undo_list_record_insert`,
`undo_list_record_delete`, plus a separate `undo_list_record_point_for_change`
on the marker-adjustment path) re-derived `at_boundary` for itself -- after the
sentinel was already on the list.  On the one case the check exists for, the
first change to a CLEAN buffer, the re-derived answer is `false` and the point
entry is dropped.  Every editing path was affected equally: insert, delete,
replace, casify, `subst-char-in-region`, `transpose-regions`.

This is the opt-in-invariant class: a rule GNU applies once above the dispatch,
which we had asked each branch to remember.  The fix does not add a reminder --
it removes the ability to get it wrong.  `undo_prepare_change` is now GNU's
whole `record_point`, boundary read first, and the recorders no longer take the
saved point at all (`undo_list_record_insert(list, beg, len)`,
`undo_list_record_delete(list, beg, text, pt)`), so there is nothing left for a
call site to record late.  Putting the prologue above the marker adjustments
also keeps the point entry ahead of the `(MARKER . ADJUSTMENT)` entries (GNU bug
16818 ordering) without the dedicated helper that used to enforce it.

`replace_range` records the insertion first (`src/insdel.c:1638-1639`), so its
prologue runs for the insertion's beg -- the old range's END -- and that is
what the replacement path now passes.

Status: FIXED.

## 116. `x-popup-menu` rejected the frame GNU accepts in a POSITION's window slot -- FIXED

Tide's navigation workflow calls `imenu` interactively.  With
`last-nonmenu-event` nil (ledger 98) both editors take the mouse-menu path;
GNU shows nothing in batch and returns nil, while Neomacs raised
`(wrong-type-argument windowp #<frame F1>)` -- the only difference left in that
case's whole snapshot.

```elisp
(x-popup-menu (list (list 0 0) (selected-frame)) '("Title" ("Pane" ("item" . 1))))
;; GNU               => nil
;; Neomacs before fix => (wrong-type-argument (windowp #<frame F1 0x100000000>))
```

The route there is `imenu-choose-buffer-index` -> `imenu--mouse-menu` ->
`popup-menu` with a nil POSITION -> `popup-menu-normalize-position`
(`lisp/menu-bar.el:2786`), which builds `((X Y) FRAME)` out of
`(mouse-pixel-position)`.  A frame in that slot is not an accident, it is the
documented shape.

GNU's decode is `x_popup_menu_1` (`src/menu.c:1239-1269`):

```c
    if (FRAMEP (window))
      { f = XFRAME (window); xpos = 0; ypos = 0; }
    else if (WINDOWP (window))
      { CHECK_LIVE_WINDOW (window); ... }
    else
      /* ??? Not really clean; should be Qwindow_or_framep
         but I don't want to make one now.  */
      wrong_type_argument (Qwindowp, window);
```

so the slot is window-or-frame, and the `windowp` name is only GNU's admitted
shortcut for the third case.  Two more facts came out of the same function and
were read back from GNU: an internal (non-live) window fails with
`window-live-p`, not `windowp`; and when BOTH coordinates are nil GNU sets
`get_current_pos_p` (`src/menu.c:1182-1184`) and replaces WINDOW with the
selected frame, so it never inspects the designator at all -- even
`((nil nil) not-a-window)` returns nil.

Neomacs' batch `x-popup-menu` had no decode.  It read the second element only
to name it in an error, and signalled `windowp` for whatever it found whenever
the first element was non-nil: a frame, a live window and a genuine mistake all
took the same exit.  That is the invented-refusal class -- a check we wrote
where GNU has a decode.

`decode_popup_menu_position_window` now follows the C: nil-nil coordinates
short-circuit before the designator is read, a frame is accepted, a window must
be live, and only the remaining values are `windowp`.  All seven GNU answers
are pinned as a test.

Status: FIXED.

## 117. `x-popup-menu` demanded a string title from a keymap MENU -- FIXED

Behind ledger 110, in the same Tide `imenu` case, sat a second refusal of the
same shape.  With the POSITION accepted, the MENU was rejected:

```elisp
(let ((map (make-sparse-keymap "Index")))
  (x-popup-menu (list (list 0 0) (selected-frame)) map))
;; GNU               => nil
;; Neomacs before fix => (wrong-type-argument (stringp keymap))
```

`imenu--mouse-menu` builds a keymap and `popup-menu` passes
`(indirect-function map)`, so the keymap MENU is the ordinary case for that
command, not an exotic one.

GNU `x_popup_menu_1` (`src/menu.c:1294-1364`) decodes MENU in three branches:

```c
  keymap = get_keymap (menu, 0, 0);
  if (CONSP (keymap))                                   /* a keymap */
    { keymap_panes (&menu, 1); ... }
  else if (CONSP (menu) && KEYMAPP (XCAR (menu)))       /* a list of keymaps */
    { ... maps[i++] = keymap = get_keymap (XCAR (tem), 1, 0); ... }
  else                                                  /* old-fashioned menu */
    { title = Fcar (menu); CHECK_STRING (title); list_of_panes (Fcdr (menu)); }
```

`CHECK_STRING (title)` lives only in the third branch.  Neomacs' batch
`x-popup-menu` had only that branch, so it demanded a string title from every
MENU, and a keymap's car is the symbol `keymap`.

The decode is now GNU's: a keymap (directly, or through a symbol's function
cell, which is what `get_keymap` follows) is accepted; a list whose car is a
keymap is accepted after resolving EVERY element with GNU's erroring
`get_keymap`, so `(list map 42)` signals `(keymapp 42)` where we used to say
`stringp` about the whole list; and only what is left is the old-fashioned menu
that must have a string title.  Eight GNU answers are pinned as a test.

Status: FIXED.

## 118. A tree-sitter parse tree was thrown away because a command narrowed the buffer -- FIXED

Commenting a line in `astro-ts-mode` left `fontified nil` on intervals GNU
leaves the property absent from.  Running the workflow with
`treesit-parser-changed-regions` and
`treesit--font-lock-mark-ranges-to-fontify` traced shows where that comes
from -- the four `C-x C-;` presses in the corpus workflow:

```
                       GNU                     Neomacs before fix
comment "const x"      nil                     ((1 . 111))
comment "hi"           ((31 . 53))             ((1 . 120))
comment "let y"        nil                     ((1 . 129))
comment "color"        nil                     ((1 . 138))
```

`treesit--pre-redisplay` (`lisp/treesit.el:2310-2321`) hands whatever those
calls return to
`treesit--font-lock-mark-ranges-to-fontify` (`lisp/treesit.el:2257-2287`),
which does `(put-text-property BEG END 'fontified nil)`.  We answered "the
whole buffer" every time, so every interval it touched carried the property.

The whole-buffer answer is GNU's, but only in one situation.
`treesit_get_affected_ranges` (`src/treesit.c:1857-1880`) reads:

```c
  if (old_tree)
    { ranges = ts_tree_get_changed_ranges (old_tree, new_tree, &len); ... }
  else
    /* If the old_tree is NULL, meaning this is the first parse, the
       changed range is the whole buffer.  */
    lisp_ranges = Fcons (Fcons (Fpoint_min (), Fpoint_max ()), Qnil);
```

so it needs `old_tree == NULL`, and GNU never lets that happen twice: a
parser's tree is deleted only after it has been diffed
(`treesit_ensure_parsed`, `src/treesit.c:1901-1949`), nothing else sets
`tree` back to NULL, and even
`Ftreesit_parser_set_included_ranges` (`src/treesit.c:2770`) only raises
`need_reparse`.

Neomacs reached that branch on every one of those keypresses, and the
instrumented reason was not the one the reparse classification suggested:

```
finish_buffer_edit: -> FullReparseRequired parser=2
  freshness=Clean(rev{tick:54, accessible: 0..110})
  edit.old_revision=rev{tick:54, accessible: 30..43}
  input_edit_is_some=true
```

The `InputEdit` was computed fine.  What failed was the guard in front of it:
the tree had been parsed over the whole buffer, but the edit was recorded
while the buffer was narrowed to bytes 30..43 -- the commented line.
`comment-region-internal` wraps its edits in `comment-with-narrowing`
(`lisp/newcomment.el:1094-1112`, used at `:1169`), a `save-restriction` plus
`narrow-to-region` around exactly the region being commented.  The four traced
windows (4..16, 30..43, 62..72, 100..118) are the four commented lines.

Neomacs treated "the restriction changed since the tree was parsed" as a
reason to distrust the tree, because its `ParserInputRevision` folded the
accessible bounds into the content version and its `PendingBufferEdit`
measured offsets from the buffer's *current* accessible start.  Under a
narrowing those offsets are in a different coordinate frame than the tree, so
refusing them was right -- the missing piece was the frame itself.

GNU keeps that frame per parser as `visible_beg`/`visible_end`
(`src/treesit.h:98-118`).  `treesit_record_change_1`
(`src/treesit.c:1420-1457`) clips each change into *that* window and converts
to window-relative offsets, so a temporary narrowing never enters the
arithmetic; and `treesit_sync_visible_region` (`src/treesit.c:1626-1740`)
reconciles the window with the buffer's restriction with four `ts_tree_edit`
calls before every parse, raising `need_reparse` if it moved.  A narrowing
moves the tree in GNU; it never invalidates it.  Neomacs had no counterpart
to that reconciliation at all, and discarding the tree stood in for it.

The window is now part of the tree (`ParsedTree { tree, visible }`), so a tree
whose frame is unknown cannot be built; changes are recorded in each parser's
own window; `sync_visible_region` runs before every parse and before
`treesit-parser-set-included-ranges`, which no longer drops the tree; and the
reparse always feeds the previous tree back, leaving the whole-buffer affected
range to GNU's single case.  The workflow's buffer, its text properties and
its notifier ranges are now identical to GNU's.

Status: FIXED.

## 119. The default `minibuffer-setup-hook' order inverts between GNU and Neomacs

Neomacs lists `minibuffer--nonselected-setup' before
`rfn-eshadow-setup-minibuffer' where GNU lists it third.  Surfaced by the
window-numbering parity suite, whose `window-numbering-update' is prepended
in both editors: the whole remaining tail of the hook then differs even
though every function on it is identical.

```sh
emacs -Q --batch --eval '(prin1 minibuffer-setup-hook)'
./target/release/neomacs -Q --batch --eval '(prin1 minibuffer-setup-hook)'
```

GNU:
`(rfn-eshadow-setup-minibuffer minibuffer--regexp-setup
minibuffer--nonselected-setup minibuffer-setup-on-screen-keyboard
minibuffer-error-initialize minibuffer-history-isearch-setup
minibuffer-history-initialize)`

Neomacs:
`(minibuffer--nonselected-setup rfn-eshadow-setup-minibuffer
minibuffer--regexp-setup minibuffer-setup-on-screen-keyboard
minibuffer-error-initialize minibuffer-history-isearch-setup
minibuffer-history-initialize)`

Both entries come from `:global` minor modes initialized with
`custom-initialize-after-file-load`: `file-name-shadow-mode`
(lisp/rfn-eshadow.el, `add-hook` at line 226) and
`minibuffer-nonselected-mode` (lisp/minibuffer.el, `add-hook` at line 5749).
Both editors load the very same files in the very same order --
`lisp/loadup.el` line 254 loads minibuffer, line 277 rfn-eshadow -- so the
inverted order comes from the `custom-add-load' /
`after-load-functions` sequencing that turns these modes on at file-load
time, not from the elisp sources.  Neomacs loads GNU's own `.el' files, so
the difference has to be in the custom/after-load machinery or in how the
dump-time initialization is replayed at startup.

Root cause found and fixed in entry 130.  The guess in the paragraph above is
refuted: the custom/after-load machinery is byte-for-byte correct in Neomacs.
The engine seeded `minibuffer-setup-hook' (and `minibuffer-exit-hook', which
rfn-eshadow.el never touches) in Rust with a frozen function list, so every
`add-hook' that builds those lists during loadup was a no-op.

Status: FIXED (see 130).

## 120. `buffer-enable-undo` erased the history it was asked to keep, destroying a base buffer's undo through an indirect buffer -- FIXED

Enabling undo in an indirect buffer threw away the BASE buffer's entire undo
history.  Ten lines, deterministic, no package involved:

```elisp
(let* ((base (get-buffer-create "b2")) s1 s2 s3 s4)
  (set-buffer base) (buffer-enable-undo) (insert "hello")
  (setq s1 (prin1-to-string buffer-undo-list))
  (let ((ind (make-indirect-buffer base "i2")))
    (setq s2 (prin1-to-string buffer-undo-list))
    (set-buffer ind) (buffer-enable-undo)
    (set-buffer base) (setq s3 (prin1-to-string buffer-undo-list))
    (set-buffer ind) (insert "Y")
    (set-buffer base) (setq s4 (prin1-to-string buffer-undo-list)))
  (list s1 s2 s3 s4))
;; GNU                => ("((1 . 6) (t . 0))" "((1 . 6) (t . 0))" "((1 . 6) (t . 0))" "((1 . 7) (t . 0))")
;; Neomacs before fix => ("((1 . 6) (t . 0))" "((1 . 6) (t . 0))" "nil"               "((6 . 7))")
```

Each step is snapshotted with `prin1-to-string` on purpose.  `record_insert`
coalesces an adjacent insertion by mutating the head cons in place
(`XSETCDR (elt, make_fixnum (beg + length))`, src/undo.c:109), so collecting
the live lists and printing them at the end reports the FINAL state for every
earlier step -- the first naive run of this probe printed `(1 . 7)` for all
four rows under GNU.

`buffer-enable-undo` is not Lisp; it is `Fbuffer_enable_undo`
(src/buffer.c:1829-1850), and its body is one conditional:

```c
  if (EQ (BVAR (XBUFFER (real_buffer), undo_list), Qt))
    bset_undo_list (XBUFFER (real_buffer), Qnil);
```

Enabling undo is a TRANSITION out of the off state, not an assignment.  Ours
called `configure_buffer_undo_list(id, nil)` unconditionally, so it also
cleared a live history -- the plain non-indirect case
`(setq buffer-undo-list nil) (insert "abc") (buffer-enable-undo)` lost
`((1 . 4) (t . 0))` too.  The indirect case is only where it does the most
damage, because base and indirect share one list.

That sharing is not a Neomacs liberty; it is what GNU does, and a direct probe
settled it rather than the header comment.  `make_indirect_buffer` copies the
base's value into the indirect's slot (src/buffer.c:894, "An indirect buffer
shares undo list of its base (Bug#18180)"), and `set_buffer_internal_2`
re-syncs the pair on EVERY buffer switch -- pushing the old buffer's list down
into the base on the way out and pulling the base's list up on the way in
(src/buffer.c:2352-2367).  Observably, under GNU 31.0.90: a boundary made in
the indirect buffer appears in the base; an insertion in the base appears in
the indirect buffer; `(setq buffer-undo-list t)` in the indirect buffer turns
undo off for the base; and the two slots read `eq`.  Our single
`SharedUndoState` already models exactly that, so it was never the defect.

The one thing GNU's per-buffer slots buy is a stale read: with two indirect
buffers `i1` and `i2` over one base, changing the list through `i1` leaves
`(buffer-local-value 'buffer-undo-list i2)` reporting the OLD list until
something makes `i2` current, at which point it catches up.  That is the lag of
a lazily-synced cache, not a behaviour anything can rely on -- so per-buffer
undo slots were not adopted; they would buy one stale-read artifact and cost a
resync on every `set-buffer`.

The fix names the domain instead of re-deriving it.  `UndoRecording::{Disabled,
Enabled}` (neovm-core/src/buffer/undo.rs) classifies a `buffer-undo-list`
value once -- GNU hand-writes `EQ (..., Qt)` at each decision point
(src/undo.c:91, src/buffer.c:1846, src/buffer.c:1869) -- and
`Buffers::enable_buffer_undo` is GNU's conditional as an exhaustive match, so
the `Enabled` arm cannot silently fall through to a reset.  `buffer-disable-undo`
is unchanged: it is a plain assignment in GNU too (lisp/simple.el:3591-3596).

Residue, unchanged from ledger 105: `record_first_change` redirects an indirect
buffer to its BASE buffer's visited-file modtime (src/undo.c:211-218), and a
`Buffer` here still cannot reach the buffer manager, so an indirect buffer
records 0.  Both probe columns above read `(t . 0)` because the base buffer
visits no file.

Status: FIXED.

## 121. A point saved in an indirect buffer was spent on an edit in its base -- FIXED

Found while confirming ledger 119 against the real binary: with 119 fixed, the
base/indirect probe agreed with GNU on every step but one, where we consed a
point entry GNU does not record.

```elisp
(let* ((base (get-buffer-create "b9")))
  (set-buffer base) (buffer-enable-undo) (insert "hello") (goto-char 6)
  (let ((ind (make-indirect-buffer base "i9")))
    (set-buffer ind) (goto-char (point-max)) (insert "Y")
    (undo-boundary)
    (set-buffer base)
    (insert "Z")
    buffer-undo-list))
;; GNU                => ((6 . 7) nil (1 . 7) (t . 0))
;; Neomacs before fix => ((6 . 7) 7 nil (1 . 7) (t . 0))
```

The stray `7` is a point entry, and `primitive-undo`'s `((integerp next)
(goto-char next))` arm acts on it, so undoing that insertion moved point to a
position the user's command never occupied.

`record_point` has THREE guards, and the third is about the BUFFER
(`src/undo.c:73-75`):

```c
  if (at_boundary
      && point_before_last_command_or_undo != beg
      && buffer_before_last_command_or_undo == current_buffer )
```

We had the first two.  The third exists because GNU's saved point is a pair of
GLOBALS, `point_before_last_command_or_undo` and
`buffer_before_last_command_or_undo` (`src/keyboard.c:232-233`), written
together at both of their assignment sites -- the command loop
(`src/keyboard.c:1536-1537`) and `Fundo_boundary` (`src/undo.c:278-279`) -- so
a point saved in one buffer is meaningless in another.  GNU's own comment at
the read says as much: "we must not do this if the buffer has changed since the
last command, since the value of point that we have will be for that buffer,
not this."

A base buffer and its indirect buffer are distinct `struct buffer`s in GNU, so
the check separates them; here they share one `SharedUndoState`, which is what
let the boundary's saved point (7, saved while the indirect buffer was current)
be read back by the base buffer's insertion at 6.  This is the one place where
our sharing is genuinely wider than GNU's: GNU shares the undo LIST between a
base and its indirect buffers, but never this point.

The fix pairs the two facts GNU always writes together.
`SharedUndoState::point_before_command_or_undo` now holds a
`PointBeforeCommand { buffer, point }` (neovm-core/src/buffer/shared.rs)
instead of a bare `CharPos0`, so saving the point without the buffer is not
expressible, and `undo_prepare_change` spells GNU's third guard as
`saved.buffer == self.id`.  The type change is what found every call site: the
compiler rejected all four fixtures that had been saving a point with no
buffer.

Status: FIXED.

## 122. The saved point-before-command was per buffer, so every `M-x` command recorded a point entry GNU had already superseded -- FIXED

scala-mode's import workflow runs `M-x scala-organise`, undoes it with `C-_`,
then runs the command twice more.  The buffer text agreed with GNU at every
step; point did not.  GNU leaves point at 271 (line 13, `object Main {` -- the
end of the import block the command deleted); Neomacs left it at 1, and the
next `scala-organise`, whose `save-excursion` marker starts wherever the undo
left point, then reported 1 where GNU reports 15.

The whole difference is one stray entry in `buffer-undo-list`.  Ten lines, no
package involved:

```elisp
(defun probe-cmd ()
  (interactive)
  (save-excursion (goto-char 6) (delete-region 3 6)))
(let ((buf (get-buffer-create "probe")))
  (set-window-buffer (selected-window) buf)   ; M-x runs the command in the
  (set-buffer buf)                            ; SELECTED window's buffer
  (buffer-enable-undo) (insert "abcdefghij")
  (set-buffer-modified-p nil) (setq buffer-undo-list nil) (goto-char 1)
  (execute-kbd-macro (kbd "M-x probe-cmd RET"))
  (let ((recorded (prin1-to-string buffer-undo-list)))
    (execute-kbd-macro (kbd "C-_"))
    (list recorded (point))))
;; GNU                => ("((\"cde\" . -3) (t . 0))" 6)
;; Neomacs before fix => ("((\"cde\" . -3) 1 (t . 0))" 1)
```

The `1` is a point entry, and `primitive-undo`'s `((pred integerp) (goto-char
next))` arm (`lisp/simple.el:3666-3668`) is processed last in the group, so it
overrides the position the deletion record had already restored.  The deletion
record is `("cde" . -3)` in both editors: `record_delete` stores a NEGATIVE
position when point sat at the end of the deleted text (`if (PT == beg + SCHARS
(string)) XSETINT (sbeg, -beg);`, `src/undo.c:174-181`), and `primitive-undo`'s
`(< pos 0)` branch (`lisp/simple.el:3745-3751`) reinserts and leaves point past
the reinsertion.  That is where GNU's 6 -- and
scala-mode's 271 -- comes from.  Neomacs computed the same `-3`, then threw the
result away one entry later.

Bind the same command to a key instead and BOTH editors record the `1` and both
end at 1.  The minibuffer read is the whole difference.

### Why the minibuffer read matters

GNU's saved point is a pair of GLOBALS, `point_before_last_command_or_undo` and
`buffer_before_last_command_or_undo` (`src/keyboard.c:232-233`), described in
`src/keyboard.h:257-266` as

> The location of point immediately before **the last command** was executed,
> or the last time the undo-boundary command added a boundary.

Singular: one saved point for the editor.  Both assignment sites overwrite it
unconditionally with whatever buffer is current -- the command loop
(`src/keyboard.c:1536-1537`) and `Fundo_boundary` (`src/undo.c:278-279`) -- and
`record_point` reads it under three guards (`src/undo.c:73-78`):

```c
  if (at_boundary
      && point_before_last_command_or_undo != beg
      && buffer_before_last_command_or_undo == current_buffer )
```

Every `M-x` is a sequence of command-loop iterations in the MINIBUFFER followed
by the chosen command, which `execute-extended-command` calls without an
iteration of its own.  By the time `scala-organise` deletes, the globals hold
the minibuffer's point and the minibuffer's buffer, so the third guard is false
and GNU records nothing.  The guard's comment says exactly why: "we must not do
this if the buffer has changed since the last command, since the value of point
that we have will be for that buffer, not this."

Ledger 121 added that third guard, but it could not fire: Neomacs kept the pair
in `SharedUndoState`, i.e. per buffer, where a buffer's own saved point always
names that buffer.  `saved.buffer == self.id` was a tautology for every
ordinary buffer, and only ever did work for a base/indirect pair, which is the
one case that shares one `SharedUndoState`.  Per-buffer storage cannot express
"superseded" at all -- nothing in it is global enough to be overwritten by a
command somewhere else.

### The fix

`SavedPointBeforeCommand` (neovm-core/src/buffer/shared.rs) is ONE
`Rc<Cell<Option<PointBeforeCommand>>>` that a `BufferManager` mints once and
every buffer it owns clones.  Saving a point therefore discards the previous
one, exactly as assigning a C global does; there is nothing to keep in sync,
and `clone`/`buffer-swap-text` cannot fork the cell.  `PointBeforeCommand` is
now private to that module, so the pair can only be reached through the two
accessors:

* `save(buffer, point)` -- GNU's paired assignment, the only writer.
* `point_saved_in(buffer) -> Option<CharPos0>` -- GNU's paired read.  It takes
  the buffer that is about to record the entry, so the third guard is not
  something a call site can forget: there is no accessor that hands out a point
  without a buffer to check it against.

`Buffer::new` takes the cell as a required argument rather than minting one,
because a buffer with a private cell is precisely the bug.  Deleting the old
`SharedUndoState` accessors is what pointed the compiler at every site.

### A second finding in the same function

Making the point global exposed a divergence that per-buffer storage had hidden.
`Fundo_boundary` (`src/undo.c:251-282`) is one early return over its whole
body -- a buffer whose `buffer-undo-list` is `t` gets neither the boundary nor
the saved point (`src/undo.c:258-259`, assignment at `:278-279`) -- and Neomacs
saved the point unconditionally:

```elisp
(let ((a (get-buffer-create "A")) (b (get-buffer-create "B")))
  (with-current-buffer a (setq buffer-undo-list t) (insert "hello") (goto-char 4))
  (with-current-buffer b
    (setq buffer-undo-list nil) (insert "world")
    (set-buffer-modified-p nil) (goto-char 1) (undo-boundary))
  (with-current-buffer a (undo-boundary))
  (with-current-buffer b (goto-char 3) (delete-region 3 5) buffer-undo-list))
;; GNU                => (("rl" . 3) 1 (t . 0) nil (1 . 6) (t . 0))
;; Neomacs before fix => (("rl" . 3)   (t . 0) nil (1 . 6) (t . 0))
```

A buffer that records nothing was spending a point saved for a buffer that
does.  The automatic path cannot reach it -- `undo-auto--ensure-boundary` gates
on `(car-safe buffer-undo-list)` (`lisp/simple.el:4077-4079`), which is nil for
`t` -- but the thirty-six unconditional `(undo-boundary)` calls in `lisp/`
(electric.el, replace.el, mouse.el, abbrev.el, ...) can, and every one of them
runs in whatever buffer is current.  The guard is now an exhaustive match on
`UndoRecording` (the classification added for ledger 120) rather than a bare
early return, so the disabled arm cannot fall through into the recording body.

Residual, deliberately not fixed here: `Fundo_boundary` also does `Fset
(Qundo_auto__last_boundary_cause, Qexplicit)` (`src/undo.c:277`) and Neomacs
leaves that variable nil -- `(progn (insert "x") (undo-boundary)
undo-auto--last-boundary-cause)` reads `explicit` under GNU and `nil` here.
Nothing observed depends on it (`undo-auto--last-boundary-amalgamating-number`
takes `car-safe`, which is nil either way), and both Neomacs callers of the
boundary sit below the obarray, so it wants its own change.

Status: FIXED.

## 123. `undo-limit` and every other variable that bounds the undo list did nothing, and truncation ran at the wrong time -- FIXED

The manual tells users that `undo-limit` is how you decide how much undo
history Emacs keeps.  Setting it changed nothing: undo lists were truncated
against two literals compiled into Rust, from `undo-boundary`, which is not
where or when GNU truncates at all.

```elisp
(let ((b (get-buffer-create "u123")))
  (set-buffer b)
  (buffer-enable-undo)
  (setq buffer-undo-list nil)
  (setq-local undo-limit 1)
  (setq-local undo-strong-limit 1)
  (dotimes (_ 10) (insert "hello") (undo-boundary))
  (let ((before (length buffer-undo-list)))
    (garbage-collect)
    (list before (length buffer-undo-list) buffer-undo-list)))
;; GNU                => (21 2 (nil (46 . 51)))
;; Neomacs before fix => (21 21 (nil (46 . 51) nil (41 . 46) nil (36 . 41) nil (31 . 36) nil (26 . 31) nil (21 . 26) nil (16 . 21) nil (11 . 16) nil (6 . 11) nil (1 . 6) (t . 0)))
```

The `before` column agreeing at 21 is the important half: it says GNU does NOT
truncate when the boundary is made, so the ten `undo-boundary` calls are not
the divergence.  The collection is.

`undo-outer-limit` and `undo-outer-limit-function` existed only as names in a
defvar table; nothing ever read them:

```elisp
(let ((b (get-buffer-create "u123b")) (calls nil))
  (set-buffer b)
  (buffer-enable-undo)
  (setq buffer-undo-list nil)
  (setq-local undo-outer-limit 1)
  (setq undo-outer-limit-function (lambda (size) (setq calls size) t))
  (insert "hello")
  (garbage-collect)
  (list calls buffer-undo-list))
;; GNU                => (64 ((1 . 6) (t . 0)))
;; Neomacs before fix => (nil ((1 . 6) (t . 0)))
```

### Where GNU truncates, and why there

`Fundo_boundary` (`src/undo.c:251-282`) conses a `nil` on the front, sets
`undo-auto--last-boundary-cause`, saves point, and returns.  It has no size
logic.  Truncation is a garbage-collector job:

```c
  /* Don't keep undo information around forever.
     Do this early on, so it is no problem if the user quits.  */
  FOR_EACH_LIVE_BUFFER (tail, buffer)
    compact_buffer (XBUFFER (buffer));            /* src/alloc.c:5796-5800 */
```

`compact_buffer` (`src/buffer.c:1854-1885`) filters before it truncates: dead
buffers, indirect buffers, and buffers whose `BUF_COMPACT` still equals their
`BUF_MODIFF` are skipped, and a `t` undo list is never passed on -- "Calling
truncate_undo_list on Qt tends to return NULL, which effectively turns undo
back on".  Then `truncate_undo_list` (`src/undo.c:289-419`) does the walk.

Doing it at GC rather than at each boundary is not an implementation detail
users cannot see.  Three consequences show up in a probe:

- The undo list a program reads right after editing is the untruncated one in
  both editors.  Truncating at boundary time made Neomacs' list shorter than
  GNU's at moments GNU has not yet collected -- with realistic limits it also
  made it *longer* forever, since nothing else ever truncated.
- `BUF_COMPACT` means a second collection with no intervening modification
  leaves the list alone.  Measured under GNU: after a collection truncates a
  buffer to 2 entries, `(setq buffer-undo-list ...)` with 6 entries and another
  `(garbage-collect)` still reports 6.
- `undo-outer-limit-function` runs inside the collector, with GC inhibited
  around it (`src/undo.c:296-298`).

### Which bindings GNU reads

The buffer's own.  `truncate_undo_list` makes the buffer current first, and
says why (`src/undo.c:296-306`):

```c
  /* Make the buffer current to get its local values of variables such
     as undo_limit.  Also so that Vundo_outer_limit_function can
     tell which buffer to operate on.  */
  record_unwind_current_buffer ();
  set_buffer_internal (b);
```

That matters because `compact_buffer` is called for every live buffer in turn:
a `setq-local undo-limit 1` in one buffer must not shorten another's history.
Measured under GNU: buffer A with a local limit of 1 truncates to 2 entries in
the same collection that leaves buffer B, on the 160000 default, at 21.  It
also means a `let` binding counts -- which is the whole point of
`combine-change-calls`, which wraps its body in `(undo-limit
most-positive-fixnum)` (`lisp/subr.el:4308-4310`).

### What `undo-outer-limit` does that `undo-limit` does not

`undo-limit` bounds the accumulated HISTORY and is applied by discarding old
groups.  `undo-outer-limit` bounds ONE command's record -- the entries before
the first boundary -- and is a last-ditch measure against a single command
consing Emacs to death, which no amount of discarding older groups can help
with.  It is checked before any of the `undo-limit` walking, against the size
accumulated up to the first boundary (`src/undo.c:349-369`), and the C code
does not act on it itself: it calls `undo-outer-limit-function` and stops only
if that returns non-nil.

What the user observes comes from `lisp/simple.el:4252-4295`, which sets that
function to `undo-outer-limit-truncate`: a `(undo discard-info)` warning
naming the buffer and the byte count, and `buffer-undo-list` emptied.  With
`undo-ask-before-discard` non-nil it asks in the echo area instead.  Measured
under GNU 31.0.90 with `undo-outer-limit` at 1: "Buffer 'C' undo info was 64
bytes long. / The undo info was discarded because it exceeded
`undo-outer-limit'." and the list becomes nil.

Two details a grep does not show.  `--batch` sets `undo-outer-limit` to nil
before anything runs (`src/emacs.c:1700-1707`), which is why no batch oracle
run could ever have caught this variable being ignored -- the batch default is
nil in both editors and always was.  The 24000000 default only exists in a real
session, and both editors now report the same thing there; measured by running
each under a pty with `-nw -Q` and writing `undo-outer-limit` and
`undo-outer-limit-function` to a file: `24000000 undo-outer-limit-truncate`.
And GNU's guard is an integer test, not a number test (`src/undo.c:352-355`):
a float is not a limit, a bignum too large for `intmax_t` fires only when it is
negative.  All four cases were measured rather than reasoned about.

### The fix

Truncation moved to where GNU does it: `compact_buffers_for_gc`
(neovm-core/src/emacs_core/undo.rs) runs at the top of
`gc_collect_from_current_roots_body`, with GNU's skips, and `add_undo_boundary`
no longer truncates.  `SharedUndoState` gained `compacted_modified_tick`, GNU's
`BUF_COMPACT`, which lives in the shared per-text state exactly as GNU's lives
in `struct buffer_text`.

The limits are now a value that can only be obtained by reading the variables.
`UndoLimits` (neovm-core/src/buffer/undo.rs) has private fields and one
constructor, `UndoLimits::read`, which takes a `UndoLimitBindings` -- a trait
whose four methods return the *Lisp values* of `undo-limit`,
`undo-strong-limit`, `undo-outer-limit` and `undo-outer-limit-function`, and
whose only production implementation reads them off the `Context` after the
buffer has been made current.  There is no constructor taking numbers, so
`truncate_undo_list(ul, 160_000, 240_000)` is no longer a thing anyone can
write.

Two enums carry the states GNU distinguishes and Neomacs previously flattened.
`OuterUndoLimit::{NoLimit, Bytes, AlwaysExceeded}` is GNU's three-way integer
guard.  `GnuIntVariable::{Int, NotAnIntSlotValue}` names the state that has no
GNU counterpart at all: GNU's `store_symval_forwarding` refuses to put a
non-integer in a `DEFVAR_INT` slot (`src/data.c:1475-1483`, `(setq undo-limit
"x")` signals `(wrong-type-argument integerp "x")`), so a string in
`undo-limit` is unreachable there.  Neomacs still backs these as plain obarray
values, and `UndoLimits::read` returns `None` for that state rather than
substituting a number nobody configured.

The walk itself is now GNU's, not a running-total cut: it scans past the first
group before making any decision, so the most recent record survives however
small the limits are, and every cut lands on a group edge, so an undo list is
never left holding half of one command's changes.  The byte accounting matches
too -- 16 per link, 16 more per cons record, `sizeof (struct Lisp_String) - 1
+ SCHARS` for a saved deletion string -- calibrated by having GNU report the
size through `undo-outer-limit-function`: a lone 100-character deletion record
is 163 bytes whether those characters are one byte or two.

Left unfixed, and separate: `forward::LispIntFwd` is declared but not wired
into `find_symbol_value`, so `(setq undo-limit "x")` and `(setq
gc-cons-threshold "x")` are both accepted here where GNU signals.  That is the
`DEFVAR_INT` forwarding gap, not an undo bug; `GnuIntVariable` marks exactly
where undo truncation meets it.

Status: FIXED.
## 124. `get-pos-property` never checked the position it was given, and `previous-property-change` reported an out-of-range one in the wrong shape -- FIXED

Found while checking an old side note that claimed our `validate_interval_range`
compares values where GNU compares pointers.  **That premise is refuted** -- see
the end of this entry -- but the probe written to test it turned up three real
divergences instead.

```elisp
(with-temp-buffer
  (insert "hello")
  (list (condition-case e (get-pos-property 500 'face) (error e))
        (condition-case e (get-pos-property 500 'syntax-table) (error e))
        (condition-case e (previous-property-change 500) (error e))))
;; GNU                => ((args-out-of-range 499 499)
;;                        (args-out-of-range 500 500)
;;                        (args-out-of-range 500 500))
;; Neomacs before fix => (nil
;;                        nil
;;                        (args-out-of-range 500))
```

### GNU has two out-of-range shapes here, and they are not interchangeable

`validate_interval_range` (src/textprop.c:128-186) is the one validator the
whole text-property family goes through.  Its bounds failure is
`args_out_of_range (begin0, end0)` (:158), and a POINT call passes one pointer
for both parameters -- `validate_interval_range (object, &position, &position,
soft)` -- so the payload carries the position **twice**.

`get_char_property_and_overlay` (src/textprop.c:642-644) does not use that
function.  It has its own bounds check that signals with `xsignal1`, so its
payload carries the position **once**.

Which shape a builtin produces is therefore not a style choice; it is which GNU
function that builtin goes through.  Our string branch of
`previous-property-change` already used the interval shape while its buffer
branch used the char-property shape, and nothing could notice, because both
validators took the same arguments and returned the same type.
`previous-char-property-change` inherited the wrong payload for free, since it
delegates exactly as GNU does (src/textprop.c:767).

### `get-pos-property`'s range check is emergent, which is why we had none

`Fget_pos_property` (src/editfns.c:275-349) contains no bounds check at all.
Its error comes from `text_property_stickiness` (src/textprop.c:1901), which
reads text properties through `Fget_text_property` twice -- at POS-1 (:1919)
and at POS (:1931) -- and GNU's own comment above the second read records the
consequence: "This signals an arg-out-of-range error if pos is outside the
buffer's accessible range."

So *which* position appears in the signal follows GNU's branch structure, and
that is measurable:

| call | GNU | why |
|---|---|---|
| `(get-pos-property 500 'face)` | `(args-out-of-range 499 499)` | POS-1 is read first |
| `(get-pos-property 500 'syntax-table)` | `(args-out-of-range 500 500)` | default-nonsticky PROPs skip the POS-1 read (:1914) |
| `(get-pos-property 0 'face)` | `(args-out-of-range 0 0)` | `POS <= BEGV` skips it too (:1912) |
| `(get-pos-property 7 'face)` in a 5-char buffer | `(args-out-of-range 7 7)` | POS-1 == ZV is in range, so POS is what fails |
| narrowed to (3,6), `(get-pos-property 9 'face)` | `(args-out-of-range 8 8)` | bounds are BEGV/ZV, not BEG/Z (:156-157) |

We reimplemented the read that GNU delegates, through a **clamping** accessor,
so every one of those answered `nil` instead.  A caller could not tell a
position outside the buffer from a position with no property there.

The fix makes the two reads go through validation, and names what GNU's C
returns as an integer: `Stickiness::{FromFollowing, FromPreceding, Neither}`
(GNU's 1 / -1 / 0), returned as `Result<Stickiness, Flow>` so that "this
function can signal" is in the type rather than in a comment, and so the call
site is a total match instead of `1 => ..., -1 if ... => ..., _ => nil`.

### The premise this started from, refuted

The side note said GNU's `validate_interval_range` distinguishes a point call
from a range call by comparing POINTERS (`EQ (*begin, *end) && begin != end`,
src/textprop.c:141) where we compare values, and that we might therefore
mis-handle an empty range.  We do not: the distinction is already carried by
having two separate functions, which is the same information in the type system
instead of in pointer identity.  Thirty probe cases spanning buffers, strings,
empty objects, narrowing, reversed ranges and two markers at one position agree
with GNU byte for byte, including the cases the pointer test exists for --
`(put-text-property 500 500 'a 'b)` returns nil quietly in both, because the
empty-range test runs BEFORE the bounds check.

Status: FIXED.

## 125. One text-property default was written in four places, so correcting it corrected nothing -- FIXED

The same probe run showed a fourth difference, in a variable rather than a
function:

```elisp
text-property-default-nonsticky
;; GNU                => ((composition . t) (syntax-table . t) (display . t))
;; Neomacs before fix => ((syntax-table . t) (display . t))
```

GNU assembles this default in **two** C files, and neither one alone is the
answer: `syms_of_textprop` seeds the alist with syntax-table and display
(src/textprop.c:2426-2429), then `syms_of_composite` conses `composition` onto
the front of whatever is already there (src/composite.c:2212-2213).  We ported
the first file and not the second, so `composition` was sticky: text inserted
next to a composed sequence inherited a `composition` property that GNU would
never give it.

What made this worth its own entry is the shape of the miss.  Our value was
spelled out at **four** separate installation sites -- `textprop.rs` twice
(control-variable registration and bootstrap registration), `xdisp.rs`, and
`load.rs`.  The first correction, made in `load.rs`, changed nothing observable,
because a later site overwrote it; the test still failed with the old value
after a verified cache miss and a freshly written pdump, which is what exposed
the duplication.  One of the four even carried a comment asserting the GNU
default -- accurately quoting src/textprop.c, and wrong about the effective
value, because it had never been checked against src/composite.c.

The value now exists once, in `default_text_property_nonsticky_alist`, cited to
both C files, and the four sites call it.

Status: FIXED.

## 126. `color-distance` rejected the "unspecified-fg"/"unspecified-bg" sentinel color names

RENUMBERED at integration: this entry was published as 120 in 2a6c6b093, a
number that entry 120 above already held, so the ledger carried two entries
numbered 120 and both the ascending and the no-duplicates invariants were
broken.  Renumbering the later-published of the two is the same resolution used
when 119 collided earlier.  Nothing else about the entry changed; if you
followed a reference to "ledger 120 (color-distance)", this is it.

GNU accepts the internal sentinel strings that `face-background' returns
for unspecified colors on a TTY frame; Neomacs signals `error'
("Invalid color").  Surfaced by the smart-mode-line suite:
`sml/-automatically-decide-theme' measures the distance of the frame
background from white and black to pick the dark or light mode-line
theme, and on a batch frame the default face's background IS the
"unspecified-bg" string -- GNU resolves it (to the tty default,
black/white) and picks 'dark, Neomacs errors inside `ignore-errors' and
picks 'light.

```sh
emacs -Q --batch --eval '(prin1 (color-distance "white" "unspecified-bg"))'
./target/release/neomacs -Q --batch --eval '(prin1 (color-distance "white" "unspecified-bg"))'
```

GNU: `589805'; Neomacs: `error ("Invalid color" "unspecified-bg")'.
Same for "unspecified-fg" (GNU 589805).  A GENUINELY invalid name
("not-a-color") signals in BOTH editors, so GNU's acceptance is the
sentinel mapping, not lenient parsing.

The mechanism is `tty_lookup_color' (src/xfaces.c:1155-1170): when the
color-name lookup on a TTY frame yields `FACE_TTY_DEFAULT_COLOR', GNU
maps the strings "unspecified-fg"/"unspecified-bg" to
`FACE_TTY_DEFAULT_FG_COLOR'/`FACE_TTY_DEFAULT_BG_COLOR' before
returning success, and `Fcolor_distance' (src/xfaces.c:4792) then
computes over the tty default fg/bg.  Neomacs' TTY color lookup lacks
that sentinel branch, so the name stays unresolved and
`Fcolor_distance' signals.

### Fixed 2026-08-17, with two corrections to the above

**The citation.** The sentinel branch is not in `tty_lookup_color'; it is in
its caller `tty_defined_color' (src/xfaces.c:1143-1174), at :1160-1167.  The
distinction matters, because the branch is guarded on what
`tty_lookup_color' LEFT BEHIND -- it fires only when the palette lookup
returned without resolving a pixel, so a palette entry actually named
`unspecified-bg' would still win.  The fix preserves that order.

**What the sentinels are worth.** The entry says GNU "computes over the tty
default fg/bg", which implies the terminal's real colours.  It does not.
`tty_defined_color' seeds its `Emacs_Color' with RGB (0, 0, 0) at :1150-1153
and the sentinel branch assigns only `pixel'; the RGB triple is never
touched.  Assigning the pixel is what makes the lookup succeed (:1170-1171).
So both sentinels carry the value BLACK, which is measurable and was measured:

```elisp
(list (color-distance "black" "unspecified-bg")
      (color-distance "black" "unspecified-fg")
      (color-distance "unspecified-fg" "unspecified-bg")
      (color-distance "white" "unspecified-bg"))
;; GNU                => (0 0 0 589805)
;; Neomacs before fix => error ("Invalid color" "unspecified-bg")
```

The two names are opposite ends of the frame and their distance from each
other is nevertheless 0.  A fix that resolved them to the frame's actual
foreground and background would match the one number the original entry
recorded (589805 against white, on a dark frame) and be wrong about the other
three.

Worth knowing where this does NOT apply: `color-values' and `color-defined-p'
answer nil for both sentinels in GNU as well, because they are Lisp and
consult `tty-color-alist'.  The sentinel branch exists only in the C
`defined_color_hook', so `color-distance' is the function that sees it, and
the fix is scoped to that path rather than added to the colour tables.

Shape: `TtyDefaultColor::{Foreground, Background}`
(neovm-core/src/emacs_core/xfaces/mod.rs) names the two sentinels and owns
the zero RGB, so "resolved to a terminal default" is a state the resolver
states rather than a magic string comparison inline, and the fact that both
carry black lives with the enum that explains why.

Verified: the 14-case probe in tmp/coord-colordist-probe.el is byte-identical
between GNU 31.0.90 and Neomacs, and
`parity_tests::smart_mode_line::smart_mode_line_package_batch` passes.

## 127. A snapshot pinned `string-match-p`'s INDEX into text that quotes the sandbox path, so company-go's invocation contract counts the harness's path length -- NOT A DIVERGENCE (harness defect), FIXED

`cargo nextest run -p neomacs-melpa-tests --release -E 'test(/company_go/)'`
fails on one field of one case, and it fails **in both editors**, which the
eleven-suites table above already names as the signature of a stale
expectation (ace_link, fixed 5dd14bf22, was the same shape):

```
snapshot mismatches: the_invocation_contract_through_a_fake_gocode (GNU Emacs),
                     the_invocation_contract_through_a_fake_gocode (Neomacs)
Expect: ... :offset-arg-passed 162)
Actual: ... :offset-arg-passed 196)     <- run in a .claude/worktrees/agent-… worktree
Actual: ... :offset-arg-passed 154)     <- run in the main checkout
```

Same suite, same commit, same binaries; only the directory the harness runs in
differs.  No harness code has changed since the pin was written (5920b0788),
so nothing but the path moved.

There is no third value.  The `16254` that appears in the same report's `Diff:`
block, and reads like a wild Neomacs answer, is expect-test's **character-level**
diff with the ANSI colours stripped: `162` against `154` shares only the `1`, so
it renders `1` + `62` + `54`.  In this worktree the same block reads `1962`
(`162` against `196`, sharing `1` and `6`).  Anyone chasing 16254 as a value is
chasing a rendering.

The field is the last one in the record:

```elisp
:offset-arg-passed
(string-match-p "c[0-9]+" <the fake gocode's recorded argv>)
```

and the recorded argv is one argument per line, the fourth of which is the
visited file inside the per-case sandbox.  Reduced to that concatenation, run
under both editors:

```elisp
(let ((root "<checkout>/tmp/melpa"))
  (dolist (sandbox '("company-go-package-batch-Ab3xYz"           ; a real sandbox name
                     "xxxxxxxxcompany-go-package-batch-Ab3xYz"   ; 8 characters longer
                     "company-go-package-batch-Ac1xYz"))         ; same length, "c1" in the suffix
    (princ (format "%-40s %S\n"
                   sandbox
                   (string-match-p
                    "c[0-9]+"
                    (concat "-s\n-f=csv-with-package\nautocomplete\n"
                            root "/" sandbox "/company-go-fixture/main.go\n"
                            "c34\n"))))))
;; GNU                => 154, 162, 121
;; Neomacs before fix => 154, 162, 121
```

The two editors agree at every path length, and the value is
`36 + (length <path to main.go>) + 1` -- 36 being
`"-s\n-f=csv-with-package\nautocomplete\n"` -- and nothing else.  The pinned 162
is simply a checkout whose sandbox path was eight characters longer than the
main one; 154 is the main checkout today; 196 is a `.claude/worktrees/agent-…`
worktree of the same commit.  Nobody changed any code between them.

GNU's shape is why an index is the wrong thing to record here.
`string-match-p` is `string-match` with the match data inhibited
(lisp/subr.el:5941-5945), and `string-match` is "Return index of start of first
match for REGEXP in STRING" (src/search.c:442-444).  `string_match_1` ends at
`return make_fixnum (string_byte_to_char (string, val));` (src/search.c:437):
an absolute character index into the *whole* subject, counted from 0.  It has to
be absolute, because that is the number `substring`, `match-end` and a resumed
`string-match` all consume -- GNU has no notion of a position relative to the
interesting part of the subject, so every character in front of the match counts,
including a filesystem path the caller happened to concatenate in.  Our oracle
normaliser rewrites the sandbox root *inside strings*
(neomacs-test-oracle/src/lib.rs:256-271, the pair at 267-268); a number
computed from one of those strings passes through it untouched and carries the
path length into the snapshot, where the `:argv` field beside it -- normalised to
`[ORACLE-SANDBOX]/company-go-fixture/main.go` -- hides the very length the number
is made of.

Two independent ways this goes red with nobody touching a line of code:

* **Move the checkout.** 154 in the main checkout, 196 in a worktree, 162
  wherever the pin was written.
* **Stay put.** `MelpaSandbox::new` names each sandbox with `tempfile`'s six
  random alphanumerics (neomacs-melpa-test-support/src/lib.rs:121-131;
  `fastrand::alphanumeric`, tempfile-3.27.0/src/util.rs:15).  A suffix that
  happens to put a `c` next to a digit -- about one run in eighty -- moves the
  first `c[0-9]+` match into the directory name: 121 instead of 154 above, from
  `Ac1xYz` instead of `Ab3xYz`.  The pin was flaky in place as well as
  unportable.

Nothing here is byte-versus-char arithmetic, which is what it first looks like.
`company-go--invoke-autocomplete` builds the cursor argument as
`(concat "c" (int-to-string (- (point) 1)))` -- a plain character position;
`position-bytes` appears nowhere in the package -- and both editors compute
`c34` for the same buffer, which the record's own `:argv` field already pins.

The fix records the cursor *argument* instead of where it was found:

```elisp
:offset-arg
(cl-find-if (lambda (argument) (string-match-p "\\`c[0-9]+\\'" argument))
            (split-string argv "\n" t))
;; => "c34"
```

This is the only sense in which an elisp probe can make the bad state
unrepresentable: the field no longer holds a number derived from harness text at
all.  Every field of the record is now either a value the package computed
(`"c34"`, the candidate, its meta and package) or a string the normaliser can
rewrite (`:argv`).  It is also strictly stronger than what it replaced -- `162`
never asserted that the offset was 34, only that some `c<digits>` existed
somewhere in the file; `"c34"` pins the offset company-go actually computed, and
would catch an off-by-one that the old field could not see.

Worth knowing for the next one: **the existing batch-isolation audit already
detects this whole class**, because it re-runs the case under a second sandbox
whose label is a different length.  On the unfixed suite,
`NEOMACS_MELPA_AUDIT_BATCH_ISOLATION=1` reports

```
case `the_invocation_contract_through_a_fake_gocode` is not batch-safe:
  batched GNU Emacs:  … :offset-arg-passed 212)
  isolated GNU Emacs: … :offset-arg-passed 217)
```

-- exactly the five-character difference between the labels
`company-go-package-batch-isolation-audit` (40) and
`the-invocation-contract-through-a-fake-gocode` (45), and nothing to do with
batch state.  The audit passes on the fixed suite.  So when that audit says
"not batch-safe" and both editors move together, suspect a path-length record
before suspecting leaked state.

## 128. Synchronous subprocess output was always decoded as UTF-8, so `coding-system-for-read` did nothing and Dired lost the file name of every entry listed after a non-ASCII one -- FIXED

Surfaced by the `diredfl` suite (rank 449), whose whole subject is the face runs
`diredfl`'s `font-lock-keywords` paint onto Dired lines.  The fixture contains a
file named `café 界.md`, and every line *after* it lost its file name.

```elisp
(with-temp-buffer
  (let ((coding-system-for-read 'no-conversion))
    (call-process "printf" nil t nil "caf\\303\\251"))
  (list (buffer-size) (char-after 4) (char-after 5)))
;; GNU                => (5 4194243 4194217)
;; Neomacs before fix => (4 233 nil)
```

4194243 and 4194217 are the eight-bit characters for the raw bytes `0xC3` and
`0xA9`.  GNU leaves them alone because it was told not to convert; Neomacs
decoded them as UTF-8 into one `é`, and the buffer came out a character short.
`binary`, `raw-text`, `latin-1`, `utf-8-dos`, `default-process-coding-system`
and `process-coding-system-alist` were all equally inert, and an undefined
coding system was accepted silently instead of signalling:

| form (abbreviated; each ran `printf` into a temp buffer) | GNU | Neomacs before fix |
|---|---|---|
| no binding at all | `(4 233 nil)` | `(4 233 nil)` |
| `coding-system-for-read` = `no-conversion` / `binary` / `raw-text` | `(5 4194243 4194217)` | `(4 233 nil)` |
| `coding-system-for-read` = `latin-1` | `(5 195 169)` | `(4 233 nil)` |
| `default-process-coding-system` = `(binary . binary)` | `(5 4194243 4194217)` | `(4 233 nil)` |
| `process-coding-system-alist` = `(("printf" binary . binary))` | `(5 4194243 4194217)` | `(4 233 nil)` |
| `coding-system-for-read` = `utf-8-dos`, child writes `a\r\nb\r\n` (chars listed) | `(4 (97 10 98 10))` | `(6 (97 13 10 98 13 10))` |
| `coding-system-for-read` = `no-such-coding-xyz` | `(coding-system-error no-such-coding-xyz)` | no error |
| into a UNIBYTE destination buffer, `utf-8` | `(5 (99 97 102 195 169))` | `(4 (99 97 102 233))` |

> **Correction, 2026-08-17 (entry 134).**  The
> `no-conversion` / `binary` / `raw-text` row above groups three coding systems
> that do not behave alike.  They agree on this entry's payload, which has no
> line ending in it, and they disagree on the end-of-line axis: `raw-text`'s
> `eol_type` is a VECTOR, so GNU DETECTS the child's line endings for it, while
> `binary` and `no-conversion` are `Qunix` and copy every CR through.  Measured
> under GNU 31.0.90 with a child writing `caf <c3> <a9> CR LF x CR LF`:
> `raw-text` lands as `(99 97 102 4194243 4194217 10 120 10)` and `binary` as
> `(99 97 102 4194243 4194217 13 10 120 13 10)`.  Entry 131 flagged the row as
> understated; entry 134 measured and fixed it.  Nothing else in this entry
> changes.

### Why this lands on Dired, of all places

`insert-directory` (lisp/files.el:8398-8528) is built entirely on the guarantee
this broke.  It runs `ls --dired`, whose trailing `//DIRED//` line gives the
**byte** offsets of every file name in the output.  For `(+ beg OFFSET)` to be a
buffer position, the bytes must enter the buffer one-for-one, so GNU reads the
child with `coding-system-for-read` bound to `no-conversion` (:8406), marks the
names with `dired-filename` from those offsets (`insert-directory-clean`,
:8332-8336), and only THEN decodes the buffer chunk by chunk, re-applying
`dired-filename` after each `decode-coding-region` (:8517-8528).

Ignoring the binding makes the buffer shorter than the byte offsets by exactly
the byte/character excess of each non-ASCII name -- three for `café 界.md`.
Every later offset therefore points into the wrong text, and
`insert-directory-clean`'s guard `(memq (char-after end) '(?\n ?\s ?/ ?* ?@ ?%
?= ?|))` (:8334) then decides at random whether to skip the name entirely or
stamp `dired-filename` onto a three-character-shifted slice.  Measured over the
suite's own fixture (`(dired-move-to-filename)` at each line's beginning-of-line,
as an offset from it):

```
;; line                                                    GNU  Neomacs before fix
;;   -rw-r--r-- ... café 界.md                               44  44
;;   -rw-r--r-- ... compiled.elc                             44  44
;;   lrwxrwxrwx ... link-to-notes -> notes.org               44  47
;;   -rwxr-xr-x ... script.sh*                               44  47
;;   drwxr-xr-x ... subdir/                                  44   1
```

`dired-move-to-filename` (lisp/dired.el:3475-3501) tries the `dired-filename`
property FIRST -- `(next-single-property-change (point) 'dired-filename nil
eol)` (:3488) -- precisely because `ls --dired` is the authoritative answer and
`directory-listing-before-filename-regexp` is the fallback for `ls` builds that
lack it.  A shifted property is worse than no property: the fallback regexp
would have found the right column.

That is what the failing pins show.  `diredfl`'s directory rule is a
match-anchored highlighter whose PRE-MATCH-FORM is `(dired-move-to-filename)`:

```elisp
(list (concat dired-re-maybe-mark dired-re-inode-size "\\(d\\)[^:]")
      '(1 diredfl-dir-priv t)
      '(".+" (dired-move-to-filename) nil (0 diredfl-dir-name t)))
```

`font-lock-fontify-anchored-keywords` (lisp/font-lock.el) searches `".+"` from
wherever that form left point, to end of line.  With point misplaced at
column 1, `".+"` matched the whole line and `(0 diredfl-dir-name t)` -- override
`t` -- painted it, which also suppressed every later privilege keyword, since
those use the default no-override:

```
;; subdir line, per-character face runs
;; GNU                => (("  " nil) ("d" (diredfl-dir-priv)) ("r" (diredfl-read-priv))
;;                        ("w" (diredfl-write-priv)) ... ("subdir/" (diredfl-dir-name)))
;; Neomacs before fix => ((" " nil)
;;                        (" drwxr-xr-x 2 exec users 4096 Jan 15  2026 subdir/" (diredfl-dir-name)))
```

So the visible symptom was in font-lock, and font-lock was innocent: the
per-character runs, the anchored-highlighter limit, and the `keep`/`t`/`prepend`
override composition all already matched GNU.  Only the position handed to the
PRE-MATCH-FORM was wrong, and it was wrong because of a subprocess decoder.

### What GNU does, and why the chain has five steps

`Fcall_process` (src/callproc.c:729-763) resolves the decode coding system once,
after the child's pipe is ready to read:

```c
      if (!NILP (Vcoding_system_for_read))          /* :732 */
	val = Vcoding_system_for_read;
      else
	{
	  if (EQ (coding_systems, Qt))                /* :736 */
	    ...
	      coding_systems = Ffind_operation_coding_system (nargs + 1, args2);
	  if (CONSP (coding_systems))                 /* :746 */
	    val = XCAR (coding_systems);
	  else if (CONSP (Vdefault_process_coding_system))
	    val = XCAR (Vdefault_process_coding_system);
	  else
	    val = Qnil;
	}
      Fcheck_coding_system (val);                   /* :753 */
```

The order is not decoration.  `coding-system-for-read` is the dynamic override a
caller binds around one call -- `insert-directory` is exactly that caller, and so
is `url-open-stream`, and so is every "give me the bytes" helper in the tree.
`process-coding-system-alist` is the per-program rule a user configures.
`default-process-coding-system` is the ambient default.  A resolver that skips
the first level does not merely lose a feature; it breaks callers that *rely* on
being able to turn conversion off for one call, which is why the damage showed
up as mislaid text properties rather than as mojibake.

`Fcheck_coding_system` at the end is the reason an unknown name signals
`coding-system-error` rather than quietly falling back -- nil is a valid coding
system there (`Fcoding_system_p` accepts it), a typo is not.

### The type-level fix

`write_output_target_in_state` in `neovm-core/src/emacs_core/callproc/mod.rs`
took `(target, output_bytes, append)` and named its coding system inline:

```rust
let text = crate::encoding::decode_bytes_to_lisp_string(output, "utf-8-unix");
```

Nothing in the signature said a decision was owed, so the literal read as a
default rather than as an omission, and every synchronous subprocess in the
editor inherited it.

The decision is now a value, `ProcessOutputDecoding`
(`neovm-core/src/emacs_core/process.rs`), with the two shapes GNU's
`setup_coding_system` reduces to: `Bytes` (a `binary` / `no-conversion` /
`raw-text` coding, copied through as eight-bit characters) and `Coding(name)`.
The nil case belongs to the second, not the first: `setup_coding_system`
rewrites nil to `undecided` (src/coding.c:5675-5676), so the last resort
DETECTS.  Measured -- `(let ((default-process-coding-system nil)) ...)` leaves
GNU with four characters, not five -- which is the sort of thing "nil means no
coding system, so copy the bytes" gets wrong by reasoning instead of running.
It is a **required parameter** of `route_captured_output_in_state` and
`write_output_target_in_state`, so subprocess output cannot reach a buffer
without the call site first naming how it is decoded -- there is no signature
left that lets a decoder be invented at the point of insertion.  The
asynchronous reader (`decode_process_output_bytes`) now goes through the same
type, so the sync and async paths can no longer drift apart.

`resolve_call_process_output_decoding` implements that chain,
including `Fcheck_coding_system`, and runs where GNU runs it: after the child is
reaped, so a coding variable rebound while the child ran is observed exactly as
GNU observes it.

It also carries the fifth step, which is easy to read past in the C:

```c
      if (NILP (BVAR (current_buffer, enable_multibyte_characters))   /* :757 */
	  && !NILP (val))
	val = raw_text_coding_system (val);
```

`Fset_buffer (buffer)` ran at :722-723, so `current_buffer` there is the
DESTINATION buffer.  Into a unibyte one, GNU does no character-code conversion
at all -- but it keeps the resolved coding's end-of-line type, which is what
`raw_text_coding_system` returns.  Both halves are measurable, and they
disagree, so neither can be guessed:

```
;; child writes CR LF, destination buffer is unibyte
;; coding-system-for-read utf-8-dos  => GNU (4 (97 10 98 10))
;; coding-system-for-read utf-8-unix => GNU (6 (97 13 10 98 13 10))
```

`ProcessOutputDecoding::without_character_conversion` is that one function, and
the resolver applies it exactly when the destination buffer is unibyte.

One more thing had to be true for `no-conversion` to mean anything.
`decode_bytes_emacs` (`neovm-core/src/encoding.rs`) routed every non-UTF-8
coding system through `String::from_utf8_lossy`, which *decodes* well-formed
UTF-8 -- so even a correctly resolved `no-conversion` would have converted.  The
byte-preserving family now short-circuits to `str_to_multibyte` after EOL
conversion, which is what GNU's `decode_coding_raw_text` does.

### start-process failure text in the process buffer

When `start-process' is given a program that does not exist, GNU Emacs 31
writes the exec failure into the process buffer before the process dies:

    <emacs-binary>: <program>: No such file or directory

    Process x exited abnormally with code 127

Neomacs writes only the exit message.  Surfaced by the jedi suite's broken
server command workflow, where `jedi:epc--start-epc' reports the buffer
content in its startup error: GNU's report contains the `<emacs-binary>:'
line, Neomacs's starts directly with `Process epc:server:N exited
abnormally with code 127'.  The workflow asserts GNU's full report (with
the emacs-binary path normalized to `@@EMACS@@'), so the case stays red
in Neomacs.  The bare `start-process' reduction reproduces only when the
buffer is read with the same accept-process-output timing epc.el uses
(the exec-failure line lands before the exit message); it was verified
end-to-end through `epc:start-epc' with a nonexistent program.

### nov-mode flow hangs in Neomacs batch

The nov EPUB mode flow (find-file on an EPUB -> nov-mode -> work-dir
initialization -> container/OPF parsing -> shr render) hangs in Neomacs
--batch: the harness run stalls with the case active at the mode entry
for the full 180s timeout, while the identical flow completes in GNU
Emacs in milliseconds.  Bisecting the flow points at the require chain
or the mode initialization, not at the unzip subprocess (a recording
unzip stand-in does not change the hang) and not at a bare missing
require (`(require 'definitely-not-a-feature)' completes in both
editors).  The nov parity suite (7 GNU-green workflows: archive
validation, container parse, mode init + metadata + rendered TOC,
chapter navigation, metadata buffer, view-source, saved-place round
trip, link parsing) is therefore not registered; it is shelved at the
stub until the hang is root-caused.

### self-insert-command inside a case body kills the GNU batch process

The caml electric-pipe workflow (a case body that runs
`self-insert-command' -- both via `execute-kbd-macro' and via a direct
call with `last-command-event' bound -- inside the shared batch's eval)
kills the GNU Emacs batch process deterministically: the process exits
255 with an empty ERR outcome and a `void-variable neomacs--oracle-error'
in the harness's error handler.  The same case passes standalone
(`-Q --batch -l').  The powershell selection-helper cases that showed
the identical signature were transient (they pass consistently now);
the caml one reproduces on every run in both invocation variants, so
the workflow is left unexercised.

### Found and NOT fixed here

`make-process` has the same gap on its own precedence chain
(src/process.c:1953-1976): with `:coding` absent it ignores
`coding-system-for-read` too. Measured, printing a 12-byte UTF-8 name through
`printf` into a multibyte process buffer, with `coding-system-for-read` bound to
`binary`: GNU 34 characters, Neomacs 31.  An explicit `:coding 'binary` is
honoured in both, so only the fallback chain is missing.  `make-pipe-process`
(src/process.c:2526-2548) and `set_network_socket_coding_system`
(src/process.c:3245-3261) each spell the chain out again with a
buffer-multibyteness rule of their own -- and theirs is NOT `Fcall_process`'s:
where `Fcall_process` downgrades to `raw_text_coding_system (val)`, those leave
`val` nil outright, deliberately, so that "the existing Emacs Lisp libraries ...
receive bare code including a sequence of CR LF" (:2535-2539).  So these are
several separate resolvers that happen to rhyme, not one shared omission, and
sharing one implementation between them would be the wrong move.  No MELPA pin
depends on any of them yet; left for its own entry rather than folded in here.
## 129. The `rg` results-buffer pin recorded raw buffer positions, so it encoded where the checkout lives and failed for BOTH editors -- NOT A DIVERGENCE (harness defect), FIXED

The `rg` MELPA suite arrived red at
`neomacs-melpa-tests/src/parity_tests/rg/workflows.rs:83`, in the case that
runs a real `rg` subprocess with `--color=always` and pins the `rg-mode'
results buffer.  ANSI-to-face translation is not involved.  Under nextest the
batch reports the mismatch for BOTH editors and both report the SAME actual
value:

```
snapshot mismatches: a_search_populates_the_results_buffer_and_navigates_files (GNU Emacs),
                     a_search_populates_the_results_buffer_and_navigates_files (Neomacs)
```

Every field agreed with the pin -- `:content`, `:command-hidden`,
`:first-match-faces`, `:file-tags`, and both navigation `:line` strings.  Three
integers did not:

```
;; pinned                       :point 168   :next-file 570   :prev-file 478
;; GNU Emacs, this worktree     :point 202   :next-file 604   :prev-file 512
;; Neomacs,   this worktree     :point 202   :next-file 604   :prev-file 512
```

Those are absolute buffer positions in a buffer whose first line spells out an
absolute directory.  `compilation-start' opens a results buffer with a
mode-setter line built from `default-directory'
(lisp/progmodes/compile.el:2115-2121):

```elisp
(when (zerop (buffer-size))
  ;; Output a mode setter, for saving and later reloading this buffer.
  (compilation-insert-annotation
   "-*- mode: " name-of-mode
   "; default-directory: "
   (prin1-to-string (abbreviate-file-name default-directory))
   " -*-\n"))
```

GNU writes it for a documented reason: the line is a file-local-variables
cookie, so a saved `*compilation*' buffer reopens in the right mode with the
right directory and its relative filenames still resolve.  It has to be the
absolute directory for that to work: `abbreviate-file-name'
(lisp/files.el:2298-2330) only applies `directory-abbrev-alist' and substitutes
`~' for the home directory, and the sandbox fixture root is under neither.  The
oracle sandbox is created below the workspace root
(`MelpaSandbox::new', `neomacs-melpa-test-support/src/lib.rs:121-136`), so the
line is as long as the checkout path is deep, and every position after it moves
with it.

The mechanism reproduces in ten lines with no package involved, and the two
editors agree exactly:

```sh
FORM='(progn (require (quote compile))
  (let ((default-directory (car (last command-line-args))))
    (with-current-buffer (compilation-start "true" nil (lambda (_) "*probe*"))
      (while (get-buffer-process (current-buffer)) (accept-process-output nil 0.05))
      (prin1 (list :dir-length (length default-directory) :point-max (point-max))))))'
for d in ./tmp/dir-a/ ./tmp/dir-aaaaaaaa/; do mkdir -p $d; EDITOR -Q --batch --eval "$FORM" $d; done
```

```elisp
;; GNU               => (:dir-length 107 :point-max 259) (:dir-length 114 :point-max 266)
;; Neomacs before fix => (:dir-length 107 :point-max 259) (:dir-length 114 :point-max 266)
```

Seven more characters of directory, seven more of buffer, in both editors.
Measured the same way through the suite's own probe: a 104-character sandbox
path yields `:point 164`, a 108-character one yields `:point 168`, and the
142-character sandbox nextest builds from an agent worktree yields `:point
202` -- in GNU Emacs and in Neomacs alike, `point = 60 + (length sandbox)`.

So the pinned `168` was never a statement about either editor.  It records a
56-character workspace root, which is neither the main checkout
(`/home/exec/Projects/github.com/eval-exec/neomacs`, 48 characters, which would
pin `160`) nor any worktree under it.  The case could not have passed where it
was committed; it was generated somewhere else and the number travelled.  This
is the integer form of the leak the harness already masks for strings -- the
oracle rewrites the sandbox path to `[ORACLE-SANDBOX]` in `:content` -- and a
mask cannot fix an integer, because by the time the position is a number the
path it encodes is gone.

The fix removes the raw position rather than masking it.  A new prelude helper
takes every position the suite reports:

```elisp
(defun rg-test-offset (position)
  (- position (save-excursion (goto-char (point-min)) (line-end-position))))
```

and the snapshot keys are renamed `:point-offset`, so the pin says what it
holds.  Subtracting the end of the one line that carries the environment makes
the checkout path unrepresentable in the recorded value: there is no longer a
number in this suite's snapshot that any absolute path can reach.  The offsets
are `272 / 402 / 310` and were verified identical across two sandbox path
lengths and between the two editors.

The rule for anyone adding a package case: a raw `(point)` out of a
`compilation-mode' buffer -- rg, ag, grep, rgrep, quickrun, any of them -- is
not a pinnable value.  Report it relative to something inside the buffer.

A second defect surfaced while measuring the first, in the field that looked
most interesting.  The case's docstring promises that "the first match on each
row carries `rg-match-face'", and the probe located that match with
`(search-forward "widget")` from `point-min'.  The first hit is inside the
fixture directory name on the header line, so the pinned `:first-match-faces`
walked the tail of that header -- `("w") ("i") ("d") ("g") ("e") ("t") ("s")
("/") ("\"") (" ") ("-") ("*") ("-")` -- and reported `nil` for all thirteen
characters.  It also read the wrong property: `rg-filter' writes the highlight
as `(propertize TEXT 'face nil 'font-lock-face 'rg-match-face)'
(`rg-result.el:475-478` in the pinned 20260517.1310 build), so `'face' is `nil'
on a real match row too.  The field named after the match highlight could not
have detected a lost highlight, a highlight on the wrong span, or a highlight
written to the wrong property.  It now locates the row by its full match text
-- as the `wgrep` case in the same file already did, for the same reason -- and
records both properties, pinning `("w" nil rg-match-face)` for each of the six
match characters and `("o" nil nil)` for the trailing text.  Measured
identical in GNU Emacs 31.0.90 and Neomacs.

Nothing in this entry is a Neomacs defect.  Every field of the record, before
and after both changes, is byte-identical between the two editors.

## 130. Two C-level minibuffer hooks were shipped pre-populated, so the `add-hook' calls that build them in GNU were no-ops -- FIXED

Root cause and fix for entry 119, which recorded the symptom and guessed
wrong about the cause.  The window-numbering parity suite
(`neomacs-melpa-tests/src/parity_tests/window_numbering/workflows.rs:52`)
compares the whole `minibuffer-setup-hook' after the package prepends
`window-numbering-update' to it; every function on the hook is identical in
both editors and only the ORDER differs, so the package itself is a
bystander.

```elisp
(list (default-value 'minibuffer-setup-hook)
      (default-value 'minibuffer-exit-hook))
;; GNU                => ((rfn-eshadow-setup-minibuffer minibuffer--regexp-setup
;;                         minibuffer--nonselected-setup
;;                         minibuffer-setup-on-screen-keyboard
;;                         minibuffer-error-initialize
;;                         minibuffer-history-isearch-setup
;;                         minibuffer-history-initialize)
;;                        (minibuffer--regexp-exit minibuffer--nonselected-exit
;;                         minibuffer-exit-on-screen-keyboard
;;                         minibuffer-restore-windows))
;; Neomacs before fix => ((minibuffer--nonselected-setup
;;                         rfn-eshadow-setup-minibuffer minibuffer--regexp-setup
;;                         minibuffer-setup-on-screen-keyboard
;;                         minibuffer-error-initialize
;;                         minibuffer-history-isearch-setup
;;                         minibuffer-history-initialize)
;;                        (minibuffer--nonselected-exit minibuffer--regexp-exit
;;                         minibuffer-exit-on-screen-keyboard
;;                         minibuffer-restore-windows))
```

Measured with `-Q --batch` against GNU 31.0.90 and against a verified build
of origin/main.  Note the exit hook, which entry 119 did not look at: it is
wrong in the same way and rfn-eshadow.el never touches it, which already
rules out the "after-load sequencing" hypothesis -- `minibuffer--regexp-exit'
and `minibuffer--nonselected-exit' are added by two modes in the SAME file.

### The variable belongs to C; the list belongs to Lisp

GNU `src/minibuf.c:2553-2559` DEFVARs both hooks and sets each to `Qnil':

```c
  DEFVAR_LISP ("minibuffer-setup-hook", Vminibuffer_setup_hook, ...);
  Vminibuffer_setup_hook = Qnil;

  DEFVAR_LISP ("minibuffer-exit-hook", Vminibuffer_exit_hook, ...);
  Vminibuffer_exit_hook = Qnil;
```

Every entry a running Emacs finds there is put on by an `add-hook' in
preloaded Lisp while loadup runs: simple.el:2888, 3271 and 3392;
minibuffer.el:3048, 5498 and 5499; and the three `:global' minor modes
`minibuffer-regexp-mode' (minibuffer.el:5641), `minibuffer-nonselected-mode'
(minibuffer.el:5738) and `file-name-shadow-mode' (rfn-eshadow.el:207).
`add-hook' conses onto the FRONT (subr.el:2378) and does nothing when the
function is already a member, so the finished list is a verbatim record of
preload order, read newest-first.  loadup.el loads simple (251), then
minibuffer (254), then rfn-eshadow (273), which is exactly the order GNU's
list reports.

Neomacs seeded the two variables in Rust
(`neovm-core/src/emacs_core/eval.rs`, `minibuffer-setup-hook' and
`minibuffer-exit-hook') with a literal function list -- GNU's post-loadup
value as it stood BEFORE `minibuffer-nonselected-mode' was added in Emacs
31.1.  Every `add-hook' above then found its function already present and
did nothing, so the seeded order survived untouched; the one function NOT in
the frozen snapshot, `minibuffer--nonselected-setup', was genuinely consed
on and therefore landed at the HEAD instead of third.  The seed did not
merely duplicate the Lisp default: it froze the list against every future
preloaded mode.

### The seed underneath the seed

Deleting the two hook seeds alone made things worse, and finding out why is
the second half of this entry.  Instrumenting `lisp/loadup.el' in a copied
runtime tree and running the real dump stage
(`neomacs-temacs --batch -l loadup --temacs=pdump`) printed the hook after
each preloaded file; with only the hooks cleared, `minibuffer--regexp-setup'
and `minibuffer--regexp-exit' never appeared at all.

The cause is a THIRD invented default, `minibuffer-regexp-mode', seeded to
`t' in the same Rust function.  That symbol has no C definition in GNU at
all: it is the `define-minor-mode' at minibuffer.el:5641, whose generated
`defcustom' carries `:set #'custom-set-minor-mode' and
`:initialize #'custom-initialize-after-file-load' (easy-mmode.el:304 and
336).  The initializer defers to after the file loads (custom.el:163-182,
via `after-load-functions', run by `do-after-load-evaluation',
subr.el:6422-6456) and then calls `custom-initialize-set', which is
(custom.el:68-82):

```elisp
(defun custom-initialize-set (symbol exp)
  (condition-case nil
      (default-toplevel-value symbol)          ; already has a default?
    (error                                     ; only then:
     (funcall (or (get symbol 'custom-set) #'set-default-toplevel-value) ...))))
```

If the symbol already has a default top-level value the function returns
having done nothing, so `custom-set-minor-mode' is never called -- and
`custom-set-minor-mode' is the ONLY thing that ever calls the mode function,
which is where the mode's `add-hook' calls live.  Measured in both editors,
so the semantics are not in dispute:

```elisp
;; A stand-in global minor mode, declared twice: once with the variable
;; unbound (GNU's state before minibuffer.el runs), once pre-bound the way
;; the Rust seed left it.
(defun probe (pre-bound)
  (let ((sym (if pre-bound 'seeded-mode 'clean-mode)) (log nil))
    (when pre-bound (set-default sym t))
    (fset sym (lambda (&optional arg)
                (set-default sym (and arg (> arg 0)))
                (push 'mode-body-ran log)))
    (custom-declare-variable sym (lambda () t) "Stand-in global minor mode."
                             :set #'custom-set-minor-mode
                             :initialize #'custom-initialize-set
                             :type 'boolean)
    (list :pre-bound pre-bound :value (default-value sym)
          :effects (nreverse log))))
(list (probe nil) (probe t))
;; GNU     => ((:pre-bound nil :value t :effects (mode-body-ran))
;;             (:pre-bound t   :value t :effects nil))
;; Neomacs => ((:pre-bound nil :value t :effects (mode-body-ran))
;;             (:pre-bound t   :value t :effects nil))
```

So `minibuffer-regexp-mode' read `t' in Neomacs while its machinery had
never been installed -- a mode that reports itself on and is off.  It was
invisible only because the hook seed happened to contain the very two
functions the suppressed mode body would have added.  `file-name-shadow-mode'
and `minibuffer-nonselected-mode' were unaffected because nothing pre-bound
them.  This is the same failure shape as ledger 125 and the
`completion-ignored'/`resize-mini-windows' cases: a Rust default invented for
a symbol Lisp owns.

### The fix

The three seeds are gone.  A hook whose variable lives in C is now created
through `Obarray::define_c_hook_variable(name)`
(`neovm-core/src/emacs_core/symbol.rs`), which takes NO value argument and
always installs nil, so "seed a C hook with a function list" is not
expressible through it; every C-level hook DEFVAR in the engine
(`after-insert-file-functions', `delete-terminal-functions',
`display-monitors-changed-functions', `kbd-macro-termination-hook',
`kill-emacs-hook', `minibuffer-exit-hook', `minibuffer-setup-hook',
`mouse-leave-buffer-hook', `post-command-hook', `post-select-region-hook',
`pre-command-hook', `resume-tty-functions', `suspend-tty-functions',
`write-region-annotate-functions') now goes through it.
`minibuffer-regexp-mode' is simply not registered in Rust any more, next to
the `minibuffer-completion-auto-choose' comment that already warned about
this exact hazard three lines away.

Two tests pin it: one bare-context test asserting every C-level hook starts
nil and `minibuffer-regexp-mode' starts unbound, and one bootstrap test
asserting the post-loadup order of both hooks equals GNU's.

Checked and deliberately left alone: the engine's other non-nil `...-mode'
seeds (`menu-bar-mode', `tool-bar-mode', `auto-composition-mode',
`auto-hscroll-mode', `indent-tabs-mode') are all genuine C DEFVARs in GNU
with the same values -- `Vmenu_bar_mode = Qt' and `Vtool_bar_mode = Qt' at
src/frame.c:7527-7550 -- and their `define-minor-mode' forms pass
`:variable' precisely so the macro does not redeclare them ("It's defined in
C/cus-start, this stops the d-m-m macro defining it again", menu-bar.el:2631).
`minibuffer-regexp-mode' was the only one of that group with no C definition
behind it.


Status: FIXED.

## 131. Asynchronous subprocess output was decoded by a coding system invented in Rust, so `make-process` ignored `coding-system-for-read` and a unibyte process buffer still converted characters -- FIXED

The half of entry 128 that entry 128 scoped out and handed on.  `call-process`
now resolves its output coding system; `make-process`, `start-process` and
everything built on them did not, and the reason they looked fine is that the
answer they never computed happened to coincide with a literal.

```elisp
(let ((buf (generate-new-buffer " *p*")))
  (let ((p (let ((coding-system-for-read 'binary))
             (make-process :name "p" :buffer buf :sentinel #'ignore
                           :command '("printf" "caf\\303\\251")))))
    (while (accept-process-output p 1))
    (while (process-live-p p) (accept-process-output p 0.05)))
  (append (with-current-buffer buf (buffer-string)) nil))
;; GNU                => (99 97 102 4194243 4194217)
;; Neomacs before fix => (99 97 102 233)
```

4194243 and 4194217 are the eight-bit characters for the raw bytes `0xC3` and
`0xA9`.  Every step of the chain was equally inert, an undefined coding system
was accepted silently, and a unibyte process buffer -- which must not have
character-code conversion applied to it at all -- got it anyway:

| form (each ran `printf` through `make-process` into a process buffer) | GNU | Neomacs before fix |
|---|---|---|
| `coding-system-for-read` = `binary` / `no-conversion` / `raw-text` | `(99 97 102 4194243 4194217)` | `(99 97 102 233)` |
| `coding-system-for-read` = `latin-1` | `(99 97 102 195 169)` | `(99 97 102 233)` |
| `process-coding-system-alist` = `(("printf" binary . binary))` | `(99 97 102 4194243 4194217)` | `(99 97 102 233)` |
| `default-process-coding-system` = `(binary . binary)` | `(99 97 102 4194243 4194217)` | `(99 97 102 233)` |
| `:coding '(nil . latin-1)` under `coding-system-for-read` = `binary` | `(99 97 102 233)` | `(99 97 102 4194243 4194217)` |
| `coding-system-for-read` = `no-such-coding-xyz` | `(coding-system-error no-such-coding-xyz)` | no error |
| `:coding 'utf-8-dos`, UNIBYTE process buffer, child writes `caf<c3><a9>\r\n` | `(99 97 102 195 169 10)` | `(99 97 102 233 10)` |
| `:coding 'utf-8-unix`, UNIBYTE process buffer, same child | `(99 97 102 195 169 13 10)` | `(99 97 102 233 13 10)` |
| `coding-system-for-write` = `latin-1`, then `(process-coding-system p)` | `(utf-8-unix . latin-1)` | `(utf-8-unix . utf-8-unix)` |
| `default-process-coding-system` = `(latin-1 . koi8-r)`, same | `(latin-1 . koi8-r)` | `(utf-8-unix . utf-8-unix)` |
| `:coding 'no-such-xyz` with `:command '("no-such-program-xyz")` | `(file-missing "Searching for program")` | `(coding-system-error no-such-xyz)` |

Measured with `-Q --batch` against GNU Emacs 31.0.90 and against a verified
build of origin/main (`tmp/refbin/neomacs`); the probes are
`tmp/pw131/pin.el` and `tmp/pw131/pin2.el`.  After the fix the same probes run
under a `cargo xtask fresh-build --release` binary answer byte-for-byte what
GNU answers on every form above -- and on the two orderings below, and on the
`set-process-buffer` case further down.  The only residue is the `raw-text-*`
NAME `process-coding-system` reports for a unibyte buffer, which is the
write-back recorded at the end of this entry; the decoded TEXT agrees.

### Why nothing noticed

`neovm-core/src/emacs_core/process.rs` created every process with

```rust
coding_decode: Value::symbol("utf-8-unix"),
coding_encode: Value::symbol("utf-8-unix"),
```

and `make-process` overwrote it only when the caller passed `:coding`.  That
literal is not a neutral placeholder -- it is a plausible-looking *answer*, and
it is the answer the chain produces in the common case, because
`default-process-coding-system` really is `(utf-8-unix . utf-8-unix)` in both
editors under a UTF-8 locale.  So the missing chain was invisible for every
caller who did not bind anything, and visible only to the callers who bind
something precisely because they must: `insert-directory` binding
`coding-system-for-read` to `no-conversion`, `url-open-stream` binding it to
`binary`, every "give me the bytes" helper in the tree.  This is the recurring
shape recorded for `completion-ignored`, `resize-mini-windows`,
`normal-erase-is-backspace` and ledgers 125 and 130: a value Lisp owns,
answered by a Rust literal.

`start-process` had the same hole through a second door.  GNU's is Lisp that
does nothing but call `make-process` (lisp/subr.el:3466-3472); Neomacs has a
separate Rust `builtin_start_process` that built its own process record and
never touched coding at all, so it too rode on the literal.

### Four resolvers that rhyme -- where they really agree

Entry 128 warned that unifying `Fcall_process`, `Fmake_process`,
`Fmake_pipe_process` and `set_network_socket_coding_system` would be wrong.
Read side by side that is right, and there are FIVE of them, not four --
`Fmake_serial_process` (src/process.c:3247-3275) spells the chain out a fifth
time.  They share a precedence *spine* -- explicit override, then
`coding-system-for-read`, then `(find-operation-coding-system ...)`, then
`default-process-coding-system`, then nil -- and differ in four independent
ways, none of which is a policy flag:

| | explicit override | dynamic override | unibyte buffer short-circuit | `find-operation-coding-system` | `default-process-coding-system` | validates |
|---|---|---|---|---|---|---|
| `Fcall_process` (callproc.c:729-763) | none | :732 | no | `call-process` + the whole argument vector, :741-744 | :748 | `Fcheck_coding_system`, :753 |
| `Fmake_process` (process.c:1950-1977) | `:coding` car, :1950-1956 | :1958 | no -- see :1942-1944 | `start-process` + NAME BUFFER COMMAND, and only if PROGRAM, :1965-1971 | :1974 | no |
| `Fmake_pipe_process` (process.c:2523-2548) | `:coding` car, :2523-2530 | :2531 | **yes**, val = nil, :2533-2539 | never -- `coding_systems` is `Qt` at :2520 and never assigned | :2544 | no |
| `Fmake_serial_process` (process.c:3247-3261) | `:coding` car, :3250-3255 | :3256 | **yes**, val = nil, :3258-3260 | never -- same dead `Qt`, :3298 | **never** | no |
| `set_network_socket_coding_system` (process.c:3301-3337) | `:coding` car, :3306-3311 | :3312 | **yes**, val = nil, :3314-3322 | `open-network-stream` + NAME BUFFER HOST SERVICE, and only if both, :3325-3330 | :3333 | no |

> **Corrected and extended, 2026-08-17, by entry 137**, which implemented the
> three rows this table was written as a spec for and re-read the C for each.
> Every conclusion holds; four cells do not, and one column is missing.
>
> * The **unibyte buffer short-circuit** column is not one column.  GNU asks a
>   DIFFERENT buffer for the two halves: pipe and network ask the process buffer
>   for decode and `current_buffer` for encode, serial asks the process buffer
>   for both.  Measured -- `make-pipe-process` with a unibyte CURRENT buffer and
>   a multibyte process buffer answers `(utf-8-unix . nil)`.
> * Serial's **`find-operation-coding-system`** cell says "same dead `Qt`,
>   :3298".  :3298 is `set_network_socket_coding_system`'s `coding_systems`,
>   where the variable is alive; only `Fmake_pipe_process`'s (:2520) is the dead
>   one.  `Fmake_serial_process` has no `coding_systems` variable at all.  The
>   conclusion -- neither reaches the alist -- is right for both, by two
>   different mechanisms.
> * The **validates** column is right about the resolvers and misleading about
>   the primitives: all four asynchronous ones signal `coding-system-error` for
>   an undefined RESOLVED coding, from `setup_process_coding_systems` ->
>   `setup_coding_system`.  Read as "an undefined name is accepted" it is wrong,
>   and that is the reading `make-network-process` shipped.
> * A **missing column**: whether a supplied `:coding` whose half is nil falls
>   through to the tail.  `Fmake_process` yes (its tail is a separate
>   `if (NILP (val))` at :1958); pipe, serial and network no (one `else if`
>   chain).  Measured, `:coding '(nil . latin-1)` under `coding-system-for-read`
>   `binary`: `(utf-8-unix . latin-1)` for `make-process`, `(nil . latin-1)` for
>   the other three.
> * Line ranges have drifted about ten lines.  Current: pipe :2517-2570, serial
>   :3247-3275, network :3291-3373.

Those are different *inputs*, not different settings of one input.  A shared
core parameterised by all five columns would have a parameter per caller, which
is another way of saying there is nothing left to share.  Two of the columns are
worth stating outright because they are easy to read past: the pipe and serial
resolvers can never reach `process-coding-system-alist` at all (the
`CONSP (coding_systems)` arm is unreachable code in both), and the serial one
never reaches `default-process-coding-system` either.

The one column that is NOT a difference is the interesting one.  Entry 128 found
the unibyte-destination rule inline in `Fcall_process` (src/callproc.c:757-759)
and read the pipe/network `val = Qnil` branches as the same rule with the
opposite sign.  They are not the same rule.  `Fcall_process` has no process
object and no file-descriptor table, so it must apply the rule on the spot; the
four asynchronous resolvers do not apply it at all, and `Fmake_process` says so
in its own comment:

```c
    /* Decide coding systems for communicating with the process.  Here
       we don't setup the structure coding_system nor pay attention to
       unibyte mode.  They are done in create_process.  */   /* :1942-1944 */
```

For every asynchronous process the rule lives in exactly ONE place,
`setup_process_coding_systems` (src/process.c:8380-8409):

```c
  coding_system = p->decode_coding_system;
  if (EQ (p->filter, Qinternal_default_process_filter)      /* :8395 */
      && BUFFERP (p->buffer))
    {
      if (NILP (BVAR (XBUFFER (p->buffer), enable_multibyte_characters)))
	coding_system = raw_text_coding_system (coding_system);   /* :8399 */
    }
  setup_coding_system (coding_system, proc_decode_coding_system[inch]);
```

That is the *same* `raw_text_coding_system` call `Fcall_process` makes, keyed on
the process's CURRENT buffer and filter, and GNU re-runs it from
`set-process-buffer` (:1312), `set-process-filter` (:1404), `create_process`
(:2277) and `set-process-coding-system` (:8036).  So the `val = Qnil` branches
in pipe/serial/network are not an EOL policy: they are a short-circuit that
skips the alist and default lookup and leaves the user-visible
`process-coding-system` nil, while the fd-level downgrade still happens
underneath whenever the default filter is inserting into a unibyte buffer.

Both halves are measurable, and the re-evaluation is measurable too:

```elisp
;; created against a MULTIBYTE buffer, handed a UNIBYTE one afterwards
(let ((mb (generate-new-buffer " *mb*")) (ub (generate-new-buffer " *ub*")))
  (with-current-buffer ub (set-buffer-multibyte nil))
  (let ((p (let ((coding-system-for-read 'utf-8-dos))
             (make-process :name "z" :buffer mb :sentinel #'ignore
                           :command '("sh" "-c" "sleep 0.3; printf 'a\\r\\nb\\r\\n'")))))
    (set-process-buffer p ub)
    (while (accept-process-output p 1))
    (car (process-coding-system p))))
;; GNU => raw-text-dos       ; not utf-8-dos: the rule ran against the NEW buffer
```

So: five honest resolvers, and one shared second stage.  That is the shape the
fix takes.

### The two counter-intuitive facts hold here too

Entry 128 had to measure both rather than reason about them.  Both hold on the
asynchronous path, and one of them was being got wrong in the opposite direction:

`nil` means DETECT, not "copy the bytes".  `setup_coding_system` rewrites nil to
`undecided` (src/coding.c:5675-5676), and `raw_text_coding_system (Qnil)` returns
bare `raw-text` (src/coding.c:5939-5940), not a byte-faithful subsidiary.
`decode_process_output_bytes` believed the opposite, citing the pipe/network CR LF
comment for it:

```elisp
(let ((buf (generate-new-buffer " *p*")))
  (let ((p (make-process :name "z" :buffer buf :sentinel #'ignore
                         :command '("sh" "-c" "sleep 0.3; printf 'caf\\303\\251'"))))
    (set-process-coding-system p nil nil)
    (while (accept-process-output p 1))
    (while (process-live-p p) (accept-process-output p 0.05)))
  (with-current-buffer buf (buffer-size)))
;; GNU                => 4    ; undecided, detected as UTF-8
;; Neomacs before fix => 5    ; bytes copied through
```

A unibyte destination drops character conversion but KEEPS end-of-line
conversion.  The two halves disagree, so neither can be guessed:

```
;; child writes  caf <c3> <a9> CR LF  into a UNIBYTE process buffer
;; :coding utf-8-dos  => GNU (99 97 102 195 169 10)      ; no chars converted, CR eaten
;; :coding utf-8-unix => GNU (99 97 102 195 169 13 10)   ; no chars converted, CR kept
```

### The type-level fix

The decision was a hole in a signature.  `decode_process_output_bytes` took
`(proc, bytes, flush)`, reached into `proc.coding_decode` and classified it
inline, so "which decoder" was answered at the point of insertion by whatever
that one function believed -- exactly the shape entry 128 removed from the
synchronous path.

GNU's two stages are now two types.

`ProcessOutputSink` (`neovm-core/src/emacs_core/process.rs`) is
`setup_process_coding_systems`' question, and it has GNU's two answers and no
others: `UnibyteProcessBuffer` (the internal default filter is inserting into a
live unibyte buffer) or `DecodedText` (a multibyte buffer, no buffer, or a Lisp
filter -- a filter is handed a decoded string, so GNU leaves the coding system
alone for it).  It is a REQUIRED parameter of `decode_process_output_bytes`,
`read_process_output` and every read helper between them, so there is no
signature left in which process output can be turned into text without the call
site first naming where it is going.  It is derived per read from the process's
live buffer and filter rather than cached, which is the same function GNU
computes with nothing left to invalidate -- and that is what makes
`set-process-buffer` behave, above.  `ProcessOutputDecoding::for_process` is the
one place the `raw_text_coding_system` downgrade is applied, reusing the
`without_character_conversion` entry 128 already measured.

`MakeProcessCodingEnvironment` is `Fmake_process`'s chain, and it is a required
parameter of the process creator.  A real subprocess can no longer be created
without its ambient coding environment being supplied, so the Rust literal
cannot stand in for a resolution that never ran.  The chain itself is
`resolve_make_process_coding_systems`, written once over a `ProcessCodingHalf`
because GNU writes the decode and encode blocks as near-mirrors
(src/process.c:1950-1977 and :1979-2008) -- the two blocks differ only in which
half of a cons they take and which dynamic variable overrides.  That is the only
sharing in the fix; the five per-caller resolvers stay five.  `ProcessCodingSystems`
has no `Default` and no field-wise constructor for the same reason.

Three details of the chain are load-bearing and each was measured, not derived:

* A SUPPLIED `:coding` shuts the dynamic override out even when its own half is
  nil.  GNU's `else` at :1957 is reached only when `tem` itself is nil, so
  `:coding '(nil . latin-1)` resumes at the alist and never consults
  `coding-system-for-read`.
* The alist is matched against the PROGRAM, not the process name --
  `start-process` has `target-idx` 2 (src/coding.c:11784), so
  `find-operation-coding-system` picks `args[3]`, the car of `:command`
  (src/coding.c:10822).  An entry keyed on the process name does not fire.
* GNU does not validate in the resolver.  The `coding-system-error` comes from
  `setup_coding_system` (src/coding.c:5678) reached through
  `create_process` -> `setup_process_coding_systems` (src/process.c:2277,
  commented "This may signal an error"), which is AFTER the program has been
  found.  Neomacs validated `:coding` eagerly, before the executable search, and
  so reported the wrong one of two errors; the check now sits where GNU's
  effect is, next to the resolver and after the search.  Nothing is left behind
  when it fires: measured, GNU's process count is unchanged across the signal.

`builtin_start_process` rebuilds its arguments in keyword form and runs the same
resolver, because GNU's `start-process` IS `make-process`.  The right long-term
answer is that the Rust `start-process` should not exist at all -- it is a Rust
shadow of a nine-line Lisp function in subr.el, and this is the second time it
has drifted -- but deleting a registered subr is its own change with its own
blast radius, and it is recorded here rather than folded in.

`coding_explicitly_set` deliberately still means "the caller passed `:coding`",
not "a coding system was resolved": a PTY status-reporting heuristic keys off
it, and the resolver answering for an absent `:coding` must not look like an
explicit one.

Three tests pin the three faces of this: the decode chain including the
`(nil . X)` and no-PROGRAM cases, the unibyte-destination rule with its
disagreeing halves, its Lisp-filter exemption and the `set-process-buffer`
re-evaluation above, and the encode chain.  Every expected value in them was
taken by running the probe under GNU.  Two of them bind
`default-process-coding-system` explicitly rather than inheriting it: the unit
bootstrap leaves it at `(undecided-unix . utf-8-unix)` while both shipped
editors have `(utf-8-unix . utf-8-unix)`, and a pin that inherits it would be
recording the runtime rather than the behaviour.

One pre-existing test moved from `let` to `let*`
(`process_coding_tty_and_kill_buffer_query_runtime_surface`): it binds
`default-process-coding-system` and then calls `start-process` in the same
binding list, so the process was created before the binding took effect.  Its
own comment says the binding exists to be seen by the process.  That was
invisible for exactly as long as the process coding was a literal.

### Found and NOT fixed here

**End-of-line DETECTION is missing from the decoder.**  A coding system whose
eol_type is unspecified detects the child's line endings in GNU; Neomacs applies
only explicit `-dos`/`-mac`/`-unix` subsidiaries.  This is not specific to
processes -- it is the shared string decoder:

```elisp
(mapcar (lambda (cs) (append (decode-coding-string "a\r\nb\r\n" cs) nil))
        '(raw-text undecided utf-8 raw-text-dos raw-text-unix binary))
;; GNU     => ((97 10 98 10) (97 10 98 10) (97 10 98 10)
;;             (97 10 98 10) (97 13 10 98 13 10) (97 13 10 98 13 10))
;; Neomacs => ((97 13 10 98 13 10) (97 13 10 98 13 10) (97 13 10 98 13 10)
;;             (97 10 98 10) (97 13 10 98 13 10) (97 13 10 98 13 10))
```

Only the first three diverge, and they are exactly the undecided-eol cases.  It
reaches `call-process` and `make-process` through the same decoder, so entry
128's `coding-system-for-read` = `raw-text` row would read `(97 10 98 10)` in
GNU and `(97 13 10 98 13 10)` here.  It also means
`ProcessOutputDecoding::Bytes` currently conflates `raw-text` (which detects
EOL) with `binary`/`no-conversion` (which do not) without any measurable
consequence -- the conflation becomes wrong the moment detection lands, and the
type is deliberately left in place so that it does.  Fixing detection changes
every `decode-coding-string`, `insert-file-contents` and file-visiting path in
the editor and belongs to its own entry with its own gate run.

> **Closed, 2026-08-17, by entry 134.**  Detection landed, and the
> `ProcessOutputDecoding::Bytes` prediction above held exactly as written: the
> type now covers only `binary` and `no-conversion`, and `raw-text` goes through
> the shared decoder.  Entry 134 also carries the correction to entry 128's
> `raw-text` row that this paragraph predicted.  The second residual below --
> `Vlast_coding_system_used` written back onto the process -- is NOT closed and
> is still open; entry 134 fixes `last-coding-system-used` for strings, regions
> and files, which is a different slot from `(process-coding-system P)`.

**`Vlast_coding_system_used` is not written back onto the process.**  After
decoding, GNU replaces `p->decode_coding_system` with the coding actually used
(`read_process_output_set_last_coding_system`, src/process.c:6417-6425), so
`(process-coding-system p)` reports `raw-text-unix` for a unibyte buffer and the
detected coding for an `undecided` one.  Neomacs reports the coding that was
resolved.  The decoded TEXT is unaffected -- this is the reporting slot only --
and it is a separate mechanism from the chain fixed here.

> **Closed, 2026-08-18, by entry 139.**  Right about the mechanism, and it
> understates the reach in three ways.  The write-back also completes the
> process's ENCODE coding system when that was still nil
> (`coding_inherit_eol_type`, src/process.c:6442-6444).  The same variable is
> written by `call-process` (src/callproc.c:913), which had no write at all
> here.  And "the decoded TEXT is unaffected" is true of one read and false of
> the next: because GNU writes the detected coding back onto the process, the
> NEXT chunk is decoded with it, which is entry 134's per-chunk residual.  The
> two really were one mechanism, as 134 predicted.

**`make-pipe-process`, `make-serial-process` and `make-network-process` still
owe their own resolvers.**  The table above is what each of them has to
implement; none of them is `Fmake_process`'s, and the network one is partly
there already (`network_process_coding_pair`).  The
`(utf-8-unix . utf-8-unix)` initialiser survives as their stand-in and now says
so in a comment: for a multibyte buffer with no `:coding`, and given that
neither pipe nor serial can reach `find-operation-coding-system`, GNU's answer
really is the car and cdr of `default-process-coding-system`, whose value here
is that pair.  It is a stand-in, not a default.  No MELPA pin depends on any of
them yet.

> **Closed, 2026-08-17, by entry 137.**  All three have their resolvers, the
> `utf-8-unix` initialiser is gone from the tree, and the pair is now a required
> parameter of every process constructor.  Two of the three notes above were
> right and one was not.  `make-network-process` was more than "partly there":
> its chain was byte-identical to GNU on every row, and what it lacked was the
> CHECK, not the resolution.  The stand-in's justification -- that for a
> multibyte buffer with no `:coding` GNU's answer is the car and cdr of
> `default-process-coding-system` -- is true of `make-pipe-process` and FALSE of
> `make-serial-process`, whose chain has no such step: GNU answers `(nil . nil)`
> there, in every configuration that does not bind `:coding` or
> `coding-system-for-read`/`-write`.  See entry 137 for the measurements and for
> four corrections to the table above.


Status: FIXED.

## 132. Every `DEFVAR_INT' variable accepted values GNU refuses, because the forward type's rule lived at the assignment sites instead of below them -- FIXED

Handed over by entry 123, which implemented GNU's undo-limit machinery and
hit this at the boundary: `forward::LispIntFwd' was declared but unwired, so
a string could sit in `undo-limit'.  Reproduced before touching anything,
with `-Q --batch' against GNU 31.0.90 and against a verified build of
origin/main.

```elisp
(list (condition-case e (setq undo-limit "x") (error e))
      undo-limit
      (condition-case e (setq gc-cons-threshold 1.5) (error e))
      gc-cons-threshold)
;; GNU                => ((wrong-type-argument integerp "x") 160000
;;                        (wrong-type-argument integerp 1.5) 800000)
;; Neomacs before fix => ("x" "x" 1.5 1.5)
```

Not one spelling of the assignment was checked, including the byte-compiled
one:

```elisp
(mapcar (lambda (f) (condition-case e (funcall f) (error (car e))))
        (list (lambda () (set 'undo-limit "x"))
              (lambda () (set-default 'undo-limit "x"))
              (lambda () (let ((undo-limit "x")) undo-limit))
              (lambda () (with-temp-buffer (setq-local undo-limit "x")))
              (lambda () (funcall (byte-compile (lambda () (setq undo-limit "x")))))))
;; GNU                => (wrong-type-argument wrong-type-argument wrong-type-argument
;;                        wrong-type-argument wrong-type-argument)
;; Neomacs before fix => ("x" "x" "x" "x" "x")
```

The premise held, and measuring the neighbouring forward types turned up two
more holes of the same shape in `Lisp_Fwd_Bool', which Neomacs HAD wired --
for exactly one variable, `inhibit-message'.  A byte-compiled `setq'
disagreed about what the variable then held:

```elisp
(setq inhibit-message nil)
(list (setq inhibit-message 5)
      inhibit-message
      (funcall (byte-compile (lambda () (setq inhibit-message 4) inhibit-message))))
;; GNU                => (5 t t)
;; Neomacs before fix => (5 t 4)
```

That third element is NOT the VM's `varset' skipping the forwarder, which is
what it looks like and what this entry first assumed.  Disassembling both
shows the divergence is decided at compile time: GNU compiles the body to
`constant 4; constant t; return' while Neomacs compiles it to
`constant 4; dup; return'.  GNU's optimizer folds a `varset X; varref X' pair
into the stored value only after checking `byte-boolean-vars', "because what
we put in might not be what we get out"
(`lisp/emacs-lisp/byte-opt.el:2285-2300'), and substitutes `t' when the
variable is on that list.  `byte-boolean-vars' is a `DEFVAR_LISP' in
`src/lread.c:5772' that `defvar_bool' itself conses onto
(`src/lread.c:5254-5262'), so it is the Boolean forward type's rule reaching
the compiler.  Measured: GNU lists 117 symbols there, Neomacs listed 0 --
`define_bool_variable' installed the descriptor and stopped.  The `setq'
itself was already storing `t'; only the compiled function's own return value
was the raw 4.

The second hole is the one the VM really did have a hand in: the first
`make-local-variable' anywhere disarmed the forwarder for the whole session,
in every buffer:

```elisp
(setq inhibit-message nil)
(let ((b (generate-new-buffer "fwd")))
  (with-current-buffer b (setq-local inhibit-message 3))
  (kill-buffer b)
  (setq inhibit-message 7)
  inhibit-message)
;; GNU                => t
;; Neomacs before fix => 7
```

Two more states GNU's C slot cannot represent were reachable here -- an
integer past `intmax_t', and unbound:

```elisp
(list (condition-case e (setq gc-cons-threshold (expt 2 200)) (error (car e)))
      gc-cons-threshold
      (condition-case e (makunbound 'gc-cons-threshold) (error e)))
;; GNU                => (overflow-error 800000
;;                        (error "Built-in variable may not be unbound : gc-cons-threshold"))
;; Neomacs before fix => (1606938044258990275541962092341162602522202993782792835301376
;;                        1606938044258990275541962092341162602522202993782792835301376
;;                        gc-cons-threshold)
```

### Which forward types enforce what, and it does not generalise

The whole switch is `store_symval_forwarding' (`src/data.c:1469-1530'), and
the five arms disagree with each other:

- `Lisp_Fwd_Int' (`data.c:1475-1483') runs `CHECK_INTEGER (newval)' and then
  `integer_to_intmax', so a non-integer is `(wrong-type-argument integerp
  VAL)' and an integer too large for the slot is `(overflow-error VAL)' --
  two different signals, and the second one is not a type error.  Negative
  values are fine; a bignum is fine as long as it fits `intmax_t'.  Measured
  under GNU: `(setq gc-cons-threshold (* most-positive-fixnum 4))' => 9223372036854775804.
- `Lisp_Fwd_Bool' (`data.c:1485-1487') is `*XBOOLVAR (valcontents) = !NILP
  (newval);' -- it never signals, it COERCES.  The integer rule does not
  generalise to it at all.  `setq' still returns the value it was handed, so
  the coercion is only visible on the next read: measured under GNU,
  `(setq visible-bell 5)' => 5 and `visible-bell' => t.
- `Lisp_Fwd_Obj' (`data.c:1489-1516') checks nothing; its body is entirely
  about propagating a `buffer_defaults' slot into buffers with no local
  value.
- `Lisp_Fwd_Buffer_Obj' (`data.c:1518-1526') checks the slot's closed
  predicate -- and only for a non-nil value, so `nil' passes an `integerp'
  slot.  This is the one arm Neomacs already had.
- `Lisp_Fwd_Kboard_Obj' (`data.c:1529-1536') checks nothing.

The read side is the mirror-image function `do_symval_forwarding'
(`data.c:1337-1360'): `Lisp_Fwd_Int' rebuilds a Lisp integer with `make_int',
`Lisp_Fwd_Bool' rebuilds `t' or `nil'.  That round trip is why a Boolean slot
given 5 reads back `t' -- GNU does not canonicalise on the way in, it simply
has nowhere to keep the 5.

`let' is not a separate rule: `specbind' records the binding kind and
`do_specbind' calls `set_internal' for every forwarded symbol
(`src/eval.c:3594-3622'), so `(let ((undo-limit "x")) ...)' signals before
the body runs.  The single exception is a per-buffer slot with no local
value in the current buffer, which `do_specbind' routes to
`set_default_internal' instead; that path writes `set_per_buffer_default'
directly (`data.c:2080-2113') and therefore skips the predicate.
`set-default' behaves the same way: per-buffer slot, no check; anything else
forwarded, `set_internal' and the full check (`data.c:2077', `data.c:2123').
And `makunbound' is refused outright for both a forwarded symbol and a
localized one that still carries a forwarder (`data.c:1805-1807',
`data.c:1725-1728') -- "Built-in variable may not be unbound".

### Why GNU's code is shaped this way

`store_symval_forwarding' is `static' and is called from five places, all
inside `data.c': `set_internal' twice (`1794', `1825'),
`set_default_internal' (`2077'), `swap_in_global_binding' (`1559') and
`swap_in_symval_forwarding' (`1603').  Nothing else can reach a forwarded
slot.  Underneath that, the C declaration does the real
work: `DEFVAR_INT' (`src/lisp.h:3513-3518') binds the symbol to an
`intmax_t *', `DEFVAR_BOOL' (`lisp.h:3507-3512') to a `bool *', so "a string
in `undo-limit'" is not a state the program can be in -- not a state it
checks for and rejects.  The check exists only to decide which signal to
raise on the way to a store that could not have been misspelled anyway.

Neomacs had the opposite arrangement.  `undo-limit' and 42 other GNU
`DEFVAR_INT' variables were ordinary `Plainval' obarray cells holding a
`Value', so the invariant could only be an opt-in check, and it was written
out by hand at the sites that happened to remember: `Lisp_Fwd_Bool''s
`!NILP' coercion lived in one arm of `set_symbol_value_id_inner' and its
matching read in four more arms of `symbol.rs', and nowhere else -- which is
why `make-local-variable', which moves the symbol to a different cell
entirely, dropped it, and why the per-buffer predicate had to be re-asked for
at `set_runtime_binding', `try_specbind' and the VM's `varset' separately.
This project has a name for that failure: an opt-in invariant each branch
must remember.

### The type-level fix

`LispIntFwd' now owns its value, and the only way to put one there is
`LispIntFwd::set(LispInteger)'.  `LispInteger''s single checking constructor
is GNU's `CHECK_INTEGER' + `integer_to_intmax' pair, so the descriptor is as
unable to hold a string as GNU's `intmax_t' is.  Above that,
`LispFwd::store' is GNU's switch, once, and it is the only producer of a
`ForwardStore'; `store_runtime_binding' -- the function that decides which
cell an assignment lands in -- takes a `ForwardChecked' and nothing else, so
a new assignment path cannot be added without the rule running.  The four
hand-written `Bool' read arms in `symbol.rs' collapsed into `LispFwd::load' /
`LispFwd::load_ref', and the scattered per-buffer predicate calls in
`set_runtime_binding', `try_specbind' and the VM's `varset' into the one
`check_forwarded_store'.

Three states that had no GNU counterpart are now unrepresentable rather than
merely unreached:

- a non-integer in a `DEFVAR_INT' slot, because the slot's setter does not
  take a `Value';
- an assignment that reaches forwarded storage unchecked, because
  `ForwardChecked' has one constructor;
- a forwarder silently dropped by `make-local-variable', because
  `make_symbol_localized' now copies the descriptor into the BLV's `fwd'
  field the way GNU's `make_blv' does, and `assignment_forwarder' resolves
  the rule through either redirect.

`ForwardStoreSite::{Set, Bind, SetDefault}' names the one distinction GNU
actually makes -- `set_internal' vs `do_specbind' vs `set_default_internal'
-- instead of leaving "does the per-buffer predicate apply here?" to be
re-derived at each site; it had already been derived two different ways.

`define_bool_variable' now conses the symbol onto `byte-boolean-vars' the way
`defvar_bool' does, so the registration function -- not its callers -- is what
tells the byte optimizer about the coercion.

43 variables changed storage (`neovm-core/src/emacs_core/symbol.rs',
`define_int_variable').  `undo-limit''s `GnuIntVariable::NotAnIntSlotValue',
which entry 123 added to name exactly this gap, is now unreachable for the
forwarded variables and survives only for `undo-outer-limit', which really is
a `DEFVAR_LISP' that may hold nil.  The pdump format goes to v57 to carry an
integer forwarder's value, matching what v56 did for the Boolean one.

### Found and NOT fixed

- Neomacs binds four variables GNU leaves unbound on this platform:
  `dos-hyper-key', `dos-keypad-mode', `dos-super-key' (msdos.c) and
  `imagemagick-render-type' (HAVE_IMAGEMAGICK), all `nil'.  Measured under
  GNU 31.0.90 on GNU/Linux: all four `(boundp ...)' => nil.  They are
  invented existence rather than invented values, and cus-start.el's
  platform filter is what stops them from erroring today.
- `display-line-numbers-offset' is `DEFVAR_INT' in GNU (`xdisp.c') but
  `defvar_buffer_local' here, so it was left out of the conversion; making
  it forwarded would change its buffer-local nature as well as its type.
- Eight GNU `DEFVAR_INT' variables have no Neomacs definition at all:
  `command-line-max-length', `large-hscroll-threshold',
  `long-line-optimizations-bol-search-limit',
  `long-line-optimizations-region-size', `max-redisplay-ticks',
  `strings-consed', `x-color-cache-bucket-size' and
  `x-mouse-click-focus-ignore-time'.
- The six allocation counters (`cons-cells-consed' and friends) are
  forwarded and typed now, but still read 0 -- nothing increments them, so
  they are the only remaining VALUE differences in the 43-variable table
  apart from `gcs-done' (1 here against GNU's 0 in the same `--batch' run).
- `baud-rate' is seeded to 38400 in Rust; GNU computes it in `init_baud_rate'
  (`src/sysdep.c') at terminal init and reports 0 under `--batch'.  38400 is
  the value a real pty gets, so the seed is right for a session and wrong for
  batch -- an invented default of the recurring kind, left alone here because
  fixing it means porting `init_baud_rate', not changing a forward type.
- `byte-boolean-vars' now gets an entry from `define_bool_variable', but that
  is one entry against GNU's 117: the other 116 GNU `DEFVAR_BOOL' variables
  are still ordinary obarray cells here, so the byte optimizer will keep
  folding their `varset'/`varref' pairs.  Wiring `Lisp_Fwd_Bool' for all of
  them is the same shape of work this entry did for `Lisp_Fwd_Int' and is
  deliberately not attempted here.
- `Lisp_Fwd_Obj' and `Lisp_Fwd_Kboard_Obj' remain unwired.  Both enforce
  nothing on assignment, so no divergence follows from that; `DEFVAR_LISP'
  variables stay ordinary obarray cells, which is observationally identical.

Status: FIXED.

## 133. The `rg` results-buffer pin was decided by where the kernel split a PTY read, so ripgrep's line buffering -- not either editor -- chose its value -- NOT A DIVERGENCE (racy harness fixture), FIXED

`parity_tests::rg::rg_package_batch` failed in roughly one full-suite run in
two and passed every time in isolation.  The failure named ONE editor:

```
snapshot mismatches: a_search_populates_the_results_buffer_and_navigates_files (Neomacs)
```

In this suite a mismatch naming both editors is a stale or path-dependent pin
(entries 127 and 129); a mismatch naming one is that editor really producing
different bytes.  It did.  The difference was one newline after the first
`File:` heading, and everything after it shifted:

```elisp
;; GNU / pin => "...\n\nFile: ./a.txt\n   1   1 widget one\n   3   3 a widget two\n..."
;; Neomacs   => "...\n\nFile: ./a.txt\n\n   1   1 widget one\n   3   3 a widget two\n..."
;;              :point-offset 272 -> 273, navigation lines 2 -> 3, 2 -> 3, 0 -> 1
```

The one-editor reading was right about the bytes and wrong about the cause.
Under load GNU Emacs 31.0.90 produces that same extra newline, at a HIGHER
rate than Neomacs.  Whichever editor loses the race on a given run is the one
the batch names.

### rg.el inserts a newline per filter call, not per search

`rg-filter' runs from `compilation-filter-hook' and opens with

```elisp
(goto-char compilation-filter-start)
(forward-line 0)
(setq beg (point))
(when (zerop rg-hit-count)
  (newline))
;; Only operate on whole lines so we don't get caught with part of an
;; escape sequence in one chunk and the rest in another.
(when (< (point) end)
  ...)
```

(`rg-result.el:451-458` in the pinned 20260517.1310 build; `rg-hit-count' is
bumped only where a match escape is rewritten, at `:482`).  The `newline' at
`:454-455` sits OUTSIDE the whole-lines guard at `:458`, so it fires on every
filter invocation until
the first COMPLETE match line has been seen -- including an invocation whose
chunk carries no complete line at all.  One invocation before the first match
gives the pinned blank line between the command line and the first heading.
Two give a second blank line.  Where the second one lands depends on where the
chunk boundary fell:

```
;; chunk lengths     ;; result
;;   (205)           pinned:      "...\n\nFile: ./a.txt\n   1   1 widget one"
;;   (73 132)        heading gap: "...\n\nFile: ./a.txt\n\n   1   1 widget one"
;;   (21 184)        heading gap: same as above
;;   (20 185)        command gap: "...\n\n\nFile: ./a.txt\n   1   1 widget one"
```

73 is `20` (the escape-wrapped `./a.txt`) + `1` (its newline) + `52` (the first
match line WITHOUT its newline): the heading is a complete line, the match line
is not, so the hit count is still zero when the second chunk arrives and its
`beg` is the start of the match row.  That is the reported failure exactly.

Package-free reduction of the rule -- the same child text delivered in one
write and in two, through a filter carrying rg-filter's shape:

```elisp
(let ((probe
       (lambda (command)
         (let* ((hits 0)
                (buffer (generate-new-buffer "*probe*"))
                (proc (start-process "probe" buffer "sh" "-c" command)))
           (set-process-sentinel proc #'ignore)
           (set-process-filter
            proc
            (lambda (p string)
              (with-current-buffer (process-buffer p)
                (let ((start (point-max)))
                  (goto-char start)
                  (insert string)
                  (save-excursion
                    (goto-char start)
                    (forward-line 0)
                    (when (zerop hits) (newline)))
                  (when (string-match-p "MATCH" string) (setq hits 1))))))
           (while (accept-process-output proc 0.5))
           (prog1 (with-current-buffer buffer (buffer-string))
             (kill-buffer buffer))))))
  (prin1 (list :one-write (funcall probe "printf 'HEAD\\nMATCH\\n'")
               :two-writes
               (funcall probe "printf 'HEAD\\n'; sleep 0.2; printf 'MATCH\\n'"))))
```

```elisp
;; GNU     => (:one-write "\nHEAD\nMATCH\n" :two-writes "\nHEAD\n\nMATCH\n")
;; Neomacs => (:one-write "\nHEAD\nMATCH\n" :two-writes "\nHEAD\n\nMATCH\n")
```

Byte-identical.  Nothing about which editor is running changes the answer;
only how many times the filter was called does.

### The chunk boundary is ripgrep's line buffering, and it only exists because the search runs on a PTY

`compilation-start' spawns through `start-file-process-shell-command'
(GNU lisp/progmodes/compile.el:2190), which is `start-process', which takes its
device from `process-connection-type' -- default `t', i.e. a pty
(GNU src/process.c:8923-8929, consulted by `is_pty_from_symbol' at
src/process.c:1345-1354).  GNU defaults it to a pty on purpose: interactive
compilations want job control and terminal-aware children, and a pty is the
only way a child can be told it is talking to a terminal.

ripgrep reads exactly that signal.  Its output for this fixture is 205 bytes,
and the number of `write(2)` calls it takes to emit them depends entirely on
whether stdout is a terminal:

```
$ strace -e trace=write rg ... > pipe          # 1 write
write(1, "\33[0m\33[35m./a.txt\33[0m\n\33[0m\33[32m1\33"..., 205) = 205

$ script -q -c 'strace -e trace=write rg ...'  # 11 writes
write(1, "\33[0m\33[35m./a.txt\33[0m", 20) = 20
write(1, "\n", 1)                       = 1
write(1, "\33[0m\33[32m1\33[0m:\33[0m1\33[0m:\33[0m\33[1"..., 52) = 52
write(1, "\n", 1)                       = 1
...
```

Eleven writes give ten places for a reader to land between, and nothing in
either editor decides which one it hits: `read_process_output' issues one
`emacs_read' of up to `read-process-output-max' bytes per call
(GNU src/process.c:6229-6282) and gets whatever the tty layer has buffered at
that instant.  Both editors read 65536 at a time and both drain what is there.

### Measured, with both editors running at once

`tmp/abc-rgreal-probe.el` runs the fixture's real workflow -- `rg-run' through
`compilation-start' on the pinned rg build -- and records the chunk lengths
`compilation-filter' was handed, how many times `rg-filter' ran its `newline',
and whether the results buffer ends up with the extra blank line.  GNU Emacs
31.0.90, the current Neomacs build, and the pre-entry-128 reference binary were
run SIMULTANEOUSLY, 8 workers each, against 16 CPU burners on a 32-thread box:

```
;; PTY (the fixture as committed)
;; GNU Emacs 31.0.90 => 11 diverged / 920 runs  (6x (73 132), 3x (21 184), 2x other)
;; Neomacs (current) =>  7 diverged / 920 runs  (4x (73 132), 3x other)
;; Neomacs pre-128   =>  1 diverged / 720 runs  (1x (20 185))
```

GNU also split its reads far more often than Neomacs did -- 60 split reads per
400 runs against Neomacs's 12 in the three-way round -- it simply survived more
of those splits by landing past the first match line.  The failure is not
Neomacs's, and it does not belong to entry 128 either: the reference binary
built before that wave, swapped in and run under the same load, produced the
same divergence.

### The fix removes the choice instead of surviving it

A pipe is not a workaround here, it is the only topology in which the pinned
value exists.  ripgrep block-buffers to a non-terminal, so the fixture's whole
result leaves the child in ONE 205-byte write, which is below `PIPE_BUF` and
therefore reaches the reader whole: one write, one chunk, one filter call, one
newline, in any editor and under any scheduling.

Every search this suite starts now goes through one prelude helper, and the
helper is the guard:

```elisp
(defun rg-test-run (pattern root)
  (let ((process-connection-type nil))
    (rg-run pattern "everything" root nil nil (list "--sort" "path" ".")))
  (let* ((buffer (get-buffer (rg-buffer-name)))
         (process (and buffer (get-buffer-process buffer)))
         (tty (and process (process-tty-name process))))
    (when tty
      (error "rg-test-run: search is PTY-connected (%s); its output would \
arrive in scheduling-dependent chunks" tty))
    buffer))
```

There is no longer an expression in this suite that reaches an `rg-mode'
buffer without having gone through it, so "pin a results buffer fed by a
line-buffering child" is not reachable from a case.  A future edit that
restores the pty fails loudly at the `error' -- in both editors, on the first
run -- instead of intermittently in a snapshot months later.  `process-tty-name'
was verified to answer `"/dev/pts/N"` for a pty and `nil` for a pipe,
identically in GNU Emacs and Neomacs, so the guard cannot itself become a
divergence.

The pinned values did not move: the same `:content`, `:point-offset 272`,
`:first-match-faces` and `:file-tags` that the pty produced on a good run are
what the pipe produces on every run.  That is the point -- the pin was always
recording the single-chunk answer, it just had no way to insist on it.

Evidence for the fix, same harness, same load, three binaries at once:

```
;; pipe (the fixture as fixed), 1840 runs total
;; GNU Emacs 31.0.90 =>   0 diverged, 0 split reads / 720 runs
;; Neomacs (current) =>   0 diverged, 0 split reads / 720 runs
;; Neomacs pre-128   =>   0 diverged, 0 split reads / 400 runs
```

Not one read was split in any of them: every run delivered the whole 205
bytes in a single filter call.

`cargo nextest run -p neomacs-melpa-tests --release -E 'test(rg_package_batch)'`
passed 14/14 consecutive runs, 12 of them with CPU burners applied.

### The rule, and what else was checked

An upstream `compilation-filter-hook' function that mutates the buffer OUTSIDE
its own whole-lines guard makes the rendered buffer a function of read
boundaries, and read boundaries are not a parity signal.  Before pinning a
compilation-derived buffer fed by a real external tool, read the package's
filter for writes that are not under the `(when (< (point) end) ...)' guard.

`ag.el` 20201031.2202, the closest sibling suite, was read for the same shape
and is clean: `ag-filter' (`ag.el:632-664`) performs every one of its
substitutions inside the guard and its escape stripping is idempotent across
chunk boundaries, so the `ag` suite's rendered-buffer pins are chunk-independent
even though its stand-in binary is a shell script that `printf`s line by line.


Status: FIXED (harness defect).  The underlying `rg-filter' behaviour is
upstream rg.el's and is unchanged; it is reproduced identically by GNU Emacs
and Neomacs, so there is nothing to fix in the engine.

## 134. An undecided end-of-line type meant "leave it alone" where GNU means "detect it", so every file, string and subprocess read as `raw-text`, `undecided` or `utf-8` kept its CR LF -- FIXED

Handed over by entry 131, which met this while fixing the asynchronous
subprocess decoder and scoped it out as its own entry.  Reproduced first, with
`-Q --batch` against GNU Emacs 31.0.90 and against a verified build of
origin/main (`tmp/refbin/neomacs`), before anything was touched.

```elisp
(mapcar (lambda (cs) (append (decode-coding-string "a\r\nb\r\n" cs) nil))
        '(raw-text undecided utf-8 latin-1 binary no-conversion))
;; GNU                => ((97 10 98 10) (97 10 98 10) (97 10 98 10) (97 10 98 10)
;;                        (97 13 10 98 13 10) (97 13 10 98 13 10))
;; Neomacs before fix => ((97 13 10 98 13 10) (97 13 10 98 13 10) (97 13 10 98 13 10)
;;                        (97 13 10 98 13 10) (97 13 10 98 13 10) (97 13 10 98 13 10))
```

The first four rows are the divergence.  The last two are the control, and they
matter as much: `binary` and `no-conversion` really do copy a CR through, so a
fix that makes them convert is as wrong as the bug it replaces.

### End of line is a SECOND axis, and its third state is a verb

A GNU coding system carries `eol_type` in slot 2 of its spec, independently of
the character code, and that slot has three concrete values -- `Qunix`, `Qdos`,
`Qmac` -- plus a fourth state that is not a value at all: a **VECTOR** of the
three subsidiaries.  `setup_coding_system` reads the vector as a demand for work
rather than as an absence of it:

```c
  if (VECTORP (eol_type))
    coding->common_flags = (CODING_REQUIRE_DECODING_MASK
			    | CODING_REQUIRE_DETECTION_MASK);   /* src/coding.c:5685-5687 */
```

The two directions then resolve it in opposite ways, and both resolutions happen
BEFORE any byte is touched.  Encoding forces unix -- `consume_chars`
(src/coding.c:7623-7625) and `encode_coding_iso_2022` (src/coding.c:4384-4386)
open with the same two lines:

```c
  eol_type = inhibit_eol_conversion ? Qunix : CODING_ID_EOL_TYPE (coding->id);
  if (VECTORP (eol_type))
    eol_type = Qunix;
```

Decoding DETECTS.  `decode_coding` runs `decode_eol` once, outside every
`decode_coding_*` (src/coding.c:7478-7480), and `decode_eol`'s first act is to
replace a vector with a concrete type by scanning the text the decoder has just
produced (src/coding.c:6783-6806), then to hand that concrete type to
`adjust_coding_eol_type`.  So "undecided" on the decode side is an instruction,
not a gap -- and Neomacs's `EolType::Undecided` was implemented as a gap:

```rust
    match eol {
        EolType::Dos => { /* collapse CR LF */ }
        EolType::Mac => { /* CR -> LF */ }
        EolType::Unix | EolType::Undecided => bytes.to_vec(),
    }
```

`Undecided` sitting in the same arm as `Unix` is the whole bug, in one line.

### Four questions the fix had to answer by measurement

**Which coding systems convert, and which are genuinely byte-preserving.**  Not
the ones whose names suggest it.  Listing `coding-system-eol-type` over all 269
systems GNU 31.0.90 defines: exactly three come back concrete while their name
carries no `-unix`/`-dos`/`-mac` suffix -- `binary`, `no-conversion` and
`no-conversion-multibyte` -- and no suffixed system comes back a vector.  So
`raw-text` DOES convert end of line (it drops the character half only), and
`binary` does not, and the two differ on exactly this axis:

```elisp
(list (append (decode-coding-string "a\rb" 'raw-text) nil)
      (append (decode-coding-string "a\rb" 'binary) nil))
;; GNU => ((97 10 98) (97 13 98))
```

**What detection reads, and what it does with mixed line endings.**  It reads
the DECODED text -- not the source bytes, because in UTF-16 a CR is the byte
pair `0D 00` -- and it reads ALL of it, ORing one flag per terminator.  Then it
folds: CR LF together with a stray CR and no LF is a DOS text with stray ^Ms
(src/coding.c:6798-6801); ANY other mixture is unix (src/coding.c:6800-6804).
Measured, every row by running it:

| input, decoded as `undecided` | GNU text | GNU `last-coding-system-used` |
|---|---|---|
| `a\nb\r\nc\n` | unchanged | `undecided-unix` |
| `a\r\nb\nc\r\n` | unchanged | `undecided-unix` |
| `a\rb\rc\r` | `(97 10 98 10 99 10)` | `undecided-mac` |
| `a\rb\r\nc\r\n` | `(97 13 98 10 99 10)` | `undecided-dos` |
| `a\rb\nc\n` | unchanged | `undecided-unix` |
| `a\r\nb\r\nc\r\nd\r\ne\nf` | unchanged | `undecided-unix` |
| `a\r\nb\r\nc\rd` | `(97 10 98 10 99 13 100)` | `undecided-dos` |
| `abc` | unchanged | `undecided` |

The sixth row is the one worth staring at.  Four CR LFs and then one bare LF,
and GNU converts NOTHING.  There is a second EOL function in `coding.c` that
would have answered `dos` for it -- `detect_eol` (src/coding.c:6376), which
stops after `MAX_EOL_CHECK_COUNT` (3) terminators and settles a disagreement the
moment it meets one -- but that function serves `Fdetect_coding_region`
(src/coding.c:8944) and never runs on a decode.  The two really do disagree, in
GNU, on the same input:

```elisp
(list (detect-coding-string "a\r\nb\r\nc\r\nd\r\ne\nf")
      (append (decode-coding-string "a\r\nb\r\nc\r\nd\r\ne\nf" 'undecided) nil))
;; GNU => ((undecided-dos) (97 13 10 98 13 10 99 13 10 100 13 10 101 10 102))
```

**Whether a lone CR converts.**  Yes, when CR is the only kind of terminator in
the text (row 3 above); no, when the text also contains an LF (rows 1, 5); and
in the CR-plus-CRLF case the text is treated as DOS, which collapses the pairs
and leaves the lone CR alone (rows 4 and 7).  A coding system whose eol_type is
already `Qunix` never converts one at all.

**Whether the three consumers share the code.**  Strings and processes did.
Files did NOT: `insert-file-contents` had its own EOL detector,
`DetectedFileEol` in `neovm-core/src/emacs_core/fileio.rs`, which scanned the
RAW bytes with `detect_eol`'s three-terminator window and attached the answer to
the coding-system NAME before calling the shared decoder.  That is a faithful
port of the wrong function, and it is measurably wrong:

```elisp
;; a file holding  a CR LF b CR LF c CR LF d LF
(with-temp-buffer (insert-file-contents f)
  (list (append (buffer-string) nil) buffer-file-coding-system))
;; GNU                => ((97 13 10 98 13 10 99 13 10 100 10) undecided-unix)
;; Neomacs before fix => ((97 10 98 10 99 10 100 10) undecided-dos)
```

### `last-coding-system-used` moves with the text, because it is the same function

`adjust_coding_eol_type` (src/coding.c:6471-6497) does two things in one call:
it picks the eol type the conversion runs with, and it rewrites `coding->id` to
the subsidiary that carries it.  `code_convert_region` then reports that id
(`Vlast_coding_system_used = CODING_ID_NAME (coding.id)`, src/coding.c:9497), so
a detected end of line is observable by NAME as well as by text -- and through
an ALIAS it is observable as the canonical subsidiary, because the vector in the
alias's shared spec holds canonical names:

```elisp
(list (append (decode-coding-string "a\r\nb\r\n" 'latin-1) nil) last-coding-system-used)
;; GNU                => ((97 10 98 10) iso-latin-1-dos)
;; Neomacs before fix => ((97 13 10 98 13 10) latin-1)
```

Neomacs reported the argument's own name for every case, and
`decode-coding-region` went further: it OVERWROTE whatever the conversion had
recorded with the argument's name, in all three of its destination branches.

There is one exception, and it is not a special case for end of line -- it is
`code_convert_string`'s identity fast path (src/coding.c:9609-9628), which
returns the string without running the conversion at all when the coding is
ASCII-compatible, the text is pure ASCII, and no EOL work is owed:

```c
      if (! NILP (CODING_ATTR_ASCII_COMPAT (attrs))
          && (STRING_MULTIBYTE (string) ? (chars == bytes) : string_ascii_p (string))
          && (EQ (CODING_ID_EOL_TYPE (coding.id), Qunix)
              || inhibit_eol_conversion
              || ! memchr (SDATA (string), encodep ? '\n' : '\r', bytes)))
        {
          if (! norecord)
            Vlast_coding_system_used = coding_system;
          return (nocopy ? string
                  : (encodep ? make_unibyte_string (SSDATA (string), bytes)
                             : make_multibyte_string (SSDATA (string), bytes, bytes)));
        }
```

It is guarded by `EQ (dst_object, Qt)` (:9608), so it is available to
`decode-coding-string` returning a string and to nothing else.
`code_convert_region` has no such path, and `insert-file-contents` reaches
`decode_coding_object` directly.  The same ASCII text therefore answers two
different names depending on which door it came in, and that is measurable too:

```elisp
;; the text "a\nb\n", decoded as `undecided'
;; (decode-coding-string ...)                       => last-coding-system-used  undecided
;; (decode-coding-string ... nil (current-buffer))  => last-coding-system-used  undecided-unix
;; (decode-coding-region ...)                       => last-coding-system-used  undecided-unix
;; (insert-file-contents ...)                       => last-coding-system-used  undecided-unix
```

The fast path also decides the RESULT'S multibyteness, which is how `binary`
came into it: its eol_type IS `Qunix`, so a CR does not block the path, and GNU
hands back a multibyte string where Neomacs built a unibyte one.

```elisp
(let ((d (decode-coding-string "a\r\nb\r\n" 'binary))) (list (append d nil) (multibyte-string-p d)))
;; GNU                => ((97 13 10 98 13 10) t)
;; Neomacs before fix => ((97 13 10 98 13 10) nil)
```

Neomacs's own fast path had the ASCII and the newline test but not the
`EQ (eol_type, Qunix)` escape, because until now nothing here knew that
`binary`'s eol type was concrete.  It also read the eol type of the coding's
BASE, so `raw-text-unix` -- whose own type is `Qunix` while `raw-text`'s is a
vector -- was refused the path and came back unibyte where GNU returns multibyte.

### The type-level fix

`EolType` keeps its four states, because it models GNU's slot and the vector is
one of them.  What changes is that the four-state type can no longer reach a
byte transformation.  `ResolvedEol` (`neovm-core/src/emacs_core/coding.rs`) has
GNU's three concrete answers and NO undecided variant, and `decode_eol_bytes`
now takes one:

```rust
pub(crate) enum ResolvedEol { Unix, Dos, Mac }

impl EolType {
    pub(crate) fn for_encode(self) -> ResolvedEol;                 // VECTOR -> Qunix
    pub(crate) fn for_decode(self, decoded: &[u8]) -> ResolvedEol; // VECTOR -> detect
}
```

The two resolvers are GNU's two, spelled as the only two ways to get a
`ResolvedEol` out of an `EolType`.  Reading the vector as "nothing to do" is not
a mistake that can be made again: there is no arm to write it in, and a caller
that has not chosen `for_encode` or `for_decode` has no value to pass.
`for_decode` takes the DECODED bytes as an argument for the same reason -- GNU
scans the produced text, and a signature that accepted the source could not.

`detected_decoded_eol` is the VECTOR branch of `decode_eol`, whole-text bitmask
and both fold rules, and `DecodeEolSeen` names the fold's RESULT rather than the
raw mask, so the mixture states cannot leak out of it into a conversion.  It
sits next to the existing `detect_eol_seen` port -- which keeps serving
`detect-coding-string`, and whose doc comment now says which of GNU's two
functions it is and why the answers differ.

`CodingEntry` (`neovm-core/src/encoding.rs`) is GNU's `dst_object == Qt` guard
promoted to a parameter: `CodeConvertString` for the two string builtins,
`CodingObject` for the region path and for file I/O, which reach
`encode_coding_object` / `decode_coding_object` directly.  The identity fast
path is available only to the first, and the reporting difference above falls
out of it instead of being a rule each call site remembers.

Three duplicate decisions are gone with it.  `DetectedFileEol` is deleted --
`insert-file-contents` now hands the coding-system name to the shared decoder
and lets `decode_eol` do what it does for strings and processes.
`builtin_coding_region` no longer re-writes `last-coding-system-used`; the
conversion below it already recorded GNU's answer.  And
`ProcessOutputDecoding::Bytes` -- which entry 131 deliberately left conflating
`raw-text` with `binary`/`no-conversion` so that the conflation would become
wrong the moment detection landed -- now covers only the two codings that
convert NOTHING, character code and end of line alike; `raw-text` goes through
the shared decoder like any other name.  The encode side keeps `raw-text` in its
own convert-nothing set, and says why: encoders never detect.

One boundary had to move with it.  A subprocess read that ends on a CR can no
longer be decoded on its own, because the coding system it belongs to may still
resolve to dos; the trailing-CR carryover that only `-dos` codings used now
covers every undecided-eol coding as well.

### Two pinned expectations moved, and both were re-derived by running GNU

`decode_insert_file_contents_defaults_to_gnu_ascii_undecided_codings` pinned
`b"a\r\nb\r\nc\r\nd\n"` as `undecided-dos` under the name
`only_first_three_eols_are_evidence`.  It was recording Neomacs's own answer:
GNU reads that file back with every CR intact and `last-coding-system-used`
`undecided-unix`.  The case is kept, renamed for what it actually shows, and now
asserts the TEXT as well as the name.

`decode_insert_file_contents_accepts_chinese_gb2312_coding` pinned
`chinese-iso-8bit-unix` for `coding-system-for-read` `cn-gb-2312-unix`.  GNU
answers `cn-gb-2312-unix` -- an alias whose eol type is already concrete is
reported verbatim, because `Fdefine_coding_system_alias` puts the alias in the
coding-system hash table as a key of its own and `adjust_coding_eol_type`
returns "Already adjusted" without rewriting the id (src/coding.c:6477-6479).
`chinese-iso-8bit-unix` is what `buffer-file-coding-system` holds, which Lisp
canonicalises separately; the field this test reads is
`last-coding-system-used`.  Measured both, in one run, under GNU 31.0.90.

No other pinned expectation moved.  The blast radius was measured rather than
assumed, because a core primitive with three consumers is exactly where it can
be large: `cargo nextest run -p neovm-oracle-tests` is 38765/38765 green after
the change, unchanged from before it, and `neovm-core` is 9002/9002.  The MELPA
suite is 921/928 with 7 red; running the seven against a verified build of
origin/main (`tmp/refbin/neomacs`, via `NEOMACS_BIN`) reproduces three of them
-- `auto_save_buffers_enhanced`, `jedi` and `quelpa` are red on both builds --
and the other four pass on both when run on their own.  Two of those four
(`closql`, `org_roam`) failed the full run on an external `make all` that raced
another suite building the SAME `sqlite3-api.so`, and the remaining two are the
comint-echo and TUI timing shapes ledger 133 already characterised.  Zero
regressions.

Against the rebuilt release binary (`cargo xtask fresh-build --release`), the
probes in `tmp/pw42/` -- the coding-system matrix, the mixed-line-ending shapes,
the two entry points and their `last-coding-system-used`, the
`insert-file-contents` matrix, the `decode-coding-region` matrix and the
`call-process` matrix -- are byte-identical to GNU Emacs 31.0.90 on every row
except the ones recorded below as not fixed.

### Corrections to earlier entries

Entry 128, dated 2026-08-17: its `coding-system-for-read` = `no-conversion` /
`binary` / `raw-text` row groups three codings that do not behave alike.  For
the payload that entry used they agree, but on the end-of-line axis they do not:
GNU converts CR LF under `raw-text` and copies it under the other two.  Entry
131 already flagged the row as understated; it is now measured, and
`call_process_output_eol_is_detected_for_an_undecided_eol_coding_system` pins
the difference.  Its `utf-8-dos`, unibyte-destination and `latin-1` rows are
unaffected.

Entry 131, dated 2026-08-17: the "End-of-line DETECTION is missing from the
decoder" residual is closed here.  Its `ProcessOutputDecoding::Bytes` prediction
held exactly as written.  Its second residual -- `Vlast_coding_system_used` is
not written back onto the process object -- is NOT closed here and is still
open; this entry fixes what `last-coding-system-used` reports for strings,
regions and files, which is a different slot from `(process-coding-system P)`.

### Found and NOT fixed here

**`inhibit-eol-conversion` is inert, in both directions.**  It appears in every
one of GNU's EOL sites as the first term of the same expression
(`setup_coding_system` src/coding.c:5681, `consume_chars` :7623, `decode_eol`
:6765, the identity fast path :9613), and Neomacs implements none of them.  It
was already inert before this entry -- the `-dos` rows below diverged on
origin/main too -- and it stays inert after:

```elisp
(let ((inhibit-eol-conversion t))
  (list (append (decode-coding-string "a\r\nb\r\n" 'utf-8-dos) nil)
        (append (encode-coding-string "a\nb" 'utf-8-dos) nil)))
;; GNU     => ((97 13 10 98 13 10) (97 10 98))
;; Neomacs => ((97 10 98 10) (97 13 10 98))
```

It is left out deliberately.  Honouring it means reading a dynamic Lisp variable
at every EOL resolution, including the context-free entries (`decode_bytes`,
`encode_lisp_string`) that process output and the file-name decoder reach
without a `Context`; that is a threading change across the whole coding seam,
not a flag.  Doing half of it -- the sites that happen to have a `ctx` -- would
leave the variable working for strings and silently ignored for processes, which
is the shape this ledger keeps having to undo.

**`decode_bytes` runs its EOL pass before the decoder, not after.**  For every
family it reaches that is ASCII-transparent for `0x0D`/`0x0A` -- UTF-8, the
single-byte charsets, Big5 and GBK, whose trail-byte ranges exclude both -- the
two orders agree, which is why nothing measures.  UTF-16 is the family where
they do not, and `decode_bytes` returns before its EOL pass for UTF-16 anyway,
so a UTF-16 process filter still gets no EOL conversion.  `decode-coding-string`
is unaffected: its pass is the outer one, after the decoder, and the UTF-16 rows
of the existing round-trip test cover it.

**Detection on a process read is per chunk, not per process.**  GNU decodes
every run of a subprocess's output through ONE `struct coding_system`, so once
`adjust_coding_eol_type` has fired the choice is sticky for that process; here
each read resolves again.  The carryover above closes the case that would
otherwise misread a split CR LF; what remains is that a later chunk with no
evidence of its own inherits unix rather than the earlier answer.  Making it
sticky means writing the resolved coding back onto the process, which is entry
131's other open residual, so the two belong together.

> **Closed, 2026-08-18, by entry 139.**  "The two belong together" was exactly
> right: the stickiness IS the write-back, and neither could be done without the
> other.  Two details of this paragraph did not hold.  The observable case is
> not "a later chunk with no evidence of its own": a chunk with no terminator
> inherits nothing in GNU either, because `decode_eol` skips
> `adjust_coding_eol_type` for `EOL_SEEN_NONE` and the process is still
> undecided.  What is observable is a later chunk whose OWN evidence disagrees
> -- bare CRs after a DOS start, which GNU keeps and per-chunk detection eats.
> And "the carryover above closes the case that would otherwise misread a split
> CR LF" closes a case GNU does not close: GNU holds a trailing CR back only for
> a concrete `Qdos` eol type (`eol_dos`, src/coding.c:1250-1251), so a split
> CR LF under an undecided coding really is misread there.  The carryover is
> removed in 139.

**`load`'s source-EOL detector is a third copy.**  `source_emacs_coding`
(`neovm-core/src/emacs_core/load.rs`) resolves the end of line itself and always
hands the decoder a concrete `-unix`/`-dos`/`-mac` name, so it is unaffected by
this change.  Its rules are GNU's minus the stray-^M-in-a-DOS-file case, which
no `.el` file in the tree exercises.  It could be deleted the way
`DetectedFileEol` was; it is left because the bootstrap loads through it and the
change has no measurable payoff.

> **Closed, 2026-08-18, by entry 139**, which deleted it -- and the sentence
> "its rules are GNU's minus the stray-^M-in-a-DOS-file case" is WRONG.
> `detect_source_eol`'s `saw_lf` / `saw_crlf` / `saw_lone_cr` cascade answers
> `Dos` when there is a CR LF and no bare LF, which is precisely the mixture
> GNU's stray-^M rule (src/coding.c:6794-6797) answers `EOL_SEEN_CRLF` for.  The
> two detectors agree on every input, so the copy really was a pure duplicate --
> and that is the finding, not a caveat on it.  The equivalence is pinned by
> `load_source_eol_detection_matches_the_shared_decoder`.

**`utf-16` with a BOM still reports the wrong BASE name.**  Decoding a
BOM-prefixed UTF-16 string leaves `last-coding-system-used` at `utf-16-dos` here
and `utf-16be-with-signature-dos` in GNU.  The end-of-line half is now right on
both sides; the divergence is in which concrete system the BOM selects, which is
`detect_coding`'s `coding_category_utf_16_auto` arm (src/coding.c:6724-6742) and
not this axis.

> **Closed, 2026-08-18, by entry 139**, which found the arm's sibling
> (`coding_category_utf_8_auto`, :6702-6722) diverging the same way, and found
> that getting the NAME right required getting the BOM RULE right: the `:bom`
> property has three states, not two, and Neomacs read the bytes where GNU reads
> the coding system.  `utf-16le` keeps a leading U+FEFF that Neomacs stripped,
> and `utf-16-be` refuses a little-endian signature that Neomacs accepted.

## 135. 147 of GNU's 148 `DEFVAR_BOOL` variables did not coerce, and `byte-boolean-vars` held one symbol where GNU holds 117, so the byte optimizer folded the coercion away even for the one that did -- FIXED

Handed over by entry 132, which wired `Lisp_Fwd_Bool` for one variable and
measured that GNU lists 117 symbols on `byte-boolean-vars` where Neomacs
listed one.  Reproduced before touching anything, `-Q --batch`, against GNU
31.0.90 and against a release build of origin/main
(`f24ca50ff`, `cargo xtask fresh-build --release`).

```elisp
(list (length byte-boolean-vars)
      (funcall (byte-compile (lambda () (setq visible-bell 4) visible-bell)))
      (progn (setq visible-bell nil) (list (setq visible-bell 5) visible-bell))
      (let ((inverse-video 3)) inverse-video)
      (progn (set-default 'inverse-video 9) (default-value 'inverse-video))
      (with-temp-buffer (setq-local indent-tabs-mode 4) indent-tabs-mode))
;; GNU                => (117 t (5 t) t t t)
;; Neomacs before fix => (1   4 (5 5) 3 9 4)
```

The forwarder was also still droppable by the first buffer-local binding, for
every variable except `inhibit-message`:

```elisp
(setq print-escape-newlines nil)
(let ((b (generate-new-buffer "fwd")))
  (with-current-buffer b (setq-local print-escape-newlines 3))
  (kill-buffer b)
  (setq print-escape-newlines 7)
  print-escape-newlines)
;; GNU                => t
;; Neomacs before fix => 7
```

### What the set actually is, measured rather than counted from the source

`grep DEFVAR_BOOL src/*.c` finds 184 distinct names, but 36 of them belong to
builds this one is not -- w32, Haiku, Android, `sfntfont`, xwidgets, native
compilation -- and GNU leaves those unbound here too.  Probed name by name
under GNU 31.0.90 on GNU/Linux: **148 are bound**, and every one of them
coerces (`(set-default 'X 5)` then `(default-value 'X)` is `t`).  Of those
148, Neomacs bound 100 and left **48 unbound entirely**.

`(length byte-boolean-vars)` is 117, not 148, and the 31-symbol shortfall is
not a build difference.  `defvar_bool` conses unconditionally
(`src/lread.c:5261`), but `syms_of_lread` declares the list and then writes
`Vbyte_boolean_vars = Qnil;` immediately below it (`src/lread.c:5772-5774`),
which throws away every cons made so far.  `main` calls thirteen `syms_of_*`
functions before `syms_of_lread` (`src/emacs.c:1976-2306`), and
`syms_of_lread` itself declares two `DEFVAR_BOOL`s above the list.  The 31
erased symbols map exactly onto that: `keyboard.c` 14, `coding.c` 6, `fns.c`
3, `fileio.c` 2, `lread.c` 2 (`load-in-progress`, `load-force-doc-strings`),
`xfaces.c`, `data.c`, `alloc.c`, `charset.c` 1 each.  Nothing decides this per
variable; it falls out of the order `main` happens to use.

That accident is observable, and being right about it in the other direction
would be a divergence of its own.  Measured under GNU:

```elisp
(list (funcall (byte-compile (lambda () (setq use-short-answers 4) use-short-answers)))
      use-short-answers)
;; GNU => (4 t)
```

`use-short-answers` is `fns.c`, declared before `syms_of_lread`, so it is not
on the list; the optimizer folds the `varset`/`varref` pair and the compiled
function returns the raw 4 -- while the variable itself holds `t`, because the
slot coerced anyway.  A Neomacs that put all 148 on the list would return `t`
there and disagree with GNU.

### Why GNU's code is shaped this way

`byte-optimize-lapcode` is willing to rewrite `varset X; varref X` into
`dup; varset X`, keeping the stored value on the stack, because for an
ordinary variable the value read back is the value written.  For a
`DEFVAR_BOOL` variable it is not: `store_symval_forwarding`'s `Lisp_Fwd_Bool`
arm is `*XBOOLVAR (valcontents) = !NILP (newval);` (`src/data.c:1485-1487`)
and `do_symval_forwarding` rebuilds `t` or `nil` (`src/data.c:1337-1360`), so
the compiler substitutes `t` for any variable on the list -- "because varset
may change the value" (`lisp/emacs-lisp/byte-opt.el:2285-2300`).  The list is
the only channel the C declaration has into the compiler, which is why
`defvar_bool` maintains it rather than a caller.  Unlike `Lisp_Fwd_Int`, this
arm cannot fail: there is no signal to raise, only a slot with nowhere to keep
the 5.

Neomacs had the coercion written out at one arm of `set_symbol_value_id_inner`
and its reads in four arms of `symbol.rs`, reached by exactly one variable,
and the registration of the other 99 spread across twelve files as ordinary
`set_symbol_value(name, Value::NIL)` pairs -- three of the eval.rs tables were
general-purpose "mark these special and give them a default" lists that
happened to contain them.  Nothing at any of those sites recorded that GNU
declares the variable with `DEFVAR_BOOL`, so nothing could act on it, and the
remaining 48 had no site at all.

### The type-level fix

`neovm-core/src/emacs_core/defvar_bool.rs` is GNU's 148 declarations as one
table: name, initial value as a `bool` (not a `Value` -- a `DEFVAR_BOOL`
variable seeded with a string is not a state either program can be in), and
`ByteBooleanVars::{Listed, ErasedByLreadInit}`.  That enum is the fact GNU
leaves to its startup order, and it is a required field, so a new row cannot
be added without answering it; `define_bool_variable` takes it as a required
argument for the same reason.  Registering the table in order reproduces GNU's
list exactly, order included, because `defvar_bool` prepends and the table is
in declaration order -- `(car byte-boolean-vars)` is `font-use-system-font`
and `(nth 116 byte-boolean-vars)` is `load-dangerous-libraries` in both
editors.

The plain-cell seeds those variables used to have were deleted -- 62 value
seeds, 23 `make_special` companions and 60 entries in three general-purpose
seed tables, across 12 files -- so the table is the only place a `DEFVAR_BOOL`
default is written.  Registration runs FIRST among the bootstrap
registrations, for the reason `main` runs every `syms_of_*` before Lisp:
`Fmake_variable_buffer_local` copies the symbol's forwarder into the BLV
(`src/data.c:2112-2140`), so a variable that gets localized later --
`indent-tabs-mode` (`bindings.el:1048`), `case-symbols-as-words`,
`comment-end-can-be-escaped`, `display-fill-column-indicator`,
`display-line-numbers-widen` -- has to be forwarded before that happens or the
coercion is dropped on the floor.  Everything downstream, including the argv
seed for `noninteractive` and `startup.el`'s `inhibit-x-resources`, now writes
through the forwarder instead of past it.

The dump needed a matching step.  A localized symbol serializes as its default
plus `local_if_set` -- the descriptor is a process-lifetime pointer and cannot
travel -- so those five came back from a pdump with a BLV and no forwarder,
which is a `Localized`-shaped hole entry 132 could not have seen with one
non-buffer-local variable wired.  `reattach_localized_bool_forwarders` rebuilds
them from the value the image did carry, and `define_bool_variable` refuses to
flip an already-localized symbol's redirect back to `Forwarded` at all, so the
orphaned-BLV state is not reachable from either direction.

Two invented defaults fell out of the comparison, both from the eval.rs table
whose comment claims "Default values match GNU's init_*() functions":

- `debugger-may-continue` was `nil`; GNU's is `debugger_may_continue = 1`
  (`src/eval.c:4508`).
- `x-use-underline-position-properties` was `nil`; GNU's is
  `x_use_underline_position_properties = true` (`src/xterm.c:32675`).

Six of the 148 are additionally buffer-local -- five because the `syms_of_*`
that declares them calls `Fmake_variable_buffer_local` on the next line
(`src/xdisp.c:38735`, `38997`, `39015`, `src/syntax.c:3815`,
`src/casefiddle.c:751`) and `indent-tabs-mode` because `bindings.el:1048`
does.  `make-window-start-visible` was one of the 48 Neomacs did not define at
all, so its buffer-locality had to be added with it.

### Measured after

The same probes, `-Q --batch`, against a `cargo xtask fresh-build --release`
binary:

```
                                        GNU        before      after
DEFVAR_BOOL variables bound             148        100         148
default value differs from GNU          --         2           0
byte-boolean-vars membership differs    --         116         0
byte-boolean-vars length                117        1           117
```

`(length byte-boolean-vars)`, `(car byte-boolean-vars)`,
`(nth 116 byte-boolean-vars)` and the whole fold/coercion probe above are now
character-identical between the two editors.

Blast radius, measured on the whole oracle suite rather than assumed: 148
variables changed storage and the coercion started firing where it never had,
so the suite was the gate.  `cargo nextest run -p neovm-oracle-tests`:
38770/38770 green, which is the previous 38765 plus the five pins added here
(`neovm-oracle-tests/src/defvar_bool_byte_boolean_vars.rs`) -- the list's
contents and order, the byte-compiled fold on and off the list, the coercion
through `setq` / `set-default` / `let` / a buffer-local binding / a killed
buffer's binding, GNU's six buffer-local `DEFVAR_BOOL`s, and a sweep asserting
all 147 probeable variables are bound, special and canonical.  No pin moved.
`cargo nextest run -p neovm-core`: 9006/9006, including five new
`forward_test.rs` cases and a table-shape test.  Three unit tests that asserted
a variable's default at the site the declaration moved away from
(`alloc_test.rs`, `indent_test.rs`, `xdisp_test.rs`) had their assertion
deleted rather than relocated, since `every_gnu_defvar_bool_variable_is_bound_
and_reads_back_canonically` now pins all 148 in one place.

### Found and NOT fixed

- `debug-on-next-call` is the one variable of the 148 whose probe still
  disagrees, and not over the forward type.  Measured under GNU,
  `(progn (set-default 'debug-on-next-call 5) (default-value 'debug-on-next-call))`
  => `nil`; here it is `t`.  GNU's answer is `nil` because setting the variable
  non-nil is what arms the debugger, the very next `funcall` enters it, and
  `Fbacktrace_debug`/`debug` clear the flag again -- the slot coerced to `t`
  first and was then reset.  Neomacs has no `debug-on-next-call` mechanism at
  all, so nothing clears it.  That is a debugger gap, pre-existing and now
  merely visible; the variable is excluded from the oracle sweep for the same
  reason GNU's own value is unstable.
- `noninteractive` is seeded `t` in the table where GNU's C default is the
  parsed argv (`noninteractive1 = noninteractive`, `src/emacs.c:1953`).  That
  matches what Neomacs already did, and the binary still overwrites it from
  the command line, but a `Context` built without one reports a batch session.
- The 36 `DEFVAR_BOOL`s belonging to other window systems stay out of the
  table; GNU leaves them unbound here too, so adding them would be invented
  existence of the kind entry 132 recorded for `dos-hyper-key`.
- `Lisp_Fwd_Obj` and `Lisp_Fwd_Kboard_Obj` remain unwired, unchanged from 132:
  neither enforces anything on assignment.

## 136. `undo-boundary` never recorded that it had happened, so `undo-auto--last-boundary-cause` stayed nil -- FIXED

Residual handed back by ledger 122, which fixed `undo-boundary` itself and
noted that GNU also does `Fset (Qundo_auto__last_boundary_cause, Qexplicit)`
(src/undo.c:277) where we left the variable alone.

```elisp
(with-temp-buffer
  (buffer-enable-undo) (insert "x") (undo-boundary)
  undo-auto--last-boundary-cause)
;; GNU                => explicit
;; Neomacs before fix => nil
```

`replace-buffer-contents` reaches the same assignment, because GNU calls
`Fundo_boundary' from `Freplace_buffer_contents' (src/editfns.c:2139) rather
than consing a boundary itself:

```elisp
(replace-buffer-contents src)   ; GNU => explicit, Neomacs before fix => nil
```

### Where the assignment sits is the whole point

It is at :277, which is **after** the early return for a buffer whose
`buffer-undo-list' is `t' (:258-259) and immediately **before** the saved
point/buffer pair (:278-279).  So a buffer that records nothing does not claim a
boundary either -- measured, and already matching before this fix, because
ledger 122 had put that guard in:

```elisp
(with-temp-buffer (setq buffer-undo-list t) (insert "x") (undo-boundary)
                  undo-auto--last-boundary-cause)
;; GNU => nil, Neomacs => nil   (both, before and after)
```

### Two things this needed that a one-line assignment would have got wrong

**The outcome had to become a type.** `BufferManager::add_undo_boundary`
returned `Option<()>`, in which the undo-disabled path and the recorded path
both answered `Some(())` -- there was nothing for a caller to branch on.  The
boundary runs below the obarray and cannot reach a Lisp symbol, so the caller
has to make the assignment, and it can only do that correctly if the boundary
tells it which of GNU's two paths ran.  It now returns
`UndoBoundaryOutcome::{Recorded, UndoDisabled}`.

**The write had to go through `set`, not through a default-value write.**  GNU
calls `Fset`, and this variable is a `defvar-local' in lisp/simple.el, so
wherever a buffer-local binding exists the assignment must land on THAT.  A
first attempt wrote the default and still diverged on exactly the case that
distinguishes them:

```elisp
(with-temp-buffer
  (buffer-enable-undo) (insert "x")
  (setq undo-auto--last-boundary-cause 'something-else)   ; makes it local
  (undo-boundary)
  undo-auto--last-boundary-cause)
;; GNU                    => explicit
;; first attempt at a fix => something-else
```

Delegating to the `set' builtin, as GNU delegates to `Fset', also picks up alias
resolution, the constant check and variable watchers.  This is the same lesson
as ledgers 110 and 124: reimplementing what GNU delegates is how the delegated
behaviour goes missing.

Verified: the six-case probe in tmp/coord-boundary-cause-probe.el is
byte-identical between GNU 31.0.90 and Neomacs.

Status: FIXED.

## 137. `make-pipe-process` and `make-serial-process` never resolved a coding system at all, so one Rust literal answered two chains -- one of which does not exist -- while `make-network-process` resolved correctly and then failed to check the answer -- FIXED

The last of the three entry 131 scoped out and handed on.  `make-process` got
its resolver there; these did not, and the `utf-8-unix` initialiser 131 left
behind as their acknowledged stand-in went on answering for both.

```elisp
;; a serial port, opened on a real character device
(let ((p (make-serial-process :port "/dev/ptmx" :speed 9600 :name "s" :noquery t
                              :buffer (generate-new-buffer " *mb*"))))
  (process-coding-system p))
;; GNU                => (nil . nil)
;; Neomacs before fix => (utf-8-unix . utf-8-unix)
```

`(nil . nil)` is not an omission on GNU's side.  `Fmake_serial_process`'s chain
is the shortest of the five: the `:coding` keyword, then
`coding-system-for-read`/`-write`, and then nothing at all -- `val` is left at
the `Qnil` it was initialised to at src/process.c:3249 and :3263.  A serial
process has no alist step and no `default-process-coding-system` step, so nil is
its normal answer, and nil means DETECT (`setup_coding_system` rewrites it to
`undecided`, src/coding.c:5675-5676).  The stand-in was not a near-miss for that;
it was the opposite of it.

Every row below was measured with `-Q --batch` against GNU Emacs 31.0.90 and
against the pre-fix build of this branch (`cargo xtask fresh-build --release`);
the probe is `tmp/pw137/pin.el` and its output is `tmp/pw137/pin-gnu.txt` and
`tmp/pw137/pin-before.txt`.

| `make-pipe-process`, `(process-coding-system P)` | GNU | Neomacs before fix |
|---|---|---|
| nothing bound, multibyte buffer | `(utf-8-unix . utf-8-unix)` | `(utf-8-unix . utf-8-unix)` |
| `coding-system-for-read` = `binary` | `(binary . utf-8-unix)` | `(utf-8-unix . utf-8-unix)` |
| `coding-system-for-write` = `latin-1` | `(utf-8-unix . latin-1)` | `(utf-8-unix . utf-8-unix)` |
| UNIBYTE process buffer | `(nil . utf-8-unix)` | `(utf-8-unix . utf-8-unix)` |
| UNIBYTE process buffer + `coding-system-for-read` = `latin-1` | `(latin-1 . utf-8-unix)` | `(utf-8-unix . utf-8-unix)` |
| UNIBYTE **current** buffer, multibyte process buffer | `(utf-8-unix . nil)` | `(utf-8-unix . utf-8-unix)` |
| `default-process-coding-system` = `(latin-1 . koi8-r)` | `(latin-1 . koi8-r)` | `(utf-8-unix . utf-8-unix)` |
| `default-process-coding-system` = nil | `(nil . nil)` | `(utf-8-unix . utf-8-unix)` |
| `process-coding-system-alist` = `(("pw137p" binary . binary))` | `(utf-8-unix . utf-8-unix)` | `(utf-8-unix . utf-8-unix)` |
| `:coding '(nil . latin-1)` under `coding-system-for-read` = `binary` | `(nil . latin-1)` | `(nil . latin-1)` |
| `coding-system-for-read` = `no-such-xyz` | `(coding-system-error no-such-xyz)` | no error |
| `default-process-coding-system` = `(no-such-xyz . no-such-xyz)` | `(coding-system-error no-such-xyz)` | no error |

| `make-serial-process` on `/dev/ptmx`, `(process-coding-system P)` | GNU | Neomacs before fix |
|---|---|---|
| nothing bound | `(nil . nil)` | `(utf-8-unix . utf-8-unix)` |
| `coding-system-for-read` = `binary` | `(binary . nil)` | `(utf-8-unix . utf-8-unix)` |
| `coding-system-for-write` = `latin-1` | `(nil . latin-1)` | `(utf-8-unix . utf-8-unix)` |
| UNIBYTE process buffer | `(nil . nil)` | `(utf-8-unix . utf-8-unix)` |
| `default-process-coding-system` = `(latin-1 . koi8-r)` | `(nil . nil)` | `(utf-8-unix . utf-8-unix)` |
| `process-coding-system-alist` = `(("pw137s" binary . binary))` | `(nil . nil)` | `(utf-8-unix . utf-8-unix)` |
| `coding-system-for-read` = `no-such-xyz` | `(coding-system-error no-such-xyz)` | no error |
| `default-process-coding-system` = `(no-such-xyz . no-such-xyz)` | `(nil . nil)` | `(utf-8-unix . utf-8-unix)` |

That last serial row is the whole shape of the primitive in one line: an
UNDEFINED `default-process-coding-system` is not an error for a serial process,
because a serial process never looks at it.

### The bytes, not just the reporting slot

A pipe process cannot be written to from Lisp and read back -- its two pipes do
not join -- so the way to make one carry bytes is the way GNU itself makes one:
hand it to `make-process` as `:stderr`.  That is not a contrivance for the test;
it is the same call GNU makes internally when `:stderr` names a buffer
(`CALLN (Fmake_pipe_process, ...)`, src/process.c:1883).  The child's stderr is
then decoded by the PIPE's chain, resolved with whatever was bound when the pipe
was created rather than when the child was spawned.  Child writes
`c a f <c3> <a9> CR LF x CR LF`:

| binding at `make-pipe-process` time | GNU | Neomacs before fix |
|---|---|---|
| none | `(99 97 102 233 13 10 120 13 10)` | `(99 97 102 233 13 10 120 13 10)` |
| `coding-system-for-read` = `binary` | `(99 97 102 4194243 4194217 13 10 120 13 10)` | `(99 97 102 233 13 10 120 13 10)` |
| `coding-system-for-read` = `raw-text` | `(99 97 102 4194243 4194217 10 120 10)` | `(99 97 102 233 13 10 120 13 10)` |
| `coding-system-for-read` = `latin-1` | `(99 97 102 195 169 10 120 10)` | `(99 97 102 233 13 10 120 13 10)` |
| `default-process-coding-system` = `(binary . binary)` | `(99 97 102 4194243 4194217 13 10 120 13 10)` | `(99 97 102 233 13 10 120 13 10)` |
| UNIBYTE pipe buffer, nothing bound | `(99 97 102 195 169 10 120 10)` | `(99 97 102 195 169 13 10 120 13 10)` |

The last row is the shared second stage catching the first stage's error.  The
downgrade `setup_process_coding_systems` applies for a unibyte process buffer
(entry 131) was already right here; what it was handed was wrong.  GNU hands it
nil, `raw_text_coding_system (Qnil)` returns bare `raw-text`, and bare
`raw-text` DETECTS the line endings (entry 134), so the CR goes.  Neomacs handed
it `utf-8-unix`, whose end-of-line type is concrete, so the CR stayed.  Two
entries' worth of correct machinery, fed one literal.

### `make-network-process` was already right, and that is the finding

The third of the three does not belong in the tables above.  Its resolver,
`network_process_coding_pair`, was built when `set_network_socket_coding_system`
was first ported, and re-measuring it row by row against GNU found no
divergence at all in the chain -- including the two rows that are easiest to get
wrong:

```elisp
;; a unibyte process buffer, and an alist entry that matches the SERVICE
(let ((network-coding-system-alist (list (cons PORT (cons 'binary 'koi8-r))))
      (b (generate-new-buffer " *ub*")))
  (with-current-buffer b (set-buffer-multibyte nil))
  (process-coding-system (make-network-process :name "n" :host "127.0.0.1"
                                               :service PORT :buffer b ...)))
;; GNU     => (nil . koi8-r)
;; Neomacs => (nil . koi8-r)     ; before the fix as well as after
```

The decode half short-circuits past the alist because the PROCESS buffer is
unibyte; the encode half reaches it anyway because it asks `current_buffer`,
which is not.  Neomacs answered that, and answered the `open-network-stream`
`target-idx` question with it (the alist is keyed on the SERVICE, not the
process name -- src/coding.c:11788), and the accepted-connection rule
(`server_accept_connection` copies the listener's pair rather than re-running
the chain, src/process.c:5152-5158).  Twelve rows, byte-identical, before any of
this entry's changes.

What it did NOT do was check the answer:

```elisp
(let ((coding-system-for-read 'no-such-xyz))
  (make-network-process :name "n" :host "127.0.0.1" :service PORT
                        :buffer (generate-new-buffer " *mb*")))
;; GNU                => (coding-system-error no-such-xyz)
;; Neomacs before fix => a live process whose decode coding is `no-such-xyz'
```

The same three rows -- a bad `coding-system-for-read`, a bad
`default-process-coding-system`, a bad `network-coding-system-alist` entry --
were all silently accepted, and so were the corresponding rows on
`make-pipe-process` and `make-serial-process`.

Neomacs validated the `:coding` KEYWORD, eagerly, before creating anything.  GNU
validates the value the CHAIN produced, and it does so in a place none of the
five resolvers can see: `setup_process_coding_systems` runs `setup_coding_system`
on each half, and that is what signals (src/coding.c:5678, reached from
src/process.c:2573, :3277, :3761).  So a bad `coding-system-for-read`, a bad
`default-process-coding-system` and a bad `network-coding-system-alist` entry all
signal in GNU and none of them signalled here.  The same hole was in the pipe and
serial creators.

### Verifying entry 131's table against the C

131's table is the spec this entry implemented, and it was read against
`src/process.c` at emacs-mirror `0ee48ac4df2` before anything was built.  It
holds on every conclusion.  Four cells needed correcting, and one of the four is
the reason the network primitive was left with a hole.

* **The unibyte short circuit is not one column.**  131 has a single
  "unibyte buffer short-circuit" cell reading **yes** for pipe, serial and
  network.  GNU asks a DIFFERENT buffer for the two halves, and which buffer it
  asks differs between the three: pipe and network ask the process buffer for
  decode (:2533-2534, :3314-3317) and `current_buffer` for encode
  (:2559-2560, :3348-3349); serial asks the process buffer for both
  (:3258-3260, :3272-3274).  Measured, and the row is unrepresentable in one
  flag:

  ```elisp
  ;; unibyte CURRENT buffer, multibyte process buffer
  ;; make-pipe-process   => GNU (utf-8-unix . nil)
  ;; make-serial-process => GNU (nil . nil)
  ```

* **Serial's alist arm is not "the same dead `Qt`".**  131 says pipe and serial
  both initialise `coding_systems` to `Qt` and never assign it, citing
  ":2520, :3298".  :2520 is right -- that is `Fmake_pipe_process`'s, and the
  `CONSP (coding_systems)` arms at :2542 and :2563 really are dead code.  :3298
  is `set_network_socket_coding_system`'s, where the variable is very much
  alive.  `Fmake_serial_process` has no `coding_systems` variable at all and no
  alist arm to make dead; its chain simply ends.  The conclusion 131 drew --
  neither can reach `find-operation-coding-system` -- is right for both, by two
  different mechanisms.

* **"validates: no" is right about the resolver and wrong about the primitive.**
  131's last column marks `Fcall_process` as the only one that validates,
  because it is the only one that calls `Fcheck_coding_system` inline.  That is
  true of the resolvers and false of the calls: all four asynchronous primitives
  signal `coding-system-error` for an undefined resolved coding, from the shared
  second stage.  Read the first way it is a note about where a check lives; read
  the second way it says an undefined name is accepted, and that second reading
  is what `make-network-process` shipped.

* **A supplied `:coding` whose half is nil does not mean the same thing in all
  five.**  This one is not in 131's table at all, and it is the row that caught
  the first version of this fix.  GNU writes the three connection primitives as
  ONE `else if` chain, so a non-nil `tem` skips every later arm and a nil half
  answers nil.  `Fmake_process` writes the same first two arms and then a
  SEPARATE `if (NILP (val))` for its tail (:1958), so there a nil half falls
  through to the alist and the default.  Measured, all four under GNU 31.0.90,
  with `coding-system-for-read` bound to `binary` and `:coding '(nil . latin-1)`:

  ```text
  make-pipe-process    => (nil . latin-1)
  make-serial-process  => (nil . latin-1)
  make-network-process => (nil . latin-1)
  make-process         => (utf-8-unix . latin-1)     ; the tail ran
  ```

  131 states the shared half of this correctly -- a supplied `:coding` shuts the
  dynamic override out even when its own half is nil -- and its
  `make-process` measurement of the fall-through is right.  What is new is that
  the other three do NOT fall through.  The first version of the pipe resolver
  written for this entry reused one helper for the step and answered
  `(utf-8-unix . latin-1)` for a pipe; running the probe is what found it, and
  the shape of the C is what explained it.

* Line ranges have drifted about ten lines.  Current: pipe :2517-2570, serial
  :3247-3275, network :3291-3373.

Two cells 131 flagged as easy to read past were checked hardest and are exactly
right.  Pipe and serial really cannot reach `process-coding-system-alist`, and
serial really never reaches `default-process-coding-system` -- both measured
above, both by binding the variable to something that would be visible and
finding it invisible.

### Why GNU's code is shaped this way

The `val = Qnil` arms carry their own explanation, at src/process.c:2535-2538:

```c
      /* We dare not decode end-of-line format by setting VAL to
	 Qraw_text, because the existing Emacs Lisp libraries
	 assume that they receive bare code including a sequence of
	 CR LF.  */
```

That is a statement about the SLOT, not about the bytes.  Leaving
`process-coding-system` nil is what stops a library from reading `raw-text` out
of it and concluding that end-of-line conversion has been turned off; the
fd-level downgrade in `setup_process_coding_systems` still happens underneath
whenever the default filter is inserting into a unibyte buffer, and it converts
end-of-line, which is precisely the last payload row above.  The comment and the
behaviour disagree by design, and only measurement separates them.

The serial chain's missing tail has a reason too.  A serial port is a byte pipe
to a device with no notion of a locale; `default-process-coding-system` is about
subprocesses that inherit the user's environment.  Detecting is the honest
default for a device, and detecting is what nil buys.

### The type-level fix

The hole was a required decision with no required parameter -- the same hole
entry 131 removed from `make-process`, still open two doors down.
`create_process_with_kind_lisp` initialised both coding slots from a literal,
and the pipe and serial creators overwrote them only when the caller passed
`:coding`.  Nothing in any signature said a resolution was owed.

`ProcessCodingSystems` is now a **required parameter of every process
constructor** (`neovm-core/src/emacs_core/process.rs`).  There is no signature
left in which a process record can come into existence without a caller naming
where its pair came from, and since the type still has no `Default` and no
field-wise constructor, the only ways to name one are the five resolvers plus
two constructors that say what they are: `inherited_from_server` (GNU's
`server_accept_connection`, which deliberately does not re-resolve) and
`gnu_make_process_initial` (both slots nil -- GNU's `make_process` state before
any resolver has run, for internal records and test fixtures that never carry
bytes).  The `utf-8-unix` literal is gone from the tree.

The three resolvers are three functions, not one parameterised one, and what
they may consult is expressed as what their environments HAVE:

```rust
struct PipeProcessCodingEnvironment {           // no operation_coding_system
    coding_system_for_read: Value,
    coding_system_for_write: Value,
    default_process_coding_system: Value,
    short_circuit: ConnectionProcessUnibyteShortCircuit,
}

struct SerialProcessCodingEnvironment {         // and no default, and no short circuit
    coding_system_for_read: Value,
    coding_system_for_write: Value,
}
```

A pipe resolver cannot consult `process-coding-system-alist` because it has no
field holding one; a serial resolver cannot reach the process default or the
buffer for the same reason.  Those are the two cells of 131's table that are
easiest to read past, and they are now facts about the types rather than
comments next to an `if`.  Serial's missing `short_circuit` is the third: GNU
does spell the arm out for both halves, but since its fallthrough is also nil
the arm cannot change an answer -- and a field that cannot change an answer is a
field someone will eventually key something real off.

`ConnectionProcessUnibyteShortCircuit` is the column 131's table collapsed.  It
has one bool per HALF, and it is built by a constructor named after the
primitive, so choosing which buffer answers for which half is a decision with a
name and a citation instead of an argument order:

```rust
fn pipe(multibyte: ProcessBufferMultibyteness) -> Self     // process buffer, current buffer
fn network(multibyte: ProcessBufferMultibyteness) -> Self  // process buffer, current buffer
```

`ProcessBufferMultibyteness` carries both buffers rather than "the" buffer, for
the same reason: a single flag cannot express the `(utf-8-unix . nil)` row.

The step the three DO share returns a two-state `ConnectionCodingStep`
(`Answered(Value)` / `Continue`) rather than a `Value`, and that is the
correction above made unrepresentable.  For the connection primitives
`Answered(nil)` and "nothing answered" are different outcomes -- the first ends
the chain, the second runs the tail -- and a helper returning a bare `Value`
cannot tell them apart.  `Fmake_process` keeps its own two-line step returning a
`Value`, precisely because there the two ARE the same thing.  The first version
of this fix shared one `Value`-returning helper between all four and was wrong
in exactly the row a shared helper cannot express.

The check moved to where GNU's effect is.  `validate_resolved_process_coding_systems`
takes the resolved PAIR rather than the `:coding` keyword, and it runs where
`setup_process_coding_systems` runs -- which for pipe and serial is immediately
after the resolver (src/process.c:2573, :3277) and for network is only after the
socket exists (:3761).  That last distinction is measurable, and getting it
wrong is how the first version of this fix introduced a divergence of its own:

```elisp
;; a port nobody is listening on, and an undefined coding system
(let ((coding-system-for-read 'no-such-xyz))
  (make-network-process :name "x" :host "127.0.0.1" :service DEAD-PORT))
;; GNU => file-error      ; the connect loses first; the coding is never checked
```

For the network primitive that meant giving both steps a home.  Five connection
strategies (TCP, local socket, datagram, listening, explicit `:local`/`:remote`
address) build a process record at twenty-four points between them -- once per
socket family, per `:nowait` branch, per TLS branch -- and every one of them
used to be handed the `:coding` keyword plus an environment and to resolve for
itself at the bottom.  They are now handed a `ProcessCodingSystems`, resolved at
the two points where the environment is final -- before the `:local`/`:remote`
early return, which cannot reach the alist because it has no HOST and SERVICE,
and after the alist lookup for everything else -- and all twenty-four create
their record through one `create_network_process_record`, which is where the
check runs.  None of them can resolve differently, and none can create a record
without the check having happened at GNU's moment.

`builtin_make_process_impl`'s `:stderr` pipe goes through
`MakeProcessCodingEnvironment::connection_variables()` rather than through
`Fmake_process`'s own answer, because GNU builds that pipe with
`CALLN (Fmake_pipe_process, ...)` and it therefore runs the PIPE chain.  That is
measurable -- it is the payload table above, every row of which binds around the
`make-pipe-process` call and not around the `make-process` one.

### How the two hard ones were actually run

A network process needs a listener and a serial process needs a device, and
neither was skipped.

The network pins run against a real loopback listener created in the same test:
`:server t :host "127.0.0.1" :service t :family 'ipv4`, with the port read back
out of `(process-contact SERVER t)`.  The accepted-connection row connects to a
second listener and then finds the `SERVER <N>` process in `process-list`.

The serial pins run against `/dev/ptmx`, which is a real character device that
`serial_open` and the `tcgetattr` in `serial_configure` both accept on any
Linux; `/dev/null` fails `tcgetattr` (`(file-error "Failed tcgetattr")`) and a
nonexistent path signals `file-missing`, both measured.  For the PAYLOAD half a
pty PAIR was built instead -- `tmp/pw137/ptyhelper.py` holds the master open and
prints the slave's path, `make-serial-process` opens the slave, and the helper
writes the bytes -- and GNU decodes them exactly as the chain predicts:

```
;; child writes  c a f <c3> <a9> CR LF x CR LF  through a pty into a serial process
;; nothing bound             => GNU (99 97 102 233 10 120 10)          ; nil = detect
;; coding-system-for-read binary => GNU (99 97 102 4194243 4194217 13 10 120 13 10)
;; :coding latin-1           => GNU (99 97 102 195 169 10 120 10)
;; default-process-coding-system (binary . binary) => GNU (99 97 102 233 10 120 10)
```

That last row is the serial chain's missing tail, measured on real bytes rather
than on a reporting slot.  **It is not pinned**, because Neomacs cannot run it:
`make-serial-process` here creates a process record without opening the port, so
no bytes ever arrive and the buffer stays empty on every row.  Pinning it would
have recorded the fixture, not the behaviour.  The device gap is recorded below.

### Measured after

Against a `cargo xtask fresh-build --release` binary, `tmp/pw137/pin.el` --
seventeen `make-pipe-process` rows, nine pipe payload rows, eleven
`make-serial-process` rows and twelve `make-network-process` rows -- is
byte-identical to GNU Emacs 31.0.90, `diff` clean.  So are the four wider probes
it was cut down from (`tmp/pw137/pipe.el`, `serial.el`, `network.el`,
`netvalid.el`).

`cargo nextest run -p neovm-core` is 9014/9014 green.  `cargo nextest run -p
neovm-oracle-tests` is 38770/38770 green.  An earlier run of the oracle suite,
mid-change, was 38769/38770 with `div_cx27_process_exit_code_various_signals`
red; it passes on its own and it passed on the final run, and its shape is a
race rather than a divergence -- it signals a `sleep 30` child and gives it one
second to be reaped, and the red run recorded `(run 0)`, meaning the signal had
not landed yet.  It touches no coding system.
`cargo check --workspace --all-targets` and `cargo fmt --all --check` are
clean.  The nine MELPA suites that build a pipe process -- `vterm`, `sly`,
`slime`, `ivy_rich`, `all_the_icons_ivy_rich`, `async_http_queue`,
`auto_pause`, `git_rebase_mode` and the `diredfl` suite entry 128 came from --
are 13/13 green; all of them create their pipe with `:buffer nil` and no
`:coding`, where GNU's answer is the car and cdr of
`default-process-coding-system` and so is unchanged by this entry.

One pinned expectation moved, and it moved because it was recording the literal
this entry deleted.  `vm_process_coding_and_tty_builtins_use_shared_runtime_state`
asserted `(utf-8-unix . utf-8-unix)` for a fixture built through
`ProcessManager::create_process` rather than through any of GNU's five
primitives -- a process no resolver has run on.  It now asserts `(nil . nil)`,
which is what GNU's `make_process` leaves in those slots.

### Found and NOT fixed here

**`make-serial-process` does not open its port.**  It builds the process record,
runs the coding chain, honours `serial-process-configure`, and never touches the
device.  So `(make-serial-process :port "/dev/no-such-tty" :speed 9600)` succeeds
where GNU signals `(file-missing "Opening serial port")`, `:port "/dev/null"`
succeeds where GNU signals `(file-error "Failed tcgetattr")`, and no serial
process ever delivers output.  The coding chain fixed here is correct in advance
of the I/O it will decode; the I/O is its own change, with a PAL question in it
(termios lives in `process/sys/`), and no MELPA pin depends on it.

**GNU Emacs 31.0.90 segfaults on an invalid coding system for a unibyte process
buffer.**  Reproduced on the shipped 31.0.90 binary, four ways, always
immediately at creation and before any Lisp can observe it:

```elisp
(let ((b (generate-new-buffer " *ub*")))
  (with-current-buffer b (set-buffer-multibyte nil))
  (let ((coding-system-for-read 'no-such-xyz))
    (make-pipe-process :name "x" :noquery t :buffer b)))
;; GNU => SIGSEGV
```

`make-network-process` with a unibyte buffer does the same, and so does an
explicit `:coding 'no-such-xyz`.  The multibyte-buffer form of all three signals
`coding-system-error` cleanly.  The cause is visible in the C:
`setup_process_coding_systems` applies `raw_text_coding_system` BEFORE
`setup_coding_system` gets to validate (src/process.c:8395-8399), and
`raw_text_coding_system` does `AREF (CODING_SYSTEM_SPEC (coding_system), 0)`
(src/coding.c:5941-5942) on a spec that is nil for an undefined name.  Neomacs
signals `coding-system-error` for these, which is what the multibyte form does
and is strictly better than crashing -- but those rows are deliberately NOT
pinned, because there is no GNU answer to pin them against.

**`Vlast_coding_system_used` is still not written back onto the process.**  Entry
131's second residual, still open, and it now has a second witness: the serial
payload probe above shows GNU reporting `utf-8-dos` from
`(process-coding-system P)` after a read that started from nil, because
`read_process_output_set_last_coding_system` (src/process.c:6417-6425) replaced
the slot with the coding actually used.  Every pin in this entry therefore reads
the slot BEFORE any output arrives, which is the only way to measure the chain
rather than the write-back.

> **Closed, 2026-08-18, by entry 139.**  The instruction in the last sentence
> stands and is now load-bearing in the other direction too: 139's pins read the
> slot AFTER output for exactly the same reason, and the two sets of pins measure
> the two halves without overlapping.

**A function-valued `network-coding-system-alist` entry can run when GNU would
not have called it.**  GNU computes the alist answer lazily, inside the `else`
arm of each half, so when BOTH halves short-circuit -- a unibyte process buffer
and a unibyte current buffer -- `find-operation-coding-system` is never called at
all.  Neomacs calls it once, up front, whenever `:coding` is absent.  Only a
function-valued entry with side effects can tell, and a function value has to be
a SYMBOL naming a function to be called at all (`if (! SYMBOLP (val)) return
Qnil;`, src/coding.c:10858), so a lambda in the alist is inert in both.  Left
alone: making it lazy means running arbitrary Lisp from inside the resolver,
which is exactly the borrow the resolvers were separated from the evaluator to
avoid.

## 138. Twenty-six variables existed only here, eight GNU declares did not exist at all, `baud-rate` held a terminal's answer in a session that has no terminal, and both variables GNU declares `DEFVAR_INT` *and* buffer-local kept only the buffer-local half -- FIXED

The residual list entries 132 and 135 left behind.  Every item was reproduced
before anything was touched, `-Q --batch`, against GNU 31.0.90 and against a
`cargo xtask fresh-build --release` binary of `42fe3dd71` (ledger 134's merge,
which contains 132 and 135).  All four items diverge.  Two of them are bigger
than the residual said -- item 1 is 26 names, not four -- and one diverges on a
different axis than the residual predicted.

```elisp
(list (mapcar #'boundp '(dos-hyper-key dos-keypad-mode dos-super-key
                         imagemagick-render-type))
      baud-rate
      (mapcar #'boundp '(command-line-max-length large-hscroll-threshold
                         long-line-optimizations-bol-search-limit
                         long-line-optimizations-region-size max-redisplay-ticks
                         strings-consed x-color-cache-bucket-size
                         x-mouse-click-focus-ignore-time))
      (condition-case e (set-default 'display-line-numbers-offset "x") (error (car e))))
;; GNU                => ((nil nil nil nil) 0 (t t t t t t t t) wrong-type-argument)
;; Neomacs before fix => ((t t t t)         38400 (nil nil nil nil nil nil nil nil) "x")
```

### Item 1: invented existence, and it was 26 names rather than four

`lisp/cus-start.el` lists every variable GNU's C layer can define across all the
platforms GNU builds for.  When one is not bound it consults a `native-p` test
before complaining -- `dos-` needs `(eq system-type 'ms-dos)`, `ns-` needs
`(featurep 'ns)`, `imagemagick-` needs `(fboundp 'imagemagick-types)`,
`xwidget-` needs `(boundp 'xwidget-internal)` -- and only then signals
"built-in variable `%S' not bound" (`lisp/cus-start.el:893-951`).  So a build
without MS-DOS support does not need `dos-hyper-key` to exist.  It needs it to
NOT exist.

Neomacs had one `for name in [...]` loop in `eval.rs` seeding 34 such names to
nil.  Probed name by name under GNU 31.0.90 on GNU/Linux: GNU binds **8** of
them and leaves **26** unbound, so 26 of the 34 were invented existence and the
four the residual named were a sample of them.

Two of the 34 leave the loop without becoming table rows.
`temporary-file-directory` is not a platform name at all -- `filelock.c:814`
declares it for every build, with the same nil init -- so it keeps a seed of its
own.  And `xwidget-webkit-disable-javascript` stays bound, because
`neovm-core/src/emacs_core/xwidget.rs` declares it beside `xwidget-list` and
`xwidget-view-list` as part of an xwidget layer Neomacs really has and this GNU
build does not: that is a build-feature difference, the same kind as
`(featurep 'x)`, not a stub.  The table is therefore 32 rows -- 7 GNU binds
here, 25 it does not.

```elisp
(mapcar #'boundp '(dos-hyper-key ns-antialias-text w32-follow-system-dark-mode
                   haiku-use-system-tooltips xwidget-internal
                   imagemagick-render-type))
;; GNU                => (nil nil nil nil nil nil)
;; Neomacs before fix => (t   t   t   t   t   t)
```

The four the residual named are all `special-variable-p` => nil here, so nothing
even marked them special; they were plain cells whose only job was to keep
`cus-start.el` quiet, and `cus-start.el` was never going to complain.

### Item 2: nothing in GNU's startup ever writes `baud-rate` under `--batch`

The residual said "GNU computes it in `init_baud_rate` at terminal init and
reports 0 under `--batch`".  Half right, and the other half is the whole
mechanism.  `init_baud_rate` cannot return 0: with `noninteractive` it sets
`emacs_ospeed = 0`, and `baud_convert[0]` is 0, and the next two lines are
`if (baud_rate == 0) baud_rate = 1200;` (`src/sysdep.c:413-437`).  GNU reports 0
under `--batch` because `init_baud_rate` is **never called** -- its only callers
are inside `init_tty` (`src/term.c:4755`, `4923`), and `--batch` creates no tty
terminal.

`DEFVAR_INT ("baud-rate", baud_rate, ...)` (`src/dispnew.c:7488`) is one of
twenty of GNU's 74 `DEFVAR_INT` declarations with no assignment beside it, and
the only one of those twenty that no `init_*` supplies either -- `alloc.c`'s
eleven get theirs from `init_alloc_once` or are counters that start at zero,
`internal-when-entered-debugger` from `init_eval`, and so on.  `baud-rate`'s
slot is `globals.f_baud_rate` (`src/globals.h:1018`), zero-initialized like any
C global, and exactly three kinds of code ever write it: `init_baud_rate` from a
tty terminal init, a window system's flat `baud_rate = 19200`
(`src/xterm.c:32279`, `src/pgtkterm.c:7034`, and the same constant in the w32,
Haiku and Android terminal inits), and Lisp.  Nothing in `syms_of_dispnew` does.

38400 was therefore not a wrong constant, it was a right constant in the wrong
place: it is what `cfgetospeed` reports for a modern pty, hoisted up into the
declaration where a session with no terminal at all can see it.  The eighth
instance of this project's recurring invented-default class -- entry 135's two,
in the same `eval.rs` neighbourhood, are the ninth and tenth -- and the variant
where the value is right for one session kind and the bug is that it is
unconditional.

### Item 3: `display-line-numbers-offset` -- the residual named the wrong axis

The residual expected making it forwarded to "change its buffer-local nature as
well as its type".  Measured, it does not: GNU declares it BOTH ways and in this
order (`src/xdisp.c:38999-39005`),

```c
  DEFVAR_INT ("display-line-numbers-offset", display_line_numbers_offset, ...);
  display_line_numbers_offset = 0;
  DEFSYM (Qdisplay_line_numbers_offset, "display-line-numbers-offset");
  Fmake_variable_buffer_local (Qdisplay_line_numbers_offset);
```

and `make_blv` copies the symbol's forwarder into the BLV
(`src/data.c:2112-2140`), so the two are orthogonal.  Exactly two variables in
GNU's tree are `DEFVAR_INT` plus `Fmake_variable_buffer_local` -- this one and
`syntax-propertize--done` (`src/syntax.c:3773-3778`) -- against the six of the
same shape entry 135 found among the `DEFVAR_BOOL`s.  Neomacs's
`defvar_buffer_local` already produced the right locality, and the whole cost of
the wrong kind was the type rule and the unbind refusal:

```elisp
(list (local-variable-if-set-p 'display-line-numbers-offset)
      (condition-case e (let ((display-line-numbers-offset "x")) t) (error (car e)))
      (with-temp-buffer
        (list (condition-case e (setq-local display-line-numbers-offset "x") (error (car e)))
              display-line-numbers-offset))
      (condition-case e (makunbound 'display-line-numbers-offset) (error (car e))))
;; GNU                => (t wrong-type-argument (wrong-type-argument 0) error)
;; Neomacs before fix => (t "x"                 ("x" "x")               nil)
```

`local-variable-if-set-p` agreed before and after; `setq-default` and
`default-value` agreed before and after.  The divergence was entirely item 2's
neighbour -- a slot that accepted a string -- reached through a redirect entry
132 had wired for the non-localized case only.

Looking for the other member of the pair turned up the same symptom with a
different cause, and it is the more instructive one.  `syntax-propertize--done`
was ALREADY declared with `define_int_variable` and localized by a
`make_symbol_localized` that copies the descriptor across (`syntax.rs:264-271`),
so a fresh `Context` refused a string exactly like GNU.  A binary that had gone
through a portable dump did not:

```elisp
(list (default-value 'syntax-propertize--done)
      (condition-case e (set-default 'syntax-propertize--done "x") (error (car e))))
;; GNU                => (-1 wrong-type-argument)
;; Neomacs before fix => (-1 "x")           ; only after a pdump round trip
```

A `Localized` symbol serialized as its default plus `local_if_set` and nothing
else, so the descriptor `make_blv` had copied in was gone on the way back.  That
is the hole entry 135 patched for `DEFVAR_BOOL` by rebuilding from
`GNU_BOOL_VARIABLES` on load -- a list that, being a list of Booleans, could not
have covered this.

### Item 4: eight `DEFVAR_INT` variables with no declaration here

All eight bound under GNU, none bound here.  GNU's values, read out of the C and
then confirmed by running GNU:

| variable | GNU | GNU `file:line` |
| --- | --- | --- |
| `command-line-max-length` | `sysconf (_SC_ARG_MAX) / 4` (626432 here) | `callproc.c:2240` |
| `large-hscroll-threshold` | 10000 | `buffer.c:6043` |
| `long-line-optimizations-bol-search-limit` | 128 | `buffer.c:6025` |
| `long-line-optimizations-region-size` | 500000 | `buffer.c:6007` |
| `max-redisplay-ticks` | 0 | `xdisp.c:39295` |
| `strings-consed` | a live allocation counter | `alloc.c:7448` |
| `x-color-cache-bucket-size` | 128 | `xterm.c:32922` |
| `x-mouse-click-focus-ignore-time` | 200 | `xterm.c:32704` |

Two of the eight are not constants and neither is pinned as one.
`command-line-max-length` is `sysconf (_SC_ARG_MAX) / 4` -- "divide it by 4 as a
crude way to go bytes->characters" -- so it describes the machine, and it is
computed here by asking the same `sysconf`, with GNU's own 4096 fallback.
`strings-consed` moved between two consecutive GNU runs in this session (69765,
then 70960), which is why the pin asserts its shape.

### Why GNU's code is shaped this way

The four items are one shape seen from four sides: in GNU a variable's existence,
its type and its storage are all consequences of a C declaration, and there is
nowhere else for any of them to be decided.  `DEFVAR_INT` binds the symbol to an
`intmax_t *` (`src/lisp.h:3513-3518`), so the slot's type is the slot; the
declaration sits inside a `syms_of_*` that a platform's `#ifdef` either compiles
or does not, so existence is the build; and the initializer is a plain
assignment on the next line, so "no initializer" means "0", not "whatever seems
reasonable".  `Fmake_variable_buffer_local` on the line after does not replace
any of that -- `make_blv` carries the forwarder into the BLV precisely so it
does not.

Neomacs has no `#ifdef` and no C globals, so each of those four facts had to be
written down somewhere, and each was written down in the shape that loses it: a
flat list of names to seed nil (existence), a `Value` cell (type), a
plausible-looking literal (the initializer), and `defvar_buffer_local` (which
answered the locality question and silently answered the type question too).

### The type-level fix

**Existence.** `neovm-core/src/emacs_core/cus_start_platform_vars.rs` is the 32
platform names as a table whose every row carries `GnuBinding::{BoundHere,
UnboundHere}`, measured under GNU rather than derived from the `#ifdef`s.  It is
a required field, so a row cannot be added without answering "does GNU bind this
in a build like this one?", and only `BoundHere` rows are registered.  The 25
`UnboundHere` rows stay in the table on purpose: deleting them would delete the
answer, and the next author would find `ns-antialias-text` in `cus-start.el` and
seed it again.  This is entry 135's `ByteBooleanVars` applied to the other
question a declaration answers.

**`baud-rate`.** The bootstrap seed is 0 -- GNU's zero-initialized global -- and
`init_baud_rate` is ported to where GNU calls it from.
`tty_init::detect_baud_rate` is `src/sysdep.c:413-437` including the
`baud_convert` table, the 9600 fallback for a speed code past its end and the
1200 substitution for zero, and `neomacs-bin` calls it from the live-tty branch
next to the `tty-erase-char` read, which is the same "while stdin is still
cooked" window `init_sys_modes` uses.  The GUI branch does what
`x_term_init`/`pgtk_term_init` do: assigns 19200.  `--batch` reaches neither, so
it keeps the 0, which is what makes the batch answer right for GNU's reason
rather than by choosing a different constant.

**`display-line-numbers-offset`** is now `define_int_variable` followed by
`make_buffer_local`, in GNU's order, and `define_int_variable` grew the
already-localized branch `define_bool_variable` has had since entry 135, so
declaring an already-localized symbol declares into the BLV instead of orphaning
it.

**The eight** are declared through entry 132's `define_int_variable` at the
Neomacs counterpart of the `syms_of_*` each belongs to, not as plain cells.

One thing entry 135 built is now gone rather than duplicated.  A `Localized`
symbol could not carry its forwarder through a portable dump, so 135 rebuilt the
Boolean ones from `GNU_BOOL_VARIABLES` on load; adding a buffer-local
`DEFVAR_INT` would have meant a second hand-maintained name list beside it --
the opt-in invariant each branch must remember, which is the failure mode entry
132 named.  Instead `DumpSymbolVal::Localized` now records WHICH forwarder
`make_blv` copied in, as `Option<DumpLocalizedForwarder>` whose two variants are
exactly the forward types that own their value (the same distinction
`LispFwd::clone_stateful` already makes), and `load_obarray` rebuilds it from the
image.  `defvar_bool::reattach_localized_bool_forwarders` is deleted.  A
localized forwarded symbol that came back from a dump without its descriptor is
no longer a state a future declaration can reach, and `syntax-propertize--done`
-- which nobody had noticed was in it -- came out fixed without being named
anywhere.  The pdump format goes to v58.

### Measured after

The same probes, `-Q --batch`, against a `cargo xtask fresh-build --release`
binary.  The sweep is over all 74 names `grep DEFVAR_INT src/*.c` finds, asking
each one `boundp` and then whether `(set-default X "x")` is GNU's
`wrong-type-argument`:

```
                                          GNU     before   after
DEFVAR_INT names bound                    52      48       52
  ... unbound (w32/Android/MS-DOS/pgtk/
      ImageMagick/GLYPH_DEBUG)            22      26       22
  ... bound but accepting a string        0       6        0
`boundp' of the 25 platform names GNU
   omits from cus-start.el                0       25       0
`boundp' of the 7 platform names GNU
   keeps                                  7       7        7
baud-rate                                 0       38400    0
display-line-numbers-offset local-if-set  t       t        t
```

The 74-name sweep is byte-identical between the two editors, `unbound` list
included.  The wider probe -- `boundp`, `special-variable-p`, `default-value`,
`local-variable-if-set-p` and a wrong-typed `set-default` over all four items
plus the `cus-start.el` platform names -- differs on eight lines, all of them
recorded under "found and not fixed" below: two values that describe the process
rather than the editor, four `x-` defaults that were nil before this entry and
still are, one missing `special` bit, and one build-feature difference.  Every
line this entry set out to change agrees.

The six that accepted a string before were `dos-hyper-key`, `dos-keypad-mode`,
`dos-super-key` and `imagemagick-render-type` -- plain cells that GNU does not
declare here at all -- plus the two buffer-local ones,
`display-line-numbers-offset` and `syntax-propertize--done`.  Item 1 and item 3
are the same six names seen from two directions.

Blast radius, measured rather than assumed: nine variables changed storage or
existence and 26 stopped existing, so the suites were the gate.
`cargo nextest run -p neovm-oracle-tests`: **38778/38778 green**, which is the
previous 38770 plus the eight pins added here
(`neovm-oracle-tests/src/defvar_int_declarations.rs`) -- the platform names GNU
omits, `baud-rate` in batch, both buffer-local `DEFVAR_INT`s, the eight new
declarations' boundness and typing, their GNU values, the `overflow-error` /
`wrong-type-argument` pair, and a sweep over all 74 `DEFVAR_INT` names asserting
Neomacs binds exactly the 52 GNU binds.  **No pin moved.**
`cargo nextest run -p neovm-core`: **9016/9016 green**, including four new
`forward_test.rs` cases and the two table-shape tests in
`cus_start_platform_vars.rs`.  One existing assertion changed rather than being
deleted: `gnu_defvar_special_test.rs` pinned `baud-rate` at 38400 with the
comment "init_baud_rate supplies 38400 on ptys", which is the third comment
asserting parity that was itself the bug in a week.

One trap worth recording, because it produced a red test that was right about
the editors and wrong about the harness.  The first version of the
`overflow-error` pin ended with `(setq large-hscroll-threshold
most-positive-fixnum)`, and its expectation was taken by running GNU directly:
2305843009213693951.  Neomacs answered 0.  Both editors are in fact identical
there -- the oracle's own normalizer rewrites any fixnum past 10^12 to 0
("large fixnums in error data are implementation artefacts",
`neovm-oracle-tests/src/common.rs`), and the GNU value had been measured outside
it.  "Take the expected value by running GNU" has to mean running it the way the
harness will.  The pin uses 999999 now.

### Corrections to earlier entries

- Entry 132's residual "`display-line-numbers-offset` is `DEFVAR_INT` in GNU
  (`xdisp.c`) but `defvar_buffer_local` here, so it was left out of the
  conversion; making it forwarded would change its buffer-local nature as well
  as its type" is wrong on the second half, corrected 2026-08-17: GNU declares
  it `DEFVAR_INT` **and** `Fmake_variable_buffer_local`
  (`src/xdisp.c:38999-39005`), the two are independent because `make_blv` copies
  the forwarder into the BLV, and the measured buffer-local behaviour
  (`local-variable-if-set-p`, `setq-default`, `default-value`, a per-buffer
  integer binding) was identical before and after the conversion.
- Entry 132's residual "GNU computes it in `init_baud_rate` (`src/sysdep.c`) at
  terminal init and reports 0 under `--batch`" is corrected 2026-08-17:
  `init_baud_rate` cannot report 0 -- its floor is 1200 (`src/sysdep.c:435-436`)
  -- and GNU reports 0 under `--batch` because `init_tty` never runs, so nothing
  ever writes the slot.  The fix is therefore two assignments at two terminal
  inits plus a zero seed, not a port of the `noninteractive` arm.
- Entry 132's residual "Neomacs binds four variables GNU leaves unbound on this
  platform" is corrected 2026-08-17 to 26 of the 34 names in that one seed loop;
  the four named were a sample of them.
- Entry 135's "Registration runs FIRST among the bootstrap registrations ... so
  a variable that gets localized later ... has to be forwarded before that
  happens or the coercion is dropped on the floor" still holds, but the dump
  half of it -- "`reattach_localized_bool_forwarders` rebuilds them from the
  value the image did carry" -- is superseded 2026-08-17: the image now carries
  the forwarder KIND per symbol and that function is deleted.

### Found and NOT fixed here

- Five of the seven platform names GNU DOES bind here are `special-variable-p`
  => nil against GNU's `t`, and four of those five hold nil where GNU holds a
  value: `x-bitmap-file-path` (GNU `("/usr/include/X11/bitmaps")`),
  `x-gtk-use-system-tooltips` (`t`, and in GNU it is `term/x-win.el`'s
  `defvaralias` onto the `DEFVAR_BOOL` `use-system-tooltips`),
  `x-scroll-event-delta-factor` (`1.0`) and `x-auto-preserve-selections`
  (`(CLIPBOARD PRIMARY)`); `vertical-centering-font-regexp` already has GNU's
  regexp and only lacks the special bit.  That is the invented-default class
  rather than the invented-existence class this entry closes, and each wants its
  own `syms_of_*` counterpart rather than a seed -- which is what
  `window-combination-limit` and `void-text-area-pointer` already have, and both
  measure identical to GNU.
- `command-line-max-length` is 1572864 here against GNU's 626432 on this
  machine, and the declaration is not what differs.  glibc derives
  `_SC_ARG_MAX` from `RLIMIT_STACK` -- `MAX (ARG_MAX, MIN (stack / 4, 6 MiB))`
  -- so GNU's own answer is not a constant either: it is 626432 whether the
  shell's `ulimit -s` is 8192 or `unlimited`, because Emacs sets its own stack
  limit, and Neomacs sets a larger one and lands on the 6 MiB cap.  Two editors
  with different stack policies, not two different declarations; the pin asserts
  the shape for exactly that reason.
- `xwidget-webkit-disable-javascript` is bound here and unbound under GNU,
  because Neomacs ships an xwidget layer this GNU build was not compiled with.
  Same class as `(featurep 'x)` being `nil` here and `t` under GNU on this
  machine -- a build-feature difference, which also changes which branch
  `cus-start.el`'s `native-p` takes for four names.  Neither is a question about
  a variable's declaration.
- `strings-consed` joins the six allocation counters entry 132 left reading 0:
  the declaration is right, nothing increments it.
- The 36 other-window-system `DEFVAR_BOOL`s stay out of entry 135's table, and
  the 22 `DEFVAR_INT`s GNU leaves unbound here stay undeclared -- 21 belonging
  to w32, Android, MS-DOS, pgtk or ImageMagick builds and `debug-end-pos` to
  `GLYPH_DEBUG` -- for the same reason the 26 names above were removed.
- `Lisp_Fwd_Obj` and `Lisp_Fwd_Kboard_Obj` remain unwired, unchanged from 132
  and 135.

Status: FIXED.

## 139. A subprocess's decoder was rebuilt from scratch on every read, so the coding system it resolved was never reported and never remembered -- FIXED

The residuals entry 131 opened and entry 134 sharpened, closed together because
they are one mechanism.  Reproduced first, with `-Q --batch` against GNU Emacs
31.0.90 and against this branch's own pre-fix `cargo xtask fresh-build --release`
binary.  The before/after probes are `tmp/pw46/pin.el`, `tmp/pw46/pin2.el` and
`tmp/pw46/pin5.el`, each with its `-gnu`, `-before` and `-after` output;
`tmp/pw46/final.el` is the consolidated probe the pins below were cut from, run
against GNU and against the fixed binary (`final-gnu.txt`, `final-after.txt`).

```elisp
;; a child writing  a CRLF b CRLF, then after a pause  x CR y CR
(let* ((buf (generate-new-buffer " *p*"))
       (p (let ((coding-system-for-read 'utf-8))
            (make-process :name "p" :buffer buf :sentinel #'ignore
                          :connection-type 'pipe
                          :command '("sh" "-c" "printf 'a\\r\\nb\\r\\n'; sleep 0.6; printf 'x\\ry\\r'")))))
  (while (accept-process-output p 1))
  (while (process-live-p p) (accept-process-output p 0.05))
  (list (append (with-current-buffer buf (buffer-string)) nil)
        (process-coding-system p)
        last-coding-system-used))
;; GNU                => ((97 10 98 10 120 13 121 13) (utf-8-dos . utf-8-unix) utf-8-dos)
;; Neomacs before fix => ((97 10 98 10 120 10 121 10) (utf-8 . utf-8-unix) prefer-utf-8-unix)
```

(The text is `tmp/pw46/pin5.el`'s pipe row and the two slots are
`tmp/pw46/pin.el`'s; neither slot depends on the connection type.)

Three separate wrongs in one line.  The second chunk's bare CRs were eaten,
because it re-detected its end-of-line type from scratch and a chunk of nothing
but CRs is `mac`.  `(process-coding-system p)` still reported the coding system
the CHAIN resolved rather than the one the DECODE used.  And
`last-coding-system-used` was not written at all -- `prefer-utf-8-unix` is
whatever the last `decode-coding-string` of the bootstrap happened to leave
there, and it is the same value after a `call-process`, after a
`make-process`, and after every other subprocess read in the editor.

### GNU has ONE `struct coding_system` per process, and that is the whole story

Everything below follows from a single fact about `src/process.c`: a subprocess's
output is decoded through `proc_decode_coding_system[channel]`, one object,
created by `setup_process_coding_systems` and reused for every read.  Three
things live in that object across reads, and Neomacs had none of them.

**The resolved end-of-line type.**  `decode_eol` resolves a VECTOR eol type by
scanning the produced text and calling `adjust_coding_eol_type`
(src/coding.c:6805), which REPLACES `coding->id` with the subsidiary.  The next
read therefore starts from a concrete type and never detects again.

**The name that replacement produced.**
`read_process_output_set_last_coding_system` (src/process.c:6417-6446) reads it
back out after every decoded run and does three writes with it:

```c
  Vlast_coding_system_used = CODING_ID_NAME (coding->id);
  /* A new coding system might be found.  */
  if (!EQ (p->decode_coding_system, Vlast_coding_system_used))
    {
      pset_decode_coding_system (p, Vlast_coding_system_used);      /* :6425 */
      ...
      if (NILP (p->encode_coding_system) && p->outfd >= 0
	  && proc_encode_coding_system[p->outfd])
	{
	  pset_encode_coding_system
	    (p, coding_inherit_eol_type (Vlast_coding_system_used, Qnil));  /* :6442 */
```

It runs for a buffer destination (:6506) and for a Lisp filter (:6565) alike.
So the reporting slot, the process's own decode slot and -- when it was still
nil -- the process's ENCODE slot all move together, and the stickiness is a
consequence of the second write rather than a separate mechanism.

**The unconsumed source bytes.**  `coding->carryover` spans reads
(src/process.c:6243, :6455).

### What the detection actually does across a chunk boundary, measured

The obvious diagnostic shape is not diagnostic.  A first chunk of CR LF and a
later chunk of bare LF reads identically whether the type is sticky or
re-detected, because DOS decoding leaves a lone LF alone.  What separates them
is a later chunk of bare CRs.  Every row measured under GNU 31.0.90, on a PIPE
connection (see below), `coding-system-for-read` `utf-8`:

| child output | GNU text | GNU `(car (process-coding-system P))` |
|---|---|---|
| `a CRLF b CRLF` / `x CR y CR` | `(97 10 98 10 120 13 121 13)` | `utf-8-dos` |
| `a CR b CR` / `x LF y LF` | `(97 10 98 10 120 10 121 10)` | `utf-8-mac` |
| `a CRLF b CRLF` / `x LF y LF` | `(97 10 98 10 120 10 121 10)` | `utf-8-dos` |
| `abc` / `x CR y CR` | `(97 98 99 120 10 121 10)` | `utf-8-mac` |
| `a CR` / `LF b CRLF` | `(97 10 10 98 10 10)` | `utf-8-mac` |

The third row is the shape that is invisible; the first is the one that shows
the bug.  The fourth says the stickiness begins when detection FIRES, not when
the process starts: a chunk with no terminator is GNU's `EOL_SEEN_NONE`, which
skips `adjust_coding_eol_type` entirely (src/coding.c:6805), so nothing is
remembered and the next chunk still detects.

The fifth row is the one that corrects entry 134.  A CR LF split across two
reads is MANGLED by GNU: the first chunk ends on a CR, nothing holds it back,
the chunk is classified `mac` on its own, and the second chunk then inherits
`mac` and turns its LF into a second newline.  Entry 134 introduced a
trailing-CR carryover for undecided-eol codings precisely to avoid that, on the
reasoning that the coding "may still resolve to dos".  GNU does not reason that
way.  The hold-back is `eol_dos`, which every decoder computes as

```c
  bool eol_dos
    = !inhibit_eol_conversion && EQ (CODING_ID_EOL_TYPE (coding->id), Qdos);
```

(src/coding.c:1250-1251 for UTF-8, and the same two lines in seven more
decoders), and it is compared against `Qdos`, not against "might become dos".
A concrete `-dos` coding does hold the CR back -- the same split under
`coding-system-for-read` `utf-8-dos` reads `(97 10 98 10)` in GNU -- and an
undecided one does not.

### The reported name is the SUBSIDIARY, and an alias reports its base's

`adjust_coding_eol_type` takes the subsidiary out of the coding system's own eol
VECTOR, which holds canonical names, so the write-back is not a string
concatenation on the caller's spelling:

```elisp
;; a child writing  a CR b CR, read with `coding-system-for-read' `latin-1'
;; GNU => text (97 10 98 10), (process-coding-system P) (iso-latin-1-mac . utf-8-unix)
```

Nine rows of that table are pinned by
`process_output_write_back_reports_the_coding_actually_used`, including the two
that show the write-back is not only about end of line: a unibyte process buffer
reports `raw-text-dos`, because the slot records the coding the fd-level
downgrade produced (entry 131) and not the one the creation-time chain resolved;
and `binary`, whose eol type is concrete `Qunix`, still moves
`last-coding-system-used` even though it adjusts nothing.

### `call-process` reports too, and only when it read into a buffer

`Fcall_process` has the same assignment, `Vlast_coding_system_used =
CODING_ID_NAME (process_coding.id)` (src/callproc.c:913), and it sits inside the
branch guarded by an open `fd0` -- the branch that read the child's output.  So
a `(:file NAME)` or discarded destination leaves the variable alone:

```elisp
(setq last-coding-system-used 'untouched)
(call-process "sh" nil nil nil "-c" "printf 'a\\r\\n'")
last-coding-system-used
;; GNU => untouched
```

```elisp
(with-temp-buffer
  (let ((coding-system-for-read 'utf-8)) (call-process "sh" nil t nil "-c" "printf 'a\\r\\nb\\r\\n'"))
  (list (append (buffer-string) nil) last-coding-system-used))
;; GNU                => ((97 10 98 10) utf-8-dos)
;; Neomacs before fix => ((97 10 98 10) prefer-utf-8-unix)
```

Entry 134 fixed what `last-coding-system-used` reports for strings, regions and
files.  These are the two doors it did not reach.

One more thing about `Fcall_process` is load-bearing, and the first version of
this fix got it wrong.  `fd_error = fd_output` (src/callproc.c:605) unless the
caller named an error destination, so for the ordinary `DESTINATION` = `t` the
child's stderr is the SAME file descriptor as its stdout, read by the same loop
through the same `struct coding_system`.  Neomacs captures the two streams
separately and routes them one after the other; handing each route its own copy
of the decoding meant the second route -- usually decoding zero bytes of stderr
-- reported an unadjusted name and overwrote the first.  Measured against GNU,
`(call-process "sh" nil t nil "-c" "printf 'a\\r\\nb\\r\\n'")` answered
`utf-8` where GNU answers `utf-8-dos`.  The decoding is now carried by `&mut`
through both routes, so `adjust_coding_eol_type`'s rewrite reaches the second
one exactly as it reaches a subprocess's second chunk.

### The type-level fix

`adjust_coding_eol_type` does two things in one call, and Neomacs had thrown one
of them away at the type level: `EolType::for_decode` returned a `ResolvedEol`,
which can say WHICH end-of-line type the conversion runs with and cannot say
whether the coding system's NAME moved with it.  Every reporting slot in Emacs
is downstream of that second half.

```rust
pub(crate) enum DecodeEolResolution {
    Specified(ResolvedEol),   // the eol type was concrete; nothing to adjust
    Adjusted(ResolvedEol),    // the VECTOR resolved: GNU rewrites coding->id
    NotSeen,                  // GNU's EOL_SEEN_NONE: no terminator, no rewrite
}
```

`EolType::resolve_for_decode` returns one of those, and `for_decode` is now
`resolve_for_decode(..).eol()` -- kept, because the sites that convert without
reporting are honest users of the smaller answer.  The three states are GNU's
three, and `NotSeen` exists as a state of its own rather than as
`Specified(Unix)` because it is the difference between reporting `utf-8` and
reporting `utf-8-unix`.

On top of it, `decode_process_run` (`neovm-core/src/encoding.rs`) is GNU's
`decode_coding` for the one caller that must report: it spends the coding
system's eol leg before the decoder, runs the decoder, then resolves the end of
line against the text the decoder PRODUCED -- GNU's order (src/coding.c:7481),
and the only order in which the answer can be reported, because the scan has to
see the bytes the conversion will touch.

`ProcessRunCoding` (`neovm-core/src/emacs_core/process.rs`) is that answer
paired with the name it applies to, and it is what makes the write-back
unforgettable.  A read off a process fd now produces a `DecodedProcessOutputRead`,
which carries one; the ONLY way to turn that into the `ProcessOutputRead` every
driver consumes is `Context::read_process_output_recording_coding`, which is
GNU's `read_process_output_set_last_coding_system`.  There is no path from
process bytes to buffer text that does not pass through the write-back, because
the two are separated by a type rather than by a convention.  The one entry
point that deliberately skips it says so in its name:
`read_process_output_without_recording_coding`, for unit fixtures that drive a
`ProcessManager` with no `Context` to write a variable into.

`ProcessOutputDecoding::Bytes` grew the coding-system NAME for the same reason.
It converts nothing, but it still reports, and a variant that had thrown its
name away could not have said `binary`.

`process_coding_uses_dos_eol_carryover` is now `eol_dos` and nothing else --
`coding_name_eol(name) == EolType::Dos` -- with the C quoted next to it, so the
undecided case entry 134 added cannot come back as an intuition.

### Two more copies of the same decision, removed and corrected

**`load`'s private detector is deleted.**  `source_emacs_coding` /
`detect_source_eol` (`neovm-core/src/emacs_core/load.rs`) were a third port of
GNU's `decode_eol` fold, next to the shared `detected_decoded_eol` and the
`detect_eol` port that serves `detect-coding-string`.  `decode_emacs_utf8_source_lisp`
now hands `utf-8-emacs` -- whose eol type is undecided -- to the shared decoder
and lets it detect.  Entry 134 predicted this was safe "minus the
stray-^M-in-a-DOS-file case"; that qualification is wrong, and the correction is
the point of `load_source_eol_detection_matches_the_shared_decoder`, which pins
the whole truth table of the fold.  `detect_source_eol`'s `saw_lf` / `saw_crlf`
/ `saw_lone_cr` cascade answers `Dos` for exactly the mixture GNU's stray-^M rule
answers `EOL_SEEN_CRLF` for, and agrees on every other combination too, so the
deletion is a deduplication with nothing behind it -- which is the finding.

**`utf-16` and `utf-8-auto` pick a concrete BASE from the byte-order mark.**
This is a different axis from the end of line, and entry 134 named it as such.
`detect_coding` has three arms, one per "auto" category, and the two that are
not `undecided` are keyed on the coding system's `:bom` being a CONS of the two
systems the mark chooses between -- measured under GNU 31.0.90,
`(coding-system-get 'utf-16 :bom)` is
`(utf-16le-with-signature . utf-16be-with-signature)` and
`(coding-system-get 'utf-8-auto :bom)` is `(utf-8-with-signature . utf-8)`.
Each arm ends in `setup_coding_system (found, coding)` (src/coding.c:6743-6754),
so the coding system a `utf-16` decode runs with, and reports, is one of the two
concrete ones:

```elisp
(let ((d (decode-coding-string "\xfe\xff\0a\0\r\0\n\0b" 'utf-16)))
  (list (append d nil) last-coding-system-used))
;; GNU                => ((97 10 98) utf-16be-with-signature-dos)
;; Neomacs before fix => ((97 10 98) utf-16-dos)
```

Getting the NAME right required getting the BOM rule right, and the BOM rule is
the coding system's, not the bytes'.  `Utf16Bom` is GNU's `enum utf_bom_type`
with all three of its values in play, because all three are needed:

* `Without` (`utf-16le`, `utf-16be`, `:bom nil`) -- a leading signature is DATA.
* `Declared` (`utf-16be-with-signature`, `utf-16le-with-signature`, and the
  `utf-16-be` / `utf-16-le` aliases of those two) -- `decode_coding_utf_16` reads
  two bytes and consumes them only if they are the signature for its OWN
  endianness, rewinding otherwise (src/coding.c:1601-1609).
* `Detect` (`utf-16`) -- the decoder consumes NOTHING: "We have already tried to
  detect BOM and failed in detect_coding" (src/coding.c:1612-1617).  A `utf-16`
  decode consumes a signature only because the arm above re-based it to a
  `Declared` system first.

Neomacs had one bool for the three, and it sniffed the bytes rather than reading
the coding system, so it stripped a signature `utf-16le` must keep and picked an
endianness `utf-16-be` must refuse:

```elisp
(append (decode-coding-string "\xff\xfe" "a\0\r\0\n\0b\0" 'utf-16le) nil)
;; GNU                => (65279 97 10 98)
;; Neomacs before fix => (97 10 98)
```

The `utf-16-be` half of the same rule is what moved a pinned expectation; see
below.

`detect_bom_auto_coding` carries GNU's refusals as well as its detections: an
odd-length last block is rejected by `detect_coding_utf_16` outright
(src/coding.c:1501-1507) before it looks at a signature, so
`(decode-coding-string "\xfe\xff\0" 'utf-16)` reports plain `utf-16` and keeps
U+FEFF as a character; and invalid UTF-8 leaves `utf-8-auto`'s `found` nil, so
that name stays too.  Thirteen rows are pinned by
`utf_16_and_utf_8_auto_pick_a_concrete_base_from_the_bom`.

### One pinned expectation moved, and it was recording us

`encoding_utf16_gnu_compatible_signatures_and_endianness` asserted that
`utf-16-be` decodes the LITTLE-endian bytes `FF FE 3D D8 00 DE` as U+1F600.  GNU
answers `(65534 15832 222)`: `utf-16-be` is an alias of
`utf-16be-with-signature`, it declares a big-endian signature, and a
little-endian one is not it.  The row is kept, re-derived by running GNU
31.0.90, and a second row added for the same character written the way the
coding system declares it (`FE FF D8 3D DE 00` -> U+1F600), so the pin now shows
the rule rather than a byte-sniffing accident.

### Corrections to earlier entries

Entry 131, dated 2026-08-18: its second residual,
"`Vlast_coding_system_used` is not written back onto the process", is closed
here.  The paragraph is right about the mechanism and understates the blast
radius: GNU's write-back also moves the process's ENCODE coding system when that
was still nil, and the same variable is written by `call-process`, which had no
write at all here.

Entry 134, dated 2026-08-18: three of its four residuals are closed here, one of
the three carries a correction to what 134 said about it, and a decision INSIDE
134 is reversed.

* "Detection on a process read is per chunk, not per process" is closed, and its
  prediction that it "belongs together" with 131's write-back residual held
  exactly: the stickiness IS the write-back.
* "`load`'s source-EOL detector is a third copy" is closed by deleting it.  Its
  claim that the copy implements "GNU's rules minus the stray-^M-in-a-DOS-file
  case" is WRONG -- the cascade covers that case -- and the two detectors are
  equivalent on every input, which is why the deletion is safe.
* "`utf-16` with a BOM still reports the wrong BASE name" is closed.
* The trailing-CR carryover 134 added for undecided-eol codings ("One boundary
  had to move with it") is REMOVED.  GNU holds a trailing CR back only for a
  concrete `Qdos` eol type; measured, a CR LF split across two reads under
  `coding-system-for-read` `utf-8` reads `(97 10 10 98 10 10)` in GNU, not
  `(97 10 98 10)`.  The carryover made Neomacs kinder than GNU and therefore
  wrong.
* "`inhibit-eol-conversion` is inert, in both directions" is NOT closed; see
  below.

Entry 137, dated 2026-08-18: its restatement of the write-back residual is closed
with 131's.  Its instruction that "every pin in this entry therefore reads the
slot BEFORE any output arrives" stands, and is now half of a pair: 139's pins
read it AFTER, so the creation-time chain and the write-back are measured by
disjoint sets of rows.

### Measured after

Against a `cargo xtask fresh-build --release` binary, `tmp/pw46/final.el` -- ten
`make-process` write-back rows, a Lisp-filter row, eight stickiness rows, six
`call-process` rows and fourteen BOM rows, each reporting text, coding pair and
`last-coding-system-used` -- is byte-identical to GNU Emacs 31.0.90 on every row
but the four `inhibit-eol-conversion` rows recorded below.  `tmp/pw46/pin5.el`,
which pins the connection type explicitly, agrees on all five PIPE rows and
differs on the three PTY rows recorded below; `tmp/pw46/pin2.el`'s twenty-one
boundary rows agree except for the three that are the same pty difference.

`cargo nextest run -p neovm-core` is 9025/9025 green.
`cargo nextest run -p neovm-oracle-tests` is 38778/38778 green.
`cargo check --workspace --all-targets` and `cargo fmt --all --check` are clean.
The MELPA suites that carry real process bytes -- `diredfl` (the suite entry 128
came from), `rg`, `slime`, `sly`, `ivy_rich`, `all_the_icons_ivy_rich`,
`async_http_queue`, `auto_pause` and `git_rebase_mode` -- are 32/32 green.  The
one red in that run was `org_roam`, which the filter pulled in by substring and
which failed on the concurrent `sqlite3-api.so` build race entry 134 already
recorded (`cannot find sqlite3-api.o`, another suite building the same source
directory at the same time); it passes on its own, and it touches no coding
system.

### Found and NOT fixed here

**A PTY EOF drops the decoder's carryover in GNU, and does not here.**  This is
why every subprocess row above is measured on a `:connection-type 'pipe`
connection.  On a pipe, GNU's EOF read returns 0, `read_process_output` sets
`CODING_MODE_LAST_BLOCK` and decodes the carryover (src/process.c:6316-6321); on
a pty it returns -1, and the `nbytes < 0` arm returns before decoding anything,
so whatever was held back is lost:

```elisp
;; child writes  a CR  under `coding-system-for-read' `utf-8-dos'
;; :connection-type 'pipe => GNU (97 13)   Neomacs (97 13)
;; :connection-type 'pty  => GNU (97)      Neomacs (97 13)
;; child writes  a <c3>  under `utf-8'
;; :connection-type 'pipe => GNU (97 4194243)   Neomacs (97 4194243)
;; :connection-type 'pty  => GNU (97)           Neomacs (97 4194243)
```

Neomacs matches GNU's PIPE behaviour on both, and diverges on the pty because it
flushes there too.  Reproducing GNU would mean modelling "this EOF came from an
error read" and then DELETING data the child really wrote, which is a data-losing
change to make deliberately or not at all; it is recorded rather than made.
`process-connection-type` defaults to `t`, so this is the common case, and it is
the reason a pin taken without `:connection-type 'pipe` measures the pty quirk
instead of the coding chain.

**`inhibit-eol-conversion` is still inert, in both directions.**  Entry 134's
residual, restated with what this entry learned about its cost.

```elisp
(let ((inhibit-eol-conversion t))
  (list (append (decode-coding-string "a\r\nb\r\n" 'utf-8-dos) nil)
        (append (encode-coding-string "a\nb" 'utf-8-dos) nil)
        (append (decode-coding-string "a\r\nb\r\n" 'undecided) nil)
        last-coding-system-used))
;; GNU     => ((97 13 10 98 13 10) (97 10 98) (97 13 10 98 13 10) undecided)
;; Neomacs => ((97 10 98 10)       (97 13 10 98) (97 10 98 10)    undecided-dos)
```

The fourth element is new evidence about its shape: the variable does not only
suppress the conversion, it suppresses the NAME ADJUSTMENT, because
`decode_eol`'s first line returns before the VECTOR branch (src/coding.c:6765).
So it is a flag on the RESOLUTION, which is now exactly two functions here --
`EolType::for_encode` and `EolType::resolve_for_decode` -- and the honest shape
is to make it a required parameter of both, so that no site can resolve without
having been told.

It is left out again, and the reason is not that it cannot be reached -- that
was checked, and it can.  The resolution is reached through `decode_eol_text`,
`decode_bytes`, `decode_bytes_emacs`, `encode_lisp_string` and `encode_string`,
context-free by design and called from eleven places in seven files (plus
fifty-one test call sites).  Every one of the eleven turns out to have a
`Context` in reach, including the one that looks like it does not:
`keyboard_input.rs`'s TTY decoder is driven from
`Context::handle_read_char_input_event` (`neovm-core/src/keyboard.rs:4580`).  So
the honest version is buildable.

What is NOT available is GNU's own mechanism.  `inhibit_eol_conversion` is a C
global behind `DEFVAR_BOOL`, read directly by eight decoders and by
`decode_eol`; the Neomacs analogue would be a process-wide cell, and there is
none to be had -- the obarray is owned by a `Context`, `LispBoolFwd` cells are
copied per thread (`neovm-core/src/emacs_core/forward.rs`), and the unit suite
runs many `Context`s on parallel threads, so a static would be read by the wrong
session.  The flag therefore has to travel as a value, and the shape that makes
it unforgettable is a required parameter of `EolType::for_encode` and
`EolType::resolve_for_decode` -- the two functions this entry has just made the
only ways to resolve an eol type -- with the un-parameterised spellings kept
`#[cfg(test)]` so production code cannot reach them.

That is a mechanical change to about seventy call sites, and it is a change
whose blast radius is every coding conversion in the editor.  Putting it behind
four measured behaviour fixes in one gate run is how a regression gets attributed
to the wrong half.  It lands in its own entry, with the design above already
settled and the fourth measurement in the block above -- that the variable
suppresses the NAME ADJUSTMENT as well as the conversion -- as its first pin.

Status: FIXED.

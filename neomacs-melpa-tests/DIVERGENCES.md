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
timeout or retry. Until then this stays a known GNU-side environmental flake,
recorded here rather than hidden behind a retry.

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

## 67. `normal-erase-is-backspace` latches OFF before the terminal's ERASE character is known -- OPEN

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

That isolates the fault precisely: the minor mode, `key-translate`, the
`keyboard-translate-table` population and the input path that honours it are
ALL correct. Only the moment of the decision is wrong -- it happens while the
terminal's ERASE character is not yet the real one, and the guard then makes
that first answer permanent.

Still open: WHERE the premature decision happens. Clearing `terminal.params` in
`configure_terminal_runtime` (neovm-core terminal/pure.rs) was tried and does
NOT fix it, so the stale parameter is not reaching the live terminal through
that path; that change was reverted rather than kept on speculation. GNU's
ordering to match is `init_sys_modes` publishing `c_cc[VERASE]`
(src/sysdep.c:1130) strictly before `command-line` calls
`normal-erase-is-backspace-setup-frame` (lisp/startup.el:1638).

Reduction: `cargo nextest run --release -p neomacs-tui-tests -E
'test(backspace_on_a_ctrl_h_erase_terminal_deletes_like_gnu)'`. The case is
committed `#[ignore]`d against this entry so the reduction is preserved without
a red suite; remove the attribute when fixing.

Status: OPEN.

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


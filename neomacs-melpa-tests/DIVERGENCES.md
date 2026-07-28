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
`target/release/neomacs`). The two that cannot be a one-liner — entry 15, an
intermittent segfault, and entry 9, which needs two files on disk — are marked
where they appear. The script that runs them all is `tmp/verify-divergences.sh`.

Entry 15 is a **memory fault**, not a behavioural difference; read it first if
you are triaging by severity.

Reproduce a failing suite with:

```sh
RUST_LOG=warn NEOMACS_BIN="$PWD/target/release/neomacs" TMPDIR="$PWD/tmp" \
  cargo nextest run -p neomacs-melpa-tests -E 'test(~parity_tests::<pkg>::)' --no-fail-fast
```

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

utf-8 and utf-8-unix are written correctly. Affects: `aa-edit-mode` (1).

## 6. `directory-files` returns undecoded bytes

File names are not decoded through `file-name-coding-system`, so a non-ASCII
name comes back as raw bytes and sorts differently.
`directory-files-recursively` inherits this.

```elisp
(directory-files DIR)  ; DIR contains "Lösung.pdf"
;; GNU     => ("." ".." "Lösung.pdf" …)          (multibyte-string-p => t)
;; Neomacs => ("." ".." "L\303\266sung.pdf" …)   (multibyte-string-p => nil)
```

Affects: `abgaben` (1).

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
(face-differs-from-default-p 'neomacs-probe-alias-face)
;; GNU     => t
;; Neomacs => (error "Invalid face" neomacs-probe-alias-face)
```

Affects: `abridge-diff` (1), via `smerge-refine`'s use of the obsolete
`smerge-refined-change` alias. Every `define-obsolete-face-alias` in the Emacs
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

## 16. `real-last-command` is never updated

`last-command` is correct; `real-last-command` stays nil forever. GNU sets it
from the previous iteration's `real-this-command` each time round the command
loop (src/keyboard.c:1354 and 1580).

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

## 21. A refused connection reports a different error

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
the network-process path rather than to error formatting generally. Affects:
`adafruit-wisdom` (1).

---

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

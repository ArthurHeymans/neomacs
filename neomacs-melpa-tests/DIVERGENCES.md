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

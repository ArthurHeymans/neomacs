# Traps when writing MELPA parity workflows

Things that cost someone an hour, or silently produced a wrong test, while
converting the parity suites. `tmp/neomacs-melpa-tests-standards.md` says what a
good suite is; this says how to avoid getting a bad one by accident.

The dangerous ones are marked **SILENT** — they do not fail, they produce a
passing test that asserts the wrong thing.

---

## Snapshots

**SILENT — `UPDATE_EXPECT=1` under parallel nextest loses updates.** Each test
is its own process patching the same `workflows.rs`; last writer wins, and some
expectations keep their old value while the run reports success. Use
`--test-threads=1` for the UPDATE_EXPECT pass.

**SILENT — serialising is necessary but not sufficient.** `expect-test` records
each macro's position at *compile* time, so once an earlier test's update
rewrites the file, a later test in the same run panics with "Failed to parse
macro invocation" and its snapshot is silently left at the old value, while the
run reports a failure that looks unrelated. The recovery that works: replace the
stale literal with an empty `expect![[r#""#]];` by hand and re-run. After any
UPDATE_EXPECT pass, check that every snapshot you expected to change actually
changed.

Refinement from a second encounter: a literal holding *stale text* may not clear
by re-running, but an **empty** literal does. One suite hit the cascade on 5 of 6
tests with empty literals already in place and a single further pass filled all
five. So if your expectations are still empty, just re-run; hand-editing is only
needed to clear stale text.

**SILENT — `#N=` back references over *strings* are flaky by construction.**
Two equal strings that happen to be the same object print as a back reference
under `print-circle`, and whether that sharing survives the oracle's normaliser
is not stable: the same workflow has produced both the shared and the unshared
form from GNU Emacs on different runs. Have every test helper `copy-sequence`
the strings it returns, so nothing can print as a back reference. This is why
"transcribe from a harness run" is necessary but not sufficient.

Sharing of *conses* is different and safe to pin: it is structural, not
incidental. The `a` suite deliberately pins `#1=`/`#1#` markers because a.el's
"immutable" operations really do share alist tails, and that suite is stable
across repeated runs. The rule is: pin sharing you can explain from the
package's data structure, never sharing that merely happens between two equal
strings.

**Transcribe expectations from a harness run, not a raw probe.** The oracle's
normaliser breaks string identity, so probe output containing `#1=` sharing
markers never matches. Applies whenever a value repeats.

**`copy-tree` / `copy-sequence` state you capture mid-workflow.** Packages share
cons cells between their variables (amx's `amx-data` and `amx-cache` do), so a
snapshot taken early can render the *final* values under a `#1=` marker and look
self-contradictory beside something captured at the same moment.

## Driving the editor in batch

**`execute-kbd-macro` only reaches the buffer of the selected window.**
`set-window-buffer` first; `with-temp-buffer` alone is not enough and the keys
land in `*scratch*`.

**SILENT — `transient-mark-mode` is nil in batch.** A region set with `C-SPC` is
never active, so any command branching on `use-region-p` takes its no-region
path. adoc-mode's styling commands insert an empty `**` pair instead of
unwrapping. Enable the mode explicitly in the workflow.

**SILENT — `sit-for` / `sleep-for` do not run idle timers in batch.** There is no
command loop to notice idleness, so a workflow that types and then waits
observes nothing. Run the due entries of `timer-idle-list` instead.

**Running timers needs a captured baseline, not a function match.** The editor
has its own pending timers (`undo-auto--boundary-timer` is live in every batch
run), so "run everything in `timer-list`" runs editor internals too. Capture
`timer-list` before the trigger, run only what appeared, and pin how many
appeared. Matching on the timer's printed function is the tempting alternative
and it is editor-specific — Neomacs need not print a closure the way GNU does.

**SILENT — a timer's delay is only assertable as a delta.** Capture a timestamp
immediately before the trigger and assert
`(round (* 10 (- (float-time (timer--time timer)) start)))`, which gives tenths
and is stable. Pinning `timer--time` itself pins wall-clock: it passes locally
forever and means nothing.

**SILENT — one baseline per *call*, not per workflow.** A delta measured from a
timestamp captured before several calls absorbs the time the earlier calls took,
so a slower editor fails on arithmetic rather than behaviour. This produced
three false failures in one suite, all blamed on Neomacs, because the earlier
calls wrote a file. Take a fresh timestamp immediately before each triggering
call, and prefer whole seconds — minutes for anything read back from a file that
stores truncated timestamps.

**A truncated timestamp cannot be asserted in seconds against a sub-second
baseline** — the delta straddles a rounding boundary. Assert in minutes, or
against the in-memory record the value was written from.

**Arbitrary literal text as keys:** `(vconcat (kbd "C-c a") (string-to-vector
"some text") [?\r])`. `kbd` swallows spaces (use `SPC`) and cannot express `[`.

**Log probe output to a file with `write-region`, not `princ`.** `princ` output
is buffered and lost when a hung probe is killed, so a hang looks like "no
output at all" and you cannot tell which form blocked.

## Test doubles

**Check `executable-find` before building fixtures.** ac-php cost an agent an
hour because its indexer needs a PHP interpreter that does not exist here. An
environment blocker is a legitimate answer — report it rather than working
around it.

**Protocol → implement it. Data format → blocked.** When the boundary is a
documented wire protocol, stand up the counterparty for real: ac-sly had no
Common Lisp, so the suite ran a `make-network-process :server t` speaking
slynk's framing and connected with the real `sly-connect`. When the missing
program generates the package's *own on-disk format*, standing it in means
authoring the package's data structure — the standards' "reimplementing the
package algorithm inside the expected value" anti-pattern.

**comint/REPL stand-ins need two things.** The response shape the package parses
— inf-ruby does `(butlast (split-string kept "\r?\n") 2)`, so print completions,
then a result line, then the prompt — and the *initial* prompt drained before
the call, or it is already in the accumulator when the package installs its
filter, the wait loop exits immediately, and you get nil candidates with no
request ever sent. Also `inhibit-field-text-motion` for prompt-matching helpers,
and `set-process-query-on-exit-flag` nil so `kill-buffer` does not prompt.

**Make async deterministic.** Wait on the sentinel or `accept-process-output`
until the process is dead. Sort concurrently recorded requests before asserting
— activity-watch's bucket and heartbeat curl processes finish in either order.

## Assertions

**SILENT — check the package is not silently a no-op in batch.** Several
packages have a path that quietly does nothing on a non-graphical or
noninteractive Emacs, and a suite that does not notice asserts the no-op while
claiming to cover the feature — passing in both editors. Four found so far:

| package | gate | effect in batch |
|---|---|---|
| `all-the-icons-ibuffer` | `all-the-icons-ibuffer-display-predicate` defaults to `display-graphic-p` | icon column renders empty |
| `ada-ts-mode` | `treesit-ready-p` | falls back to a non-treesit path |
| `activity-watch-mode` | its own `noninteractive` guard | refuses to switch on |
| DDSKK (via `ac-skk`) | `(unless noninteractive …)` in `skk-save-jisyo` | no dictionary is written |

Assert the gate itself in its own workflow, both open and closed, so the suite
says which path it exercised.

**SILENT — a fixture must be able to tell the two answers apart.** Four buffers
under 1k made `file-size-human-readable` return the same digits as the raw size,
so the human-readable setting would have been asserted with a fixture that could
not distinguish it. A 2048-byte buffer reads `2k` against `2048`. Same principle
as making each element of an alignment fixture wrong by a different amount.

**SILENT — do not take the head of a package's list to mean "the item I just
added".** If the command re-sorts, the head is whichever entry sorts first.
`alarm-clock-set` calls `alarm-clock--list-prepare`, which sorts, so three
workflows agreed with themselves and reported the same alarm three times. Look
the record up by a field you set.

**Pin deltas, not absolute counts,** for anything editor-wide. amx's
`amx-detect-new-commands` returns a total command count; only the +1 per newly
defined command is portable. Same reasoning: keep a package's history length
small enough that the fixture fills it entirely, or real editor commands leak in.

**Themes with a `min-colors` clause cannot be asserted by resolved appearance.**
A batch frame is a 0-colour `mono` display, so the clause matches nothing and
`face-attribute … nil t` is `unspecified` for every themed face in *both*
editors. `display-color-cells` is 0 and neither `tty-color-mode` nor
`set-terminal-parameter` moves it. Pin the registered spec instead — exact
colour strings plus the display clause — and record the display facts with
`face-spec-set-match-display` so the reason is on the record. Themes using
`((t …))` (abyss, acme) are unaffected.

**Before concluding a theme is unassertable, check whether its display class is
itself a customization.** `alect-themes` reads its clause from
`alect-display-class`, which defaults to `((type graphic))` — unsatisfiable in
batch however many colours the display claims — but documents nil, "All
terminals", as a supported value, and a nil clause matches a batch display. So
that family is assertable by *resolved* appearance with no faking at all: pin
the stock graphical-only behaviour once, then set the documented option and read
real colours back through `face-attribute … nil t`.

**SILENT — compute fixture positions, do not eyeball them.** An afterglow
workflow whose fixture had a deliberately empty line was one character short, so
the empty-line guard case exercised a *non-empty* line and asserted an overlay
where the entire point was to assert none. It passed in both editors and looked
right. Same shape as the `transient-mark-mode` trap: a green test asserting the
opposite of its own name.

The fix for the *class*, not just the fixture: **assert text, not arithmetic.**
Make every assertion whole-buffer text rather than a column or offset, and make
each element of the fixture wrong by a *different* amount. Then a miscomputed
position shows up as text and cannot be hidden by an arithmetic mistake shared
between fixture and expectation. align-cljlet's suite is built that way.

**If an indenter consults faces, assert both with and without
`font-lock-ensure`.** actionscript-mode's `as3-count-scope-depth` decides whether
a brace counts by looking at its face, so the indenter only works on a fontified
buffer — a suite that always fontifies would silently depend on that.

## Reducing a divergence

**Re-run a candidate reduction on its own before reporting it.** Some bugs
damage process-wide state — catalogue entry 25 leaves `mode-name` void
everywhere — so a probe file that runs the real trigger first and the candidate
second sees the candidate fail against already-broken state and blames the wrong
form. This is a property of the bug, not carelessness; it caught a careful agent.

**Prefer designing a known divergence out of reach over citing it.** When a
catalogued divergence is not what the package under test is about, build the
suite so it cannot be reached: `all-the-icons-completion` goes through
`completion-metadata-get` — the package's real route, which a UI calls when it
renders candidates — so entry 11 never arises without a minibuffer anywhere, and
passes explicit buffer candidates so entry 13's ordering cannot reach an
assertion. `all-the-icons-ibuffer` does the same with prefixed fixtures and
name-sorted assertions. Citing after the fact leaves a red test that says
nothing new; designing it out leaves a green test that covers the package.

**Assert a private-use glyph as character codes *and* its font family.** Code
lists stay readable and diff-friendly in a snapshot where a raw glyph does not,
but codes alone are not enough: an unknown extension and a directory can resolve
to the *same code point in different fonts*, which a code-only assertion calls
equal.

**Cite an existing catalogue entry rather than re-witnessing it.** Every red test
should be a distinct problem, or the failure count stops carrying information.

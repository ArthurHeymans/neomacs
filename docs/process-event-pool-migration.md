# Process / Network / Event-Pool Migration — Status & Remaining Work

Last updated: 2026-07-02. Owner: oracle-parity effort (see `git log` commits
referenced below). Method and evidence for every claim: live-GNU oracle probes
(`emacs --batch -Q` vs `./target/release/neomacs --batch -Q`), GNU source study
(`process.c`, `keyboard.c`, `timefns.c` in the emacs-mirror checkout), and the
neovm-oracle-tests suite in live mode.

This document is the durable, verbose record of the audit's divergence catalog
(D1–D14), the migration slices (S1–S10), what has landed, what is in flight,
and exactly how to execute the remaining slices. A scratch copy of the original
audit lives in `drafts/process-event-pool-audit.md` (gitignored by repo
convention); this file supersedes it.

---

## 1. The GNU model (the contract being matched)

GNU Emacs runs **one synchronous loop** — `wait_reading_process_output`
(process.c:5336) — as the only place where process I/O, filters, sentinels,
timers, and async-connect completion ever execute. Per iteration, in order:

1. `maybe_quit` / pending-signal processing.
2. **Deadline check — break BEFORE servicing timers** (process.c:5469-5478).
3. Async DNS (`getaddrinfo_a`) / TLS / `:nowait` connect completion →
   `connect_network_socket`.
4. `timer_check` (keyboard.c:4911) unless `just_wait_proc < 0`: copies
   `timer-list`/`timer-idle-list` (so timers can reschedule themselves), fires
   ALL ripe timers one at a time via the Lisp `timer-event-handler`
   (re-looping `timer_check_2` while it returns `{0,0}`), returns the delta to
   the next timer.
5. `status_notify` if `update_tick != process_tick`.
6. `pselect` with timeout = min(remaining-deadline, next-timer-delta). No
   polling cap; wakes exactly on readiness.
7. Server accepts — `server_accept_connection` creates the connection process,
   calls the `:log` function, and `exec_sentinel(proc, "open from HOST\n")`
   **immediately, same iteration** (the only push-style sentinel in GNU).
8. `read_process_output` per ready fd, round-robin: chunk =
   `read-process-output-max`, decoding carryover in `p->decoding_buf`,
   adaptive read buffering (delay ladder: increment ≈10ms, max ≈50ms), filter
   runs synchronously (current buffer = process buffer, `running_asynch_code`,
   match-data save/restore). EOF ⇒ status transition + `tick++`.
9. Keyboard check; loop.

**Status pipeline, not an event queue.** Every transition writes `p->status`
and bumps `p->tick`. `status_notify` (process.c:7873), for each process with
`tick != update_tick`: drains remaining output FIRST (the drained bytes feed
the wait's `got_some_output`), derives the sentinel message from the
**current** status via `status_message(p)` — so transitions that happen within
one wait **collapse into a single sentinel with the final message** — then
deactivates/removes terminated processes BEFORE `exec_sentinel`.

Key contract points verified empirically this cycle:

- **`got_some_output` counts read bytes only** (process.c:5588/6018 and the
  status_notify drain). Connect completions, accepts, sentinel runs, and EOFs
  are *serviced* inside the wait but never complete it; `accept-process-output`
  returns `t` only when actual output was read.
- **A WAIT_PROC wait breaks when the target's INTERNAL status is neither
  `Qrun` nor a pending connect** (the `!EQ (wait_proc->status, Qrun) &&
  !connecting_status` drain-then-break). GNU-internal statuses differ from the
  `process-status` projection: a listen server is stored `Qlisten` (so
  `accept-process-output` on a server returns immediately — verified), while a
  connected netconn is stored `Qrun` (projected to `open` by
  `Fprocess_status`), and an io-paused connection (`stop-process` on a
  netconn: `p->command = Qt`) stays internally `Qrun` and does NOT break the
  wait even though `process-status` projects `stop`.
- **Blocking (non-`:nowait`) client connects fire NO sentinel** —
  `connect_network_socket` contains zero `exec_sentinel` calls. Only the
  deferred `:nowait` completion path delivers `"open\n"` / `"failed with code
  N\n"`.
- **Peer EOF on a running network connection sets `(exit . 256)`**
  (process.c:6090); `status_message` maps network exit 0 → `"deleted\n"` and
  non-zero → `"connection broken by remote peer\n"`.
- **Observation points decode pending child status**: both `Fprocess_status`
  and `Fprocess_exit_status` run `update_status` when `raw_status_new` is set
  (raw status arrives asynchronously via SIGCHLD). So `process-status`
  immediately after a wait reports `exit` even though the sentinel
  notification is still pending for the next `status_notify`.
- **Timers live in timer.el.** C keeps only the `timer-list`/`timer-idle-list`
  variables, `timer_check`, and `current_timespec` (nanosecond resolution:
  `PSEC = (ns % 1000) * 1000`). `run-at-time`, `timer-activate`, sorted
  insertion, retriggering at **old time + repeat**, and `timer-max-repeats`
  catch-up are all Lisp.
- **`send_process` can re-enter the wait**: on EAGAIN it queues the remainder
  in `p->write_queue` and calls `wait_reading_process_output(0, 20ms, …)`, so
  filters can run during `process-send-string` (documented cascading).
- **`stop-process` semantics split**: network/serial/`make-pipe-process`
  connections → pause reading (`p->command = Qt`), status untouched; real
  subprocesses (any `:connection-type`) → `process_send_signal(SIGTSTP)` with
  **status untouched** — status only changes if `waitpid(WUNTRACED)` later
  *observes* a stop. (Verified: an sh child ignores SIGTSTP — kernel state
  stays S — and GNU keeps reporting `run`.)

## 2. Divergence catalog (D1–D14) and disposition

| # | Divergence | Status |
|---|---|---|
| D1 | `current-time` PSEC always 0 (µs truncation in `TimeMicros::now`) | **FIXED** `497350bff` |
| D2 | Native timer "second brain" (`TimerManager`, unregistered `run-at-time`/`timer-activate` builtins, `now+interval` rescheduling) | **DELETED** `5829b6f11` (was dead code: no live writers; the real brain was already Lisp timer-list + `timer-event-handler`); the real psec bug was `decode_lisp_time`'s nil branch — **FIXED** `0dd82abd1` |
| D3 | Wait loop serviced ripe timers after a wake even when the deadline had elapsed | **FIXED** `bf71ed726` (deadline-first, GNU loop order). Note: the exact-boundary fire-count (cx64/cx179/cx338) is a jitter race **inside GNU itself** (10 idle GNU runs: `21 20 20 20 20 20 20 20 20 21`); classified EXPECTED-TIMING |
| D4 | Sentinel messages stored at event time instead of derived from current status at notify | **PARTIALLY FIXED** in `d02f31edb` (EOF path derives via `gnu_process_status_message_for_process`); remaining audit pass = S4c |
| D5 | Spurious `"open\n"` sentinel for blocking client connects | **FIXED** `d02f31edb` |
| D6 | Network EOF set `(exit . 0)`; "connection broken" delivered rounds late; accepts/EOF treated as wait-completing activity | **FIXED**: exit-256 + derived message + reap in `d02f31edb`; completion semantics in S4b (in tree, see §4) |
| D7 | `stop-process` eagerly sets status `stop` for real subprocesses | **OPEN — S5** |
| D8 | seqpacket not advertised in `featurep 'make-network-process` though fully implemented | **FIXED** `69b874f9b` |
| D9 | Child exit observed only inside wait iterations; `process-status` stale outside waits (the load-flaky family's root cause) | **IN TREE — S6** (see §4) |
| D10 | Fixed 4096-byte reads; no `read-process-output-max`; no adaptive read buffering | **OPEN — S7** |
| D11 | `process-send-string` = `write_all`+`flush`; never re-enters the wait; `write_queue` field exists unused | **OPEN — S8** |
| D12 | DNS always blocking, even for `:nowait` | **OPEN — S9** |
| D13 | 50ms polling cap per wait iteration instead of exact timeouts | **OPEN — S9** |
| D14 | Lisp threads: mutex ownership error where GNU blocks; dynamic `let` leaks across threads; `all-threads` misses blocked workers; `thread-signal` handler detail | **OPEN — S10** (separate subsystem) |

## 3. Landed slices (all pushed to main)

| Commit | Slice | Content |
|---|---|---|
| `497350bff` | S1 | `TimeMicros::now` uses `subsec_nanos`; `PSEC = (ns%1000)*1000` like GNU `Ftime_convert`. Fixed `relative/repeating_timer_microseconds`. |
| `69b874f9b` | S2 | `(:type seqpacket)` added to `make_network_process_subfeatures` (verified end-to-end vs GNU first: local seqpacket server+client+data byte-identical). |
| `0dd82abd1` | S3 | `decode_lisp_time` nil branch keeps the ns remainder (`ticks += psecs/1000`) — timer vectors built by timer.el's `timer--time-setter` now carry PSEC. Also normalized `div_u5_timer_create_cancel_reorder` to be clock-independent (raw `memq` tails embedded wall-clock timer vectors that can never match across processes); expectation regenerated and verified **via GNU** (refresh mode). |
| `bf71ed726` | S3 | `run_timers` flag threaded through the post-block service pass, false when the deadline elapsed at wake — GNU's loop-top deadline-break-before-`timer_check` order. Ready fds are still drained at the deadline (GNU reads the final pselect's fds). |
| `5829b6f11` | S3b | Dead native `TimerManager` deleted (timer.rs 695→~100 lines, keeping only `sleep-for`, which is C in GNU too). `eval.rs` field, the wait loop's empty-Vec fire pass, and keyboard.rs's always-None `next_fire_time` merge removed. Ten bare-Context unit tests rewritten to push real GNU timer vectors onto `timer-list` with a timer.el-shaped `timer-event-handler` stub — they now exercise the production dispatch path. |
| `d02f31edb` | S4a | D5 + D6a: no sentinel for sync connects (two creation sites removed); network EOF → `(exit . 256)` + sentinel text derived from status + reap per `delete-exited-processes`. Both network probe scripts diff byte-identical vs GNU (normalize ephemeral ports). The unit test that pinned creation-time `"open\n"` was retargeted to the `delete-process` `"deleted\n"` sentinel (same state-preservation assertions) with the sync-connect silence locked in by an explicit nil assertion; its expectation verified against live GNU. |

Oracle trajectory: pre-audit clean baseline 131 fails → **121** after S1–S4a
(all targets fixed: seqpacket, u5, both microseconds tests, cx179,
`network_client_open_delete_sentinels`), zero regressions across five clean
idle full-suite gates.

## 4. IN FLIGHT — S4b + S6 (in tree, uncommitted at time of writing)

These two interlock: S4b (only output completes waits) removes the hazard that
previously made S6's observation-time polling unsafe (a poll that parked a
pending status used to complete waits spuriously via the status-notification
activity path).

**S4b — completion semantics** (`process.rs`, `wait.rs`):

- `ProcessOutputServiceOutcome` gained a `serviced` flag alongside the
  completing `activity`; `record_serviced()` marks non-output servicing.
- Reclassified sites (all in the wait's service pass): `:nowait` connect
  completion ×3 (Retrying/Connected/Failed), server accepts, status
  notifications ×2, stderr-pipe terminal EOF, network EOF → serviced.
  Data reads (×3 sites) remain the only completing activity.
- `run_process_status_notification` now returns `(drained_output, notified)` —
  GNU `status_notify`'s return counts DRAINED bytes into `got_some_output`, so
  the drain completes waits while the sentinel run only services.
- New `WaitCompletion::TargetProcessTerminated`: a targeted wait
  (`accept-process-output PROC`) returns nil immediately when the target is
  missing (reaped) or `process_status_ends_target_wait` — the GNU-internal
  rule mapped onto neomacs's storage model:
  - stored `run` for a **network server** ends the wait (GNU stores `Qlisten`
    internally; neomacs stores `run` + projects via `process_contact_server_p`);
  - stored `run`/`open`/`connect` for anything else keeps waiting (io-paused
    connections included: pause is a separate flag, GNU-internal status stays
    `Qrun`);
  - `listen`/`stop`/`exit`/`signal`/`failed`/`closed` end the wait.
- Verified probes (byte-parity with GNU): listen-wait returns immediately
  `(nil t)`; open connection waits full timeout; io-paused connection waits
  full timeout with public status `stop`; sentinel-only activity returns nil;
  `:nowait` connect completes without terminating its wait `(nil t ("open")
  open)`; both network sentinel scripts byte-identical.

**S6 — observation points** (`process.rs`):

- New `process_effective_status(process)`: the GNU `update_status` view —
  decode `pending_terminal_status` when `status_notify_pending`, else stored
  status. Used by `process_public_status_symbol`, `process_live_status_value`
  (whose deliberate pending→`run` masking branch is removed), and
  `process-exit-status`.
- `builtin_process_status_impl` and `builtin_process_exit_status_impl` now
  poll `check_child_exit(id)` (non-blocking `try_wait`) before reporting —
  neomacs's equivalent of GNU's asynchronous SIGCHLD `raw_status_new`, decoded
  at exactly GNU's two observation points. The sentinel notification stays
  with the wait loop (tick/update_tick split preserved).
- Rationale for reversing the old "do not probe the OS here" comment: that
  comment guarded against the pre-S4b world where a status change completed
  waits. S4b removed that coupling.

**State at pause/handoff:** all of the above compiles (`cargo check --tests`
clean); a release build + verification run covers: the S6 target test
(`div_v8_process_attributes_status_type_tty` — GNU reports `(exit signal)`
membership + reaped process-list where neomacs reported `run` + live entry),
the flipped flaky family ×3 runs, the S4b probes, and a fresh
`status-after-exit` probe. Remaining before push: unit-test suite for
process/wait, oracle family gate, **clean idle full regression** (see §6
methodology), commit (S4b and S6 as separate commits), push.

## 5. REMAINING SLICES — detailed execution guidance

### S4c — finish derive-at-notify (D4 residue)

Sweep for any remaining sentinel deliveries that pass a stored/hardcoded
message where GNU derives from current status at notify time. Known clean:
accepts keep their literal `"open from HOST\n"` (GNU calls `exec_sentinel`
directly at accept — the one push-style sentinel); `:nowait` completion keeps
literal `"open\n"`/`"failed with code N\n"` (GNU wait-loop literals too).
Audit `run_process_sentinel_callback` call sites; anything reporting a
terminal state should route through `gnu_process_status_message_for_process`.
Watch the `AcceptedNetworkConnection.sentinel_message` field (process.rs
~1051): with accepts staying literal it is acceptable, but consider deriving
at delivery for uniformity.

### S5 — signal-send semantics (D7)

- GNU `Fstop_process`/`Fcontinue_process` (process.c): for
  network/serial/pipe-**connection** objects, set/clear `p->command = Qt` and
  add/remove the read fd — status untouched. For real subprocesses,
  `process_send_signal(SIGTSTP/SIGCONT, current_group)` — **status untouched**;
  only `waitpid(WUNTRACED|WCONTINUED)` observation may set `stop`/`run`.
- neomacs today: eagerly sets status `stop` on `stop-process` for real
  children (u1 diverges: GNU `run`, NEO `stop`; the child never actually
  stopped — kernel state S).
- Implementation: find the native stop/continue/interrupt/quit/kill process
  builtins; remove eager status writes for real children; ensure
  `poll_child_exit_status` uses `WUNTRACED|WCONTINUED` semantics
  (`try_wait` cannot see stops — this needs `waitpid(pid, &st, WNOHANG |
  WUNTRACED | WCONTINUED)` via libc on unix, or keeping the current behavior
  for stop-observation with a documented gap). Check what
  `process_send_signal`-equivalent does about process groups (GNU signals the
  pgrp for pty children, the pid for pipe children).
- Oracle target: `div_u1_process_signal_combo` (`(stop . run)` expected).
- Gate: probe `tmp/t_stop.el` pattern (kernel-state assertion) + family +
  full regression.

### S7 — read-process-output-max + adaptive read buffering (D10)

- GNU: `p->readmax` from `read-process-output-max` (default 65536 in 29+,
  4096 historically — check the 31.0.90 default in process.c), carryover
  buffer for partial multibyte sequences, adaptive delay:
  `READ_OUTPUT_DELAY_INCREMENT` = TIMESPEC_HZ/100 (~10ms), MAX = ×5,
  MAX_MAX = ×7; if a read returns < 256 bytes, delay += 2×increment; if a
  full `readmax` chunk, delay -= increment; `process_output_delay_count` and
  `read_output_skip` gate an early pselect timeout (process.c:5679-5710,
  6283-6307).
- neomacs today: fixed 4096-byte reads (process.rs read paths), no defvar
  wiring, `process-adaptive-read-buffering` accessors exist but no engine.
- Implementation: wire the `read-process-output-max` defvar into the read
  chunk size per process (snapshot at process creation like GNU's
  `p->readmax`); implement the delay ladder in the wait loop's fd-servicing
  decision. This changes output CHUNKING visible to filters — gate carefully
  against `div_cx45_process_env_coding_narrow_output_filter_hash_mega` and
  the filter-chunk family.

### S8 — send_process write queue + re-entrant wait (D11)

- GNU `send_process` (process.c:6712): loop writes; on EAGAIN push the
  remainder onto `p->write_queue` and call
  `wait_reading_process_output(0, 20*1000*1000 ns, 0, 0, Qnil, NULL, 0)` —
  filters/sentinels can run during `process-send-string`. EPIPE ⇒ status
  `(exit . 256)`, deactivate, signal error. Datagrams use `sendto` whole.
- neomacs today: `process_send_string` does blocking `write_all` + `flush`
  (process.rs ~224) — can deadlock when the child's stdin pipe is full and
  the child is itself blocked writing output (nobody drains). The
  `write_queue` field exists but is unused.
- Implementation: non-blocking writes on unix (`O_NONBLOCK` on the child
  stdin fd / socket), EAGAIN → queue + bounded re-entrant wait; the wait's
  writable-fd path already exists for `:nowait` connects — extend to flush
  write queues when fds become writable.
- Risk: re-entrancy — filters running inside `process-send-string` is a GNU
  contract but new for neomacs callers; gate broadly (jsonrpc/eglot flows in
  TUI tests).

### S9 — async DNS + exact timeouts (D12, D13)

- `:nowait` with a hostname should not block on `getaddrinfo`. GNU uses
  `getaddrinfo_a` when available and polls completion in the wait loop
  (process.c:5410-5431, `check_for_dns`). Rust: resolve on a `std::thread`
  with the result delivered through the poller's notify (the one legitimate
  auxiliary thread — it never touches Lisp; the wait loop applies the result
  inside its iteration, matching GNU's model).
- Drop the 50ms polling cap (wait.rs `base_timeout`): timeout should be
  exactly min(deadline remaining, next-timer delta, ∞) — the poller already
  wakes on fd readiness and `Poller::notify()`. Audit the cap's current
  consumers first (GUI frame scheduling may rely on periodic wakes; if so,
  scope the exact-timeout change to `--batch`/no-display waits first).

### S10 — Lisp threads (D14) — separate subsystem effort

- GNU model (`thread.c`): every Lisp thread is an OS thread holding THE
  global lock while running Lisp; switches happen only at blocking points
  (`thread_select` inside `wait_reading_process_output`, `mutex-lock`,
  `condition-wait`, `thread-yield`, `sleep-for`). Dynamic bindings are
  swapped on context switch: `unbind_for_thread_switch` /
  `rebind_for_thread_switch` walk the outgoing/incoming thread's specpdl.
- Four deterministic oracle divergences:
  1. `mutex_lock_blocks_other_thread` — NEO errors "Cannot unlock mutex owned
     by another thread" where GNU's second thread blocks parked on the mutex.
  2. `thread_dynamic_binding_isolation` — a `let` in one NEO thread is
     visible in another (GNU: `nil`, NEO: `(main "*scratch*" local)`); needs
     the specpdl swap.
  3. `all_threads_includes_live_worker` — a worker blocked in `mutex-lock`
     must appear in `all-threads` with `thread-live-p` t.
  4. `thread_signal_condition_handler` — `(void-variable log)` propagation
     detail in the handler path.
- This is an architecture slice: audit how neomacs implements `make-thread`
  today (OS threads? green?), then either implement the GIL + binding-swap
  model or document divergence. Estimated as the largest remaining item.

## 6. Methodology (hard-won, follow these)

- **Gates**: every slice gets (a) targeted GNU-vs-NEO probes (write forms to
  `./tmp/*.el`, run both binaries, `diff` normalized output), (b) neovm-core
  unit tests for touched paths, (c) the oracle process/network/timer family
  (`-E 'test(process) or test(network) or test(timer) or …'`, ~257 tests,
  seconds), (d) a **clean idle full regression** before push (~30 min;
  38.5k tests). Compare leaf-name sets against the previous clean baseline
  with `comm`; re-run any "new" failure standalone ×2 before believing it.
- **Full regressions are only valid on an idle machine.** A run concurrent
  with builds/tests produced 127 timeouts (baseline: 6) and a garbage diff.
- **`cargo build | tail -1 && …` masks failures** — the `&&` sees `tail`'s
  exit code. Use `set -o pipefail`, check `grep -c "^error"`, or verify the
  binary mtime.
- **Rebuild ⇒ pdump regen**: `cargo xtask fresh-build --release --skip-build
  --no-byte-compile` after every `cargo build --release -p neomacs`, or the
  binary panics on the stale dump.
- **Oracle expect strings come from GNU**: `NEOVM_ORACLE_MODE=refresh
  UPDATE_EXPECT=1 cargo nextest run -p neovm-oracle-tests <test>` regenerates
  from live GNU. Never hand-write them. Live mode ignores expects (parity
  only); refresh mode asserts expect == GNU.
- **Study GNU's C before classifying any divergence** as expected/artifact —
  two earlier misclassifications (window top_line, "queued pipeline") were
  both corrected only after reading the source. Empirical probes beat
  reasoning about microdynamics: when GNU behavior looks jitter-dependent,
  run GNU 10× (see the cx64 boundary study).
- **GNU does it in Lisp ⇒ neomacs must not do it in Rust.** The timer brain
  is the canonical example: the fix was deleting Rust code and trusting
  timer.el.
- Timing tests that print wall-clock-bearing values (timer vectors, ephemeral
  ports) can never match across two processes — normalize the form, not the
  harness.

## 7. Known-remaining divergences outside this subsystem

- **EXPECTED-TIMING**: the exact-boundary repeating-timer trio
  (`div_cx64_timer_repeated_invocation_in_order`,
  `div_cx179/cx338_repeat_timer_fires_multiple`) — fire-at-deadline is a race
  GNU itself flips on (2/10 idle runs); visible mainly on idle machines.
- **Display-backend geometry** (~15 tests): window-pixel/frame metrics,
  wrapped `vertical-motion`, `count-screen-lines` under the `neo` backend.
- **EXPECTED**: `neo` ≠ `x` window-system probes; absent features
  (native-comp, treesit, dbus, lcms2, sqlite differences); `features` list
  contents; GC-count internals; weak-hash GC-conservatism.
- **Noted real bug (unfixed, small)**: `split-window-below` leaves the
  window's horizontal `normal-size` at 0.5 after split+delete (should be
  1.0; a vertical split must not touch horizontal fractions). See
  `refactor/window-topline-decouple` merge (`25a0ae2b3`) for the adjacent
  geometry model.

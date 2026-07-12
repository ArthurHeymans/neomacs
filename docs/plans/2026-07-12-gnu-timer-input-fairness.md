# GNU Timer/Input Fairness Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Prevent due timers that reschedule themselves from starving interactive command input, matching GNU Emacs timer semantics.

**Architecture:** Timer servicing shallow-copies the ordinary and idle timer lists, then re-decodes each shared timer vector as it selects work from that stable batch. The wait scheduler checks pending command input before starting another batch, preserving GNU's timers-first ordering without allowing callback-created timers to monopolize one service pass. GUI publishing defers Lisp-backed chrome and title snapshots while `throw-on-input` is dynamically active, so the host callback cannot swallow that non-local exit.

**Tech Stack:** Rust, GNU Emacs Lisp timer representation, crossbeam input channel, `cargo nextest`.

---

### Task 1: Specify stable timer-batch behavior

**Files:**
- Test: `neovm-core/src/emacs_core/process_test.rs`
- Modify: `neovm-core/src/emacs_core/timer.rs`
- Modify: `neovm-core/src/keyboard.rs`
- Modify: `neovm-core/src/emacs_core/process.rs`

1. Add a regression where a zero-delay timer callback schedules another zero-delay timer.
2. Assert one timer-service call runs only the timer batch that existed when the call began.
3. Run the focused test with `cargo nextest` and verify it fails because the current live-list loop runs the replacement timer immediately.
4. Shallow-copy the timer-list cells once, re-reading each shared timer vector before selecting it, and run only that batch.
5. Run the focused test and neighboring timer tests with `cargo nextest`.

### Task 2: Specify command-input fairness

**Files:**
- Test: `neovm-core/src/emacs_core/process_test.rs`
- Modify: `neovm-core/src/emacs_core/timer.rs`

1. Add a regression combining a self-rescheduling zero-delay idle timer with asynchronously delivered command input.
2. Assert the wait returns `InputPending` rather than remaining inside timer service.
3. Run the focused test and verify the starvation failure.
4. Verify the existing wait loop stages/checks command input before retrying another stable timer batch.
5. Run the focused wait and timer suites with `cargo nextest`.

### Task 3: Cover the real M-x path and verify

**Files:**
- Test: `neovm-core/src/emacs_core/load_test.rs` or `neomacs-tui-tests/tests/basic.rs`

1. Add the narrowest deterministic regression that enters `M-x`, lets eager completion schedule its background idle timer, and delivers printable input asynchronously.
2. Assert the printable input reaches `self-insert-command` without requiring `C-g`.
3. Run the focused test with `cargo nextest`.
4. Run all affected crate tests with `cargo nextest`.
5. Review the diff for GNU ordering fidelity, then commit the implementation.

### Task 4: Keep redisplay outside Lisp non-local control flow

**Files:**
- Modify: `neomacs-bin/src/main.rs`

1. Detect the dynamic `throw-on-input` scope without evaluating Lisp.
2. Defer Lisp-backed chrome and title snapshots until an ordinary redisplay.
3. Verify GUI chrome startup behavior with `cargo nextest`.

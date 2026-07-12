# Frontend Event Queue Implementation Plan

> **For Codex:** Execute this plan incrementally with test-driven development. Run tests only with `cargo nextest`; do not build or test with `--release`.

**Goal:** Route every frontend event through one ordered semantic seam so renderer lifecycle acknowledgements are serviced internally and can never masquerade as command input.

**Architecture:** Retain one physical FIFO in `KeyboardRuntime`, but put exhaustive classification and internal-event servicing behind a `frontend_events` module. Queue consumers ask semantic questions (pending command input, wait-special service, throw-on-input, next Lisp-visible event) instead of independently matching transport variants. Only leading internal events are drained, preserving the ordering required by pointer events and their presentation snapshots.

**Tech Stack:** Rust, crossbeam channels, Neomacs evaluator/keyboard runtime, `cargo nextest`.

---

### Task 1: Lock down the false-interruption regression

**Files:**
- Modify: `neovm-core/src/emacs_core/eval_test.rs`

1. Add a test that stages `InputEvent::PresentationRetired` while `throw-on-input` is active and evaluates a simple form.
2. Assert evaluation completes normally and the retirement does not cause a non-local exit.
3. Run the focused test with `cargo nextest run -p neovm-core <test-name>` and confirm it fails for the expected false-pending-input reason.

### Task 2: Introduce exhaustive frontend-event semantics

**Files:**
- Create: `neovm-core/src/frontend_events.rs`
- Modify: `neovm-core/src/lib.rs`
- Modify: `neovm-core/src/keyboard.rs`
- Test: `neovm-core/src/frontend_events.rs`

1. Define private semantic categories for command, Lisp-special, and internal events.
2. Classify every `InputEvent` variant explicitly, without a wildcard arm.
3. Express pending-input, throw-on-input, and wait-special policy through semantic operations in the new module.
4. Add focused policy tests, including `PresentationRetired` being internal and non-interrupting.
5. Run the new module tests with `cargo nextest` and make them pass.

### Task 3: Service internal events at the queue boundary

**Files:**
- Modify: `neovm-core/src/frontend_events.rs`
- Modify: `neovm-core/src/keyboard.rs`
- Test: `neovm-core/src/emacs_core/eval_test.rs`
- Test: `neovm-core/src/keyboard_test.rs`

1. Add a queue service operation that drains only leading internal events.
2. Handle `PresentationRetired` by releasing the captured interaction snapshot with no redisplay, idle, or Lisp-visible effect.
3. Add tests that retirement releases roots, requests no redisplay, and does not satisfy `input-pending-p`.
4. Add FIFO tests for pointer-before-retirement, retirement-before-pointer, and internal-before-key ordering.
5. Run each focused test with `cargo nextest`, observing red before implementation and green afterward.

### Task 4: Route all scheduler and evaluator decisions through the seam

**Files:**
- Modify: `neovm-core/src/emacs_core/eval.rs`
- Modify: `neovm-core/src/emacs_core/reader.rs`
- Modify: `neovm-core/src/emacs_core/wait.rs`
- Modify: `neovm-core/src/keyboard.rs`
- Test: `neovm-core/src/emacs_core/eval_test.rs`

1. Service leading internal events before `while-no-input`/throw polling.
2. Service them before `input-pending-p`, timer-batch preemption, idle decisions, and blocking waits.
3. Ensure `read_char` receives only Lisp-visible events and an internal acknowledgement cannot restart the redisplay loop.
4. Remove the old scattered classifier functions after all callers use semantic operations.
5. Add or adapt regression tests for timer preemption, idle state, and read-loop redisplay amplification.
6. Run the affected focused test groups with `cargo nextest`.

### Task 5: Verify, review, and commit

**Files:**
- Modify as required by review findings.

1. Run `cargo fmt --all -- --check` and format if needed.
2. Run `cargo nextest run -p neovm-core` once as the full affected suite.
3. Review the diff against the accepted design and repository standards; fix actionable findings.
4. Re-run the focused regressions and the full affected suite after fixes.
5. Commit the implementation with a focused message.

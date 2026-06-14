//! Event-loop / wait-machinery regression tests (Neomacs-only).
//!
//! These guard the GNU-faithful unified-wait redesign (issue #132). Each
//! test exercises one of the three things the editor blocks on through the
//! single poll primitive:
//!
//!   * **host keyboard input** — a keystroke wakes the command loop and is
//!     echoed (the cross-platform `Poller::notify` input-wakeup path);
//!   * **timer timeout** — a due timer fires while the editor sits idle in a
//!     pure-timeout wait (the wakeable poll that replaced blind
//!     `thread::sleep`);
//!   * **subprocess output** — an async child's stdout becomes readable on a
//!     poller fd, is filtered into its buffer, and drives a redisplay (this
//!     also reaps the child on EOF).
//!
//! The synchronous-shell-command test covers the fourth thing the wait loop
//! must handle: the command loop *blocking* in `wait_reading_process_output`
//! until a child exits, draining its output, then returning responsive. This
//! is also the path that issue #132 broke (a child suspending Neomacs via job
//! control). We cannot reproduce #132's suspend deterministically in a pty
//! (a synchronous shell command gives the child a pipe, not the controlling
//! terminal, and an interactive `bash -ic` would source the developer's real
//! `~/.bashrc`), so the environment-independent proof that each child runs in
//! its own process group — the actual fix — lives in the `child_isolation_tests`
//! unit test in `neovm-core/src/emacs_core/callproc/mod.rs`.

mod support;
use neomacs_tui_tests::*;
use std::time::Duration;
use support::*;

// ── Local helpers ───────────────────────────────────────────

/// Boot a single Neomacs TUI session and wait until *scratch* is rendered.
fn boot_neo(extra_args: &str) -> TuiSession {
    let mut neo = TuiSession::neomacs(extra_args);
    let startup_ready = |grid: &[String]| {
        grid.iter().any(|row| row.contains("*scratch*"))
            && grid
                .iter()
                .any(|row| row.contains("This buffer is for text that is not saved"))
    };
    neo.read_until(Duration::from_secs(20), startup_ready);
    settle_session(&mut neo);
    neo
}

/// Eval `expression` via `M-:` and assert the echo area shows `expected`.
/// Doubles as a liveness probe: it only succeeds if the command loop is
/// actively reading and processing host input.
fn assert_eval_echoes(neo: &mut TuiSession, expression: &str, expected: &str) {
    eval_expression_one(neo, expression);
    let shows = |grid: &[String]| grid.iter().any(|row| row.contains(expected));
    neo.read_until(Duration::from_secs(8), shows);
    assert!(
        shows(&neo.text_grid()),
        "expected `{expected}` from evaluating `{expression}`:\n{}",
        neo.text_grid().join("\n")
    );
}

// ── Tests ──────────────────────────────────────────────────

/// A burst of self-inserting keystrokes must wake the command loop and
/// echo into *scratch*. This is the host-input wakeup path: the frontend
/// thread delivers the key and notifies the poller, which unblocks the
/// wait so the command loop can run.
#[test]
fn keyboard_input_echoes_into_scratch() {
    let mut neo = boot_neo("");

    neo.send(b"event-loop-typed-probe");
    let typed = |grid: &[String]| {
        grid.iter()
            .any(|row| row.contains("event-loop-typed-probe"))
    };
    neo.read_until(Duration::from_secs(5), typed);

    assert!(
        typed(&neo.text_grid()),
        "self-inserting keystrokes should reach the command loop and echo \
         into *scratch*:\n{}",
        neo.text_grid().join("\n")
    );
}

/// A one-shot timer scheduled while the editor is otherwise idle must fire
/// and mutate the buffer, and the idle redisplay must paint the change
/// without any intervening keypress. Exercises the pure-timeout wakeable
/// wait that replaced blind `thread::sleep`.
#[test]
fn timer_fires_and_redisplays_while_idle() {
    let mut neo = boot_neo("");

    eval_expression_one(
        &mut neo,
        "(run-with-timer 0.2 nil (lambda () (insert \"timer-fired-probe\")))",
    );

    let fired = |grid: &[String]| grid.iter().any(|row| row.contains("timer-fired-probe"));
    neo.read_until(Duration::from_secs(5), fired);

    assert!(
        fired(&neo.text_grid()),
        "an idle one-shot timer should fire and the idle redisplay should \
         paint its buffer mutation without a keypress:\n{}",
        neo.text_grid().join("\n")
    );
}

/// An async subprocess' stdout becoming readable must drive the output
/// into *Async Shell Command* and trigger a redisplay. Exercises the
/// process-output poll backend and child reaping on EOF.
#[test]
fn async_subprocess_output_drives_redisplay() {
    let mut neo = boot_neo("");

    neo.send_key("M-&");
    let prompt_ready =
        |grid: &[String]| grid.iter().any(|row| row.contains("Async shell command:"));
    neo.read_until(Duration::from_secs(8), prompt_ready);
    neo.read(Duration::from_millis(300));

    neo.send(b"printf neo-async-probe");
    neo.send_key("RET");

    let appeared = |grid: &[String]| {
        grid.iter().any(|row| row.contains("*Async Shell Command*"))
            && grid.iter().any(|row| row.contains("neo-async-probe"))
    };
    neo.read_until(Duration::from_secs(12), appeared);

    assert!(
        appeared(&neo.text_grid()),
        "async subprocess output should appear in *Async Shell Command*:\n{}",
        neo.text_grid().join("\n")
    );
}

/// A synchronous shell command (`M-!`) blocks the command loop in
/// `wait_reading_process_output` until the child exits, draining its output.
/// The command must complete (output appears) and — critically — the editor
/// must return responsive afterward (an eval round-trips). This is the
/// command-loop side of the issue #132 fix: a child can no longer wedge or
/// suspend Neomacs while it is waited on.
#[test]
fn synchronous_shell_command_completes_and_editor_stays_responsive() {
    let mut neo = boot_neo("");

    neo.send_key("M-!");
    let prompt_ready = |grid: &[String]| grid.iter().any(|row| row.contains("Shell command:"));
    neo.read_until(Duration::from_secs(8), prompt_ready);
    neo.read(Duration::from_millis(300));
    neo.send(b"printf neo-sync-probe");
    neo.send_key("RET");

    let output = |grid: &[String]| grid.iter().any(|row| row.contains("neo-sync-probe"));
    neo.read_until(Duration::from_secs(8), output);
    assert!(
        output(&neo.text_grid()),
        "synchronous shell command output should appear:\n{}",
        neo.text_grid().join("\n")
    );

    // The command loop must be responsive again: an eval round-trips.
    assert_eval_echoes(&mut neo, "(+ 40 2)", "42");
}

/// Issue #132 (hang): with `shell-command-switch "-ic"`, a synchronous `M-!`
/// launches an interactive `bash -ic …`. Before the fix Neomacs wedged forever
/// in `command.output()` (`wchan = pipe_read`): the interactive child, left as
/// a background process group on Neomacs's controlling pty, was SIGTTOU/SIGTTIN-
/// stopped during its own job-control init and never exited. The fix `setsid`s
/// every pipe-stdio child (`isolate_child_command`), giving it no controlling
/// terminal, so `bash -i` degrades to "no job control" and runs to completion.
/// This guards that the synchronous shell-command returns and the editor stays
/// responsive afterward.
#[test]
fn interactive_switch_synchronous_shell_command_stays_responsive() {
    let mut neo = boot_neo("");

    eval_expression_one(&mut neo, "(setq shell-command-switch \"-ic\")");
    neo.read(Duration::from_millis(500));

    neo.send_key("M-!");
    let prompt_ready = |grid: &[String]| grid.iter().any(|row| row.contains("Shell command:"));
    neo.read_until(Duration::from_secs(8), prompt_ready);
    neo.read(Duration::from_millis(300));
    neo.send(b"true");
    neo.send_key("RET");
    neo.read(Duration::from_secs(2));

    // The synchronous interactive shell command must return and leave the
    // command loop responsive: an eval round-trips.
    assert_eval_echoes(&mut neo, "(+ 40 2)", "42");
}

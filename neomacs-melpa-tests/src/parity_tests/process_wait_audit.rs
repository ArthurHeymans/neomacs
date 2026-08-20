//! A standing audit of how parity fixtures decide a subprocess has finished.
//!
//! DIVERGENCES.md 133, 140, 144 and 165 all circle one defect: a fixture pins
//! text that came out of a child process, and gates that pin on the child
//! being dead.  Those are different moments, and GNU's own ordering says which
//! way round they are.
//!
//! `handle_child_signal` (GNU src/process.c:7691, loop at :7736) reaps the
//! child and, in a single pass, sets `raw_status_new` (:7748) -- which is all
//! `Fprocess_status` needs to answer `exit` (:1188-1189), so `process-live-p`
//! goes nil right there -- and calls `delete_read_fd` (:7760), which STOPS the
//! event loop from reading the child's pipe.  Whatever the child had already
//! written is recovered only by the drain loop inside `status_notify`
//! (:7896-7911), which runs immediately before `exec_sentinel` (:7937).
//!
//! So `process-live-p` going nil is not merely "a bit early".  It is the exact
//! instant ordinary reading stops, and everything still queued arrives later.
//!
//! The one case where it is NOT early is worth stating, because it is why a
//! bare grep over-selects: a pipe or network connection has no pid, so
//! `handle_child_signal` cannot reach it.  Its status changes only when
//! `read_process_output` returns 0 (:6072-6079 for `PIPECONN_P`, :6082-6090
//! for the rest), which is after the final read.  For those, death IS the
//! output ending.  `make-process :stderr BUFFER` builds exactly such a process
//! (:1882-1889, via `Fmake_pipe_process`, type `Qpipe`, see :229).
//!
//! This module keeps the population honest.  Every `process-live-p` used as a
//! loop condition in a parity fixture must be accounted for here, with a
//! verdict saying why it is not the defect above.  A new one fails this test
//! rather than moving a snapshot months later.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Why a fixture's `process-live-p` loop is not a pin gated on the clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProcessWaitVerdict {
    /// The fixture already called `delete-process` and is only confirming the
    /// process died.  Nothing downstream reads the child's output.
    Teardown,
    /// The loop bails out on death, but the condition it actually waits for is
    /// a fact the child or its sentinel produced -- a prompt the child wrote,
    /// a flag a sentinel set -- and it signals if that fact never arrives.
    CausalWitness,
    /// The awaited process is a pipe or network connection, not a child with a
    /// pid, so its death IS its EOF.  See the module docstring.
    PipeEof,
    /// The wait is real, but no pinned value depends on the child's output.
    OutputNotPinned,
    /// Carries the shape of the defect and has NOT been reproduced or fixed.
    /// Each needs its own reproduction before its gate can be chosen; see
    /// DIVERGENCES.md 165.  This bucket may shrink, never grow.
    NeedsAudit,
}

use ProcessWaitVerdict::*;

/// Every suite that uses `process-live-p` as a loop condition, how many such
/// loops it has, and why each one is accounted for.
const PROCESS_WAIT_AUDIT: &[(&str, usize, ProcessWaitVerdict, &str)] = &[
    (
        "abs_mode",
        1,
        Teardown,
        "abs-test-drain-processes; the *erlang* pin now gates on the sentinel",
    ),
    (
        "activity_watch_mode",
        1,
        NeedsAudit,
        "aw-test-drain waits out every process, then drains until nothing arrives",
    ),
    (
        "agitjo",
        1,
        OutputNotPinned,
        "the child writes nothing and agitjo-test-events is never called",
    ),
    (
        "ast_grep",
        1,
        NeedsAudit,
        "pins process-status and a log file the child wrote",
    ),
    (
        "async_backup",
        1,
        NeedsAudit,
        "process-live-p plus one accept-process-output",
    ),
    (
        "async_status",
        1,
        NeedsAudit,
        "pins the exit status and a file the child wrote",
    ),
    (
        "auctex_cluttex",
        2,
        NeedsAudit,
        "pins TeX-sentinel-function state behind process-live-p plus one accept",
    ),
    (
        "auctex_latexmk",
        1,
        NeedsAudit,
        "pins the latexmk output buffer behind process-live-p plus one accept",
    ),
    (
        "blacken",
        2,
        PipeEof,
        "*blacken-error* is a :stderr pipe process; blacken.el:87-110 never waits for it",
    ),
    (
        "browse_at_remote",
        2,
        NeedsAudit,
        "one teardown; one VC-annotate wait followed by stable-sample rounds",
    ),
    (
        "find_file_in_project",
        1,
        Teardown,
        "confirms an already-deleted process died",
    ),
    (
        "ggtags",
        1,
        Teardown,
        "teardown only; the pins gate on ggtags-global-exit-info",
    ),
    (
        "git_commit_mode",
        1,
        Teardown,
        "confirms an already-deleted process died",
    ),
    (
        "magit",
        1,
        NeedsAudit,
        "drains every descriptor while the main child lives; Git gets a separate stderr pipe",
    ),
    (
        "magit_gitflow",
        1,
        NeedsAudit,
        "returns the exit status, but the case reads Magit buffers afterwards",
    ),
    (
        "nodejs_repl",
        2,
        CausalWitness,
        "waits for the prompt the child itself wrote, and signals if it never comes",
    ),
    (
        "org_cliplink",
        1,
        CausalWitness,
        "then waits for transport-error, which the delegated curl sentinel sets",
    ),
    (
        "org_ref",
        2,
        Teardown,
        "two cleanup sweeps over already-deleted processes",
    ),
    (
        "overseer",
        1,
        OutputNotPinned,
        "the case pins the process's own liveness after its buffer is killed",
    ),
    (
        "pipenv",
        1,
        NeedsAudit,
        "its own comment says it is settling the real sentinel by the clock",
    ),
    (
        "python_mode",
        2,
        CausalWitness,
        "one waits for the new prompt the child wrote; one is teardown",
    ),
    (
        "robe",
        1,
        Teardown,
        "bounded graceful close, then delete-process",
    ),
    (
        "rspec_mode",
        1,
        Teardown,
        "teardown only; the pins gate on compilation-finish-functions",
    ),
    (
        "tide",
        1,
        Teardown,
        "confirms a forcibly disposed process died",
    ),
    (
        "treemacs_magit",
        1,
        NeedsAudit,
        "waits for Magit's child, then reads Treemacs node state",
    ),
    (
        "zeal_at_point",
        1,
        OutputNotPinned,
        "the helper returns only the status and exit status",
    ),
];

/// The unaudited backlog DIVERGENCES.md 165 hands on.  This number may shrink,
/// never grow: a new clock gate must be classified, not appended.
const NEEDS_AUDIT_SITE_BUDGET: usize = 13;

fn parity_tests_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/parity_tests")
}

/// True when LINE uses `process-live-p` as a `while` loop condition.
fn is_process_live_p_loop(line: &str) -> bool {
    line.contains("while (process-live-p")
        || (line.contains("while (and") && line.contains("process-live-p"))
}

fn suite_name(root: &Path, file: &Path) -> Option<String> {
    let relative = file.strip_prefix(root).ok()?;
    let first = relative.components().next()?;
    let name = first.as_os_str().to_str()?;
    Some(name.strip_suffix(".rs").unwrap_or(name).to_string())
}

fn collect_rust_files(directory: &Path, into: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => panic!("cannot read {}: {error}", directory.display()),
    };
    for entry in entries {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect_rust_files(&path, into);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            into.push(path);
        }
    }
}

fn measure_loops() -> BTreeMap<String, usize> {
    let root = parity_tests_directory();
    let mut files = Vec::new();
    collect_rust_files(&root, &mut files);

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for file in files {
        let Some(suite) = suite_name(&root, &file) else {
            continue;
        };
        if suite == "process_wait_audit" {
            continue;
        }
        let contents = fs::read_to_string(&file).expect("fixture source is UTF-8");
        let hits = contents
            .lines()
            .filter(|line| is_process_live_p_loop(line))
            .count();
        if hits > 0 {
            *counts.entry(suite).or_default() += hits;
        }
    }
    counts
}

#[test]
fn every_process_live_p_wait_is_audited() {
    let measured = measure_loops();
    let audited: BTreeMap<&str, usize> = PROCESS_WAIT_AUDIT
        .iter()
        .map(|(suite, count, _, _)| (*suite, *count))
        .collect();

    let mut problems = Vec::new();

    for (suite, count) in &measured {
        match audited.get(suite.as_str()) {
            None => problems.push(format!(
                "{suite}: {count} `process-live-p' loop(s) and no entry in PROCESS_WAIT_AUDIT. \
                 A wait on a child's death is not a wait on its output having been read -- see \
                 this module's docstring and DIVERGENCES.md 133/140/144/165. Gate the pin on a \
                 causal witness, or classify the wait here and say why it is safe."
            )),
            Some(expected) if expected != count => problems.push(format!(
                "{suite}: PROCESS_WAIT_AUDIT records {expected} `process-live-p' loop(s) but the \
                 fixture now has {count}. A wait was added or removed; re-read it and update the \
                 entry deliberately rather than bumping the number."
            )),
            Some(_) => {}
        }
    }

    for suite in audited.keys() {
        if !measured.contains_key(*suite) {
            problems.push(format!(
                "{suite}: PROCESS_WAIT_AUDIT still lists it, but it has no `process-live-p' loop \
                 left. Drop the entry."
            ));
        }
    }

    let needs_audit_sites: usize = PROCESS_WAIT_AUDIT
        .iter()
        .filter(|(_, _, verdict, _)| *verdict == NeedsAudit)
        .map(|(_, count, _, _)| *count)
        .sum();
    if needs_audit_sites > NEEDS_AUDIT_SITE_BUDGET {
        problems.push(format!(
            "the NeedsAudit backlog grew from {NEEDS_AUDIT_SITE_BUDGET} to {needs_audit_sites} \
             site(s). That bucket is a debt DIVERGENCES.md 165 handed on; it may shrink, never grow."
        ));
    }

    assert!(
        problems.is_empty(),
        "unaudited subprocess waits in the MELPA parity fixtures:\n  - {}",
        problems.join("\n  - ")
    );
}

#[test]
fn the_audit_table_matches_the_backlog_it_claims() {
    let needs_audit_sites: usize = PROCESS_WAIT_AUDIT
        .iter()
        .filter(|(_, _, verdict, _)| *verdict == NeedsAudit)
        .map(|(_, count, _, _)| *count)
        .sum();
    assert_eq!(
        needs_audit_sites, NEEDS_AUDIT_SITE_BUDGET,
        "NEEDS_AUDIT_SITE_BUDGET must equal the NeedsAudit sites actually listed; \
         lower it as they are closed"
    );
}

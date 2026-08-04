use std::time::Duration;

use crate::{CachedMelpaOracle, MAGIT_MELPA_PIN};

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod blame;
mod clone;
mod formatting;
mod git;
mod prompts;
mod status;
mod workflows;

const MAGIT_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn magit_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(MAGIT_MELPA_PIN, "magit.el")
        .expect("prepare pinned Magit source and dependencies below ./tmp")
        .with_prelude(
            r##"(progn
                   (setq magit-git-global-arguments
                         (append
                          '("-c" "init.defaultBranch=master"
                            "-c" "user.name=A U Thor"
                            "-c" "user.email=a.u.thor@example.com")
                          (and (boundp 'magit-git-global-arguments)
                               magit-git-global-arguments)))
                   (defun neomacs-magit-test-wait-for-process (process)
                     (while (process-live-p process)
                       ;; Magit gives Git a separate stderr pipe.  Drain every
                       ;; ready descriptor so its pipe closes before Magit's
                       ;; main-process sentinel kills the stderr buffer.
                       (accept-process-output nil 0.05)))
                   (defun neomacs-magit-test-wait-for-blame ()
                     ;; The quickstart sentinel replaces the initial process
                     ;; with a full-file blame process, then the final sentinel
                     ;; normally clears this buffer-local variable.  Test the
                     ;; process status too: a sentinel error must not turn a
                     ;; dead process object into a busy loop in the harness.
                     (while (and magit-blame-process
                                 (process-live-p magit-blame-process))
                       (accept-process-output nil 0.05))))"##,
        )
        .with_timeout(MAGIT_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed Magit parity test").into()
}

/// Multi-probe batch for `assert_magit_parity` cases (2a).
pub(crate) fn assert_magit_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(magit_oracle(), &name, "magit_parity", cases);
}

// BEGIN generated package batch tests

#[test]
fn magit_package_batch() {
    let cases: Vec<ParityBatchCase> = [
        blame::blame_public_surface_batch_cases(),
        clone::clone_public_surface_batch_cases(),
        formatting::formatting_public_surface_batch_cases(),
        git::git_public_surface_batch_cases(),
        prompts::prompts_public_surface_batch_cases(),
        status::status_public_surface_batch_cases(),
        workflows::workflows_public_surface_batch_cases(),
    ]
    .into_iter()
    .flatten()
    .collect();
    assert_magit_batch(&cases);
}

// END generated package batch tests

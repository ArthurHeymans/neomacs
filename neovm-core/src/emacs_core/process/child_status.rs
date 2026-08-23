//! GNU's asynchronous child-status recording (`handle_child_signal`,
//! src/process.c:7691), and the type that keeps Lisp from seeing an exited
//! child as running.
//!
//! # What GNU does, and where
//!
//! `handle_child_signal` is the SIGCHLD handler.  It walks the process alist
//! itself and stamps every child whose state changed:
//!
//! ```c
//!   FOR_EACH_PROCESS (tail, proc)                          /* :7734 */
//!     {
//!       struct Lisp_Process *p = XPROCESS (proc);
//!       int status;
//!
//!       if (p->alive
//!           && child_status_changed (p->pid, &status,
//!                                    WUNTRACED | WCONTINUED))   /* :7741-7742 */
//!         {
//!           changed = true;
//!           p->tick = ++process_tick;                      /* :7745 */
//!           p->raw_status = status;                        /* :7746 */
//!           p->raw_status_new = 1;                         /* :7747 */
//!
//!           if (WIFSIGNALED (status) || WIFEXITED (status)) /* :7750 */
//!             {
//!               bool clear_desc_flag = 0;
//!               p->alive = 0;                              /* :7752 */
//!               if (p->infd >= 0) clear_desc_flag = 1;
//!               if (clear_desc_flag) delete_read_fd (p->infd);   /* :7760 */
//!             }
//!         }
//!     }
//!   if (changed) child_signal_notify ();                   /* :7766-7767 */
//! ```
//!
//! Its own header states the contract in one sentence (:7668-7671):
//!
//! ```text
//!    All we do is change the status; we do not run sentinels or print
//!    notifications.  That is saved for the next time keyboard input is
//!    done, in order to avoid timing errors.
//! ```
//!
//! So the recording is *only* a recording, and the notification is
//! `status_notify`'s job (:7862).  The consequence is Lisp-visible and is
//! what this module exists for: in GNU, `(process-live-p exited-child)` is
//! `nil` with nobody having called `accept-process-output` or waited at all.
//!
//! # Why the sweep is NOT in a signal handler here
//!
//! GNU's own comments above `handle_child_signal` enumerate what a SIGCHLD
//! handler may legitimately do, and the list is short (:7673-7688):
//!
//! * *"** WARNING: this can be called during garbage collection.  Therefore,
//!   it must not be fooled by the presence of mark bits in Lisp objects."*
//! * *"** Malloc WARNING: This should never call malloc either directly or
//!   indirectly; if it does, that is a bug."*
//!
//! and `child_signal_notify` (:7616-7650) carries the third, with a stack
//! trace as evidence: an `emacs_perror` was REMOVED from the handler because
//! `strerror_l` is not reentrant and reaches `malloc` through the locale
//! machinery.  All the handler is allowed to do at the end is
//! `emacs_write (fd, &dummy, 1)` to a self-pipe.
//!
//! Those three constraints are the whole design input, and in a Rust port
//! they are decisive.  This port's process table is a
//! `HashMap<ProcessId, Process>` owned by the Lisp thread; a signal is
//! delivered to an arbitrary thread (which is why GNU has
//! `deliver_process_signal`'s FORWARD_SIGNAL_TO_MAIN_THREAD, src/sysdep.c:
//! 1729-1751), and reading that map from a handler while the Lisp thread
//! mutates it is a data race, not merely a lock-order problem.  Iterating it
//! allocates.  So the sweep cannot live where GNU's lives.
//!
//! What CAN live in a handler is exactly what GNU puts there at the end: a
//! byte on a self-pipe.  That is a wake-up, and it changes no Lisp answer --
//! it only decides how soon a safe point is reached.  This port already has
//! the wake-up in another form: a `pidfd` per child, registered with the wait
//! poller (`sys::ChildStatusSource`), which makes the poller return the
//! moment a child terminates.
//!
//! **So the recording is placed where it is safe and made unavoidable by
//! type instead.**  The sweep runs on the Lisp thread at GNU's own
//! `update_status` call sites, and a subr cannot report a status without it:
//! [`ObservedProcess`] has private fields, and the constructor that sweeps
//! takes an [`UpdateStatusSite`] naming which of GNU's eight `update_status`
//! lines the caller is.  A subr that has not named a site cannot spell a
//! status at all -- see [`ObservedProcess`] for the exact scope of that,
//! which is the Lisp-visible answer and not the manager's own internals.
//!
//! # Why that is Lisp-indistinguishable from GNU's placement
//!
//! GNU's own comment says the recording exists so that the answer is ready
//! "the next time keyboard input is done".  The only way Lisp learns a
//! process's status is by calling one of the entry points in
//! [`UpdateStatusSite`], so sweeping immediately before answering one of them
//! gives the same answer GNU gives.  The window this does NOT close is the
//! one that is not a Lisp answer at all: GNU's handler also *reaps* the child
//! (via `child_status_changed` -> `waitpid`), so GNU's exited child leaves no
//! zombie, and this port's does until the first observation.  Measured,
//! `-Q --batch`, a child that exits with nobody waiting, then a one-second
//! pure-Lisp spin:
//!
//! ```text
//!                                        GNU 31.0.90   this port
//!   (process-attributes pid) 'state          nil          "Z"
//! ```
//!
//! That row needs the trigger itself, not the sweep, and it is recorded as a
//! residual rather than hidden here.
//!
//! # The pipe is not a child, and cannot be swept
//!
//! `handle_child_signal` passes `p->pid` to `child_status_changed`, and
//! `get_child_status` opens with `eassert (child > 0)` (src/sysdep.c:461).
//! A pipe, network or serial connection has no pid, so the handler cannot
//! reach it; its status changes in exactly one other place --
//! `read_process_output` returning 0 (:6072-6079), which is inside the wait.
//! Ledger 165's finding is that `process-live-p` therefore means the
//! OPPOSITE thing for a pipe, and a fix keyed on "the child exited" that
//! touched pipes would be wrong for half the process kinds.
//!
//! [`SweepableChild`] is that rule as a type: it carries the OS pid, and its
//! only constructor is the membership test.  A pidless process is not
//! skipped by an `if` inside the loop -- it cannot be built, so it cannot be
//! in the population.

use super::{
    Process, ProcessId, ProcessManager, ProcessStatusSymbol, Value, process_effective_status,
    process_public_status_symbol,
};

/// A process that GNU's SIGCHLD sweep may harvest.
///
/// GNU's membership test is `p->alive && child_status_changed (p->pid, ...)`
/// (src/process.c:7741-7742), and the pid half of it is enforced one frame
/// down by `eassert (child > 0)` (src/sysdep.c:461).  Both halves are in
/// [`SweepableChild::of`], and there is no other constructor: a connection
/// with no child is not a member of the population rather than an early
/// `continue` inside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SweepableChild {
    id: ProcessId,
}

impl SweepableChild {
    /// GNU's `p->alive` (:7741) plus the pid that `get_child_status` requires.
    ///
    /// This port spells `p->alive` as "the recorded status is one that can
    /// still change" -- `run` or `stop`, exactly the pair
    /// `poll_child_status_change` already keeps polling so a later
    /// `WCONTINUED` stays observable -- and spells `p->pid` as any of the
    /// three child handles a spawn may have left behind.
    pub(super) fn of(id: ProcessId, proc: &Process) -> Option<Self> {
        let status_can_change = matches!(
            ProcessStatusSymbol::from_status_value(proc.status),
            Some(ProcessStatusSymbol::Run | ProcessStatusSymbol::Stop)
        );
        // GNU's `p->alive` itself, since ledger 187: the pid is present in
        // exactly the state in which `waitpid` may be called on it.
        let alive = proc.live_io.child.pid_if_unreaped().is_some();
        (status_can_change && alive).then_some(Self { id })
    }

    pub(super) fn id(self) -> ProcessId {
        self.id
    }
}

/// One of the eight places GNU calls `update_status` (src/process.c:717).
///
/// The list is closed and mechanically derivable -- `grep -n 'update_status
/// ('` over src/process.c gives the definition plus exactly these eight call
/// sites -- so it can be a finite type rather than a convention.  Every read
/// of a Lisp-visible process status in this port has to name the GNU line it
/// is, and [`UpdateStatusSite::recording`] then decides whether that line
/// needs the sweep run first.
///
/// The point of the enum is the same as ledger 177's `PostImageInit`: a new
/// site cannot be added without a GNU citation and a classification, because
/// [`UpdateStatusSite::ALL`] is declared with length
/// [`UpdateStatusSite::COUNT`] (derived from the last discriminant) and
/// [`UpdateStatusSite::gnu`] and [`UpdateStatusSite::recording`] are
/// exhaustive matches.  An empty or short table is a compile error, not a
/// silent omission, and `child_status_test.rs` asserts the absolute count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub(crate) enum UpdateStatusSite {
    /// `Fdelete_process` (:1143).  This one does NOT harvest, and the line
    /// above it is why: `p->raw_status_new = 0;` at :1123 THROWS THE RECORD
    /// AWAY before anything else happens, so :1141's `if (p->raw_status_new)`
    /// can only be true for a status that `record_kill_process`'s own SIGKILL
    /// produced between :1136 and :1140.  Measured, a child that exited 7
    /// with nobody waiting: both editors answer `signal` / 9, because GNU
    /// discards the 7 and this port never recorded it.
    DeleteProcess = 0,
    /// `Fprocess_status` (:1189).  Also `process-live-p`, which is
    /// `(memq (process-status process) '(run open listen connect stop))`
    /// in lisp/subr.el:3538-3540 and has no C of its own.
    ProcessStatus,
    /// `Fprocess_exit_status` (:1213).
    ProcessExitStatus,
    /// `send_process` (:6726) -- `process-send-string` and
    /// `process-send-region`, which error "Process %s not running: %s" when
    /// the settled status is not `run` (:6727-6728).
    SendProcess,
    /// `Fprocess_send_eof` (:7453), with the same "not running" error
    /// (:7454-7455).
    ProcessSendEof,
    /// `wait_reading_process_output` (:5562), on the process being waited
    /// for.
    WaitReadingProcessOutput,
    /// `read_process_output`'s pipe-connection EOF arm (:6087).
    ReadProcessOutputPipeEof,
    /// `status_notify` (:7915), immediately before `status_message` and the
    /// removal decision.
    StatusNotify,
}

/// Where the record this site reads was made.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Recording {
    /// GNU reaches this site with the record already made *and so does this
    /// port*, because the site is inside the wait/notification machinery
    /// that has just done the discovery itself.
    AlreadyRecorded {
        /// Where this port made the record, so the claim is checkable.
        by: &'static str,
    },
    /// GNU reaches this site with the record already made because
    /// `handle_child_signal` ran ASYNCHRONOUSLY, and this port does not,
    /// because it has no asynchronous trigger to run
    /// [`ProcessManager::record_child_status_changes`] from.
    ///
    /// **This is the open divergence, and the `why` says why the obvious
    /// substitute is not one.**  Sweeping here -- running GNU's
    /// `handle_child_signal` body on demand, at the observation -- gives the
    /// right answer to the question and the wrong answer to the program:
    /// GNU's record is late by the tens of microseconds a SIGCHLD takes to
    /// be delivered and handled, and a `waitpid (WNOHANG)` at the
    /// observation is ground truth.  Measured, ledger 180 §6b and §8:
    /// `(while (process-live-p p) (accept-process-output p 1))` loses its
    /// sentinel 0/60 in GNU and 4/60 with the sweep wired here, and
    /// `treemacs-magit`'s `extending_a_real_commit_schedules_the_same_project
    /// _refresh` fails DETERMINISTICALLY, because magit's post-commit hook is
    /// keyed on `last-command` and the sentinel then runs after the `let`
    /// that bound it has unwound.
    AsynchronousInGnu {
        /// GNU's line that makes the record for this site.
        by: &'static str,
        /// Why this port does not make it here.
        why: &'static str,
    },
}

impl UpdateStatusSite {
    /// Derived from the last discriminant, so a new variant that is not added
    /// to [`Self::ALL`] is a compile error.
    pub(crate) const COUNT: usize = Self::StatusNotify as usize + 1;

    pub(crate) const ALL: [Self; Self::COUNT] = [
        Self::DeleteProcess,
        Self::ProcessStatus,
        Self::ProcessExitStatus,
        Self::SendProcess,
        Self::ProcessSendEof,
        Self::WaitReadingProcessOutput,
        Self::ReadProcessOutputPipeEof,
        Self::StatusNotify,
    ];

    /// `file:line` of the `update_status` call in the GNU tree.
    pub(crate) fn gnu(self) -> &'static str {
        match self {
            Self::DeleteProcess => "src/process.c:1143",
            Self::ProcessStatus => "src/process.c:1189",
            Self::ProcessExitStatus => "src/process.c:1213",
            Self::SendProcess => "src/process.c:6726",
            Self::ProcessSendEof => "src/process.c:7453",
            Self::WaitReadingProcessOutput => "src/process.c:5562",
            Self::ReadProcessOutputPipeEof => "src/process.c:6087",
            Self::StatusNotify => "src/process.c:7915",
        }
    }

    /// The Lisp entry point, or `""` for a site with no Lisp name of its own.
    pub(crate) fn lisp(self) -> &'static str {
        match self {
            Self::DeleteProcess => "delete-process",
            Self::ProcessStatus => "process-status",
            Self::ProcessExitStatus => "process-exit-status",
            Self::SendProcess => "process-send-string",
            Self::ProcessSendEof => "process-send-eof",
            Self::WaitReadingProcessOutput => "accept-process-output",
            Self::ReadProcessOutputPipeEof => "",
            Self::StatusNotify => "",
        }
    }

    pub(crate) fn recording(self) -> Recording {
        match self {
            // The four Lisp entry points a program can reach with no wait
            // having run at all.  These are the divergence.
            Self::ProcessStatus
            | Self::ProcessExitStatus
            | Self::SendProcess
            | Self::ProcessSendEof => Recording::AsynchronousInGnu {
                by: "handle_child_signal, src/process.c:7734-7763",
                why: "this port has no asynchronous trigger; sweeping at the \
                      observation instead is measured to lose sentinels \
                      (ledger 180 §6b, §8.1)",
            },
            // The four sites that must not, or need not, sweep here.
            Self::DeleteProcess => Recording::AlreadyRecorded {
                by: "src/process.c:1123 discards the record before this line reads it",
            },
            Self::WaitReadingProcessOutput => Recording::AlreadyRecorded {
                by: "poll_process_output_for_ids -> check_child_status_change",
            },
            Self::ReadProcessOutputPipeEof => Recording::AlreadyRecorded {
                by: "the pipe EOF arm has no child to sweep (SweepableChild::of)",
            },
            Self::StatusNotify => Recording::AlreadyRecorded {
                by: "run_process_status_notification is entered on status_notify_pending",
            },
        }
    }
}

/// A process whose child status has been recorded, and the only route by
/// which a *subr* can obtain one.
///
/// [`process_effective_status`] (GNU `update_status`'s view, src/process.c:
/// 717-721) and [`process_public_status_symbol`] (GNU `Fprocess_status`'s
/// return value, :1188-1201) were `pub(crate)`; they are now private to the
/// parent module and to this child of it, so no `builtins` entry point can
/// call either.  The two methods below are their only public spelling, and
/// an `ObservedProcess` has private fields and exactly two constructors:
/// [`ProcessManager::observe`], which sweeps for every site whose
/// [`Recording`] says so, and
/// [`ProcessManager::read_status_without_recording`], which takes an
/// enumerated [`UnrecordedStatusRead`].
///
/// **The scope of that guarantee, stated exactly, because it is narrower
/// than "nothing can read a status".**  It covers the Lisp-visible ANSWER:
/// to write a subr that reports a process's status, you must name a GNU
/// `update_status` line or one of the enumerated holes.  It does not cover
/// the manager's own internals, which read the `status` field directly and
/// must -- 24 such reads, of which the seven that reach a Lisp answer all go
/// through a named funnel (`gnu_process_status_message_for_status` for the
/// sentinel text, `process_status_ends_target_wait` for the wait, the
/// `live_process_ids` predicates for the service order, and
/// `internal-default-process-sentinel`, `delete-process` and
/// `continue-process`, none of which GNU passes through `update_status`
/// either: GNU's own `Finternal_default_process_sentinel` reads `p->status`
/// bare at :7958, and `Fcontinue_process` never touches it).
///
/// Within that scope the point stands: "the child has exited and Lisp still
/// reads `run`" is not rejected by a check, it is a sentence with no
/// grammar -- to write it you would need a status value, and a subr's status
/// values all come from here.
pub(crate) struct ObservedProcess<'a> {
    proc: &'a Process,
}

impl<'a> ObservedProcess<'a> {
    /// Private: the only caller is [`ProcessManager::observe`], in this
    /// module, after the sweep.
    fn new(proc: &'a Process) -> Self {
        Self { proc }
    }

    /// GNU `p->status` after `update_status` (:717-721): the raw pair, e.g.
    /// `(exit . 7)`, as `Fprocess_exit_status` reads it (:1214-1218).
    pub(crate) fn settled_status(&self) -> Value {
        process_effective_status(self.proc)
    }

    /// GNU `Fprocess_status`'s return value, after the connection remap of
    /// :1193-1201.
    pub(crate) fn public_status_symbol(&self) -> Value {
        process_public_status_symbol(self.proc)
    }

    /// GNU `send_process`'s liveness gate (src/process.c:6725-6728) and
    /// `Fprocess_send_eof`'s (:7451-7455), which are the same two lines:
    /// `update_status`, then `! EQ (p->status, Qrun)` is an error.  GNU reads
    /// `p->status` there because `update_status` has just WRITTEN it; this
    /// port reads the settled view instead, which is the same value.
    pub(crate) fn allows_send(&self) -> bool {
        super::process_allows_send(self.proc)
    }

    /// The process itself, for the fields that are not its status.
    pub(crate) fn process(&self) -> &'a Process {
        self.proc
    }
}

impl ProcessManager {
    /// GNU `handle_child_signal`'s `FOR_EACH_PROCESS` arm (src/process.c:
    /// 7734-7763), run at a safe point instead of in the handler.
    ///
    /// The walk order is GNU's: `FOR_EACH_PROCESS` is
    /// `FOR_EACH_ALIST_VALUE (Vprocess_alist, ...)` (:343) and `make_process`
    /// conses onto the front (:953), so the alist is newest-first and a
    /// descending `ProcessId` reproduces it -- the same identity
    /// `list_processes` and `live_process_ids` already use (ledger 175 §3).
    /// Order does not change what is recorded, since each child is harvested
    /// independently; it is matched so that a future reader does not have to
    /// wonder.
    ///
    /// `check_child_status_change` is GNU's per-process body: the
    /// `child_status_changed` probe (:7742), the `delete_read_fd` on a
    /// terminal status (:7760, spelled here as unregistering the child's
    /// status source from the poller), and the `raw_status`/`raw_status_new`
    /// stamp (:7746-7747, spelled `pending_status`/`status_notify_pending`).
    pub(crate) fn record_child_status_changes(&mut self) {
        let mut population: Vec<SweepableChild> = self
            .processes
            .iter()
            .filter_map(|(id, proc)| SweepableChild::of(*id, proc))
            .collect();
        if population.is_empty() {
            return;
        }
        population.sort_unstable_by(|a, b| b.id().cmp(&a.id()));
        for child in population {
            self.check_child_status_change(child.id());
        }
    }

    /// GNU's `update_status` at `site`, then the read.
    ///
    /// Returns `None` for an id that names no process at all, live or
    /// retired -- the analogue of `get_process` having answered `nil` before
    /// any of these sites was reached.
    pub(crate) fn observe(
        &mut self,
        site: UpdateStatusSite,
        id: ProcessId,
    ) -> Option<ObservedProcess<'_>> {
        match site.recording() {
            // No arm sweeps today.  `AlreadyRecorded` needs nothing;
            // `AsynchronousInGnu` is the open divergence, and the variant's
            // docstring is where the reason lives rather than a comment here.
            // When this port grows the trigger, those four arms become
            // `self.record_child_status_changes()` and nothing else changes:
            // every Lisp-visible status already comes through this function.
            Recording::AlreadyRecorded { .. } | Recording::AsynchronousInGnu { .. } => {}
        }
        self.get_any(id).map(ObservedProcess::new)
    }
}

/// A Lisp-visible status read that this port CANNOT put the sweep in front
/// of, with the reason.
///
/// Every such read is a hole in the guarantee above, so the holes are a
/// finite type rather than a habit: adding one is adding a variant, which
/// forces a GNU citation and a written reason through the exhaustive
/// [`UnrecordedStatusRead::why`] match, and `COUNT` is asserted in
/// `process_test.rs` so a second hole cannot appear unremarked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub(crate) enum UnrecordedStatusRead {
    /// The `%s` mode-line construct.  GNU's `decode_mode_spec` spells it
    /// `obj = Fsymbol_name (Fprocess_status (obj));` (src/xdisp.c:29717-
    /// 29725), so in GNU it IS one of `Fprocess_status`'s callers and does
    /// harvest.
    ///
    /// Here it cannot: `expand_mode_line_percent_in_state` (xdisp.rs:2644)
    /// takes `&ProcessManager`, and so does every frame of the recursive
    /// mode-line renderer above it.  Threading `&mut` through redisplay to
    /// reach one `%` spec is a change to redisplay, not to process status,
    /// and it is not measurable from `--batch`: `format-mode-line` answers
    /// `""` for EVERY spec there, `%b` included, in both editors.
    ModeLinePercentS = 0,
}

impl UnrecordedStatusRead {
    pub(crate) const COUNT: usize = Self::ModeLinePercentS as usize + 1;
    pub(crate) const ALL: [Self; Self::COUNT] = [Self::ModeLinePercentS];

    pub(crate) fn gnu(self) -> &'static str {
        match self {
            Self::ModeLinePercentS => "src/xdisp.c:29723",
        }
    }

    pub(crate) fn why(self) -> &'static str {
        match self {
            Self::ModeLinePercentS => {
                "the recursive mode-line renderer holds &ProcessManager, not &mut"
            }
        }
    }
}

impl ProcessManager {
    /// Read a Lisp-visible status at one of the enumerated holes, WITHOUT
    /// GNU's recording having been made here.
    ///
    /// The `site` argument is not used at run time; it exists so the call
    /// cannot be written without naming which hole it is.
    pub(crate) fn read_status_without_recording(
        &self,
        site: UnrecordedStatusRead,
        id: ProcessId,
    ) -> Option<ObservedProcess<'_>> {
        let _ = site;
        self.get_any(id).map(ObservedProcess::new)
    }
}

//! GNU-style wait policy for VM event servicing.
//!
//! GNU Emacs routes process waits, input waits, timer waits, and redisplay
//! through `wait_reading_process_output` with explicit policy flags.  This
//! module gives Neomacs the same shape: callers describe what may be serviced
//! and what should complete the wait; lower-level process/input code only
//! performs the service pass.

use std::time::{Duration, Instant};

use super::error::Flow;
use super::process::{ProcessId, WaitBackendInterest};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WaitDeadline {
    Poll,
    Until(Instant),
    Forever,
}

impl WaitDeadline {
    fn expired(self, now: Instant) -> bool {
        matches!(self, Self::Until(deadline) if now >= deadline)
    }

    fn remaining(self, now: Instant) -> Option<Duration> {
        match self {
            Self::Poll => Some(Duration::ZERO),
            Self::Until(deadline) => Some(deadline.saturating_duration_since(now)),
            Self::Forever => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyboardWaitPolicy {
    ServiceSpecialOnly,
    WaitForSpecialInput,
    YieldOnCommandInput,
    ReadCommandInput,
}

impl KeyboardWaitPolicy {
    pub(crate) fn completes_on_command_input(self) -> bool {
        matches!(self, Self::YieldOnCommandInput | Self::ReadCommandInput)
    }

    fn waits_for_host_input(self) -> bool {
        matches!(
            self,
            Self::WaitForSpecialInput | Self::YieldOnCommandInput | Self::ReadCommandInput
        )
    }

    pub(crate) fn sets_waiting_for_user_input(self) -> bool {
        matches!(self, Self::ReadCommandInput)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProcessWaitPolicy {
    None,
    ServiceAny,
    Any,
    Target(ProcessId),
    TargetOnly(ProcessId),
}

impl ProcessWaitPolicy {
    pub(crate) fn target_process(self) -> Option<ProcessId> {
        match self {
            Self::Target(id) | Self::TargetOnly(id) => Some(id),
            Self::None | Self::ServiceAny | Self::Any => None,
        }
    }

    pub(crate) fn just_this_one(self) -> bool {
        matches!(self, Self::TargetOnly(_))
    }

    pub(crate) fn services_processes(self) -> bool {
        !matches!(self, Self::None)
    }

    fn satisfied_by(self, outcome: WaitServiceOutcome) -> bool {
        match self {
            Self::Any => outcome.any_process_activity,
            Self::Target(_) | Self::TargetOnly(_) => outcome.target_process_activity,
            Self::None | Self::ServiceAny => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TimerWaitPolicy {
    Run,
    Suppress,
}

impl TimerWaitPolicy {
    pub(crate) fn allow(self) -> bool {
        matches!(self, Self::Run)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SpecialInputWaitPolicy {
    ServiceOnly,
    CompleteOnAny,
    CompleteOnResize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WaitRequest {
    pub(crate) deadline: WaitDeadline,
    pub(crate) keyboard: KeyboardWaitPolicy,
    pub(crate) processes: ProcessWaitPolicy,
    pub(crate) timers: TimerWaitPolicy,
    pub(crate) redisplay: bool,
    pub(crate) special_input: SpecialInputWaitPolicy,
}

impl WaitRequest {
    pub(crate) fn accept_process_output(
        deadline: WaitDeadline,
        processes: ProcessWaitPolicy,
        timers: TimerWaitPolicy,
    ) -> Self {
        Self {
            deadline,
            keyboard: KeyboardWaitPolicy::YieldOnCommandInput,
            processes,
            timers,
            redisplay: false,
            special_input: SpecialInputWaitPolicy::ServiceOnly,
        }
    }

    pub(crate) fn read_command_input(deadline: WaitDeadline) -> Self {
        Self {
            deadline,
            keyboard: KeyboardWaitPolicy::ReadCommandInput,
            processes: ProcessWaitPolicy::ServiceAny,
            timers: TimerWaitPolicy::Run,
            redisplay: true,
            special_input: SpecialInputWaitPolicy::ServiceOnly,
        }
    }

    pub(crate) fn service_once(redisplay: bool) -> Self {
        Self {
            deadline: WaitDeadline::Poll,
            keyboard: KeyboardWaitPolicy::ServiceSpecialOnly,
            processes: ProcessWaitPolicy::ServiceAny,
            timers: TimerWaitPolicy::Run,
            redisplay,
            special_input: SpecialInputWaitPolicy::ServiceOnly,
        }
    }

    pub(crate) fn input_pending_poll(timers: TimerWaitPolicy) -> Self {
        Self {
            deadline: WaitDeadline::Poll,
            keyboard: KeyboardWaitPolicy::YieldOnCommandInput,
            processes: ProcessWaitPolicy::None,
            timers,
            redisplay: false,
            special_input: SpecialInputWaitPolicy::ServiceOnly,
        }
    }

    pub(crate) fn sleep_until(deadline: Instant) -> Self {
        Self {
            deadline: WaitDeadline::Until(deadline),
            keyboard: KeyboardWaitPolicy::ServiceSpecialOnly,
            processes: ProcessWaitPolicy::ServiceAny,
            timers: TimerWaitPolicy::Run,
            redisplay: false,
            special_input: SpecialInputWaitPolicy::ServiceOnly,
        }
    }

    pub(crate) fn resize_ack(deadline: Instant) -> Self {
        Self {
            deadline: WaitDeadline::Until(deadline),
            keyboard: KeyboardWaitPolicy::WaitForSpecialInput,
            processes: ProcessWaitPolicy::None,
            timers: TimerWaitPolicy::Suppress,
            redisplay: false,
            special_input: SpecialInputWaitPolicy::CompleteOnResize,
        }
    }

    fn completion_for(self, outcome: WaitServiceOutcome) -> Option<WaitCompletion> {
        if self.keyboard.completes_on_command_input() && outcome.command_input_pending {
            return Some(WaitCompletion::CommandInputPending);
        }

        if self.processes.satisfied_by(outcome) {
            return Some(WaitCompletion::ProcessActivity);
        }

        match self.special_input {
            SpecialInputWaitPolicy::CompleteOnAny if outcome.special_input_activity => {
                return Some(WaitCompletion::SpecialInputActivity);
            }
            SpecialInputWaitPolicy::CompleteOnResize if outcome.resize_activity => {
                return Some(WaitCompletion::SpecialInputActivity);
            }
            SpecialInputWaitPolicy::ServiceOnly
            | SpecialInputWaitPolicy::CompleteOnAny
            | SpecialInputWaitPolicy::CompleteOnResize => {}
        }

        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WaitCompletion {
    ProcessActivity,
    CommandInputPending,
    SpecialInputActivity,
    DeadlineElapsed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct WaitServiceOutcome {
    pub(crate) any_process_activity: bool,
    pub(crate) target_process_activity: bool,
    pub(crate) timers_fired: bool,
    pub(crate) command_input_pending: bool,
    pub(crate) special_input_activity: bool,
    pub(crate) resize_activity: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WaitOutcome {
    pub(crate) completion: WaitCompletion,
    pub(crate) service: WaitServiceOutcome,
}

impl super::eval::Context {
    pub(crate) fn service_wait_request_once(
        &mut self,
        request: &WaitRequest,
    ) -> Result<WaitServiceOutcome, Flow> {
        let mut outcome = WaitServiceOutcome::default();
        let special_input = self.service_wait_request_special_input_events()?;
        outcome.special_input_activity = special_input.activity;
        outcome.resize_activity = special_input.resize_activity;
        if request.keyboard.completes_on_command_input()
            && self.stage_pending_command_input_for_wait_request()?
        {
            outcome.command_input_pending = true;
            if request.redisplay && special_input.redisplay_needed {
                self.redisplay();
            }
            return Ok(outcome);
        }
        if request.timers.allow() {
            outcome.timers_fired = self.service_pending_timers_with_wait_policy(false);
        }
        let process_outcome = self.poll_process_output_with_wait_policy(request.processes);
        outcome.any_process_activity = process_outcome.any_process_activity;
        outcome.target_process_activity = process_outcome.target_process_activity;
        if request.redisplay && (special_input.redisplay_needed || outcome.timers_fired) {
            self.redisplay();
        }
        Ok(outcome)
    }

    pub(crate) fn service_wait_request_ready_processes(
        &mut self,
        request: &WaitRequest,
        ready_processes: Vec<ProcessId>,
    ) -> Result<WaitServiceOutcome, Flow> {
        let mut outcome = WaitServiceOutcome::default();
        let special_input = self.service_wait_request_special_input_events()?;
        outcome.special_input_activity = special_input.activity;
        outcome.resize_activity = special_input.resize_activity;
        if request.keyboard.completes_on_command_input()
            && self.stage_pending_command_input_for_wait_request()?
        {
            outcome.command_input_pending = true;
            if request.redisplay && special_input.redisplay_needed {
                self.redisplay();
            }
            return Ok(outcome);
        }
        if request.timers.allow() {
            outcome.timers_fired = self.service_pending_timers_with_wait_policy(false);
        }
        let process_outcome =
            self.poll_ready_process_output_with_wait_policy(ready_processes, request.processes);
        outcome.any_process_activity = process_outcome.any_process_activity;
        outcome.target_process_activity = process_outcome.target_process_activity;
        if request.redisplay && (special_input.redisplay_needed || outcome.timers_fired) {
            self.redisplay();
        }
        Ok(outcome)
    }

    pub(crate) fn wait_reading_process_output(
        &mut self,
        request: WaitRequest,
    ) -> Result<WaitOutcome, Flow> {
        let mut outcome = self.service_wait_request_once(&request)?;
        if let Some(completion) = request.completion_for(outcome) {
            return Ok(WaitOutcome {
                completion,
                service: outcome,
            });
        }
        if matches!(request.deadline, WaitDeadline::Poll)
            || request.deadline.expired(Instant::now())
        {
            return Ok(WaitOutcome {
                completion: WaitCompletion::DeadlineElapsed,
                service: outcome,
            });
        }

        loop {
            let now = Instant::now();
            if request.deadline.expired(now) {
                return Ok(WaitOutcome {
                    completion: WaitCompletion::DeadlineElapsed,
                    service: outcome,
                });
            }

            let wait_time = self.next_wait_request_timeout(&request, now);
            if wait_time.is_zero() {
                outcome = self.service_wait_request_once(&request)?;
            } else if self.wait_request_can_use_backend(&request) {
                let backend = self
                    .processes
                    .wait_for_backend_events(
                        wait_time,
                        WaitBackendInterest::for_wait_request(
                            request.keyboard.waits_for_host_input(),
                            request.processes.services_processes(),
                        ),
                    )
                    .unwrap_or_default();
                if backend.input_wakeup {
                    self.clear_input_wakeup_fd();
                    let _ = self.stage_next_host_input_event_if_available()?;
                }
                outcome =
                    self.service_wait_request_ready_processes(&request, backend.ready_processes)?;
            } else if request.keyboard.waits_for_host_input() && self.input_rx.is_some() {
                let _ = self.wait_for_next_host_input_event(
                    wait_time,
                    request.keyboard.sets_waiting_for_user_input(),
                )?;
                outcome = self.service_wait_request_once(&request)?;
            } else {
                let ready_processes = if request.processes.services_processes() {
                    self.processes.wait_for_output(wait_time)
                } else {
                    std::thread::sleep(wait_time);
                    Vec::new()
                };
                outcome = self.service_wait_request_ready_processes(&request, ready_processes)?;
            }

            if let Some(completion) = request.completion_for(outcome) {
                return Ok(WaitOutcome {
                    completion,
                    service: outcome,
                });
            }
        }
    }

    pub(crate) fn next_wait_request_timeout(
        &self,
        request: &WaitRequest,
        now: Instant,
    ) -> Duration {
        let mut timeout = request
            .deadline
            .remaining(now)
            .unwrap_or_else(|| Duration::from_millis(50))
            .min(Duration::from_millis(50));

        if request.timers.allow() {
            if let Some(next) = self.next_input_wait_timeout() {
                timeout = timeout.min(next);
            }
        }

        timeout
    }

    fn wait_request_can_use_backend(&self, request: &WaitRequest) -> bool {
        if !self.processes.has_input_wakeup_backend() {
            return false;
        }
        if request.processes.services_processes() {
            return true;
        }
        request.keyboard.waits_for_host_input() && self.processes.live_process_ids().is_empty()
    }
}

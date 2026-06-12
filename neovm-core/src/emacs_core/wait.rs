//! GNU-style wait policy for VM event servicing.
//!
//! GNU Emacs routes process waits, input waits, timer waits, and redisplay
//! through `wait_reading_process_output` with explicit policy flags.  This
//! module gives Neomacs the same shape: callers describe what may be serviced
//! and what should complete the wait; lower-level process/input code only
//! performs the service pass.

use std::time::{Duration, Instant};

use super::error::Flow;
use super::process::ProcessId;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct WaitBackendEvents {
    pub(crate) input_wakeup: bool,
    pub(crate) ready_processes: Vec<ProcessId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WaitBackendInterest {
    ProcessesOnly,
    InputWakeupOnly,
    InputWakeupAndProcesses,
}

impl WaitBackendInterest {
    pub(crate) fn processes_only() -> Self {
        Self::ProcessesOnly
    }

    pub(crate) fn input_wakeup_only() -> Self {
        Self::InputWakeupOnly
    }

    fn from_wait_flags(input_wakeup: bool, processes: bool) -> Option<Self> {
        match (input_wakeup, processes) {
            (true, true) => Some(Self::InputWakeupAndProcesses),
            (true, false) => Some(Self::InputWakeupOnly),
            (false, true) => Some(Self::ProcessesOnly),
            (false, false) => None,
        }
    }

    pub(crate) fn wants_input_wakeup(self) -> bool {
        matches!(self, Self::InputWakeupOnly | Self::InputWakeupAndProcesses)
    }

    pub(crate) fn wants_processes(self) -> bool {
        matches!(self, Self::ProcessesOnly | Self::InputWakeupAndProcesses)
    }
}

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
            Self::Any => outcome.has_any_process_activity(),
            Self::Target(_) | Self::TargetOnly(_) => outcome.has_target_process_activity(),
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

    fn backend_interest(self) -> Option<WaitBackendInterest> {
        WaitBackendInterest::from_wait_flags(
            self.keyboard.waits_for_host_input(),
            self.processes.services_processes(),
        )
    }

    fn completion_for(self, outcome: WaitServiceOutcome) -> Option<WaitCompletion> {
        if self.keyboard.completes_on_command_input() && outcome.command_input_pending {
            return Some(WaitCompletion::CommandInputPending);
        }

        if self.processes.satisfied_by(outcome) {
            return Some(WaitCompletion::ProcessActivity);
        }

        match self.special_input {
            SpecialInputWaitPolicy::CompleteOnAny if outcome.has_special_input_activity() => {
                return Some(WaitCompletion::SpecialInputActivity);
            }
            SpecialInputWaitPolicy::CompleteOnResize if outcome.has_resize_activity() => {
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
enum WaitProcessActivity {
    #[default]
    None,
    Any,
    Target,
}

impl WaitProcessActivity {
    fn record(self, target: bool) -> Self {
        if target {
            Self::Target
        } else if matches!(self, Self::Target) {
            Self::Target
        } else {
            Self::Any
        }
    }

    fn any(self) -> bool {
        matches!(self, Self::Any | Self::Target)
    }

    fn target(self) -> bool {
        matches!(self, Self::Target)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum WaitSpecialInputActivity {
    #[default]
    None,
    Any,
    Resize,
}

impl WaitSpecialInputActivity {
    pub(crate) fn record(self, activity: Self) -> Self {
        match (self, activity) {
            (Self::Resize, _) | (_, Self::Resize) => Self::Resize,
            (Self::Any, _) | (_, Self::Any) => Self::Any,
            (Self::None, Self::None) => Self::None,
        }
    }

    pub(crate) fn any(self) -> bool {
        matches!(self, Self::Any | Self::Resize)
    }

    pub(crate) fn resize(self) -> bool {
        matches!(self, Self::Resize)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct WaitServiceOutcome {
    process_activity: WaitProcessActivity,
    special_input_activity: WaitSpecialInputActivity,
    pub(crate) timers_fired: bool,
    pub(crate) command_input_pending: bool,
}

impl WaitServiceOutcome {
    pub(crate) fn record_process_activity(&mut self, target: bool) {
        self.process_activity = self.process_activity.record(target);
    }

    pub(crate) fn has_any_process_activity(self) -> bool {
        self.process_activity.any()
    }

    pub(crate) fn has_target_process_activity(self) -> bool {
        self.process_activity.target()
    }

    fn absorb_process_activity(&mut self, process_outcome: Self) {
        self.process_activity = process_outcome.process_activity;
    }

    pub(crate) fn record_special_input_activity(&mut self, activity: WaitSpecialInputActivity) {
        self.special_input_activity = self.special_input_activity.record(activity);
    }

    pub(crate) fn has_special_input_activity(self) -> bool {
        self.special_input_activity.any()
    }

    pub(crate) fn has_resize_activity(self) -> bool {
        self.special_input_activity.resize()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WaitOutcome {
    pub(crate) completion: WaitCompletion,
    pub(crate) service: WaitServiceOutcome,
}

#[derive(Debug, PartialEq, Eq)]
enum WaitProcessService {
    Poll,
    Ready(Vec<ProcessId>),
}

#[derive(Debug, PartialEq, Eq)]
enum WaitBlockStrategy {
    ServiceNow,
    Backend(WaitBackendInterest),
    HostInput,
    ProcessOutput,
    Sleep,
}

impl super::eval::Context {
    pub(crate) fn service_wait_request_once(
        &mut self,
        request: &WaitRequest,
    ) -> Result<WaitServiceOutcome, Flow> {
        self.service_wait_request_processes(request, WaitProcessService::Poll)
    }

    pub(crate) fn service_wait_request_ready_processes(
        &mut self,
        request: &WaitRequest,
        ready_processes: Vec<ProcessId>,
    ) -> Result<WaitServiceOutcome, Flow> {
        self.service_wait_request_processes(request, WaitProcessService::Ready(ready_processes))
    }

    fn service_wait_request_processes(
        &mut self,
        request: &WaitRequest,
        process_service: WaitProcessService,
    ) -> Result<WaitServiceOutcome, Flow> {
        let mut outcome = WaitServiceOutcome::default();
        let special_input = self.service_wait_request_special_input_events()?;
        outcome.record_special_input_activity(special_input.activity);
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
        let process_outcome = match process_service {
            WaitProcessService::Poll => {
                self.poll_process_output_with_wait_policy(request.processes)
            }
            WaitProcessService::Ready(ready_processes) => {
                self.poll_ready_process_output_with_wait_policy(ready_processes, request.processes)
            }
        };
        outcome.absorb_process_activity(process_outcome);
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
            match self.wait_block_strategy(&request, wait_time) {
                WaitBlockStrategy::ServiceNow => {
                    outcome = self.service_wait_request_once(&request)?;
                }
                WaitBlockStrategy::Backend(interest) => {
                    let backend = self
                        .processes
                        .wait_for_backend_events(wait_time, interest)
                        .unwrap_or_default();
                    if backend.input_wakeup {
                        self.clear_input_wakeup_fd();
                        let _ = self.stage_next_host_input_event_if_available()?;
                    }
                    outcome = self
                        .service_wait_request_ready_processes(&request, backend.ready_processes)?;
                }
                WaitBlockStrategy::HostInput => {
                    let _ = self.wait_for_next_host_input_event(
                        wait_time,
                        request.keyboard.sets_waiting_for_user_input(),
                    )?;
                    outcome = self.service_wait_request_once(&request)?;
                }
                WaitBlockStrategy::ProcessOutput => {
                    let ready_processes = self.processes.wait_for_output(wait_time);
                    outcome =
                        self.service_wait_request_ready_processes(&request, ready_processes)?;
                }
                WaitBlockStrategy::Sleep => {
                    std::thread::sleep(wait_time);
                    outcome = self.service_wait_request_ready_processes(&request, Vec::new())?;
                }
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

    fn wait_block_strategy(&self, request: &WaitRequest, wait_time: Duration) -> WaitBlockStrategy {
        if wait_time.is_zero() {
            return WaitBlockStrategy::ServiceNow;
        }

        if let Some(interest) = self.wait_backend_interest_for_request(request) {
            return WaitBlockStrategy::Backend(interest);
        }

        if request.keyboard.waits_for_host_input() && self.input_rx.is_some() {
            return WaitBlockStrategy::HostInput;
        }

        if request.processes.services_processes() {
            return WaitBlockStrategy::ProcessOutput;
        }

        WaitBlockStrategy::Sleep
    }

    fn wait_backend_interest_for_request(
        &self,
        request: &WaitRequest,
    ) -> Option<WaitBackendInterest> {
        if !self.processes.has_wait_input_wakeup_backend() {
            return None;
        }
        if request.processes.services_processes() {
            return request.backend_interest();
        }
        if request.keyboard.waits_for_host_input() && self.processes.live_process_ids().is_empty() {
            return request.backend_interest();
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_process_activity_implies_any_process_activity() {
        let mut outcome = WaitServiceOutcome::default();

        outcome.record_process_activity(true);

        assert!(outcome.has_target_process_activity());
        assert!(outcome.has_any_process_activity());
    }

    #[test]
    fn resize_special_input_activity_implies_any_special_input_activity() {
        let mut outcome = WaitServiceOutcome::default();

        outcome.record_special_input_activity(WaitSpecialInputActivity::Resize);

        assert!(outcome.has_resize_activity());
        assert!(outcome.has_special_input_activity());
    }
}

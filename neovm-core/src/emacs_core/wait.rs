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
pub(crate) struct WaitSourceEvents {
    input_wakeup: bool,
    ready_processes: Vec<ProcessId>,
}

impl WaitSourceEvents {
    pub(crate) fn from_sources(input_wakeup: bool, ready_processes: Vec<ProcessId>) -> Self {
        Self {
            input_wakeup,
            ready_processes,
        }
    }

    pub(crate) fn input_wakeup() -> Self {
        Self::from_sources(true, Vec::new())
    }

    pub(crate) fn ready_processes(processes: Vec<ProcessId>) -> Self {
        Self {
            input_wakeup: false,
            ready_processes: processes,
        }
    }

    pub(crate) fn has_input_wakeup(&self) -> bool {
        self.input_wakeup
    }

    pub(crate) fn has_ready_processes(&self) -> bool {
        !self.ready_processes.is_empty()
    }

    pub(crate) fn has_ready_process(&self, process: ProcessId) -> bool {
        self.ready_processes.contains(&process)
    }

    pub(crate) fn is_empty(&self) -> bool {
        !self.input_wakeup && self.ready_processes.is_empty()
    }

    fn into_process_service(self) -> WaitProcessService {
        WaitProcessService::Ready(self.ready_processes)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WaitDeadline {
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
pub(crate) enum ProcessOutputWaitTiming {
    Poll,
    For(Duration),
    Forever,
}

impl ProcessOutputWaitTiming {
    fn into_deadline(self) -> WaitDeadline {
        match self {
            Self::Poll => WaitDeadline::Poll,
            Self::For(duration) => WaitDeadline::Until(Instant::now() + duration),
            Self::Forever => WaitDeadline::Forever,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyboardWaitPolicy {
    ServiceSpecialOnly,
    WaitForSpecialInput,
    YieldOnCommandInput,
    ReadCommandInput,
}

impl KeyboardWaitPolicy {
    fn completes_on_command_input(self) -> bool {
        matches!(self, Self::YieldOnCommandInput | Self::ReadCommandInput)
    }

    fn waits_for_host_input(self) -> bool {
        matches!(
            self,
            Self::WaitForSpecialInput | Self::YieldOnCommandInput | Self::ReadCommandInput
        )
    }

    fn sets_waiting_for_user_input(self) -> bool {
        matches!(self, Self::ReadCommandInput)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProcessWaitPolicy {
    None,
    ServiceAny,
    Any,
    Target(ProcessId),
    TargetOnly(ProcessId),
}

impl ProcessWaitPolicy {
    fn target(process: ProcessId, just_this_one: bool) -> Self {
        if just_this_one {
            Self::TargetOnly(process)
        } else {
            Self::Target(process)
        }
    }

    fn target_process(self) -> Option<ProcessId> {
        match self {
            Self::Target(id) | Self::TargetOnly(id) => Some(id),
            Self::None | Self::ServiceAny | Self::Any => None,
        }
    }

    fn just_this_one(self) -> bool {
        matches!(self, Self::TargetOnly(_))
    }

    fn services_processes(self) -> bool {
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
enum TimerWaitPolicy {
    Run,
    Suppress,
}

impl TimerWaitPolicy {
    fn allow(self) -> bool {
        matches!(self, Self::Run)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpecialInputWaitPolicy {
    Suppress,
    ServiceOnly,
    CompleteOnAny,
    CompleteOnResize,
}

impl SpecialInputWaitPolicy {
    fn services_input(self) -> bool {
        !matches!(self, Self::Suppress)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WaitRequest {
    deadline: WaitDeadline,
    keyboard: KeyboardWaitPolicy,
    processes: ProcessWaitPolicy,
    timers: TimerWaitPolicy,
    redisplay: bool,
    special_input: SpecialInputWaitPolicy,
}

impl WaitRequest {
    fn accept_process_output(
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

    fn accept_process_output_with_timers(
        deadline: WaitDeadline,
        processes: ProcessWaitPolicy,
    ) -> Self {
        Self::accept_process_output(deadline, processes, TimerWaitPolicy::Run)
    }

    fn accept_process_output_without_timers(
        deadline: WaitDeadline,
        processes: ProcessWaitPolicy,
    ) -> Self {
        Self::accept_process_output(deadline, processes, TimerWaitPolicy::Suppress)
    }

    pub(crate) fn accept_any_process_output_with_timers(timing: ProcessOutputWaitTiming) -> Self {
        Self::accept_process_output_with_timers(timing.into_deadline(), ProcessWaitPolicy::Any)
    }

    pub(crate) fn accept_any_process_output_without_timers(
        timing: ProcessOutputWaitTiming,
    ) -> Self {
        Self::accept_process_output_without_timers(timing.into_deadline(), ProcessWaitPolicy::Any)
    }

    pub(crate) fn accept_target_process_output_with_timers(
        timing: ProcessOutputWaitTiming,
        process: ProcessId,
        just_this_one: bool,
    ) -> Self {
        Self::accept_process_output_with_timers(
            timing.into_deadline(),
            ProcessWaitPolicy::target(process, just_this_one),
        )
    }

    pub(crate) fn accept_target_process_output_without_timers(
        timing: ProcessOutputWaitTiming,
        process: ProcessId,
        just_this_one: bool,
    ) -> Self {
        Self::accept_process_output_without_timers(
            timing.into_deadline(),
            ProcessWaitPolicy::target(process, just_this_one),
        )
    }

    fn read_command_input(deadline: WaitDeadline) -> Self {
        Self {
            deadline,
            keyboard: KeyboardWaitPolicy::ReadCommandInput,
            processes: ProcessWaitPolicy::ServiceAny,
            timers: TimerWaitPolicy::Run,
            redisplay: true,
            special_input: SpecialInputWaitPolicy::ServiceOnly,
        }
    }

    pub(crate) fn read_command_input_until(deadline: Instant) -> Self {
        Self::read_command_input(WaitDeadline::Until(deadline))
    }

    pub(crate) fn read_command_input_forever() -> Self {
        Self::read_command_input(WaitDeadline::Forever)
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

    fn input_pending_poll(timers: TimerWaitPolicy) -> Self {
        Self {
            deadline: WaitDeadline::Poll,
            keyboard: KeyboardWaitPolicy::YieldOnCommandInput,
            processes: ProcessWaitPolicy::None,
            timers,
            redisplay: false,
            special_input: SpecialInputWaitPolicy::ServiceOnly,
        }
    }

    pub(crate) fn input_pending_without_timers() -> Self {
        Self::input_pending_poll(TimerWaitPolicy::Suppress)
    }

    pub(crate) fn input_pending_with_timers() -> Self {
        Self::input_pending_poll(TimerWaitPolicy::Run)
    }

    pub(crate) fn timer_service(redisplay: bool) -> Self {
        Self {
            deadline: WaitDeadline::Poll,
            keyboard: KeyboardWaitPolicy::ServiceSpecialOnly,
            processes: ProcessWaitPolicy::None,
            timers: TimerWaitPolicy::Run,
            redisplay,
            special_input: SpecialInputWaitPolicy::Suppress,
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

    fn deadline(self) -> WaitDeadline {
        self.deadline
    }

    pub(crate) fn deadline_is_poll(self) -> bool {
        matches!(self.deadline, WaitDeadline::Poll)
    }

    pub(crate) fn deadline_is_finite(self) -> bool {
        matches!(self.deadline, WaitDeadline::Until(_))
    }

    pub(crate) fn deadline_is_forever(self) -> bool {
        matches!(self.deadline, WaitDeadline::Forever)
    }

    pub(crate) fn target_process(self) -> Option<ProcessId> {
        self.processes.target_process()
    }

    pub(crate) fn completes_on_any_process_activity(self) -> bool {
        matches!(self.processes, ProcessWaitPolicy::Any)
    }

    pub(crate) fn completes_on_target_process_activity(self, process: ProcessId) -> bool {
        matches!(
            self.processes,
            ProcessWaitPolicy::Target(id) | ProcessWaitPolicy::TargetOnly(id) if id == process
        )
    }

    pub(crate) fn restricts_process_service_to_target(self) -> bool {
        self.processes.just_this_one()
    }

    pub(crate) fn services_process_output(self) -> bool {
        self.processes.services_processes()
    }

    fn services_special_input(self) -> bool {
        self.special_input.services_input()
    }

    fn waits_for_host_input(self) -> bool {
        self.keyboard.waits_for_host_input()
    }

    fn completes_on_command_input(self) -> bool {
        self.keyboard.completes_on_command_input()
    }

    fn sets_waiting_for_user_input(self) -> bool {
        self.keyboard.sets_waiting_for_user_input()
    }

    fn runs_timers(self) -> bool {
        self.timers.allow()
    }

    fn poll_or_deadline_elapsed(self, now: Instant) -> bool {
        matches!(self.deadline, WaitDeadline::Poll) || self.deadline.expired(now)
    }

    fn deadline_elapsed(self, now: Instant) -> bool {
        self.deadline.expired(now)
    }

    fn base_timeout(self, now: Instant) -> Duration {
        self.deadline
            .remaining(now)
            .unwrap_or_else(|| Duration::from_millis(50))
            .min(Duration::from_millis(50))
    }

    fn needs_redisplay_after_service(
        self,
        special_input: WaitSpecialInputOutcome,
        outcome: WaitServiceOutcome,
    ) -> bool {
        self.redisplay && (special_input.redisplay_needed() || outcome.has_timer_activity())
    }

    fn needs_redisplay_after_command_input(self, special_input: WaitSpecialInputOutcome) -> bool {
        self.redisplay && special_input.redisplay_needed()
    }

    fn completion_for(self, outcome: WaitServiceOutcome) -> Option<WaitCompletion> {
        if self.completes_on_command_input() && outcome.has_command_input_pending() {
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
            SpecialInputWaitPolicy::Suppress
            | SpecialInputWaitPolicy::ServiceOnly
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
pub(crate) struct WaitProcessOutcome {
    activity: WaitProcessActivity,
}

impl WaitProcessOutcome {
    pub(crate) fn record_activity(&mut self, target: bool) {
        self.activity = self.activity.record(target);
    }

    pub(crate) fn has_any_process_activity(self) -> bool {
        self.activity.any()
    }

    pub(crate) fn has_target_process_activity(self) -> bool {
        self.activity.target()
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
    timers_fired: bool,
    command_input_pending: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct WaitSpecialInputOutcome {
    redisplay_needed: bool,
    activity: WaitSpecialInputActivity,
}

impl WaitSpecialInputOutcome {
    pub(crate) fn record_activity(&mut self, activity: WaitSpecialInputActivity) {
        self.activity = self.activity.record(activity);
    }

    pub(crate) fn activity(self) -> WaitSpecialInputActivity {
        self.activity
    }

    pub(crate) fn request_redisplay(&mut self) {
        self.redisplay_needed = true;
    }

    pub(crate) fn redisplay_needed(self) -> bool {
        self.redisplay_needed
    }
}

impl WaitServiceOutcome {
    pub(crate) fn has_any_process_activity(self) -> bool {
        self.process_activity.any()
    }

    pub(crate) fn has_target_process_activity(self) -> bool {
        self.process_activity.target()
    }

    fn absorb_process_activity(&mut self, process_outcome: WaitProcessOutcome) {
        self.process_activity = process_outcome.activity;
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

    pub(crate) fn record_command_input_pending(&mut self) {
        self.command_input_pending = true;
    }

    pub(crate) fn has_command_input_pending(self) -> bool {
        self.command_input_pending
    }

    pub(crate) fn record_timer_activity(&mut self, fired: bool) {
        self.timers_fired |= fired;
    }

    pub(crate) fn has_timer_activity(self) -> bool {
        self.timers_fired
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WaitOutcome {
    completion: WaitCompletion,
    service: WaitServiceOutcome,
}

impl WaitOutcome {
    pub(crate) fn completion(self) -> WaitCompletion {
        self.completion
    }

    pub(crate) fn service(self) -> WaitServiceOutcome {
        self.service
    }
}

#[derive(Debug, PartialEq, Eq)]
enum WaitProcessService {
    Poll,
    Ready(Vec<ProcessId>),
}

#[derive(Debug, PartialEq, Eq)]
struct WaitBlockActivity {
    input_wakeup: bool,
    process_service: WaitProcessService,
}

impl WaitBlockActivity {
    fn poll() -> Self {
        Self {
            input_wakeup: false,
            process_service: WaitProcessService::Poll,
        }
    }

    fn ready_processes(processes: Vec<ProcessId>) -> Self {
        Self {
            input_wakeup: false,
            process_service: WaitProcessService::Ready(processes),
        }
    }

    fn from_source_events(events: WaitSourceEvents) -> Self {
        Self {
            input_wakeup: events.has_input_wakeup(),
            process_service: events.into_process_service(),
        }
    }

    fn has_input_wakeup(&self) -> bool {
        self.input_wakeup
    }

    fn into_process_service(self) -> WaitProcessService {
        self.process_service
    }
}

#[derive(Debug, PartialEq, Eq)]
enum WaitBlockStrategy {
    ServiceNow,
    BackendInputWakeup,
    BackendProcesses,
    BackendInputWakeupAndProcesses,
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

    pub(crate) fn service_wait_request_source_events(
        &mut self,
        request: &WaitRequest,
        events: WaitSourceEvents,
    ) -> Result<WaitServiceOutcome, Flow> {
        self.service_wait_request_block_activity(
            request,
            WaitBlockActivity::from_source_events(events),
        )
    }

    fn service_wait_request_block_activity(
        &mut self,
        request: &WaitRequest,
        activity: WaitBlockActivity,
    ) -> Result<WaitServiceOutcome, Flow> {
        if activity.has_input_wakeup() {
            self.clear_input_wakeup_fd();
            let _ = self.stage_next_host_input_event_if_available()?;
        }
        self.service_wait_request_processes(request, activity.into_process_service())
    }

    fn service_wait_request_processes(
        &mut self,
        request: &WaitRequest,
        process_service: WaitProcessService,
    ) -> Result<WaitServiceOutcome, Flow> {
        let mut outcome = WaitServiceOutcome::default();
        let special_input = if request.services_special_input() {
            self.service_wait_request_special_input_events()?
        } else {
            WaitSpecialInputOutcome::default()
        };
        outcome.record_special_input_activity(special_input.activity());
        if request.completes_on_command_input()
            && self.stage_pending_command_input_for_wait_request()?
        {
            outcome.record_command_input_pending();
            if request.needs_redisplay_after_command_input(special_input) {
                self.redisplay();
            }
            return Ok(outcome);
        }
        if request.runs_timers() {
            outcome.record_timer_activity(self.service_pending_timers_with_wait_policy(false));
        }
        let process_outcome = match process_service {
            WaitProcessService::Poll => self.poll_process_output_for_wait_request(request),
            WaitProcessService::Ready(ready_processes) => {
                self.poll_ready_process_output_for_wait_request(ready_processes, request)
            }
        };
        outcome.absorb_process_activity(process_outcome);
        if request.needs_redisplay_after_service(special_input, outcome) {
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
        if request.poll_or_deadline_elapsed(Instant::now()) {
            return Ok(WaitOutcome {
                completion: WaitCompletion::DeadlineElapsed,
                service: outcome,
            });
        }

        loop {
            let now = Instant::now();
            if request.deadline_elapsed(now) {
                return Ok(WaitOutcome {
                    completion: WaitCompletion::DeadlineElapsed,
                    service: outcome,
                });
            }

            let wait_time = self.next_wait_request_timeout(&request, now);
            let activity = self.block_for_wait_request(&request, wait_time)?;
            outcome = self.service_wait_request_block_activity(&request, activity)?;

            if let Some(completion) = request.completion_for(outcome) {
                return Ok(WaitOutcome {
                    completion,
                    service: outcome,
                });
            }
        }
    }

    fn block_for_wait_request(
        &mut self,
        request: &WaitRequest,
        wait_time: Duration,
    ) -> Result<WaitBlockActivity, Flow> {
        match self.wait_block_strategy(request, wait_time) {
            WaitBlockStrategy::ServiceNow => Ok(WaitBlockActivity::poll()),
            WaitBlockStrategy::BackendInputWakeup => {
                let events = self
                    .processes
                    .wait_for_input_wakeup_events(wait_time)
                    .unwrap_or_default();
                Ok(WaitBlockActivity::from_source_events(events))
            }
            WaitBlockStrategy::BackendProcesses => {
                let events = self
                    .processes
                    .wait_for_process_backend_events(wait_time)
                    .unwrap_or_default();
                Ok(WaitBlockActivity::from_source_events(events))
            }
            WaitBlockStrategy::BackendInputWakeupAndProcesses => {
                let events = self
                    .processes
                    .wait_for_input_wakeup_or_process_events(wait_time)
                    .unwrap_or_default();
                Ok(WaitBlockActivity::from_source_events(events))
            }
            WaitBlockStrategy::HostInput => {
                let _ = self.wait_for_next_host_input_event(
                    wait_time,
                    request.sets_waiting_for_user_input(),
                )?;
                Ok(WaitBlockActivity::poll())
            }
            WaitBlockStrategy::ProcessOutput => {
                let events = self.processes.wait_for_process_events(wait_time);
                Ok(WaitBlockActivity::from_source_events(events))
            }
            WaitBlockStrategy::Sleep => {
                std::thread::sleep(wait_time);
                Ok(WaitBlockActivity::ready_processes(Vec::new()))
            }
        }
    }

    pub(crate) fn next_wait_request_timeout(
        &self,
        request: &WaitRequest,
        now: Instant,
    ) -> Duration {
        let mut timeout = request.base_timeout(now);

        if request.runs_timers() {
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

        if let Some(strategy) = self.wait_backend_interest_for_request(request) {
            return strategy;
        }

        if request.waits_for_host_input() && self.input_rx.is_some() {
            return WaitBlockStrategy::HostInput;
        }

        if request.services_process_output() {
            return WaitBlockStrategy::ProcessOutput;
        }

        WaitBlockStrategy::Sleep
    }

    fn wait_backend_interest_for_request(
        &self,
        request: &WaitRequest,
    ) -> Option<WaitBlockStrategy> {
        if !self.processes.has_wait_input_wakeup_backend() {
            return None;
        }
        if request.services_process_output() {
            return if request.waits_for_host_input() {
                Some(WaitBlockStrategy::BackendInputWakeupAndProcesses)
            } else {
                Some(WaitBlockStrategy::BackendProcesses)
            };
        }
        if request.waits_for_host_input() && self.processes.live_process_ids().is_empty() {
            return Some(WaitBlockStrategy::BackendInputWakeup);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_process_activity_implies_any_process_activity() {
        let mut outcome = WaitProcessOutcome::default();

        outcome.record_activity(true);

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

    #[test]
    fn special_input_outcome_records_activity_explicitly() {
        let mut outcome = WaitSpecialInputOutcome::default();

        outcome.record_activity(WaitSpecialInputActivity::Resize);

        assert_eq!(outcome.activity(), WaitSpecialInputActivity::Resize);
    }

    #[test]
    fn special_input_outcome_records_redisplay_explicitly() {
        let mut outcome = WaitSpecialInputOutcome::default();

        outcome.request_redisplay();

        assert!(outcome.redisplay_needed());
    }

    #[test]
    fn command_input_pending_is_recorded_explicitly() {
        let mut outcome = WaitServiceOutcome::default();

        outcome.record_command_input_pending();

        assert!(outcome.has_command_input_pending());
    }

    #[test]
    fn timer_activity_is_recorded_explicitly() {
        let mut outcome = WaitServiceOutcome::default();

        outcome.record_timer_activity(true);

        assert!(outcome.has_timer_activity());
    }

    #[test]
    fn source_events_construct_input_wakeup_explicitly() {
        let events = WaitSourceEvents::input_wakeup();

        assert!(events.has_input_wakeup());
        assert!(!events.has_ready_processes());
    }

    #[test]
    fn source_events_construct_ready_processes_explicitly() {
        let events = WaitSourceEvents::ready_processes(vec![7]);

        assert!(!events.has_input_wakeup());
        assert!(events.has_ready_process(7));
    }

    #[test]
    fn source_events_query_individual_ready_processes() {
        let events = WaitSourceEvents::ready_processes(vec![7]);

        assert!(events.has_ready_process(7));
        assert!(!events.has_ready_process(8));
    }

    #[test]
    fn source_events_empty_query_reflects_recorded_activity() {
        let empty = WaitSourceEvents::default();
        let ready = WaitSourceEvents::ready_processes(vec![7]);

        assert!(empty.is_empty());
        assert!(!ready.is_empty());
    }

    #[test]
    fn source_events_convert_to_process_service() {
        let events = WaitSourceEvents::ready_processes(vec![7]);

        assert_eq!(
            events.into_process_service(),
            WaitProcessService::Ready(vec![7])
        );
    }

    #[test]
    fn block_activity_from_source_events_preserves_wakeup_and_processes() {
        let events = WaitSourceEvents::from_sources(true, vec![3]);

        let activity = WaitBlockActivity::from_source_events(events);

        assert!(activity.has_input_wakeup());
        assert_eq!(
            activity.into_process_service(),
            WaitProcessService::Ready(vec![3])
        );
    }

    #[test]
    fn block_activity_from_ready_processes_has_no_input_wakeup() {
        let activity = WaitBlockActivity::ready_processes(vec![4, 9]);

        assert!(!activity.has_input_wakeup());
        assert_eq!(
            activity.into_process_service(),
            WaitProcessService::Ready(vec![4, 9])
        );
    }

    #[test]
    fn context_services_source_events_directly() {
        let mut context = crate::emacs_core::eval::Context::new();
        let request = WaitRequest::service_once(false);

        let outcome = context
            .service_wait_request_source_events(&request, WaitSourceEvents::default())
            .expect("service source events");

        assert!(!outcome.has_command_input_pending());
        assert!(!outcome.has_any_process_activity());
    }

    #[test]
    fn block_for_wait_request_zero_timeout_returns_poll_activity() {
        let mut context = crate::emacs_core::eval::Context::new();
        let request = WaitRequest::service_once(false);

        let activity = context
            .block_for_wait_request(&request, Duration::ZERO)
            .expect("block for wait request");

        assert!(!activity.has_input_wakeup());
        assert_eq!(activity.into_process_service(), WaitProcessService::Poll);
    }

    #[test]
    fn wait_request_exposes_deadline_and_process_completion_queries() {
        let request = WaitRequest::accept_target_process_output_with_timers(
            ProcessOutputWaitTiming::Poll,
            12,
            false,
        );

        assert_eq!(request.deadline(), WaitDeadline::Poll);
        assert_eq!(request.target_process(), Some(12));
        assert!(request.completes_on_target_process_activity(12));
        assert!(!request.completes_on_any_process_activity());
        assert!(!request.restricts_process_service_to_target());
    }

    #[test]
    fn wait_request_accept_process_output_constructors_capture_timer_policy() {
        let run = WaitRequest::accept_any_process_output_with_timers(ProcessOutputWaitTiming::Poll);
        let suppress =
            WaitRequest::accept_any_process_output_without_timers(ProcessOutputWaitTiming::Poll);

        assert!(run.runs_timers());
        assert!(!suppress.runs_timers());
    }

    #[test]
    fn wait_request_accept_process_output_named_constructors_capture_process_scope() {
        let any = WaitRequest::accept_any_process_output_with_timers(ProcessOutputWaitTiming::Poll);
        let target = WaitRequest::accept_target_process_output_with_timers(
            ProcessOutputWaitTiming::Poll,
            7,
            false,
        );
        let target_only = WaitRequest::accept_target_process_output_without_timers(
            ProcessOutputWaitTiming::Forever,
            9,
            true,
        );

        assert!(any.completes_on_any_process_activity());
        assert_eq!(any.target_process(), None);
        assert!(target.completes_on_target_process_activity(7));
        assert!(!target.restricts_process_service_to_target());
        assert!(target_only.completes_on_target_process_activity(9));
        assert!(target_only.restricts_process_service_to_target());
        assert!(!target_only.runs_timers());
        assert!(target_only.deadline_is_forever());
    }

    #[test]
    fn wait_request_process_output_timing_converts_duration_to_finite_deadline() {
        let request = WaitRequest::accept_any_process_output_with_timers(
            ProcessOutputWaitTiming::For(Duration::from_millis(5)),
        );

        assert!(request.deadline_is_finite());
    }

    #[test]
    fn wait_request_timer_service_suppresses_special_input_and_processes() {
        let request = WaitRequest::timer_service(true);

        assert_eq!(request.deadline(), WaitDeadline::Poll);
        assert_eq!(request.target_process(), None);
        assert!(!request.completes_on_any_process_activity());
        assert!(!request.services_special_input());
    }

    #[test]
    fn wait_request_input_pending_constructors_capture_timer_policy() {
        let suppress = WaitRequest::input_pending_without_timers();
        let run = WaitRequest::input_pending_with_timers();

        assert!(!suppress.runs_timers());
        assert!(run.runs_timers());
    }

    #[test]
    fn wait_request_exposes_scheduler_queries() {
        let now = Instant::now();
        let read = WaitRequest::read_command_input_until(now + Duration::from_secs(1));
        let poll = WaitRequest::service_once(true);
        let resize = WaitRequest::resize_ack(now);

        assert!(read.waits_for_host_input());
        assert!(read.completes_on_command_input());
        assert!(read.sets_waiting_for_user_input());
        assert!(read.runs_timers());
        assert!(!read.poll_or_deadline_elapsed(now));
        assert_eq!(
            read.base_timeout(now + Duration::from_secs(2)),
            Duration::ZERO
        );

        assert!(!poll.waits_for_host_input());
        assert!(!poll.completes_on_command_input());
        assert!(poll.poll_or_deadline_elapsed(now));

        assert!(resize.waits_for_host_input());
        assert!(!resize.runs_timers());
    }

    #[test]
    fn wait_request_redisplay_query_tracks_request_and_activity() {
        let redisplay = WaitRequest::service_once(true);
        let quiet = WaitRequest::service_once(false);
        let mut special = WaitSpecialInputOutcome::default();
        let mut service = WaitServiceOutcome::default();

        assert!(!redisplay.needs_redisplay_after_service(special, service));

        special.request_redisplay();
        assert!(redisplay.needs_redisplay_after_service(special, service));
        assert!(!quiet.needs_redisplay_after_service(special, service));

        special = WaitSpecialInputOutcome::default();
        service.record_timer_activity(true);
        assert!(redisplay.needs_redisplay_after_service(special, service));
    }

    #[test]
    fn wait_outcome_exposes_completion_and_service_queries() {
        let mut service = WaitServiceOutcome::default();
        service.record_command_input_pending();
        let outcome = WaitOutcome {
            completion: WaitCompletion::CommandInputPending,
            service,
        };

        assert_eq!(outcome.completion(), WaitCompletion::CommandInputPending);
        assert!(outcome.service().has_command_input_pending());
    }
}

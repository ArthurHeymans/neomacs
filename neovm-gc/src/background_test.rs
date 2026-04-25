use super::*;
use std::collections::VecDeque;

fn major_reclaim_plan() -> CollectionPlan {
    CollectionPlan {
        kind: CollectionKind::Major,
        phase: crate::plan::CollectionPhase::Reclaim,
        concurrent: true,
        parallel: true,
        worker_count: 1,
        mark_slice_budget: 1,
        target_old_regions: 0,
        selected_old_blocks: Vec::new(),
        estimated_compaction_bytes: 0,
        estimated_reclaim_bytes: 0,
    }
}

fn completed_progress() -> MajorMarkProgress {
    MajorMarkProgress {
        completed: true,
        drained_objects: 1,
        elapsed_nanos: 1,
        mark_steps: 1,
        mark_rounds: 1,
        remaining_work: 0,
    }
}

#[derive(Debug)]
struct FakeRuntime {
    active_plan: Option<CollectionPlan>,
    recommended_plan: Option<CollectionPlan>,
    poll_results: VecDeque<Result<Option<MajorMarkProgress>, AllocError>>,
    prepare_results: VecDeque<Result<bool, AllocError>>,
    advance_results: VecDeque<Result<Option<CollectionStats>, AllocError>>,
    begin_calls: usize,
    prepare_calls: usize,
    advance_calls: usize,
    commit_calls: usize,
    finish_calls: usize,
}

impl FakeRuntime {
    fn reclaim_ready() -> Self {
        Self {
            active_plan: Some(major_reclaim_plan()),
            recommended_plan: None,
            poll_results: VecDeque::new(),
            prepare_results: VecDeque::new(),
            advance_results: VecDeque::new(),
            begin_calls: 0,
            prepare_calls: 0,
            advance_calls: 0,
            commit_calls: 0,
            finish_calls: 0,
        }
    }
}

impl BackgroundCollectionRuntime for FakeRuntime {
    fn active_major_mark_plan(&self) -> Option<CollectionPlan> {
        self.active_plan.clone()
    }

    fn recommended_background_plan(&self) -> Option<CollectionPlan> {
        self.recommended_plan.clone()
    }

    fn begin_major_mark(&mut self, plan: CollectionPlan) -> Result<(), AllocError> {
        self.begin_calls = self.begin_calls.saturating_add(1);
        self.active_plan = Some(plan);
        Ok(())
    }

    fn poll_background_mark_round(&mut self) -> Result<Option<MajorMarkProgress>, AllocError> {
        self.poll_results
            .pop_front()
            .unwrap_or_else(|| Ok(Some(completed_progress())))
    }

    fn prepare_active_reclaim_if_needed(&mut self) -> Result<bool, AllocError> {
        self.prepare_calls = self.prepare_calls.saturating_add(1);
        self.prepare_results.pop_front().unwrap_or(Ok(false))
    }

    fn advance_active_reclaim_commit(&mut self) -> Result<Option<CollectionStats>, AllocError> {
        self.advance_calls = self.advance_calls.saturating_add(1);
        let result = self.advance_results.pop_front().unwrap_or(Ok(None));
        if matches!(result, Ok(Some(_))) {
            self.active_plan = None;
        }
        result
    }

    fn commit_active_reclaim_if_ready(&mut self) -> Result<Option<CollectionStats>, AllocError> {
        self.commit_calls = self.commit_calls.saturating_add(1);
        panic!("coordinator should use bounded reclaim assists, not one-shot commit")
    }

    fn finish_active_major_collection_if_ready(
        &mut self,
    ) -> Result<Option<CollectionStats>, AllocError> {
        self.finish_calls = self.finish_calls.saturating_add(1);
        Ok(None)
    }
}

#[test]
fn tick_uses_bounded_reclaim_assist_when_auto_finish_is_enabled() {
    let mut runtime = FakeRuntime::reclaim_ready();
    runtime
        .poll_results
        .push_back(Ok(Some(completed_progress())));
    runtime.prepare_results.push_back(Ok(false));
    runtime.advance_results.push_back(Ok(None));

    let mut collector = BackgroundCollector::new(BackgroundCollectorConfig::default());
    let status = collector
        .tick(&mut runtime)
        .expect("background tick should succeed");

    assert!(matches!(
        status,
        BackgroundCollectionStatus::ReadyToFinish(progress) if progress.completed
    ));
    assert_eq!(runtime.prepare_calls, 1);
    assert_eq!(runtime.advance_calls, 1);
    assert_eq!(runtime.commit_calls, 0);
    assert_eq!(runtime.finish_calls, 0);
}

#[test]
fn repeated_ticks_finish_after_multiple_reclaim_assists() {
    let mut runtime = FakeRuntime::reclaim_ready();
    runtime
        .poll_results
        .push_back(Ok(Some(completed_progress())));
    runtime
        .poll_results
        .push_back(Ok(Some(completed_progress())));
    runtime.prepare_results.push_back(Ok(false));
    runtime.prepare_results.push_back(Ok(false));
    runtime.advance_results.push_back(Ok(None));
    runtime.advance_results.push_back(Ok(Some(CollectionStats {
        collections: 1,
        major_collections: 1,
        ..CollectionStats::default()
    })));

    let mut collector = BackgroundCollector::new(BackgroundCollectorConfig::default());
    let first = collector
        .tick(&mut runtime)
        .expect("first tick should succeed");
    assert!(matches!(
        first,
        BackgroundCollectionStatus::ReadyToFinish(_)
    ));

    let second = collector
        .tick(&mut runtime)
        .expect("second tick should succeed");
    assert!(matches!(
        second,
        BackgroundCollectionStatus::Finished(CollectionStats {
            collections: 1,
            major_collections: 1,
            ..
        })
    ));
    assert_eq!(runtime.advance_calls, 2);
    assert_eq!(collector.stats().sessions_finished, 1);
}

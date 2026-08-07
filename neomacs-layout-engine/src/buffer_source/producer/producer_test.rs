use super::*;
use crate::display_source::DisplaySourceContext;
use crate::neovm_bridge::LayoutBufferSnapshot;
use neovm_core::emacs_core::Context;

/// One consumed element plus the walk position it left behind: the pair the
/// pipeline actually observes per loop iteration.
type ConsumedStep = (BufferSourceConsumedItem, DisplaySourceTextPosition);

fn buffer_snapshot(text: &str) -> (BufferId, LayoutBufferSnapshot) {
    let mut eval = Context::new();
    let buffer_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = eval
            .buffer_manager_mut()
            .get_mut(buffer_id)
            .expect("buffer");
        buffer.insert(text);
    }
    let buffer = eval.buffer_manager().get(buffer_id).expect("buffer");
    (buffer_id, LayoutBufferSnapshot::from_buffer(buffer))
}

fn producer<'a>(
    buffer_id: BufferId,
    snapshot: &'a LayoutBufferSnapshot,
) -> BufferElementProducer<'a, LayoutBufferSnapshot> {
    BufferElementProducer::new(buffer_id, snapshot, 0, 0)
}

/// Consume one element, advancing the walk position the way the render loop
/// does (`progress` publishes the item's end charpos).
fn consume_one<B: LayoutBufferView>(
    producer: &mut BufferElementProducer<'_, B>,
    position: &mut DisplaySourceTextPosition,
) -> Option<BufferSourceConsumedItem> {
    let mut context = DisplaySourceContext::empty();
    let item = producer.next_consumed_item(&mut context, position)?;
    if let BufferSourceConsumedItem::Renderable(step) = &item {
        *position = position.with_charpos(step.end_charpos());
    }
    Some(item)
}

fn drain<B: LayoutBufferView>(
    producer: &mut BufferElementProducer<'_, B>,
    position: &mut DisplaySourceTextPosition,
    count: usize,
) -> Vec<ConsumedStep> {
    let mut steps = Vec::new();
    for _ in 0..count {
        let Some(item) = consume_one(producer, position) else {
            break;
        };
        steps.push((item, *position));
    }
    steps
}

/// Drive a producer to the wrap candidate `candidate`, consume past it (the
/// overflow attempt), queue a split remainder the way the renderer does, and
/// then hand the seated producer back for the wrap retry.
fn walk_to_wrap_candidate<'a>(
    buffer_id: BufferId,
    snapshot: &'a LayoutBufferSnapshot,
    candidate: DisplaySourceTextPosition,
) -> (
    BufferElementProducer<'a, LayoutBufferSnapshot>,
    DisplaySourceTextPosition,
) {
    let mut producer = producer(buffer_id, snapshot);
    let mut position = candidate;
    let overflow_attempt = drain(&mut producer, &mut position, 2);
    // The renderer's per-char / fit split: the tail of the consumed run goes
    // back into the producer's pending queue at a position past the candidate.
    let remainder: Vec<_> = overflow_attempt
        .iter()
        .filter_map(|(item, _)| match item {
            BufferSourceConsumedItem::Renderable(step) => Some(step.clone()),
            BufferSourceConsumedItem::DisplayPropertyReplacement(_) => None,
        })
        .collect();
    assert!(
        !remainder.is_empty(),
        "wrap corpus must produce renderable items before the retry"
    );
    producer.prepend_pending_render_items(remainder);
    assert!(producer.has_pending_render_items());
    (producer, candidate)
}

/// The pre-extraction `rewind_source_consumption_to`, verbatim: clear the
/// pending queue, reseat the cursor at the retry position. The reference the
/// snapshot/restore round trip is characterized against, written out here so
/// the comparison does not run through `restore` itself.
fn legacy_rewind<B: LayoutBufferView>(
    producer: &mut BufferElementProducer<'_, B>,
    source_position: DisplaySourceTextPosition,
) {
    producer.source_consumption.clear_pending_render_items();
    producer
        .source_cursor
        .reset_to(CharPos0::new(source_position.charpos().max(0) as usize));
}

/// The wrap retry restored from a snapshot taken AT the candidate must produce
/// exactly the stream today's `rewind_source_consumption_to` produces (GNU
/// `SAVE_IT` / `RESTORE_IT`).
fn assert_snapshot_restore_matches_rewind(text: &str, candidate: DisplaySourceTextPosition) {
    let (buffer_id, snapshot) = buffer_snapshot(text);

    // The seating saved at the wrap candidate, before the overflow attempt.
    let saved = {
        let mut seated = producer(buffer_id, &snapshot);
        seated.rewind_to(candidate);
        seated.snapshot()
    };

    let (mut restored, mut restore_position) =
        walk_to_wrap_candidate(buffer_id, &snapshot, candidate);
    restored.restore(saved);
    let restored_stream = drain(&mut restored, &mut restore_position, 8);

    let (mut rewound, mut rewind_position) =
        walk_to_wrap_candidate(buffer_id, &snapshot, candidate);
    legacy_rewind(&mut rewound, candidate);
    let rewound_stream = drain(&mut rewound, &mut rewind_position, 8);

    assert!(
        !restored_stream.is_empty(),
        "the retry must re-produce elements from the candidate"
    );
    let (first, _) = &restored_stream[0];
    let BufferSourceConsumedItem::Renderable(step) = first else {
        panic!("the wrap corpus is plain text");
    };
    assert_eq!(step.source_step_char().start_charpos(), candidate.charpos());
    assert_eq!(restored_stream, rewound_stream);
    assert_eq!(restore_position, rewind_position);
}

#[test]
fn producer_snapshot_restore_matches_rewind_on_a_character_wrap_candidate() {
    // A single long line whose wrap candidate falls mid-run.
    assert_snapshot_restore_matches_rewind(
        "abcdefghijklmnopqrstuvwxyz\nsecond line\n",
        DisplaySourceTextPosition::new(6, 6),
    );
}

#[test]
fn producer_snapshot_restore_matches_rewind_on_a_word_wrap_candidate() {
    // Word wrap breaks at the space before "wrapping"; the candidate is the
    // break char itself.
    assert_snapshot_restore_matches_rewind(
        "hello world wrapping line of text\ntail\n",
        DisplaySourceTextPosition::new(11, 11),
    );
}

#[test]
fn producer_snapshot_preserves_a_pending_queue_the_rewind_would_drop() {
    // Snapshot/restore is state-faithful where the rewind is position-only:
    // restoring a snapshot taken with queued remainders brings them back,
    // which is what lets P4.3 delete the fit split.
    let (buffer_id, snapshot) = buffer_snapshot("abcdef\nghi\n");
    let mut producer = producer(buffer_id, &snapshot);
    let mut position = DisplaySourceTextPosition::new(0, 0);
    let first = consume_one(&mut producer, &mut position).expect("leading run");
    let BufferSourceConsumedItem::Renderable(step) = first else {
        panic!("plain text must produce a renderable run");
    };

    producer.prepend_pending_render_items(vec![step]);
    let saved = producer.snapshot();
    assert_eq!(producer.pending_render_items_len(), 1);

    legacy_rewind(&mut producer, DisplaySourceTextPosition::new(0, 0));
    assert!(!producer.has_pending_render_items());

    producer.restore(saved);
    assert_eq!(producer.pending_render_items_len(), 1);
}

#[test]
fn producer_rewind_matches_the_pre_extraction_rewind() {
    // `rewind_to` is `restore` of a synthesized seating; it must still be the
    // old clear-queue-and-reseat, byte for byte.
    let (buffer_id, snapshot) = buffer_snapshot("abcdef\nghi\n");
    let candidate = DisplaySourceTextPosition::new(3, 3);

    let (mut rewound, _) = walk_to_wrap_candidate(buffer_id, &snapshot, candidate);
    rewound.rewind_to(candidate);

    let (mut legacy, _) = walk_to_wrap_candidate(buffer_id, &snapshot, candidate);
    legacy_rewind(&mut legacy, candidate);

    assert_eq!(rewound.snapshot(), legacy.snapshot());
}

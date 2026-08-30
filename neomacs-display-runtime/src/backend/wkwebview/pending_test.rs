use super::{CAPACITY, PendingCommand, PendingCommands};

fn create(id: u32) -> PendingCommand {
    PendingCommand::Create {
        id,
        width: 100.0,
        height: 50.0,
    }
}

fn load(id: u32, url: &str) -> PendingCommand {
    PendingCommand::LoadUri {
        id,
        url: url.to_string(),
    }
}

fn script(id: u32, script: &str) -> PendingCommand {
    PendingCommand::Script {
        id,
        script: script.to_string(),
    }
}

#[test]
fn a_fresh_queue_is_empty() {
    assert!(PendingCommands::new().is_empty());
}

/// Review catch (PR #297): only the create size used to be kept, so a
/// `Create` -> `LoadUri` pair issued before the window existed produced an
/// empty `WKWebView`. The URL has to survive, and it has to arrive after the
/// create.
#[test]
fn a_create_and_the_load_that_follows_it_both_survive() {
    let mut pending = PendingCommands::new();
    pending.push(create(1));
    pending.push(load(1, "https://example.invalid/"));

    assert_eq!(
        pending.take(),
        vec![create(1), load(1, "https://example.invalid/")]
    );
}

/// Replay order is the order the commands were issued in. A script that runs
/// before its page is loaded is a no-op against the wrong document, so this
/// is the property the whole queue exists for.
#[test]
fn commands_replay_in_arrival_order() {
    let mut pending = PendingCommands::new();
    pending.push(create(1));
    pending.push(load(1, "https://example.invalid/"));
    pending.push(script(1, "scrollTo(0, 100)"));

    let replayed = pending.take();
    assert_eq!(replayed.len(), 3);
    assert!(matches!(replayed[0], PendingCommand::Create { .. }));
    assert!(matches!(replayed[1], PendingCommand::LoadUri { .. }));
    assert!(matches!(replayed[2], PendingCommand::Script { .. }));
}

/// Two views interleaved keep their relative order too, which is why this is
/// one queue rather than one per id.
#[test]
fn interleaved_ids_keep_their_relative_order() {
    let mut pending = PendingCommands::new();
    pending.push(create(1));
    pending.push(create(2));
    pending.push(load(2, "b"));
    pending.push(load(1, "a"));

    let ids: Vec<u32> = pending.take().iter().map(PendingCommand::id).collect();
    assert_eq!(ids, vec![1, 2, 2, 1]);
}

/// A resize before attach used to be lost entirely -- the view was built from
/// the stale create size and the resize was never replayed.
#[test]
fn a_resize_before_attach_is_replayed_after_the_create() {
    let mut pending = PendingCommands::new();
    pending.push(create(1));
    pending.push(PendingCommand::Resize {
        id: 1,
        width: 640.0,
        height: 480.0,
    });

    assert_eq!(
        pending.take().last(),
        Some(&PendingCommand::Resize {
            id: 1,
            width: 640.0,
            height: 480.0,
        })
    );
}

/// An xwidget killed before the window existed must not be built by the
/// replay, and must take its queued commands with it.
#[test]
fn forgetting_an_id_drops_only_that_ids_commands() {
    let mut pending = PendingCommands::new();
    pending.push(create(1));
    pending.push(load(1, "a"));
    pending.push(create(2));
    pending.push(load(2, "b"));

    pending.forget(1);

    let ids: Vec<u32> = pending.take().iter().map(PendingCommand::id).collect();
    assert_eq!(ids, vec![2, 2]);
}

#[test]
fn taking_the_queue_leaves_it_empty() {
    let mut pending = PendingCommands::new();
    pending.push(create(1));
    let _ = pending.take();
    assert!(pending.is_empty());
}

/// A window that never arrives must not let this grow without bound, and what
/// is kept has to be the oldest commands: dropping from the front would evict
/// a `Create` and orphan everything queued after it.
#[test]
fn the_queue_is_capped_and_keeps_the_oldest() {
    let mut pending = PendingCommands::new();
    for id in 0..u32::try_from(CAPACITY + 10).expect("fits in u32") {
        pending.push(create(id));
    }
    assert_eq!(pending.len(), CAPACITY);

    let replayed = pending.take();
    assert_eq!(replayed[0].id(), 0);
    assert_eq!(
        replayed[CAPACITY - 1].id(),
        u32::try_from(CAPACITY - 1).expect("fits in u32")
    );
}

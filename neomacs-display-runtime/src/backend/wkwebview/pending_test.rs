use super::{CAPACITY, PendingCommands, WebKitViewCommand};

fn create(id: u32) -> WebKitViewCommand {
    WebKitViewCommand::Create {
        id,
        width: 100.0,
        height: 50.0,
    }
}

fn load(id: u32, url: &str) -> WebKitViewCommand {
    WebKitViewCommand::LoadUri {
        id,
        url: url.to_string(),
    }
}

fn script(id: u32, script: &str) -> WebKitViewCommand {
    WebKitViewCommand::ExecuteScript {
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
    assert!(matches!(replayed[0], WebKitViewCommand::Create { .. }));
    assert!(matches!(replayed[1], WebKitViewCommand::LoadUri { .. }));
    assert!(matches!(
        replayed[2],
        WebKitViewCommand::ExecuteScript { .. }
    ));
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

    let ids: Vec<u32> = pending.take().iter().map(WebKitViewCommand::id).collect();
    assert_eq!(ids, vec![1, 2, 2, 1]);
}

/// A resize before attach used to be lost entirely -- the view was built from
/// the stale create size and the resize was never replayed.
#[test]
fn a_resize_before_attach_is_replayed_after_the_create() {
    let mut pending = PendingCommands::new();
    pending.push(create(1));
    pending.push(WebKitViewCommand::Resize {
        id: 1,
        width: 640.0,
        height: 480.0,
    });

    assert_eq!(
        pending.take().last(),
        Some(&WebKitViewCommand::Resize {
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

    let ids: Vec<u32> = pending.take().iter().map(WebKitViewCommand::id).collect();
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
/// a `Create` and orphan everything queued after it. (Independent creates
/// only -- the id-boundary cases are the three tests below.)
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

fn fill_with_creates(pending: &mut PendingCommands, count: usize) {
    for id in 0..u32::try_from(count).expect("fits in u32") {
        pending.push(create(id));
    }
}

fn ids_in(commands: &[WebKitViewCommand]) -> Vec<u32> {
    commands.iter().map(WebKitViewCommand::id).collect()
}

/// Re-review catch (PR #297, P2 case 1): dropping only the newest command is
/// not atomic at an id boundary. A `Create` accepted as the last entry with
/// its `LoadUri` dropped replays into a blank view -- the very failure the
/// queue exists to prevent. Overflow has to take the id's earlier commands
/// with it.
#[test]
fn a_load_that_overflows_takes_its_create_with_it() {
    let mut pending = PendingCommands::new();
    fill_with_creates(&mut pending, CAPACITY - 1);
    pending.push(create(9_000)); // the 256th entry, accepted
    pending.push(load(9_000, "https://example.invalid/")); // overflows

    let ids = ids_in(&pending.take());
    assert!(
        !ids.contains(&9_000),
        "an id that lost a command must not be replayed at all"
    );
}

/// Re-review catch (PR #297, P2 case 2): once an id has lost a command it must
/// stay rejected even after `forget` frees a slot, or a later `LoadUri` for a
/// never-created view is accepted as an orphan.
#[test]
fn an_overflowed_id_stays_rejected_after_room_frees_up() {
    let mut pending = PendingCommands::new();
    fill_with_creates(&mut pending, CAPACITY);
    pending.push(create(300)); // dropped: at capacity
    pending.forget(0); // a slot frees up
    pending.push(load(300, "https://example.invalid/"));

    assert!(!ids_in(&pending.take()).contains(&300));
}

/// ...and `Destroy` is what ends the rejection, so a fresh lifecycle for that
/// id is accepted again.
#[test]
fn destroying_an_overflowed_id_lifts_its_rejection() {
    let mut pending = PendingCommands::new();
    fill_with_creates(&mut pending, CAPACITY);
    pending.push(create(300)); // dropped
    pending.forget(0);
    pending.forget(300); // Destroy
    pending.push(create(300));

    assert!(ids_in(&pending.take()).contains(&300));
}

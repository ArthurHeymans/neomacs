use super::{Action, CAPACITY, Lifecycle};
use crate::backend::wkwebview::command::WebKitViewCommand;

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

fn destroy(id: u32) -> WebKitViewCommand {
    WebKitViewCommand::Destroy { id }
}

fn applied(actions: &[Action]) -> Vec<&WebKitViewCommand> {
    actions
        .iter()
        .filter_map(|a| match a {
            Action::Apply(c) => Some(c),
            Action::Bind => None,
        })
        .collect()
}

fn ids(actions: &[Action]) -> Vec<u32> {
    applied(actions).iter().map(|c| c.id()).collect()
}

/// With no window, every command is deferred and nothing happens.
fn defer_all(lc: &mut Lifecycle, commands: impl IntoIterator<Item = WebKitViewCommand>) {
    for c in commands {
        assert!(lc.dispatch(c, false).is_empty());
    }
}

fn fill_with_creates(lc: &mut Lifecycle, count: usize) {
    defer_all(
        lc,
        (0..u32::try_from(count).expect("fits in u32")).map(create),
    );
}

// ---- deferral and replay -------------------------------------------------

#[test]
fn a_fresh_lifecycle_is_unbound_with_nothing_pending() {
    let lc = Lifecycle::new();
    assert!(!lc.is_bound());
    assert!(!lc.has_pending());
}

/// Review catch (PR #297, round one): only the create size used to be kept,
/// so a `Create` -> `LoadUri` pair issued before the window existed produced
/// an empty `WKWebView`. The URL has to survive, after the create.
#[test]
fn a_create_and_the_load_that_follows_it_both_replay_on_bind() {
    let mut lc = Lifecycle::new();
    defer_all(&mut lc, [create(1), load(1, "https://example.invalid/")]);
    assert!(lc.has_pending());

    let actions = lc.bind();
    assert_eq!(actions[0], Action::Bind);
    assert_eq!(
        applied(&actions),
        vec![&create(1), &load(1, "https://example.invalid/")]
    );
    assert!(lc.is_bound());
    assert!(!lc.has_pending());
}

/// Replay order is the order the commands were issued in, across ids too --
/// which is why this is one queue rather than one per id.
#[test]
fn replay_keeps_arrival_order_across_ids() {
    let mut lc = Lifecycle::new();
    defer_all(
        &mut lc,
        [
            create(1),
            create(2),
            load(2, "b"),
            load(1, "a"),
            script(1, "s"),
        ],
    );
    assert_eq!(ids(&lc.bind()), vec![1, 2, 2, 1, 1]);
}

/// A command that arrives when the window has just become available binds
/// first, replays, then applies itself -- in that order.
#[test]
fn the_first_command_with_a_window_binds_replays_then_applies() {
    let mut lc = Lifecycle::new();
    defer_all(&mut lc, [create(1)]);
    let actions = lc.dispatch(load(1, "a"), true);
    assert_eq!(actions[0], Action::Bind);
    assert_eq!(applied(&actions), vec![&create(1), &load(1, "a")]);
}

#[test]
fn once_bound_commands_apply_directly() {
    let mut lc = Lifecycle::new();
    let _ = lc.bind();
    assert_eq!(lc.dispatch(create(1), true), vec![Action::Apply(create(1))]);
    assert_eq!(
        lc.dispatch(create(2), false),
        vec![Action::Apply(create(2))]
    );
}

#[test]
fn bind_is_idempotent() {
    let mut lc = Lifecycle::new();
    assert_eq!(lc.bind(), vec![Action::Bind]);
    assert!(lc.bind().is_empty());
}

// ---- destroy --------------------------------------------------------------

/// An xwidget killed before the window existed must not be built by the
/// replay, and must take its queued commands with it.
#[test]
fn destroying_a_pending_id_drops_only_that_ids_commands() {
    let mut lc = Lifecycle::new();
    defer_all(&mut lc, [create(1), load(1, "a"), create(2), load(2, "b")]);
    assert!(lc.dispatch(destroy(1), false).is_empty());
    assert_eq!(ids(&lc.bind()), vec![2, 2]);
}

/// Re-review catch (PR #297, round three, P2): every command used to attach
/// before dispatching, so a `Destroy` arriving just as the window became
/// available bound the host, replayed the doomed xwidget's `Create` and
/// `LoadUri`, and only then removed it -- a view briefly built and possibly
/// already fetching. A kill must never be the reason to bind.
#[test]
fn destroy_never_binds_and_never_replays_the_xwidget_it_kills() {
    let mut lc = Lifecycle::new();
    defer_all(&mut lc, [create(1), load(1, "https://example.invalid/")]);

    // The window is now available, and the next command is the kill.
    let actions = lc.dispatch(destroy(1), true);

    assert!(
        actions.is_empty(),
        "no Bind, no Create, no LoadUri: got {actions:?}"
    );
    assert!(!lc.is_bound(), "a kill is not a reason to bind");
    assert!(!lc.has_pending(), "and the xwidget's queued work is gone");
    // A later bind has nothing of id 1 to replay either.
    assert_eq!(lc.bind(), vec![Action::Bind]);
}

/// Once bound, a destroy is applied to the live view like any other command.
#[test]
fn a_bound_destroy_is_applied() {
    let mut lc = Lifecycle::new();
    let _ = lc.bind();
    assert_eq!(
        lc.dispatch(destroy(7), true),
        vec![Action::Apply(destroy(7))]
    );
}

// ---- overflow --------------------------------------------------------------

/// A window that never arrives must not let the queue grow without bound,
/// and what is kept has to be the oldest commands.
#[test]
fn the_queue_is_capped_and_keeps_the_oldest() {
    let mut lc = Lifecycle::new();
    fill_with_creates(&mut lc, CAPACITY + 10);
    let replayed = ids(&lc.bind());
    assert_eq!(replayed.len(), CAPACITY);
    assert_eq!(replayed[0], 0);
    assert_eq!(replayed[CAPACITY - 1], u32::try_from(CAPACITY - 1).unwrap());
}

/// Re-review catch (PR #297, round two, P2 case 1): dropping only the newest
/// command is not atomic at an id boundary. A `Create` accepted as the last
/// entry with its `LoadUri` dropped replays into a blank view -- the very
/// failure the queue exists to prevent.
#[test]
fn a_load_that_overflows_takes_its_create_with_it() {
    let mut lc = Lifecycle::new();
    fill_with_creates(&mut lc, CAPACITY - 1);
    defer_all(
        &mut lc,
        [create(9_000), load(9_000, "https://example.invalid/")],
    );
    assert!(!ids(&lc.bind()).contains(&9_000));
}

/// Re-review catch (round two, P2 case 2): once an id has lost a command it
/// must stay refused even after a destroy frees a slot.
#[test]
fn an_overflowed_id_stays_refused_after_room_frees_up() {
    let mut lc = Lifecycle::new();
    fill_with_creates(&mut lc, CAPACITY);
    defer_all(&mut lc, [load(0, "evicts id 0")]); // id 0 had a queued create: evicted, refused
    let _ = lc.dispatch(destroy(5), false); // frees a slot
    defer_all(&mut lc, [load(0, "https://example.invalid/")]);
    assert!(!ids(&lc.bind()).contains(&0));
}

/// Re-review catch (round three, follow-up): refusal used to live in the
/// queue only, so once bound a refused id could get a fresh `Create` before
/// its `Destroy`. It is lifecycle state, and it survives binding.
#[test]
fn a_refused_id_stays_refused_after_binding_until_destroy() {
    let mut lc = Lifecycle::new();
    fill_with_creates(&mut lc, CAPACITY);
    defer_all(&mut lc, [load(3, "evicts id 3")]);
    let _ = lc.bind();

    assert!(
        lc.dispatch(create(3), true).is_empty(),
        "still refused once bound"
    );
    assert_eq!(
        lc.dispatch(destroy(3), true),
        vec![Action::Apply(destroy(3))]
    );
    assert_eq!(lc.dispatch(create(3), true), vec![Action::Apply(create(3))]);
}

/// Re-review catch (round three, P2): the queue was capped but the refusal
/// set was not, so a stream of fresh ids after saturation grew memory -- and
/// emitted one warning each -- without bound. Retained state must stay under
/// the cap however many ids arrive, and evicting into the refusal set must
/// not be a way around that either.
#[test]
fn retained_state_stays_bounded_under_a_flood_of_fresh_ids() {
    let mut lc = Lifecycle::new();
    fill_with_creates(&mut lc, CAPACITY);
    assert_eq!(lc.retained(), CAPACITY);

    // Evict every queued id into the refusal set: sum must not grow.
    for id in 0..u32::try_from(CAPACITY).unwrap() {
        defer_all(&mut lc, [load(id, "evict")]);
    }
    assert!(
        lc.retained() <= CAPACITY,
        "evictions moved ids, not added them"
    );

    // Then a flood of ids that never had anything queued.
    fill_with_creates(&mut lc, 10 * CAPACITY);
    assert!(
        lc.retained() <= CAPACITY,
        "fresh ids are dropped, not remembered"
    );
}

use super::*;

fn request(filters: impl IntoIterator<Item = W32Filter>) -> W32Request {
    W32Request::new(filters.into_iter().collect())
}

#[test]
fn w32_filters_keep_name_and_metadata_interests_distinct() {
    let created = notify::Event::new(notify::EventKind::Create(notify::event::CreateKind::File))
        .add_path(PathBuf::from(r"C:\watched\created.txt"));
    let modified = notify::Event::new(notify::EventKind::Modify(ModifyKind::Any))
        .add_path(PathBuf::from(r"C:\watched\modified.txt"));

    assert_eq!(
        event_actions(&created, &request([W32Filter::FileName])),
        [(0, W32Action::Added)]
    );
    assert!(event_actions(&modified, &request([W32Filter::FileName])).is_empty());
    assert!(event_actions(&created, &request([W32Filter::Attributes])).is_empty());
    assert_eq!(
        event_actions(&modified, &request([W32Filter::Attributes])),
        [(0, W32Action::Modified)]
    );
}

#[test]
fn w32_rename_both_preserves_the_ordered_old_and_new_halves() {
    let renamed = notify::Event::new(notify::EventKind::Modify(ModifyKind::Name(
        RenameMode::Both,
    )))
    .add_path(PathBuf::from(r"C:\watched\old.txt"))
    .add_path(PathBuf::from(r"C:\watched\new.txt"));

    assert_eq!(
        event_actions(&renamed, &request([W32Filter::FileName])),
        [(0, W32Action::RenamedFrom), (1, W32Action::RenamedTo)]
    );
}

#[test]
fn w32_request_parsing_and_lisp_event_shape_match_gnu() {
    assert_eq!(
        W32Filter::from_lisp_name("security-desc"),
        Some(W32Filter::SecurityDescriptor)
    );
    assert_eq!(W32Filter::from_lisp_name("unknown-filter"), None);
    assert!(request([W32Filter::Subtree]).recursive());

    let event = W32Event {
        watch_id: WatchId::new(42, 0),
        action: W32Action::RenamedTo,
        path: PathBuf::from(r"nested\new.txt"),
    };
    let fields = crate::emacs_core::value::list_to_vec(&event.into_lisp())
        .expect("w32 event is a proper list");
    assert_eq!(
        fields,
        [
            Value::fixnum(42),
            Value::symbol("renamed-to"),
            Value::string(r"nested\new.txt")
        ]
    );
}

#[test]
fn recursive_physical_watch_satisfies_both_logical_watch_modes() {
    assert!(WatchMode::Recursive.covers(WatchMode::Direct));
    assert!(WatchMode::Recursive.covers(WatchMode::Recursive));
    assert!(WatchMode::Direct.covers(WatchMode::Direct));
    assert!(!WatchMode::Direct.covers(WatchMode::Recursive));
}

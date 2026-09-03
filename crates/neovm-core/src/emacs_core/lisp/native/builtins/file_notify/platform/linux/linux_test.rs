use super::*;

#[test]
fn combined_inotify_masks_keep_gnus_observable_aspect_order() {
    let mask =
        EventMask::ATTRIB | EventMask::CLOSE_WRITE | EventMask::MOVED_FROM | EventMask::ISDIR;
    assert_eq!(
        InotifyBackend::aspects(mask),
        ["isdir", "moved-from", "close-write", "attrib"]
    );
}

#[test]
fn inactive_native_watch_terminates_without_a_queued_data_event() {
    let watch_id = WatchId::new(11, 0);
    let activity = WatchActivity::active();
    activity.terminate();
    let mut backend = InotifyBackend {
        worker: None,
        watches: vec![InotifyWatch {
            common: FileWatch {
                id: watch_id.clone(),
                path: PathBuf::from("watched"),
                request: InotifyRequest::new(vec!["modify".to_owned()]),
            },
            native_descriptor: 5,
            activity,
        }],
        next_id: 12,
    };

    assert!(!backend.valid_p(&watch_id));
    let batch = backend.drain_events().expect("reconcile native lifecycle");
    assert!(batch.events.is_empty());
    assert_eq!(batch.terminated, [watch_id]);
    assert!(!backend.has_watches());
}

#[test]
fn inotify_request_keeps_creation_controls_out_of_the_event_filter() {
    let request = InotifyRequest::new(vec![
        "modify".to_owned(),
        "dont-follow".to_owned(),
        "onlydir".to_owned(),
    ]);

    assert!(request.watch_mask.contains(WatchMask::MODIFY));
    assert!(request.watch_mask.contains(WatchMask::DONT_FOLLOW));
    assert!(request.watch_mask.contains(WatchMask::ONLYDIR));
    assert!(request.event_mask.contains(EventMask::MODIFY));
    assert!(!request.event_mask.contains(EventMask::ATTRIB));
}

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

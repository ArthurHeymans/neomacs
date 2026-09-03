use super::*;

#[test]
fn a_full_delivery_queue_latches_one_rescan_request() {
    let (sender, receiver) = channel_with_capacity(1, None);

    assert_eq!(sender.publish(1), PublishOutcome::Published);
    assert_eq!(sender.publish(2), PublishOutcome::Overflowed);
    assert_eq!(receiver.try_recv(), Ok(1));
    assert!(receiver.take_overflow());
    assert!(!receiver.take_overflow(), "overflow state is coalesced");
}

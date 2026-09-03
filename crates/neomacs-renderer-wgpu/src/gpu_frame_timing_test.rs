use super::timestamp_delta_us;

#[test]
fn timestamp_ticks_are_converted_with_the_queue_period() {
    assert_eq!(timestamp_delta_us(100, 1_100, 2.5), Some(3));
}

#[test]
fn wrapped_timestamp_is_rejected_instead_of_becoming_a_huge_sample() {
    assert_eq!(timestamp_delta_us(1_100, 100, 1.0), None);
}

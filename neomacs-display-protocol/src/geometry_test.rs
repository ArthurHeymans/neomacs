use crate::geometry::{
    DeviceScale, FrameSpace, LayoutRect, RowSpace, SpaceTranslation, WindowSpace,
};

#[test]
fn typed_transforms_compose_row_window_and_frame_spaces() {
    let local = LayoutRect::<RowSpace>::from_px(3.25, 4.5, 20.0, 10.0);
    let row_to_window = SpaceTranslation::<RowSpace, WindowSpace>::from_px(8.0, 18.0);
    let window_to_frame = SpaceTranslation::<WindowSpace, FrameSpace>::from_px(320.0, 100.0);

    let in_frame = row_to_window.then(window_to_frame).map_rect(local);

    assert_eq!(in_frame.x().to_px(), 331.25);
    assert_eq!(in_frame.y().to_px(), 122.5);
    assert_eq!(in_frame.width().to_px(), 20.0);
    assert_eq!(in_frame.height().to_px(), 10.0);
}

#[test]
fn device_scale_is_applied_only_at_the_frame_boundary() {
    let frame_rect = LayoutRect::<FrameSpace>::from_px(10.0, 20.0, 80.0, 40.0);
    let scale = DeviceScale::new(1.75).expect("positive finite scale");

    let device_rect = scale.map_frame_rect(frame_rect);

    assert_eq!(device_rect.x(), 17.5);
    assert_eq!(device_rect.y(), 35.0);
    assert_eq!(device_rect.width(), 140.0);
    assert_eq!(device_rect.height(), 70.0);
}

#[test]
fn device_scale_rejects_invalid_values() {
    assert!(DeviceScale::new(0.0).is_err());
    assert!(DeviceScale::new(f32::NAN).is_err());
    assert!(DeviceScale::new(f32::INFINITY).is_err());
}

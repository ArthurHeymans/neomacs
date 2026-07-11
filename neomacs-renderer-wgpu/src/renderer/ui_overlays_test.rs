use super::ui_overlays::placed_chrome_item_bounds;
use neomacs_display_protocol::frame_chrome::{BandRect, FrameRect};
use neomacs_display_protocol::types::Rect;

#[test]
fn frame_chrome_item_projection_uses_authoritative_band_origin_once() {
    let band = FrameRect::new(0.0, 33.0, 800.0, 34.0).expect("toolbar band");
    let item = BandRect::new(5.0, 0.0, 24.0, 34.0).expect("local toolbar item");

    assert_eq!(
        placed_chrome_item_bounds(band, item),
        Rect::new(5.0, 33.0, 24.0, 34.0)
    );
}

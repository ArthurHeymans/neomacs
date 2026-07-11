use super::*;
use std::io::Cursor;

#[test]
fn freed_or_replaced_image_loads_reject_late_decode_outcomes() {
    let mut loads = ImageLoadLifecycle::default();

    let freed = loads.begin(41);
    loads.free(41);
    assert!(!loads.accept(freed));

    let old = loads.begin(42);
    let current = loads.begin(42);
    assert!(!loads.accept(old));
    assert!(loads.accept(current));
    assert!(!loads.accept(current), "a duplicate terminal is stale");
    let replacement = loads.begin(42);
    assert!(loads.accept(replacement), "a new generation remains valid");
    assert!(loads.active.is_empty());
}

#[test]
fn ready_and_failed_terminals_consume_their_active_generations() {
    let mut loads = ImageLoadLifecycle::default();
    let ready = loads.begin(51);
    let failed = loads.begin(52);

    let ready =
        WorkerDecodeOutcome::Ready(ImageCache::decoded_image(ready, 1, 1, vec![0, 0, 0, 255]));
    assert!(matches!(
        loads.take_current(ready),
        Some(WorkerDecodeOutcome::Ready(_))
    ));
    assert_eq!(loads.active.len(), 1);

    assert!(matches!(
        loads.take_current(WorkerDecodeOutcome::Failed(failed)),
        Some(WorkerDecodeOutcome::Failed(_))
    ));
    assert!(loads.active.is_empty());
    assert!(
        loads
            .take_current(WorkerDecodeOutcome::Failed(failed))
            .is_none()
    );
}

fn png_bytes(pixels: Vec<u8>, width: u32, height: u32) -> Vec<u8> {
    let image = image::RgbaImage::from_raw(width, height, pixels).unwrap();
    let mut bytes = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    bytes.into_inner()
}

#[test]
fn decoded_opaque_png_reports_gnu_corner_background_without_lisp_background() {
    let data = png_bytes([0x12, 0x34, 0x56, 0xff].repeat(4), 2, 2);
    let decoded = ImageCache::decode_data_with_metadata(&data, 0, 0, (0, 0)).unwrap();

    assert_eq!(decoded.metadata.width, 2);
    assert_eq!(decoded.metadata.height, 2);
    assert!(!decoded.metadata.background_transparent);
    assert_eq!(decoded.metadata.background, 0x12_34_56);
}

#[test]
fn decoded_transparent_png_stays_transparent_with_explicit_lisp_background() {
    let data = png_bytes([0x12, 0x34, 0x56, 0x00].repeat(4), 2, 2);
    let decoded = ImageCache::decode_data_with_metadata(&data, 0, 0, (0, 0xff_aa_bb_cc)).unwrap();

    assert!(decoded.metadata.background_transparent);
    assert_ne!(decoded.metadata.background, 0xaa_bb_cc);
}

#[test]
fn decoded_partial_alpha_png_corners_are_gnu_draw_not_transparent_mask() {
    for alpha in [1, 254] {
        let data = png_bytes([0x12, 0x34, 0x56, alpha].repeat(4), 2, 2);
        let decoded = ImageCache::decode_data_with_metadata(&data, 0, 0, (0, 0)).unwrap();

        assert!(
            !decoded.metadata.background_transparent,
            "GNU mask DRAW includes nonzero alpha {alpha}"
        );
    }
}

#[test]
fn decoded_corner_mask_tie_uses_gnu_first_corner_winner() {
    let metadata = |alphas: [u8; 4]| {
        let pixels = alphas
            .into_iter()
            .flat_map(|alpha| [0x12, 0x34, 0x56, alpha])
            .collect();
        let data = png_bytes(pixels, 2, 2);
        ImageCache::decode_data_with_metadata(&data, 0, 0, (0, 0))
            .unwrap()
            .metadata
    };

    assert!(!metadata([1, 0, 0, 254]).background_transparent);
    assert!(metadata([0, 1, 254, 0]).background_transparent);
}

#[test]
fn decoded_transparent_svg_stays_transparent_with_explicit_lisp_background() {
    let data = br##"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4"><rect x="1" y="1" width="2" height="2" fill="#123456"/></svg>"##;
    let decoded = ImageCache::decode_data_with_metadata(data, 0, 0, (0, 0xff_aa_bb_cc)).unwrap();

    assert!(decoded.metadata.background_transparent);
    assert_ne!(decoded.metadata.background, 0xaa_bb_cc);
}

#[test]
fn decoded_xpm_distinguishes_transparent_and_opaque_corner_backgrounds() {
    let transparent = br#"/* XPM */
static char *icon[] = {
"2 2 2 1",
". c None",
"x c #123456",
"..",
".x"};"#;
    let opaque = br#"/* XPM */
static char *icon[] = {
"2 2 1 1",
"x c #123456",
"xx",
"xx"};"#;

    let transparent = ImageCache::decode_data_with_metadata(transparent, 0, 0, (0, 0)).unwrap();
    let opaque = ImageCache::decode_data_with_metadata(opaque, 0, 0, (0, 0)).unwrap();
    assert!(transparent.metadata.background_transparent);
    assert!(!opaque.metadata.background_transparent);
    assert_eq!(opaque.metadata.background, 0x12_34_56);
}

#[test]
fn test_convert_argb32_to_rgba_basic() {
    // Create a 2x2 ARGB32 image
    // ARGB32 format: A, R, G, B (4 bytes per pixel)
    let width = 2u32;
    let height = 2u32;
    let stride = width * 4; // No padding
    let data: Vec<u8> = vec![
        // Row 0
        255, 100, 150, 200, // Pixel (0,0): A=255, R=100, G=150, B=200
        128, 50, 75, 100, // Pixel (1,0): A=128, R=50, G=75, B=100
        // Row 1
        64, 25, 37, 50, // Pixel (0,1): A=64, R=25, G=37, B=50
        0, 0, 0, 0, // Pixel (1,1): A=0, R=0, G=0, B=0 (transparent)
    ];

    let result = ImageCache::convert_argb32_to_rgba(&data, width, height, stride, 0, 0);
    assert!(result.is_some());

    let (w, h, rgba) = result.unwrap();
    assert_eq!(w, 2);
    assert_eq!(h, 2);
    assert_eq!(rgba.len(), 16); // 2x2x4 bytes

    // Expected RGBA output: R, G, B, A
    // Pixel (0,0): R=100, G=150, B=200, A=255
    assert_eq!(&rgba[0..4], &[100, 150, 200, 255]);
    // Pixel (1,0): R=50, G=75, B=100, A=128
    assert_eq!(&rgba[4..8], &[50, 75, 100, 128]);
    // Pixel (0,1): R=25, G=37, B=50, A=64
    assert_eq!(&rgba[8..12], &[25, 37, 50, 64]);
    // Pixel (1,1): R=0, G=0, B=0, A=0
    assert_eq!(&rgba[12..16], &[0, 0, 0, 0]);
}

#[test]
fn test_convert_argb32_with_stride_padding() {
    // 2x2 image with stride = 12 (4 bytes padding per row)
    let width = 2u32;
    let height = 2u32;
    let stride = 12u32; // 8 bytes data + 4 bytes padding per row
    let data: Vec<u8> = vec![
        // Row 0 (8 bytes data + 4 bytes padding)
        255, 100, 150, 200, // Pixel (0,0)
        128, 50, 75, 100, // Pixel (1,0)
        0, 0, 0, 0, // Padding (ignored)
        // Row 1 (8 bytes data + 4 bytes padding)
        64, 25, 37, 50, // Pixel (0,1)
        32, 10, 20, 30, // Pixel (1,1)
        0, 0, 0, 0, // Padding (ignored)
    ];

    let result = ImageCache::convert_argb32_to_rgba(&data, width, height, stride, 0, 0);
    assert!(result.is_some());

    let (w, h, rgba) = result.unwrap();
    assert_eq!(w, 2);
    assert_eq!(h, 2);

    // Verify conversion (padding should be ignored)
    assert_eq!(&rgba[0..4], &[100, 150, 200, 255]); // Pixel (0,0)
    assert_eq!(&rgba[4..8], &[50, 75, 100, 128]); // Pixel (1,0)
    assert_eq!(&rgba[8..12], &[25, 37, 50, 64]); // Pixel (0,1)
    assert_eq!(&rgba[12..16], &[10, 20, 30, 32]); // Pixel (1,1)
}

#[test]
fn test_convert_argb32_invalid_data_size() {
    // Data too small for 2x2 image
    let data: Vec<u8> = vec![255, 100, 150, 200]; // Only 1 pixel
    let result = ImageCache::convert_argb32_to_rgba(&data, 2, 2, 8, 0, 0);
    assert!(result.is_none());
}

#[test]
fn test_convert_rgb24_to_rgba_basic() {
    // Create a 2x2 RGB24 image
    // RGB24 format: R, G, B (3 bytes per pixel)
    let width = 2u32;
    let height = 2u32;
    let stride = width * 3; // No padding
    let data: Vec<u8> = vec![
        // Row 0
        100, 150, 200, // Pixel (0,0): R=100, G=150, B=200
        50, 75, 100, // Pixel (1,0): R=50, G=75, B=100
        // Row 1
        25, 37, 50, // Pixel (0,1): R=25, G=37, B=50
        0, 0, 0, // Pixel (1,1): R=0, G=0, B=0 (black)
    ];

    let result = ImageCache::convert_rgb24_to_rgba(&data, width, height, stride, 0, 0);
    assert!(result.is_some());

    let (w, h, rgba) = result.unwrap();
    assert_eq!(w, 2);
    assert_eq!(h, 2);
    assert_eq!(rgba.len(), 16); // 2x2x4 bytes

    // Expected RGBA output: R, G, B, A (A should always be 255)
    assert_eq!(&rgba[0..4], &[100, 150, 200, 255]);
    assert_eq!(&rgba[4..8], &[50, 75, 100, 255]);
    assert_eq!(&rgba[8..12], &[25, 37, 50, 255]);
    assert_eq!(&rgba[12..16], &[0, 0, 0, 255]);
}

#[test]
fn test_convert_rgb24_with_stride_padding() {
    // 2x2 image with stride = 8 (2 bytes padding per row)
    let width = 2u32;
    let height = 2u32;
    let stride = 8u32; // 6 bytes data + 2 bytes padding per row
    let data: Vec<u8> = vec![
        // Row 0 (6 bytes data + 2 bytes padding)
        100, 150, 200, // Pixel (0,0)
        50, 75, 100, // Pixel (1,0)
        0, 0, // Padding (ignored)
        // Row 1 (6 bytes data + 2 bytes padding)
        25, 37, 50, // Pixel (0,1)
        10, 20, 30, // Pixel (1,1)
        0, 0, // Padding (ignored)
    ];

    let result = ImageCache::convert_rgb24_to_rgba(&data, width, height, stride, 0, 0);
    assert!(result.is_some());

    let (w, h, rgba) = result.unwrap();
    assert_eq!(w, 2);
    assert_eq!(h, 2);

    // Verify conversion (padding should be ignored)
    assert_eq!(&rgba[0..4], &[100, 150, 200, 255]); // Pixel (0,0)
    assert_eq!(&rgba[4..8], &[50, 75, 100, 255]); // Pixel (1,0)
    assert_eq!(&rgba[8..12], &[25, 37, 50, 255]); // Pixel (0,1)
    assert_eq!(&rgba[12..16], &[10, 20, 30, 255]); // Pixel (1,1)
}

#[test]
fn test_convert_rgb24_invalid_data_size() {
    // Data too small for 2x2 image
    let data: Vec<u8> = vec![100, 150, 200]; // Only 1 pixel
    let result = ImageCache::convert_rgb24_to_rgba(&data, 2, 2, 6, 0, 0);
    assert!(result.is_none());
}

#[test]
fn test_constrain_dimensions() {
    // No constraints (uses MAX_TEXTURE_SIZE internally)
    assert_eq!(ImageCache::constrain_dimensions(100, 100, 0, 0), (100, 100));

    // Width constrained
    assert_eq!(
        ImageCache::constrain_dimensions(200, 100, 100, 0),
        (100, 50)
    );

    // Height constrained
    assert_eq!(
        ImageCache::constrain_dimensions(100, 200, 0, 100),
        (50, 100)
    );

    // Both constrained, width is limiting factor
    assert_eq!(
        ImageCache::constrain_dimensions(400, 200, 100, 100),
        (100, 50)
    );

    // Both constrained, height is limiting factor
    assert_eq!(
        ImageCache::constrain_dimensions(200, 400, 100, 100),
        (50, 100)
    );

    // Minimum 1x1 - very narrow image
    let (w, h) = ImageCache::constrain_dimensions(1, 1000, 10, 100);
    assert_eq!(w, 1);
    assert_eq!(h, 100); // Height is constrained to 100, width stays 1 (min)
}

#[test]
fn test_convert_argb32_single_pixel() {
    // Single pixel image - edge case
    let data: Vec<u8> = vec![255, 128, 64, 32]; // A=255, R=128, G=64, B=32
    let result = ImageCache::convert_argb32_to_rgba(&data, 1, 1, 4, 0, 0);
    assert!(result.is_some());

    let (w, h, rgba) = result.unwrap();
    assert_eq!(w, 1);
    assert_eq!(h, 1);
    assert_eq!(rgba, vec![128, 64, 32, 255]); // R=128, G=64, B=32, A=255
}

#[test]
fn test_convert_rgb24_single_pixel() {
    // Single pixel image - edge case
    let data: Vec<u8> = vec![128, 64, 32]; // R=128, G=64, B=32
    let result = ImageCache::convert_rgb24_to_rgba(&data, 1, 1, 3, 0, 0);
    assert!(result.is_some());

    let (w, h, rgba) = result.unwrap();
    assert_eq!(w, 1);
    assert_eq!(h, 1);
    assert_eq!(rgba, vec![128, 64, 32, 255]); // R=128, G=64, B=32, A=255
}

#[test]
fn lru_victim_prefers_least_recent_stamp_over_smallest_id() {
    // Insert order 1, 2, 3 (stamps 1, 2, 3), then id 1 is accessed again
    // (stamp 4). FIFO-by-smallest-id would evict 1; LRU must evict 2.
    let entries = [(1u32, 4u64), (2, 2), (3, 3)];
    assert_eq!(lru_victim(entries.iter().copied()), Some(2));
}

#[test]
fn lru_victim_repeated_touches_protect_hot_entries() {
    // 3 was inserted last but 1 and 3 were both re-read afterwards; the
    // coldest entry is 2 regardless of insertion order.
    let entries = [(1u32, 5u64), (2, 2), (3, 6)];
    assert_eq!(lru_victim(entries.iter().copied()), Some(2));
}

#[test]
fn lru_victim_matches_insert_order_when_never_touched() {
    let entries = [(1u32, 1u64), (2, 2), (3, 3)];
    assert_eq!(lru_victim(entries.iter().copied()), Some(1));
}

#[test]
fn lru_victim_of_no_entries_is_none() {
    assert_eq!(lru_victim(std::iter::empty()), None);
}

//! Unit tests for the fringe-bitmap registry and `define-fringe-bitmap`.

use super::*;
use crate::emacs_core::Context;
use crate::emacs_core::value::Value;

/// `magit-fringe-bitmap>` collapsed-arrow rows (width 8). Stored MSB-aligned, so
/// `#b01100000` (= 0x60) becomes `0x6000`: columns 1 and 2 set.
const MAGIT_ARROW_GT: [u32; 8] = [
    0b01100000, 0b00110000, 0b00011000, 0b00001100, 0b00011000, 0b00110000, 0b01100000, 0b00000000,
];

#[test]
fn parse_bits_rows_is_msb_aligned_width_8() {
    let rows = parse_bits_rows(&MAGIT_ARROW_GT, 8);
    assert_eq!(rows.len(), 8);
    // 0x60 << (16 - 8) = 0x6000. Leftmost column (bit 15) is clear, columns 1,2 set.
    assert_eq!(rows[0], 0x6000);
    // Renderer reads column b as (bits >> (15 - b)) & 1.
    assert_eq!((rows[0] >> 15) & 1, 0, "column 0 clear");
    assert_eq!((rows[0] >> 14) & 1, 1, "column 1 set");
    assert_eq!((rows[0] >> 13) & 1, 1, "column 2 set");
    assert_eq!((rows[0] >> 12) & 1, 0, "column 3 clear");
}

#[test]
fn parse_bits_rows_uses_only_width_low_bits() {
    // Width 4: only the low 4 bits matter; high bits are masked off.
    let rows = parse_bits_rows(&[0b1111_1010], 4);
    // mask 0b1111 -> 0b1010, shifted up by 16-4 = 12 -> 0xA000.
    assert_eq!(rows[0], 0xA000);
    assert_eq!((rows[0] >> 15) & 1, 1, "col 0");
    assert_eq!((rows[0] >> 14) & 1, 0, "col 1");
    assert_eq!((rows[0] >> 13) & 1, 1, "col 2");
    assert_eq!((rows[0] >> 12) & 1, 0, "col 3");
}

#[test]
fn parse_bits_rows_width_16_keeps_all_bits() {
    let rows = parse_bits_rows(&[0xC003], 16);
    assert_eq!(rows[0], 0xC003);
}

#[test]
fn fit_rows_to_height_centers_when_taller() {
    let (rows, h) = fit_rows_to_height(vec![0x6000, 0x3000], Some(6));
    assert_eq!(h, 6);
    // 4 extra rows: fill1 = 2, fill2 = 2.
    assert_eq!(rows, vec![0, 0, 0x6000, 0x3000, 0, 0]);
}

#[test]
fn fit_rows_to_height_defaults_to_natural_length() {
    let (rows, h) = fit_rows_to_height(vec![0x6000, 0x3000, 0x1800], None);
    assert_eq!(h, 3);
    assert_eq!(rows.len(), 3);
}

fn define_via_eval(eval: &mut Context, form: &str) -> Value {
    eval.eval_str(form).expect("define-fringe-bitmap eval")
}

#[test]
fn define_fringe_bitmap_stores_bits_and_returns_index() {
    let mut eval = Context::new();
    let index_val = define_via_eval(
        &mut eval,
        "(define-fringe-bitmap 'magit-fringe-bitmap> [#b01100000 #b00110000 #b00011000 \
         #b00001100 #b00011000 #b00110000 #b01100000 #b00000000])",
    );
    let index = index_val.as_fixnum().expect("index is an integer");
    // User bitmaps start at index 25.
    assert!(index >= 25, "user index {index} should be >= 25");

    // The `'fringe` property was set to the same index.
    let prop = eval
        .eval_str("(get 'magit-fringe-bitmap> 'fringe)")
        .expect("get fringe prop");
    assert_eq!(prop.as_fixnum(), Some(index));

    // The registry has the bits, MSB-aligned, with default width 8, height 8.
    let bitmap = eval
        .fringe_bitmaps
        .get_by_index(index as u32)
        .expect("registry entry by index");
    assert_eq!(bitmap.width, 8);
    assert_eq!(bitmap.height, 8);
    assert_eq!(bitmap.bits.len(), 8);
    assert_eq!(bitmap.bits[0], 0x6000);
    assert_eq!(bitmap.period, 0);
}

#[test]
fn define_fringe_bitmap_string_bits_parse_msb_first() {
    let mut eval = Context::new();
    // A unibyte string row "\140" == 0x60; same as the vector form above.
    let index_val = define_via_eval(
        &mut eval,
        "(define-fringe-bitmap 'test-str-bitmap \"\\140\\060\" nil 8)",
    );
    let index = index_val.as_fixnum().expect("index") as u32;
    let bitmap = eval.fringe_bitmaps.get_by_index(index).expect("entry");
    assert_eq!(bitmap.width, 8);
    assert_eq!(bitmap.bits[0], 0x6000, "0x60 -> MSB-aligned 0x6000");
    assert_eq!(bitmap.bits[1], 0x3000, "0x30 -> MSB-aligned 0x3000");
}

#[test]
fn define_fringe_bitmap_redefine_keeps_index() {
    let mut eval = Context::new();
    let first = define_via_eval(
        &mut eval,
        "(define-fringe-bitmap 'redef-bitmap [#b10000000])",
    )
    .as_fixnum()
    .expect("first index");
    let second = define_via_eval(
        &mut eval,
        "(define-fringe-bitmap 'redef-bitmap [#b11000000 #b11000000])",
    )
    .as_fixnum()
    .expect("second index");
    assert_eq!(first, second, "redefining keeps the same index");
    let bitmap = eval
        .fringe_bitmaps
        .get_by_index(second as u32)
        .expect("entry");
    assert_eq!(bitmap.bits.len(), 2);
}

#[test]
fn define_fringe_bitmap_align_top_and_bottom_parse() {
    let mut eval = Context::new();
    let top = define_via_eval(
        &mut eval,
        "(define-fringe-bitmap 'top-bitmap [#b10000000] nil 8 'top)",
    )
    .as_fixnum()
    .expect("top index") as u32;
    assert_eq!(
        eval.fringe_bitmaps.get_by_index(top).expect("top").align,
        FringeBitmapAlign::Top
    );

    let bottom = define_via_eval(
        &mut eval,
        "(define-fringe-bitmap 'bottom-bitmap [#b10000000] nil 8 'bottom)",
    )
    .as_fixnum()
    .expect("bottom index") as u32;
    assert_eq!(
        eval.fringe_bitmaps
            .get_by_index(bottom)
            .expect("bottom")
            .align,
        FringeBitmapAlign::Bottom
    );
}

#[test]
fn define_fringe_bitmap_periodic_align_sets_period() {
    let mut eval = Context::new();
    let index = define_via_eval(
        &mut eval,
        "(define-fringe-bitmap 'periodic-bitmap [#b10101010 #b01010101] nil 8 '(top t))",
    )
    .as_fixnum()
    .expect("periodic index") as u32;
    let bitmap = eval.fringe_bitmaps.get_by_index(index).expect("entry");
    assert_eq!(bitmap.period, 2, "period == natural row count");
    assert_eq!(bitmap.height, 255, "periodic height forced to 255");
    assert_eq!(bitmap.bits.len(), 255);
}

#[test]
fn destroy_fringe_bitmap_removes_entry() {
    let mut eval = Context::new();
    let index = define_via_eval(&mut eval, "(define-fringe-bitmap 'doomed [#b10000000])")
        .as_fixnum()
        .expect("index") as u32;
    assert!(eval.fringe_bitmaps.get_by_index(index).is_some());
    eval.eval_str("(destroy-fringe-bitmap 'doomed)")
        .expect("destroy");
    assert!(
        eval.fringe_bitmaps.get_by_index(index).is_none(),
        "destroyed bitmap removed from registry"
    );
    let prop = eval
        .eval_str("(get 'doomed 'fringe)")
        .expect("get prop after destroy");
    assert!(prop.is_nil(), "fringe property cleared");
}

#[test]
fn set_fringe_bitmap_face_records_override() {
    let mut eval = Context::new();
    let index = define_via_eval(&mut eval, "(define-fringe-bitmap 'faced [#b10000000])")
        .as_fixnum()
        .expect("index") as u32;
    eval.eval_str("(set-fringe-bitmap-face 'faced 'magit-section-heading)")
        .expect("set-fringe-bitmap-face");
    let bitmap = eval.fringe_bitmaps.get_by_index(index).expect("entry");
    assert_eq!(bitmap.face.as_deref(), Some("magit-section-heading"));

    // A subsequent geometry-only redefinition preserves the face override.
    define_via_eval(
        &mut eval,
        "(define-fringe-bitmap 'faced [#b11000000 #b11000000])",
    );
    let bitmap = eval.fringe_bitmaps.get_by_index(index).expect("entry");
    assert_eq!(
        bitmap.face.as_deref(),
        Some("magit-section-heading"),
        "redefining geometry keeps the set-fringe-bitmap-face override"
    );
}

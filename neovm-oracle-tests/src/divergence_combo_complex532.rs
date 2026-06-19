/// Batch 532: terminal ops, display capabilities, and fontset operations.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx532_terminal_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((term (frame-terminal (selected-frame))))
  (terminal-name term))
"##,
    );
}

#[test]
fn div_cx532_terminal_live() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((term (frame-terminal (selected-frame))))
  (terminal-live-p term))
"##,
    );
}

#[test]
fn div_cx532_terminal_params() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((term (frame-terminal (selected-frame))))
  (set-terminal-parameter term 'test 'value)
  (terminal-parameter term 'test))
"##,
    );
}

#[test]
fn div_cx532_device_class() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((term (frame-terminal (selected-frame))))
  (device-class term))
"##,
    );
}

#[test]
fn div_cx532_display_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (display-type) (display-supports-face-attributes-p '(:weight bold)))
"##,
    );
}

#[test]
fn div_cx532_display_visual() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(display-visual-class)
"##,
    );
}

#[test]
fn div_cx532_display_color_cells() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (display-color-cells) (display-color-p))
"##,
    );
}

#[test]
fn div_cx532_display_pixel_dim() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (display-pixel-width) (display-pixel-height))
"##,
    );
}

#[test]
fn div_cx532_display_mm_dim() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (display-mm-width) (display-mm-height))
"##,
    );
}

#[test]
fn div_cx532_display_save_under() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(display-save-under)
"##,
    );
}

#[test]
fn div_cx532_display_backing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(display-backing-store)
"##,
    );
}

#[test]
fn div_cx532_fontset_font() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (fontset-font (fontset-default) 'ascii)
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx532_fontset_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (fontset-list)
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx532_fontset_alias() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (new-fontset "cx532-fs" (fontset-spec "Monospace" 'ascii))
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx532_frame_fontset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (frame-parameter (selected-frame) 'font)
  (error (car e)))
"##,
    );
}

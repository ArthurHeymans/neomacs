//! Complex combo batch 432 — 17 probes into X11/GUI backend stubs,
//! selection, cut buffer, font selection, frame focus, display
//! connection, and remaining pixel/frame edge operations.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

/// x-create-frame / x-focus-frame: frame creation and focus.
#[test]
fn div_cx432_x_create_focus_frame() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (x-focus-frame (selected-frame)) (error (car e)))
      (condition-case e (x-parse-geometry "80x24+0+0") (error (car e))))
"##,
    );
}

/// x-dnd-*: drag and drop functions (likely stubbed).
#[test]
fn div_cx432_x_dnd_protocol() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (x-dnd-get-drop-x-y) (error (car e))))
"##,
    );
}

/// x-own-selection-internal / x-get-selection-internal.
#[test]
fn div_cx432_x_own_get_selection() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (x-own-selection-internal 'PRIMARY "test") (error (car e)))
      (condition-case e (x-get-selection-internal 'PRIMARY) (error (car e))))
"##,
    );
}

/// x-set-cut-buffer / x-get-cut-buffer (X11 cut buffers).
#[test]
fn div_cx432_x_cut_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (x-set-cut-buffer "cut test") (error (car e)))
      (condition-case e (x-get-cut-buffer) (error (car e))))
"##,
    );
}

/// x-display-list / x-open-connection / x-close-connection.
#[test]
fn div_cx432_x_display_connect() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (x-display-list) (error (car e))))
"##,
    );
}

/// x-window-property / x-change-window-property.
#[test]
fn div_cx432_x_window_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (x-window-property "WM_NAME") (error (car e)))
      (condition-case e (x-change-window-property "TEST" "data") (error (car e))))
"##,
    );
}

/// x-get-atom-name / x-intern-atom.
#[test]
fn div_cx432_x_atom_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (x-intern-atom "WM_PROTOCOLS") (error (car e)))
      (condition-case e (x-get-atom-name 1) (error (car e))))
"##,
    );
}

/// x-select-font / x-list-fonts deep.
#[test]
fn div_cx432_x_select_list_fonts() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (length (x-list-fonts "monospace")) (error (car e)))
      (condition-case e (x-list-fonts "*") (error (car e))))
"##,
    );
}

/// gui-backend-* selection functions.
#[test]
fn div_cx432_gui_backend_selection() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (gui-backend-get-selection 'PRIMARY) (error (car e)))
      (condition-case e (gui-backend-selection-owner-p 'PRIMARY) (error (car e)))
      (condition-case e (gui-backend-selection-exists-p 'PRIMARY) (error (car e))))
"##,
    );
}

/// face-attribute with multiple resolution frames.
#[test]
fn div_cx432_face_attribute_multi_frame() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (face-attribute 'bold :weight nil 'default)
      (face-attribute 'italic :slant nil 'default)
      (face-attribute 'default :inherit nil 'default))
"##,
    );
}

/// window-state-put with ignore-window-parameters.
#[test]
fn div_cx432_window_state_put_ignore() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "test")
  (let ((state (window-state-get (selected-window))))
    (window-state-put state nil 'safe)))
"##,
    );
}

/// display-pixel-dimensions / display-mm-dimensions (monitor).
#[test]
fn div_cx432_display_pixel_mm_monitor() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (display-pixel-width) (display-pixel-height)
      (display-mm-width) (display-mm-height))
"##,
    );
}

/// font-get with custom font property.
#[test]
fn div_cx432_font_get_custom_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((f (font-spec :family "Monospace")))
  (font-put f :neo-cx432-prop 'custom-value)
  (list (font-get f :neo-cx432-prop)
        (font-get f :family)))
"##,
    );
}

/// menu-bar-open / tooltip functions in batch.
#[test]
fn div_cx432_menu_tooltip_batch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (menu-bar-open (selected-frame)) (error (car e)))
      (condition-case e (tooltip-show "test") (error (car e))))
"##,
    );
}

/// x-send-client-message (X11 client messaging).
#[test]
fn div_cx432_x_send_client_message() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (x-send-client-message (selected-frame) (selected-frame) 0 nil "TEST")
  (error (car e)))
"##,
    );
}

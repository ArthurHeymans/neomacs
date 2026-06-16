//! Complex combo batch 214 — `window` configuration / `window-state` /
//! `window-combination` / `split-window` / `delete-window` /
//! `balance-windows` operations.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx214_split_window_and_delete_round_trip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((n-before (length (window-list))))
  (let ((win (split-window)))
    (let ((n-after-split (length (window-list))))
      (delete-window win)
      (let ((n-after-delete (length (window-list))))
        (list n-before n-after-split n-after-delete
              (>= n-after-split (1+ n-before))
              (= n-after-delete n-before)))))
"##,
    );
}

#[test]
fn div_cx214_window_configuration_save_restore() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((config (current-window-configuration))
      (n-before (length (window-list))))
  (split-window)
  (split-window)
  (let ((n-split (length (window-list))))
    (set-window-configuration config)
    (let ((n-restored (length (window-list))))
      (list n-before n-split n-restored
            (= n-before n-restored)))))
"##,
    );
}

#[test]
fn div_cx214_window_state_get_put() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((state (window-state-get)))
      (list (consp state)
            (window-state-get)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx214_save_window_excursion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((n-before (length (window-list))))
  (save-window-excursion
    (split-window)
    (let ((n-inside (length (window-list))))
      (split-window)
      (let ((n-inside-2 (length (window-list))))
        (list n-before n-inside n-inside-2))))
  (let ((n-after (length (window-list))))
    n-after))
"##,
    );
}

#[test]
fn div_cx214_window_edges_pixel_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((win (selected-window)))
  (list (window-edges win)
        (window-inside-edges win)
        (window-pixel-edges win)
        (window-inside-pixel-edges win)
        (window-absolute-pixel-edges win)))
"##,
    );
}

#[test]
fn div_cx214_window_margins_fringes_scroll_bar_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((win (selected-window)))
  (list (window-margins win)
        (window-fringes win)
        (window-scroll-bar-width win)
        (window-scroll-bars win)))
"##,
    );
}

#[test]
fn div_cx214_balance_windows_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (fboundp 'balance-windows)
      (fboundp 'balance-windows-area)
      (fboundp 'window-tree)
      (fboundp 'window-combined-p)
      (fboundp 'window-parent))
"##,
    );
}

#[test]
fn div_cx214_window_tree_structure_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((tree (window-tree)))
  (list (consp tree)
        (car tree)
        (>= (length tree) 2)))
"##,
    );
}

#[test]
fn div_cx214_get_buffer_window_list_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((buf (current-buffer)))
  (list (consp (get-buffer-window-list buf))
        (windowp (get-buffer-window buf))
        (eq (get-buffer-window buf) (selected-window))))
"##,
    );
}

#[test]
fn div_cx214_window_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((n-before (length (window-list)))
      (config (current-window-configuration)))
  (with-temp-buffer
    (buffer-enable-undo)
    (insert "Window config mega test buffer content")
    (put-text-property 1 6 'face 'bold)
    (let ((m (set-marker (make-marker) 8))
          (ov (make-overlay 4 14)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (window-configuration-to-register ?c)
      (narrow-to-region 2 18)
      (split-window)
      (let ((n-split (length (window-list))))
        (let ((state (list n-before n-split
                           (buffer-string)
                           (marker-position m)
                           (overlay-start ov) (overlay-end ov)
                           (text-properties-at 1))))
          (delete-other-windows)
          (jump-to-register ?c)
          (widen)
          (list state (buffer-string) (marker-position m)
                (overlay-start ov) (overlay-end ov)
                (text-properties-at 1)))))))
"##,
    );
}

//! Oracle parity tests for side-window primitives.
//!
//! Covers: window-main-window, display-buffer-in-side-window,
//! window-toggle-side-windows, window-side/window-slot parameters,
//! window-sides-vertical, window-sides-slots, window-sides-reversed,
//! and edge/error cases.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::assert_oracle_parity;

// ---------------------------------------------------------------------------
// window-main-window
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_main_window_no_side_windows() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((main (window-main-window))
       (root (frame-root-window))
       (main-is-root (eq main root)))
  (list main-is-root))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_main_window_after_side_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((side (display-buffer-in-side-window
              (get-buffer-create "*side-main-test*")
              '((side . left))))
       (main (window-main-window))
       (root (frame-root-window))
       (side-window (window-with-parameter 'window-side 'left)))
  (list (not (eq main root))
        (eq main (window-parent side-window))
        (numberp (window-parent side-window))))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// display-buffer-in-side-window — basic placement
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_display_buffer_left() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*side-left*"))
       (side (display-buffer-in-side-window
              buf
              '((side . left))))
       (edges (window-edges side))
       (side-param (window-parameter side 'window-side))
       (slot-param (window-parameter side 'window-slot))
       (dedicated (window-dedicated-p side))
       (buf-in-side (window-buffer side)))
  (list (eq side-param 'left)
        (eq buf-in-side buf)
        dedicated
        (numberp slot-param)
        (list (car edges) (cadr edges))))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_display_buffer_right() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*side-right*"))
       (side (display-buffer-in-side-window
              buf
              '((side . right))))
       (edges (window-edges side))
       (side-param (window-parameter side 'window-side))
       (slot-param (window-parameter side 'window-slot)))
  (list (eq side-param 'right)
        (numberp slot-param)
        (list (car edges) (cadr edges))))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_display_buffer_top() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*side-top*"))
       (side (display-buffer-in-side-window
              buf
              '((side . top))))
       (edges (window-edges side))
       (side-param (window-parameter side 'window-side))
       (slot-param (window-parameter side 'window-slot)))
  (list (eq side-param 'top)
        (numberp slot-param)
        (list (car edges) (cadr edges))))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_display_buffer_bottom() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*side-bottom*"))
       (side (display-buffer-in-side-window
              buf
              '((side . bottom))))
       (edges (window-edges side))
       (side-param (window-parameter side 'window-side))
       (slot-param (window-parameter side 'window-slot)))
  (list (eq side-param 'bottom)
        (numberp slot-param)
        (list (car edges) (cadr edges))))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// display-buffer-in-side-window — error cases
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_invalid_side_signals_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // An invalid side should signal an error
    let form = r#"
(condition-case err
    (progn
      (display-buffer-in-side-window
       (get-buffer-create "*bad-side*")
       '((side . front)))
      (list 'no-error))
  (error (list 'error-caught (car (cdr err)))))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_invalid_slot_signals_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(condition-case err
    (progn
      (display-buffer-in-side-window
       (get-buffer-create "*bad-slot*")
       '((side . left) (slot . "not-a-number")))
      (list 'no-error))
  (error (list 'error-caught (car (cdr err)))))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// side window slot management — multiple windows on same side
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_multiple_slots_same_side() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf0 (get-buffer-create "*slot-0*"))
       (buf1 (get-buffer-create "*slot-1*"))
       (buf2 (get-buffer-create "*slot-2*"))
       (w0 (display-buffer-in-side-window buf0 '((side . left) (slot . 0))))
       (w1 (display-buffer-in-side-window buf1 '((side . left) (slot . 1))))
       (w2 (display-buffer-in-side-window buf2 '((side . left) (slot . -1))))
       (slot0 (window-parameter w0 'window-slot))
       (slot1 (window-parameter w1 'window-slot))
       (slot2 (window-parameter w2 'window-slot)))
  (list (numberp slot0)
        (numberp slot1)
        (numberp slot2)
        ;; Each window should have the same side
        (eq (window-parameter w0 'window-side) 'left)
        (eq (window-parameter w1 'window-side) 'left)
        (eq (window-parameter w2 'window-side) 'left)
        ;; The windows should be distinct
        (not (eq w0 w1))
        (not (eq w1 w2))
        (not (eq w0 w2))))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_slot_reuse_same_slot() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf-a (get-buffer-create "*reuse-a*"))
       (buf-b (get-buffer-create "*reuse-b*"))
       (w-a (display-buffer-in-side-window buf-a '((side . right) (slot . 2))))
       (w-b (display-buffer-in-side-window buf-b '((side . right) (slot . 2))))
       (buf-a-after (window-buffer w-a))
       (buf-b-after (window-buffer w-b)))
  (list (eq w-a w-b)
        (eq buf-b-after buf-b)))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// side window dedication
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_dedicated_by_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*dedicated-side*"))
       (side (display-buffer-in-side-window
              buf
              '((side . right))))
       (dedicated (window-dedicated-p side)))
  (list dedicated))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_dedicated_explicit_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*side-dedicated-nil*"))
       (side (display-buffer-in-side-window
              buf
              '((side . bottom) (dedicated . nil)))))
  (list (window-dedicated-p side)))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// window-toggle-side-windows
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_toggle_no_side_windows_signals_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(condition-case err
    (progn
      (window-toggle-side-windows)
      (list 'no-error))
  (error (list 'error-caught (car (cdr err)))))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_toggle_after_create_deletes_side_windows() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*toggle-test*"))
       (_side (display-buffer-in-side-window
               buf
               '((side . left))))
       (_toggle (window-toggle-side-windows))
       (any-side-left (window-with-parameter 'window-side 'left))
       (any-side-right (window-with-parameter 'window-side 'right))
       (any-side-top (window-with-parameter 'window-side 'top))
       (any-side-bottom (window-with-parameter 'window-side 'bottom)))
  (list any-side-left any-side-right any-side-top any-side-bottom))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// window-sides-vertical effects
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_sides_vertical_left_occupies_full_height() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((window-sides-vertical t)
       (buf (get-buffer-create "*vert-left*"))
       (side (display-buffer-in-side-window
              buf
              '((side . left))))
       (edges (window-edges side))
       (root (frame-root-window))
       (root-edges (window-edges root))
       (height (nth 3 edges))
       (root-height (nth 3 root-edges)))
  (list (= height root-height)))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_sides_vertical_right_occupies_full_height() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((window-sides-vertical t)
       (buf (get-buffer-create "*vert-right*"))
       (side (display-buffer-in-side-window
              buf
              '((side . right))))
       (edges (window-edges side))
       (root (frame-root-window))
       (root-edges (window-edges root))
       (height (nth 3 edges))
       (root-height (nth 3 root-edges)))
  (list (= height root-height)))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// side window deletion
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_delete_side_window_removes_it() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*delete-side-test*"))
       (side (display-buffer-in-side-window
              buf
              '((side . bottom))))
       (side-existed (window-live-p side))
       (_deleted (delete-window side))
       (side-gone (not (window-live-p side)))
       (still-has-side (window-with-parameter 'window-side 'bottom)))
  (list side-existed side-gone (not still-has-side)))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// window--sides-shown buffer-local variable
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_sides_shown_set_on_display() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*sides-shown-test*"))
       (_side (display-buffer-in-side-window
               buf
               '((side . left))))
       (shown (buffer-local-value 'window--sides-shown buf)))
  (list shown))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// side window with explicit window-width/window-height
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_explicit_width_left() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*explicit-width*"))
       (side (display-buffer-in-side-window
              buf
              '((side . left) (window-width . 25))))
       (edges (window-edges side)))
  (list (nth 2 edges)))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_explicit_height_top() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*explicit-height*"))
       (side (display-buffer-in-side-window
              buf
              '((side . top) (window-height . 5))))
       (edges (window-edges side)))
  (list (nth 3 edges)))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// window-sides-slots — limiting number of slots
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_sides_slots_zero_prevents_creation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((window-sides-slots '(0 0 0 0))
       (result (display-buffer-in-side-window
                (get-buffer-create "*slot-zero*")
                '((side . left)))))
  (list result))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// side windows and split-window interaction
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_split_side_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*split-side*"))
       (side (display-buffer-in-side-window
              buf
              '((side . left))))
       (child (condition-case err
                  (split-window side)
                (error (list 'split-failed (car (cdr err)))))))
  (list (if (windowp child)
            (list (window-parameter side 'window-side)
                  (window-parameter child 'window-side))
          child)))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// side windows and other-window / select-window interactions
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_other_window_skips_side_windows() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((main (selected-window))
       (side-buf (get-buffer-create "*other-win-side*"))
       (side (display-buffer-in-side-window
              side-buf
              '((side . left))))
       ;; Try selecting the side window explicitly, then use other-window
       (_ (select-window side))
       (selected-after-side-select (selected-window))
       (other (other-window-for-scrolling))
       (other-is-side (window-parameter other 'window-side)))
  (list (eq selected-after-side-select side)
        (window-parameter other 'window-side)))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// side window frame parameter persistence
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_window_state_after_toggle_restore() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*state-restore*"))
       (_side (display-buffer-in-side-window
               buf
               '((side . right) (slot . 0))))
       (before-toggle (window-with-parameter 'window-side 'right))
       (had-side-before (and before-toggle t))
       (_toggle (window-toggle-side-windows))
       (after-toggle-gone (window-with-parameter 'window-side 'right))
       (_restore (window-toggle-side-windows))
       (after-restore (window-with-parameter 'window-side 'right)))
  (list had-side-before
        (not after-toggle-gone)
        (and after-restore t)))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// side windows on all four sides simultaneously
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_all_four_sides() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf-l (get-buffer-create "*all-l*"))
       (buf-r (get-buffer-create "*all-r*"))
       (buf-t (get-buffer-create "*all-t*"))
       (buf-b (get-buffer-create "*all-b*"))
       (wl (display-buffer-in-side-window buf-l '((side . left))))
       (wr (display-buffer-in-side-window buf-r '((side . right))))
       (wt (display-buffer-in-side-window buf-t '((side . top))))
       (wb (display-buffer-in-side-window buf-b '((side . bottom))))
       (main-w (window-main-window))
       (side-windows (list wl wr wt wb)))
  (list (every 'window-live-p side-windows)
        (not (memq main-w side-windows))
        (eq (window-parameter wl 'window-side) 'left)
        (eq (window-parameter wr 'window-side) 'right)
        (eq (window-parameter wt 'window-side) 'top)
        (eq (window-parameter wb 'window-side) 'bottom)))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// side window default side (bottom when not specified)
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_default_side_is_bottom() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*default-side*"))
       (side (display-buffer-in-side-window
              buf
              '()))
       (side-param (window-parameter side 'window-side)))
  (list (eq side-param 'bottom)))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// side window persistent parameters
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_parameters_are_persistent() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*persist-params*"))
       (side (display-buffer-in-side-window
              buf
              '((side . left))))
       (side-entry (assq 'window-side window-persistent-parameters))
       (slot-entry (assq 'window-slot window-persistent-parameters)))
  (list (not (null side-entry))
        (not (null slot-entry))
        (eq (cdr side-entry) 'writable)
        (eq (cdr slot-entry) 'writable)))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// side window: same buffer in a side window is reused
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_same_buffer_same_side_reuses() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*reuse-buf*"))
       (w1 (display-buffer-in-side-window buf '((side . left) (slot . 0))))
       (w2 (display-buffer buf))
       (w2-is-w1 (eq w1 w2)))
  (list w2-is-w1))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// window--sides-check integrity after multi-side creation
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_sides_check_no_crash_after_all_sides() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf-l (get-buffer-create "*check-l*"))
       (buf-r (get-buffer-create "*check-r*"))
       (buf-t (get-buffer-create "*check-t*"))
       (buf-b (get-buffer-create "*check-b*"))
       (_wl (display-buffer-in-side-window buf-l '((side . left))))
       (_wr (display-buffer-in-side-window buf-r '((side . right))))
       (_wt (display-buffer-in-side-window buf-t '((side . top))))
       (_wb (display-buffer-in-side-window buf-b '((side . bottom))))
       ;; window--sides-check should not error
       (result (condition-case err
                   (progn
                     (window--sides-check (selected-frame))
                     'ok)
                 (error (list 'check-failed (car (cdr err)))))))
  (list result))
"#;
    assert_oracle_parity(form);
}

// ===========================================================================
// Combo tests: side windows + complex interactions
// ===========================================================================

// ---------------------------------------------------------------------------
// Combo: side windows + quit-restore (window-prev-buffers / window-next-buffers)
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_combo_quit_restore_prev_next_buffers() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf-a (get-buffer-create "*sw-qr-a*"))
       (buf-b (get-buffer-create "*sw-qr-b*"))
       (sw (display-buffer-in-side-window buf-a '((side . left) (slot . 0))))
       (_ (display-buffer-in-side-window buf-b '((side . left) (slot . 0))))
       ;; buf-b replaced buf-a; window-prev-buffers should list buf-a
       (prev (window-prev-buffers sw))
       (next (window-next-buffers sw))
       (quit-restore (window-parameter sw 'quit-restore)))
  (list (not (null prev))
        (null next)
        (not (null quit-restore))
        (eq (car (car prev)) buf-a)
        ;; quit-restore should indicate this is a side window
        (eq (nth 0 quit-restore) 'window)
        (eq (nth 1 quit-restore) sw)
        (eq (nth 2 quit-restore) buf-b)))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// Combo: side windows + delete-other-windows preserves main window
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_combo_delete_other_windows_preserves_side() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*sw-del-other*"))
       (sw (display-buffer-in-side-window buf '((side . left))))
       (main (window-main-window))
       (_ (select-window main))
       (_ (delete-other-windows main))
       (sw-still-alive (window-live-p sw))
       (side-remains (window-with-parameter 'window-side 'left)))
  (list sw-still-alive
        (and side-remains t)))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// Combo: side windows + buffer kill → window behavior
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_combo_buffer_kill_window_live() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*sw-kill-buf*"))
       (sw (display-buffer-in-side-window buf '((side . right))))
       (was-live (window-live-p sw))
       (_ (kill-buffer buf))
       (still-live (window-live-p sw)))
  (list was-live still-live))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// Combo: side windows + switch-to-buffer vs dedicated
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_combo_switch_to_buffer_dedicated_side() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf1 (get-buffer-create "*sw-switch-1*"))
       (buf2 (get-buffer-create "*sw-switch-2*"))
       (sw (display-buffer-in-side-window buf1 '((side . left))))
       (selected-before (selected-window))
       (_ (select-window sw))
       ;; Try switching buffer in dedicated side window
       (result (condition-case err
                   (progn
                     (switch-to-buffer buf2)
                     (list 'switched (eq (current-buffer) buf2)))
                 (error (list 'cannot-switch (car (cdr err)))))))
  (list result
        (window-parameter sw 'window-side)
        (window-dedicated-p sw)))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// Combo: side windows + resize + re-split
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_combo_resize_and_split() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*sw-resize*"))
       (sw (display-buffer-in-side-window
            buf
            '((side . left) (window-width . 15))))
       (orig-width (window-total-width sw))
       ;; Now try splitting the side window
       (child (condition-case err
                  (split-window sw 5 'below)
                (error (list 'split-err (car (cdr err))))))
       (after-width (window-total-width sw)))
  (list orig-width
        after-width
        (if (windowp child)
            (list 'child-created
                  (window-total-width child)
                  (window-parameter child 'window-side)
                  (window-parameter child 'window-slot))
          child)))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// Combo: side windows + balance-windows
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_combo_balance_windows_ignores_side() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*sw-balance*"))
       (sw (display-buffer-in-side-window buf '((side . left))))
       (main (window-main-window))
       ;; Split main window
       (_ (select-window main))
       (lower (split-window main nil 'below))
       (widths-before (list (window-total-width sw)
                            (window-total-width main)
                            (window-total-width lower)))
       (_balance (balance-windows))
       (widths-after (list (window-total-width sw)
                           (window-total-width main)
                           (window-total-width lower))))
  (list (equal widths-before widths-after)
        widths-before
        widths-after))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// Combo: side windows + window-state-get/put round-trip
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_combo_window_state_round_trip() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*sw-state-round*"))
       (sw (display-buffer-in-side-window
            buf
            '((side . left) (slot . 0))))
       ;; Save state of the entire frame
       (state (window-state-get (frame-root-window) t))
       (side-before (window-parameter sw 'window-side))
       (slot-before (window-parameter sw 'window-slot))
       (buf-before (window-buffer sw))
       ;; Delete all windows and restore
       (_ (delete-other-windows (window-main-window)))
       (_ (window-state-put state (frame-root-window) 'safe))
       ;; Find the side window again
       (sw-after (window-with-parameter 'window-side 'left))
       (side-after (and sw-after (window-parameter sw-after 'window-side)))
       (slot-after (and sw-after (window-parameter sw-after 'window-slot)))
       (buf-after (and sw-after (window-buffer sw-after))))
  (list side-before slot-before
        side-after slot-after
        (eq buf-before buf-after)))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// Combo: side windows + narrow + widen in side window buffer
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_combo_narrow_widen_in_side_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*sw-narrow*"))
       (_ (with-current-buffer buf
            (insert "line one\nline two\nline three\nline four\n")))
       (sw (display-buffer-in-side-window buf '((side . bottom))))
       (total-lines-before (count-lines (point-min) (point-max)))
       ;; Narrow in the side window's buffer
       (_ (with-current-buffer buf
            (narrow-to-region 10 25)))
       (narrow-start (point-min))
       (narrow-end (point-max))
       (_ (widen))
       (total-lines-after (count-lines (point-min) (point-max))))
  (list total-lines-before
        narrow-start narrow-end
        (= total-lines-before total-lines-after)))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// Combo: side windows + window-configuration-change-hook
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_combo_config_change_hook_fires() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((hook-fired nil))
  (let* ((buf (get-buffer-create "*sw-hook*"))
         (_add (add-hook 'window-configuration-change-hook
                         (lambda () (setq hook-fired t))))
         (sw (display-buffer-in-side-window buf '((side . left)))))
    (list hook-fired
          (window-parameter sw 'window-side))))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// Combo: side windows + fit-window-to-buffer
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_combo_fit_window_to_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*sw-fit*"))
       (_ (with-current-buffer buf
            (insert "short\n")))
       (sw (display-buffer-in-side-window buf '((side . bottom))))
       (height-before (window-total-height sw))
       (_fit (condition-case err
                 (fit-window-to-buffer sw)
               (error (list 'fit-err (car (cdr err))))))
       (height-after (window-total-height sw)))
  (list height-before height-after
        (< height-after height-before)))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// Combo: side windows + replace-buffer-in-windows
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_combo_replace_buffer_in_windows() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf1 (get-buffer-create "*sw-replace-1*"))
       (buf2 (get-buffer-create "*sw-replace-2*"))
       (sw (display-buffer-in-side-window buf1 '((side . left))))
       (_replace (replace-buffer-in-windows buf1 buf2))
       (buf-in-sw (window-buffer sw)))
  (list (eq buf-in-sw buf2)
        (window-parameter sw 'window-side)))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// Combo: side windows + set-window-dedicated-p change
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_combo_change_dedication_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*sw-ded-change*"))
       (sw (display-buffer-in-side-window buf '((side . top))))
       (ded-before (window-dedicated-p sw))
       (_ (set-window-dedicated-p sw 'direct))
       (ded-after (window-dedicated-p sw)))
  (list ded-before ded-after))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// Combo: side windows + other-window (not for-scrolling) cycles main only
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_combo_other_window_cycle_skips_sides() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf-l (get-buffer-create "*sw-ow-l*"))
       (buf-r (get-buffer-create "*sw-ow-r*"))
       (wl (display-buffer-in-side-window buf-l '((side . left))))
       (wr (display-buffer-in-side-window buf-r '((side . right))))
       (main (window-main-window))
       ;; Split main to have multiple non-side windows
       (_ (select-window main))
       (lower (split-window main nil 'below))
       ;; Now cycle: from main, call other-window twice
       ;; Should visit only main-area windows, not side windows
       (_ (select-window main))
       (w1 (selected-window))
       (_ (other-window 1))
       (w2 (selected-window))
       (_ (other-window 1))
       (w3 (selected-window))
       (w1-side (window-parameter w1 'window-side))
       (w2-side (window-parameter w2 'window-side))
       (w3-side (window-parameter w3 'window-side)))
  (list w1-side w2-side w3-side
        (eq w1 w3)  ;; should cycle back to start
        (not (eq w1 w2))))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// Combo: side window + minibuffer interaction
// ---------------------------------------------------------------------------

#[test]
fn oracle_side_window_combo_minibuffer_window_never_side() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((mini (minibuffer-window))
       (mini-side (window-parameter mini 'window-side))
       (buf (get-buffer-create "*sw-mini*"))
       (sw (display-buffer-in-side-window buf '((side . left))))
       (mini-still (minibuffer-window))
       (mini-side-after (window-parameter mini-still 'window-side)))
  (list mini-side mini-side-after
        (not (eq sw mini-still))))
"#;
    assert_oracle_parity(form);
}

// ---------------------------------------------------------------------------
// Combo: side windows + window-resize (shrink/enlarge)
// ---------------------------------------------------------------------------

// ===========================================================================
// Deep probes: y-offset-1 divergence investigation
// ===========================================================================
fn oracle_side_window_deep_mode_line_height_effect_on_offset() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*sw-offset*"))
       (sw (display-buffer-in-side-window buf '((side . left))))
       (edges (window-edges sw))
       (pixel-edges (window-pixel-edges sw))
       (root-edges (window-edges (frame-root-window)))
       (mode-line-h (window-mode-line-height sw))
       (header-line-h (window-header-line-height sw))
       (tab-line-h (window-tab-line-height sw))
       (top-line (window-top-line sw))
       (pixel-top (window-pixel-top sw)))
  (list (nth 0 edges) (nth 1 edges) (nth 2 edges) (nth 3 edges)
        mode-line-h header-line-h tab-line-h
        top-line pixel-top))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_deep_all_four_sides_edges_compare() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((bl (get-buffer-create "*edges-l*"))
       (br (get-buffer-create "*edges-r*"))
       (bt (get-buffer-create "*edges-t*"))
       (bb (get-buffer-create "*edges-b*"))
       (wl (display-buffer-in-side-window bl '((side . left))))
       (wr (display-buffer-in-side-window br '((side . right))))
       (wt (display-buffer-in-side-window bt '((side . top))))
       (wb (display-buffer-in-side-window bb '((side . bottom))))
       (root (frame-root-window))
       (root-edges (window-edges root))
       (l-edges (window-edges wl))
       (r-edges (window-edges wr))
       (t-edges (window-edges wt))
       (b-edges (window-edges wb)))
  (list l-edges r-edges t-edges b-edges root-edges))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_deep_minibuffer_window_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((mini (minibuffer-window))
       (mini-edges (window-edges mini))
       (root (frame-root-window))
       (root-edges (window-edges root))
       (frame-edges (window-edges (frame-root-window) t))
       (buf (get-buffer-create "*sw-miniedge*"))
       (sw (display-buffer-in-side-window buf '((side . left))))
       (sw-edges (window-edges sw))
       (mini-edges-after (window-edges mini)))
  (list mini-edges root-edges sw-edges mini-edges-after))
"#;
    assert_oracle_parity(form);
}

// ===========================================================================
// Deep probes: dedication divergence investigation
// ===========================================================================

#[test]
fn oracle_side_window_deep_display_buffer_mark_dedicated_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((val display-buffer-mark-dedicated)
       (buf (get-buffer-create "*sw-ded-mark*"))
       (sw (display-buffer-in-side-window buf '((side . bottom))))
       (ded (window-dedicated-p sw)))
  (list val ded))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_deep_dedicated_explicit_soft() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*sw-ded-soft*"))
       (sw (display-buffer-in-side-window
            buf
            '((side . left) (dedicated . soft)))))
  (list (window-dedicated-p sw)))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_deep_dedicated_blocks_switch_to_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf1 (get-buffer-create "*sw-ded-block-1*"))
       (buf2 (get-buffer-create "*sw-ded-block-2*"))
       (sw (display-buffer-in-side-window buf1 '((side . left))))
       (_ (select-window sw))
       (result (condition-case err
                   (progn
                     (switch-to-buffer buf2 'norecord)
                     (list 'ok (eq (current-buffer) buf2)
                           (eq (window-buffer sw) buf2)))
                 (error (list 'blocked (car (cdr err)))))))
  (list result))
"#;
    assert_oracle_parity(form);
}

// ===========================================================================
// Deep probes: kill-buffer lifecycle divergence investigation
// ===========================================================================

#[test]
fn oracle_side_window_deep_bury_buffer_in_side_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf1 (get-buffer-create "*sw-bury-1*"))
       (buf2 (get-buffer-create "*sw-bury-2*"))
       (sw (display-buffer-in-side-window buf1 '((side . left))))
       (_ (display-buffer-in-side-window buf2 '((side . left) (slot . 0))))
       (_ (bury-buffer buf2))
       (buf-in-sw (window-buffer sw))
       (prev (window-prev-buffers sw))
       (next (window-next-buffers sw)))
  (list buf-in-sw
        (length prev)
        (length next)))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_deep_switch_to_prev_buffer_after_bury() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf-a (get-buffer-create "*sw-spb-a*"))
       (buf-b (get-buffer-create "*sw-spb-b*"))
       (buf-c (get-buffer-create "*sw-spb-c*"))
       (sw (display-buffer-in-side-window buf-a '((side . right))))
       (_ (display-buffer-in-side-window buf-b '((side . right) (slot . 0))))
       (_ (display-buffer-in-side-window buf-c '((side . right) (slot . 0))))
       (_ (select-window sw))
       (_ (switch-to-prev-buffer sw))
       (buf-after-spb (window-buffer sw))
       (_ (switch-to-next-buffer sw))
       (buf-after-snb (window-buffer sw)))
  (list (eq buf-after-spb buf-b)
        (eq buf-after-snb buf-c)))
"#;
    assert_oracle_parity(form);
}

// ===========================================================================
// Deep probes: window-configuration-change-hook investigation
// ===========================================================================

#[test]
fn oracle_side_window_deep_config_change_hook_delete_side_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((hook-fired 0))
  (let* ((buf (get-buffer-create "*sw-hook-del*"))
         (_add (add-hook 'window-configuration-change-hook
                         (lambda () (setq hook-fired (1+ hook-fired)))))
         (sw (display-buffer-in-side-window buf '((side . left))))
         (count-after-create hook-fired)
         (_del (delete-window sw))
         (count-after-delete hook-fired))
    (list count-after-create count-after-delete)))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_deep_config_change_hook_toggle_side_windows() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((hook-count 0))
  (let* ((buf (get-buffer-create "*sw-hook-tog*"))
         (_add (add-hook 'window-configuration-change-hook
                         (lambda () (setq hook-count (1+ hook-count)))))
         (_ (display-buffer-in-side-window buf '((side . bottom))))
         (count-after-create hook-count)
         (_ (window-toggle-side-windows))
         (count-after-toggle1 hook-count)
         (_ (window-toggle-side-windows))
         (count-after-toggle2 hook-count))
    (list count-after-create count-after-toggle1 count-after-toggle2)))
"#;
    assert_oracle_parity(form);
}

// ===========================================================================
// Deep probes: window-sides-vertical investigation
// ===========================================================================

#[test]
fn oracle_side_window_deep_sides_vertical_vs_horizontal_dimensions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((window-sides-vertical t)
       (buf-l (get-buffer-create "*sw-dim-l*"))
       (buf-r (get-buffer-create "*sw-dim-r*"))
       (wl (display-buffer-in-side-window buf-l '((side . left) (window-width . 20))))
       (wr (display-buffer-in-side-window buf-r '((side . right) (window-width . 20))))
       (l-size (window-total-size wl))
       (r-size (window-total-size wr))
       (l-h-size (window-total-size wl t))
       (r-h-size (window-total-size wr t))
       (l-pixel (window-pixel-width wl))
       (r-pixel (window-pixel-width wr))
       (l-body (list (window-body-width wl) (window-body-height wl))))
  (list l-size r-size l-h-size r-h-size l-pixel r-pixel l-body))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_deep_sides_vertical_nil_left_not_full_height() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((window-sides-vertical nil)
       (buf (get-buffer-create "*sw-vert-nil*"))
       (sw (display-buffer-in-side-window buf '((side . left))))
       (sw-edges (window-edges sw))
       (root-edges (window-edges (frame-root-window)))
       (sw-bottom (nth 3 sw-edges))
       (root-bottom (nth 3 root-edges)))
  (list (= sw-bottom root-bottom)
        sw-bottom root-bottom))
"#;
    assert_oracle_parity(form);
}

// ===========================================================================
// New territory: functions not yet tested
// ===========================================================================

#[test]
fn oracle_side_window_get_buffer_window_returns_side_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*sw-gbw*"))
       (sw (display-buffer-in-side-window buf '((side . left))))
       (found (get-buffer-window buf))
       (found-all (get-buffer-window buf 'all-frames)))
  (list (eq found sw)
        (eq found-all sw)))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_walk_windows_filters_with_side_param() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf-l (get-buffer-create "*sw-walk-l*"))
       (buf-r (get-buffer-create "*sw-walk-r*"))
       (wl (display-buffer-in-side-window buf-l '((side . left))))
       (wr (display-buffer-in-side-window buf-r '((side . right))))
       (all-windows 0)
       (side-windows 0)
       (main-windows 0))
  (walk-windows (lambda (w)
                  (setq all-windows (1+ all-windows))
                  (if (window-parameter w 'window-side)
                      (setq side-windows (1+ side-windows))
                    (setq main-windows (1+ main-windows))))
                'nominibuf)
  (list all-windows side-windows main-windows
        (> all-windows 0)))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_delete_other_windows_from_side_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*sw-del-other-from-side*"))
       (sw (display-buffer-in-side-window buf '((side . left))))
       (_ (select-window sw))
       (result (condition-case err
                   (progn
                     (delete-other-windows sw)
                     (list 'ok
                           (window-live-p sw)
                           (length (window-list nil 'nominibuf))))
                 (error (list 'error (car (cdr err)))))))
  (list result))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_set_window_buffer_directly() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf1 (get-buffer-create "*sw-setbuf-1*"))
       (buf2 (get-buffer-create "*sw-setbuf-2*"))
       (sw (display-buffer-in-side-window buf1 '((side . right))))
       (ded-before (window-dedicated-p sw))
       (_ (set-window-buffer sw buf2))
       (buf-now (window-buffer sw))
       (side-still (window-parameter sw 'window-side))
       (ded-after (window-dedicated-p sw)))
  (list (eq buf-now buf2)
        (eq side-still 'right)
        ded-before ded-after))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_display_buffer_reuse_window_over_side() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf1 (get-buffer-create "*sw-reuse-ovr-1*"))
       (buf2 (get-buffer-create "*sw-reuse-ovr-2*"))
       (sw (display-buffer-in-side-window buf1 '((side . left))))
       ;; Now use plain display-buffer for buf2 - should it reuse the side window?
       ;; display-buffer-reuse-window says no (side windows are dedicated)
       (result (condition-case err
                   (let ((w2 (display-buffer buf2)))
                     (list 'got-window
                           (eq w2 sw)
                           (window-parameter w2 'window-side)))
                 (error (list 'err (car (cdr err)))))))
  (list result))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_scroll_bars_fringes_margins() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*sw-scroll*"))
       (sw (display-buffer-in-side-window buf '((side . left))))
       (scroll-bars (window-scroll-bars sw))
       (fringes (window-fringes sw))
       (margins (window-margins sw)))
  (list scroll-bars fringes margins))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_split_with_explicit_side_parameter() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((main (window-main-window))
       (_ (select-window main))
       (sw (split-window main nil 'right))
       (side-param (window-parameter sw 'window-side)))
  (list side-param
        (window-parameter sw 'window-slot)))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_set_window_parameter_side_manually() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((main (window-main-window))
       (orig-side (window-parameter main 'window-side))
       (_ (set-window-parameter main 'window-side 'left))
       (after-set (window-parameter main 'window-side))
       (main-now (window-main-window))
       (_cleanup (set-window-parameter main 'window-side nil))
       (after-cleanup (window-parameter main 'window-side)))
  (list orig-side after-set main-now after-cleanup))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_window_list_with_different_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*sw-wl*"))
       (sw (display-buffer-in-side-window buf '((side . left))))
       (all-no-mini (window-list nil 'nominibuf))
       (all-with-mini (window-list nil 'nominibuf nil))
       (has-side (memq sw all-no-mini))
       (count (length all-no-mini)))
  (list (and has-side t)
        count
        (= (length all-no-mini) (length all-with-mini))))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_display_buffer_in_side_window_with_extra_params() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*sw-extra-params*"))
       (sw (display-buffer-in-side-window
            buf
            '((side . bottom)
              (window-parameters (no-other-window . t)
                                 (modeline . nil)))))
       (side (window-parameter sw 'window-side))
       (slot (window-parameter sw 'window-slot))
       (no-other (window-parameter sw 'no-other-window))
       (modeline (window-parameter sw 'modeline)))
  (list (eq side 'bottom)
        (numberp slot)
        no-other
        modeline))
"#;
    assert_oracle_parity(form);
}

// ===========================================================================
// More deep probes: dedication blocking, atom, delete-main, use-time
// ===========================================================================

#[test]
fn oracle_side_window_deep_set_window_buffer_on_dedicated_nil_side() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf1 (get-buffer-create "*sw-setbuf-nil-1*"))
       (buf2 (get-buffer-create "*sw-setbuf-nil-2*"))
       (sw (display-buffer-in-side-window
            buf1
            '((side . left) (dedicated . nil))))
       (_ (set-window-buffer sw buf2))
       (buf-now (window-buffer sw)))
  (list (eq buf-now buf2)
        (window-dedicated-p sw)))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_deep_display_buffer_reuse_window_with_side_alist() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf1 (get-buffer-create "*sw-reuse-alist-1*"))
       (buf2 (get-buffer-create "*sw-reuse-alist-2*"))
       (_ (display-buffer-in-side-window buf1 '((side . left))))
       (result (condition-case err
                   (let ((w (display-buffer
                             buf2
                             '((display-buffer-reuse-window
                                display-buffer-in-side-window)
                               (side . left)))))
                     (list 'got-window
                           (eq (window-buffer w) buf2)
                           (window-parameter w 'window-side)
                           (window-dedicated-p w)))
                 (error (list 'err (car (cdr err)))))))
  (list result))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_deep_window_atom_root_side_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*sw-atom*"))
       (sw (display-buffer-in-side-window buf '((side . left))))
       (atom-root (window-atom-root sw)))
  (list (eq atom-root sw)
        (window-parameter sw 'window-atom)))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_deep_delete_main_window_with_side_windows() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*sw-del-main*"))
       (sw (display-buffer-in-side-window buf '((side . left))))
       (main (window-main-window))
       (result (condition-case err
                   (progn
                     (select-window main)
                     (delete-window main)
                     (list 'after-delete
                           (window-live-p sw)
                           (length (window-list nil 'nominibuf))))
                 (error (list 'delete-err (car (cdr err)))))))
  (list result))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_deep_window_use_time_side_vs_main() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*sw-use-time*"))
       (sw (display-buffer-in-side-window buf '((side . left))))
       (main (window-main-window))
       (sw-time (window-use-time sw))
       (main-time (window-use-time main)))
  (list (numberp sw-time)
        (numberp main-time)
        (>= sw-time main-time)))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_deep_window_resize_with_side_window_present() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*sw-resize-main*"))
       (sw (display-buffer-in-side-window buf '((side . right) (window-width . 25))))
       (main (window-main-window))
       (main-width-before (window-total-width main))
       (_ (select-window main))
       (_enlarge (condition-case err
                     (enlarge-window 2 t)
                   (error (list 'enlarge-err (car (cdr err))))))
       (main-width-after (window-total-width main))
       (sw-width-after (window-total-width sw)))
  (list main-width-before main-width-after sw-width-after))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_deep_set_window_start_in_side_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*sw-start*"))
       (_ (with-current-buffer buf
            (insert "line 1\nline 2\nline 3\nline 4\nline 5\n"
                    "line 6\nline 7\nline 8\nline 9\nline 10\n")))
       (sw (display-buffer-in-side-window buf '((side . bottom))))
       (start-before (window-start sw))
       (_ (set-window-start sw 10))
       (start-after (window-start sw))
       (point-at (window-point sw)))
  (list start-before start-after point-at))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_deep_split_window_below_side_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*sw-split-below*"))
       (sw (display-buffer-in-side-window buf '((side . top))))
       (sw-height-before (window-total-height sw))
       (result (condition-case err
                   (let ((lower (split-window sw nil 'below)))
                     (list 'split-ok
                           (window-parameter lower 'window-side)
                           (window-parameter lower 'window-slot)
                           (window-total-height sw)
                           (window-total-height lower)))
                 (error (list 'split-err (car (cdr err)))))))
  (list result))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_deep_window_body_vs_total_dimensions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*sw-body*"))
       (sw (display-buffer-in-side-window buf '((side . left))))
       (total-w (window-total-width sw))
       (body-w (window-body-width sw))
       (total-h (window-total-height sw))
       (body-h (window-body-height sw))
       (margins (window-margins sw))
       (fringes (window-fringes sw))
       (scroll-bar-w (car (window-scroll-bars sw)))
       (right-div (window-right-divider-width sw)))
  (list total-w body-w total-h body-h margins fringes scroll-bar-w right-div))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_side_window_deep_two_side_windows_opposite_sides_vertical() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((window-sides-vertical t)
       (buf-l (get-buffer-create "*sw-opp-l*"))
       (buf-r (get-buffer-create "*sw-opp-r*"))
       (wl (display-buffer-in-side-window buf-l '((side . left) (window-width . 15))))
       (wr (display-buffer-in-side-window buf-r '((side . right) (window-width . 15))))
       (l-edges (window-edges wl))
       (r-edges (window-edges wr))
       (main-edges (window-edges (window-main-window)))
       (l-height (nth 3 l-edges))
       (r-height (nth 3 r-edges))
       (main-height (nth 3 main-edges)))
  (list l-edges r-edges main-edges
        (= l-height r-height)
        (= l-height main-height)))
"#;
    assert_oracle_parity(form);
}

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

#[test]
fn oracle_side_window_combo_window_resize_enlarge_shrink() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((buf (get-buffer-create "*sw-resize2*"))
       (sw (display-buffer-in-side-window
            buf
            '((side . left) (window-width . 20))))
       (width-before (window-total-width sw))
       (_enlarge (condition-case err
                     (enlarge-window 5 t)
                   (error (list 'enlarge-err (car (cdr err))))))
       (width-after-enlarge (window-total-width sw))
       (_shrink (condition-case err
                    (shrink-window 5 t)
                  (error (list 'shrink-err (car (cdr err))))))
       (width-after-shrink (window-total-width sw)))
  (list width-before width-after-enlarge width-after-shrink
        ;; Should be able to restore original width
        (= width-before width-after-shrink)))
"#;
    assert_oracle_parity(form);
}

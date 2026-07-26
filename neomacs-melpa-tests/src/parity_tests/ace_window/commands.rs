use super::assert_ace_window_parity;
use expect_test::expect;

#[test]
fn ace_window_public_selection_commands_forward_exact_mode_lines_and_actions() {
    let elisp_form = r##"(progn
         (setq ace-window--test-events nil)
         (cl-letf
             (((symbol-function 'aw-select)
               (lambda (mode-line action)
                 (push
                  (list
                   mode-line
                   action)
                  ace-window--test-events)
                 (list
                  'selected
                  mode-line
                  action))))
           (list
            (ace-select-window)
            (ace-delete-window)
            (ace-swap-window)
            (ace-delete-other-windows)
            (nreverse
             ace-window--test-events))))"##;
    let expect = expect![[
        r#"OK ((selected " Ace - Window" aw-switch-to-window) (selected " Ace - Delete Window" aw-delete-window) (selected " Ace - Swap Window" aw-swap-window) (selected " Ace - Delete Other Windows" delete-other-windows) ((" Ace - Window" aw-switch-to-window) (" Ace - Delete Window" aw-delete-window) (" Ace - Swap Window" aw-swap-window) (" Ace - Delete Other Windows" delete-other-windows)))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_main_command_resets_path_and_dispatches_zero_swap_delete_and_default_arguments() {
    let elisp_form = r##"(mapcar
         (lambda (argument)
           (let ((aw-ignore-on t))
             (setq
              avy-current-path
              "prebound"
              ace-window--test-events
              nil)
             (cl-letf
                 (((symbol-function
                    'ace-select-window)
                   (lambda ()
                     (push
                      (list
                       'select
                       aw-ignore-on)
                      ace-window--test-events)
                     'select-result))
                  ((symbol-function
                    'ace-swap-window)
                   (lambda ()
                     (push '(swap)
                           ace-window--test-events)
                     'swap-result))
                  ((symbol-function
                    'ace-delete-window)
                   (lambda ()
                     (push '(delete)
                           ace-window--test-events)
                     'delete-result)))
               (list
                argument
                (ace-window argument)
                avy-current-path
                aw-ignore-on
                (nreverse
                 ace-window--test-events)))))
         '(0 4 16 1 7))"##;
    let expect = expect![[
        r#"OK ((0 select-result "" t ((select nil))) (4 swap-result "" t ((swap))) (16 delete-result "" t ((delete))) (1 select-result "" t ((select t))) (7 select-result "" t ((select t))))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_display_buffer_maps_reusable_frame_contract_and_short_circuits() {
    let elisp_form = r##"(mapcar
         (lambda (fixture)
           (setq
            ace-window--test-events
            nil
            ace-window--test-count
            (nth 2 fixture))
           (cl-letf
               (((symbol-function
                  'aw-window-list)
                 (lambda ()
                   (make-list
                    ace-window--test-count
                    'window)))
                ((symbol-function 'aw-select)
                 (lambda (mode-line)
                   (push
                    (list
                     'select
                     mode-line
                     aw-scope
                     aw-ignore-current)
                    ace-window--test-events)
                   'selected-window))
                ((symbol-function
                  'window--display-buffer)
                 (lambda
                     (buffer window action)
                   (push
                    (list
                     'display
                     buffer
                     window
                     action)
                    ace-window--test-events)
                   'display-result)))
             (list
              fixture
              (ace-display-buffer
               'fixture-buffer
               (list
                (cons
                 'inhibit-same-window
                 (nth 1 fixture))
                (cons
                 'reusable-frames
                 (nth 0 fixture))))
              (nreverse
               ace-window--test-events))))
         '((nil nil 3)
           (visible t 3)
           (0 nil 3)
           (t t 3)
           (other nil 3)
           (visible nil 1)))"##;
    let expect = expect![[
        r#"OK (((nil nil 3) display-result ((select "Ace - Display Buffer" frame nil) (display fixture-buffer selected-window reuse))) ((visible t 3) display-result ((select "Ace - Display Buffer" visible t) (display fixture-buffer selected-window reuse))) ((0 nil 3) display-result ((select "Ace - Display Buffer" global nil) (display fixture-buffer selected-window reuse))) ((t t 3) display-result ((select "Ace - Display Buffer" global t) (display fixture-buffer selected-window reuse))) ((other nil 3) nil nil) ((visible nil 1) nil nil))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_transpose_frame_forwards_selected_window_frame_and_result() {
    let elisp_form = r##"(progn
         (setq ace-window--test-events nil)
         (cl-letf
             (((symbol-function
                'window-frame)
               (lambda (window)
                 (push
                  (list
                   'window-frame
                   window)
                  ace-window--test-events)
                 'fixture-frame))
              ((symbol-function
                'transpose-frame)
               (lambda (frame)
                 (push
                  (list
                   'transpose
                   frame)
                  ace-window--test-events)
                 'transpose-result)))
           (list
            (aw-transpose-frame
             'fixture-window)
            (nreverse
             ace-window--test-events))))"##;
    let expect =
        expect!["OK (transpose-result ((window-frame fixture-window) (transpose fixture-frame)))"];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_window_order_covers_frame_positions_edges_and_reverse_setting() {
    let elisp_form = r##"(progn
         (setq
          ace-window--test-frames
          '((left-frame . (10 . 0))
            (right-frame . (20 . 0))
            (nil-frame . (nil . nil))
            (same-frame . (10 . 0)))
          ace-window--test-window-frames
          '((left . left-frame)
            (right . right-frame)
            (top-left . same-frame)
            (top-right . same-frame)
            (bottom-left . same-frame)
            (nil-left . nil-frame)
            (nil-right . nil-frame))
          ace-window--test-edges
          '((left . (0 0 10 10))
            (right . (0 0 10 10))
            (top-left . (0 0 10 10))
            (top-right . (10 0 20 10))
            (bottom-left . (0 10 10 20))
            (nil-left . (0 0 10 10))
            (nil-right . (10 0 20 10))))
         (cl-letf
             (((symbol-function
                'window-frame)
               (lambda (window)
                 (cdr
                  (assq
                   window
                   ace-window--test-window-frames))))
              ((symbol-function
                'window-edges)
               (lambda (window)
                 (cdr
                  (assq
                   window
                   ace-window--test-edges))))
              ((symbol-function
                'frame-position)
               (lambda
                   (&optional frame)
                 (cdr
                  (assq
                   frame
                   ace-window--test-frames)))))
           (mapcar
            (lambda (fixture)
              (let ((aw-reverse-frame-list
                     (nth 2 fixture)))
                (list
                 fixture
                 (and
                  (aw-window<
                   (nth 0 fixture)
                   (nth 1 fixture))
                  t))))
            '((left right nil)
              (left right t)
              (right left nil)
              (right left t)
              (top-left top-right nil)
              (top-right top-left nil)
              (top-left bottom-left nil)
              (bottom-left top-left nil)
              (nil-left nil-right nil)))))"##;
    let expect = expect![
        "OK (((left right nil) t) ((left right t) nil) ((right left nil) nil) ((right left t) t) ((top-left top-right nil) t) ((top-right top-left nil) nil) ((top-left bottom-left nil) t) ((bottom-left top-left nil) nil) ((nil-left nil-right nil) t))"
    ];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_ring_push_uses_argument_for_deduplication_but_stores_selected_window() {
    let elisp_form = r##"(let ((aw--window-ring
              (make-ring 4)))
         (setq
          ace-window--test-selected
          'selected-a)
         (cl-letf
             (((symbol-function
                'selected-window)
               (lambda ()
                 ace-window--test-selected)))
           (let ((results nil))
             (push
              (list
               (aw--push-window
                'target-a)
               (ring-elements
                aw--window-ring))
              results)
             (setq
              ace-window--test-selected
              'selected-b)
             (push
              (list
               (aw--push-window
                'selected-a)
               (ring-elements
                aw--window-ring))
              results)
             (push
              (list
               (aw--push-window
                'target-c)
               (ring-elements
                aw--window-ring))
              results)
             (nreverse results))))"##;
    let expect = expect![
        "OK ((selected-a (selected-a)) (nil (selected-a)) (selected-b (selected-b selected-a)))"
    ];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_ring_pop_skips_dead_and_current_then_uses_two_window_fallback_or_errors() {
    let elisp_form = r##"(mapcar
         (lambda (branch)
           (let ((aw--window-ring
                  (make-ring 5)))
             (when (eq branch 'stored)
               (dolist
                   (window
                    '(live current dead))
                 (ring-insert
                  aw--window-ring
                  window)))
             (setq
              ace-window--test-selected
              'current
              ace-window--test-events
              nil)
             (cl-letf
                 (((symbol-function
                    'selected-window)
                   (lambda ()
                     ace-window--test-selected))
                  ((symbol-function
                    'window-live-p)
                   (lambda (window)
                     (not
                      (eq window 'dead))))
                  ((symbol-function
                    'aw-window-list)
                   (lambda ()
                     (if (eq branch
                             'fallback)
                         '(current other)
                       '(current
                         other
                         third))))
                  ((symbol-function
                    'other-window)
                   (lambda (count)
                     (push
                      (list
                       'other-window
                       count)
                      ace-window--test-events)
                     (setq
                      ace-window--test-selected
                      'other))))
               (list
                branch
                (condition-case error
                    (list
                     'ok
                     (aw--pop-window))
                  (error
                   (list 'error error)))
                (ring-elements
                 aw--window-ring)
                (nreverse
                 ace-window--test-events)))))
         '(stored fallback error))"##;
    let expect = expect![[
        r#"OK ((stored (ok live) nil nil) (fallback (ok other) nil ((other-window 1))) (error (error (error "No previous windows stored")) nil nil))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_switch_to_window_pushes_history_focuses_other_frame_selects_or_errors() {
    let elisp_form = r##"(mapcar
         (lambda (fixture)
           (setq ace-window--test-events
                 nil)
           (cl-letf
               (((symbol-function
                 'window-frame)
                 (lambda (window)
                   (if (eq window
                           'other)
                       'other-frame
                     'current-frame)))
                ((symbol-function
                  'selected-frame)
                 (lambda ()
                   'current-frame))
                ((symbol-function
                  'frame-live-p)
                 (lambda (frame)
                   (not
                    (eq frame 'dead-frame))))
                ((symbol-function
                 'window-live-p)
                 (lambda (window)
                   (not
                    (eq window 'dead))))
                ((symbol-function
                  'aw--push-window)
                 (lambda (window)
                   (push
                    (list 'push window)
                    ace-window--test-events)))
                ((symbol-function
                  'selected-window)
                 (lambda ()
                   'start-window))
                ((symbol-function
                  'select-frame-set-input-focus)
                 (lambda (frame)
                   (push
                    (list 'focus frame)
                    ace-window--test-events)))
                ((symbol-function
                  'select-window)
                 (lambda (window)
                   (push
                    (list 'select window)
                    ace-window--test-events)
                   'select-result)))
             (list
              fixture
              (condition-case error
                  (list
                   'ok
                   (aw-switch-to-window
                    fixture))
                (error
                 (list 'error error)))
              (nreverse
               ace-window--test-events))))
         '(current other dead))"##;
    let expect = expect![[
        r#"OK ((current (ok select-result) ((push start-window) (select current))) (other (ok select-result) ((push start-window) (focus other-frame) (select other))) (dead (error (error "Got a dead window dead")) ((push start-window))))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_flip_and_switch_buffer_in_window_compose_exact_helpers() {
    let elisp_form = r##"(mapcar
         (lambda (operation)
           (setq ace-window--test-events nil)
           (cl-letf
               (((symbol-function
                  'aw--pop-window)
                 (lambda ()
                   (push '(pop)
                         ace-window--test-events)
                   'previous-window))
                ((symbol-function
                  'aw-switch-to-window)
                 (lambda (window)
                   (push
                    (list 'switch window)
                    ace-window--test-events)
                   'switch-result))
                ((symbol-function
                  'aw--switch-buffer)
                 (lambda ()
                   (push '(switch-buffer)
                         ace-window--test-events)
                   'buffer-result)))
             (list
              operation
              (if (eq operation 'flip)
                  (aw-flip-window)
                (aw-switch-buffer-in-window
                 'target-window))
              (nreverse
               ace-window--test-events))))
         '(flip switch-buffer))"##;
    let expect = expect![
        "OK ((flip switch-result ((pop) (switch previous-window))) (switch-buffer buffer-result ((switch target-window) (switch-buffer))))"
    ];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_dispatch_help_formats_keys_deletes_backgrounds_and_reenters_without_minibuffer_message()
 {
    let elisp_form = r##"(let ((aw-dispatch-alist
              '((120 fixture-command
                     "Delete Window")
                (110 aw-flip-window)))
             (aw-overlays-back
              '(overlay-a overlay-b))
             (aw-minibuffer-flag t))
         (setq ace-window--test-events nil)
         (cl-letf
             (((symbol-function 'message)
               (lambda (format string)
                 (push
                  (list
                   'message
                   format
                   (substring-no-properties
                    string)
                   (get-text-property
                    0
                    'face
                    string)
                   (get-text-property
                    (1+
                     (string-match
                      "\n"
                      string))
                    'face
                    string))
                  ace-window--test-events)
                 string))
              ((symbol-function
                'delete-overlay)
               (lambda (overlay)
                 (push
                  (list 'delete overlay)
                  ace-window--test-events)))
              ((symbol-function 'ace-window)
               (lambda (argument)
                 (interactive "p")
                 (push
                  (list
                   'ace-window
                   argument
                   aw-minibuffer-flag)
                  ace-window--test-events)
                 'ace-result)))
           (list
            (aw-show-dispatch-help)
            (nreverse
             ace-window--test-events))))"##;
    let expect = expect![[
        r#"OK (ace-result ((message "%s" "x: Delete Window\nn: aw-flip-window" aw-key-face aw-key-face) (delete overlay-a) (delete overlay-b) (ace-window 1 nil)))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_delete_window_covers_single_frame_live_kill_and_dead_paths() {
    let elisp_form = r##"(mapcar
         (lambda (fixture)
           (setq
            ace-window--test-events
            nil
            ace-window--test-delete-frame-kind
            (car fixture)
            ace-window--test-window-count
            (nth 1 fixture)
            ace-window--test-window-live
            (nth 2 fixture))
           (cl-letf
               (((symbol-function
                  'window-frame)
                 (lambda (_window)
                   (if (eq
                        ace-window--test-delete-frame-kind
                        'other-frame)
                       'other-frame
                     'current-frame)))
                ((symbol-function
                  'selected-frame)
                 (lambda ()
                   'current-frame))
                ((symbol-function
                  'frame-live-p)
                 (lambda (_frame)
                   t))
                ((symbol-function
                  'select-frame-set-input-focus)
                 (lambda (frame)
                   (push
                    (list 'focus frame)
                    ace-window--test-events)))
                ((symbol-function
                  'window-list)
                 (lambda
                     (&optional _frame
                      _minibuffer _window)
                   (make-list
                    ace-window--test-window-count
                    'window)))
                ((symbol-function
                  'window-live-p)
                 (lambda (_window)
                   ace-window--test-window-live))
                ((symbol-function
                  'window-buffer)
                 (lambda (_window)
                   'fixture-buffer))
                ((symbol-function
                  'delete-frame)
                 (lambda (frame)
                   (push
                    (list 'delete-frame frame)
                    ace-window--test-events)
                   'frame-deleted))
                ((symbol-function
                  'delete-window)
                 (lambda (window)
                   (push
                    (list 'delete-window window)
                    ace-window--test-events)
                   'window-deleted))
                ((symbol-function
                  'kill-buffer)
                 (lambda (buffer)
                   (push
                    (list 'kill buffer)
                    ace-window--test-events)
                   t)))
             (list
              fixture
              (condition-case error
                  (list
                   'ok
                   (aw-delete-window
                    'target-window
                    (nth 3 fixture)))
                (error
                 (list 'error error)))
              (nreverse
               ace-window--test-events))))
         '((single 1 t nil)
           (other-frame 2 t t)
           (dead 2 nil t)))"##;
    let expect = expect![[
        r#"OK (((single 1 t nil) (ok frame-deleted) ((delete-frame current-frame))) ((other-frame 2 t t) (ok t) ((focus other-frame) (delete-window target-window) (kill fixture-buffer))) ((dead 2 nil t) (error (error "Got a dead window target-window")) nil))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_switch_buffer_prefers_ivy_then_ido_then_builtin_interactive_command() {
    let elisp_form = r##"(mapcar
         (lambda (branch)
           (setq ace-window--test-events nil)
           (cl-progv
               '(ivy-mode ido-mode)
               (list
                (eq branch 'ivy)
                (eq branch 'ido))
             (cl-letf
                 (((symbol-function
                    'ivy-switch-buffer)
                   (lambda ()
                     (push 'ivy
                           ace-window--test-events)
                     'ivy-result))
                  ((symbol-function
                    'ido-switch-buffer)
                   (lambda ()
                     (push 'ido
                           ace-window--test-events)
                     'ido-result))
                  ((symbol-function
                    'switch-to-buffer)
                   (lambda ()
                     (interactive)
                     (push 'builtin
                           ace-window--test-events)
                     'builtin-result)))
               (list
                branch
                (aw--switch-buffer)
                (nreverse
                 ace-window--test-events)))))
         '(ivy ido builtin))"##;
    let expect = expect![
        "OK ((ivy ivy-result (ivy)) (ido ido-result (ido)) (builtin builtin-result (builtin)))"
    ];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_swap_window_covers_same_dead_and_both_selection_inversion_paths() {
    let elisp_form = r##"(mapcar
         (lambda (fixture)
           (setq
            ace-window--test-events
            nil
            ace-window--test-selected
            'current
            ace-window--test-buffers
            (list
             (cons 'current 'buffer-a)
             (cons 'target 'buffer-b)))
           (let ((aw-swap-invert
                  (nth 1 fixture)))
             (cl-letf
                 (((symbol-function
                    'window-frame)
                   (lambda (_window)
                     'current-frame))
                  ((symbol-function
                    'selected-frame)
                   (lambda ()
                     'current-frame))
                  ((symbol-function
                    'selected-window)
                   (lambda ()
                     ace-window--test-selected))
                  ((symbol-function
                    'window-live-p)
                   (lambda (window)
                     (not
                      (eq window 'dead))))
                  ((symbol-function
                    'window-buffer)
                   (lambda (window)
                     (cdr
                      (assq
                       window
                       ace-window--test-buffers))))
                  ((symbol-function
                    'set-window-buffer)
                   (lambda (window buffer)
                     (setcdr
                      (assq
                       window
                       ace-window--test-buffers)
                      buffer)
                     (push
                      (list
                       'set
                       window
                       buffer)
                      ace-window--test-events)))
                  ((symbol-function
                    'select-window)
                   (lambda (window)
                     (setq
                      ace-window--test-selected
                      window)
                     (push
                      (list 'select window)
                      ace-window--test-events)))
                  ((symbol-function
                    'aw--push-window)
                   (lambda (window)
                     (push
                      (list 'push window)
                      ace-window--test-events))))
               (list
                fixture
                (aw-swap-window
                 (car fixture))
                ace-window--test-selected
                ace-window--test-buffers
                (nreverse
                 ace-window--test-events)))))
         '((current nil)
           (dead nil)
           (target nil)
           (target t)))"##;
    let expect = expect![
        "OK (((current nil) nil current ((current . buffer-a) (target . buffer-b)) nil) ((dead nil) nil current ((current . buffer-a) (target . buffer-b)) nil) ((target nil) #1=((select target)) target ((current . buffer-b) (target . buffer-a)) ((push current) (set current buffer-b) (set target buffer-a) . #1#)) ((target t) #2=((select current)) current ((current . buffer-b) (target . buffer-a)) ((push current) (set target buffer-a) (set current buffer-b) . #2#)))"
    ];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_move_and_copy_preserve_their_distinct_buffer_position_contracts() {
    let elisp_form = r##"(mapcar
         (lambda (operation)
           (setq ace-window--test-events nil)
           (cl-letf
               (((symbol-function
                  'current-buffer)
                 (lambda ()
                   'source-buffer))
                ((symbol-function
                  'other-buffer)
                 (lambda
                     (&optional _buffer
                      _visible-ok frame)
                   (push
                    (list
                     'other-buffer
                     frame)
                    ace-window--test-events)
                   'other-buffer))
                ((symbol-function
                  'switch-to-buffer)
                 (lambda (buffer)
                   (push
                    (list
                     'switch-buffer
                     buffer)
                    ace-window--test-events)))
                ((symbol-function
                  'aw-switch-to-window)
                 (lambda (window)
                   (push
                    (list
                     'switch-window
                     window)
                    ace-window--test-events)))
                ((symbol-function
                  'window-start)
                 (lambda
                     (&optional _window)
                   17))
                ((symbol-function 'point)
                 (lambda () 23))
                ((symbol-function
                  'frame-selected-window)
                 (lambda
                     (&optional _frame)
                   'selected-target))
                ((symbol-function
                  'set-window-start)
                 (lambda (window position)
                   (push
                    (list
                     'set-start
                     window
                     position)
                    ace-window--test-events)))
                ((symbol-function 'goto-char)
                 (lambda (position)
                   (push
                    (list 'goto position)
                    ace-window--test-events))))
             (list
              operation
              (funcall operation
                       'target-window)
              (nreverse
               ace-window--test-events))))
         '(aw-move-window
           aw-copy-window))"##;
    let expect = expect![
        "OK ((aw-move-window #1=((switch-buffer source-buffer)) ((other-buffer nil) (switch-buffer other-buffer) (switch-window target-window) . #1#)) (aw-copy-window #2=((goto 23)) ((switch-window target-window) (switch-buffer source-buffer) (set-start selected-target 17) . #2#)))"
    ];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_split_wrappers_and_fair_split_cover_both_aspect_ratio_branches() {
    let elisp_form = r##"(mapcar
         (lambda (fixture)
           (setq
            ace-window--test-events
            nil
            ace-window--test-width
            (nth 0 fixture)
            ace-window--test-height
            (nth 1 fixture))
           (let ((aw-fair-aspect-ratio
                  (nth 2 fixture)))
             (cl-letf
                 (((symbol-function
                    'select-window)
                   (lambda (window)
                     (push
                      (list 'select window)
                      ace-window--test-events)))
                  ((symbol-function
                    'split-window-vertically)
                   (lambda ()
                     (push '(split-vert)
                           ace-window--test-events)
                     'vertical-result))
                  ((symbol-function
                    'split-window-horizontally)
                   (lambda ()
                     (push '(split-horz)
                           ace-window--test-events)
                     'horizontal-result))
                  ((symbol-function
                    'window-body-width)
                 (lambda (_window)
                     ace-window--test-width))
                  ((symbol-function
                    'window-body-height)
                 (lambda (_window)
                     ace-window--test-height)))
               (list
                fixture
                (aw-split-window-fair
                 'target-window)
                (nreverse
                 ace-window--test-events)))))
         '((100 20 2)
           (40 30 2)
           (60 30 2)))"##;
    let expect = expect![
        "OK (((100 20 2) horizontal-result ((select target-window) (split-horz))) ((40 30 2) vertical-result ((select target-window) #1=(split-vert))) ((60 30 2) vertical-result ((select target-window) #1#)))"
    ];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_other_window_operations_always_flip_back_on_success_and_error() {
    let elisp_form = r##"(mapcar
         (lambda (fixture)
           (setq
            ace-window--test-events
            nil
            ace-window--test-branch
            (car fixture)
            ace-window--test-error
            (nth 1 fixture))
           (cl-letf
               (((symbol-function
                  'aw-switch-to-window)
                 (lambda (window)
                   (push
                    (list 'switch window)
                    ace-window--test-events)))
                ((symbol-function
                  'aw--switch-buffer)
                 (lambda ()
                   (push '(switch-buffer)
                         ace-window--test-events)
                   (if ace-window--test-error
                       (error
                        "switch failure")
                     'switch-result)))
                ((symbol-function
                  'read-key-sequence)
                 (lambda (prompt)
                   (push
                    (list 'read prompt)
                    ace-window--test-events)
                   "k"))
                ((symbol-function
                  'key-binding)
                 (lambda (key)
                   (push
                    (list 'binding key)
                    ace-window--test-events)
                   'ace-window--test-command))
                ((symbol-function
                  'ace-window--test-command)
                 (lambda ()
                   (push '(command)
                         ace-window--test-events)
                   (if ace-window--test-error
                       (error
                        "command failure")
                     'command-result)))
                ((symbol-function
                  'aw-flip-window)
                 (lambda ()
                   (push '(flip)
                         ace-window--test-events)
                   'flip-result)))
             (list
              fixture
              (condition-case error
                  (list
                   'ok
                   (funcall
                    (if (eq
                         ace-window--test-branch
                         'buffer)
                        #'aw-switch-buffer-other-window
                      #'aw-execute-command-other-window)
                    'target-window))
                (error
                 (list 'error error)))
              (nreverse
               ace-window--test-events))))
         '((buffer nil)
           (buffer t)
           (command nil)
           (command t)))"##;
    let expect = expect![[
        r#"OK (((buffer nil) (ok switch-result) ((switch target-window) #1=(switch-buffer) #2=(flip))) ((buffer t) (error (error "switch failure")) ((switch target-window) #1# #2#)) ((command nil) (ok command-result) ((switch target-window) (read "Enter key sequence: ") (binding "k") #3=(command) #2#)) ((command t) (error (error "command failure")) ((switch target-window) (read "Enter key sequence: ") (binding "k") #3# #2#)))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_face_relative_height_covers_unspecified_float_integer_and_error_values() {
    let elisp_form = r##"(mapcar
         (lambda (height)
           (setq ace-window--test-height
                 height)
           (cl-letf
               (((symbol-function
                  'face-attribute)
                 (lambda
                     (_face _attribute
                      &optional _frame
                      _inherit)
                   ace-window--test-height)))
             (list
              height
              (condition-case error
                  (list
                   'ok
                   (aw--face-rel-height))
                (error
                 (list 'error error))))))
         '(unspecified
           0.2 1.0 2.8
           120 bad))"##;
    let expect = expect![[
        r#"OK ((unspecified (ok 1)) (0.2 (ok 1)) (1.0 (ok 1)) (2.8 (ok 2)) (120 (ok 1)) (bad (error (error "unexpected: bad"))))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_offset_uses_face_height_line_lengths_and_horizontal_scroll() {
    let elisp_form = r##"(let ((buffer
              (generate-new-buffer
               " *ace-window-offset*")))
         (unwind-protect
             (progn
               (with-current-buffer buffer
                 (insert
                  "a\nbb\nccc\n"))
               (setq
                ace-window--test-buffer
                buffer)
               (mapcar
                (lambda (fixture)
                  (setq
                   ace-window--test-height
                   (car fixture)
                   ace-window--test-hscroll
                   (cadr fixture))
                  (cl-letf
                      (((symbol-function
                         'window-buffer)
                        (lambda (_window)
                          ace-window--test-buffer))
                       ((symbol-function
                         'window-hscroll)
                        (lambda (_window)
                          ace-window--test-hscroll))
                       ((symbol-function
                         'window-start)
                        (lambda (_window)
                          1))
                       ((symbol-function
                         'window-end)
                        (lambda (_window)
                          10))
                       ((symbol-function
                         'aw--face-rel-height)
                        (lambda ()
                          ace-window--test-height)))
                    (list
                     fixture
                     (aw-offset
                      'fixture-window))))
                '((1 0)
                  (1 1)
                  (1 2)
                  (2 0)
                  (2 3))))
           (when (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect = expect!["OK (((1 0) 1) ((1 1) 2) ((1 2) 5) ((2 0) 3) ((2 3) 9))"];
    assert_ace_window_parity(elisp_form, expect);
}

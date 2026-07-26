use super::assert_ace_window_parity;
use expect_test::expect;

#[test]
fn ace_window_overlay_string_covers_character_path_tab_newline_wide_and_invalid_styles() {
    let elisp_form = r##"(save-window-excursion
         (let ((buffer
                (generate-new-buffer
                 " *ace-window-overlay-string*")))
           (unwind-protect
               (progn
                 (set-window-buffer
                  (selected-window)
                  buffer)
                 (with-current-buffer buffer
                   (insert "A\t\n界x"))
                 (mapcar
                  (lambda (fixture)
                    (let ((aw-leading-char-style
                           (nth 0 fixture)))
                      (condition-case error
                          (list
                           'ok
                           (aw--overlay-str
                            (selected-window)
                            (nth 1 fixture)
                            (nth 2 fixture)))
                        (error
                         (list 'error error)))))
                  '((char 1 (97 98))
                    (path 1 (97 98))
                    (char 2 (49))
                    (char 3 (50))
                    (char 4 (51))
                    (char 99 (52))
                    (invalid 1 (53)))))
             (when (buffer-live-p buffer)
               (kill-buffer buffer)))))"##;
    let expect = expect![[
        r#"OK ((ok "b") (ok "ba") (ok "1       ") (ok "2\n") (ok "3 ") (ok "4") (error (error "Bad ‘aw-leading-char-style’: invalid")))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_point_visible_predicate_respects_horizontal_scroll_and_width_bounds() {
    let elisp_form = r##"(mapcar
         (lambda (fixture)
           (setq
            ace-window--test-column
            (nth 0 fixture)
            ace-window--test-hscroll
            (nth 1 fixture)
            ace-window--test-width
            (nth 2 fixture))
           (cl-letf
               (((symbol-function
                  'current-column)
                 (lambda ()
                   ace-window--test-column))
                ((symbol-function
                  'window-hscroll)
                 (lambda
                     (&optional _window)
                   ace-window--test-hscroll))
                ((symbol-function
                  'window-width)
                 (lambda
                     (&optional _window)
                   ace-window--test-width)))
             (list
              fixture
              (and
               (aw--point-visible-p)
               t))))
         '((0 0 10)
           (5 3 3)
           (6 3 3)
           (2 3 10)
           (12 3 10)))"##;
    let expect =
        expect!["OK (((0 0 10) t) ((5 3 3) t) ((6 3 3) nil) ((2 3 10) nil) ((12 3 10) t))"];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_background_overlay_creation_is_configurable_and_uses_exact_ranges() {
    let elisp_form = r##"(mapcar
         (lambda (enabled)
           (let ((aw-background enabled)
                 (aw-overlays-back
                  'prebound))
             (setq ace-window--test-events
                   nil)
             (cl-letf
                 (((symbol-function
                    'window-start)
                   (lambda (window)
                     (if (eq window 'w1)
                         1
                       11)))
                  ((symbol-function
                    'window-end)
                   (lambda (window)
                     (if (eq window 'w1)
                         10
                       20)))
                  ((symbol-function
                    'window-buffer)
                   (lambda (window)
                     (if (eq window 'w1)
                         'b1
                       'b2)))
                  ((symbol-function
                    'make-overlay)
                   (lambda (start end buffer)
                     (let ((overlay
                            (intern
                             (format
                              "ol-%s"
                              buffer))))
                       (push
                        (list
                         'make
                         overlay
                         start
                         end
                         buffer)
                        ace-window--test-events)
                       overlay)))
                  ((symbol-function
                    'overlay-put)
                   (lambda
                       (overlay property value)
                     (push
                      (list
                       'put
                       overlay
                       property
                       value)
                      ace-window--test-events))))
               (list
                enabled
                (aw--make-backgrounds
                 '(w1 w2))
                aw-overlays-back
                (nreverse
                 ace-window--test-events)))))
         '(nil t))"##;
    let expect = expect![
        "OK ((nil nil prebound nil) (t #1=(ol-b1 ol-b2) #1# ((make ol-b1 1 10 b1) (put ol-b1 face aw-background-face) (make ol-b2 11 20 b2) (put ol-b2 face aw-background-face))))"
    ];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_mode_line_updates_optional_minibuffer_message_and_forces_refresh() {
    let elisp_form = r##"(mapcar
         (lambda (fixture)
           (let ((aw-minibuffer-flag
                  (car fixture))
                 (ace-window-mode
                  'prebound))
             (setq ace-window--test-events
                   nil)
             (cl-letf
                 (((symbol-function 'message)
                   (lambda
                       (format &rest arguments)
                     (push
                      (cons
                       'message
                       (cons
                        format
                        arguments))
                      ace-window--test-events)))
                  ((symbol-function
                    'force-mode-line-update)
                   (lambda (&rest arguments)
                     (push
                      (cons
                       'force
                       arguments)
                      ace-window--test-events))))
               (list
                fixture
                (aw-set-mode-line
                 (cdr fixture))
                ace-window-mode
                (nreverse
                 ace-window--test-events)))))
         '((nil . " Ace - Window")
           (t . " Ace - Window")
           (t)))"##;
    let expect = expect![[
        r#"OK (((nil . " Ace - Window") #1=((force)) " Ace - Window" #1#) ((t . " Ace - Window") #2=((force)) " Ace - Window" ((message "%s" "Ace - Window") . #2#)) ((t) #3=((force)) nil #3#))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_horizontal_scroll_restoration_skips_dead_windows_and_clears_state() {
    let elisp_form = r##"(let ((aw--windows-hscroll
              '((live-1 . 3)
                (dead . 7)
                (live-2 . 0))))
         (setq ace-window--test-events nil)
         (cl-letf
             (((symbol-function
                'window-live-p)
               (lambda (window)
                 (not (eq window 'dead))))
              ((symbol-function
                'set-window-hscroll)
               (lambda (window value)
                 (push
                  (list window value)
                  ace-window--test-events))))
           (list
            (aw--restore-windows-hscroll)
            aw--windows-hscroll
            (nreverse
             ace-window--test-events))))"##;
    let expect = expect!["OK (nil nil ((live-1 3) (live-2 0)))"];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_done_cleans_mode_overlays_temporary_buffers_and_state_in_order() {
    let elisp_form = r##"(let ((empty-buffer
              (generate-new-buffer
               " *ace-window-empty*"))
             (nonempty-buffer
              (generate-new-buffer
               " *ace-window-nonempty*")))
         (unwind-protect
             (progn
               (with-current-buffer empty-buffer
                 (insert " ")
                 (setq buffer-read-only t))
               (with-current-buffer
                   nonempty-buffer
                 (insert "x")
                 (setq buffer-read-only t))
               (let ((aw-overlays-back
                      '(background-1
                        background-2))
                     (aw-empty-buffers-list
                      (list
                       empty-buffer
                       nonempty-buffer))
                     (aw--windows-points nil))
                 (setq
                  ace-window--test-events
                  nil)
                 (cl-letf
                     (((symbol-function
                        'aw-set-mode-line)
                       (lambda (value)
                         (push
                          (list
                           'mode-line
                           value)
                          ace-window--test-events)))
                      ((symbol-function
                        'delete-overlay)
                       (lambda (overlay)
                         (push
                          (list
                           'delete
                           overlay)
                          ace-window--test-events)))
                      ((symbol-function
                        'avy--remove-leading-chars)
                       (lambda ()
                         (push
                          '(remove-leading)
                          ace-window--test-events)))
                      ((symbol-function
                        'aw--restore-windows-hscroll)
                       (lambda ()
                         (push
                          '(restore-hscroll)
                          ace-window--test-events))))
                   (list
                    (aw--done)
                    aw-overlays-back
                    aw-empty-buffers-list
                    (with-current-buffer
                        empty-buffer
                      (buffer-string))
                    (with-current-buffer
                        nonempty-buffer
                      (buffer-string))
                    (nreverse
                     ace-window--test-events)))))
           (dolist
               (buffer
                (list
                 empty-buffer
                 nonempty-buffer))
             (when (buffer-live-p buffer)
               (kill-buffer buffer)))))"##;
    let expect = expect![[
        r#"OK (nil nil nil "" "x" ((mode-line nil) (delete background-1) (delete background-2) (remove-leading) (restore-hscroll)))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_done_restores_saved_points_in_live_windows() {
    let elisp_form = r##"(save-window-excursion
         (let ((buffer
                (generate-new-buffer
                 " *ace-window-points*")))
           (unwind-protect
               (progn
                 (set-window-buffer
                  (selected-window)
                  buffer)
                 (with-current-buffer buffer
                   (insert "abcdef")
                   (goto-char 6))
                 (let ((aw-overlays-back nil)
                       (aw-empty-buffers-list
                        nil)
                       (aw--windows-points
                        (list
                         (cons
                          (selected-window)
                          3))))
                   (cl-letf
                       (((symbol-function
                          'aw-set-mode-line)
                         (lambda (_value)))
                        ((symbol-function
                          'avy--remove-leading-chars)
                         (lambda ()))
                        ((symbol-function
                          'aw--restore-windows-hscroll)
                         (lambda ())))
                     (list
                      (aw--done)
                      (window-point
                       (selected-window))
                      aw--windows-points))))
             (when (buffer-live-p buffer)
               (kill-buffer buffer)))))"##;
    let expect = expect!["OK (nil 3 nil)"];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_real_lead_overlay_handles_empty_buffer_and_records_overlay_properties() {
    let elisp_form = r##"(save-window-excursion
         (let ((buffer
                (generate-new-buffer
                 " *ace-window-real-overlay*")))
           (unwind-protect
               (progn
                 (set-window-buffer
                  (selected-window)
                  buffer)
                 (set-window-start
                  (selected-window)
                  1)
                 (let ((aw-empty-buffers-list
                        nil)
                       (aw--windows-hscroll
                        nil)
                       (aw--windows-points
                        nil)
                       (avy--overlays-lead
                        nil)
                       (aw-char-position
                        'top-left))
                   (cl-letf
                       (((symbol-function
                          'aw--face-rel-height)
                         (lambda () 1)))
                     (aw--lead-overlay
                      '(49)
                      (cons
                       1
                       (selected-window)))
                     (let ((overlay
                            (car
                             avy--overlays-lead)))
                       (prog1
                           (list
                            (with-current-buffer
                                buffer
                              (buffer-string))
                            (length
                             aw-empty-buffers-list)
                            (overlay-start
                             overlay)
                            (overlay-end
                             overlay)
                            (overlay-get
                             overlay
                             'display)
                            (overlay-get
                             overlay
                             'face)
                            (eq
                             (overlay-get
                              overlay
                              'window)
                             (selected-window))
                            aw--windows-hscroll
                            aw--windows-points)
                         (delete-overlay
                          overlay))))))
             (when (buffer-live-p buffer)
               (kill-buffer buffer)))))"##;
    let expect = expect![[r#"OK (" " 1 1 2 "1" aw-leading-char-face t nil nil)"#]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_real_lead_overlay_covers_left_hscroll_scaled_face_and_minibuffer_branches() {
    let elisp_form = r##"(save-window-excursion
         (let ((buffer
                (generate-new-buffer
                 " *ace-window-branch-overlay*"))
               (window
                (selected-window)))
           (unwind-protect
               (progn
                 (set-window-buffer window buffer)
                 (with-current-buffer buffer
                   (insert
                    "alpha\nbeta\ngamma\n"))
                 (set-window-point window 7)
                 (setq
                  ace-window--test-events
                  nil
                  ace-window--test-hscroll
                  2)
                 (let ((aw-empty-buffers-list
                        nil)
                       (aw--windows-hscroll
                        nil)
                       (aw--windows-points
                        nil)
                       (avy--overlays-lead
                        nil)
                       (aw-char-position
                        'left))
                   (cl-letf
                       (((symbol-function
                          'window-hscroll)
                         (lambda
                             (&optional _window)
                           ace-window--test-hscroll))
                        ((symbol-function
                          'window-width)
                         (lambda
                             (&optional _window)
                           80))
                        ((symbol-function
                          'scroll-right)
                         (lambda
                             (&optional _count)
                           (setq
                            ace-window--test-hscroll
                            (max
                             0
                             (1-
                              ace-window--test-hscroll)))
                           (push
                            (list
                             'scroll-right
                             ace-window--test-hscroll)
                            ace-window--test-events)
                           ace-window--test-hscroll))
                        ((symbol-function
                          'move-to-window-line)
                         (lambda (line)
                           (push
                            (list
                             'move-to-window-line
                             line)
                            ace-window--test-events)
                           (goto-char 12)
                           0))
                        ((symbol-function
                          'move-to-column)
                         (lambda
                             (column
                              &optional _force)
                           (push
                            (list
                             'move-to-column
                             column)
                            ace-window--test-events)
                           column))
                        ((symbol-function
                          'recenter)
                         (lambda
                             (&optional position)
                           (push
                            (list
                             'recenter
                             position)
                            ace-window--test-events)))
                        ((symbol-function
                          'aw--face-rel-height)
                         (lambda () 2))
                        ((symbol-function
                          'aw--overlay-str)
                         (lambda
                             (overlay-window
                              position path)
                           (push
                            (list
                             'overlay-string
                             (eq
                              overlay-window
                              window)
                             position
                             path)
                            ace-window--test-events)
                           "fixture-display"))
                        ((symbol-function
                          'window-minibuffer-p)
                         (lambda
                             (overlay-window)
                           (eq
                            overlay-window
                            window))))
                     (aw--lead-overlay
                      '(97 98)
                      (cons 7 window))
                     (let ((overlay
                            (car
                             avy--overlays-lead)))
                       (prog1
                           (list
                            ace-window--test-hscroll
                            (mapcar
                             #'cdr
                             aw--windows-hscroll)
                            (and
                             aw--windows-points
                             (eq
                              (caar
                               aw--windows-points)
                              window))
                            (and
                             aw--windows-points
                             (cdar
                              aw--windows-points))
                            (window-point window)
                            (overlay-start
                             overlay)
                            (overlay-end
                             overlay)
                            (overlay-get
                             overlay
                             'display)
                            (overlay-get
                             overlay
                             'face)
                            (eq
                             (overlay-get
                              overlay
                              'window)
                             window)
                            (nreverse
                             ace-window--test-events))
                         (delete-overlay
                          overlay))))))
             (when (buffer-live-p buffer)
               (kill-buffer buffer)))))"##;
    let expect = expect![[
        r#"OK (0 (1 2) t 7 13 12 13 "fixture-display" aw-minibuffer-leading-char-face t ((scroll-right 1) (scroll-right 0) (move-to-window-line -1) (move-to-column 0) (recenter -1) (overlay-string t 12 (97 98))))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_remove_leading_chars_wrapper_forwards_once() {
    let elisp_form = r##"(progn
         (setq ace-window--test-events nil)
         (cl-letf
             (((symbol-function
                'avy--remove-leading-chars)
               (lambda ()
                 (push 'remove
                       ace-window--test-events)
                 'removed)))
           (list
            (aw--remove-leading-chars)
            (nreverse
             ace-window--test-events))))"##;
    let expect = expect!["OK (removed (remove))"];
    assert_ace_window_parity(elisp_form, expect);
}

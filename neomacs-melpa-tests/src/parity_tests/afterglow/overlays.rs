use expect_test::expect;

use super::assert_afterglow_parity;

#[test]
fn afterglow_current_line_empty_distinguishes_content_whitespace_and_blank_lines() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "alpha\n   \n\nomega")
         (mapcar
          (lambda (position)
            (goto-char
             position)
            (let ((before
                   (point))
                  (empty
                   (afterglow--current-line-empty-p)))
              (list
               position
               empty
               before
               (point)
               (line-beginning-position)
               (line-end-position))))
          '(2 7 11 13)))"##;
    let expect =
        expect!["OK ((2 nil 2 2 1 6) (7 t 7 7 7 10) (11 t 11 11 11 11) (13 nil 13 13 12 17))"];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_apply_overlay_uses_custom_bounds_face_priority_and_timer_arguments() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "0123456789")
         (let ((afterglow--temp-overlay nil)
               timer-call)
           (cl-letf
               (((symbol-function
                  'run-with-timer)
                 (lambda (duration repeat callback &rest arguments)
                   (setq timer-call
                         (list
                          duration
                          repeat
                          (functionp callback)
                          arguments))
                   'fixture-timer)))
             (let ((result
                    (afterglow--apply-overlay
                     (list
                      :thing
                      (lambda ()
                        (cons
                         3
                         8))
                      :duration
                      2.5
                      :face
                      'highlight))))
               (list
                result
                (overlayp
                 afterglow--temp-overlay)
                (eq
                 (overlay-buffer afterglow--temp-overlay)
                 (current-buffer))
                (overlay-start
                 afterglow--temp-overlay)
                (overlay-end
                 afterglow--temp-overlay)
                (buffer-substring
                 (overlay-start afterglow--temp-overlay)
                 (overlay-end afterglow--temp-overlay))
                (overlay-properties
                 afterglow--temp-overlay)
                (overlay-get
                 afterglow--temp-overlay
                 'afterglow)
                timer-call)))))"##;
    let expect = expect![[
        r#"OK (fixture-timer t t 3 8 "23456" (priority 100 face highlight) nil (2.5 nil t nil))"#
    ]];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_apply_overlay_deletes_previous_overlay_when_custom_bounds_are_absent() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "content")
         (let* ((previous
                 (make-overlay
                  2
                  5))
                (afterglow--temp-overlay
                 previous)
                timer-call)
           (cl-letf
               (((symbol-function
                  'run-with-timer)
                 (lambda (&rest arguments)
                   (setq timer-call
                         arguments)
                   'fixture-timer)))
             (let ((result
                    (afterglow--apply-overlay
                     (list
                      :thing
                      (lambda ()
                        nil)))))
               (list
                result
                (eq
                 previous
                 afterglow--temp-overlay)
                (overlay-buffer
                 previous)
                (overlay-start
                 previous)
                (overlay-end
                 previous)
                (overlays-in
                 (point-min)
                 (point-max))
                timer-call)))))"##;
    let expect = expect!["OK (nil t nil nil nil nil nil)"];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_apply_overlay_tracks_active_region_and_skips_inactive_region() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "alpha beta")
         (goto-char
          2)
         (set-mark
          6)
         (activate-mark)
         (let ((transient-mark-mode
                t)
               (afterglow--temp-overlay nil)
               timer-calls)
           (cl-letf
               (((symbol-function
                  'run-with-timer)
                 (lambda (duration repeat callback &rest arguments)
                   (push
                    (list
                     duration
                     repeat
                     (functionp callback)
                     arguments)
                    timer-calls)
                   'fixture-timer)))
             (afterglow--apply-overlay
              '(:thing region
                :duration 6
                :face highlight))
             (let ((active-state
                    (list
                     (region-beginning)
                     (region-end)
                     (overlay-start
                      afterglow--temp-overlay)
                     (overlay-end
                      afterglow--temp-overlay)
                     (buffer-substring
                      (overlay-start afterglow--temp-overlay)
                      (overlay-end afterglow--temp-overlay))
                     (overlay-get
                      afterglow--temp-overlay
                      'face)))
                   (active-overlay
                    afterglow--temp-overlay))
               (deactivate-mark)
               (let ((inactive-result
                      (afterglow--apply-overlay
                       '(:thing region))))
                 (list
                  active-state
                  inactive-result
                  (eq
                   active-overlay
                   afterglow--temp-overlay)
                  (overlay-buffer
                   active-overlay)
                  (overlays-in
                   (point-min)
                   (point-max))
                  (nreverse timer-calls)))))))"##;
    let expect = expect![[
        r#"OK ((2 6 2 6 "lpha" highlight) fixture-timer nil nil (#<overlay in no buffer>) ((6 nil t nil) (1 nil t nil)))"#
    ]];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_line_overlays_cover_default_bounded_oversized_and_zero_width_geometry() {
    let elisp_form = r##"(let ((afterglow-default-duration
                21)
               (afterglow-default-face
                'underline)
               (afterglow--temp-overlay nil)
               timer-calls)
         (cl-letf
             (((symbol-function
                'run-with-timer)
               (lambda (duration repeat callback &rest arguments)
                 (push
                  (list
                   duration
                   repeat
                   (functionp callback)
                   arguments)
                  timer-calls)
                 'fixture-timer)))
           (let ((cases
                  (mapcar
                   (lambda (properties)
                     (with-temp-buffer
                       (insert
                        "prefix\nabcdefgh\nsuffix")
                       (goto-char
                        10)
                       (let ((result
                              (afterglow--apply-overlay
                               properties)))
                         (list
                          properties
                          result
                          (overlay-start
                           afterglow--temp-overlay)
                          (overlay-end
                           afterglow--temp-overlay)
                          (buffer-substring
                           (overlay-start afterglow--temp-overlay)
                           (overlay-end afterglow--temp-overlay))
                          (overlay-get
                           afterglow--temp-overlay
                           'face)
                          (overlay-get
                           afterglow--temp-overlay
                           'priority)))))
                   '((:thing line)
                     (:thing line :width 3 :duration 4 :face highlight)
                     (:thing line :width 99)
                     (:thing line :width 0)))))
             (list
              cases
              (nreverse timer-calls)))))"##;
    let expect = expect![[
        r#"OK ((((:thing line) fixture-timer 8 16 "abcdefgh" underline 100) ((:thing line :width 3 :duration 4 :face highlight) fixture-timer 8 11 "abc" highlight 100) ((:thing line :width 99) fixture-timer 8 16 "abcdefgh" underline 100) ((:thing line :width 0) fixture-timer 8 8 "" underline 100)) ((21 nil t nil) (4 nil t nil) (21 nil t nil) (21 nil t nil)))"#
    ]];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_empty_line_removes_the_previous_overlay_without_scheduling_a_timer() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "alpha\n   \nomega")
         (let* ((previous
                 (make-overlay
                  1
                  6))
                (afterglow--temp-overlay
                 previous)
                timer-calls)
           (goto-char
            8)
           (cl-letf
               (((symbol-function
                  'run-with-timer)
                 (lambda (&rest arguments)
                   (push
                    arguments
                    timer-calls)
                   'fixture-timer)))
             (let ((result
                    (afterglow--apply-overlay
                     '(:thing line
                       :duration 5))))
               (list
                result
                (afterglow--current-line-empty-p)
                (eq
                 previous
                 afterglow--temp-overlay)
                (overlay-buffer
                 previous)
                (overlays-in
                 (point-min)
                 (point-max))
                timer-calls)))))"##;
    let expect = expect!["OK (nil t t nil nil nil)"];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_window_overlay_uses_the_visible_window_extent_and_current_buffer() {
    let elisp_form = r##"(let ((buffer
              (generate-new-buffer
               " *afterglow-window*"))
             (afterglow--temp-overlay nil)
             timer-call)
         (unwind-protect
             (save-window-excursion
               (set-window-buffer
                (selected-window)
                buffer)
               (with-current-buffer buffer
                 (insert
                  "one\ntwo\nthree\n")
                 (goto-char
                  6)
                 (cl-letf
                     (((symbol-function
                        'run-with-timer)
                       (lambda (duration repeat callback &rest arguments)
                         (setq timer-call
                               (list
                                duration
                                repeat
                                (functionp callback)
                                arguments))
                         'fixture-timer)))
                   (let ((result
                          (afterglow--apply-overlay
                           '(:thing window
                             :duration 8
                             :face highlight))))
                     (list
                      result
                      (window-start
                       (selected-window))
                      (window-end
                       (selected-window)
                       t)
                      (overlay-start
                       afterglow--temp-overlay)
                      (overlay-end
                       afterglow--temp-overlay)
                      (eq
                       (overlay-buffer afterglow--temp-overlay)
                       buffer)
                      (buffer-substring
                       (overlay-start afterglow--temp-overlay)
                       (overlay-end afterglow--temp-overlay))
                      (overlay-properties
                       afterglow--temp-overlay)
                      timer-call)))))
           (kill-buffer
            buffer)))"##;
    let expect = expect![[
        r#"OK (fixture-timer 1 15 1 15 t "one\ntwo\nthree\n" (priority 100 face highlight) (8 nil t nil))"#
    ]];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_default_thing_bounds_highlight_word_then_leave_deleted_overlay_when_absent() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "alpha beta\n   ")
         (goto-char
          8)
         (let ((afterglow-default-duration
                13)
               (afterglow-default-face
                'underline)
               (afterglow--temp-overlay nil)
               timer-calls)
           (cl-letf
               (((symbol-function
                  'run-with-timer)
                 (lambda (duration repeat callback &rest arguments)
                   (push
                    (list
                     duration
                     repeat
                     (functionp callback)
                     arguments)
                    timer-calls)
                   'fixture-timer)))
             (afterglow--apply-overlay
              '(:thing word))
             (let ((word-overlay
                    afterglow--temp-overlay)
                   (word-state
                    (list
                     (overlay-start
                      afterglow--temp-overlay)
                     (overlay-end
                      afterglow--temp-overlay)
                     (buffer-substring
                      (overlay-start afterglow--temp-overlay)
                      (overlay-end afterglow--temp-overlay))
                     (overlay-get
                      afterglow--temp-overlay
                      'face)
                     (overlay-get
                      afterglow--temp-overlay
                      'priority))))
               (goto-char
                (point-max))
               (let ((absent-result
                      (afterglow--apply-overlay
                       '(:thing word))))
                 (list
                  word-state
                  absent-result
                  (eq
                   word-overlay
                   afterglow--temp-overlay)
                  (overlay-buffer
                   word-overlay)
                  (overlays-in
                   (point-min)
                   (point-max))
                  (nreverse timer-calls)))))))"##;
    let expect = expect![[r#"OK ((7 11 "beta" underline 100) nil t nil nil ((13 nil t nil)))"#]];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_remove_overlays_deletes_only_tagged_overlays_not_its_own_untagged_overlay() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "alpha beta")
         (goto-char
          2)
         (let ((afterglow--temp-overlay nil))
           (cl-letf
               (((symbol-function
                  'run-with-timer)
                 (lambda (&rest _arguments)
                   'fixture-timer)))
             (afterglow--apply-overlay
              '(:thing word
                :duration 90))
             (let ((package-overlay
                    afterglow--temp-overlay)
                   (tagged
                    (make-overlay
                     7
                     9))
                   (unrelated
                    (make-overlay
                     10
                     11)))
               (overlay-put
                tagged
                'afterglow
                t)
               (overlay-put
                unrelated
                'fixture
                t)
               (let ((result
                      (afterglow--remove-overlays)))
                 (list
                  result
                  (overlay-get
                   package-overlay
                   'afterglow)
                  (overlay-buffer
                   package-overlay)
                  (overlay-start
                   package-overlay)
                  (overlay-end
                   package-overlay)
                  (overlay-buffer
                   tagged)
                  (overlay-buffer
                   unrelated)
                  (mapcar
                   (lambda (overlay)
                     (list
                      (overlay-start overlay)
                      (overlay-end overlay)
                      (overlay-properties overlay)))
                   (sort
                    (overlays-in
                     (point-min)
                     (point-max))
                    (lambda (left right)
                      (<
                       (overlay-start left)
                       (overlay-start right)))))))))))"##;
    let expect = expect![
        "OK (nil nil (:buffer nil) 1 6 nil (:buffer nil) ((1 6 (priority 100 face hl-line)) (10 11 (fixture t))))"
    ];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_replacing_overlay_across_buffers_detaches_the_old_overlay() {
    let elisp_form = r##"(let ((first-buffer
              (generate-new-buffer
               " *afterglow-first*"))
             (second-buffer
              (generate-new-buffer
               " *afterglow-second*"))
             (afterglow--temp-overlay nil)
             timer-calls)
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'run-with-timer)
                   (lambda (duration repeat callback &rest arguments)
                     (push
                      (list
                       duration
                       repeat
                       (functionp callback)
                       arguments)
                      timer-calls)
                     'fixture-timer)))
               (with-current-buffer first-buffer
                 (insert
                  "first")
                 (afterglow--apply-overlay
                  (list
                   :thing
                   (lambda ()
                     (cons
                      2
                      5))
                   :duration
                   1)))
               (let ((first-overlay
                      afterglow--temp-overlay))
                 (with-current-buffer second-buffer
                   (insert
                    "second")
                   (afterglow--apply-overlay
                    (list
                     :thing
                     (lambda ()
                       (cons
                        1
                        7))
                     :duration
                     2
                     :face
                     'highlight)))
                 (list
                  (overlay-buffer
                   first-overlay)
                  (overlay-start
                   first-overlay)
                  (overlay-end
                   first-overlay)
                  (eq
                   (overlay-buffer afterglow--temp-overlay)
                   second-buffer)
                  (overlay-start
                   afterglow--temp-overlay)
                  (overlay-end
                   afterglow--temp-overlay)
                  (with-current-buffer second-buffer
                    (buffer-substring
                     (overlay-start afterglow--temp-overlay)
                     (overlay-end afterglow--temp-overlay)))
                  (overlay-properties
                   afterglow--temp-overlay)
                  (nreverse timer-calls))))
           (kill-buffer
            first-buffer)
           (kill-buffer
            second-buffer)))"##;
    let expect = expect![[
        r#"OK (nil nil nil t 1 7 "second" (priority 100 face highlight) ((1 nil t nil) (2 nil t nil)))"#
    ]];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_zero_delay_timer_is_scheduled_then_detaches_the_overlay() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "alpha")
         (goto-char
          2)
         (let ((afterglow--temp-overlay nil)
               (real-run-with-timer
                (symbol-function
                 'run-with-timer))
               timer)
           (cl-letf
               (((symbol-function
                  'run-with-timer)
                 (lambda (duration repeat callback &rest arguments)
                   (setq timer
                         (apply
                          real-run-with-timer
                          duration
                          repeat
                          callback
                          arguments)))))
             (let ((result
                    (afterglow--apply-overlay
                     '(:thing word
                       :duration 0
                       :face highlight))))
               (let ((before
                      (list
                       (eq
                        result
                        timer)
                       (timerp timer)
                       (and
                        (memq
                         timer
                         timer-list)
                        t)
                       (overlay-start
                        afterglow--temp-overlay)
                       (overlay-end
                        afterglow--temp-overlay)
                       (overlay-buffer
                        afterglow--temp-overlay))))
                 (sleep-for
                  0.05)
                 (list
                  before
                  (timerp timer)
                  (and
                   (memq
                    timer
                    timer-list)
                   t)
                  (overlay-buffer
                   afterglow--temp-overlay)
                  (overlay-start
                   afterglow--temp-overlay)
                  (overlay-end
                   afterglow--temp-overlay)
                  (overlays-in
                   (point-min)
                   (point-max))))))))"##;
    let expect = expect!["OK ((t t t 1 6 (:buffer nil)) t nil nil nil nil nil)"];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_earlier_timer_callback_deletes_the_newest_global_overlay() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "alpha beta")
         (let ((afterglow--temp-overlay nil)
               (real-run-with-timer
                (symbol-function
                 'run-with-timer))
               timers)
           (cl-letf
               (((symbol-function
                  'run-with-timer)
                 (lambda (duration repeat callback &rest arguments)
                   (let ((timer
                          (apply
                           real-run-with-timer
                           duration
                           repeat
                           callback
                           arguments)))
                     (push
                      timer
                      timers)
                     timer))))
             (goto-char
              2)
             (afterglow--apply-overlay
              '(:thing word
                :duration 0.01))
             (let ((first-overlay
                    afterglow--temp-overlay))
               (goto-char
                8)
               (afterglow--apply-overlay
                '(:thing word
                  :duration 60))
               (let* ((second-overlay
                       afterglow--temp-overlay)
                      (before
                       (list
                        (overlay-buffer
                         first-overlay)
                        (overlay-start
                         second-overlay)
                        (overlay-end
                         second-overlay)
                        (buffer-substring
                         (overlay-start second-overlay)
                         (overlay-end second-overlay))
                        (mapcar
                         (lambda (timer)
                           (list
                           (timerp timer)
                           (and
                            (memq
                             timer
                             timer-list)
                            t)))
                         (reverse timers)))))
                 (sleep-for
                  0.05)
                 (let ((after
                        (list
                         (overlay-buffer
                          second-overlay)
                         (overlay-start
                          second-overlay)
                         (overlay-end
                          second-overlay)
                         (overlays-in
                          (point-min)
                          (point-max))
                         (mapcar
                          (lambda (timer)
                            (list
                             (timerp timer)
                             (and
                              (memq
                               timer
                               timer-list)
                              t)))
                          (reverse timers)))))
                   (mapc
                    #'cancel-timer
                    timers)
                   (list
                    before
                    after)))))))"##;
    let expect =
        expect![[r#"OK ((nil 7 11 "beta" ((t t) (t t))) (nil nil nil nil ((t nil) (t t))))"#]];
    assert_afterglow_parity(elisp_form, expect);
}

use expect_test::expect;

use super::assert_amread_mode_parity;

/// The reading session itself.  Enabling the mode asks for the voice language,
/// installs exactly one repeating timer at the configured word rate, and each
/// firing moves the highlight on to the next word.  Pins the prompt, the timer
/// count and interval, and the overlay's bounds, covered text and face at each
/// of the first five steps.
#[test]
fn enabling_the_mode_installs_one_timer_and_walks_the_highlight_word_by_word() {
    let elisp_form = r##"(amr-test-in-buffer
 (let ((baseline (amr-test-timer-baseline))
       (amread-scroll-style 'word)
       (amread-word-speed 3.0)
       (amread-voice-reader-enabled nil))
   (amr-test-with-answers '("english")
     (amread-mode 1)
     (let ((new (amr-test-new-timers baseline)) steps)
       (dotimes (_ 5)
         (dolist (timer new) (timer-event-handler timer))
         (push (amr-test-overlay) steps))
       (list :prompts (reverse amr-test-prompts)
             :mode (and amread-mode t)
             :new-timer-count (length new)
             :repeat-hundredths (round (* 100 (timer--repeat-delay (car new))))
             :steps (reverse steps)
             :point (point))))))"##;

    let expect = expect![[
        r#"OK (:prompts (("[amread] Select language: " "english")) :mode t :new-timer-count 1 :repeat-hundredths 33 :steps ((:start 1 :end 4 :text "Der" :face amread-highlight-face) (:start 5 :end 8 :text "Weg" :face amread-highlight-face) (:start 9 :end 12 :text "ist" :face amread-highlight-face) (:start 13 :end 16 :text "das" :face amread-highlight-face) (:start 17 :end 22 :text "Ziel." :face amread-highlight-face)) :point 23)"#
    ]];

    assert_amread_mode_parity(elisp_form, expect);
}

/// The two customizations that set the pace.  `amread-word-speed` is words per
/// second, so it appears as the reciprocal in the timer's repeat delay -- 3.0
/// gives 0.33s and 1.5 gives 0.67s -- and the line style uses
/// `amread-line-speed` directly.
///
/// Only the repeat interval is asserted.  The initial delay is a literal in the
/// source (0 and 1 second), and measuring it here means timing
/// `timer--time` against a baseline taken before `amread-mode` returns, which
/// absorbs however long enabling the mode took -- it read 0 tenths in a probe
/// and 17 under the harness on the same code.  That is wall-clock, not
/// behaviour, so it is left out rather than pinned.
#[test]
fn the_reading_speed_and_scroll_style_decide_the_timer_interval() {
    let elisp_form = r##"(let (results)
  (dolist (spec '((word 3.0) (word 1.5) (line nil)))
    (push
     (amr-test-in-buffer
      (let ((baseline (amr-test-timer-baseline))
            (amread-scroll-style (nth 0 spec))
            (amread-word-speed (or (nth 1 spec) 3.0))
            (amread-line-speed 4.0)
            (amread-voice-reader-enabled nil))
        (amr-test-with-answers '("english")
          (amread-mode 1)
          (let ((new (amr-test-new-timers baseline)))
            (list :style (nth 0 spec)
                  :speed (nth 1 spec)
                  :count (length new)
                  :repeat-hundredths (round (* 100 (timer--repeat-delay (car new)))))))))
     results))
  (nreverse results))"##;

    let expect = expect![[
        r#"OK ((:style word :speed 3.0 :count 1 :repeat-hundredths 33) (:style word :speed 1.5 :count 1 :repeat-hundredths 67) (:style line :speed nil :count 1 :repeat-hundredths 400))"#
    ]];

    assert_amread_mode_parity(elisp_form, expect);
}

/// Pausing and resuming with `amread-pause-or-resume`.  Pausing cancels the
/// timer and deletes the highlight but keeps the position, and it also clears
/// `amread-scroll-style`, which is why resuming asks for the style again as
/// well as the language -- all three prompts are pinned in order.
///
/// Resuming produces one empty highlight before moving on: it returns point to
/// the saved position, which is the end of the last word, so the first firing
/// finds no word characters to skip and highlights a zero-width range.
#[test]
fn pausing_cancels_the_timer_and_resuming_restarts_from_the_saved_position() {
    let elisp_form = r##"(amr-test-in-buffer
 (let ((baseline (amr-test-timer-baseline))
       (amread-scroll-style 'word)
       (amread-word-speed 3.0)
       (amread-voice-reader-enabled nil))
   (amr-test-with-answers '("english" "word" "english")
     (amread-mode 1)
     (let ((new (amr-test-new-timers baseline)))
       (dolist (timer new) (timer-event-handler timer))
       (dolist (timer new) (timer-event-handler timer))
       (let ((running (list :overlay (amr-test-overlay)
                            :timer-live (and amread--timer t)
                            :position amread--current-position)))
         (amread-pause-or-resume)
         (let ((paused (list :timer-live (and amread--timer t)
                             :overlay (amr-test-overlay)
                             :position amread--current-position
                             :scroll-style amread-scroll-style)))
           (amread-pause-or-resume)
           (let ((resumed-new (amr-test-new-timers baseline)))
             (dolist (timer resumed-new) (timer-event-handler timer))
             (list :prompts (reverse amr-test-prompts)
                   :running running :paused paused
                   :resumed (list :timer-live (and amread--timer t)
                                  :overlay (amr-test-overlay))))))))))"##;

    let expect = expect![[
        r#"OK (:prompts (("[amread] Select language: " "english") ("amread-mode scroll style: " "word") ("[amread] Select language: " "english")) :running (:overlay (:start 5 :end 8 :text "Weg" :face amread-highlight-face) :timer-live t :position 8) :paused (:timer-live nil :overlay no-overlay :position 8 :scroll-style nil) :resumed (:timer-live t :overlay (:start 8 :end 8 :text "" :face amread-highlight-face)))"#
    ]];

    assert_amread_mode_parity(elisp_form, expect);
}

/// Reading off the end.  The fixture is short, so firing the timer repeatedly
/// reaches the end of the buffer, at which point the package turns itself off
/// rather than looping: seventeen firings, mode off, timer variable cleared and
/// the timer no longer scheduled, saved position cleared, highlight gone.
#[test]
fn reaching_the_end_of_the_buffer_turns_the_mode_off_by_itself() {
    let elisp_form = r##"(amr-test-in-buffer
 (let ((baseline (amr-test-timer-baseline))
       (amread-scroll-style 'word)
       (amread-word-speed 3.0)
       (amread-voice-reader-enabled nil))
   (amr-test-with-answers '("english")
     (amread-mode 1)
     (let ((new (amr-test-new-timers baseline)) (fired 0))
       (while (and amread-mode (< fired 40))
         (dolist (timer new)
           (when (memq timer timer-list) (timer-event-handler timer)))
         (setq fired (1+ fired)))
       (list :fired fired
             :mode-still-on (and amread-mode t)
             :timer-var amread--timer
             :position amread--current-position
             :overlay (amr-test-overlay)
             :timer-still-scheduled (and (memq (car new) timer-list) t)
             :point-at-end (= (point) (point-max)))))))"##;

    let expect = expect![[
        r#"OK (:fired 17 :mode-still-on nil :timer-var nil :position nil :overlay no-overlay :timer-still-scheduled nil :point-at-end t)"#
    ]];

    assert_amread_mode_parity(elisp_form, expect);
}

/// Switching the mode off mid-session leaves nothing running: the timer is
/// cancelled and gone from `timer-list`, the overlay is deleted, the buffer's
/// read-only state is restored and the scroll style is cleared.
///
/// One asymmetry is recorded rather than glossed: `amread--overlay` still holds
/// the overlay object afterwards.  `amread-stop` calls `delete-overlay` but
/// never sets the variable back to nil, so what is left is a dead overlay with
/// no buffer -- harmless here, but it means a later session reuses that object
/// instead of making a fresh one.
#[test]
fn turning_the_mode_off_cancels_the_timer_and_deletes_the_overlay() {
    let elisp_form = r##"(amr-test-in-buffer
 (let ((baseline (amr-test-timer-baseline))
       (amread-scroll-style 'word)
       (amread-word-speed 3.0)
       (amread-voice-reader-enabled nil))
   (amr-test-with-answers '("english")
     (amread-mode 1)
     (let ((new (amr-test-new-timers baseline)))
       (dolist (timer new) (timer-event-handler timer))
       (dolist (timer new) (timer-event-handler timer))
       (let ((on (list :timer-live (and amread--timer t)
                       :overlay (amr-test-overlay)
                       :new-timers (length (amr-test-new-timers baseline))
                       :read-only buffer-read-only)))
         (amread-mode -1)
         (list :on on
               :off (list :mode (and amread-mode t)
                          :timer-var amread--timer
                          :overlay (amr-test-overlay)
                          :overlay-var-still-set (and amread--overlay t)
                          :overlay-buffer (and amread--overlay
                                               (overlay-buffer amread--overlay) t)
                          :leftover-timers (length (amr-test-new-timers baseline))
                          :read-only buffer-read-only
                          :scroll-style amread-scroll-style)))))))"##;

    let expect = expect![[
        r#"OK (:on (:timer-live t :overlay (:start 5 :end 8 :text "Weg" :face amread-highlight-face) :new-timers 1 :read-only t) :off (:mode nil :timer-var nil :overlay no-overlay :overlay-var-still-set t :overlay-buffer nil :leftover-timers 0 :read-only nil :scroll-style nil))"#
    ]];

    assert_amread_mode_parity(elisp_form, expect);
}

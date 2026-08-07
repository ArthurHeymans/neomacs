use expect_test::expect;

use super::ParityBatchCase;

fn defaults_and_commands_are_registered() -> ParityBatchCase {
    ParityBatchCase::value(
        "defaults_and_commands_are_registered",
        r####"
(list :interface flyspell-correct-interface
      :highlight flyspell-correct-highlight
      :direction flyspell-correct-default-direction
      :abort-on-quit flyspell-correct-abort-on-quit
      :auto-delay flyspell-correct-auto-delay
      :at-point (commandp 'flyspell-correct-at-point)
      :previous (commandp 'flyspell-correct-previous)
      :next (commandp 'flyspell-correct-next)
      :wrapper (commandp 'flyspell-correct-wrapper)
      :region (commandp 'flyspell-correct-region)
      :auto-mode (commandp 'flyspell-correct-auto-mode)
      :feature (featurep 'flyspell-correct))
"####,
        expect![
            "OK (:interface flyspell-correct-completing-read :highlight t :direction backward :abort-on-quit t :auto-delay 1.6 :at-point t :previous t :next t :wrapper t :region t :auto-mode t :feature t)"
        ],
    )
}

fn completing_read_actions_include_candidates_and_control_ops() -> ParityBatchCase {
    ParityBatchCase::value(
        "completing_read_actions_include_candidates_and_control_ops",
        r####"
(let* ((result nil)
       (fake-completing-read
        (lambda (prompt collection &rest _)
          (let* ((all (all-completions "" collection))
                 (choice (car all)))
            (setq result
                  (list :prompt prompt
                        :count (length all)
                        :first (substring-no-properties choice)
                        :has-save
                        (cl-some (lambda (c)
                                   (string-match-p "Save" c))
                                 all)
                        :has-skip
                        (cl-some (lambda (c)
                                   (string-match-p "Skip" c))
                                 all)))
            choice))))
  (cl-letf (((symbol-function 'completing-read) fake-completing-read))
    (let ((choice
           (flyspell-correct-completing-read
            '("correct" "corret" "corr")
            "misspeled")))
      (list :choice choice
            :result result
            :actions (mapcar #'car flyspell-correct--cr-actions)
            :key flyspell-correct--cr-key))))
"####,
        expect![[
            r#"OK (:choice "correct" :result (:prompt "Suggestions for \"misspeled\": " :count 8 :first "1 correct" :has-save 4 :has-skip 4) :actions (save session buffer skip stop) :key "@")"#
        ]],
    )
}

fn highlight_overlay_tracks_word_bounds() -> ParityBatchCase {
    ParityBatchCase::value(
        "highlight_overlay_tracks_word_bounds",
        r####"
(with-temp-buffer
  (insert "hello misspeled world")
  (goto-char (point-min))
  (search-forward "misspeled")
  (forward-word -1)
  (let ((flyspell-correct-highlight t)
        (flyspell-correct-overlay nil))
    (cl-letf (((symbol-function 'flyspell-get-word)
               (lambda (&rest _)
                 (list "misspeled"
                       (match-beginning 0)
                       (match-end 0)))))
      (search-forward "misspeled")
      (let* ((start (match-beginning 0))
             (end (match-end 0)))
        (cl-letf (((symbol-function 'flyspell-get-word)
                   (lambda (&rest _) (list "misspeled" start end))))
          (flyspell-correct--highlight-add)
          (let ((ov flyspell-correct-overlay))
            (list :overlayp (overlayp ov)
                  :start (and ov (overlay-start ov))
                  :end (and ov (overlay-end ov))
                  :face (and ov (overlay-get ov 'face))
                  :after
                  (progn
                    (flyspell-correct--highlight-remove)
                    flyspell-correct-overlay))))))))
"####,
        expect!["OK (:overlayp nil :start nil :end nil :face nil :after nil)"],
    )
}

fn wrapper_dispatches_to_next_or_previous_by_direction() -> ParityBatchCase {
    ParityBatchCase::value(
        "wrapper_dispatches_to_next_or_previous_by_direction",
        r####"
(let (calls)
  (cl-letf (((symbol-function 'flyspell-correct-move)
             (lambda (position &optional forward rapid)
               (push (list :pos position :forward forward :rapid rapid)
                     calls))))
    (let ((flyspell-correct-default-direction 'backward)
          (current-prefix-arg nil))
      (flyspell-correct-wrapper))
    (let ((flyspell-correct-default-direction 'forward)
          (current-prefix-arg 4))
      (flyspell-correct-wrapper))
    (nreverse calls)))
"####,
        expect!["OK ((:pos 1 :forward nil :rapid nil) (:pos 1 :forward t :rapid nil))"],
    )
}

fn auto_mode_timer_helpers_cancel_cleanly() -> ParityBatchCase {
    ParityBatchCase::value(
        "auto_mode_timer_helpers_cancel_cleanly",
        r####"
(with-temp-buffer
  (let ((flyspell-correct--auto-timer (run-with-timer 1000 nil #'ignore)))
    (flyspell-correct-auto-cancel-timer)
    (list :timer-cancelled (null flyspell-correct--auto-timer)
          :soon-without-flyspell
          (progn
            ;; Without flyspell-mode, auto-soon should leave no timer.
            (flyspell-correct-auto-soon)
            (null flyspell-correct--auto-timer))
          :auto-mode (commandp 'flyspell-correct-auto-mode))))
"####,
        expect!["OK (:timer-cancelled t :soon-without-flyspell t :auto-mode t)"],
    )
}

pub(super) fn workflow_batch_cases() -> Vec<ParityBatchCase> {
    vec![
        defaults_and_commands_are_registered(),
        completing_read_actions_include_candidates_and_control_ops(),
        highlight_overlay_tracks_word_bounds(),
        wrapper_dispatches_to_next_or_previous_by_direction(),
        auto_mode_timer_helpers_cancel_cleanly(),
    ]
}

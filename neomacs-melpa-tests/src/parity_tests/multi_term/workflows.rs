use expect_test::expect;

use super::ParityBatchCase;

fn creating_terms_appends_numbered_buffers_to_the_list() -> ParityBatchCase {
    ParityBatchCase::value(
        "creating_terms_appends_numbered_buffers_to_the_list",
        r####"
(neomacs-multi-term-test-with-fakes
 (lambda ()
   (save-window-excursion
     (multi-term)
     (let ((first (neomacs-multi-term-test-names)))
       (multi-term)
       (list :first first
             :second (neomacs-multi-term-test-names)
             :current (buffer-name)
             :internal (and (bound-and-true-p multi-term-internal-ran) t))))))
"####,
        expect![[
            r#"OK (:first ("*terminal<1>*") :second ("*terminal<1>*" "*terminal<2>*") :current "*terminal<2>*" :internal t)"#
        ]],
    )
}

fn next_and_prev_cycle_the_managed_buffer_list() -> ParityBatchCase {
    ParityBatchCase::value(
        "next_and_prev_cycle_the_managed_buffer_list",
        r####"
(neomacs-multi-term-test-with-fakes
 (lambda ()
   (save-window-excursion
     (multi-term)
     (multi-term)
     (multi-term)
     (let ((names (neomacs-multi-term-test-names))
           states)
       (switch-to-buffer (car multi-term-buffer-list))
       (push (cons 'start (buffer-name)) states)
       (multi-term-next)
       (push (cons 'next (buffer-name)) states)
       (multi-term-next)
       (push (cons 'next2 (buffer-name)) states)
       (multi-term-prev)
       (push (cons 'prev (buffer-name)) states)
       (list :names names :states (nreverse states))))))
"####,
        expect![[
            r#"OK (:names ("*terminal<1>*" "*terminal<2>*" "*terminal<3>*") :states ((start . "*terminal<1>*") (next . "*terminal<2>*") (next2 . "*terminal<3>*") (prev . "*terminal<2>*")))"#
        ]],
    )
}

fn dedicated_open_creates_a_dedicated_window_and_toggle_closes_it() -> ParityBatchCase {
    ParityBatchCase::value(
        "dedicated_open_creates_a_dedicated_window_and_toggle_closes_it",
        r####"
(neomacs-multi-term-test-with-fakes
 (lambda ()
   (save-window-excursion
     (delete-other-windows)
     (multi-term-dedicated-open)
     (let ((opened
            (list :exist (and (multi-term-dedicated-exist-p) t)
                  :buffer (and multi-term-dedicated-buffer
                               (buffer-name multi-term-dedicated-buffer))
                  :dedicated
                  (and multi-term-dedicated-window
                       (window-dedicated-p multi-term-dedicated-window))
                  :name (multi-term-dedicated-get-buffer-name)
                  :windows (length (window-list)))))
       (multi-term-dedicated-toggle)
       (list :opened opened
             :closed
             (list :exist (and (multi-term-dedicated-exist-p) t)
                   :windows (length (window-list))))))))
"####,
        expect![[
            r#"OK (:opened (:exist t :buffer "*MULTI-TERM-DEDICATED*" :dedicated t :name "*MULTI-TERM-DEDICATED*" :windows 2) :closed (:exist nil :windows 1))"#
        ]],
    )
}

fn kill_buffer_hook_removes_terms_from_the_managed_list() -> ParityBatchCase {
    ParityBatchCase::value(
        "kill_buffer_hook_removes_terms_from_the_managed_list",
        r####"
(neomacs-multi-term-test-with-fakes
 (lambda ()
   (save-window-excursion
     (multi-term)
     (multi-term)
     (let* ((before (neomacs-multi-term-test-names))
            (victim (car multi-term-buffer-list)))
       (switch-to-buffer victim)
       (setq major-mode 'term-mode)
       (multi-term-kill-buffer-hook)
       (kill-buffer victim)
       (list :before before
             :after (neomacs-multi-term-test-names)
             :victim (buffer-name victim))))))
"####,
        expect![[
            r#"OK (:before ("*terminal<1>*" "*terminal<2>*") :after ("*terminal<2>*") :victim nil)"#
        ]],
    )
}

fn buffer_existence_and_naming_helpers_are_deterministic() -> ParityBatchCase {
    ParityBatchCase::value(
        "buffer_existence_and_naming_helpers_are_deterministic",
        r####"
(neomacs-multi-term-test-with-fakes
 (lambda ()
   (let* ((alive (get-buffer-create "*probe*"))
          (dead (get-buffer-create "*dead-probe*")))
     (kill-buffer dead)
     (list :alive (and (multi-term-buffer-exist-p alive) t)
           :dead (and (multi-term-buffer-exist-p dead) t)
           :window-alive (and (multi-term-window-exist-p (selected-window)) t)
           :window-dead (and (multi-term-window-exist-p nil) t)
           :dedicated-name (multi-term-dedicated-get-buffer-name)
           :program multi-term-program
           :buffer-name multi-term-buffer-name))))
"####,
        expect![[
            r#"OK (:alive t :dead nil :window-alive t :window-dead nil :dedicated-name "*MULTI-TERM-DEDICATED*" :program "/bin/sh" :buffer-name "terminal")"#
        ]],
    )
}

pub(super) fn workflow_batch_cases() -> Vec<ParityBatchCase> {
    vec![
        creating_terms_appends_numbered_buffers_to_the_list(),
        next_and_prev_cycle_the_managed_buffer_list(),
        dedicated_open_creates_a_dedicated_window_and_toggle_closes_it(),
        kill_buffer_hook_removes_terms_from_the_managed_list(),
        buffer_existence_and_naming_helpers_are_deterministic(),
    ]
}

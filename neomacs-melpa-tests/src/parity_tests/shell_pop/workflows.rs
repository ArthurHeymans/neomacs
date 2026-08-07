use expect_test::expect;

use super::ParityBatchCase;

fn defaults_and_buffer_naming_are_deterministic() -> ParityBatchCase {
    ParityBatchCase::value(
        "defaults_and_buffer_naming_are_deterministic",
        r####"
(list :size shell-pop-window-size
      :height shell-pop-window-height
      :position shell-pop-window-position
      :shell-type (list (car shell-pop-shell-type)
                        (cadr shell-pop-shell-type))
      :autocd shell-pop-autocd-to-working-dir
      :cleanup shell-pop-cleanup-buffer-at-process-exit
      :command (commandp 'shell-pop)
      :name-1 (shell-pop--shell-buffer-name 1)
      :name-3 (shell-pop--shell-buffer-name 3)
      :pos-top (shell-pop--translate-position "top")
      :pos-bottom (shell-pop--translate-position "bottom")
      :pos-left (shell-pop--translate-position "left")
      :pos-right (shell-pop--translate-position "right")
      :feature (featurep 'shell-pop))
"####,
        expect![[
            r#"OK (:size 30 :height 30 :position "bottom" :shell-type ("shell" "*shell*") :autocd t :cleanup t :command t :name-1 "*shell-1*" :name-3 "*shell-3*" :pos-top above :pos-bottom below :pos-left left :pos-right right :feature t)"#
        ]],
    )
}

fn window_size_scales_with_height_setting() -> ParityBatchCase {
    ParityBatchCase::value(
        "window_size_scales_with_height_setting",
        r####"
(let ((shell-pop-full-span nil)
      (shell-pop-window-position "bottom"))
  (list :default
        (let ((shell-pop-window-height 30))
          (shell-pop--calculate-window-size))
        :half
        (let ((shell-pop-window-height 50))
          (shell-pop--calculate-window-size))
        :almost-full
        (let ((shell-pop-window-height 90))
          (shell-pop--calculate-window-size))))
"####,
        expect!["OK (:default 17 :half 12 :almost-full 2)"],
    )
}

fn switch_to_shell_buffer_creates_and_renames_via_mode_func() -> ParityBatchCase {
    ParityBatchCase::value(
        "switch_to_shell_buffer_creates_and_renames_via_mode_func",
        r####"
(let ((shell-pop-internal-mode "shell")
      (shell-pop-internal-mode-buffer "*shell*")
      (shell-pop-internal-mode-func #'neomacs-shell-pop-test-fake-shell)
      (shell-pop-last-shell-buffer-index 1)
      (shell-pop--is-shell-buffer nil)
      (created nil))
  (unwind-protect
      (progn
        (dolist (name '("*shell*" "*shell*-1*" "*shell*-2*"))
          (when (get-buffer name)
            (let ((kill-buffer-query-functions nil)
                  (kill-buffer-hook nil))
              (kill-buffer name))))
        (shell-pop--switch-to-shell-buffer 1)
        (setq created (buffer-name))
        (list :created created
              :is-shell shell-pop--is-shell-buffer
              :index shell-pop-last-shell-buffer-index
              :lives (and (get-buffer "*shell*-1*") t)
              :second
              (progn
                (shell-pop--switch-to-shell-buffer 2)
                (list :name (buffer-name)
                      :index shell-pop-last-shell-buffer-index))))
    (dolist (name '("*shell*" "*shell*-1*" "*shell*-2*"))
      (when (get-buffer name)
        (let ((kill-buffer-query-functions nil)
              (kill-buffer-hook nil))
          (kill-buffer name))))))
"####,
        expect![[
            r#"OK (:created "*shell-1*" :is-shell t :index 1 :lives nil :second (:name "*shell-2*" :index 2))"#
        ]],
    )
}

fn unused_index_scan_skips_existing_shell_buffers() -> ParityBatchCase {
    ParityBatchCase::value(
        "unused_index_scan_skips_existing_shell_buffers",
        r####"
(let ((shell-pop-internal-mode-buffer "*shell*")
      (a (get-buffer-create "*shell*-1*"))
      (b (get-buffer-create "*shell*-2*")))
  (unwind-protect
      (let ((cell (shell-pop-get-unused-internal-mode-buffer-window)))
        (list :index (car cell)
              :window (cdr cell)))
    (let ((kill-buffer-query-functions nil)
          (kill-buffer-hook nil))
      (when (buffer-live-p a) (kill-buffer a))
      (when (buffer-live-p b) (kill-buffer b)))))
"####,
        expect!["OK (:index 3 :window nil)"],
    )
}

fn target_index_handles_prefix_and_default() -> ParityBatchCase {
    ParityBatchCase::value(
        "target_index_handles_prefix_and_default",
        r####"
(let ((shell-pop-last-shell-buffer-index 5))
  (list :nil (shell-pop--target-index nil)
        :numeric (shell-pop--target-index 3)
        :raw-prefix (shell-pop--target-index '(4))))
"####,
        expect!["OK (:nil 5 :numeric 3 :raw-prefix 3)"],
    )
}

pub(super) fn workflow_batch_cases() -> Vec<ParityBatchCase> {
    vec![
        defaults_and_buffer_naming_are_deterministic(),
        window_size_scales_with_height_setting(),
        switch_to_shell_buffer_creates_and_renames_via_mode_func(),
        unused_index_scan_skips_existing_shell_buffers(),
        target_index_handles_prefix_and_default(),
    ]
}

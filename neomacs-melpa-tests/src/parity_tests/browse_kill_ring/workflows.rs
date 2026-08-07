use expect_test::expect;

use super::ParityBatchCase;

fn defaults_and_commands_are_registered() -> ParityBatchCase {
    ParityBatchCase::value(
        "defaults_and_commands_are_registered",
        r####"
(list :style browse-kill-ring-display-style
      :quit browse-kill-ring-quit-action
      :separator browse-kill-ring-separator
      :duplicates browse-kill-ring-display-duplicates
      :replace-yank browse-kill-ring-replace-yank
      :preview browse-kill-ring-show-preview
      :browse (commandp 'browse-kill-ring)
      :insert (commandp 'browse-kill-ring-insert)
      :forward (commandp 'browse-kill-ring-forward)
      :previous (commandp 'browse-kill-ring-previous)
      :delete (commandp 'browse-kill-ring-delete)
      :keys (fboundp 'browse-kill-ring-default-keybindings)
      :elide (fboundp 'browse-kill-ring-elide)
      :feature (featurep 'browse-kill-ring))
"####,
        expect![[
            r#"OK (:style separated :quit save-and-restore :separator "-------" :duplicates t :replace-yank t :preview t :browse t :insert t :forward t :previous t :delete t :keys t :elide t :feature t)"#
        ]],
    )
}

fn elide_truncates_long_items_when_maximum_set() -> ParityBatchCase {
    ParityBatchCase::value(
        "elide_truncates_long_items_when_maximum_set",
        r####"
(let ((browse-kill-ring-maximum-display-length nil)
      (long (make-string 40 ?x)))
  (list :unlimited (browse-kill-ring-elide long)
        :limited
        (let ((browse-kill-ring-maximum-display-length 10))
          (browse-kill-ring-elide long))
        :short
        (let ((browse-kill-ring-maximum-display-length 10))
          (browse-kill-ring-elide "abc"))))
"####,
        expect![[
            r#"OK (:unlimited "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" :limited #("xxxxxxx..." 7 10 (browse-kill-ring-extra t)) :short "abc")"#
        ]],
    )
}

fn setup_populates_browser_buffer_with_kill_ring_items() -> ParityBatchCase {
    ParityBatchCase::value(
        "setup_populates_browser_buffer_with_kill_ring_items",
        r####"
(let ((kill-ring '("alpha" "beta" "gamma"))
      (kill-ring-yank-pointer nil)
      (browse-kill-ring-display-style 'separated)
      (browse-kill-ring-display-duplicates t)
      (browse-kill-ring-maximum-display-length nil)
      (browse-kill-ring-show-preview nil)
      (orig (get-buffer-create " *neomacs-bkr-orig*"))
      (kill-buf (get-buffer-create " *neomacs-bkr*")))
  (unwind-protect
      (with-current-buffer orig
        (let ((window-config (current-window-configuration)))
          (browse-kill-ring-setup kill-buf orig nil nil window-config)
          (with-current-buffer kill-buf
            (list :mode major-mode
                  :text (string-trim (buffer-string))
                  :has-alpha (and (search-forward "alpha" nil t) t)
                  :has-beta (and (search-forward "beta" nil t) t)
                  :has-gamma (and (search-forward "gamma" nil t) t)
                  :overlays
                  (length
                   (cl-remove-if-not
                    (lambda (o) (overlay-get o 'browse-kill-ring-target))
                    (overlays-in (point-min) (point-max))))))))
    (let ((kill-buffer-hook nil)
          (kill-buffer-query-functions nil))
      (when (buffer-live-p orig) (kill-buffer orig))
      (when (buffer-live-p kill-buf) (kill-buffer kill-buf)))))
"####,
        expect![[
            r#"OK (:mode browse-kill-ring-mode :text #("alpha\n-------\nbeta\n-------\ngamma" 6 13 (browse-kill-ring-separator t browse-kill-ring-extra t) 19 26 (browse-kill-ring-separator t browse-kill-ring-extra t)) :has-alpha t :has-beta t :has-gamma t :overlays 3)"#
        ]],
    )
}

fn insert_and_highlight_copies_string_into_target_buffer() -> ParityBatchCase {
    ParityBatchCase::value(
        "insert_and_highlight_copies_string_into_target_buffer",
        r####"
(with-temp-buffer
  (let ((browse-kill-ring-highlight-inserted-item nil))
    (browse-kill-ring-insert-and-highlight "hello")
    (list :text (buffer-string)
          :point (point))))
"####,
        expect![[r#"OK (:text "hello" :point 6)"#]],
    )
}

fn default_keybindings_install_yank_pop_binding() -> ParityBatchCase {
    ParityBatchCase::value(
        "default_keybindings_install_yank_pop_binding",
        r####"
(let ((browse-kill-ring-replace-yank t))
  (browse-kill-ring-default-keybindings)
  (list :m-y (lookup-key (current-global-map) (kbd "M-y"))
        :command (commandp 'browse-kill-ring)))
"####,
        expect!["OK (:m-y yank-pop :command t)"],
    )
}

pub(super) fn workflow_batch_cases() -> Vec<ParityBatchCase> {
    vec![
        defaults_and_commands_are_registered(),
        elide_truncates_long_items_when_maximum_set(),
        setup_populates_browser_buffer_with_kill_ring_items(),
        insert_and_highlight_copies_string_into_target_buffer(),
        default_keybindings_install_yank_pop_binding(),
    ]
}

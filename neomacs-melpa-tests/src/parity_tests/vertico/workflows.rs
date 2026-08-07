use expect_test::expect;

use super::ParityBatchCase;

fn mode_registers_completion_ui_and_map_commands() -> ParityBatchCase {
    ParityBatchCase::value(
        "mode_registers_completion_ui_and_map_commands",
        r####"
(list :mode (commandp 'vertico-mode)
      :next (commandp 'vertico-next)
      :previous (commandp 'vertico-previous)
      :first (commandp 'vertico-first)
      :last (commandp 'vertico-last)
      :exit (commandp 'vertico-exit)
      :insert (commandp 'vertico-insert)
      :count vertico-count
      :preselect vertico-preselect
      :sort (functionp vertico-sort-function)
      :style (assq 'vertico completion-styles-alist))
"####,
        expect![
            "OK (:mode t :next t :previous t :first t :last t :exit t :insert t :count 10 :preselect directory :sort t :style nil)"
        ],
    )
}

fn move_to_front_and_cycle_helpers_reorder_candidates() -> ParityBatchCase {
    ParityBatchCase::value(
        "move_to_front_and_cycle_helpers_reorder_candidates",
        r####"
(let* ((list '("a" "b" "c" "d"))
       (front (vertico--move-to-front "c" (copy-sequence list)))
       (cycled (vertico--cycle (copy-sequence list) 2)))
  (list :front front
        :cycled cycled
        :identity (vertico--move-to-front "missing" (copy-sequence list))))
"####,
        expect![[
            r#"OK (:front ("c" "a" "b" "d") :cycled ("c" "d" "a" "b") :identity ("a" "b" "c" "d"))"#
        ]],
    )
}

fn navigation_updates_index_with_optional_wrapping() -> ParityBatchCase {
    ParityBatchCase::value(
        "navigation_updates_index_with_optional_wrapping",
        r####"
(neomacs-vertico-test-with-session
 (lambda ()
   (vertico--goto 0)
   (let ((start vertico--index))
     (vertico-next)
     (let ((next vertico--index))
       (vertico-previous)
       (let ((prev vertico--index))
         (vertico-last)
         (let ((last vertico--index))
           (vertico-first)
           (list :start start
                 :next next
                 :prev prev
                 :last last
                 :first vertico--index
                 :total vertico--total)))))))
"####,
        expect!["OK (:start 0 :next 1 :prev 0 :last 4 :first 0 :total 5)"],
    )
}

fn format_count_and_candidate_helpers_report_session_state() -> ParityBatchCase {
    ParityBatchCase::value(
        "format_count_and_candidate_helpers_report_session_state",
        r####"
(neomacs-vertico-test-with-session
 (lambda ()
   (vertico--goto 1)
   (list :count (vertico--format-count)
         :candidate (vertico--candidate)
         :match-p (and (vertico--match-p "app") t)
         :scroll (progn
                   (setq vertico--index 4)
                   (vertico--compute-scroll)
                   vertico--scroll))))
"####,
        expect![[r#"OK (:count "2/5    " :candidate "apricot" :match-p t :scroll 2)"#]],
    )
}

fn mode_toggle_installs_and_removes_completion_in_region_function() -> ParityBatchCase {
    ParityBatchCase::value(
        "mode_toggle_installs_and_removes_completion_in_region_function",
        r####"
(let ((before completion-in-region-function)
      (vertico-mode nil))
  (vertico-mode 1)
  (let ((on completion-in-region-function)
        (mode (and vertico-mode t)))
    (vertico-mode -1)
    (list :before before
          :on on
          :mode-on mode
          :mode-off (and vertico-mode t)
          :after completion-in-region-function
          :map-next (lookup-key vertico-map (kbd "C-n"))
          :map-prev (lookup-key vertico-map (kbd "C-p")))))
"####,
        expect![
            "OK (:before completion--in-region :on completion--in-region :mode-on t :mode-off nil :after completion--in-region :map-next nil :map-prev nil)"
        ],
    )
}

pub(super) fn workflow_batch_cases() -> Vec<ParityBatchCase> {
    vec![
        mode_registers_completion_ui_and_map_commands(),
        move_to_front_and_cycle_helpers_reorder_candidates(),
        navigation_updates_index_with_optional_wrapping(),
        format_count_and_candidate_helpers_report_session_state(),
        mode_toggle_installs_and_removes_completion_in_region_function(),
    ]
}

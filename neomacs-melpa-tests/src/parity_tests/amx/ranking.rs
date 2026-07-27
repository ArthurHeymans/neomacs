use expect_test::expect;

use super::assert_amx_parity;

#[test]
fn sorting_rules_apply_frequency_then_shorter_name_then_alphabetical_order() {
    let elisp_form = r##"
(mapcar
 (lambda (pair)
   (let ((left (car pair))
         (right (cadr pair)))
     (list
      left right
      (amx-sorting-rules left right)
      (amx-sorting-rules right left))))
 '(((amx-test-alpha . 9) (amx-test-beta . 2))
   ((amx-test-alpha . 2) (amx-test-beta . 2))
   ((amx-test-beta . 2) (amx-test-gamma . 2))
   ((aa . 0) (ab . 0))
   ((same . 3) (same . 3))
   ((nil-count) (one . 1))))
"##;
    let expect = expect![
        "OK (((amx-test-alpha . 9) (amx-test-beta . 2) t nil) ((amx-test-alpha . 2) (amx-test-beta . 2) nil t) ((amx-test-beta . 2) (amx-test-gamma . 2) t nil) ((aa . 0) (ab . 0) t nil) ((same . 3) (same . 3) nil nil) ((nil-count) (one . 1) nil t))"
    ];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn update_counter_initializes_shared_data_cell_and_increments_existing_counts() {
    let elisp_form = r##"
(let* ((fresh (list 'amx-test-alpha))
       (existing (cons 'amx-test-beta 4)))
  (setq amx-data nil)
  (amx-update-counter fresh)
  (let ((after-first
         (list
          (copy-tree fresh)
          (copy-tree amx-data)
          (eq fresh (car amx-data)))))
    (amx-update-counter fresh)
    (amx-update-counter existing)
    (list
     after-first
     fresh
     existing
     amx-data
     (eq fresh (car amx-data))
     (memq existing amx-data))))
"##;
    let expect = expect![
        "OK (((amx-test-alpha . 1) ((amx-test-alpha . 1)) t) #1=(amx-test-alpha . 2) (amx-test-beta . 5) (#1#) t nil)"
    ];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn ranking_moves_executed_command_to_front_and_repairs_history_tail_order() {
    let elisp_form = r##"
(let* ((alpha (cons 'amx-test-alpha 8))
       (beta (cons 'amx-test-beta 2))
       (gamma (cons 'amx-test-gamma 6))
       (delta (cons 'amx-test-delta 1))
       (amx-history-length 2))
  (setq amx-cache (list alpha beta gamma delta)
        amx-data (list alpha beta gamma delta)
        amx-history
        '(amx-test-alpha amx-test-gamma))
  (amx-rank 'amx-test-beta)
  (list
   amx-cache
   amx-data
   (mapcar
    (lambda (cell)
      (memq cell amx-data))
    amx-cache)
   (eq (car amx-cache) beta)))
"##;
    let expect = expect![
        "OK ((#2=(amx-test-beta . 3) #1=(amx-test-alpha . 8) #3=(amx-test-gamma . 6) #4=(amx-test-delta . 1)) #6=(#1# . #5=(#2# . #7=(#3# . #8=(#4#)))) (#5# #6# #7# #8#) t)"
    ];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn restore_history_truncates_history_and_promotes_only_known_commands() {
    let elisp_form = r##"
(let ((amx-history-length 3))
  (setq amx-cache
        '((amx-test-alpha . 8)
          (amx-test-beta . 6)
          (amx-test-gamma . 4)
          (amx-test-delta . 2))
        amx-history
        '(amx-test-gamma
          amx-test-missing
          amx-test-beta
          amx-test-alpha))
  (amx-restore-history)
  (list amx-history amx-cache))
"##;
    let expect = expect![
        "OK ((amx-test-gamma amx-test-missing amx-test-beta) ((amx-test-gamma . 4) (amx-test-beta . 6) (amx-test-alpha . 8) (amx-test-delta . 2)))"
    ];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn save_history_respects_configured_length_and_available_cache_entries() {
    let elisp_form = r##"
(mapcar
 (lambda (length)
   (let ((amx-history-length length))
     (setq amx-history '(stale)
           amx-cache
           '((amx-test-alpha . 9)
             (amx-test-beta . 7)
             (amx-test-gamma . 5)))
     (list length
           (amx-save-history)
           amx-history)))
 '(1 2 7))
"##;
    let expect = expect![
        "OK ((1 #1=(amx-test-alpha) #1#) (2 #2=(amx-test-alpha amx-test-beta) #2#) (7 #3=(amx-test-alpha amx-test-beta amx-test-gamma) #3#))"
    ];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn sort_according_to_cache_orders_known_commands_and_retains_unknowns() {
    let elisp_form = r##"
(progn
  (setq amx-cache
        '((amx-test-beta . 7)
          (amx-test-alpha . 5)
          (amx-test-gamma . 3)))
  (list
   (amx-sort-according-to-cache
    '(outside-a amx-test-alpha outside-b
      amx-test-gamma amx-test-beta))
   (amx-sort-according-to-cache
    '(outside-b outside-a))
   (amx-sort-according-to-cache nil)))
"##;
    let expect = expect![
        "OK ((amx-test-beta amx-test-alpha amx-test-gamma outside-b outside-a) (outside-a outside-b) nil)"
    ];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn mutable_list_cell_utilities_detect_remove_and_reinsert_exact_cells() {
    let elisp_form = r##"
(let* ((items (list 'zero 'one 'two 'three))
       (position
        (amx-detect-position
         items
         (lambda (cell)
           (eq (car cell) 'two))))
       (missing
        (amx-detect-position
         items
         (lambda (cell)
           (eq (car cell) 'absent))))
       (removed
        (amx-remove-nth-cell position items)))
  (let ((after-remove (copy-sequence items)))
    (amx-insert-cell removed 1 items)
    (list
     position
     missing
     after-remove
     items
     (car removed)
     (eq (nthcdr 1 items) removed))))
"##;
    let expect = expect!["OK (2 nil (zero one three) (zero two one three) two t)"];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn update_if_needed_distinguishes_forced_stale_recount_and_noop_paths() {
    let elisp_form = r##"
(let ((run
       (lambda (last-update count-commands detected)
         (let (events)
           (cl-letf
               (((symbol-function 'amx-detect-new-commands)
                 (lambda ()
                   (push 'detect events)
                   detected))
                ((symbol-function 'amx-update)
                 (lambda ()
                   (push 'update events)))
                ((symbol-function 'amx--debug-message)
                 (lambda (&rest arguments)
                   (push
                    (cons 'debug arguments)
                    events))))
             (setq amx-last-update-time
                   last-update)
             (amx-update-if-needed count-commands)
             (nreverse events))))))
  (list
   (funcall run nil nil nil)
   (funcall run '(1 2 3 4) nil nil)
   (funcall run '(1 2 3 4) t nil)
   (funcall run '(1 2 3 4) t 17)))
"##;
    let expect = expect![[
        r#"OK ((update) ((debug "No update needed at this time.")) (detect (debug "No update needed at this time.")) (detect update))"#
    ]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn initialize_is_idempotent_but_reinit_reloads_recounts_rebuilds_and_hooks() {
    let elisp_form = r##"
(let ((amx-initialized nil)
      (kill-emacs-hook nil)
      events)
  (cl-letf
      (((symbol-function 'amx-load-save-file)
        (lambda ()
          (push 'load events)))
       ((symbol-function 'amx-detect-new-commands)
        (lambda ()
          (push 'detect events)))
       ((symbol-function 'amx-rebuild-cache)
        (lambda ()
          (push 'rebuild events))))
    (amx-initialize)
    (let ((after-first
           (list
            amx-initialized
            (memq 'amx-save-to-file
                  kill-emacs-hook)
            (nreverse events))))
      (setq events nil)
      (amx-initialize)
      (let ((after-second
             (list amx-initialized events)))
        (amx-initialize t)
        (list
         after-first
         after-second
         amx-initialized
         (memq 'amx-save-to-file
               kill-emacs-hook)
         (nreverse events))))))
"##;
    let expect = expect![
        "OK ((t #1=(amx-save-to-file) (load detect rebuild)) (t nil) t #1# (load detect rebuild))"
    ];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn command_count_detection_notices_new_interactive_definitions_once() {
    let elisp_form = r##"
(let ((amx-command-count 0))
  (let ((initial (amx-detect-new-commands))
        second
        after-definition
        final)
    (setq second (amx-detect-new-commands))
    (fset
     'amx-test-new-command
     (lambda ()
       (interactive)
       'new))
    (setq after-definition
          (amx-detect-new-commands)
          final
          (amx-detect-new-commands))
    (list
     (and initial (> initial 0))
     second
     (and after-definition
          (> after-definition initial))
     final
     (= amx-command-count
        after-definition))))
"##;
    let expect = expect!["OK (t nil t nil t)"];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn full_update_saves_rebuilds_timestamps_and_emits_ordered_debug_events() {
    let elisp_form = r##"
(let ((amx-last-update-time nil)
      events)
  (cl-letf
      (((symbol-function 'amx-save-history)
        (lambda ()
          (push 'save events)))
       ((symbol-function 'amx-rebuild-cache)
        (lambda ()
          (push 'rebuild events)))
       ((symbol-function 'current-time)
        (lambda ()
          '(26000 12345 678901 0)))
       ((symbol-function 'amx--debug-message)
        (lambda (&rest arguments)
          (push (cons 'debug arguments)
                events))))
    (list
     (amx-update)
     amx-last-update-time
     (nreverse events))))
"##;
    let expect = expect![[
        r#"OK (#1=((debug "Finished full update")) (26000 12345 678901 0) ((debug "Doing full update") save rebuild . #1#))"#
    ]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn idle_update_skips_active_minibuffer_and_recounts_only_after_interval() {
    let elisp_form = r##"
(let ((run
       (lambda (active minibuffer force interval last elapsed)
         (let (events)
           (cl-letf
               (((symbol-function 'amx-active)
                 (lambda () active))
                ((symbol-function 'minibufferp)
                 (lambda (&optional _) minibuffer))
                ((symbol-function 'amx-initialize)
                 (lambda (&rest _)
                   (push 'initialize events)))
                ((symbol-function 'time-since)
                 (lambda (&rest _) 'elapsed))
                ((symbol-function 'float-time)
                 (lambda (&optional _) elapsed))
                ((symbol-function 'amx-update-if-needed)
                 (lambda (&optional recount)
                   (push
                    (list 'update recount)
                    events))))
             (setq amx-auto-update-interval
                   interval
                   amx-last-update-time last)
             (amx-idle-update force)
             (nreverse events))))))
  (list
   (funcall run t t nil 5 '(1) 999)
   (funcall run nil t nil 5 '(1) 999)
   (funcall run nil nil nil nil '(1) 999)
   (funcall run nil nil nil 5 '(1) 299)
   (funcall run nil nil nil 5 '(1) 301)
   (funcall run nil nil t 5 '(1) 0)))
"##;
    let expect = expect![
        "OK (nil (initialize (update t)) (initialize (update nil)) (initialize (update nil)) (initialize (update t)) (initialize (update t)))"
    ];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn auto_update_interval_setter_cancels_old_timer_and_schedules_exact_repeating_delay() {
    let elisp_form = r##"
(let ((amx-test-timer-events nil)
      (amx-long-idle-update-timer
       'old-timer)
      (amx-auto-update-interval nil))
  (amx-set-auto-update-interval
   'amx-auto-update-interval 5)
  (let ((enabled
         (list
          amx-auto-update-interval
          amx-long-idle-update-timer
          (nreverse amx-test-timer-events))))
    (setq amx-test-timer-events nil)
    (amx-set-auto-update-interval
     'amx-auto-update-interval nil)
    (list
     enabled
     amx-auto-update-interval
     amx-long-idle-update-timer
     (nreverse amx-test-timer-events))))
"##;
    let expect = expect![
        "OK ((5 amx-test-timer-2 ((cancel old-timer) (schedule amx-test-timer-2 301 t amx-idle-update nil))) nil nil ((cancel amx-test-timer-2)))"
    ];
    assert_amx_parity(elisp_form, expect);
}

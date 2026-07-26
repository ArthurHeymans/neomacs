use super::assert_achievements_functions_parity;
use expect_test::expect;

#[test]
fn achievements_earned_message_updates_echo_area_and_prepends_log_entries() {
    let elisp_form = r##"(let ((buffer
              (get-buffer-create
               "*achievements-log*"))
             (achievement
              (make-achievement
               "Fixture"
               "Detailed description")))
         (unwind-protect
             (progn
               (with-current-buffer buffer
                 (erase-buffer)
                 (insert "Older entry"))
               (setq
                achievements--test-messages
                nil)
               (cl-letf
                   (((symbol-function 'message)
                     (lambda
                         (format-string
                          &rest arguments)
                       (let ((value
                              (apply
                               #'format
                               format-string
                               arguments)))
                         (push
                          value
                          achievements--test-messages)
                         value))))
                 (list
                  (achievements-earned-message
                   achievement)
                  (nreverse
                   achievements--test-messages)
                  (with-current-buffer buffer
                    (buffer-string))
                  (with-current-buffer buffer
                    (point)))))
           (when
               (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK (nil ("ACHIEVEMENT UNLOCKED: You've earned the `Fixture' achievement!") "You've earned the `Fixture' achievement! [Detailed description]\nOlder entry" 64)"#
    ]];
    assert_achievements_functions_parity(elisp_form, expect);
}

#[test]
fn achievements_tabulated_entries_filter_disabled_and_locked_records_and_hide_unearned_descriptions()
 {
    let elisp_form = r##"(let* ((earned
                  (make-achievement
                   "Earned"
                   "Visible earned"
                   :points 5))
                 (unearned
                  (make-achievement
                   "Unearned"
                   "Hidden until earned"
                   :points 7))
                 (disabled
                  (make-achievement
                   "Disabled"
                   "Never listed"
                   :points 9))
                 (locked
                  (make-achievement
                   "Locked"
                   "High score"
                   :points 11
                   :min-score 50))
                 (achievements-score 10)
                 (achievements-list
                  (list
                   earned
                   unearned
                   disabled
                   locked)))
         (setf
          (emacs-achievement-predicate
           earned)
          t
          (emacs-achievement-predicate
           unearned)
          (lambda () nil)
          (emacs-achievement-predicate
           disabled)
          nil
          (emacs-achievement-predicate
           locked)
          t)
         (mapcar
          (lambda (entry)
            (list
             (car entry)
             (append
              (cadr entry)
              nil)))
          (achievements-tabulated-list-entries)))"##;
    let expect = expect![[
        r#"OK (("Earned" ("✓" "5" "Earned" "Visible earned")) ("Unearned" ("" "7" "Unearned" "")))"#
    ]];
    assert_achievements_functions_parity(elisp_form, expect);
}

#[test]
fn achievements_disable_covers_missing_declined_and_confirmed_rows() {
    let elisp_form = r##"(let* ((achievement
                  (make-achievement
                   "Fixture"
                   "Description"))
                 (achievements-list
                  (list achievement)))
         (mapcar
          (lambda (fixture)
            (setf
             (emacs-achievement-predicate
              achievement)
             (lambda () t))
            (setq
             achievements--test-events
             nil)
            (cl-letf
                (((symbol-function
                   'tabulated-list-get-id)
                  (lambda ()
                    (car fixture)))
                 ((symbol-function
                   'y-or-n-p)
                  (lambda (prompt)
                    (push
                     (list 'prompt prompt)
                     achievements--test-events)
                    (cadr fixture)))
                 ((symbol-function
                   'revert-buffer)
                  (lambda (&rest arguments)
                    (push
                     (cons
                      'revert
                      arguments)
                     achievements--test-events)
                    'reverted)))
              (list
               fixture
               (achievements-disable)
               (cond
                ((null
                  (emacs-achievement-predicate
                   achievement))
                 'disabled)
                (t 'enabled))
               (nreverse
                achievements--test-events))))
          '(("Missing" t)
            ("Fixture" nil)
            ("Fixture" t))))"##;
    let expect = expect![[
        r#"OK ((("Missing" t) nil enabled nil) (("Fixture" nil) nil enabled ((prompt "Do you really want to disable this achievement? "))) (("Fixture" t) reverted disabled ((prompt "Do you really want to disable this achievement? ") (revert))))"#
    ]];
    assert_achievements_functions_parity(elisp_form, expect);
}

#[test]
fn achievements_list_command_creates_mode_updates_score_and_prints_in_order() {
    let elisp_form = r##"(progn
         (setq
          achievements--test-events
          nil)
         (cl-letf
             (((symbol-function
                'pop-to-buffer)
               (lambda (buffer)
                 (push
                  (list
                   'pop-to-buffer
                   buffer)
                  achievements--test-events)
                 'fixture-buffer))
              ((symbol-function
                'achievements-list-mode)
               (lambda ()
                 (push
                  '(list-mode)
                  achievements--test-events)
                 'mode-result))
              ((symbol-function
                'achievements-update-score)
               (lambda ()
                 (push
                  '(update-score)
                  achievements--test-events)
                 'score-result))
              ((symbol-function
                'tabulated-list-print)
               (lambda
                   (&optional remember-position
                    update)
                 (push
                  (list
                   'print
                   remember-position
                   update)
                  achievements--test-events)
                 'print-result)))
           (list
            (achievements-list-achievements)
            (nreverse
             achievements--test-events))))"##;
    let expect = expect![[
        r#"OK (print-result ((pop-to-buffer "*Achievements*") (list-mode) (update-score) (print t nil)))"#
    ]];
    assert_achievements_functions_parity(elisp_form, expect);
}

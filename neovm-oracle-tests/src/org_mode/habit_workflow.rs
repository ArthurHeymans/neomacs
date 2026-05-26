use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_habit_parse_urgency_faces_graph_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-habit)
  (with-temp-buffer
    (let ((org-habit-preceding-days 5)
          (org-habit-following-days 4)
          (org-habit-today-glyph ?!)
          (org-habit-completed-glyph ?*))
      (org-mode)
      (insert "* TODO Run\n")
      (insert "SCHEDULED: <2026-05-24 Sun .+2d/5d>\n")
      (insert ":PROPERTIES:\n:STYLE: habit\n:END:\n")
      (insert ":LOGBOOK:\n")
      (insert "- State \"DONE\" from \"TODO\" [2026-05-20 Wed]\n")
      (insert "- State \"DONE\" from \"TODO\" [2026-05-23 Sat]\n")
      (insert ":END:\n")
      (goto-char (point-min))
      (let* ((habit (org-habit-parse-todo))
             (today (encode-time 0 0 12 27 5 2026))
             (graph (org-habit-build-graph
                     habit
                     (encode-time 0 0 12 22 5 2026)
                     today
                     (encode-time 0 0 12 31 5 2026))))
        (list
         habit
         (org-habit-get-urgency habit today)
         (mapcar (lambda (offset)
                   (org-habit-get-faces
                    habit
                    (+ (time-to-days today) offset)))
                 '(-4 -2 0 2 5))
         graph
         (mapcar (lambda (i)
                   (list (aref graph i)
                         (get-text-property i 'face graph)
                         (get-text-property i 'help-echo graph)))
                 (number-sequence 0 (1- (length graph)))))))))"#,
    );
}

#[test]
fn org_habit_repeater_types_shift_graph_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-habit)
  (with-temp-buffer
    (let ((org-habit-preceding-days 4)
          (org-habit-following-days 3))
      (org-mode)
      (insert "* TODO Plus\nSCHEDULED: <2026-05-20 Wed +3d/6d>\n")
      (insert ":PROPERTIES:\n:STYLE: habit\n:END:\n")
      (insert "- State \"DONE\" from \"TODO\" [2026-05-21 Thu]\n")
      (insert "- State \"DONE\" from \"TODO\" [2026-05-25 Mon]\n")
      (insert "* TODO Double\nSCHEDULED: <2026-05-20 Wed ++3d/6d>\n")
      (insert ":PROPERTIES:\n:STYLE: habit\n:END:\n")
      (insert "- State \"DONE\" from \"TODO\" [2026-05-21 Thu]\n")
      (insert "- State \"DONE\" from \"TODO\" [2026-05-25 Mon]\n")
      (let (out)
        (goto-char (point-min))
        (while (re-search-forward "^\\* TODO" nil t)
          (beginning-of-line)
          (let* ((habit (org-habit-parse-todo))
                 (graph (org-habit-build-graph
                         habit
                         (encode-time 0 0 12 22 5 2026)
                         (encode-time 0 0 12 26 5 2026)
                         (encode-time 0 0 12 29 5 2026))))
            (push (list (org-get-heading t t t t)
                        habit
                        (org-habit-get-urgency
                         habit
                         (encode-time 0 0 12 26 5 2026))
                        graph
                        (mapcar (lambda (i)
                                  (get-text-property i 'face graph))
                                (number-sequence 0 (1- (length graph)))))
                  out))
          (forward-line 1))
        (nreverse out)))))"#,
    );
}

#[test]
fn org_habit_invalid_repeater_errors_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-habit)
  (with-temp-buffer
    (org-mode)
    (insert "* TODO Missing schedule\n:PROPERTIES:\n:STYLE: habit\n:END:\n")
    (insert "* TODO Missing repeat\nSCHEDULED: <2026-05-27 Wed>\n")
    (insert ":PROPERTIES:\n:STYLE: habit\n:END:\n")
    (insert "* TODO Bad deadline\nSCHEDULED: <2026-05-27 Wed .+2d/2d>\n")
    (insert ":PROPERTIES:\n:STYLE: habit\n:END:\n")
    (let (out)
      (goto-char (point-min))
      (while (re-search-forward "^\\* TODO" nil t)
        (beginning-of-line)
        (push
         (list
          (org-get-heading t t t t)
          (condition-case err
              (org-habit-parse-todo)
            (error (list (car err) (cadr err)))))
         out)
        (forward-line 1))
      (nreverse out))))"#,
    );
}

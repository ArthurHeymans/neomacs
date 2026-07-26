use super::assert_achievements_functions_parity;
use expect_test::expect;

#[test]
fn achievements_earned_predicate_handles_literals_functions_and_errors() {
    let elisp_form = r##"(progn
         (setq
          achievements--test-messages
          nil)
         (cl-letf
             (((symbol-function 'message)
               (lambda
                   (format-string
                    &rest arguments)
                 (push
                  (apply
                   #'format
                   format-string
                   arguments)
                  achievements--test-messages))))
           (mapcar
            (lambda (fixture)
              (let ((achievement
                     (make-achievement
                      (car fixture)
                      "Fixture")))
                (setf
                 (emacs-achievement-predicate
                  achievement)
                 (cadr fixture))
                (setq
                 achievements--test-messages
                 nil)
                (list
                 (car fixture)
                 (achievements-earned-p
                  achievement)
                 (nreverse
                  achievements--test-messages))))
            (list
             (list "literal true" t)
             (list "literal nil" nil)
             (list "non-function"
                   'not-callable)
             (list "function true"
                   (lambda () 'earned))
             (list "function false"
                   (lambda () nil))
             (list "function error"
                   (lambda ()
                     (error
                      "predicate boom")))))))"##;
    let expect = expect![[
        r#"OK (("literal true" t nil) ("literal nil" nil nil) ("non-function" nil nil) ("function true" earned nil) ("function false" nil nil) ("function error" #1=("Error while checking if you have earned the function error achievement: (error predicate boom)") #1#))"#
    ]];
    assert_achievements_functions_parity(elisp_form, expect);
}

#[test]
fn achievements_update_score_totals_mutates_persistent_records_and_unlocks_features() {
    let elisp_form = r##"(let* ((persistent
                  (make-achievement
                   "Persistent"
                   "Earned"
                   :points 5
                   :unlocks
                   'advanced-fixture))
                 (transient
                  (make-achievement
                   "Transient"
                   "Earned repeatedly"
                   :points 3
                   :transient t))
                 (unearned
                  (make-achievement
                   "Unearned"
                   "Not yet"
                   :points 2))
                 (already
                  (make-achievement
                   "Already"
                   "Saved"
                   :points 4))
                 (broken
                  (make-achievement
                   "Broken"
                   "Signals"
                   :points 6))
                 (achievements-list
                  (list
                   persistent
                   transient
                   unearned
                   already
                   broken))
                 (achievements-score 99)
                 (achievements-total 99)
                 (achievements-display-when-earned
                  t))
         (setf
          (emacs-achievement-predicate
           persistent)
          (lambda () t)
          (emacs-achievement-predicate
           transient)
          (lambda () t)
          (emacs-achievement-predicate
           unearned)
          (lambda () nil)
          (emacs-achievement-predicate
           already)
          t
          (emacs-achievement-predicate
           broken)
          (lambda ()
            (error "broken predicate")))
         (setq
          achievements--test-events
          nil)
         (cl-letf
             (((symbol-function
                'achievements-earned-message)
               (lambda (achievement)
                 (push
                  (list
                   'earned-message
                   (emacs-achievement-name
                    achievement))
                  achievements--test-events)))
              ((symbol-function
                'achievements-save-achievements)
               (lambda ()
                 (push '(save)
                       achievements--test-events)
                 'saved))
              ((symbol-function 'require)
               (lambda
                   (feature
                    &optional filename
                    noerror)
                 (push
                  (list
                   'require
                   feature
                   filename
                   noerror)
                  achievements--test-events)
                 feature))
              ((symbol-function 'message)
               (lambda
                   (format-string
                    &rest arguments)
                 (push
                  (list
                   'message
                   (apply
                    #'format
                    format-string
                    arguments))
                  achievements--test-events))))
           (list
            (achievements-update-score)
            achievements-score
            achievements-total
            (mapcar
             (lambda (achievement)
               (let ((predicate
                      (emacs-achievement-predicate
                       achievement)))
                 (cond
                  ((eq predicate t)
                   'earned)
                  ((functionp predicate)
                   'function)
                  (t predicate))))
             achievements-list)
            (nreverse
             achievements--test-events))))"##;
    let expect = expect![[
        r#"OK (18 18 20 (earned function function earned earned) ((require advanced-fixture nil t) (earned-message "Persistent") (message "Error while checking if you have earned the Broken achievement: (error broken predicate)") (earned-message "Broken") (save)))"#
    ]];
    assert_achievements_functions_parity(elisp_form, expect);
}

#[test]
fn achievements_update_score_suppresses_earned_notifications_when_configured() {
    let elisp_form = r##"(let* ((achievement
                  (make-achievement
                   "Silent"
                   "Earned"
                   :points 8))
                 (achievements-list
                  (list achievement))
                 (achievements-display-when-earned
                  nil)
                 (achievements-score 0)
                 (achievements-total 0))
         (setf
          (emacs-achievement-predicate
           achievement)
          (lambda () t))
         (setq
          achievements--test-events
          nil)
         (cl-letf
             (((symbol-function
                'achievements-earned-message)
               (lambda (_achievement)
                 (push '(message)
                       achievements--test-events)))
              ((symbol-function
                'achievements-save-achievements)
               (lambda ()
                 (push '(save)
                       achievements--test-events))))
           (list
            (achievements-update-score)
            achievements-score
            achievements-total
            (emacs-achievement-predicate
             achievement)
            (nreverse
             achievements--test-events))))"##;
    let expect = expect!["OK (8 8 8 t ((save)))"];
    assert_achievements_functions_parity(elisp_form, expect);
}

#[test]
fn achievements_get_by_name_returns_first_match_and_nil_for_missing() {
    let elisp_form = r##"(let* ((first
                  (make-achievement
                   "Duplicate"
                   "First"
                   :points 1))
                 (second
                  (make-achievement
                   "Duplicate"
                   "Second"
                   :points 2))
                 (other
                  (make-achievement
                   "Other"
                   "Other"
                   :points 3))
                 (achievements-list
                  (list first
                        second
                        other)))
         (mapcar
          (lambda (name)
            (let ((result
                   (achievements-get-achievements-by-name
                    name)))
              (and result
                   (list
                    (emacs-achievement-description
                     result)
                    (emacs-achievement-points
                     result)))))
          '("Duplicate"
            "Other"
            "Missing")))"##;
    let expect = expect![[r#"OK (("First" 1) ("Other" 3) nil)"#]];
    assert_achievements_functions_parity(elisp_form, expect);
}

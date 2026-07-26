use super::assert_achievements_functions_parity;
use expect_test::expect;

#[test]
fn achievements_constructor_defaults_slots_and_convenience_predicates_match() {
    let elisp_form = r##"(progn
         (setq
          achievements--test-events
          nil
          achievements--test-variable
          7)
         (cl-letf
             (((symbol-function 'featurep)
               (lambda (feature)
                 (push
                  (list 'feature feature)
                  achievements--test-events)
                 (eq feature
                     'available-feature)))
              ((symbol-function
                'achievements-variable-was-set)
               (lambda (variable)
                 (push
                  (list 'variable variable)
                  achievements--test-events)
                 t))
              ((symbol-function
                'achievements-command-was-run)
               (lambda (command)
                 (push
                  (list 'command command)
                  achievements--test-events)
                 t)))
           (mapcar
            (lambda (achievement)
              (list
               (emacs-achievement-name
                achievement)
               (emacs-achievement-description
                achievement)
               (emacs-achievement-points
                achievement)
               (emacs-achievement-transient
                achievement)
               (emacs-achievement-min-score
                achievement)
               (emacs-achievement-unlocks
                achievement)
               (and
                (emacs-achievement-post-command
                 achievement)
                t)
               (funcall
                (emacs-achievement-predicate
                 achievement))
               (nreverse
                (prog1
                    achievements--test-events
                  (setq
                   achievements--test-events
                   nil)))))
            (list
             (make-achievement
              "defaults"
              "Default description")
             (make-achievement
              "slots"
              nil
              :points 13
              :transient t
              :min-score 21
              :unlocks 'next-feature
              :predicate
              '(= achievements--test-variable
                  7))
             (make-achievement
              "convenience"
              "Combined"
              :package
              'available-feature
              :variable
              '(fixture-variable 9)
              :command
              '(fixture-command . 3))
             (make-achievement
              "post-command"
              "Post"
              :post-command
              (lambda () t))))))"##;
    let expect = expect![[
        r#"OK (("defaults" "Default description" 5 nil 0 nil nil t nil) ("slots" nil 13 t 21 next-feature nil t nil) ("convenience" "Combined" 5 nil 0 nil nil t ((feature available-feature) (variable (fixture-variable 9)) (command (fixture-command . 3)))) ("post-command" "Post" 5 nil 0 nil t nil nil))"#
    ]];
    assert_achievements_functions_parity(elisp_form, expect);
}

#[test]
fn achievements_constructor_is_read_only_only_for_name_and_other_slots_mutate() {
    let elisp_form = r##"(let ((achievement
              (make-achievement
               "Original"
               "Before")))
         (list
          (condition-case error
              (progn
                (setf
                 (emacs-achievement-name
                  achievement)
                 "Changed")
                'changed)
            (error
             (list 'error error)))
          (progn
            (setf
             (emacs-achievement-description
              achievement)
             "After"
             (emacs-achievement-points
              achievement)
             8
             (emacs-achievement-predicate
              achievement)
             t)
            (list
             (emacs-achievement-name
              achievement)
             (emacs-achievement-description
              achievement)
             (emacs-achievement-points
              achievement)
             (emacs-achievement-predicate
              achievement)))))"##;
    let expect = expect![[
        r#"OK ((error (error "emacs-achievement-name is a read-only slot")) ("Original" "After" 8 t))"#
    ]];
    assert_achievements_functions_parity(elisp_form, expect);
}

#[test]
fn achievements_defachievement_appends_and_deduplicates_by_name() {
    let elisp_form = r##"(let ((achievements-list nil))
         (defachievement
          "Alpha"
          "First"
          :points 1)
         (defachievement
          "Beta"
          :points 2)
         (defachievement
          "Alpha"
          "Replacement"
          :points 99)
         (mapcar
          (lambda (achievement)
            (list
             (emacs-achievement-name
              achievement)
             (emacs-achievement-description
              achievement)
             (emacs-achievement-points
              achievement)
             (funcall
              (emacs-achievement-predicate
               achievement))))
          achievements-list))"##;
    let expect = expect![[r#"OK (("Alpha" "First" 1 t) ("Beta" nil 2 t))"#]];
    assert_achievements_functions_parity(elisp_form, expect);
}

#[test]
fn achievements_defachievement_macroexpansion_contract_matches() {
    let elisp_form = r##"(macroexpand-1
         '(defachievement
           "Fixture"
           "Description"
           :points 17
           :predicate
           '(> fixture 2)))"##;
    let expect = expect![[
        r#"OK (add-to-list 'achievements-list (make-achievement "Fixture" "Description" :points 17 :predicate '(> fixture 2)) t (lambda (a b) (equal (emacs-achievement-name a) (emacs-achievement-name b))))"#
    ]];
    assert_achievements_functions_parity(elisp_form, expect);
}

#[test]
fn achievements_defcommand_achievements_generates_exact_records_and_predicates() {
    let elisp_form = r##"(let ((achievements-list nil))
         (defcommand-achievements
           "Used `%s' for %s."
           ((fixture-one
             "One"
             "editing")
            ((fixture-two
              fixture-three)
             "Either"
             "navigation"))
           :points 9
           :min-score 4)
         (setq
          achievements--test-events
          nil)
         (cl-letf
             (((symbol-function
                'achievements-command-was-run)
               (lambda (command)
                 (push command
                       achievements--test-events)
                 (eq command
                     'fixture-one))))
           (mapcar
            (lambda (achievement)
              (setq
               achievements--test-events
               nil)
              (list
               (emacs-achievement-name
                achievement)
               (emacs-achievement-description
                achievement)
               (emacs-achievement-points
                achievement)
               (emacs-achievement-min-score
                achievement)
               (funcall
                (emacs-achievement-predicate
                 achievement))
               (nreverse
                achievements--test-events)))
            achievements-list)))"##;
    let expect = expect![[
        r#"OK (("One" "Used `fixture-one' for (editing)." 9 4 t (fixture-one)) ("Either" "Used `(fixture-two fixture-three)' for (navigation)." 9 4 nil ((fixture-two fixture-three))))"#
    ]];
    assert_achievements_functions_parity(elisp_form, expect);
}

#[test]
fn achievements_defvalue_achievements_uses_override_labels_and_exact_values() {
    let elisp_form = r##"(let ((achievements-list nil))
         (defvalue-achievements
           fixture-option
           "Selected %s with `%s'."
           (("Default label" alpha)
            ("Override label" beta
             "Rendered beta"))
           :points 3)
         (setq
          achievements--test-events
          nil)
         (cl-letf
             (((symbol-function
                'achievements-variable-was-set)
               (lambda (variable)
                 (push variable
                       achievements--test-events)
                 (equal
                  variable
                  '(fixture-option
                    beta)))))
           (mapcar
            (lambda (achievement)
              (setq
               achievements--test-events
               nil)
              (list
               (emacs-achievement-name
                achievement)
               (emacs-achievement-description
                achievement)
               (emacs-achievement-points
                achievement)
               (funcall
                (emacs-achievement-predicate
                 achievement))
               (nreverse
                achievements--test-events)))
            achievements-list)))"##;
    let expect = expect![[
        r#"OK (("Default label" "Selected alpha with `fixture-option'." 3 nil ((fixture-option alpha))) ("Override label" "Selected Rendered beta with `fixture-option'." 3 t ((fixture-option beta))))"#
    ]];
    assert_achievements_functions_parity(elisp_form, expect);
}

use expect_test::expect;

use super::assert_attrap_parity;

#[test]
fn attrap_option_captures_match_data_and_applies_a_delayed_multiform_buffer_repair() {
    let elisp_form = r##"(with-temp-buffer
          (insert "alpha beta gamma")
          (goto-char
           (point-min))
          (re-search-forward
           "\\(alpha\\) \\(beta\\)")
          (let* ((option
                  (attrap-option
                      (list
                       'replace
                       (match-string 2)
                       'after
                       (match-string 1))
                    (goto-char
                     (match-beginning 2))
                    (replace-match
                     "BETA"
                     nil
                     nil
                     nil
                     2)
                    (list
                     :applied
                     (match-string 1)
                     (buffer-string)
                     (point))))
                 (shape
                  (list
                   (copy-tree
                    (car option))
                   (functionp
                    (cdr option))))
                 (captured-match
                  (match-data t)))
            (string-match
             "\\(unrelated\\)"
             "unrelated")
            (goto-char
             (point-max))
            (let ((result
                   (funcall
                    (cdr option))))
              (list
               shape
               captured-match
               result
               (buffer-string)
               (point)
               (match-string 1)))))"##;
    let expect = expect![[
        r#"OK (((replace "beta" after "alpha") t) (1 11 1 6 7 11 (:buffer nil)) (:applied "alpha" "alpha BETA gamma" 11) "alpha BETA gamma" 11 "alpha")"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_one_option_builds_one_callable_repair_and_preserves_description_identity() {
    let elisp_form = r##"(with-temp-buffer
          (insert "prefix payload suffix")
          (goto-char
           (point-min))
          (search-forward "payload")
          (let* ((description
                  (list
                   'replace
                   'payload))
                 (options
                  (attrap-one-option
                      description
                    (replace-match "fixed")
                    (list
                     :done
                     (buffer-string)))))
            (list
             (length options)
             (eq
              description
              (caar options))
             (attrap-test-option-shape
              options)
             (funcall
              (cdar options))
             (buffer-string))))"##;
    let expect = expect![[
        r#"OK (1 t (((replace payload) t)) (:done "prefix fixed suffix") "prefix fixed suffix")"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_alternatives_evaluates_conditions_in_order_and_appends_only_successful_lists() {
    let elisp_form = r##"(let (events)
          (list
           (attrap-alternatives
            ((progn
               (push :condition-a events)
               t)
             (push :body-a events)
             '((a . repair-a)))
            ((progn
               (push :condition-b events)
               nil)
             (push :unexpected-body-b events)
             '((b . repair-b)))
            ((progn
               (push :condition-c events)
               :truthy)
             (push :body-c-1 events)
             (push :body-c-2 events)
             '((c . repair-c)
               (d . repair-d)))
            ((progn
               (push :condition-d events)
               t)
             nil))
           (nreverse events)))"##;
    let expect = expect![
        "OK (((a . repair-a) (c . repair-c) (d . repair-d)) (:condition-a :body-a :condition-b :condition-c :body-c-1 :body-c-2 :condition-d))"
    ];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_alternatives_propagates_condition_and_body_errors_without_running_later_clauses() {
    let elisp_form = r##"(mapcar
          (lambda (failure)
            (let (events)
              (list
               failure
               (attrap-test-error-data
                (lambda ()
                  (attrap-alternatives
                   ((progn
                      (push :condition-a events)
                      (when
                          (eq failure 'condition)
                        (error
                         "condition failed"))
                      t)
                    (push :body-a events)
                    (when
                        (eq failure 'body)
                      (error
                       "body failed"))
                    '((a . repair)))
                   ((progn
                      (push :condition-b events)
                      t)
                    (push :body-b events)
                    '((b . repair))))))
               (nreverse events))))
          '(none condition body))"##;
    let expect = expect![[
        r#"OK ((none (:ok ((a . repair) (b . repair))) (:condition-a :body-a :condition-b :body-b)) (condition (:error error ("condition failed")) (:condition-a)) (body (:error error ("body failed")) (:condition-a :body-a)))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_insert_language_pragma_and_operator_parenthesizing_apply_real_haskell_edits() {
    let elisp_form = r##"(list
          (mapcar
           (lambda (name)
             (list
              name
              (attrap-add-operator-parens
               name)))
           '("map"
             "_private"
             "Type'"
             "+"
             ">>="
             ":*:"))
          (with-temp-buffer
            (insert
             "module Demo where\n\nvalue = 1\n")
            (goto-char
             (point-max))
            (let ((option
                   (attrap-insert-language-pragma
                    "LambdaCase")))
              (list
               (attrap-test-option-shape
                (list option))
               (funcall
                (cdr option))
               (buffer-string)
               (point)))))"##;
    let expect = expect![[
        r#"OK ((("map" "map") ("_private" "_private") ("Type'" "Type'") ("+" "(+)") (">>=" "(>>=)") (":*:" "(:*:)")) ((((use-extension "LambdaCase") t)) nil "{-# LANGUAGE LambdaCase #-}\nmodule Demo where\n\nvalue = 1\n" 29))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_add_to_import_handles_empty_existing_and_operator_import_lists() {
    let elisp_form = r##"(mapcar
          (lambda (case)
            (pcase-let
                ((`(,missing ,contents ,line ,column)
                  case))
              (with-temp-buffer
                (insert contents)
                (goto-char
                 (point-min))
                (let ((option
                       (attrap-add-to-import
                        missing
                        "Data.Map"
                        line
                        column)))
                  (list
                   missing
                   (car option)
                   (funcall
                    (cdr option))
                   (buffer-string)
                   (point))))))
          '(("lookup"
             "import Data.Map ()\nmain = pure ()\n"
             "1"
             "18")
            ("insert"
             "import Data.Map (lookup )\nmain = pure ()\n"
             "1"
             "25")
            ("++"
             "import Data.Map (lookup)\nmain = pure ()\n"
             "1"
             "24")
            (">>="
             "import Data.Map\n  ( lookup\n  )\nmain = pure ()\n"
             "3"
             "4")))"##;
    let expect = expect![[
        r#"OK (("lookup" (add-to-import-list "Data.Map") nil "import Data.Map (lookup)\nmain = pure ()\n" 24) ("insert" (add-to-import-list "Data.Map") nil "import Data.Map (lookup,insert )\nmain = pure ()\n" 31) ("++" (add-to-import-list "Data.Map") nil "import Data.Map (lookup,(++))\nmain = pure ()\n" 29) (">>=" (add-to-import-list "Data.Map") nil "import Data.Map\n  ( lookup\n  ),(>>=)\nmain = pure ()\n" 37))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_select_and_apply_option_rejects_empty_and_applies_single_without_completion() {
    let elisp_form = r##"(with-temp-buffer
          (insert "alpha payload omega")
          (goto-char
           (point-min))
          (search-forward "payload")
          (let ((original-point
                 (point))
                completion-called)
            (cl-letf
                (((symbol-function
                   'completing-read)
                  (lambda (&rest _arguments)
                    (setq completion-called t)
                    (error
                     "completion should not run"))))
              (list
               (attrap-test-error-data
                (lambda ()
                  (attrap-select-and-apply-option
                   nil)))
               (attrap-select-and-apply-option
                (list
                 (cons
                  '(replace payload)
                  (lambda ()
                    (replace-match "fixed")
                    :single-result))))
               completion-called
               (buffer-string)
               (=
                original-point
                (point))))))"##;
    let expect = expect![[
        r#"OK ((:error error ("No fixer applies to the issue at point")) :single-result nil "alpha fixed omega" nil)"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_select_and_apply_option_formats_multiple_choices_and_applies_selected_repair() {
    let elisp_form = r##"(with-temp-buffer
          (insert "one target three")
          (goto-char
           (point-min))
          (search-forward "target")
          (let ((original-point
                 (point))
                prompt
                collection
                predicate
                require-match)
            (cl-letf
                (((symbol-function
                   'completing-read)
                  (lambda
                    (actual-prompt
                     actual-collection
                     actual-predicate
                     actual-require-match
                     &rest _arguments)
                    (setq prompt actual-prompt
                          collection
                          (mapcar
                           (lambda (entry)
                             (list
                              (car entry)
                              (functionp
                               (cdr entry))))
                           actual-collection)
                          predicate actual-predicate
                          require-match
                          actual-require-match)
                    "(replace target by second)")))
              (let ((result
                     (attrap-select-and-apply-option
                      (list
                       (cons
                        '(replace target by first)
                        (lambda ()
                          (replace-match "FIRST")
                          :first))
                       (cons
                        '(replace target by second)
                        (lambda ()
                          (replace-match "SECOND")
                          :second))))))
                (list
                 result
                 prompt
                 collection
                 predicate
                 require-match
                 (buffer-string)
                 (=
                  original-point
                  (point)))))))"##;
    let expect = expect![[
        r#"OK (:second "repair using: " (("(replace target by first)" t) ("(replace target by second)" t)) nil t "one SECOND three" t)"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_select_and_apply_option_uses_first_duplicate_and_propagates_repair_failures() {
    let elisp_form = r##"(mapcar
          (lambda (scenario)
            (let (events)
              (cl-letf
                  (((symbol-function
                     'completing-read)
                    (lambda (&rest _arguments)
                      (if
                          (eq scenario 'missing)
                          "missing"
                        "duplicate"))))
                (list
                 scenario
                 (attrap-test-error-data
                  (lambda ()
                    (attrap-select-and-apply-option
                     (list
                      (cons
                       'duplicate
                       (lambda ()
                         (push :first events)
                         (when
                             (eq scenario 'repair-error)
                           (error
                            "repair failed"))
                         :first))
                      (cons
                       "duplicate"
                       (lambda ()
                         (push :second events)
                         :second))))))
                 (nreverse events)))))
          '(duplicate repair-error missing))"##;
    let expect = expect![[
        r#"OK ((duplicate (:ok :first) (:first)) (repair-error (:error error ("repair failed")) (:first)) (missing (:error void-function (nil)) nil))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

use expect_test::expect;

use super::{assert_academic_phrases_parity, assert_academic_phrases_signal_parity};

#[test]
fn academic_phrases_insert_drives_exact_prompts_and_replaces_three_choices() {
    let elisp_form = r##"(let* ((item
                     (ht
                      (:id 99)
                      (:template "A [{1}] B [{2}] C [{3}]")
                      (:choices
                       '(("alpha"
                          "aleph")
                         ("beta")
                         ("gamma"
                          "gimel")))))
                    (fixture
                     (ht
                      (:fixture
                       (ht
                        (:title "Fixture")
                        (:items
                         (list
                          item))))))
                    (academic-phrases--all-phrases
                     fixture)
                    (responses
                     '("Fixture"
                       "A [alpha/aleph] B [beta] C [gamma/gimel]"
                       "aleph"
                       "beta"
                       "gimel"))
                    calls)
               (cl-letf
                   (((symbol-function
                      'completing-read)
                     (lambda (prompt collection
                              &optional predicate require-match
                              initial-input history default
                              inherit-input-method)
                       (push
                        (list
                         prompt
                         (copy-tree
                          collection)
                         predicate
                         require-match
                         initial-input
                         history
                         default
                         inherit-input-method)
                        calls)
                       (pop
                        responses))))
                 (with-temp-buffer
                   (let ((result
                          (academic-phrases--insert
                           fixture)))
                     (list
                      result
                      (buffer-string)
                      (nreverse
                       calls)
                      responses
                      (eq
                       item
                       (academic-phrases--filter-item
                        :fixture
                        99
                        fixture)))))))"##;
    let expect = expect![[
        r#"OK (nil "A aleph B beta C gimel" (("Choose a category: " ("Fixture") nil t nil nil nil nil) ("Choose a phrase: " (("A [alpha/aleph] B [beta] C [gamma/gimel]" . 99)) nil t nil nil nil nil) ("A [alpha/aleph] B [beta] C [gamma/gimel]" ("alpha" "aleph") nil t nil nil nil nil) ("A [alpha/aleph] B [beta] C [gamma/gimel]" ("beta") nil t nil nil nil nil) ("A [alpha/aleph] B [beta] C [gamma/gimel]" ("gamma" "gimel") nil t nil nil nil nil)) nil t)"#
    ]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_insert_skips_choice_prompts_for_literal_templates() {
    let elisp_form = r##"(let* ((item
                     (ht
                      (:id 5)
                      (:template "A literal academic phrase.")
                      (:choices
                       '(()))))
                    (fixture
                     (ht
                      (:fixture
                       (ht
                        (:title "Fixture")
                        (:items
                         (list
                          item))))))
                    (academic-phrases--all-phrases
                     fixture)
                    (responses
                     '("Fixture"
                       "A literal academic phrase."))
                    calls)
               (cl-letf
                   (((symbol-function
                      'completing-read)
                     (lambda (prompt collection
                              &optional predicate require-match
                              &rest ignored)
                       (push
                        (list
                         prompt
                         (copy-tree
                          collection)
                         predicate
                         require-match
                         ignored)
                        calls)
                       (pop
                        responses))))
                 (with-temp-buffer
                   (insert
                    "prefix:")
                   (let ((result
                          (academic-phrases--insert
                           fixture)))
                     (list
                      result
                      (point)
                      (buffer-string)
                      (nreverse
                       calls)
                      responses)))))"##;
    let expect = expect![[
        r#"OK (nil 34 "prefix:A literal academic phrase." (("Choose a category: " ("Fixture") nil t nil) ("Choose a phrase: " (("A literal academic phrase." . 5)) nil t nil)) nil)"#
    ]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_insert_uses_the_passed_categories_but_leaks_item_lookup_to_global_data() {
    let elisp_form = r##"(let* ((passed-item
                     (ht
                      (:id 1)
                      (:template "Passed [{1}]")
                      (:choices
                       '(("passed-choice")))))
                    (global-item
                     (ht
                      (:id 2)
                      (:template "Global [{1}]")
                      (:choices
                       '(("global-choice")))))
                    (passed
                     (ht
                      (:shared
                       (ht
                        (:title "Passed Category")
                        (:items
                         (list
                          passed-item))))))
                    (academic-phrases--all-phrases
                     (ht
                      (:shared
                       (ht
                        (:title "Global Category")
                        (:items
                         (list
                          global-item))))))
                    (responses
                     '("Passed Category"
                       "Global [global-choice]"
                       "global-choice"))
                    calls)
               (cl-letf
                   (((symbol-function
                      'completing-read)
                     (lambda (prompt collection
                              &optional predicate require-match
                              &rest ignored)
                       (push
                        (list
                         prompt
                         (copy-tree
                          collection)
                         predicate
                         require-match
                         ignored)
                        calls)
                       (pop
                        responses))))
                 (with-temp-buffer
                   (let ((result
                          (academic-phrases--insert
                           passed)))
                     (list
                      result
                      (buffer-string)
                      (nreverse
                       calls)
                      responses
                      (eq
                       passed-item
                       (car
                        (academic-phrases--get-items
                         :shared
                         passed)))
                      (eq
                       global-item
                       (car
                        (academic-phrases--get-items
                         :shared))))))))"##;
    let expect = expect![[
        r#"OK (nil "Global global-choice" (("Choose a category: " ("Passed Category") nil t nil) ("Choose a phrase: " (("Global [global-choice]" . 2)) nil t nil) ("Global [global-choice]" ("global-choice") nil t nil)) nil t t)"#
    ]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_insert_surfaces_invalid_category_responses_before_mutating_the_buffer() {
    let elisp_form = r##"(let* ((fixture
                     (ht
                      (:fixture
                       (ht
                        (:title "Fixture")
                        (:items nil)))))
                    (academic-phrases--all-phrases
                     fixture))
               (cl-letf
                   (((symbol-function
                      'completing-read)
                     (lambda (&rest _)
                       "Not a category")))
                 (academic-phrases--insert
                  fixture)))"##;
    let expect = expect!["ERR (wrong-type-argument hash-table-p nil)"];

    assert_academic_phrases_signal_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_topic_command_forwards_the_live_global_table_and_interactive_call() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'academic-phrases--insert)
                     (lambda (phrases)
                       (push
                        (list
                         (eq
                          phrases
                          academic-phrases--all-phrases)
                         (hash-table-count
                          phrases))
                        calls)
                       'inserted)))
                 (list
                  (academic-phrases)
                  (call-interactively
                   #'academic-phrases)
                  (nreverse
                   calls)
                  (interactive-form
                   'academic-phrases))))"##;
    let expect = expect!["OK (inserted inserted ((t 57) (t 57)) (interactive nil))"];

    assert_academic_phrases_parity(elisp_form, expect);
}

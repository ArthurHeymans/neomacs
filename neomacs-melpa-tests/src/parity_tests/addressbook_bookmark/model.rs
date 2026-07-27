use expect_test::expect;

use super::assert_addressbook_bookmark_parity;

#[test]
fn addressbook_bookmark_predicates_cover_records_names_missing_and_malformed_entries() {
    let elisp_form = r##"(let ((bookmark-alist
                '(("Ada" (type . "addressbook") (email . "ada@example.test"))
                  ("File" (type . "file") (filename . "/work/file"))
                  ("Missing type" (email . "none@example.test")))))
         (mapcar
          (lambda (value)
            (list
             value
             (condition-case error-data
                 (addressbook-bookmark-addressbook-p
                  value)
               (error
                (list
                 'signal
                 (car
                  error-data)
                 (cdr
                  error-data))))
             (condition-case error-data
                 (addressbook-bookmark-p
                  value)
               (error
                (list
                 'signal
                 (car
                  error-data)
                 (cdr
                  error-data))))))
          (list
           (car
            bookmark-alist)
           (cadr
            bookmark-alist)
           "Ada"
           "File"
           "Unknown"
           nil
           42)))"##;
    let expect = expect![[
        r#"OK ((("Ada" (type . "addressbook") (email . "ada@example.test")) t t) (("File" (type . "file") (filename . "/work/file")) nil nil) ("Ada" t t) ("File" nil nil) ("Unknown" nil nil) (nil nil nil) (42 nil nil))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_alist_only_preserves_order_duplicates_and_records() {
    let elisp_form = r##"(let* ((ada
                  '("Ada" (type . "addressbook") (email . "ada@example.test")))
                 (bookmark-alist
                  (list
                   '("File" (type . "file"))
                   ada
                   '("No type")
                   '("Bob" (type . "addressbook"))
                   ada)))
         (let ((result
                (addressbook-alist-only)))
           (list
            result
            (length
             result)
            (eq
             (car
              result)
             ada)
            (eq
             (car
              result)
             (car
              (last
               result))))))"##;
    let expect = expect![[
        r#"OK ((#1=("Ada" (type . "addressbook") (email . "ada@example.test")) ("Bob" (type . "addressbook")) #1#) 3 t t)"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_make_entry_records_every_field_and_handler_in_order() {
    let elisp_form = r##"(cl-letf (((symbol-function
                       'current-time)
                      (lambda ()
                        '(26000 12345 0 0))))
         (let ((entry
                (addressbook-bookmark-make-entry
                 "Ada"
                 "math, programming"
                 "ada@example.test, countess@example.test"
                 "+44 123"
                 "https://example.test"
                 "1 Engine Way"
                 "London"
                 "London"
                 "SW1"
                 "UK"
                 "notes"
                 "/images/ada.png")))
         (list
          entry
          (mapcar
           (lambda (key)
             (cons
              key
              (assoc-default
               key
               entry)))
           '(type
             location
             image
             email
             phone
             web
             street
             city
             state
             zipcode
             country
             note
             group
             handler))
          (bookmark-get-filename
           entry))))"##;
    let expect = expect![[
        r#"OK (("Ada" (position . 0) (last-modified 26000 12345 0 0) (type . "addressbook") (location . "Addressbook entry") (image . "/images/ada.png") (email . "ada@example.test, countess@example.test") (phone . "+44 123") (web . "https://example.test") (street . "1 Engine Way") (city . "London") (state . "London") (zipcode . "SW1") (country . "UK") (note . "notes") (group . "math, programming") (handler . addressbook-bookmark-jump)) ((type . "addressbook") (location . "Addressbook entry") (image . "/images/ada.png") (email . "ada@example.test, countess@example.test") (phone . "+44 123") (web . "https://example.test") (street . "1 Engine Way") (city . "London") (state . "London") (zipcode . "SW1") (country . "UK") (note . "notes") (group . "math, programming") (handler . addressbook-bookmark-jump)) nil)"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_read_name_repeats_until_empty_and_joins_exactly() {
    let elisp_form = r##"(let ((responses
                '("one"
                  "two"
                  "three"
                  ""))
               prompts)
         (cl-letf (((symbol-function
                     'read-string)
                    (lambda (prompt &rest _arguments)
                      (push
                       prompt
                       prompts)
                      (pop
                       responses))))
           (list
            (addressbook-read-name
             "Value: ")
            (nreverse
             prompts)
            responses)))"##;
    let expect =
        expect![[r#"OK ("one, two, three" ("Value: " "Value: " "Value: " "Value: ") nil)"#]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_read_name_handles_immediate_and_single_value_termination() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function
                     'read-string)
                    (lambda (prompt &rest _arguments)
                      (push
                       prompt
                       calls)
                      "")))
           (let ((empty
                  (addressbook-read-name
                   "Empty: ")))
             (cl-letf (((symbol-function
                         'read-string)
                        (let ((responses
                               '("only"
                                 "")))
                          (lambda (prompt &rest _arguments)
                            (push
                             prompt
                             calls)
                            (pop
                             responses)))))
               (list
                empty
                (addressbook-read-name
                 "Single: ")
                (nreverse
                 calls))))))"##;
    let expect = expect![[r#"OK ("" "only" ("Empty: " "Single: " "Single: "))"#]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_maybe_save_increments_counter_and_obeys_save_threshold() {
    let elisp_form = r##"(let ((bookmark-alist-modification-count
                4)
               threshold-calls
               save-calls)
         (cl-letf (((symbol-function
                     'bookmark-time-to-save-p)
                    (lambda ()
                      (push
                       bookmark-alist-modification-count
                       threshold-calls)
                      (= bookmark-alist-modification-count 5)))
                   ((symbol-function
                     'bookmark-save)
                    (lambda (&rest arguments)
                      (push
                       arguments
                       save-calls)
                      'saved)))
           (let ((first
                  (addressbook-maybe-save-bookmark))
                 second)
             (setq
              second
              (addressbook-maybe-save-bookmark))
             (list
              first
              second
              bookmark-alist-modification-count
              (nreverse
               threshold-calls)
              (nreverse
               save-calls)))))"##;
    let expect = expect!["OK (saved nil 6 (5 6) (nil))"];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_filter_setup_uses_bookmark_sorting_then_filters_contacts() {
    let elisp_form = r##"(let ((bookmark-alist
                '(("Zulu" (type . "addressbook"))
                  ("File" (type . "file"))
                  ("Ada" (type . "addressbook"))))
               calls)
         (cl-letf (((symbol-function
                     'bookmark-maybe-sort-alist)
                    (lambda ()
                      (push
                       (copy-tree
                        bookmark-alist)
                       calls)
                      (sort
                       (copy-tree
                        bookmark-alist)
                       (lambda (left right)
                         (string-lessp
                          (car
                           left)
                          (car
                           right)))))))
           (list
            (addressbook-bookmark-filter-setup-alist)
            (nreverse
             calls)
            bookmark-alist)))"##;
    let expect = expect![[
        r#"OK ((("Ada" (type . "addressbook")) ("Zulu" (type . "addressbook"))) ((("Zulu" (type . "addressbook")) ("File" (type . "file")) ("Ada" (type . "addressbook")))) (("Zulu" (type . "addressbook")) ("File" (type . "file")) ("Ada" (type . "addressbook"))))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_complete_multiple_selects_helm_or_standard_protocol() {
    let elisp_form = r##"(progn
         (setq
          helm-mode
          t)
         (let (calls)
         (cl-letf (((symbol-function
                     'completing-read)
                    (lambda (&rest arguments)
                      (push
                       (list
                        'single
                        arguments
                        helm-comp-read-use-marked)
                       calls)
                      "Ada"))
                   ((symbol-function
                     'completing-read-multiple)
                    (lambda (&rest arguments)
                      (push
                       (list
                        'multiple
                        arguments
                        (bound-and-true-p
                         helm-comp-read-use-marked))
                       calls)
                      '("Ada"
                        "Bob"))))
           (list
            (addressbook-complete-multiple
             "Contact: "
             '("Ada"
               "Bob")
             #'stringp
             t
             "A"
             'contact-history)
            (progn
              (setq
               helm-mode
               nil)
              (addressbook-complete-multiple
               "Contact: "
               '("Ada"
                 "Bob")
               #'stringp
               nil
               nil
               'contact-history))
            (nreverse
             calls)))))"##;
    let expect = expect![[
        r#"OK ("Ada" ("Ada" "Bob") ((single ("Contact: " ("Ada" "Bob") stringp t "A" contact-history) t) (multiple ("Contact: " ("Ada" "Bob") stringp nil nil contact-history) nil)))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

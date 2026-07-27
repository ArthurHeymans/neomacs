use expect_test::expect;

use super::assert_addressbook_bookmark_parity;

#[test]
fn addressbook_bookmark_set_one_contact_records_all_fields_and_side_effects() {
    let elisp_form = r##"(let ((bookmark-alist
                '(("File" (type . "file"))))
               read-name-calls
               read-string-calls
               rebuild-calls
               save-calls
               messages)
         (cl-letf (((symbol-function
                     'addressbook-read-name)
                    (lambda (prompt)
                      (push
                       prompt
                       read-name-calls)
                      (cdr
                       (assoc
                        prompt
                        '(("Group: " . "science")
                          ("Mail: " . "ada@example.test")
                          ("Phone: " . "+44")
                          ("Web: " . "https://example.test"))))))
                   ((symbol-function
                     'read-string)
                    (lambda (prompt &rest _arguments)
                      (push
                       prompt
                       read-string-calls)
                      (cdr
                       (assoc
                        prompt
                        '(("Street: " . "1 Engine Way")
                          ("City: " . "London")
                          ("State: " . "London")
                          ("Zipcode: " . "SW1")
                          ("Country: " . "UK")
                          ("Note: " . "programmer")
                          ("Image path: " . "/images/ada.png"))))))
                   ((symbol-function
                     'bookmark-maybe-load-default-file)
                    (lambda ()
                      'loaded))
                   ((symbol-function
                     'bookmark-bmenu-surreptitiously-rebuild-list)
                    (lambda ()
                      (push
                       'rebuild
                       rebuild-calls)))
                   ((symbol-function
                     'addressbook-maybe-save-bookmark)
                    (lambda ()
                      (push
                       'save
                       save-calls)))
                   ((symbol-function
                     'message)
                    (lambda (format-string &rest arguments)
                      (push
                       (apply
                        #'format
                        format-string
                        arguments)
                       messages)))
                   ((symbol-function
                     'current-time)
                    (lambda ()
                      '(26000 12345 0 0))))
           (let ((return
                  (addressbook-bookmark-set-1
                   "Ada")))
             (list
              return
              bookmark-alist
              (nreverse
               read-name-calls)
              (nreverse
               read-string-calls)
              (nreverse
               rebuild-calls)
              (nreverse
               save-calls)
              (nreverse
               messages)))))"##;
    let expect = expect![[
        r#"OK (#1=("1 Contact(s) added.") (("Ada" (position . 0) (last-modified 26000 12345 0 0) (type . "addressbook") (location . "Addressbook entry") (image . "/images/ada.png") (email . "ada@example.test") (phone . "+44") (web . "https://example.test") (street . "1 Engine Way") (city . "London") (state . "London") (zipcode . "SW1") (country . "UK") (note . "programmer") (group . "science") (handler . addressbook-bookmark-jump)) ("File" (type . "file"))) ("Group: " "Mail: " "Phone: " "Web: ") ("Street: " "City: " "State: " "Zipcode: " "Country: " "Note: " "Image path: ") (rebuild) (save) #1#)"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_set_updates_existing_contact_without_reordering() {
    let elisp_form = r##"(let* ((existing
                  '("Ada"
                    (type . "addressbook")
                    (email . "old@example.test")
                    (note . "old")))
                 (file
                  '("Ada File"
                    (type . "file")
                    (filename . "/ada")))
                 (bookmark-alist
                  (list
                   existing
                   file))
                 side-effects)
         (cl-letf (((symbol-function
                     'addressbook-read-name)
                    (lambda (prompt)
                      (if
                          (equal
                           prompt
                           "Mail: ")
                          "new@example.test"
                        "")))
                   ((symbol-function
                     'read-string)
                    (lambda (prompt &rest _arguments)
                      (if
                          (equal
                           prompt
                           "Note: ")
                          "new"
                        "")))
                   ((symbol-function
                     'bookmark-maybe-load-default-file)
                    (lambda ()
                      nil))
                   ((symbol-function
                     'bookmark-bmenu-surreptitiously-rebuild-list)
                    (lambda ()
                      (push
                       'rebuild
                       side-effects)))
                   ((symbol-function
                     'addressbook-maybe-save-bookmark)
                    (lambda ()
                      (push
                       'save
                       side-effects)))
                   ((symbol-function
                     'message)
                    (lambda (&rest _arguments)
                      (push
                       'message
                       side-effects)))
                   ((symbol-function
                     'current-time)
                    (lambda ()
                      '(26000 12345 0 0))))
           (addressbook-bookmark-set-1
            "Ada")
           (list
            bookmark-alist
            (eq
             existing
             (car
              bookmark-alist))
            (eq
             file
             (cadr
              bookmark-alist))
            (nreverse
             side-effects))))"##;
    let expect = expect![[
        r#"OK ((("Ada" (position . 0) (last-modified 26000 12345 0 0) (type . "addressbook") (location . "Addressbook entry") (image . "") (email . "new@example.test") (phone . "") (web . "") (street . "") (city . "") (state . "") (zipcode . "") (country . "") (note . "new") (group . "") (handler . addressbook-bookmark-jump)) ("Ada File" (type . "file") (filename . "/ada"))) t t (rebuild save message))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_set_same_name_non_contact_adds_duplicate_without_mutating_file() {
    let elisp_form = r##"(let* ((file
                  '("Ada"
                    (type . "file")
                    (filename . "/work/ada.txt")))
                 (bookmark-alist
                  (list
                   file))
                 side-effects)
         (cl-letf (((symbol-function
                     'addressbook-read-name)
                    (lambda (_prompt)
                      ""))
                   ((symbol-function
                     'read-string)
                    (lambda (prompt &rest _arguments)
                      (if
                          (equal
                           prompt
                           "Note: ")
                          "new contact"
                        "")))
                   ((symbol-function
                     'bookmark-maybe-load-default-file)
                    (lambda ()
                      nil))
                   ((symbol-function
                     'bookmark-bmenu-surreptitiously-rebuild-list)
                    (lambda ()
                      (push
                       'rebuild
                       side-effects)))
                   ((symbol-function
                     'addressbook-maybe-save-bookmark)
                    (lambda ()
                      (push
                       'save
                       side-effects)))
                   ((symbol-function
                     'message)
                    (lambda (&rest _arguments)
                      (push
                       'message
                       side-effects)))
                   ((symbol-function
                     'current-time)
                    (lambda ()
                      '(26000 12345 0 0))))
           (addressbook-bookmark-set-1
            "Ada")
           (list
            bookmark-alist
            (eq
             file
             (cadr
              bookmark-alist))
            file
            (mapcar
             (lambda (entry)
               (assoc-default
                'type
                entry))
             bookmark-alist)
            (nreverse
             side-effects))))"##;
    let expect = expect![[
        r#"OK ((("Ada" (position . 0) (last-modified 26000 12345 0 0) (type . "addressbook") (location . "Addressbook entry") (image . "") (email . "") (phone . "") (web . "") (street . "") (city . "") (state . "") (zipcode . "") (country . "") (note . "new contact") (group . "") (handler . addressbook-bookmark-jump)) #1=("Ada" (type . "file") (filename . "/work/ada.txt"))) t #1# ("addressbook" "file") (rebuild save message))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_set_without_contact_recurses_until_user_declines() {
    let elisp_form = r##"(let ((bookmark-alist nil)
               (names
                '("Ada"
                  "Bob"))
               confirmations
               rebuild-calls
               save-calls
               messages)
         (cl-letf (((symbol-function
                     'addressbook-read-name)
                    (lambda (_prompt)
                      ""))
                   ((symbol-function
                     'read-string)
                    (lambda (prompt &rest _arguments)
                      (if
                          (equal
                           prompt
                           "Name: ")
                          (pop
                           names)
                        "")))
                   ((symbol-function
                     'y-or-n-p)
                    (lambda (prompt)
                      (push
                       prompt
                       confirmations)
                      (= (length confirmations) 1)))
                   ((symbol-function
                     'bookmark-maybe-load-default-file)
                    (lambda ()
                      nil))
                   ((symbol-function
                     'bookmark-bmenu-surreptitiously-rebuild-list)
                    (lambda ()
                      (push
                       'rebuild
                       rebuild-calls)))
                   ((symbol-function
                     'addressbook-maybe-save-bookmark)
                    (lambda ()
                      (push
                       'save
                       save-calls)))
                   ((symbol-function
                     'message)
                    (lambda (format-string &rest arguments)
                      (push
                       (apply
                        #'format
                        format-string
                        arguments)
                       messages)))
                   ((symbol-function
                     'current-time)
                    (lambda ()
                      '(26000 12345 0 0))))
           (let ((return
                  (addressbook-bookmark-set-1)))
             (list
              return
              bookmark-alist
              names
              (nreverse
               confirmations)
              (nreverse
               rebuild-calls)
              (nreverse
               save-calls)
              (nreverse
               messages)))))"##;
    let expect = expect![[
        r#"OK (#5=("2 Contact(s) added.") (("Bob" (position . 0) (last-modified . #1=(26000 12345 0 0)) #2=(type . "addressbook") #3=(location . "Addressbook entry") (image . "") (email . "") (phone . "") (web . "") (street . "") (city . "") (state . "") (zipcode . "") (country . "") (note . "") (group . "") . #4=((handler . addressbook-bookmark-jump))) ("Ada" (position . 0) (last-modified . #1#) #2# #3# (image . "") (email . "") (phone . "") (web . "") (street . "") (city . "") (state . "") (zipcode . "") (country . "") (note . "") (group . "") . #4#)) nil ("`Ada' Recorded. Add a new contact? " "`Bob' Recorded. Add a new contact? ") (rebuild rebuild) (save save) #5#)"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_from_mail_parses_new_contact_and_records_side_effects() {
    let elisp_form = r##"(let ((bookmark-alist nil)
               read-calls
               side-effects)
         (cl-letf (((symbol-function
                     'read-string)
                    (lambda (prompt initial &rest _arguments)
                      (push
                       (list
                        prompt
                        initial)
                       read-calls)
                      initial))
                   ((symbol-function
                     'message)
                    (lambda (format-string &rest arguments)
                      (push
                       (apply
                        #'format
                        format-string
                        arguments)
                       side-effects)))
                   ((symbol-function
                     'bookmark-bmenu-surreptitiously-rebuild-list)
                    (lambda ()
                      (push
                       'rebuild
                       side-effects)))
                   ((symbol-function
                     'addressbook-maybe-save-bookmark)
                    (lambda ()
                      (push
                       'save
                       side-effects)))
                   ((symbol-function
                     'current-time)
                    (lambda ()
                      '(26000 12345 0 0))))
           (let ((return
                  (addressbook--bookmark-from-mail
                   "Ada Lovelace <ada@example.test>")))
             (list
              return
              bookmark-alist
              (nreverse
               read-calls)
              (nreverse
               side-effects)))))"##;
    let expect = expect![[
        r#"OK (#1=(save) (("Ada Lovelace" (position . 0) (last-modified 26000 12345 0 0) (type . "addressbook") (location . "Addressbook entry") (image . "") (email . "ada@example.test") (phone . "") (web . "") (street . "") (city . "") (state . "") (zipcode . "") (country . "") (note . "") (group . "") (handler . addressbook-bookmark-jump))) (("Name: " "Ada Lovelace") ("Email: " "ada@example.test")) ("`Ada Lovelace' bookmarked" rebuild . #1#))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_from_mail_existing_contact_preserves_exact_duplicate_logic() {
    let elisp_form = r##"(let ((bookmark-alist
                '(("Ada"
                   (type . "addressbook")
                   (email . "old@example.test, same@example.test")))))
         (cl-letf (((symbol-function
                     'read-string)
                    (lambda (_prompt initial &rest _arguments)
                      initial))
                   ((symbol-function
                     'message)
                    (lambda (&rest _arguments)
                      nil))
                   ((symbol-function
                     'bookmark-bmenu-surreptitiously-rebuild-list)
                    (lambda ()
                      nil))
                   ((symbol-function
                     'addressbook-maybe-save-bookmark)
                    (lambda ()
                      nil))
                   ((symbol-function
                     'current-time)
                    (lambda ()
                      '(26000 12345 0 0))))
           (addressbook--bookmark-from-mail
            "Ada <same@example.test>")
           (let ((after-member
                  (copy-tree
                   bookmark-alist)))
             (addressbook--bookmark-from-mail
              "Ada <new@example.test>")
             (list
              after-member
              bookmark-alist))))"##;
    let expect = expect![[
        r#"OK ((("Ada" (position . 0) (last-modified 26000 12345 0 0) (type . "addressbook") (location . "Addressbook entry") (image . "") (email . "same@example.test, old@example.test, same@example.test") (phone . "") (web . "") (street . "") (city . "") (state . "") (zipcode . "") (country . "") (note . "") (group . "") (handler . addressbook-bookmark-jump))) (("Ada" (position . 0) (last-modified 26000 12345 0 0) (type . "addressbook") (location . "Addressbook entry") (image . "") (email . "new@example.test") (phone . "") (web . "") (street . "") (city . "") (state . "") (zipcode . "") (country . "") (note . "") (group . "") (handler . addressbook-bookmark-jump))))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_from_mail_same_name_non_contact_adds_duplicate_without_mutation() {
    let elisp_form = r##"(let* ((file
                  '("Ada"
                    (type . "file")
                    (filename . "/work/ada.txt")))
                 (bookmark-alist
                  (list
                   file))
                 side-effects)
         (cl-letf (((symbol-function
                     'read-string)
                    (lambda (_prompt initial &rest _arguments)
                      initial))
                   ((symbol-function
                     'message)
                    (lambda (format-string &rest arguments)
                      (push
                       (apply
                        #'format
                        format-string
                        arguments)
                       side-effects)))
                   ((symbol-function
                     'bookmark-bmenu-surreptitiously-rebuild-list)
                    (lambda ()
                      (push
                       'rebuild
                       side-effects)))
                   ((symbol-function
                     'addressbook-maybe-save-bookmark)
                    (lambda ()
                      (push
                       'save
                       side-effects)))
                   ((symbol-function
                     'current-time)
                    (lambda ()
                      '(26000 12345 0 0))))
           (addressbook--bookmark-from-mail
            "Ada <ada@example.test>")
           (list
            bookmark-alist
            (eq
             file
             (cadr
              bookmark-alist))
            file
            (mapcar
             (lambda (entry)
               (assoc-default
                'type
                entry))
             bookmark-alist)
            (nreverse
             side-effects))))"##;
    let expect = expect![[
        r#"OK ((("Ada" (position . 0) (last-modified 26000 12345 0 0) (type . "addressbook") (location . "Addressbook entry") (image . "") (email . "ada@example.test") (phone . "") (web . "") (street . "") (city . "") (state . "") (zipcode . "") (country . "") (note . "") (group . "") (handler . addressbook-bookmark-jump)) #1=("Ada" (type . "file") (filename . "/work/ada.txt"))) t #1# ("addressbook" "file") ("`Ada' bookmarked" rebuild save))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_from_mail_nil_data_only_prompts_and_does_not_mutate() {
    let elisp_form = r##"(let ((bookmark-alist
                '(("Existing" (type . "addressbook"))))
               prompts
               side-effects)
         (cl-letf (((symbol-function
                     'read-string)
                    (lambda (prompt initial &rest _arguments)
                      (push
                       (list
                        prompt
                        initial)
                       prompts)
                      "entered"))
                   ((symbol-function
                     'message)
                    (lambda (&rest arguments)
                      (push
                       arguments
                       side-effects)))
                   ((symbol-function
                     'bookmark-bmenu-surreptitiously-rebuild-list)
                    (lambda ()
                      (push
                       'rebuild
                       side-effects)))
                   ((symbol-function
                     'addressbook-maybe-save-bookmark)
                    (lambda ()
                      (push
                       'save
                       side-effects))))
           (list
            (addressbook--bookmark-from-mail
             nil)
            bookmark-alist
            (nreverse
             prompts)
            side-effects)))"##;
    let expect = expect![[
        r#"OK (nil (("Existing" (type . "addressbook"))) (("Name: " nil) ("Email: " nil)) nil)"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_edit_confirmed_replaces_record_and_declined_preserves_it() {
    let elisp_form = r##"(let* ((confirmed
                  '("Ada"
                    (type . "addressbook")
                    (email . "old@example.test")
                    (group . "old")
                    (phone . "")
                    (web . "")
                    (street . "")
                    (city . "")
                    (state . "")
                    (zipcode . "")
                    (country . "")
                    (note . "old")
                    (image . "")))
                 (declined
                  (copy-tree
                   confirmed))
                 (answers
                  '("Augusta"
                    "math"
                    "new@example.test"
                    "+44"
                    "https://example.test"
                    "Street"
                    "City"
                    "State"
                    "Zip"
                    "Country"
                    "Note"
                    "image.png"
                    "Ignored"
                    "ignored"
                    "ignored"
                    "ignored"
                    "ignored"
                    "ignored"
                    "ignored"
                    "ignored"
                    "ignored"
                    "ignored"
                    "ignored"
                    "ignored"))
                 save-calls
                 confirmations)
         (cl-letf (((symbol-function
                     'read-string)
                    (lambda (_prompt _initial &rest _arguments)
                      (pop
                       answers)))
                   ((symbol-function
                     'y-or-n-p)
                    (lambda (prompt)
                      (push
                       prompt
                       confirmations)
                      (= (length confirmations) 1)))
                   ((symbol-function
                     'addressbook-maybe-save-bookmark)
                    (lambda ()
                      (push
                       'save
                       save-calls)))
                   ((symbol-function
                     'current-time)
                    (lambda ()
                      '(26000 12345 0 0))))
           (let ((yes-result
                  (addressbook-bookmark-edit
                   confirmed))
                 (no-result
                  (addressbook-bookmark-edit
                   declined)))
             (list
              yes-result
              confirmed
              no-result
              declined
              (nreverse
               confirmations)
              (nreverse
               save-calls)))))"##;
    let expect = expect![[
        r#"OK (("Augusta" . #1=((position . 0) (last-modified 26000 12345 0 0) (type . "addressbook") (location . "Addressbook entry") (image . "image.png") (email . "new@example.test") (phone . "+44") (web . "https://example.test") (street . "Street") (city . "City") (state . "State") (zipcode . "Zip") (country . "Country") (note . "Note") (group . "math") (handler . addressbook-bookmark-jump))) ("Augusta" . #1#) nil ("Ada" (type . "addressbook") (email . "old@example.test") (group . "old") (phone . "") (web . "") (street . "") (city . "") (state . "") (zipcode . "") (country . "") (note . "old") (image . "")) ("Save changes? " "Save changes? ") (save))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_gnus_and_mu4e_adapters_forward_exact_sender_data() {
    let elisp_form = r##"(progn
         (setq
          gnus-article-current
          '(article . 17))
         (let (calls
               required)
         (cl-letf (((symbol-function
                     'require)
                    (lambda (feature &rest _arguments)
                      (push
                       feature
                       required)
                      feature))
                   ((symbol-function
                     'gnus-summary-article-header)
                    (lambda (article)
                      (push
                       (list
                        'header
                        article)
                       calls)
                      [0 1 "Gnus Sender <gnus@example.test>"]))
                   ((symbol-function
                     'mu4e-message-at-point)
                    (lambda ()
                      '(:from
                        (("Mu Sender" . "mu@example.test")))))
                   ((symbol-function
                     'addressbook--bookmark-from-mail)
                    (lambda (data)
                      (push
                       (list
                        'record
                        data)
                       calls)
                      data)))
           (list
            (addressbook-gnus-sum-bookmark)
            (addressbook-get-mu4e-from-field)
            (addressbook-mu4e-bookmark)
            (nreverse
             required)
            (nreverse
             calls)))))"##;
    let expect = expect![[
        r#"OK ("Gnus Sender <gnus@example.test>" "Mu Sender <mu@example.test>" "Mu Sender <mu@example.test>" (gnus-sum) ((header 17) (record "Gnus Sender <gnus@example.test>") (record "Mu Sender <mu@example.test>")))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

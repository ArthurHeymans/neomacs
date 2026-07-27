use expect_test::expect;

use super::assert_addressbook_bookmark_parity;

#[test]
fn addressbook_bookmark_mail_wrappers_forward_prefix_and_cc_arguments_exactly() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function
                     'addressbook-set-mail-buffer-1)
                    (lambda (&optional bookmark-name append cc)
                      (push
                       (list
                        bookmark-name
                        append
                        cc)
                       calls)
                      'forwarded)))
           (list
            (addressbook-set-mail-buffer
             nil)
            (addressbook-set-mail-buffer
             '(4))
            (addressbook-set-mail-buffer-and-cc
             nil)
            (addressbook-set-mail-buffer-and-cc
             '(16))
            (nreverse
             calls)
            (interactive-form
             'addressbook-set-mail-buffer)
            (interactive-form
             'addressbook-set-mail-buffer-and-cc))))"##;
    let expect = expect![[
        r#"OK (forwarded forwarded forwarded forwarded ((nil nil nil) (nil (4) nil) (nil nil cc) (nil (16) cc)) (interactive "P") (interactive "P"))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_set_mail_uses_contact_mode_and_selects_one_of_multiple_emails() {
    let elisp_form = r##"(let* ((mail-buffer
                 (generate-new-buffer
                   "*addressbook-mail-parity*"))
                 (major-mode
                  'addressbook-mode)
                 completion-calls
                 pop-calls
                 load-calls
                 result)
         (unwind-protect
             (progn
               (with-current-buffer
                   mail-buffer
                 (message-mode)
                 (erase-buffer)
                 (insert
                  "To: \nSubject: \n\n"))
               (cl-letf (((symbol-function
                           'bookmark-maybe-load-default-file)
                          (lambda ()
                            (push
                             'load
                             load-calls)))
                         ((symbol-function
                           'addressbook-get-contact-data)
                          (lambda ()
                            '("Ada"
                              (type . "addressbook")
                              (email . "ada@example.test, countess@example.test"))))
                         ((symbol-function
                           'message-buffers)
                          (lambda ()
                            (list
                             mail-buffer)))
                         ((symbol-function
                           'pop-to-buffer)
                          (lambda (buffer &rest arguments)
                            (push
                             (list
                              (if
                                  (bufferp
                                   buffer)
                                  (buffer-name
                                   buffer)
                                buffer)
                              arguments)
                             pop-calls)
                            (set-buffer
                             (if
                                 (bufferp
                                  buffer)
                                 buffer
                               mail-buffer))))
                         ((symbol-function
                           'completing-read)
                          (lambda (&rest arguments)
                            (push
                             arguments
                             completion-calls)
                            "countess@example.test"))
                         ((symbol-function
                           'font-lock-ensure)
                          (lambda (&rest _arguments)
                            'fontified)))
                 (let ((return
                        (addressbook-set-mail-buffer-1
                         nil
                         t)))
                   (with-current-buffer
                       mail-buffer
                     (setq
                      result
                      (list
                       return
                       (buffer-string)
                       (point)))))
                 (setq
                  result
                  (append
                   result
                   (list
                    (nreverse
                     load-calls)
                    (nreverse
                     pop-calls)
                    (nreverse
                     completion-calls))))))
           (when
               (buffer-live-p
                mail-buffer)
             (kill-buffer
              mail-buffer)))
         result)"##;
    let expect = expect![[
        r#"OK (fontified "To: countess@example.test\nSubject: \n\n" 36 (load) (("*addressbook-mail-parity*" nil)) (("Choose mail: " ("ada@example.test" "countess@example.test") nil t)))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_set_mail_uses_named_bookmark_and_cc_header() {
    let elisp_form = r##"(let* ((mail-buffer
                  (generate-new-buffer
                   "*addressbook-mail-cc-parity*"))
                 (bookmark-alist
                  '(("Ada"
                     (type . "addressbook")
                     (email . "ada@example.test"))))
                 goto-calls
                 result)
         (unwind-protect
             (progn
               (with-current-buffer
                   mail-buffer
                 (insert
                  "To: recipient@example.test\nCc: \nSubject: Topic\n\n"))
               (cl-letf (((symbol-function
                           'bookmark-maybe-load-default-file)
                          (lambda ()
                            nil))
                         ((symbol-function
                           'message-buffers)
                          (lambda ()
                            (list
                             mail-buffer)))
                         ((symbol-function
                           'pop-to-buffer)
                          (lambda (buffer &rest _arguments)
                            (set-buffer
                             buffer)))
                         ((symbol-function
                           'message-goto-cc)
                          (lambda ()
                            (push
                             'cc
                             goto-calls)
                            (goto-char
                             (point-min))
                            (search-forward
                             "Cc: ")))
                         ((symbol-function
                           'font-lock-ensure)
                          (lambda (&rest _arguments)
                            nil)))
                 (let ((return
                        (addressbook-set-mail-buffer-1
                         "Ada"
                         nil
                         'cc)))
                   (with-current-buffer
                       mail-buffer
                     (setq
                      result
                      (list
                       return
                       (buffer-string)
                       (nreverse
                        goto-calls)))))))
           (when
               (buffer-live-p
                mail-buffer)
             (kill-buffer
              mail-buffer)))
         result)"##;
    let expect = expect![[
        r#"OK (nil "To: recipient@example.test\nCc: ada@example.test\nSubject: Topic\n\n" (cc))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_set_mail_append_creates_aligned_continuation_line() {
    let elisp_form = r##"(let* ((mail-buffer
                  (generate-new-buffer
                   "*addressbook-mail-append-parity*"))
                 (bookmark-alist
                  '(("Ada"
                     (type . "addressbook")
                     (email . "ada@example.test"))))
                 next-header-calls
                 result)
         (unwind-protect
             (progn
               (with-current-buffer
                   mail-buffer
                 (message-mode)
                 (erase-buffer)
                 (insert
                  "To: first@example.test\nSubject: Topic\n\n"))
               (cl-letf (((symbol-function
                           'bookmark-maybe-load-default-file)
                          (lambda ()
                            nil))
                         ((symbol-function
                           'pop-to-buffer)
                          (lambda (_buffer &rest _arguments)
                            (set-buffer
                             mail-buffer)))
                         ((symbol-function
                           'message-next-header)
                          (lambda ()
                            (push
                             'next
                             next-header-calls)
                            (forward-line
                             1)))
                         ((symbol-function
                           'font-lock-ensure)
                          (lambda (&rest _arguments)
                            nil)))
                 (let ((return
                        (addressbook-set-mail-buffer-1
                         "Ada"
                         t)))
                   (with-current-buffer
                       mail-buffer
                     (setq
                      result
                      (list
                       return
                       (buffer-string)
                       (nreverse
                        next-header-calls)))))))
           (when
               (buffer-live-p
                mail-buffer)
             (kill-buffer
              mail-buffer)))
         result)"##;
    let expect = expect![[
        r#"OK (nil "To: first@example.test,\n    ada@example.test\nSubject: Topic\n\n" (next))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_set_mail_without_buffers_composes_and_uses_newsgroups_fallback() {
    let elisp_form = r##"(let* ((mail-buffer
                  (generate-new-buffer
                   "*addressbook-compose-parity*"))
                 (bookmark-alist
                  '(("List"
                     (type . "addressbook")
                     (email . "group.example.test"))))
                 (buffer-queries
                  0)
                 compose-calls
                 result)
         (unwind-protect
             (cl-letf (((symbol-function
                         'bookmark-maybe-load-default-file)
                        (lambda ()
                          nil))
                       ((symbol-function
                         'message-buffers)
                        (lambda ()
                          (setq
                           buffer-queries
                           (1+
                            buffer-queries))
                          (if
                              (= buffer-queries 1)
                              nil
                            (list
                             mail-buffer))))
                       ((symbol-function
                         'compose-mail)
                        (lambda (&rest arguments)
                          (push
                           arguments
                           compose-calls)
                          (with-current-buffer
                              mail-buffer
                            (erase-buffer)
                            (insert
                             "Newsgroups: \nSubject: \n\n"))
                          'composed))
                       ((symbol-function
                         'font-lock-ensure)
                        (lambda (&rest _arguments)
                          'fontified)))
               (let ((return
                      (addressbook-set-mail-buffer-1
                       "List")))
                 (with-current-buffer
                     mail-buffer
                   (setq
                    result
                    (list
                     return
                     (buffer-string)
                     buffer-queries
                     (nreverse
                      compose-calls))))))
           (when
               (buffer-live-p
                mail-buffer)
             (kill-buffer
              mail-buffer)))
         result)"##;
    let expect = expect![[
        r#"OK (fontified "Newsgroups: group.example.test\nSubject: \n\n" 2 ((nil nil nil nil switch-to-buffer-other-window)))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_set_mail_multiple_buffers_prompts_but_mutates_first_buffer() {
    let elisp_form = r##"(let* ((first-buffer
                  (generate-new-buffer
                   "*addressbook-mail-first*"))
                 (second-buffer
                  (generate-new-buffer
                   "*addressbook-mail-second*"))
                 (bookmark-alist
                  '(("Ada"
                     (type . "addressbook")
                     (email . "ada@example.test"))))
                 completion-calls
                 pop-calls
                 result)
         (unwind-protect
             (progn
               (dolist
                   (buffer
                    (list
                     first-buffer
                     second-buffer))
                 (with-current-buffer
                     buffer
                   (insert
                    "To: \nSubject: \n\n")))
               (cl-letf (((symbol-function
                           'bookmark-maybe-load-default-file)
                          (lambda ()
                            nil))
                         ((symbol-function
                           'message-buffers)
                          (lambda ()
                            (list
                             first-buffer
                             second-buffer)))
                         ((symbol-function
                           'completing-read)
                          (lambda (&rest arguments)
                            (push
                             arguments
                             completion-calls)
                            second-buffer))
                         ((symbol-function
                           'pop-to-buffer)
                          (lambda (buffer &rest arguments)
                            (push
                             (list
                              (buffer-name
                               buffer)
                              arguments)
                             pop-calls)
                            (set-buffer
                             buffer)))
                         ((symbol-function
                           'font-lock-ensure)
                          (lambda (&rest _arguments)
                            nil)))
                 (let ((return
                        (addressbook-set-mail-buffer-1
                         "Ada"
                         t)))
                   (setq
                    result
                    (list
                     return
                     (with-current-buffer
                         first-buffer
                       (buffer-string))
                     (with-current-buffer
                         second-buffer
                       (buffer-string))
                     (mapcar
                      (lambda (arguments)
                        (list
                         (car
                          arguments)
                         (mapcar
                          #'buffer-name
                          (cadr
                           arguments))
                         (nthcdr
                          2
                          arguments)))
                      (nreverse
                       completion-calls))
                     (nreverse
                      pop-calls))))))
           (dolist
               (buffer
                (list
                 first-buffer
                 second-buffer))
             (when
                 (buffer-live-p
                  buffer)
               (kill-buffer
                buffer))))
         result)"##;
    let expect = expect![[
        r#"OK (nil "To: ada@example.test\nSubject: \n\n" "To: \nSubject: \n\n" (("MailBuffer: " ("*addressbook-mail-first*" "*addressbook-mail-second*") (nil t))) (("*addressbook-mail-second*" nil)))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_set_mail_bcc_uses_bcc_navigation_branch() {
    let elisp_form = r##"(let* ((mail-buffer
                  (generate-new-buffer
                   "*addressbook-mail-bcc-parity*"))
                 (bookmark-alist
                  '(("Ada"
                     (type . "addressbook")
                     (email . "ada@example.test"))))
                 goto-calls
                 result)
         (unwind-protect
             (progn
               (with-current-buffer
                   mail-buffer
                 (insert
                  "To: recipient@example.test\nBcc: \nSubject: Topic\n\n"))
               (cl-letf (((symbol-function
                           'bookmark-maybe-load-default-file)
                          (lambda ()
                            nil))
                         ((symbol-function
                           'message-buffers)
                          (lambda ()
                            (list
                             mail-buffer)))
                         ((symbol-function
                           'pop-to-buffer)
                          (lambda (buffer &rest _arguments)
                            (set-buffer
                             buffer)))
                         ((symbol-function
                           'message-goto-bcc)
                          (lambda ()
                            (push
                             'bcc
                             goto-calls)
                            (goto-char
                             (point-min))
                            (search-forward
                             "Bcc: ")))
                         ((symbol-function
                           'message-goto-cc)
                          (lambda ()
                            (push
                             'unexpected-cc
                             goto-calls)))
                         ((symbol-function
                           'font-lock-ensure)
                          (lambda (&rest _arguments)
                            nil)))
                 (let ((return
                        (addressbook-set-mail-buffer-1
                         "Ada"
                         nil
                         'bcc)))
                   (with-current-buffer
                       mail-buffer
                     (setq
                      result
                      (list
                       return
                       (buffer-string)
                       (nreverse
                        goto-calls)))))))
           (when
               (buffer-live-p
                mail-buffer)
             (kill-buffer
              mail-buffer)))
         result)"##;
    let expect = expect![[
        r#"OK (nil "To: recipient@example.test\nBcc: ada@example.test\nSubject: Topic\n\n" (bcc))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_set_mail_signals_when_no_contact_can_be_resolved() {
    let elisp_form = r##"(let ((bookmark-alist
                '(("File" (type . "file"))))
               side-effects)
         (cl-letf (((symbol-function
                     'bookmark-maybe-load-default-file)
                    (lambda ()
                      (push
                       'load
                       side-effects)))
                   ((symbol-function
                     'message-buffers)
                    (lambda ()
                      (push
                       'buffers
                       side-effects)
                      nil))
                   ((symbol-function
                     'compose-mail)
                    (lambda (&rest arguments)
                      (push
                       (list
                        'compose
                        arguments)
                       side-effects))))
           (list
            (condition-case error-data
                (addressbook-set-mail-buffer-1
                 "Unknown")
              (error
               (list
                'signal
                (car
                 error-data)
                (cdr
                 error-data))))
            (nreverse
             side-effects))))"##;
    let expect =
        expect![[r#"OK ((signal error ("No contact found to set mail buffer")) (load buffers))"#]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_set_mail_for_all_scans_each_rendered_contact() {
    let elisp_form = r##"(let ((addressbook-buffer-name
                "*addressbook-all-mail-parity*")
               calls
               result)
         (unwind-protect
             (progn
               (with-current-buffer
                   (get-buffer-create
                    addressbook-buffer-name)
                 (insert
                  "Addressbook\n\nName: Ada\nMail: ada@example.test\n---\nName: Bob\nMail: bob@example.test\n"))
               (cl-letf (((symbol-function
                           'addressbook-set-mail-buffer-1)
                          (lambda (&optional bookmark-name append cc)
                            (push
                             (list
                              bookmark-name
                              append
                              cc
                              (buffer-name)
                              (line-number-at-pos))
                             calls)
                            'added)))
                 (setq
                  result
                  (list
                   (addressbook-set-mail-buffer-for-all)
                   (nreverse
                    calls)))))
           (when
               (get-buffer
                addressbook-buffer-name)
             (kill-buffer
              addressbook-buffer-name)))
         result)"##;
    let expect = expect![[
        r#"OK (nil ((nil t nil "*addressbook-all-mail-parity*" 3) (nil t nil "*addressbook-all-mail-parity*" 6)))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_mode_revert_rebuilds_contacts_preserves_current_and_header() {
    let elisp_form = r##"(let (pp-calls
               header-calls)
         (with-temp-buffer
           (insert
            "Name: Ada\n---\nName: Bob\n---\n")
           (goto-char
            (point-min))
           (search-forward
            "Bob")
           (cl-letf (((symbol-function
                       'addressbook-get-contact-data)
                      (lambda ()
                        (save-excursion
                          (forward-line
                           0)
                          (if
                              (looking-at
                               "Name: \\(.*\\)")
                              (list
                               (match-string
                                1))
                            '("Bob")))))
                     ((symbol-function
                       'addressbook-pp-info)
                      (lambda (name &optional append)
                        (push
                         (list
                          name
                          append)
                         pp-calls)
                        (insert
                         "Name: "
                         name
                         "\n---\n")))
                     ((symbol-function
                       'addressbook--insert-header)
                      (lambda ()
                        (push
                         'header
                         header-calls)
                        (goto-char
                         (point-min))
                        (insert
                         "Addressbook Test\n"))))
             (let ((return
                    (addressbook-mode-revert)))
               (list
                return
                (buffer-string)
                (buffer-substring-no-properties
                 (line-beginning-position)
                 (line-end-position))
                (nreverse
                 pp-calls)
                (nreverse
                 header-calls))))))"##;
    let expect = expect![[
        r#"OK (0 "Addressbook Test\nName: Bob\n---\nName: Ada\n---\n" "Name: Bob" (("Ada" t) ("Bob" t)) (header))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_edit_command_updates_name_property_and_reverts_on_success_only() {
    let elisp_form = r##"(let (calls)
         (with-temp-buffer
           (insert
            "Name:    Ada\n")
           (put-text-property
            (point-min)
            (+ (point-min) 5)
            'name
            "Ada")
           (goto-char
            (point-max))
           (cl-letf (((symbol-function
                       'addressbook-get-contact-data)
                      (lambda ()
                        (push
                         'get
                         calls)
                        '("Ada"
                          (type . "addressbook"))))
                     ((symbol-function
                       'addressbook-bookmark-edit)
                      (lambda (_bookmark)
                        (push
                         'edit
                         calls)
                        '("Augusta"
                          (type . "addressbook"))))
                     ((symbol-function
                       'addressbook--goto-name)
                      (lambda ()
                        (push
                         'goto
                         calls)
                        (goto-char
                         (point-min))))
                     ((symbol-function
                       'revert-buffer)
                      (lambda (&rest arguments)
                        (push
                         (list
                          'revert
                          arguments)
                         calls)
                        'reverted)))
             (let ((return
                    (addressbook-edit)))
               (list
                return
                (get-text-property
                 (point-min)
                 'name)
                (nreverse
                 calls))))))"##;
    let expect = expect![[r#"OK (reverted "Augusta" (get edit goto (revert nil)))"#]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_jump_visits_first_then_appends_remaining_contacts() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function
                     'bookmark-jump)
                    (lambda (bookmark &rest arguments)
                      (push
                       (list
                        bookmark
                        current-prefix-arg
                        arguments)
                       calls)
                      bookmark)))
           (let ((current-prefix-arg
                  nil))
             (list
              (addressbook-jump
               '("Ada"
                 "Bob"
                 "Carol"))
              (nreverse
               calls)))))"##;
    let expect = expect![[r#"OK (nil (("Ada" nil nil) ("Bob" #1=(4) nil) ("Carol" #1# nil)))"#]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_set_and_quit_commands_delegate_in_the_expected_buffer() {
    let elisp_form = r##"(let ((addressbook-buffer-name
                "*addressbook-command-parity*")
               calls
               result)
         (unwind-protect
             (progn
               (get-buffer-create
                addressbook-buffer-name)
               (cl-letf (((symbol-function
                           'addressbook-bookmark-set-1)
                          (lambda (&rest arguments)
                            (push
                             (list
                              'set
                              arguments
                              (buffer-name))
                             calls)
                            'set))
                         ((symbol-function
                           'quit-window)
                          (lambda (&rest arguments)
                            (push
                             (list
                              'quit
                              arguments
                              (buffer-name))
                             calls)
                            'quit)))
                 (setq
                  result
                  (list
                   (addressbook-bookmark-set)
                   (addressbook-quit)
                   (nreverse
                    calls)))))
           (when
               (get-buffer
                addressbook-buffer-name)
             (kill-buffer
              addressbook-buffer-name)))
         result)"##;
    let expect = expect![[
        r#"OK (set quit ((set nil "*scratch*") (quit nil "*addressbook-command-parity*")))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_bmenu_edit_rebuilds_and_positions_changed_entry() {
    let elisp_form = r##"(let ((bookmark-alist
                '(("Ada" (type . "addressbook"))))
               (query-count
                0)
               calls)
         (cl-letf (((symbol-function
                     'bookmark-bmenu-bookmark)
                    (lambda ()
                      (setq
                       query-count
                       (1+
                        query-count))
                      (if
                          (= query-count 1)
                          "Ada"
                        (buffer-substring-no-properties
                         (line-beginning-position)
                         (line-end-position)))))
                   ((symbol-function
                     'addressbook-bookmark-edit)
                    (lambda (bookmark)
                      (push
                       (list
                        'edit
                        bookmark)
                       calls)
                      '("Augusta"
                        (type . "addressbook"))))
                   ((symbol-function
                     'bookmark-bmenu-surreptitiously-rebuild-list)
                    (lambda ()
                      (push
                       'rebuild
                       calls)))
                   ((symbol-function
                     'bookmark-bmenu-ensure-position)
                    (lambda ()
                      (push
                       'ensure
                       calls)
                      'positioned)))
           (with-temp-buffer
             (insert
              "Ada\nAugusta\n")
             (goto-char
              (point-max))
             (let ((return
                    (addressbook-bmenu-edit)))
               (list
                return
                (line-number-at-pos)
                (nreverse
                 calls))))))"##;
    let expect =
        expect![[r#"OK (positioned 2 ((edit ("Ada" (type . "addressbook"))) rebuild ensure))"#]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

use expect_test::expect;

use super::assert_addressbook_bookmark_parity;

#[test]
fn addressbook_bookmark_mail_completion_builds_exact_ranges_candidates_and_properties() {
    let elisp_form = r##"(let ((bookmark-alist
                '(("Ada Lovelace"
                   (type . "addressbook")
                   (email . "ada@example.test, countess@example.test"))
                  ("A very long contact name beyond width"
                   (type . "addressbook")
                   (email . "long@example.test"))
                  ("No Mail"
                   (type . "addressbook")
                   (email . ""))
                  ("File"
                   (type . "file")
                   (email . "ignored@example.test")))))
         (with-temp-buffer
           (insert
            "To: prefix@example.test,  ad")
           (goto-char
            (point-max))
           (let* ((result
                   (addressbook-message-complete))
                  (beg
                   (nth
                    0
                    result))
                  (end
                   (nth
                    1
                    result))
                  (collection
                   (nth
                    2
                    result)))
             (list
              beg
              end
              (buffer-substring-no-properties
               beg
               end)
              collection
              (mapcar
               (lambda (candidate)
                 (list
                  (substring-no-properties
                   candidate)
                  (get-text-property
                   (1-
                    (length
                     candidate))
                   'face
                   candidate)))
               collection)))))"##;
    let expect = expect![[
        r#"OK (27 29 "ad" (#("Ada Lovelace           ada@example.test" 23 39 (face font-lock-doc-face)) #("Ada Lovelace           countess@example.test" 23 44 (face font-lock-doc-face)) #("A very long contact na long@example.test" 23 40 (face font-lock-doc-face))) (("Ada Lovelace           ada@example.test" font-lock-doc-face) ("Ada Lovelace           countess@example.test" font-lock-doc-face) ("A very long contact na long@example.test" font-lock-doc-face)))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_mail_completion_exit_function_removes_display_name_prefix() {
    let elisp_form = r##"(let ((bookmark-alist
                '(("Ada Lovelace"
                   (type . "addressbook")
                   (email . "ada@example.test")))))
         (with-temp-buffer
           (insert
            "Cc: ad")
           (goto-char
            (point-max))
           (let* ((result
                   (addressbook-message-complete))
                  (beg
                   (nth
                    0
                    result))
                  (end
                   (nth
                    1
                    result))
                  (candidate
                   (car
                    (nth
                     2
                     result)))
                  (exit-function
                   (plist-get
                    (nthcdr
                     3
                     result)
                    :exit-function)))
             (delete-region
              beg
              end)
             (goto-char
              beg)
             (insert
              candidate)
             (funcall
              exit-function
              candidate
              'finished)
             (list
              (buffer-string)
              (point)
              (substring-no-properties
               candidate)
              (get-text-property
               (1-
                (length
                 candidate))
               'face
               candidate)))))"##;
    let expect = expect![[
        r#"OK (#("Cc: ada@example.test" 4 20 (face font-lock-doc-face)) 21 "Ada Lovelace           ada@example.test" font-lock-doc-face)"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_mail_completion_returns_nil_for_empty_or_point_ranges() {
    let elisp_form = r##"(let ((bookmark-alist
                '(("Ada"
                   (type . "addressbook")
                   (email . "ada@example.test")))))
         (mapcar
          (lambda (fixture)
            (with-temp-buffer
              (insert
               fixture)
              (goto-char
               (point-max))
              (addressbook-message-complete)))
          '(""
            "To: "
            "To: ada@example.test,"
            "To: ada@example.test,   ")))"##;
    let expect = expect!["OK (nil nil nil nil)"];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_turn_on_mail_completion_sets_exact_message_protocol() {
    let elisp_form = r##"(let ((message-tab-body-function
                'old-tab)
               (message-completion-alist
                'old-completion)
               load-calls)
         (cl-letf (((symbol-function
                     'bookmark-maybe-load-default-file)
                    (lambda ()
                      (push
                       'load
                       load-calls)
                      'loaded)))
           (let ((return
                  (addressbook-turn-on-mail-completion)))
             (list
              return
              message-tab-body-function
              message-completion-alist
              (nreverse
               load-calls)))))"##;
    let expect = expect![[
        r#"OK (#1=(("^\\(Newsgroups\\|Followup-To\\|Posted-To\\|Gcc\\):" . message-expand-group) ("^\\(Newsgroups\\|Followup-To\\|Posted-To\\|Gcc\\):" . addressbook-message-complete) ("^\\(Resent-\\)?\\(To\\|B?Cc\\):" . addressbook-message-complete) ("^\\(Reply-To\\|From\\|Mail-Followup-To\\|Mail-Copies-To\\):" . addressbook-message-complete) ("^\\(Disposition-Notification-To\\|Return-Receipt-To\\):" . addressbook-message-complete)) nil #1# (load))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

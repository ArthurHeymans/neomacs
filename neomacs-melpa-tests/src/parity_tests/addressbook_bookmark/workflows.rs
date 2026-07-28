use expect_test::expect;

use super::assert_addressbook_bookmark_parity;

/// Recording contacts through `addressbook-bookmark-set'.  The prompts show
/// the shape of the interview - name once, then group, mail, phone and web
/// asked repeatedly until answered empty, then the postal fields - and the
/// resulting bookmark records carry every field the package defines, tagged
/// with its own "addressbook" type and jump handler, so `addressbook-alist-only'
/// can tell them apart from ordinary bookmarks.  Nothing is written to disk
/// yet: only the modification counter moves.
#[test]
fn creating_contacts_records_addressbook_bookmarks() {
    let elisp_form = r##"(let ((bookmark-default-file (ab-test-path "book/contacts.bmk"))
      (bookmark-alist nil)
      (bookmark-save-flag nil)
      (bookmark-alist-modification-count 0))
  (make-directory (ab-test-path "book") t)
  (let ((first (ab-test-with-answers (copy-sequence ab-test-zoe)
                 (addressbook-bookmark-set)
                 (list (mapcar #'car bookmark-alist)
                       (ab-test-record-text "Zoë Müller")
                       (mapcar #'car (ab-test-asked))))))
    (ab-test-with-answers (copy-sequence ab-test-ann)
      (addressbook-bookmark-set))
    (list first
          (mapcar #'car bookmark-alist)
          (ab-test-record-text "Ann Smith")
          (mapcar #'car (addressbook-alist-only))
          (addressbook-bookmark-p "Zoë Müller")
          bookmark-alist-modification-count
          (file-exists-p (ab-test-path "book/contacts.bmk")))))"##;
    let expect = expect![[
        r#"OK ((("Zoë Müller") "(\"Zoë Müller\" (city . \"Köln\") (country . \"Deutschland\") (email . \"zoe@example.org, z.mueller@example.net\") (group . \"Freunde\") (handler . addressbook-bookmark-jump) (image . \"\") (last-modified <TIME>) (location . \"Addressbook entry\") (note . \"Grüße aus Köln\") (phone . \"+49 221 4711\") (position . 0) (state . \"NRW\") (street . \"Hauptstraße 7\") (type . \"addressbook\") (web . \"https://zoë.example\") (zipcode . \"50667\"))" ("Name: " "Group: " "Group: " "Mail: " "Mail: " "Mail: " "Phone: " "Phone: " "Web: " "Web: " "Street: " "City: " "State: " "Zipcode: " "Country: " "Note: " "Image path: " "`Zoë Müller' Recorded. Add a new contact? ")) ("Ann Smith" "Zoë Müller") "(\"Ann Smith\" (city . \"Springfield\") (country . \"USA\") (email . \"ann@example.com\") (group . \"Work\") (handler . addressbook-bookmark-jump) (image . \"\") (last-modified <TIME>) (location . \"Addressbook entry\") (note . \"\") (phone . \"\") (position . 0) (state . \"IL\") (street . \"12 Main Street\") (type . \"addressbook\") (web . \"\") (zipcode . \"62704\"))" ("Ann Smith" "Zoë Müller") t 2 nil)"#
    ]];

    assert_addressbook_bookmark_parity(elisp_form, expect);
}

/// The strongest assertion available for a package that stores user data: save
/// the address book, read the file back, and reload it into an emptied
/// `bookmark-alist'.  The contact's name, street, city and note are not ASCII
/// and the file itself lives under an accented name with a space and an
/// apostrophe, so this pins the decoded file name `directory-files' reports,
/// the exact text written, the UTF-8 bytes behind the accented name, the
/// absence of any `?' replacement byte, and that the reloaded record is
/// identical to the saved one.
#[test]
fn the_bookmark_file_round_trips_a_non_ascii_contact_and_filename() {
    let elisp_form = r##"(let* ((directory (ab-test-path "carnet d'adresses"))
       (file (expand-file-name "répertoire.bmk" directory))
       (bookmark-default-file file)
       (bookmark-alist nil)
       (bookmark-save-flag nil)
       (bookmark-alist-modification-count 0))
  (make-directory directory t)
  (ab-test-with-answers (copy-sequence ab-test-zoe) (addressbook-bookmark-set))
  (ab-test-with-answers (copy-sequence ab-test-ann) (addressbook-bookmark-set))
  (bookmark-save)
  (let ((saved (ab-test-record-text "Zoë Müller"))
        (contents (ab-test-file-contents file))
        (bytes (ab-test-file-bytes file)))
    (setq bookmark-alist nil)
    (bookmark-load file t t)
    (list (list (file-exists-p file)
                (directory-files directory)
                (mapcar #'multibyte-string-p (directory-files directory))
                bookmark-file-coding-system)
          (ab-test-normalize contents)
          (list (and (string-search "?" bytes) t)
                (append (string-to-list
                         (let ((start (string-search "Zo" bytes)))
                           (substring bytes start (+ start 14))))
                        nil))
          (mapcar #'car bookmark-alist)
          (equal saved (ab-test-record-text "Zoë Müller"))
          (ab-test-record-text "Zoë Müller"))))"##;
    let expect = expect![[
        r#"OK ((t ("." ".." "répertoire.bmk") (nil nil t) utf-8-emacs-unix) ";;;; Emacs Bookmark Format Version 1;;;; -*- coding: utf-8-emacs; mode: lisp-data -*-\n;;; This format is meant to be slightly human-readable;\n;;; nevertheless, you probably don't want to edit it.\n;;; -*- End Of Bookmark File Format Version Stamp -*-\n((\"Ann Smith\"\n  (position . 0)\n  (last-modified <TIME>)\n  (type . \"addressbook\")\n  (location . \"Addressbook entry\")\n  (image . \"\")\n  (email . \"ann@example.com\")\n  (phone . \"\")\n  (web . \"\")\n  (street . \"12 Main Street\")\n  (city . \"Springfield\")\n  (state . \"IL\")\n  (zipcode . \"62704\")\n  (country . \"USA\")\n  (note . \"\")\n  (group . \"Work\")\n  (handler . addressbook-bookmark-jump))\n(\"Zoë Müller\"\n (position . 0)\n (last-modified <TIME>)\n (type . \"addressbook\")\n (location . \"Addressbook entry\")\n (image . \"\")\n (email . \"zoe@example.org, z.mueller@example.net\")\n (phone . \"+49 221 4711\")\n (web . \"https://zoë.example\")\n (street . \"Hauptstraße 7\")\n (city . \"Köln\")\n (state . \"NRW\")\n (zipcode . \"50667\")\n (country . \"Deutschland\")\n (note . \"Grüße aus Köln\")\n (group . \"Freunde\")\n (handler . addressbook-bookmark-jump))\n)\n" (nil (90 111 195 171 32 77 195 188 108 108 101 114 34 10)) ("Ann Smith" "Zoë Müller") t "(\"Zoë Müller\" (city . \"Köln\") (country . \"Deutschland\") (email . \"zoe@example.org, z.mueller@example.net\") (group . \"Freunde\") (handler . addressbook-bookmark-jump) (image . \"\") (last-modified <TIME>) (location . \"Addressbook entry\") (note . \"Grüße aus Köln\") (phone . \"+49 221 4711\") (position . 0) (state . \"NRW\") (street . \"Hauptstraße 7\") (type . \"addressbook\") (web . \"https://zoë.example\") (zipcode . \"50667\"))")"#
    ]];

    assert_addressbook_bookmark_parity(elisp_form, expect);
}

/// The same round trip for a user whose `bookmark-file-coding-system' is
/// latin-1 - an address book written before UTF-8 was the default.  Every
/// character here is representable in latin-1, so the file must contain single
/// latin-1 bytes for the accented letters, no `?' replacements, and reloading
/// must give back exactly the same contact.
#[test]
fn a_latin_1_bookmark_file_keeps_its_accented_contact() {
    let elisp_form = r##"(let* ((directory (ab-test-path "carnet"))
       (file (expand-file-name "adressen.bmk" directory))
       (bookmark-default-file file)
       (bookmark-alist nil)
       (bookmark-save-flag nil)
       (bookmark-file-coding-system 'latin-1)
       (bookmark-alist-modification-count 0))
  (make-directory directory t)
  (ab-test-with-answers (copy-sequence ab-test-zoe) (addressbook-bookmark-set))
  (bookmark-save)
  (let* ((bytes (ab-test-file-bytes file))
         (saved (ab-test-record-text "Zoë Müller")))
    (setq bookmark-alist nil)
    (bookmark-load file t t)
    (list (and (string-search "?" bytes) t)
          (append (string-to-list (substring bytes (or (string-search "Zo" bytes) 0)
                                             (+ 12 (or (string-search "Zo" bytes) 0))))
                  nil)
          bookmark-file-coding-system
          (mapcar #'car bookmark-alist)
          (equal saved (ab-test-record-text (caar bookmark-alist)))
          (assoc-default 'city (car bookmark-alist)))))"##;
    let expect = expect![[
        r#"OK (nil (90 111 235 32 77 252 108 108 101 114 34 10) iso-latin-1-unix ("Zoë Müller") t "Köln")"#
    ]];

    assert_addressbook_bookmark_parity(elisp_form, expect);
}

/// `addressbook-jump' on two contacts renders the address book: a header
/// naming the user, a separator between contacts, one labelled line per
/// non-empty field - the second contact has no phone, web or note, and those
/// lines are simply absent - and a `name' text property on each Name label,
/// which is how `addressbook-get-contact-data' finds the contact at point in
/// the read-only buffer.
#[test]
fn the_addressbook_buffer_renders_every_recorded_field() {
    let elisp_form = r##"(let ((bookmark-default-file (ab-test-path "book/contacts.bmk"))
      (bookmark-alist nil)
      (bookmark-save-flag nil)
      (bookmark-alist-modification-count 0)
      (user-login-name "melpa-test"))
  (make-directory (ab-test-path "book") t)
  (ab-test-with-answers (copy-sequence ab-test-zoe) (addressbook-bookmark-set))
  (ab-test-with-answers (copy-sequence ab-test-ann) (addressbook-bookmark-set))
  (unwind-protect
      (progn
        (addressbook-jump (list "Zoë Müller" "Ann Smith"))
        (with-current-buffer addressbook-buffer-name
          (list (buffer-name)
                major-mode
                buffer-read-only
                (buffer-substring-no-properties (point-min) (point-max))
                (list (get-text-property (point-min) 'face)
                      (save-excursion (goto-char (point-min))
                                      (search-forward "Name:")
                                      (list (get-text-property (- (point) 1) 'name)
                                            (get-text-property (- (point) 1) 'face))))
                (point)
                (car (addressbook-get-contact-data)))))
    (when (get-buffer addressbook-buffer-name)
      (kill-buffer addressbook-buffer-name))))"##;
    let expect = expect![[
        r#"OK ("*addressbook*" addressbook-mode t "Addressbook Melpa-Test\n\n---------------------------------------------\nName:    Zoë Müller\nGroup:   Freunde\nMail:    zoe@example.org, z.mueller@example.net\nPhone:   +49 221 4711\nWeb:     https://zoë.example\nStreet:  Hauptstraße 7\nCity:    Köln\nState:   NRW\nZipcode: 50667\nCountry: Deutschland\nNote:    Grüße aus Köln\n---------------------------------------------\nName:    Ann Smith\nGroup:   Work\nMail:    ann@example.com\nStreet:  12 Main Street\nCity:    Springfield\nState:   IL\nZipcode: 62704\nCountry: USA\n---------------------------------------------\n" (((:foreground "green" :underline t)) ("Zoë Müller" ((:underline t)))) 1 "Zoë Müller")"#
    ]];

    assert_addressbook_bookmark_parity(elisp_form, expect);
}

/// Pressing `e' in the address book edits the contact under point.  Each
/// prompt arrives with the stored value as its initial input - recorded here,
/// so the edit form is pinned field by field - and answering them rewrites the
/// bookmark record and re-renders the buffer with the new address, leaving the
/// other contact untouched.
#[test]
fn editing_a_contact_offers_its_values_and_rewrites_the_entry() {
    let elisp_form = r##"(let ((bookmark-default-file (ab-test-path "book/contacts.bmk"))
      (bookmark-alist nil)
      (bookmark-save-flag nil)
      (bookmark-alist-modification-count 0)
      (user-login-name "melpa-test"))
  (make-directory (ab-test-path "book") t)
  (ab-test-with-answers (copy-sequence ab-test-zoe) (addressbook-bookmark-set))
  (ab-test-with-answers (copy-sequence ab-test-ann) (addressbook-bookmark-set))
  (unwind-protect
      (progn
        (addressbook-jump (list "Zoë Müller" "Ann Smith"))
        (with-current-buffer addressbook-buffer-name
          (set-window-buffer (selected-window) (current-buffer))
          (goto-char (point-min))
          (search-forward "Name:    Zoë")
          (let ((edited (ab-test-with-answers (copy-sequence ab-test-zoe-edit)
                          (execute-kbd-macro (kbd "e"))
                          (ab-test-asked))))
            (list (key-binding (kbd "e"))
                  edited
                  (ab-test-record-text "Zoë Müller")
                  (buffer-substring-no-properties (point-min) (point-max))
                  bookmark-alist-modification-count))))
    (when (get-buffer addressbook-buffer-name)
      (kill-buffer addressbook-buffer-name))))"##;
    let expect = expect![[
        r#"OK (addressbook-edit (("Name: " . "Zoë Müller") ("Group: " . "Freunde") ("Mail: " . "zoe@example.org, z.mueller@example.net") ("Phone: " . "+49 221 4711") ("Web: " . "https://zoë.example") ("Street: " . "Hauptstraße 7") ("City: " . "Köln") ("State: " . "NRW") ("Zipcode: " . "50667") ("Country: " . "Deutschland") ("Note: " . "Grüße aus Köln") ("Image path: " . "") ("Save changes? " . :y-or-n-p)) "(\"Zoë Müller\" (city . \"München\") (country . \"Deutschland\") (email . \"zoe@example.org\") (group . \"Freunde\") (handler . addressbook-bookmark-jump) (image . \"\") (last-modified <TIME>) (location . \"Addressbook entry\") (note . \"Umgezogen\") (phone . \"+49 221 4711\") (position . 0) (state . \"Bayern\") (street . \"Sendlinger Straße 1\") (type . \"addressbook\") (web . \"https://zoë.example\") (zipcode . \"80331\"))" "Addressbook Melpa-Test\n\n---------------------------------------------\nName:    Zoë Müller\nGroup:   Freunde\nMail:    zoe@example.org\nPhone:   +49 221 4711\nWeb:     https://zoë.example\nStreet:  Sendlinger Straße 1\nCity:    München\nState:   Bayern\nZipcode: 80331\nCountry: Deutschland\nNote:    Umgezogen\n---------------------------------------------\nName:    Ann Smith\nGroup:   Work\nMail:    ann@example.com\nStreet:  12 Main Street\nCity:    Springfield\nState:   IL\nZipcode: 62704\nCountry: USA\n---------------------------------------------\n" 3)"#
    ]];

    assert_addressbook_bookmark_parity(elisp_form, expect);
}

/// Contacts are deleted with Emacs's own `bookmark-delete', and that part
/// works: the entry leaves `bookmark-alist', the saved file no longer mentions
/// it, and a reload confirms it is gone.  Refreshing the open address book with
/// `g' afterwards does not: `addressbook-mode-revert' re-renders every name it
/// finds in the buffer text, the deleted one no longer has a record, and the
/// package signals `(wrong-type-argument stringp nil)' - leaving the buffer
/// without its header and with only the surviving contact.
#[test]
fn deleting_a_contact_leaves_the_buffer_refresh_broken() {
    let elisp_form = r##"(let ((bookmark-default-file (ab-test-path "book/contacts.bmk"))
      (bookmark-alist nil)
      (bookmark-save-flag nil)
      (bookmark-alist-modification-count 0)
      (user-login-name "melpa-test"))
  (make-directory (ab-test-path "book") t)
  (ab-test-with-answers (copy-sequence ab-test-zoe) (addressbook-bookmark-set))
  (ab-test-with-answers (copy-sequence ab-test-ann) (addressbook-bookmark-set))
  (unwind-protect
      (progn
        (addressbook-jump (list "Zoë Müller" "Ann Smith"))
        (bookmark-delete "Ann Smith")
        (bookmark-save)
        (let ((after-delete (list (mapcar #'car bookmark-alist)
                                  (mapcar #'car (addressbook-alist-only))
                                  (assoc "Ann Smith" bookmark-alist)
                                  (and (string-search "Ann Smith"
                                                      (ab-test-file-contents
                                                       (ab-test-path "book/contacts.bmk")))
                                       t))))
          (setq bookmark-alist nil)
          (bookmark-load (ab-test-path "book/contacts.bmk") t t)
          (let ((reloaded (mapcar #'car bookmark-alist)))
            (with-current-buffer addressbook-buffer-name
              (set-window-buffer (selected-window) (current-buffer))
              (goto-char (point-min))
              (list after-delete
                    reloaded
                    (condition-case error
                        (progn (execute-kbd-macro (kbd "g")) :reverted)
                      (error error))
                    (buffer-substring-no-properties (point-min) (point-max)))))))
    (when (get-buffer addressbook-buffer-name)
      (kill-buffer addressbook-buffer-name))))"##;
    let expect = expect![[
        r#"OK ((("Zoë Müller") ("Zoë Müller") nil nil) ("Zoë Müller") (wrong-type-argument stringp nil) "Name:    Zoë Müller\nGroup:   Freunde\nMail:    zoe@example.org, z.mueller@example.net\nPhone:   +49 221 4711\nWeb:     https://zoë.example\nStreet:  Hauptstraße 7\nCity:    Köln\nState:   NRW\nZipcode: 50667\nCountry: Deutschland\nNote:    Grüße aus Köln\n---------------------------------------------\n")"#
    ]];

    assert_addressbook_bookmark_parity(elisp_form, expect);
}

/// `addressbook-turn-on-mail-completion' wires the address book into
/// `message-mode' completion.  On a `To:' line, TAB after a name that matches
/// one contact completes straight to its address - the display name is trimmed
/// away by the completion's exit function - while a name with two recorded
/// addresses completes as far as their common prefix and a second TAB lists
/// both.
#[test]
fn mail_completion_offers_the_recorded_addresses() {
    let elisp_form = r##"(let ((bookmark-default-file (ab-test-path "book/contacts.bmk"))
      (bookmark-alist nil)
      (bookmark-save-flag nil)
      (bookmark-alist-modification-count 0))
  (make-directory (ab-test-path "book") t)
  (ab-test-with-answers (copy-sequence ab-test-zoe) (addressbook-bookmark-set))
  (ab-test-with-answers (copy-sequence ab-test-ann) (addressbook-bookmark-set))
  (addressbook-turn-on-mail-completion)
  (let ((buffer (generate-new-buffer "*addressbook-mail*")))
    (unwind-protect
        (with-current-buffer buffer
          (set-window-buffer (selected-window) buffer)
          (message-mode)
          (insert "To: \nCc: \nSubject: \n--text follows this line--\n")
          (goto-char (point-min))
          (end-of-line)
          (let ((before-binding (key-binding (kbd "TAB"))))
          (insert "Ann")
          (let ((candidates (nth 2 (addressbook-message-complete))))
            (execute-kbd-macro (kbd "TAB"))
            (let ((unique (list (buffer-substring-no-properties
                                 (line-beginning-position) (line-end-position))
                                (point))))
              (forward-line 1)
              (end-of-line)
              (insert "Zo")
              (execute-kbd-macro (kbd "TAB"))
              (execute-kbd-macro (kbd "TAB"))
              (list (mapcar #'car message-completion-alist)
                    (list before-binding (key-binding (kbd "TAB")))
                    candidates
                    unique
                    (buffer-substring-no-properties
                     (line-beginning-position) (line-end-position))
                    (and (get-buffer "*Completions*")
                         (with-current-buffer "*Completions*"
                           (buffer-substring-no-properties (point-min) (point-max)))))))))
      (kill-buffer buffer)
      (when (get-buffer "*Completions*") (kill-buffer "*Completions*")))))"##;
    let expect = expect![[
        r#"OK (("^\\(Newsgroups\\|Followup-To\\|Posted-To\\|Gcc\\):" "^\\(Newsgroups\\|Followup-To\\|Posted-To\\|Gcc\\):" "^\\(Resent-\\)?\\(To\\|B?Cc\\):" "^\\(Reply-To\\|From\\|Mail-Followup-To\\|Mail-Copies-To\\):" "^\\(Disposition-Notification-To\\|Return-Receipt-To\\):") (message-tab completion-at-point) (#("Ann Smith              ann@example.com" 23 38 (face font-lock-doc-face)) #("Zoë Müller             zoe@example.org" 23 38 (face font-lock-doc-face)) #("Zoë Müller             z.mueller@example.net" 23 44 (face font-lock-doc-face))) ("To: ann@example.com" 20) "Cc: Zoë Müller             z" "Type M-RET on a completion to select it.\nType M-<down> or M-<up> to move point between completions.\n\n2 possible completions:\nZoë Müller             z.mueller@example.net\nZoë Müller             zoe@example.org")"#
    ]];

    assert_addressbook_bookmark_parity(elisp_form, expect);
}

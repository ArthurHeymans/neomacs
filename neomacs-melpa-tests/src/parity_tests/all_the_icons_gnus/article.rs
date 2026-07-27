use expect_test::expect;

use super::assert_all_the_icons_gnus_parity;

#[test]
fn ordinary_rfc_style_article_headers_remain_textually_and_property_identical() {
    let elisp_form = r##"
(with-temp-buffer
  (insert
   "From: Alice <alice@example.invalid>\n"
   "Subject: Quarterly report\n"
   "To: Bob <bob@example.invalid>\n"
   "CC: Team <team@example.invalid>\n"
   "Reply-To: replies@example.invalid\n"
   "Date: Mon, 01 Jan 2024 12:34:00 +0000\n"
   "Organization: Example & Co.\n"
   "Content-Type: text/plain; charset=utf-8\n"
   "User-Agent: Gnus\n"
   "X-mailer: NeoMacs\n"
   "X-PGP-Fingerprint: ABCD 1234\n")
  (set-buffer-modified-p nil)
  (let ((result (all-the-icons-gnus--add-faces)))
    (list
     result
     (buffer-string)
     (buffer-modified-p)
     (text-properties-at (point-min))
     (next-property-change
      (point-min)
      nil
      (point-max)))))
"##;
    let expect = expect![[
        r#"OK (nil "From: Alice <alice@example.invalid>\nSubject: Quarterly report\nTo: Bob <bob@example.invalid>\nCC: Team <team@example.invalid>\nReply-To: replies@example.invalid\nDate: Mon, 01 Jan 2024 12:34:00 +0000\nOrganization: Example & Co.\nContent-Type: text/plain; charset=utf-8\nUser-Agent: Gnus\nX-mailer: NeoMacs\nX-PGP-Fingerprint: ABCD 1234\n" nil nil 329)"#
    ]];
    assert_all_the_icons_gnus_parity(elisp_form, expect);
}

#[test]
fn every_registered_sanitized_header_is_composed_with_exact_icon_and_face() {
    let elisp_form = r##"
(with-temp-buffer
  (dolist (header
           '("From:  : Alice"
             "Subject:  : Quarterly report"
             "To:  : Bob"
             "CC:  : Team"
             "Reply-To:  : replies@example.invalid"
             "Date:  : 2024-01-01"
             "Organization:  : Example"
             "Content-Type:  : text/plain"
             "User-Agent:  : Gnus"
             "X-mailer:  : NeoMacs"
             "X-PGP-Fingerprint:  : ABCD"))
    (insert header "\n"))
  (all-the-icons-gnus--add-faces)
  (mapcar
   (lambda (entry)
     (goto-char (point-min))
     (when (re-search-forward (car entry) nil t)
       (let ((start (match-beginning 1))
             (end (match-end 1)))
         (list
          (match-string-no-properties 1)
          (buffer-substring-no-properties start end)
          (all-the-icons-gnus-test-properties-at start)
          (buffer-substring-no-properties
           end
           (line-end-position))))))
   pretty-gnus-article-alist))
"##;
    let expect = expect![[
        r##"OK (("X-PGP-Fingerprint:  : " "X-PGP-Fingerprint:  : " (:face (:foreground "#375E97") :composition (22 "" (:family "FontAwesome" :height 1.2) (raise -0.24))) "ABCD") ("X-mailer:  : " "X-mailer:  : " (:face (:foreground "#375E97") :composition (13 "" (:family "FontAwesome" :height 1.2) (raise -0.24))) "NeoMacs") ("User-Agent:  : " "User-Agent:  : " (:face (:foreground "#375E97") :composition (15 "" (:family "FontAwesome" :height 1.2) (raise -0.24))) "Gnus") ("Content-Type:  : " "Content-Type:  : " (:face (:foreground "#375E97") :composition (17 "" (:family "FontAwesome" :height 1.2) (raise -0.24))) "text/plain") ("Organization:  : " "Organization:  : " (:face (:foreground "#375E97") :composition (17 "" (:family "FontAwesome" :height 1.2) (raise -0.24))) "Example") ("Date:  : " "Date:  : " (:face (:foreground "#375E97") :composition (9 "" (:family "FontAwesome" :height 1.2) (raise -0.24))) "2024-01-01") ("Reply-To:  : " "Reply-To:  : " (:face (:foreground "#375E97") :composition (13 "" (:family "FontAwesome" :height 1.2) (raise -0.24))) "replies@example.invalid") ("CC:  : " "CC:  : " (:face (:foreground "#375E97") :composition (7 "" (:family "github-octicons" :height 1.2) (raise -0.24))) "Team") ("To:  : " "To:  : " (:face (:foreground "#375E97") :composition (7 "" (:family "FontAwesome" :height 1.2) (raise -0.24))) "Bob") ("Subject:  : " "Subject:  : " (:face (:foreground "#375E97") :composition (12 "" (:family "FontAwesome" :height 1.2) (raise -0.24))) "Quarterly report") ("From:  : " "From:  : " (:face (:foreground "#375E97") :composition (9 "" (:family "FontAwesome" :height 1.2) (raise -0.24))) "Alice"))"##
    ]];
    assert_all_the_icons_gnus_parity(elisp_form, expect);
}

#[test]
fn composition_changes_only_header_token_and_preserves_unicode_payload_and_newlines() {
    let elisp_form = r##"
(with-temp-buffer
  (insert
   "From:  : Żaneta <żaneta@example.invalid>\n"
   "Subject:  : Zażółć gęślą jaźń — release ✓\n"
   "\nBody remains untouched & literal.\n")
  (let ((before (buffer-string)))
    (all-the-icons-gnus--add-faces)
    (list
     before
     (buffer-substring-no-properties
      (point-min)
      (point-max))
     (equal
      before
      (buffer-substring-no-properties
       (point-min)
       (point-max)))
     (let ((position (point-min)))
       (list
        (all-the-icons-gnus-test-properties-at position)
        (next-single-property-change
         position 'composition nil (point-max))))
     (progn
       (goto-char (point-min))
       (forward-line 1)
       (all-the-icons-gnus-test-properties-at (point)))
     (progn
       (goto-char (point-min))
       (search-forward "Body")
       (text-properties-at (- (point) (length "Body")))))))
"##;
    let expect = expect![[
        r##"OK ("From:  : Żaneta <żaneta@example.invalid>\nSubject:  : Zażółć gęślą jaźń — release ✓\n\nBody remains untouched & literal.\n" "From:  : Żaneta <żaneta@example.invalid>\nSubject:  : Zażółć gęślą jaźń — release ✓\n\nBody remains untouched & literal.\n" t ((:face (:foreground "#375E97") :composition (9 "" (:family "FontAwesome" :height 1.2) (raise -0.24))) 10) (:face (:foreground "#375E97") :composition (12 "" (:family "FontAwesome" :height 1.2) (raise -0.24))) nil)"##
    ]];
    assert_all_the_icons_gnus_parity(elisp_form, expect);
}

#[test]
fn repeated_header_occurrences_are_all_composed_in_one_pass() {
    let elisp_form = r##"
(with-temp-buffer
  (insert
   "From:  : First\n"
   "From:  : Second\n"
   "From:  : Third\n")
  (all-the-icons-gnus--add-faces)
  (let ((regexp
         (car
          (cl-find-if
           (lambda (entry)
             (string-match-p "From" (car entry)))
           pretty-gnus-article-alist)))
        occurrences)
    (goto-char (point-min))
    (while (re-search-forward regexp nil t)
      (let ((position (match-beginning 1)))
       (push
        (list
         (line-number-at-pos position)
         (all-the-icons-gnus-test-properties-at position))
        occurrences)))
    (nreverse occurrences)))
"##;
    let expect = expect![[
        r##"OK ((1 (:face (:foreground "#375E97") :composition (9 "" (:family "FontAwesome" :height 1.2) (raise -0.24)))) (2 (:face (:foreground "#375E97") :composition (9 "" (:family "FontAwesome" :height 1.2) (raise -0.24)))) (3 (:face (:foreground "#375E97") :composition (9 "" (:family "FontAwesome" :height 1.2) (raise -0.24)))))"##
    ]];
    assert_all_the_icons_gnus_parity(elisp_form, expect);
}

#[test]
fn header_search_respects_case_fold_search_for_lowercase_article_fields() {
    let elisp_form = r##"
(mapcar
 (lambda (case-fold)
   (with-temp-buffer
     (insert "from:  : lowercase sender\n")
     (let ((case-fold-search case-fold))
       (all-the-icons-gnus--add-faces)
       (list
        case-fold
        (all-the-icons-gnus-test-properties-at
         (point-min))))))
 '(t nil))
"##;
    let expect = expect![[
        r##"OK ((t (:face (:foreground "#375E97") :composition (9 "" (:family "FontAwesome" :height 1.2) (raise -0.24)))) (nil (:face nil :composition nil)))"##
    ]];
    assert_all_the_icons_gnus_parity(elisp_form, expect);
}

#[test]
fn narrowing_limits_composition_to_accessible_article_region() {
    let elisp_form = r##"
(with-temp-buffer
  (insert
   "From:  : Outside before\n"
   "Subject:  : Inside\n"
   "From:  : Outside after\n")
  (let ((inside-start
         (progn
           (goto-char (point-min))
           (forward-line 1)
           (point)))
        (inside-end
         (progn
           (forward-line 1)
           (point))))
    (save-restriction
      (narrow-to-region inside-start inside-end)
      (all-the-icons-gnus--add-faces))
    (list
     (all-the-icons-gnus-test-properties-at
      (point-min))
     (all-the-icons-gnus-test-properties-at
      inside-start)
     (all-the-icons-gnus-test-properties-at
      inside-end)
     (buffer-substring-no-properties
      (point-min)
      (point-max)))))
"##;
    let expect = expect![[
        r##"OK ((:face nil :composition nil) (:face (:foreground "#375E97") :composition (12 "" (:family "FontAwesome" :height 1.2) (raise -0.24))) (:face nil :composition nil) "From:  : Outside before\nSubject:  : Inside\nFrom:  : Outside after\n")"##
    ]];
    assert_all_the_icons_gnus_parity(elisp_form, expect);
}

#[test]
fn repeated_face_pass_is_idempotent_for_composition_and_face_properties() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "Date:  : 2024-01-01\n")
  (all-the-icons-gnus--add-faces)
  (let ((first
         (list
          (buffer-substring-no-properties
           (point-min)
           (point-max))
          (all-the-icons-gnus-test-properties-at
           (point-min)))))
    (all-the-icons-gnus--add-faces)
    (list
     first
     (buffer-substring-no-properties
      (point-min)
      (point-max))
     (all-the-icons-gnus-test-properties-at
      (point-min))
     (equal
      (cadr first)
      (all-the-icons-gnus-test-properties-at
       (point-min))))))
"##;
    let expect = expect![[
        r##"OK (("Date:  : 2024-01-01\n" (:face (:foreground "#375E97") :composition (9 "" (:family "FontAwesome" :height 1.2) (raise -0.24)))) "Date:  : 2024-01-01\n" (:face (:foreground "#375E97") :composition (9 "" (:family "FontAwesome" :height 1.2) (raise -0.24))) t)"##
    ]];
    assert_all_the_icons_gnus_parity(elisp_form, expect);
}

#[test]
fn silent_modifications_preserve_read_only_modified_and_undo_state() {
    let elisp_form = r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "Organization:  : Example\n")
  (set-buffer-modified-p nil)
  (setq buffer-read-only t)
  (let ((undo-before buffer-undo-list)
        (modified-before (buffer-modified-p)))
    (list
     (all-the-icons-gnus--add-faces)
     modified-before
     (buffer-modified-p)
     (equal undo-before buffer-undo-list)
     buffer-read-only
     (all-the-icons-gnus-test-properties-at
      (point-min)))))
"##;
    let expect = expect![[
        r##"OK (nil nil nil t t (:face (:foreground "#375E97") :composition (17 "" (:family "FontAwesome" :height 1.2) (raise -0.24))))"##
    ]];
    assert_all_the_icons_gnus_parity(elisp_form, expect);
}

#[test]
fn interactive_article_face_command_reports_interactive_form_and_applies_properties() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "User-Agent:  : Gnus\n")
  (let ((commandp-before
         (commandp 'all-the-icons-gnus--add-faces))
        (interactive
         (interactive-form
          'all-the-icons-gnus--add-faces)))
    (call-interactively 'all-the-icons-gnus--add-faces)
    (list
     commandp-before
     interactive
     (all-the-icons-gnus-test-properties-at
      (point-min)))))
"##;
    let expect = expect![[
        r##"OK (t (interactive nil) (:face (:foreground "#375E97") :composition (15 "" (:family "FontAwesome" :height 1.2) (raise -0.24))))"##
    ]];
    assert_all_the_icons_gnus_parity(elisp_form, expect);
}

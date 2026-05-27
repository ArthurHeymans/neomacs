use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_pcomplete_case_command_at_point_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-pcomplete)
  (with-temp-buffer
    (org-mode)
    (insert "#+STARTUP: fold\n")
    (insert "#+PROPERTY: Effort_ALL 0:15 0:30\n")
    (insert "* TODO Heading\n")
    (insert ":PROPERTIES:\n:Effort: 0:15\n:END:\n")
    (goto-char (point-min))
    (search-forward "STARTUP")
    (list (org-pcomplete-case-double '("todo" "done" "Wait"))
          (org-thing-at-point)
          (org-command-at-point))))"##,
    );
}

#[test]
fn org_ctags_lookup_replace_tag_table_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-ctags)
  (let* ((root (make-temp-file "org-ctags" t))
         (topic (expand-file-name "topic.org" root))
         (tags (expand-file-name "TAGS" root))
         (tags-file-name tags))
    (unwind-protect
        (progn
          (with-temp-file topic
            (insert "* Alpha\nBody\n* Beta\nBody\n"))
          (with-temp-file tags
            (insert "\f\n" topic ",20\n"
                    "Alpha\177Alpha\0011,1\n"
                    "Beta\177Beta\0013,14\n"))
          (let ((found (org-ctags-get-filename-for-tag "Alpha")))
            (list (org-ctags-string-search-and-replace
                   "a" "X" "abracadabra")
                  (list (file-name-nondirectory (nth 0 found))
                        (nth 1 found)
                        (nth 2 found))
                  (sort (org-ctags-all-tags-in-current-tags-table)
                        #'string<))))
      (delete-directory root t))))"#,
    );
}

#[test]
fn org_ctags_point_append_narrow_decline_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-ctags)
  (with-temp-buffer
    (org-mode)
    (insert "* Source\n")
    (insert "Plain AlphaBeta text before [[WikiTopic]] and Mixed_Word_99.\n")
    (let ((probe
           (lambda (needle offset)
             (goto-char (point-min))
             (search-forward needle)
             (forward-char offset)
             (org-ctags-find-tag-at-point))))
          (org-ctags-new-topic-template "* <<%t>>\nBody for %t.\n\n"))
      (let ((point-tags (list (funcall probe "AlphaBeta" -3)
                              (funcall probe "WikiTopic" -5)
                              (funcall probe "Mixed_Word_99" -4))))
        (goto-char (point-max))
        (let ((appended (org-ctags-append-topic "fresh topic" t))
              (narrowed (buffer-narrowed-p))
              (narrow-text (buffer-substring-no-properties
                            (point-min) (point-max)))
              (narrow-point (list (line-number-at-pos)
                                  (- (point) (point-min)))))
          (widen)
          (let ((declined
                 (cl-letf (((symbol-function 'y-or-n-p)
                            (lambda (&rest _) nil)))
                   (org-ctags-ask-append-topic "declined topic")))
                (full-text (buffer-substring-no-properties
                            (point-min) (point-max))))
            (list point-tags
                  appended
                  narrowed
                  narrow-point
                  narrow-text
                  declined
                  (string-match-p "declined topic" full-text)
                  (org-ctags-fail-silently "anything")
                  full-text))))))"#,
    );
}

#[test]
fn org_crypt_detect_encrypted_entry_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-crypt)
  (with-temp-buffer
    (org-mode)
    (insert "* Secret :crypt:\n")
    (insert ":PROPERTIES:\n:CRYPTKEY: nil\n:END:\n")
    (insert "-----BEGIN PGP MESSAGE-----\nabc\n-----END PGP MESSAGE-----\n")
    (insert "* Plain\n")
    (goto-char (point-min))
    (search-forward "Secret")
    (beginning-of-line)
    (let ((encrypted (org-at-encrypted-entry-p))
          (key (let ((org-crypt-key nil))
                 (org-crypt-key-for-heading))))
      (list (and encrypted
                 (list (- (car encrypted) (point-min))
                       (- (cdr encrypted) (point-min))))
            key
            (and encrypted
                 (org-crypt--encrypted-text
                  (car encrypted)
                  (cdr encrypted)))))))"#,
    );
}

#[test]
fn org_macs_plist_string_visibility_time_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (let ((prop-string (copy-sequence "aaBBcc"))
        (now (float-time (encode-time 0 0 12 27 5 2026))))
    (add-text-properties 2 4 '(face bold invisible org-fold-outline)
                         prop-string)
    (with-temp-buffer
      (org-mode)
      (insert "* Alpha\nVisible line\n** Hidden\nSecret line\n* Beta\n")
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (let* ((secret-pos (save-excursion
                           (goto-char (point-min))
                           (search-forward "Secret")
                           (point)))
             (visible-pos (save-excursion
                            (goto-char (point-min))
                            (search-forward "Beta")
                            (point)))
             (plist-a '(:a 1 :b 2 :drop 9))
             (plist-b '(:b override :c nil :d 4))
             (combined (org-combine-plists plist-a plist-b))
             (deleted (org-plist-delete-all combined '(:drop :c)))
             (added (org-add-props (copy-sequence "PROP")
                        '(face italic)
                      'mouse-face 'highlight
                      'help-echo "help"))
             (restricted (org-no-properties (copy-sequence prop-string) t))
             (plain (org-no-properties (copy-sequence prop-string)))
             (template
              (org-fill-template
               "%noweb-ref/%noweb/%missing/%tangle-mode"
               '(("noweb" . "N")
                 ("noweb-ref" . "NR")
                 ("tangle-mode" . "TM")
                 ("missing" . nil))))
             (escapes
              (org-replace-escapes
               "%-8a|%b|%c|%a"
               '(("%a" . "alpha")
                 ("%b" . "%a-beta")
                 ("%c" . nil)))))
        (let ((org-matcher-time-now now))
          (list (org-uniquify-alist
                 '((a 1) (b 2) (a 3 4) (c) (b 5)))
                (org-delete-all '(b d) '(a b c b d e))
                combined
                deleted
                (org-make-parameter-alist
                 '(:alpha 1 :beta two :gamma nil))
                (mapcar (lambda (s)
                          (list s
                                (org-unbracket-string "[" "]" s)
                                (org-strip-quotes s)
                                (org-shorten-string s 10)))
                        '("[inside]" "\"quoted\""
                          "short" "long words break here"))
                (org-remove-tabs "a\tbb\tc" 4)
                (org-remove-blank-lines "a\n\n  \n b\n\nc")
                (list (org-wrap "one two three four five" 9)
                      (org-wrap "one two three four five" nil 2))
                (org-remove-indentation
                 "    alpha\n      beta\n    gamma\n")
                template
                escapes
                (list (get-text-property 0 'face added)
                      (get-text-property 0 'mouse-face added)
                      (get-text-property 0 'help-echo added)
                      (org-find-text-property-in-string 'face added))
                (list restricted
                      (text-properties-at 2 restricted)
                      plain
                      (text-properties-at 2 plain))
                (list (not (null (org-invisible-p secret-pos)))
                      (not (null (org-invisible-p secret-pos t)))
                      (org-invisible-p visible-pos)
                      (save-excursion
                        (goto-char secret-pos)
                        (org-find-visible))
                      (save-excursion
                        (goto-char visible-pos)
                        (org-find-invisible)))
                (mapcar (lambda (pair)
                          (list (car pair)
                                (org-parse-time-string (cdr pair))
                                (org-parse-time-string (cdr pair) t)))
                        '((active . "<2026-05-27 Wed 13:45>")
                          (range . "<2026-05-27 Wed 13:45-15:00>")))
                (list (org-time< "<2026-05-27 Wed>" "<2026-05-28 Thu>")
                      (org-time= "<2026-05-27 Wed>" "<2026-05-27 Wed>")
                      (org-time<> "<2026-05-27 Wed>" "<2026-05-28 Thu>")
                      (org-time> 10 5)
                      (org-time<= nil 5)
                      (org-2ft "not a time"))))))))"##,
    );
}

use crate::common::return_if_neovm_enable_oracle_proptest_not_set;
use crate::common::{assert_oracle_parity, assert_oracle_parity_with_shared_tempdir};

#[test]
fn org_mobile_files_index_checksums_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-mobile)
  (let* ((root (make-temp-file "org-mobile-index" t))
         (org-directory (expand-file-name "org" root))
         (stage (expand-file-name "stage" root))
         (nested (expand-file-name "nested" org-directory))
         (main (expand-file-name "main.org" org-directory))
         (extra (expand-file-name "extra.org" nested))
         (skip (expand-file-name "skip.org" nested))
         (outside (expand-file-name "outside.org" root))
         (org-mobile-directory stage)
         (org-mobile-index-file "index.org")
         (org-mobile-files
          (list 'org-agenda-files nested outside skip))
         (org-mobile-files-exclude-regexp "skip\\.org\\'")
         (org-agenda-files (list main))
         (org-tag-alist '(("work" . ?w) ("home" . ?h)))
         (org-todo-keywords '((sequence "TODO(t)" "WAIT(w)" "|" "DONE(d)")))
         (org-mobile-allpriorities "A B C")
         (org-mobile-checksum-files nil))
    (unwind-protect
        (progn
          (make-directory nested t)
          (make-directory stage t)
          (with-temp-file main
            (insert "#+FILETAGS: :work:\n* TODO Main :alpha:\n"))
          (with-temp-file extra
            (insert "* WAIT Extra :beta:\n"))
          (with-temp-file skip
            (insert "* TODO Skip\n"))
          (with-temp-file outside
            (insert "* DONE Outside :zeta:\n"))
          (let* ((alist (org-mobile-files-alist))
                 (org-mobile-files-alist alist))
            (org-mobile-create-index-file)
            (let ((index
                   (with-temp-buffer
                     (insert-file-contents
                      (expand-file-name "index.org" stage))
                     (buffer-string))))
              (list (mapcar (lambda (entry)
                              (cons (file-relative-name (car entry) root)
                                    (cdr entry)))
                            alist)
                    (sort org-mobile-checksum-files
                          (lambda (a b) (string< (car a) (car b))))
                    index))))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_mobile_move_capture_apply_edit_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-id)
  (require 'org-mobile)
  (let* ((root (make-temp-file "org-mobile-apply" t))
         (org-directory root)
         (stage (expand-file-name "stage" root))
         (target (expand-file-name "tasks.org" root))
         (inbox (expand-file-name "from-mobile.org" root))
         (capture (expand-file-name org-mobile-capture-file stage))
         (checksums (expand-file-name "checksums.dat" stage))
         (org-mobile-directory stage)
         (org-mobile-inbox-for-pull inbox)
         (org-id-locations-file (expand-file-name "ids.el" root))
         (org-id-track-globally t)
         (org-mobile-force-mobile-change nil)
         (org-log-done nil))
    (unwind-protect
        (progn
          (make-directory stage t)
          (with-temp-file target
            (insert "#+TITLE: Tasks\n")
            (insert "* TODO Task :old:\n")
            (insert ":PROPERTIES:\n:ID: mobile-task\n:END:\n")
            (insert "Old body line\n"))
          (with-temp-file inbox (insert "* Existing inbox\n"))
          (with-temp-file capture
            (insert "* F(edit:todo) [[id:mobile-task][Task]]\n")
            (insert "** Old value\nTODO\n** New value\nDONE\n")
            (insert "* F(edit:tags) [[id:mobile-task][Task]]\n")
            (insert "** Old value\nold\n** New value\nnew:mobile\n")
            (insert "* F(edit:body) [[id:mobile-task][Task]]\n")
            (insert "** Old value\nOld body line\n")
            (insert "** New value\nNew body line\nSecond line\n"))
          (with-temp-file checksums
            (insert "00000000000000000000000000000000  mobileorg.org\n"))
          (org-id-update-id-locations (list target) t)
          (let ((marker (org-mobile-move-capture)))
            (with-current-buffer (marker-buffer marker)
              (save-restriction
                (org-mode)
                (org-mobile-apply marker (point-max))))
            (let ((target-text
                   (with-current-buffer (find-file-noselect target)
                     (buffer-substring-no-properties
                      (point-min) (point-max))))
                  (inbox-text
                   (with-current-buffer (find-file-noselect inbox)
                     (buffer-substring-no-properties
                      (point-min) (point-max))))
                  (capture-text
                   (with-temp-buffer
                     (insert-file-contents capture)
                     (buffer-string)))
                  (checksum-text
                   (with-temp-buffer
                     (insert-file-contents checksums)
                     (buffer-string))))
              (list (markerp marker)
                    (replace-regexp-in-string
                     "^#\\+LAST_MOBILE_CHANGE:.*\n"
                     "#+LAST_MOBILE_CHANGE: <time>\n"
                     target-text)
                    inbox-text
                    capture-text
                    checksum-text))))
      (when (get-file-buffer target) (kill-buffer (get-file-buffer target)))
      (when (get-file-buffer inbox) (kill-buffer (get-file-buffer inbox)))
      (when (file-exists-p org-id-locations-file)
        (delete-file org-id-locations-file))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_mobile_olp_locate_edit_refile_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-mobile)
  (let* ((root (make-temp-file "org-mobile-olp" t))
         (org-directory root)
         (file (expand-file-name "space file.org" root))
         (org-mobile-force-mobile-change '(heading priority body tags))
         (org-archive-location "::* Archived")
         (org-log-done nil))
    (unwind-protect
        (progn
          (with-temp-file file
            (insert "* Parent/One\n")
            (insert "** TODO [#B] Child :old:\n")
            (insert "Body one\n")
            (insert "* Inbox\n"))
          (with-current-buffer (find-file-noselect file)
            (org-mode)
            (goto-char (point-min))
            (search-forward "Child")
            (beginning-of-line)
            (let* ((olp (org-mobile-get-outline-path-link (point-marker)))
                   (found (org-mobile-locate-entry olp))
                   before after)
              (setq before (list olp
                                 (markerp found)
                                 (and (markerp found)
                                      (with-current-buffer (marker-buffer found)
                                        (org-get-heading t t t t)))))
              (org-mobile-edit "heading" "Different" "Renamed Child")
              (org-mobile-edit "priority" "A" "C")
              (org-mobile-edit "tags" "wrong" "new:mobile")
              (org-mobile-edit "body" "wrong"
                               "Replacement body\nwith whitespace\n")
              (setq after (buffer-substring-no-properties
                           (point-min) (point-max)))
              (goto-char (point-min))
              (search-forward "Renamed Child")
              (beginning-of-line)
              (org-mobile-edit "addheading" nil "Inserted sibling")
              (list before
                    after
                    (buffer-substring-no-properties
                     (point-min) (point-max)))))))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_mobile_escape_compare_timestamp_checksum_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-mobile)
  (let* ((root (make-temp-file "org-mobile-utils" t))
         (stage (expand-file-name "stage" root))
         (capture (expand-file-name org-mobile-capture-file stage))
         (checksums (expand-file-name "checksums.dat" stage))
         (org-mobile-directory stage)
         (org-mobile-checksum-binary "md5sum"))
    (unwind-protect
        (progn
          (make-directory stage t)
          (with-temp-file capture
            (insert "* F(edit:body) [[olp:/Parent\\/One/Child%20Two][Child Two]]\n")
            (insert "** Old value\nA body\n\n** New value\nA body\n"))
          (with-temp-file checksums
            (insert "00000000000000000000000000000000  mobileorg.org\n"))
          (let ((escape (mapcar #'org-mobile-escape-olp
                                '("Parent/One" "Space Name" "hash#tag"
                                  "percent%value")))
                (tag-same (list
                           (org-mobile-tags-same-p
                            '("work" "home") '("home" "work"))
                           (org-mobile-tags-same-p
                            '("work" "home") '("work" "other"))))
                (body-same (list
                            (org-mobile-bodies-same-p
                             "A\nB\n" "A\nB")
                            (org-mobile-bodies-same-p
                             "A\n\nB" "A\nB"))))
            (org-mobile-update-checksum-for-capture-file
             (with-temp-buffer
               (insert-file-contents capture)
               (buffer-string)))
            (with-current-buffer (find-file-noselect capture)
              (org-mode)
              (goto-char (point-min))
              (let ((read-one (org-mobile-smart-read))
                    timestamped)
                (org-mobile-timestamp-buffer (current-buffer))
                (setq timestamped
                      (replace-regexp-in-string
                       "^#\\+LAST_MOBILE_CHANGE:.*"
                       "#+LAST_MOBILE_CHANGE: <time>"
                       (buffer-substring-no-properties
                        (point-min) (point-max))))
                (save-buffer)
                (list escape
                      tag-same
                      body-same
                      read-one
                      timestamped
                      (with-temp-buffer
                        (insert-file-contents checksums)
                        (buffer-string)))))))
      (when (get-file-buffer capture)
        (kill-buffer (get-file-buffer capture)))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_mobile_push_agenda_index_checksums_hooks_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-mobile)
  (let* ((root (make-temp-file "org-mobile-push" t))
         (org-directory (expand-file-name "org" root))
         (stage (expand-file-name "stage" root))
         (tasks (expand-file-name "tasks.org" org-directory))
         (inbox (expand-file-name "from-mobile.org" org-directory))
         (events nil)
         (org-mobile-directory stage)
         (org-mobile-inbox-for-pull inbox)
         (org-mobile-index-file "index.org")
         (org-mobile-files '(org-agenda-files))
         (org-agenda-files (list tasks))
         (org-mobile-agendas '("t"))
         (org-mobile-force-id-on-agenda-items nil)
         (org-mobile-checksum-binary "md5sum")
         (org-mobile-use-encryption nil)
         (org-mobile-pre-push-hook
          (list (lambda ()
                  (push (list 'pre
                              (file-exists-p org-mobile-directory)
                              org-agenda-files)
                        events))))
         (org-mobile-post-push-hook
          (list (lambda ()
                  (push (list 'post
                              (sort (directory-files org-mobile-directory
                                                     nil "^[^.]" nil)
                                    #'string<)
                              org-mobile-checksum-files)
                        events)))))
    (unwind-protect
        (progn
          (make-directory org-directory t)
          (make-directory stage t)
          (with-temp-file inbox (insert "* Existing mobile inbox\n"))
          (with-temp-file tasks
            (insert "#+TITLE: Mobile Tasks\n")
            (insert "#+TAGS: work(w) home(h)\n")
            (insert "#+TODO: TODO NEXT WAIT | DONE\n")
            (insert "* TODO Alpha :work:\n")
            (insert "SCHEDULED: <2026-05-27 Wed>\n")
            (insert ":PROPERTIES:\n:Effort: 0:30\n:END:\n")
            (insert "Body alpha\n")
            (insert "* NEXT Beta :home:\n")
            (insert "DEADLINE: <2026-05-28 Thu>\n"))
          (let ((org-agenda-start-day "2026-05-27")
                (org-agenda-span 3)
                (org-agenda-use-time-grid nil)
                (org-agenda-show-all-dates nil)
                (org-agenda-prefix-format "%-8:c%?-12t% s")
                (org-agenda-window-setup 'current-window)
                (messages nil))
            (cl-letf (((symbol-function 'message)
                       (lambda (fmt &rest args)
                         (push (apply #'format fmt args) messages))))
              (org-mobile-push)
              (let ((stage-files
                     (sort (directory-files stage nil "^[^.]" nil)
                           #'string<))
                    (read-stage
                     (lambda (file)
                       (with-temp-buffer
                         (insert-file-contents
                          (expand-file-name file stage))
                         (buffer-string)))))
                (list (nreverse events)
                      (nreverse messages)
                      stage-files
                      (funcall read-stage "index.org")
                      (replace-regexp-in-string
                       "\\[[0-9][^]\n]+\\]"
                       "[stamp]"
                       (funcall read-stage "agendas.org"))
                      (funcall read-stage "tasks.org")
                      (funcall read-stage "mobileorg.org")
                      (sort
                       (split-string
                        (funcall read-stage "checksums.dat") "\n" t)
                       #'string<)))))))
      (dolist (file (list tasks inbox))
        (when (get-file-buffer file) (kill-buffer (get-file-buffer file))))
      (when (get-buffer "*SUMO*") (kill-buffer "*SUMO*"))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_mobile_apply_refile_delete_archive_flag_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_shared_tempdir(
        r##"(progn
  (require 'org)
  (require 'org-archive)
  (require 'org-mobile)
  (let* ((root (file-name-as-directory (getenv "NEOVM_ORACLE_TEST_TMPDIR")))
         (org-directory root)
         (file (expand-file-name "tasks.org" root))
         (capture (expand-file-name "mobileorg.org" root))
         (org-mobile-force-mobile-change t)
         (org-archive-location "::* Archived")
         (org-log-done nil)
         messages)
    (unwind-protect
        (progn
          (with-temp-file file
            (insert "#+TITLE: Mobile Actions\n")
            (insert "* Source\n")
            (insert "** TODO Refile me :old:\nBody refile\n")
            (insert "** TODO Delete me\nBody delete\n")
            (insert "** DONE Archive sibling\nBody archive\n")
            (insert "** TODO Flag me\nBody flag\n")
            (insert "* Target\n")
            (insert "** Existing child\n")
            (insert "* Tail\n"))
          (with-temp-file capture
            (insert "* F(refile:ignored) [[olp:tasks.org:/Source/Refile%20me][Refile me]]\n")
            (insert "** Old value\nignored\n")
            (insert "** New value\nolp:tasks.org:/Target\n")
            (insert "* F(delete:ignored) [[olp:tasks.org:/Source/Delete%20me][Delete me]]\n")
            (insert "** Old value\nignored\n** New value\nignored\n")
            (insert "* F(archive-sibling:ignored) [[olp:tasks.org:/Source/Archive%20sibling][Archive sibling]]\n")
            (insert "** Old value\nignored\n** New value\nignored\n")
            (insert "* F() [[olp:tasks.org:/Source/Flag%20me][Flag me]]\n")
            (insert "Flag note line one\nFlag note line two\n")
            (insert "* F(delete:ignored) [[olp:tasks.org:/Missing][Missing]]\n")
            (insert "** Old value\nignored\n** New value\nignored\n")
            (insert "* New mobile capture\nCaptured body\n"))
          (with-current-buffer (find-file-noselect capture)
            (org-mode)
            (cl-letf (((symbol-function 'message)
                       (lambda (fmt &rest args)
                         (push (apply #'format fmt args) messages)))
                      ((symbol-function 'sit-for) (lambda (&rest _) nil)))
              (org-mobile-apply (point-min) (point-max)))
            (let ((capture-after
                   (buffer-substring-no-properties
                    (point-min) (point-max))))
              (with-current-buffer (find-file-noselect file)
                (org-mode)
                (let ((text (replace-regexp-in-string
                             "^#\\+LAST_MOBILE_CHANGE:.*\n"
                             "#+LAST_MOBILE_CHANGE: <time>\n"
                             (buffer-substring-no-properties
                              (point-min) (point-max)))))
                  (list (nreverse messages)
                        capture-after
                        text
                        org-mobile-last-flagged-files
                        (org-map-entries
                         (lambda ()
                           (list (org-get-heading t t t t)
                                 (org-current-level)
                                 (org-get-tags nil t)
                                 (org-entry-get nil "THEFLAGGINGNOTE")
                                 (org-entry-get nil "ARCHIVE_TIME")))
                          nil nil)))))))
      (dolist (path (list file capture))
        (when (get-file-buffer path) (kill-buffer (get-file-buffer path)))))))"##,
    );
}

#[test]
fn org_mobile_escape_pull_push_flagsync_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-mobile)
  (let* ((root (make-temp-file "org-mobile-deep" t))
         (file (expand-file-name "tasks.org" root))
         (org-mobile-directory root)
         (org-mobile-files (list file)))
    (unwind-protect
        (progn
          (with-temp-file file
            (insert "* TODO Alpha :work:\n")
            (insert "Alpha body.\n")
            (insert "** DONE Sub A\n")
            (insert "Sub A body.\n")
            (insert "* WAIT Beta\n")
            (insert "Beta body.\n"))
          (let ((escape-olp (org-mobile-escape-olp "A/B:C"))
                (tags-same (org-mobile-tags-same-p '("a" "b") '("b" "a")))
                (tags-diff (org-mobile-tags-same-p '("a" "b") '("a" "c")))
                (body-same (org-mobile-bodies-same-p "  A \n B  " "A\nB"))
                (body-diff (org-mobile-bodies-same-p "A\nB" "A\n C")))
            (let ((pull-result
                   (condition-case err
                       (progn (org-mobile-pull) 'ok)
                     (error (cons (car err) (cdr err))))))
              (let ((checksums-exist (file-exists-p
                                      (expand-file-name "checksums.dat" root))))
                (list escape-olp
                      tags-same
                      tags-diff
                      body-same
                      body-diff
                      pull-result
                      checksums-exist
                      (replace-regexp-in-string
                       (regexp-quote root) "<root>"
                       (buffer-substring-no-properties
                        (point-min) (point-max))))))))
      (delete-directory root t))))"##,
    );
}

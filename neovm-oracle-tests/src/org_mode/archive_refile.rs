use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_archive_subtree_file_context_properties_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-archive)
  (let* ((file (make-temp-file "org-archive-source" nil ".org"))
         (archive (concat file "_archive")))
    (unwind-protect
        (progn
          (with-temp-file file
            (insert "#+CATEGORY: Work\n")
            (insert "* Parent :client:\n")
            (insert ":PROPERTIES:\n:Owner: Ada\n:END:\n")
            (insert "** TODO Ship feature :urgent:\n")
            (insert "DEADLINE: <2026-06-01 Mon>\n")
            (insert "Body\n")
            (insert "** TODO Keep\n"))
          (with-current-buffer (find-file-noselect file)
            (org-mode)
            (let ((org-archive-location (concat archive "::* Archive"))
                  (org-archive-stamp-time nil)
                  (org-archive-subtree-add-inherited-tags t)
                  (org-archive-save-context-info '(file olpath category todo itags))
                  (org-archive-subtree-save-file-p nil))
              (goto-char (point-min))
              (search-forward "Ship feature")
              (beginning-of-line)
              (org-archive-subtree)
              (save-buffer)
              (let ((source (buffer-substring-no-properties
                             (point-min) (point-max)))
                    (archived (with-current-buffer
                                  (find-file-noselect archive)
                                (buffer-substring-no-properties
                                 (point-min) (point-max)))))
                (list source
                      (replace-regexp-in-string
                       (regexp-quote file)
                       "<source-file>"
                       archived))))))
      (dolist (buf (list (get-file-buffer file)
                         (get-file-buffer archive)))
        (when buf (kill-buffer buf)))
      (when (file-exists-p file) (delete-file file))
      (when (file-exists-p archive) (delete-file archive)))))"##,
    );
}

#[test]
fn org_refile_copy_with_logbook_and_bookmark_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-refile)
  (let ((file (make-temp-file "org-refile-copy" nil ".org"
                              "* Inbox\n** TODO Task :inbox:\nBody\n* Projects\n** Target\n")))
    (unwind-protect
        (with-current-buffer (find-file-noselect file)
          (org-mode)
          (let ((org-refile-keep t)
                (org-log-refile 'time)
                (org-log-into-drawer t))
            (goto-char (point-min))
            (search-forward "Task")
            (beginning-of-line)
            (let ((target-pos (save-excursion
                                (goto-char (point-min))
                                (search-forward "Target")
                                (line-beginning-position))))
              (org-refile nil nil (list "Target" file nil target-pos)))
            (save-buffer)
            (list (plist-get org-bookmark-names-plist :last-refile)
                  (replace-regexp-in-string
                   "- Refiled on \\[.*\\]"
                   "- Refiled on [stamp]"
                   (buffer-substring-no-properties
                    (point-min) (point-max)))))))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (when (file-exists-p file) (delete-file file)))))"##,
    );
}

#[test]
fn org_sparse_tree_match_visibility_and_map_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "#+TODO: TODO WAIT | DONE\n")
    (insert "* TODO Alpha :work:\nAlpha body\n")
    (insert "** WAIT Hidden child :work:\nChild body\n")
    (insert "* TODO Beta :home:\nBeta body\n")
    (insert "** TODO Matched child :work:\nChild body\n")
    (insert "* DONE Gamma :work:\nGamma body\n")
    (goto-char (point-min))
    (org-match-sparse-tree nil "+work+TODO=\"TODO\"")
    (list
     (org-map-entries
      (lambda ()
        (list (org-get-heading t t t t)
              (org-get-tags nil t)
              (not (null (org-invisible-p (line-end-position))))))
      nil
      nil)
     (let (states)
       (goto-char (point-min))
       (while (re-search-forward "^\\*+ " nil t)
         (push (list (org-get-heading t t t t)
                     (not (null (org-invisible-p (line-end-position)))))
               states))
       (nreverse states)))))"##,
    );
}

#[test]
fn org_refile_targets_cache_new_child_outline_path_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-refile)
  (let* ((one (make-temp-file "org-refile-one" nil ".org"
                              "#+TITLE: One\n* Projects\n** Alpha :work:\n*** Leaf\n* Inbox\n"))
         (two (make-temp-file "org-refile-two" nil ".org"
                              "#+TITLE: Two\n* Areas\n** Beta :home:\n"))
         (org-refile-targets `((,(list one two) . (:maxlevel . 3))))
         (org-refile-use-outline-path 'title)
         (org-refile-use-cache t)
         (normalize-file
          (lambda (file)
            (cond
             ((null file) nil)
             ((string-prefix-p "org-refile-one" file) "<one>")
             ((string-prefix-p "org-refile-two" file) "<two>")
             (t file))))
         first second child)
    (unwind-protect
        (progn
          (org-refile-cache-clear)
          (setq first (mapcar (lambda (target)
                                (list (car target)
                                      (funcall normalize-file
                                               (and (nth 1 target)
                                                    (file-name-nondirectory
                                                     (nth 1 target))))
                                      (not (null (nth 3 target)))))
                              (with-current-buffer (find-file-noselect one)
                                (org-mode)
                                (org-refile-get-targets))))
          (setq second (mapcar (lambda (target)
                                 (list (car target)
                                       (funcall normalize-file
                                                (and (nth 1 target)
                                                     (file-name-nondirectory
                                                      (nth 1 target))))
                                       (not (null (nth 3 target)))))
                               (with-current-buffer (find-file-noselect one)
                                 (org-mode)
                                 (org-refile-get-targets))))
          (setq child
                (with-current-buffer (find-file-noselect two)
                  (org-mode)
                  (let* ((targets (org-refile-get-targets))
                         (parent (seq-find
                                  (lambda (target)
                                    (string-match-p "/Areas/Beta\\'" (car target)))
                                  targets)))
                    (org-refile-new-child parent "Gamma :new:")
                    (save-buffer)
                    (buffer-substring-no-properties
                     (point-min) (point-max)))))
          (list first
                second
                child
                (not (null (org-refile-cache-get
                            (expand-file-name one)
                            "^\\*\\{1,3\\}[ \t]")))))
      (dolist (file (list one two))
        (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
        (when (file-exists-p file) (delete-file file))))))"##,
    );
}

#[test]
fn org_archive_sibling_reversed_order_stats_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-archive)
  (with-temp-buffer
    (let ((org-archive-reversed-order t)
          (org-provide-todo-statistics t)
          (org-todo-keywords '((sequence "TODO" "|" "DONE"))))
      (org-mode)
      (insert "* Parent [1/3]\n")
      (insert "** DONE Old one\n")
      (insert "Body one\n")
      (insert "** DONE Old two\n")
      (insert "Body two\n")
      (insert "** TODO Keep\n")
      (goto-char (point-min))
      (search-forward "Old two")
      (beginning-of-line)
      (org-archive-to-archive-sibling)
      (goto-char (point-min))
      (search-forward "Old one")
      (beginning-of-line)
      (org-archive-to-archive-sibling)
      (org-update-statistics-cookies t)
      (list (replace-regexp-in-string
             ":ARCHIVE_TIME: .*"
             ":ARCHIVE_TIME: [stamp]"
             (buffer-substring-no-properties (point-min) (point-max)))
            (mapcar
             (lambda (needle)
               (save-excursion
                 (goto-char (point-min))
                 (search-forward needle)
                 (list needle
                       (org-current-level)
                       (not (null (org-invisible-p (line-end-position)))))))
             '("Parent" "Archive" "Old one" "Old two" "Keep"))))))"##,
    );
}

#[test]
fn org_archive_all_done_tag_then_move_old_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-archive)
  (with-temp-buffer
    (let ((org-archive-location "::* Archived")
          (org-archive-save-context-info '(time todo category olpath))
          (org-archive-stamp-time nil)
          (org-confirm-babel-evaluate nil))
      (org-mode)
      (insert "#+CATEGORY: Batch\n")
      (insert "* Project\n")
      (insert "** DONE Closed\nCLOSED: [2026-05-01 Fri]\n")
      (insert "** DONE Old timestamp\nSCHEDULED: <2026-05-01 Fri>\n")
      (insert "** TODO Active\nSCHEDULED: <2026-06-01 Mon>\n")
      (insert "** DONE Fresh\nSCHEDULED: <2026-05-27 Wed>\n")
      (goto-char (point-min))
      (search-forward "Project")
      (beginning-of-line)
      (cl-letf (((symbol-function 'y-or-n-p) (lambda (&rest _) t)))
        (org-archive-all-done 'tag)
        (org-archive-all-old nil))
      (list (buffer-substring-no-properties (point-min) (point-max))
            (org-map-entries
             (lambda ()
               (list (org-get-heading t t t t)
                     (org-get-todo-state)
                     (org-get-tags nil t)
                     (org-entry-get nil "ARCHIVE_CATEGORY")
                     (org-entry-get nil "ARCHIVE_TODO")))
             nil nil)))))"##,
    );
}

#[test]
fn org_archive_property_locations_hooks_files_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-archive)
  (let* ((root (make-temp-file "org-archive-props" t))
         (source (expand-file-name "source.org" root))
         (archive-a (expand-file-name "archive-a.org" root))
         (archive-b (expand-file-name "archive-b.org" root))
         (events nil)
         (org-archive-location "%s_archive::* Default Archive")
         (org-archive-save-context-info
          '(file category todo olpath itags ltags))
         (org-archive-subtree-add-inherited-tags t)
         (org-archive-subtree-save-file-p t)
         (org-archive-hook
          (list (lambda ()
                  (push (list (org-get-heading t t t t)
                              (org-current-level)
                              (org-get-tags nil t))
                        events)))))
    (unwind-protect
        (progn
          (with-temp-file source
            (insert "#+CATEGORY: Cases\n")
            (insert "* Project :client:\n")
            (insert ":PROPERTIES:\n:ARCHIVE: " archive-a "::* Project Archive\n:END:\n")
            (insert "** DONE Task A :done:\n")
            (insert "Body A\n")
            (insert "** TODO Active\n")
            (insert "* Other :ops:\n")
            (insert ":PROPERTIES:\n:ARCHIVE: " archive-b "::* Other Archive\n:END:\n")
            (insert "** DONE Task B :closed:\n")
            (insert "Body B\n"))
          (with-current-buffer (find-file-noselect source)
            (org-mode)
            (goto-char (point-min))
            (search-forward "Task A")
            (beginning-of-line)
            (let ((loc-a (org-archive--compute-location
                          (org-entry-get nil "ARCHIVE" t))))
              (org-archive-subtree)
              (goto-char (point-min))
              (search-forward "Task B")
              (beginning-of-line)
              (let ((loc-b (org-archive--compute-location
                            (org-entry-get nil "ARCHIVE" t))))
                (org-archive-subtree)
                (save-buffer)
                (list (list (file-relative-name (car loc-a) root)
                            (cdr loc-a))
                      (list (file-relative-name (car loc-b) root)
                            (cdr loc-b))
                      (nreverse events)
                      (sort (mapcar (lambda (file)
                                      (file-relative-name file root))
                                    (org-all-archive-files))
                            #'string<)
                      (sort (mapcar (lambda (file)
                                      (file-relative-name file root))
                                    (org-add-archive-files
                                     (list source)))
                            #'string<)
                      (replace-regexp-in-string
                       (regexp-quote root)
                       "<root>"
                       (buffer-substring-no-properties
                        (point-min) (point-max)))
                      (with-current-buffer (find-file-noselect archive-a)
                        (replace-regexp-in-string
                         (regexp-quote root)
                         "<root>"
                         (buffer-substring-no-properties
                          (point-min) (point-max))))
                      (with-current-buffer (find-file-noselect archive-b)
                        (replace-regexp-in-string
                         (regexp-quote root)
                         "<root>"
                         (buffer-substring-no-properties
                          (point-min) (point-max)))))))))
      (dolist (file (list source archive-a archive-b))
        (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
        (when (file-exists-p file) (delete-file file)))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_refile_completion_new_parent_verify_history_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-refile)
  (let* ((root (make-temp-file "org-refile-complete" t))
         (file (expand-file-name "targets.org" root))
         (org-refile-targets `((,file . (:maxlevel . 3))))
         (org-refile-use-outline-path t)
         (org-outline-path-complete-in-steps t)
         (org-refile-allow-creating-parent-nodes 'confirm)
         (org-refile-history nil)
         prompts answers)
    (unwind-protect
        (progn
          (with-temp-file file
            (insert "* Inbox\n")
            (insert "* Projects :target:\n")
            (insert "** Skip :skip:\n*** Hidden target\n")
            (insert "** Keep :target:\n"))
          (with-current-buffer (find-file-noselect file)
            (org-mode)
            (let ((org-refile-target-verify-function
                   (lambda ()
                     (let ((tags (org-get-tags nil t)))
                       (cond
                        ((member "skip" tags)
                         (org-end-of-subtree t t)
                         nil)
                        ((or (= (org-current-level) 1)
                             (member "target" tags))
                         t)
                        (t nil))))))
              (cl-letf (((symbol-function 'completing-read)
                         (lambda (prompt collection &rest _)
                           (push (list prompt
                                       (sort
                                        (mapcar #'car
                                                (if (functionp collection)
                                                    (all-completions
                                                     "" collection)
                                                  collection))
                                        #'string<))
                                 prompts)
                           (pop answers)))
                        ((symbol-function 'y-or-n-p)
                         (lambda (prompt)
                           (push prompt prompts)
                           t)))
                (setq answers '("Projects/Keep/New child"))
                (let ((new-target
                       (org-refile-get-location "Move to" nil
                                                org-refile-allow-creating-parent-nodes))
                      (after-new (buffer-substring-no-properties
                                  (point-min) (point-max))))
                  (setq answers '("Projects/Keep/New child"))
                  (let ((existing
                         (org-refile-get-location "Again" nil nil)))
                    (list (list (car new-target)
                                (file-relative-name (nth 1 new-target) root)
                                (nth 2 new-target)
                                (not (null (nth 3 new-target))))
                          (list (car existing)
                                (file-relative-name (nth 1 existing) root)
                                (nth 2 existing)
                                (not (null (nth 3 existing))))
                          (nreverse prompts)
                          org-refile-history
                          after-new
                          (buffer-substring-no-properties
                           (point-min) (point-max)))))))))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_refile_active_region_reverse_order_log_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-refile)
  (let* ((file (make-temp-file "org-refile-region" nil ".org"
                               "* Inbox
Loose note line
Continued context
** TODO Task A :inbox:
Task body
** TODO Task B :inbox:
Task B body
* Projects
** Target
*** Existing child
"))
         (events nil))
    (unwind-protect
        (with-current-buffer (find-file-noselect file)
          (org-mode)
          (let ((org-refile-active-region-within-subtree t)
                (org-log-refile 'time)
                (org-log-into-drawer t)
                (org-reverse-note-order nil)
                (org-after-refile-insert-hook
                 (list (lambda ()
                         (push (org-get-heading t t t t) events)))))
            (let ((target-pos (save-excursion
                                (goto-char (point-min))
                                (search-forward "Target")
                                (line-beginning-position))))
              (goto-char (point-min))
              (search-forward "Loose note line")
              (beginning-of-line)
              (let ((beg (point)))
                (search-forward "Continued context")
                (end-of-line)
                (transient-mark-mode 1)
                (set-mark beg)
                (activate-mark)
                (org-refile nil nil (list "Target" file nil target-pos)))
              (goto-char (point-min))
              (search-forward "Task A")
              (beginning-of-line)
              (org-refile-reverse
               nil nil (list "Target" file nil target-pos) "Reverse")
              (save-buffer)
              (list (nreverse events)
                    (plist-get org-bookmark-names-plist :last-refile)
                    (replace-regexp-in-string
                     "- Refiled on \\[.*\\]"
                     "- Refiled on [stamp]"
                     (buffer-substring-no-properties
                      (point-min) (point-max)))))))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (when (file-exists-p file) (delete-file file)))))"##,
    );
}

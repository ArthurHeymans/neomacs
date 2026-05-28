use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_capture_table_line_and_plain_append_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-capture)
  (let* ((file (make-temp-file
                "org-capture-table" nil ".org"
                "* Log\n| When | Text |\n|------|------|\n"))
         (org-capture-templates
          `(("t" "Table" table-line
             (file+headline ,file "Log")
             "| %u | %i |"
             :empty-lines 0)
            ("p" "Plain" plain
             (file+headline ,file "Log")
             "Plain: %i\n"
             :empty-lines 0))))
    (unwind-protect
        (progn
          (org-capture-string "row text" "t")
          (org-capture-finalize)
          (org-capture-string "plain text" "p")
          (org-capture-finalize)
          (with-temp-buffer
            (insert-file-contents file)
            (replace-regexp-in-string
             "\\[[0-9-]+ [A-Za-z]+\\]"
             "[date]"
             (buffer-string))))
      (dolist (buf '("CAPTURE-org-capture-table" "CAPTURE-org-capture-table.org"))
        (when (get-buffer buf) (kill-buffer buf)))
      (when (file-exists-p file) (delete-file file)))))"##,
    );
}

#[test]
fn org_capture_item_checkitem_prepend_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-capture)
  (let* ((file (make-temp-file "org-capture-items" nil ".org"
                               "* List\n- existing\n"))
         (org-capture-templates
          `(("i" "Item" item
             (file+headline ,file "List")
             "%i"
             :prepend t
             :empty-lines 0)
            ("c" "Check" checkitem
             (file+headline ,file "List")
             "%i"
             :empty-lines 0))))
    (unwind-protect
        (progn
          (org-capture-string "first" "i")
          (org-capture-finalize)
          (org-capture-string "done-ish" "c")
          (org-capture-finalize)
          (with-temp-buffer
            (insert-file-contents file)
            (buffer-string)))
      (dolist (buf '("CAPTURE-org-capture-items" "CAPTURE-org-capture-items.org"))
        (when (get-buffer buf) (kill-buffer buf)))
      (when (file-exists-p file) (delete-file file)))))"##,
    );
}

#[test]
fn org_capture_template_expand_context_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-capture)
  (with-temp-buffer
    (let ((buffer-file-name "/tmp/source.org"))
      (org-mode)
      (insert "* Source\nBody\n")
      (goto-char (point-min))
      (search-forward "Source")
      (let ((org-capture-plist
             (list :template
                   "* TODO %i\nFrom: %a\nFile: %F\nName: %f\nTime: %<%Y-%m-%d>\n%(upcase \"ok\")\n"
                   :initial "Initial text"
                   :annotation "[[file:/tmp/source.org::*Source][Source]]"
                   :original-file "/tmp/source.org"
                   :original-file-nondirectory "source.org"
                   :default-time (encode-time 0 30 9 27 5 2026)
                   :buffer (current-buffer))))
        (org-capture-fill-template)))))"##,
    );
}

#[test]
fn org_capture_olp_datetree_week_clock_template_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-capture)
  (require 'org-datetree)
  (require 'org-clock)
  (let* ((file (make-temp-file "org-capture-datetree" nil ".org"
                               "* Journal\n** Work\n"))
         (org-overriding-default-time (encode-time 0 30 9 27 5 2026))
         (org-capture-templates
          `(("w" "Week" entry
             (file+olp+datetree ,file "Journal" "Work")
             "* TODO %?%i\nCreated: %U\n"
             :tree-type week
             :clock-in t
             :clock-keep nil
             :empty-lines 0))))
    (unwind-protect
        (progn
          (org-capture-string "Captured body" "w")
          (let ((during (list (org-clocking-p)
                              org-clock-current-task
                              (and (markerp org-clock-marker)
                                   (marker-buffer org-clock-marker)))))
            (org-capture-finalize)
            (with-temp-buffer
              (insert-file-contents file)
              (list during
                    (org-clocking-p)
                    (replace-regexp-in-string
                     "=> +[-0-9:]+"
                     "=> [duration]"
                     (replace-regexp-in-string
                      "\\[[0-9][^]\n]+\\]"
                      "[stamp]"
                      (buffer-string)))))))
      (dolist (buf '("CAPTURE-org-capture-datetree"
                     "CAPTURE-org-capture-datetree.org"))
        (when (get-buffer buf) (kill-buffer buf)))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (when (file-exists-p file) (delete-file file)))))"##,
    );
}

#[test]
fn org_capture_regexp_function_prepend_append_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-capture)
  (let* ((file (make-temp-file "org-capture-targets" nil ".org"
                               "* Inbox\n:marker:\nold\n:end:\n* Tail\n"))
         (finder
          (lambda ()
            (goto-char (point-min))
            (search-forward "* Tail")
            (end-of-line)))
         (org-capture-templates
          `(("r" "Regexp prepend" plain
             (file+regexp ,file ":marker:")
             "REGEXP:%i\n"
             :prepend t
             :empty-lines 0)
            ("f" "Function append" entry
             (file+function ,file ,finder)
             "* FUNCTION %i\n"
             :empty-lines 0))))
    (unwind-protect
        (progn
          (org-capture-string "one" "r")
          (let ((regexp-pos (marker-position org-capture-last-stored-marker)))
            (org-capture-finalize)
            (org-capture-string "two" "f")
            (let ((function-pos (marker-position org-capture-last-stored-marker)))
              (org-capture-finalize)
              (with-temp-buffer
                (insert-file-contents file)
                (list regexp-pos
                      function-pos
                      (buffer-string))))))
      (dolist (buf '("CAPTURE-org-capture-targets"
                     "CAPTURE-org-capture-targets.org"))
        (when (get-buffer buf) (kill-buffer buf)))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (when (file-exists-p file) (delete-file file)))))"##,
    );
}

#[test]
fn org_capture_clock_target_resume_and_links_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-capture)
  (require 'org-clock)
  (let* ((file (make-temp-file "org-capture-clock" nil ".org"
                               "* TODO Clocked\nBody\n* Notes\n"))
         (org-capture-templates
          `(("c" "Clock note" entry
             (clock)
             "* NOTE %?\nFrom clock: %k\nLink: %K\nInitial: %i\n"
             :clock-in t
             :clock-resume t
             :empty-lines 0))))
    (unwind-protect
        (with-current-buffer (find-file-noselect file)
          (org-mode)
          (let ((org-clock-into-drawer "LOGBOOK")
                (org-clock-history-length 5)
                (org-clock-persist nil))
            (goto-char (point-min))
            (search-forward "Clocked")
            (beginning-of-line)
            (org-clock-in nil (encode-time 0 0 9 27 5 2026))
            (let ((before-task org-clock-current-task))
              (org-capture-string "captured text" "c")
              (let ((capture-task org-clock-current-task)
                    (capture-running (org-clocking-p)))
                (org-capture-finalize)
                (let ((after-task org-clock-current-task)
                      (after-running (org-clocking-p)))
                  (when (org-clocking-p)
                    (org-clock-out nil t (encode-time 0 45 9 27 5 2026)))
                  (save-buffer)
                  (list before-task
                        capture-task
                        capture-running
                        after-task
                        after-running
                        (replace-regexp-in-string
                         (regexp-quote file)
                         "<file>"
                         (replace-regexp-in-string
                          "=> +[-0-9:]+"
                          "=> [duration]"
                          (replace-regexp-in-string
                           "\\[[0-9][^]\n]+\\]"
                           "[stamp]"
                           (buffer-substring-no-properties
                            (point-min) (point-max)))))))))))
      (dolist (buf '("CAPTURE-org-capture-clock"
                     "CAPTURE-org-capture-clock.org"))
        (when (get-buffer buf) (kill-buffer buf)))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (when (file-exists-p file) (delete-file file)))))"##,
    );
}

#[test]
fn org_capture_kill_finalize_goto_marker_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-capture)
  (let* ((file (make-temp-file "org-capture-life" nil ".org"
                               "* Inbox\nOld line\n* Done\n"))
         (org-capture-templates
          `(("e" "Entry" entry
             (file+headline ,file "Inbox")
             "* TODO %i\nCaptured from %f\n"
             :empty-lines 0))))
    (unwind-protect
        (progn
          (org-capture-string "transient body" "e")
          (let ((capture-state
                 (list (buffer-name)
                       (buffer-narrowed-p)
                       (marker-position org-capture-last-stored-marker)
                       (buffer-substring-no-properties
                        (point-min) (point-max)))))
            (org-capture-kill)
            (let ((after-kill
                   (with-temp-buffer
                     (insert-file-contents file)
                     (buffer-string))))
              (org-capture-string "stored body" "e")
              (let ((before-finalize
                     (list (buffer-name)
                           (marker-position org-capture-last-stored-marker)
                           (buffer-substring-no-properties
                            (point-min) (point-max)))))
                (org-capture-finalize)
                (let ((stored-pos
                       (marker-position org-capture-last-stored-marker))
                      (stored-buffer
                       (buffer-name
                        (marker-buffer org-capture-last-stored-marker)))
                      goto-line)
                  (org-capture-goto-last-stored)
                  (setq goto-line
                        (buffer-substring-no-properties
                         (line-beginning-position)
                         (line-end-position)))
                  (list capture-state
                        after-kill
                        before-finalize
                        stored-pos
                        stored-buffer
                        goto-line
                        (buffer-substring-no-properties
                         (point-min) (point-max))))))))
      (dolist (buf '("CAPTURE-org-capture-life"
                     "CAPTURE-org-capture-life.org"))
        (when (get-buffer buf) (kill-buffer buf)))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (when (file-exists-p file) (delete-file file)))))"##,
    );
}

#[test]
fn org_capture_prompt_placeholders_history_tags_props_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-capture)
  (with-temp-buffer
    (let ((buffer-file-name "/tmp/capture-target.org")
          (org-capture--prompt-history-table
           (make-hash-table :test #'equal))
          prompts)
      (org-mode)
      (insert "#+TAGS: work urgent home\n")
      (insert "* Target\n")
      (insert ":PROPERTIES:\n:Owner_ALL: Ada Bea Cy\n:END:\n")
      (goto-char (point-min))
      (search-forward "Target")
      (beginning-of-line)
      (let ((org-capture-plist
             (list :template
                   "* TODO %^{Title|Default|Alpha|Beta} :%^g:\nSCHEDULED: %^{When}t\nCLOSED: %^{Closed}U\n:PROPERTIES:\n:Owner: %^{Owner|Ada}p\n:END:\nRepeated: %\\1\nElisp: %(concat \"ok-\" \"%^{Title|Default|Alpha|Beta}\")\n"
                   :default-time (encode-time 0 0 9 27 5 2026)
                   :buffer (current-buffer)
                   :pos (point-marker))))
        (cl-letf (((symbol-function 'org-completing-read)
                   (lambda (prompt collection &rest _)
                     (push (list 'string prompt collection) prompts)
                     "Beta"))
                  ((symbol-function 'completing-read-multiple)
                   (lambda (prompt collection &rest _)
                     (push (list 'tags prompt
                                 (sort
                                  (mapcar (lambda (entry)
                                            (if (consp entry)
                                                (car entry)
                                              entry))
                                          collection)
                                  #'string<))
                           prompts)
                     '("work" "urgent")))
                  ((symbol-function 'org-read-date)
                   (lambda (with-time to-time from-string prompt &rest _)
                     (push (list 'date with-time to-time from-string prompt)
                           prompts)
                     (encode-time 0 45 10 27 5 2026)))
                  ((symbol-function 'org-read-property-value)
                   (lambda (property pom default &rest _)
                     (push (list 'property
                                 property
                                 (marker-position pom)
                                 default)
                           prompts)
                     "Bea")))
          (list (org-capture-fill-template)
                (nreverse prompts)
                (gethash "Title" org-capture--prompt-history-table)
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_capture_finalize_hooks_stats_narrow_prompt_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-capture)
  (let* ((file (make-temp-file "org-capture-hooks" nil ".org"
                               "* Project [0/1]\n** Inbox\n- [ ] Existing\n** Archive\n"))
         (events nil)
         (answers '("Hooked Title"))
         (org-overriding-default-time (encode-time 0 15 11 27 5 2026))
         (org-capture--prompt-history-table (make-hash-table :test #'equal))
         (org-capture-templates
          `(("e" "Entry hooks" entry
             (file+olp ,file "Project" "Inbox")
             "* TODO %^{Title|Default}\nSCHEDULED: %t\nFrom: %a\n%i\n"
             :prepend t
             :empty-lines 1
             :prepare-finalize
             ,(lambda ()
                (push (list 'prepare
                            (buffer-name)
                            (buffer-narrowed-p)
                            (point-min)
                            (point-max))
                      events)
                (goto-char (point-max))
                (insert "Prepared line\n"))
             :before-finalize
             ,(lambda ()
                (push (list 'before
                            (buffer-name)
                            (marker-position
                             (org-capture-get :begin-marker 'local))
                            (marker-position
                             (org-capture-get :end-marker 'local)))
                      events))
             :after-finalize
             ,(lambda ()
                (push (list 'after
                            (plist-get org-capture-plist :key)
                            (plist-get org-capture-plist
                                       :captured-entry-size)
                            (marker-position
                             org-capture-last-stored-marker))
                      events)))
            ("c" "Check stats" checkitem
             (file+olp ,file "Project" "Inbox")
             "%i"
             :empty-lines 0
             :after-finalize
             ,(lambda ()
                (push (list 'check-after
                            (plist-get org-capture-plist :key)
                            (marker-position
                             org-capture-last-stored-marker))
                      events))))))
    (unwind-protect
        (progn
          (with-current-buffer (find-file-noselect file)
            (org-mode)
            (goto-char (point-min))
            (search-forward "Inbox")
            (org-narrow-to-subtree)
            (let ((narrow-before (list (point-min)
                                       (point-max)
                                       (buffer-narrowed-p))))
              (cl-letf (((symbol-function 'org-completing-read)
                         (lambda (prompt collection &rest _)
                           (push (list 'prompt
                                       prompt
                                       (sort
                                        (mapcar (lambda (entry)
                                                  (if (consp entry)
                                                      (car entry)
                                                    entry))
                                                collection)
                                        #'string<))
                                 events)
                           (pop answers))))
                (org-capture-string
                 "Initial body"
                 "e"))
              (let ((capture-before
                     (list (buffer-name)
                           (buffer-narrowed-p)
                           (buffer-substring-no-properties
                            (point-min) (point-max))
                           (marker-position
                            org-capture-last-stored-marker))))
                (org-capture-finalize)
                (let ((target-after-entry
                       (with-current-buffer (find-file-noselect file)
                         (list (buffer-narrowed-p)
                               (point-min)
                               (point-max)
                               (buffer-substring-no-properties
                                (point-min) (point-max))))))
                  (org-capture-string "Captured checkbox" "c")
                  (let ((check-before
                         (list (buffer-name)
                               (buffer-substring-no-properties
                                (point-min) (point-max))
                               (marker-position
                                org-capture-last-stored-marker))))
                    (org-capture-finalize)
                    (with-current-buffer (find-file-noselect file)
                      (widen)
                      (goto-char (point-min))
                      (org-update-statistics-cookies t)
                      (save-buffer)
                      (list narrow-before
                            capture-before
                            target-after-entry
                            check-before
                            (nreverse events)
                            (gethash "Title"
                                     org-capture--prompt-history-table)
                            (marker-position
                             org-capture-last-stored-marker)
                            (replace-regexp-in-string
                             "\\[[0-9][^]\n]+\\]"
                             "[stamp]"
                             (buffer-substring-no-properties
                              (point-min) (point-max)))))))))))
      (dolist (buf '("CAPTURE-org-capture-hooks"
                     "CAPTURE-org-capture-hooks.org"))
        (when (get-buffer buf) (kill-buffer buf)))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (when (file-exists-p file) (delete-file file)))))"##,
    );
}

#[test]
fn org_capture_refile_cross_file_marker_link_lifecycle_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-capture)
  (require 'org-refile)
  (let* ((inbox (make-temp-file "org-capture-refile-inbox" nil ".org"
                                "#+TITLE: Inbox\n* Inbox\n"))
         (projects (make-temp-file "org-capture-refile-projects" nil ".org"
                                   "#+TITLE: Projects\n* Projects\n** Alpha\n** Beta\n"))
         (events nil)
         (org-refile-targets `((,projects . (:maxlevel . 2))))
         (org-refile-use-outline-path 'title)
         (org-refile-use-cache t)
         (org-capture-templates
          `(("t" "Task refile" entry
             (file+headline ,inbox "Inbox")
             "* TODO %i\n:PROPERTIES:\n:Source: %a\n:END:\nBody: %?\n"
             :empty-lines 0
             :after-finalize
             ,(lambda ()
                (push (list 'after
                            (marker-position
                             org-capture-last-stored-marker)
                            (buffer-name
                             (marker-buffer
                              org-capture-last-stored-marker)))
                      events))))))
    (unwind-protect
        (progn
          (org-refile-cache-clear)
          (with-current-buffer (find-file-noselect projects)
            (org-mode))
          (with-temp-buffer
            (let ((buffer-file-name "/tmp/source-note.org"))
              (org-mode)
              (insert "* Source Head\ncontext\n")
              (goto-char (point-min))
              (search-forward "Source Head")
              (let ((org-capture-link-is-already-stored nil))
                (org-store-link nil))))
          (let (capture-before marker-after-finalize refile-targets
                inbox-after-finalize inbox-after-refile projects-after-refile
                goto-line last-bookmark cache-present)
            (org-capture-string "captured task" "t")
            (setq capture-before
                  (list (buffer-name)
                        (marker-position org-capture-last-stored-marker)
                        (buffer-substring-no-properties
                         (point-min) (point-max))))
            (org-capture-finalize)
            (setq marker-after-finalize
                  (list (marker-position org-capture-last-stored-marker)
                        (buffer-name
                         (marker-buffer org-capture-last-stored-marker))))
            (setq inbox-after-finalize
                  (with-current-buffer (find-file-noselect inbox)
                    (buffer-substring-no-properties
                     (point-min) (point-max))))
            (with-current-buffer (find-file-noselect inbox)
              (org-mode)
              (goto-char (point-min))
              (search-forward "captured task")
              (beginning-of-line)
              (setq refile-targets
                    (mapcar (lambda (target)
                              (list (car target)
                                    (and (nth 1 target)
                                         (file-name-nondirectory
                                          (nth 1 target)))
                                    (not (null (nth 3 target)))))
                            (org-refile-get-targets)))
              (let ((target
                     (seq-find
                      (lambda (entry)
                        (string-match-p "/Projects/Beta\\'" (car entry)))
                      (org-refile-get-targets))))
                (org-refile nil nil target))
              (setq last-bookmark
                    (plist-get org-bookmark-names-plist :last-refile))
              (save-buffer)
              (setq inbox-after-refile
                    (buffer-substring-no-properties
                     (point-min) (point-max))))
            (with-current-buffer (find-file-noselect projects)
              (org-mode)
              (save-buffer)
              (setq projects-after-refile
                    (replace-regexp-in-string
                     "\\[\\[file:/tmp/source-note.org[^]\n]+\\]"
                     "[source-link]"
                     (buffer-substring-no-properties
                      (point-min) (point-max)))))
            (org-capture-goto-last-stored)
            (setq goto-line
                  (buffer-substring-no-properties
                   (line-beginning-position)
                   (line-end-position)))
            (setq cache-present
                  (not (null
                        (org-refile-cache-get
                         (expand-file-name projects)
                         "^\\*\\{1,2\\}[ \t]"))))
            (list capture-before
                  marker-after-finalize
                  inbox-after-finalize
                  refile-targets
                  inbox-after-refile
                  projects-after-refile
                  goto-line
                  last-bookmark
                  cache-present
                  (nreverse events))))
      (dolist (file (list inbox projects))
        (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
        (when (file-exists-p file) (delete-file file))))))"##,
    );
}

#[test]
fn org_capture_template_escape_clipboard_elisp_matrix_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-capture)
  (let* ((root (make-temp-file "org-capture-template" t))
         (snippet (expand-file-name "snippet.txt" root))
         (org-capture--prompt-history-table
          (make-hash-table :test #'equal))
         (kill-ring '("Kill value"))
         (kill-ring-yank-pointer kill-ring)
         (prompts nil))
    (unwind-protect
        (progn
          (with-temp-file snippet
            (insert "Snippet %u stays literal\n")
            (insert "Second line\n"))
          (with-temp-buffer
            (let ((buffer-file-name "/tmp/source capture.org")
                  (org-store-link-plist
                   '(:annotation "[[file:/tmp/source capture.org::*Head][Head]]"
                     :initial "Initial\n  second"
                     :custom "CustomValue")))
              (org-mode)
              (insert "#+TAGS: work urgent home\n")
              (insert "#+PROPERTY: Owner_ALL Ada Bea Cy\n")
              (insert "* Head\n:PROPERTIES:\n:Owner: Ada\n:END:\n")
              (goto-char (point-min))
              (search-forward "Head")
              (beginning-of-line)
              (let ((org-capture-plist
                     (list :template
                           (concat
                            "\\% escaped and \\\\%i literal\n"
                            "%[" snippet "]"
                            "* TODO %^{Title|Default|One|Two} :%^g:\n"
                            "Chosen again %\\1\n"
                            "Initial: %i\n"
                            "Annotation: %a\n"
                            "Bare: %l\n"
                            "Link-only: %L\n"
                            "Clip: %C\n"
                            "Clip link: %L\n"
                            "Plist: %:custom\n"
                            "Elisp: %(concat \"%:custom\" \"|\" \"%i\" \"|\" \"%a\")\n"
                            "Date: %<%Y-%m-%d %H:%M>\n"
                            "Owner: %^{Owner|Ada}p\n")
                           :default-time
                           (encode-time 0 30 8 27 5 2026)
                           :buffer (current-buffer)
                           :pos (point-marker)
                           :target-entry-p t)))
                (cl-letf (((symbol-function 'org-get-x-clipboard)
                           (lambda (type)
                             (pcase type
                               ('PRIMARY "Primary text")
                               ('CLIPBOARD "https://clip.example/path")
                               (_ nil))))
                          ((symbol-function 'org-completing-read)
                           (lambda (prompt collection &rest _)
                             (push (list 'string prompt collection) prompts)
                             "Two"))
                          ((symbol-function 'completing-read-multiple)
                           (lambda (prompt collection &rest _)
                             (push (list 'tags prompt
                                         (sort
                                          (mapcar (lambda (entry)
                                                    (if (consp entry)
                                                        (car entry)
                                                      entry))
                                                  collection)
                                          #'string<))
                                   prompts)
                             '("work" "urgent")))
                          ((symbol-function 'read-string)
                           (lambda (prompt &optional initial history default
                                           &rest _)
                             (push (list 'read-string prompt initial
                                         (and (symbolp history) history)
                                         default)
                                   prompts)
                             initial))
                          ((symbol-function 'org-read-property-value)
                           (lambda (property pom default &rest _)
                             (push (list 'property property
                                         (marker-position pom)
                                         default)
                                   prompts)
                             "Bea")))
                  (let ((filled (org-capture-fill-template)))
                    (list filled
                          (nreverse prompts)
                          (gethash "Title"
                                   org-capture--prompt-history-table)
                          org-store-link-plist
                          (buffer-substring-no-properties
                           (point-min) (point-max)))))))))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_capture_template_expand_body_placeholders_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-capture)
  (let* ((root (make-temp-file "org-cap-deep" t))
         (file (expand-file-name "tasks.org" root))
         (org-capture-templates
          `(("t" "Todo" entry (file+headline ,file "Inbox")
             "** TODO %?\n:PROPERTIES:\n:Source: %a\n:Created: %U\n:END:\n%i\n"
             :empty-lines 0)
            ("n" "Note" plain (file+olp ,file "Notes" "Quick")
             "- %U %a\n  %?\n"
             :empty-lines 0))))
    (unwind-protect
        (progn
          (with-temp-file file
            (insert "#+TITLE: Tasks\n")
            (insert "* Inbox\n")
            (insert "* Notes\n")
            (insert "** Quick\n"))
          ;; Capture with template t
          (org-capture-string "First task" "t")
          (let ((after-t (with-current-buffer (org-capture-get :buffer)
                           (buffer-substring-no-properties
                            (point-min) (point-max)))))
            (org-capture-finalize)
            ;; Capture with template n
            (org-capture-string "Quick note" "n")
            (let ((after-n (with-current-buffer (org-capture-get :buffer)
                            (buffer-substring-no-properties
                             (point-min) (point-max)))))
              (org-capture-finalize)
              ;; Read final file
              (let ((final-content
                     (with-temp-buffer
                       (insert-file-contents file)
                       (buffer-string)))
                    ;; Check element structure
                    (elements
                     (with-current-buffer (find-file-noselect file)
                       (prog1
                           (org-element-map (org-element-parse-buffer)
                               '(headline item property-drawer)
                             (lambda (el)
                               (list (org-element-type el)
                                     (org-element-property :level el)
                                     (org-element-property :raw-value el)
                                     (org-element-property :key el)
                                     (org-element-property :value el))))
                         (kill-buffer)))))
                 (list after-t
                       after-n
                       (replace-regexp-in-string
                        "CLOSED: \\[.*\\]" "CLOSED: [stamp]"
                        (replace-regexp-in-string
                         ":Created: \\[.*\\]" ":Created: [stamp]"
                         (replace-regexp-in-string
                          ":Source: .*" ":Source: [src]"
                          final-content)))
                       elements)))))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_capture_insert_todo_edit_clock_refile_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-capture)
  (require 'org-refile)
  (let* ((root (make-temp-file "org-capture-clock-" t))
         (target (expand-file-name "target.org" root))
         (refile-target (expand-file-name "refile.org" root))
         (org-capture-templates
          `(("t" "Todo" entry (file+headline ,target "Inbox")
             "* TODO %?\n%U\n")))
         (org-refile-targets `((,refile-target :maxlevel . 2))))
    (unwind-protect
        (progn
          ;; Create target
          (with-temp-file target
            (insert "* Inbox\n* Tasks\n** Sub\n"))
          ;; Create refile target
          (with-temp-file refile-target
            (insert "* Refile inbox\n* Refile tasks\n** Refile sub\n"))
          ;; Capture
          (org-capture nil "t")
          (insert "Captured task alpha")
          (org-capture-finalize)
          ;; Edit: add clock to captured heading
          (with-current-buffer (find-file-noselect target)
            (goto-char (point-min))
            (search-forward "Captured task")
            (beginning-of-line)
            (org-clock-in)
            (org-clock-out)
            ;; Refile
            (goto-char (point-min))
            (search-forward "Captured task")
            (beginning-of-line)
            (let ((before-refile (buffer-substring-no-properties
                                  (point-min) (point-max))))
              (org-refile nil nil
                          (list "Refile tasks" refile-target nil nil))
              ;; Read refile target
              (let ((target-after (with-current-buffer
                                      (find-file-noselect target)
                                    (buffer-substring-no-properties
                                     (point-min) (point-max))))
                    (refile-after (with-current-buffer
                                     (find-file-noselect refile-target)
                                   (buffer-substring-no-properties
                                    (point-min) (point-max)))))
                (list before-refile
                      target-after
                      refile-after))))))
      (delete-directory root t))))"##,
    );
}

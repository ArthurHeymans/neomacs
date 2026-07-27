use expect_test::expect;

use super::assert_agent_recall_parity;

#[test]
fn agent_recall_project_and_transcript_path_helpers_cover_conventional_and_org_layouts() {
    let elisp_form = r##"(list
                      (agent-recall--project-name
                       "/work/clients/alpha/.agent-shell/transcripts/")
                      (agent-recall--project-root
                       "/work/clients/alpha/.agent-shell/transcripts/")
                      (agent-recall--transcript-dir-from-file
                       "/work/alpha/transcripts/session.org")
                      (cl-letf (((symbol-function
                                 'agent-recall--read-working-directory)
                                (lambda (_file) "/metadata/project")))
                        (list
                         (agent-recall--project-root-for-session
                          "/work/alpha/.agent-shell/transcripts/a.md")
                         (agent-recall--project-root-for-session
                          "/archive/a.org"))))"##;
    let expect = expect![[
        r#"OK ("alpha" "/work/clients/alpha" "/work/alpha/transcripts/" ("/work/alpha" "/metadata/project"))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_lists_only_configured_transcript_patterns_without_duplicates() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "list-transcripts"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (agent-recall-file-patterns
                            '("*.md" "*.org" "session*")))
                      (when (file-exists-p root)
                        (delete-directory root t))
                      (make-directory root t)
                      (dolist (entry '(("a.md" . "markdown")
                                       ("b.org" . "org")
                                       ("session.log" . "log")
                                       ("notes.txt" . "text")
                                       (".hidden.md" . "hidden")))
                        (with-temp-file (expand-file-name (car entry) root)
                          (insert (cdr entry))))
                      (unwind-protect
                          (mapcar #'file-name-nondirectory
                                  (sort
                                   (agent-recall--list-transcript-files root)
                                   #'string<))
                        (delete-directory root t)))"##;
    let expect = expect![[r#"OK (".hidden.md" "a.md" "b.org" "session.log")"#]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_index_save_and_load_round_trip_metadata_atomically() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "index-round-trip"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (agent-recall-index-file
                            (expand-file-name "state/index.el" root))
                           (agent-recall--index
                            (make-hash-table :test 'equal))
                           (agent-recall--index-loaded-p t))
                      (when (file-exists-p root)
                        (delete-directory root t))
                      (puthash "/project/a.md"
                               '(:project "alpha"
                                 :dir "/project"
                                 :timestamp "2026-01-02-03-04-05"
                                 :session-id "session-a"
                                 :preview "First question")
                               agent-recall--index)
                      (puthash "/project/b.org"
                               '(:project "beta"
                                 :dir "/project"
                                 :timestamp "2026-02-03-04-05-06"
                                 :session-id nil
                                 :preview "(empty)")
                               agent-recall--index)
                      (agent-recall--index-save)
                      (let ((disk-prefix
                             (with-temp-buffer
                               (insert-file-contents
                                agent-recall-index-file)
                               (buffer-substring-no-properties
                                (point-min)
                                (line-end-position))))
                            (temp-leftovers
                             (directory-files
                              (file-name-directory agent-recall-index-file)
                              nil "^\\.index-")))
                        (setq agent-recall--index nil
                              agent-recall--index-loaded-p nil)
                        (agent-recall--index-load)
                        (prog1
                            (list disk-prefix
                                  temp-leftovers
                                  agent-recall--index-loaded-p
                                  (hash-table-test agent-recall--index)
                                  (sort
                                   (mapcar
                                    (lambda (key)
                                      (cons key
                                            (gethash key
                                                     agent-recall--index)))
                                    (hash-table-keys agent-recall--index))
                                   (lambda (a b)
                                     (string< (car a) (car b)))))
                          (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK (";; agent-recall transcript index -*- no-byte-compile: t -*-" nil t equal (("/project/a.md" :project "alpha" :dir "/project" :timestamp "2026-01-02-03-04-05" :session-id "session-a" :preview "First question") ("/project/b.org" :project "beta" :dir "/project" :timestamp "2026-02-03-04-05-06" :session-id nil :preview "(empty)")))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_index_load_recovers_from_missing_non_table_and_reader_error_files() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "index-recovery"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (agent-recall-index-file
                            (expand-file-name "index.el" root))
                           outcomes)
                      (when (file-exists-p root)
                        (delete-directory root t))
                      (make-directory root t)
                      (dolist (content '(nil
                                         "(alpha beta)\n"
                                         "#<broken reader object"))
                        (if content
                            (with-temp-file agent-recall-index-file
                              (insert content))
                          (when (file-exists-p agent-recall-index-file)
                            (delete-file agent-recall-index-file)))
                        (setq agent-recall--index nil
                              agent-recall--index-loaded-p nil)
                        (push
                         (list agent-recall--index-loaded-p
                               (progn
                                 (agent-recall--index-load)
                                 agent-recall--index-loaded-p)
                               (hash-table-p agent-recall--index)
                               (hash-table-count agent-recall--index))
                         outcomes))
                      (delete-directory root t)
                      (nreverse outcomes))"##;
    let expect = expect!["OK ((nil t t 0) (nil t t 0) (nil t t 0))"];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_index_add_derives_real_project_preview_timestamp_and_session() {
    let elisp_form = r###"(let* ((root (expand-file-name
                                   "index-add/project/.agent-shell/transcripts"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (file (expand-file-name
                                  "2026-07-10-17-07-00.md" root))
                           (agent-recall-index-file
                            (expand-file-name
                             "../../../state/index.el" root))
                           (agent-recall--index
                            (make-hash-table :test 'equal))
                           (agent-recall--index-loaded-p t))
                      (make-directory root t)
                      (with-temp-file file
                        (insert "# Session\n\n## User\n> Design a resilient index\n\n"
                                "## Agent\nUse atomic rename.\n"))
                      (agent-recall--index-add file "session-007")
                      (let ((entry (gethash file agent-recall--index)))
                        (prog1
                            (list (plist-get entry :project)
                                  (file-name-nondirectory
                                   (plist-get entry :dir))
                                  (plist-get entry :timestamp)
                                  (plist-get entry :session-id)
                                  (plist-get entry :preview)
                                  (file-exists-p agent-recall-index-file))
                          (delete-directory
                           (expand-file-name "../../.." root) t))))"###;
    let expect = expect![[
        r#"OK ("project" "transcripts" "2026-07-10-17-07-00" "session-007" "Design a resilient index" t)"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_index_dirs_and_files_deduplicate_and_drop_deleted_transcripts() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "index-membership"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (one-dir (expand-file-name "one" root))
                           (two-dir (expand-file-name "two" root))
                           (one (expand-file-name "a.md" one-dir))
                           (two (expand-file-name "b.org" one-dir))
                           (gone (expand-file-name "gone.md" two-dir))
                           (agent-recall--index
                            (make-hash-table :test 'equal))
                           (agent-recall--index-loaded-p t))
                      (make-directory one-dir t)
                      (make-directory two-dir t)
                      (with-temp-file one (insert "one"))
                      (with-temp-file two (insert "two"))
                      (dolist (entry (list (cons one one-dir)
                                           (cons two one-dir)
                                           (cons gone two-dir)))
                        (puthash (car entry)
                                 (list :dir (directory-file-name
                                             (cdr entry)))
                                 agent-recall--index))
                      (prog1
                          (list
                           (sort
                            (mapcar #'file-name-nondirectory
                                    (agent-recall--index-dirs))
                            #'string<)
                           (sort
                            (mapcar #'file-name-nondirectory
                                    (agent-recall--index-files))
                            #'string<))
                        (delete-directory root t)))"##;
    let expect = expect![[r#"OK (("one" "two") ("a.md" "b.org"))"#]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_reindex_combines_conventional_and_extra_transcripts_in_one_index() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "reindex"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (project-dir
                            (expand-file-name
                             "projects/alpha/.agent-shell/transcripts"
                             root))
                           (extra-dir
                            (expand-file-name "org-archive" root))
                           (markdown
                            (expand-file-name
                             "2026-01-02-03-04-05.md" project-dir))
                           (org
                            (expand-file-name
                             "2026-02-03-04-05-06.org" extra-dir))
                           (agent-recall-search-paths
                            (list (expand-file-name "projects" root)))
                           (agent-recall-extra-transcript-dirs
                            (list (list :dir extra-dir)))
                           (agent-recall-index-file
                            (expand-file-name "state/index.el" root))
                           (agent-recall--index nil)
                           (agent-recall--index-loaded-p nil))
                      (make-directory project-dir t)
                      (make-directory extra-dir t)
                      (with-temp-file markdown
                        (insert "# Session\n## User\n> Markdown work\n"))
                      (with-temp-file org
                        (insert "#+PROPERTY: Working_Directory /work/beta\n"
                                "** User\nOrg work\n"))
                      (cl-letf (((symbol-function 'shell-command-to-string)
                                 (lambda (_command)
                                   (concat project-dir "\n")))
                                ((symbol-function
                                  'agent-recall--resolve-session-id)
                                 (lambda (file)
                                   (if (string-suffix-p ".md" file)
                                       "markdown-session"
                                     nil))))
                        (agent-recall-reindex))
                      (let (entries)
                        (maphash
                         (lambda (file props)
                           (push
                            (list (file-name-nondirectory file)
                                  (plist-get props :project)
                                  (plist-get props :timestamp)
                                  (plist-get props :session-id)
                                  (plist-get props :preview))
                            entries))
                         agent-recall--index)
                        (prog1
                            (list
                             (sort entries
                                   (lambda (a b)
                                     (string< (car a) (car b))))
                             agent-recall--index-loaded-p
                             (file-exists-p agent-recall-index-file))
                          (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK ((("2026-01-02-03-04-05.md" "alpha" "2026-01-02-03-04-05" "markdown-session" "Markdown work") ("2026-02-03-04-05-06.org" "beta" "2026-02-03-04-05-06" nil "Org work")) t t)"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_search_symlink_tree_handles_duplicate_projects_and_rebuilds_cleanly() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "symlink-index"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (one (expand-file-name
                                 "clients/one/shared/.agent-shell/transcripts"
                                 root))
                           (two (expand-file-name
                                 "clients/two/shared/.agent-shell/transcripts"
                                 root))
                           (third (expand-file-name
                                   "clients/three/unique/.agent-shell/transcripts"
                                   root))
                           (agent-recall-index-file
                            (expand-file-name "state/index.el" root))
                           (agent-recall--index
                            (make-hash-table :test 'equal))
                           (agent-recall--index-loaded-p t))
                      (dolist (dir (list one two third))
                        (make-directory dir t)
                        (puthash (expand-file-name "session.md" dir)
                                 (list :dir dir)
                                 agent-recall--index))
                      (let* ((first (agent-recall--ensure-symlink-dir))
                             (first-links
                              (sort
                               (directory-files first nil "^[^.]" t)
                               #'string<)))
                        (with-temp-file (expand-file-name "stale" first)
                          (insert "stale"))
                        (let ((second (agent-recall--ensure-symlink-dir)))
                          (prog1
                              (list first-links
                                    (sort
                                     (directory-files second nil "^[^.]" t)
                                     #'string<)
                                    (file-symlink-p
                                     (expand-file-name "shared" second))
                                    (file-symlink-p
                                     (expand-file-name "shared-1" second))
                                    agent-recall--symlink-dir)
                            (delete-directory root t)))))"##;
    let expect = expect![[
        r#"OK (("shared" "shared-1" "unique") ("shared" "shared-1" "unique") "[ORACLE-SANDBOX]/symlink-index/clients/two/shared/.agent-shell/transcripts" "[ORACLE-SANDBOX]/symlink-index/clients/one/shared/.agent-shell/transcripts" "[ORACLE-SANDBOX]/symlink-index/state/search")"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_invalidate_cache_clears_index_and_session_resolution_state() {
    let elisp_form = r##"(let ((agent-recall--index
                           (let ((table (make-hash-table :test 'equal)))
                             (puthash "a" '(:project "p") table)
                             table))
                          (agent-recall--index-loaded-p t)
                          (agent-recall--session-id-cache
                           (let ((table (make-hash-table :test 'equal)))
                             (puthash "a" "session-a" table)
                             table)))
                      (agent-recall-invalidate-cache)
                      (list agent-recall--index
                            agent-recall--index-loaded-p
                            (hash-table-count
                             agent-recall--session-id-cache)))"##;
    let expect = expect!["OK (nil nil 0)"];
    assert_agent_recall_parity(elisp_form, expect);
}

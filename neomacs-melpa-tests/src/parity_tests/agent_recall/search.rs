use expect_test::expect;

use super::assert_agent_recall_batch;

#[test]
fn search_public_surface_batch() {
    assert_agent_recall_batch(&[
        (
            "agent_recall_reindexes_then_searches_real_transcript_files_with_builtin_grep",
            r####"(let* ((root
                                 (expand-file-name
                                  "search"
                                  (getenv
                                   "NEOMACS_TEST_SANDBOX_ROOT")))
                                (alpha-dir
                                 (expand-file-name
                                  "alpha/.agent-shell/transcripts"
                                  root))
                                (beta-dir
                                 (expand-file-name
                                  "beta/.agent-shell/transcripts"
                                  root))
                                (agent-recall-search-paths
                                 (list root))
                                (agent-recall-index-file
                                 (expand-file-name
                                  "state/index.el"
                                  root))
                                (agent-recall-search-function
                                 'grep)
                                (agent-recall-search-context-lines
                                 0)
                                (agent-recall-auto-transcript-mode
                                 t)
                                (agent-recall--index nil)
                                (agent-recall--index-loaded-p nil)
                                (agent-recall--session-id-cache
                                 (make-hash-table
                                  :test
                                  'equal)))
                           (make-directory
                            alpha-dir
                            t)
                           (make-directory
                            beta-dir
                            t)
                           (with-temp-file
                               (expand-file-name
                                "alpha-session.md"
                                alpha-dir)
                             (insert
                              "## User Message\n"
                              "> Trace needle-token through the parser.\n\n"
                              "## Agent Response\n"
                              "The token enters the recovery queue.\n"))
                           (with-temp-file
                               (expand-file-name
                                "beta-session.org"
                                beta-dir)
                             (insert
                              "** User\n"
                              "Document the needle-token deployment fix.\n\n"
                              "** Assistant\n"
                              "The cache key now includes the target.\n"))
                           (with-temp-file
                               (expand-file-name
                                "ignored.txt"
                                alpha-dir)
                             (insert
                              "needle-token must not be searched"))
                           (unwind-protect
                               (progn
                                 (agent-recall-reindex)
                                 (agent-recall-search
                                  "needle-token")
                                 (let ((process
                                        (get-buffer-process
                                         "*grep*")))
                                   (while
                                       (and process
                                            (process-live-p process))
                                     (accept-process-output
                                      process
                                      0.05))
                                   (when process
                                     (accept-process-output
                                      process
                                      0.05))
                                   (with-current-buffer
                                       "*grep*"
                                     (let ((matches
                                            (seq-filter
                                             (lambda (line)
                                               (string-match-p
                                                "\\.\\(?:md\\|org\\):[0-9]+:"
                                                line))
                                             (split-string
                                              (buffer-substring-no-properties
                                               (point-min)
                                               (point-max))
                                              "\n"
                                              t))))
                                       (list
                                        (hash-table-count
                                         agent-recall--index)
                                        (derived-mode-p
                                         'grep-mode)
                                        agent-recall--search-buffer-p
                                        (and process
                                             (process-status process))
                                        (and process
                                             (process-exit-status process))
                                        (sort
                                         (mapcar
                                          (lambda (line)
                                            (replace-regexp-in-string
                                             (regexp-quote root)
                                             "[ROOT]"
                                             line
                                             t
                                             t))
                                          matches)
                                         #'string<))))))
                             (when-let ((buffer
                                         (get-buffer
                                          "*grep*")))
                               (kill-buffer buffer))
                             (delete-directory
                              root
                              t)))"####,
            true,
            expect![[
        r#"OK (2 grep-mode t exit 0 ("[ROOT]/alpha/.agent-shell/transcripts/alpha-session.md:2:> Trace needle-token through the parser." "[ROOT]/beta/.agent-shell/transcripts/beta-session.org:2:Document the needle-token deployment fix."))"#
    ]],
        ),
    ]);
}

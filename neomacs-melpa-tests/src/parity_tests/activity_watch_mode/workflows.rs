use expect_test::expect;

use super::assert_activity_watch_mode_parity;

/// The watcher's lifecycle, including the guard the package puts on it.  In a
/// plain batch session `activity-watch-mode' refuses to switch on at all.  In
/// an interactive-looking one it switches on but installs nothing yet - the
/// real work is deferred by a second - and after that second all three save
/// hooks, the `pre-command-hook' starter, the two second sampler and the thirty
/// second idle stopper are in place.  Switching it off again is guarded the
/// same way: done from batch the flag flips but the hooks and the timer stay,
/// and only the interactive route removes them.
#[test]
fn enabling_the_mode_installs_the_watch_hooks_and_timers() {
    let elisp_form = r##"(progn
  (aw-test-setup-server)
  (let ((buffer (aw-test-open "work/main.el" "(defun demo () 42)\n")))
    (unwind-protect
        (with-current-buffer buffer
          (let ((in-batch (progn (activity-watch-mode 1)
                                 (list activity-watch-mode
                                       (and (memq #'activity-watch--save after-save-hook) t)
                                       activity-watch-timer
                                       activity-watch-idle-timer))))
            (aw-test-interactive (activity-watch-mode 1))
            (let ((deferred (list activity-watch-mode
                                  (and (memq #'activity-watch--save after-save-hook) t)
                                  activity-watch-timer)))
              (aw-test-settle 1.3)
              (let ((running
                     (list activity-watch-mode
                           (mapcar (lambda (hook)
                                     (and (memq #'activity-watch--save
                                                (symbol-value hook))
                                          t))
                                   '(after-save-hook auto-save-hook first-change-hook))
                           (and (memq #'activity-watch--start-timer pre-command-hook) t)
                           (list (timer--function activity-watch-timer)
                                 (timer--repeat-delay activity-watch-timer))
                           (list (timer--function activity-watch-idle-timer)
                                 (timer--time activity-watch-idle-timer)
                                 (timer--repeat-delay activity-watch-idle-timer))
                           activity-watch-init-started
                           activity-watch-init-finished)))
                (activity-watch-mode -1)
                (let ((off-in-batch
                       (list activity-watch-mode
                             (and (memq #'activity-watch--save after-save-hook) t)
                             (and activity-watch-timer t))))
                  (aw-test-interactive (activity-watch-mode 1) (activity-watch-mode -1))
                  (list in-batch
                        deferred
                        running
                        off-in-batch
                        (list activity-watch-mode
                              (mapcar (lambda (hook)
                                        (and (memq #'activity-watch--save
                                                   (symbol-value hook))
                                             t))
                                      '(after-save-hook auto-save-hook first-change-hook))
                              (and (memq #'activity-watch--start-timer pre-command-hook) t)
                              activity-watch-timer
                              activity-watch-idle-timer)
                        (assq 'activity-watch-mode minor-mode-alist)))))))
      (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK ((nil nil nil nil) (t nil nil) (t (t t t) t (activity-watch--save 2) (activity-watch--stop-timer (0 30 0 0) t) t t) (nil t t) (nil (nil nil nil) nil nil nil) (activity-watch-mode " activity-watch"))"#
    ]];

    assert_activity_watch_mode_parity(elisp_form, expect);
}

/// What ActivityWatch actually receives.  The first edit in a watched buffer
/// creates the editor bucket and posts one heartbeat, and this pins both
/// complete requests: method, URL, the `pulsetime' query parameter, the
/// Content-Type header and the exact JSON body, including which fields carry
/// the language, project, file and branch.  Saving the same file straight
/// afterwards adds nothing, because a heartbeat for that file has just been
/// sent.
#[test]
fn saving_a_watched_file_creates_the_bucket_and_posts_one_heartbeat() {
    let elisp_form = r##"(progn
  (aw-test-setup-server)
  (aw-test-park-sampler)
  (let ((buffer (aw-test-open "work/main.el" "(defun demo () 42)\n"))
        (activity-watch-project-name-resolvers nil)
        (activity-watch-max-heartbeat-per-sec 3600))
    (unwind-protect
        (aw-test-watching buffer
          (insert ";; first edit\n")
          (aw-test-drain)
          (let ((after-edit (aw-test-requests)))
            (save-buffer)
            (aw-test-drain)
            (list after-edit
                  (equal after-edit (aw-test-requests))
                  activity-watch-bucket-created
                  (equal activity-watch-last-file-path (buffer-file-name))
                  (and activity-watch-last-heartbeat-time t)
                  (buffer-modified-p))))
      (activity-watch-turn-off)
      (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK ((("POST" "http://localhost:5600/api/0/buckets/aw-watcher-emacs_<HOST>" ("\"Content-Type: application/json\"") "{\"hostname\":\"<HOST>\",\"client\":\"emacs-activity-watch\",\"type\":\"app.editor.activity\"}") ("POST" "http://localhost:5600/api/0/buckets/aw-watcher-emacs_<HOST>/heartbeat?pulsetime=30" ("\"Content-Type: application/json\"") "{\"timestamp\":\"<TIME>\",\"duration\":0,\"data\":{\"language\":\"emacs-lisp-mode\",\"project\":\"unknown\",\"file\":\"[ORACLE-SANDBOX]/work/main.el\",\"branch\":\"unknown\"}}")) t t t t nil)"#
    ]];

    assert_activity_watch_mode_parity(elisp_form, expect);
}

/// When a heartbeat is *not* sent.  A second save of the same file inside the
/// `activity-watch-max-heartbeat-per-sec' window is dropped, and a buffer with
/// no file behind it never reports at all.  An auto-save file, though, is
/// reported: the package means to skip those, but it hands
/// `auto-save-file-name-p' the full path instead of the file name, so the check
/// never matches - the same call answers 0 for the bare name and nil for the
/// path.  Dropping the rate limit to zero then makes every activity event
/// report, so one edit-and-save cycle sends two heartbeats.
#[test]
fn heartbeats_are_skipped_for_rate_limits_and_buffers_without_a_file() {
    let elisp_form = r##"(progn
  (aw-test-setup-server)
  (aw-test-park-sampler)
  (let ((tracked (aw-test-open "work/main.el" "(defun demo () 42)\n"))
        (scratch (generate-new-buffer "*activity-watch-scratch*"))
        (autosaved (aw-test-open "work/#main.el#" ";; recovery\n"))
        (activity-watch-project-name-resolvers nil)
        (activity-watch-max-heartbeat-per-sec 3600))
    (unwind-protect
        (let* ((after-first-edit
                (aw-test-watching tracked
                  (insert ";; first edit\n")
                  (aw-test-drain)
                  (length (aw-test-requests))))
               (after-second-save
                (with-current-buffer tracked
                  (insert ";; second edit\n")
                  (save-buffer)
                  (aw-test-drain)
                  (length (aw-test-requests))))
               (after-scratch
                (aw-test-watching scratch
                  (insert "notes about nothing\n")
                  (aw-test-drain)
                  (list (buffer-file-name) (length (aw-test-requests)))))
               (after-autosave
                (aw-test-watching autosaved
                  (insert ";; more\n")
                  (aw-test-drain)
                  (list (auto-save-file-name-p (buffer-file-name))
                        (auto-save-file-name-p
                         (file-name-nondirectory (buffer-file-name)))
                        (length (aw-test-requests))))))
          (setq activity-watch-max-heartbeat-per-sec 0)
          (with-current-buffer tracked
            (insert ";; third edit\n")
            (save-buffer)
            (aw-test-drain))
          (let ((after-limit-lifted (length (aw-test-requests))))
            (with-current-buffer tracked
              (insert ";; fourth edit\n")
              (save-buffer)
              (aw-test-drain))
            (list after-first-edit
                  after-second-save
                  after-scratch
                  after-autosave
                  after-limit-lifted
                  (length (aw-test-requests))
                  (delete-dups (mapcar (lambda (request) (nth 1 request))
                                       (aw-test-requests))))))
      (activity-watch-turn-off)
      (dolist (buffer (list tracked scratch autosaved))
        (with-current-buffer buffer (set-buffer-modified-p nil))
        (kill-buffer buffer)))))"##;
    let expect = expect![[
        r#"OK (2 2 (nil 2) (nil 0 3) 5 7 ("http://localhost:5600/api/0/buckets/aw-watcher-emacs_<HOST>" "http://localhost:5600/api/0/buckets/aw-watcher-emacs_<HOST>/heartbeat?pulsetime=30"))"#
    ]];

    assert_activity_watch_mode_parity(elisp_form, expect);
}

/// Project attribution.  With the default resolver list nothing here is
/// available, so heartbeats carry the fallback name.  Switching to the `cwd'
/// resolver names the file's own directory, and switching to the `project'
/// resolver - after project.el is loaded - names the repository root a level
/// above it.  `activity-watch-refresh-project-name', the package's autoloaded
/// command, is what re-resolves the cached buffer-local name in between.
#[test]
fn the_heartbeat_names_the_project_the_configured_resolver_finds() {
    let elisp_form = r##"(progn
  (aw-test-setup-server)
  (aw-test-park-sampler)
  (make-directory (aw-test-path "repo/.git") t)
  (let ((buffer (aw-test-open "repo/src/lib.el" "(defun lib () 1)\n"))
        (activity-watch-max-heartbeat-per-sec 0))
    (unwind-protect
        (aw-test-watching buffer
          (insert ";; edit one\n")
          (save-buffer)
          (aw-test-drain)
          (let ((default-resolvers
                 (list activity-watch-project-name-resolvers
                       activity-watch-project-name
                       (mapcar (lambda (request) (nth 3 request)) (aw-test-requests)))))
            (aw-test-forget-requests)
            (setq activity-watch-project-name-resolvers '(cwd))
            (activity-watch-refresh-project-name)
            (insert ";; edit two\n")
            (save-buffer)
            (aw-test-drain)
            (let ((cwd-resolver (list activity-watch-project-name
                                      (mapcar (lambda (request) (nth 3 request))
                                              (aw-test-requests)))))
              (aw-test-forget-requests)
              (require 'project)
              (setq activity-watch-project-name-resolvers '(project))
              (activity-watch-refresh-project-name)
              (insert ";; edit three\n")
              (save-buffer)
              (aw-test-drain)
              (list default-resolvers
                    cwd-resolver
                    (list activity-watch-project-name
                          (mapcar (lambda (request) (nth 3 request))
                                  (aw-test-requests)))))))
      (activity-watch-turn-off)
      (with-current-buffer buffer (set-buffer-modified-p nil))
      (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK (((projectile project magit-dir-force magit-origin) "unknown" ("{\"hostname\":\"<HOST>\",\"client\":\"emacs-activity-watch\",\"type\":\"app.editor.activity\"}" "{\"hostname\":\"<HOST>\",\"client\":\"emacs-activity-watch\",\"type\":\"app.editor.activity\"}" "{\"timestamp\":\"<TIME>\",\"duration\":0,\"data\":{\"language\":\"emacs-lisp-mode\",\"project\":\"unknown\",\"file\":\"[ORACLE-SANDBOX]/repo/src/lib.el\",\"branch\":\"unknown\"}}" "{\"timestamp\":\"<TIME>\",\"duration\":0,\"data\":{\"language\":\"emacs-lisp-mode\",\"project\":\"unknown\",\"file\":\"[ORACLE-SANDBOX]/repo/src/lib.el\",\"branch\":\"unknown\"}}")) ("src" ("{\"timestamp\":\"<TIME>\",\"duration\":0,\"data\":{\"language\":\"emacs-lisp-mode\",\"project\":\"src\",\"file\":\"[ORACLE-SANDBOX]/repo/src/lib.el\",\"branch\":\"unknown\"}}" "{\"timestamp\":\"<TIME>\",\"duration\":0,\"data\":{\"language\":\"emacs-lisp-mode\",\"project\":\"src\",\"file\":\"[ORACLE-SANDBOX]/repo/src/lib.el\",\"branch\":\"unknown\"}}")) ("repo" ("{\"timestamp\":\"<TIME>\",\"duration\":0,\"data\":{\"language\":\"emacs-lisp-mode\",\"project\":\"repo\",\"file\":\"[ORACLE-SANDBOX]/repo/src/lib.el\",\"branch\":\"unknown\"}}" "{\"timestamp\":\"<TIME>\",\"duration\":0,\"data\":{\"language\":\"emacs-lisp-mode\",\"project\":\"repo\",\"file\":\"[ORACLE-SANDBOX]/repo/src/lib.el\",\"branch\":\"unknown\"}}")))"#
    ]];

    assert_activity_watch_mode_parity(elisp_form, expect);
}

/// The server is there but unhappy: both requests come back 500.  The bucket
/// is never marked as created, the package's error callback reports the
/// server's body and switches the watcher off - locally and globally - and
/// request logs its own failure too.  The hooks and the sampler are gone
/// afterwards, so the next edit and save send nothing at all.
#[test]
fn a_failing_server_turns_the_watcher_off() {
    let elisp_form = r##"(progn
  (aw-test-setup-server "500")
  (aw-test-park-sampler)
  (let ((buffer (aw-test-open "work/main.el" "(defun demo () 42)\n"))
        (activity-watch-project-name-resolvers nil)
        (activity-watch-max-heartbeat-per-sec 0))
    (unwind-protect
        (aw-test-watching buffer
          (aw-test-interactive
           (insert ";; first edit\n")
           (aw-test-drain)
           (aw-test-settle 0.5))
          (let ((after-failure
                 (list (length (aw-test-requests))
                       activity-watch-mode
                       (bound-and-true-p global-activity-watch-mode)
                       activity-watch-bucket-created
                       (and (memq #'activity-watch--save after-save-hook) t)
                       (and activity-watch-timer t)
                       (let ((messages (with-current-buffer "*Messages*" (buffer-string))))
                         (list (and (string-match-p "{\"error\":\"bucket missing\"}" messages) t)
                               (and (string-match-p "request-default-error-callback" messages) t))))))
            (aw-test-forget-requests)
            (aw-test-interactive
             (insert ";; second edit\n")
             (save-buffer)
             (aw-test-drain))
            (list after-failure
                  (aw-test-requests))))
      (activity-watch-turn-off)
      (with-current-buffer buffer (set-buffer-modified-p nil))
      (kill-buffer buffer))))"##;
    let expect = expect![[r#"OK ((2 nil nil nil nil nil (t t)) no-request)"#]];

    assert_activity_watch_mode_parity(elisp_form, expect);
}

/// With `activity-watch-org-clock-active' set, a heartbeat sent while an Org
/// clock is running carries the clocked task's property - the configured
/// `activity-watch-org-clock-property', lowercased, as the first key of the
/// data object - so time can be attributed to the ticket being worked on.
/// After clocking out the same edits produce heartbeats without it.
#[test]
fn an_active_org_clock_adds_its_ticket_property_to_the_heartbeat() {
    let elisp_form = r##"(progn
  (aw-test-setup-server)
  (aw-test-park-sampler)
  (require 'org)
  (require 'org-clock)
  (let ((org-clock-persist nil)
        (org-log-done nil)
        (tasks (aw-test-open "work/tasks.org"
                             (concat "* Fix the parser\n"
                                     ":PROPERTIES:\n"
                                     ":TICKET_ID: OPS-4711\n"
                                     ":END:\n"
                                     "Some notes.\n")))
        (code (aw-test-open "work/main.el" "(defun demo () 42)\n"))
        (activity-watch-project-name-resolvers nil)
        (activity-watch-max-heartbeat-per-sec 0)
        (activity-watch-org-clock-active t))
    (unwind-protect
        (progn
          (with-current-buffer tasks
            (goto-char (point-min))
            (org-clock-in))
          (aw-test-watching code
            (insert ";; edit while clocked in\n")
            (save-buffer)
            (aw-test-drain)
            (let ((clocked (mapcar (lambda (request) (nth 3 request)) (aw-test-requests))))
              (aw-test-forget-requests)
              (with-current-buffer tasks (org-clock-out))
              (insert ";; edit after clocking out\n")
              (save-buffer)
              (aw-test-drain)
              (list activity-watch-org-clock-property
                    (and (marker-buffer org-clock-marker) t)
                    clocked
                    (mapcar (lambda (request) (nth 3 request)) (aw-test-requests))))))
      (activity-watch-turn-off)
      (dolist (buffer (list tasks code))
        (with-current-buffer buffer (set-buffer-modified-p nil))
        (kill-buffer buffer)))))"##;
    let expect = expect![[
        r#"OK ("TICKET_ID" nil ("{\"hostname\":\"<HOST>\",\"client\":\"emacs-activity-watch\",\"type\":\"app.editor.activity\"}" "{\"hostname\":\"<HOST>\",\"client\":\"emacs-activity-watch\",\"type\":\"app.editor.activity\"}" "{\"timestamp\":\"<TIME>\",\"duration\":0,\"data\":{\"ticket_id\":\"OPS-4711\",\"language\":\"emacs-lisp-mode\",\"project\":\"unknown\",\"file\":\"[ORACLE-SANDBOX]/work/main.el\",\"branch\":\"unknown\"}}" "{\"timestamp\":\"<TIME>\",\"duration\":0,\"data\":{\"ticket_id\":\"OPS-4711\",\"language\":\"emacs-lisp-mode\",\"project\":\"unknown\",\"file\":\"[ORACLE-SANDBOX]/work/main.el\",\"branch\":\"unknown\"}}") ("{\"timestamp\":\"<TIME>\",\"duration\":0,\"data\":{\"language\":\"emacs-lisp-mode\",\"project\":\"unknown\",\"file\":\"[ORACLE-SANDBOX]/work/main.el\",\"branch\":\"unknown\"}}" "{\"timestamp\":\"<TIME>\",\"duration\":0,\"data\":{\"language\":\"emacs-lisp-mode\",\"project\":\"unknown\",\"file\":\"[ORACLE-SANDBOX]/work/main.el\",\"branch\":\"unknown\"}}"))"#
    ]];

    assert_activity_watch_mode_parity(elisp_form, expect);
}

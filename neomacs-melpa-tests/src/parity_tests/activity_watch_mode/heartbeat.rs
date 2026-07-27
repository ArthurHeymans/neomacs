use expect_test::expect;

use super::assert_activity_watch_mode_parity;

#[test]
fn activity_watch_mode_initialization_and_bucket_id_are_exact_and_idempotent() {
    let elisp_form = r##"(let ((activity-watch-init-started
                nil)
               (activity-watch-init-finished
                nil)
               calls)
         (cl-letf
             (((symbol-function
                'system-name)
               (lambda ()
                 (push
                  'system-name
                  calls)
                 "fixture-host")))
           (list
            (activity-watch--bucket-id)
            (activity-watch--init)
            activity-watch-init-started
            activity-watch-init-finished
            (activity-watch--init)
            activity-watch-init-started
            activity-watch-init-finished
            (nreverse calls))))"##;
    let expect = expect![[r#"OK ("aw-watcher-emacs_fixture-host" t t t nil t t (system-name))"#]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_create_bucket_builds_exact_request_and_success_callback_state() {
    let elisp_form = r##"(let ((activity-watch-api-host
                "https://activity.invalid")
               (activity-watch-bucket-created
                nil)
               requests)
         (cl-letf
             (((symbol-function
                'system-name)
               (lambda ()
                 "fixture-host"))
              ((symbol-function
                'request)
               (lambda
                 (url
                  &rest arguments)
                 (push
                  (cons url arguments)
                  requests)
                 'request-result)))
           (let ((first
                  (activity-watch--create-bucket)))
             (let* ((request
                     (car requests))
                    (success
                     (plist-get
                      (cdr request)
                      :success))
                    (before-success
                     activity-watch-bucket-created)
                    (callback-result
                     (funcall
                      success
                      :data
                      'ignored
                      :response
                      'ignored))
                    (after-success
                     activity-watch-bucket-created)
                    (second
                     (activity-watch--create-bucket)))
               (list
                first
                (list
                 (car request)
                 (plist-get
                  (cdr request)
                  :type)
                 (plist-get
                  (cdr request)
                  :data)
                 (plist-get
                  (cdr request)
                  :headers)
                 (functionp success))
                before-success
                callback-result
                after-success
                second
                (length requests))))))"##;
    let expect = expect![[
        r#"OK (request-result ("https://activity.invalid/api/0/buckets/aw-watcher-emacs_fixture-host" "POST" "{\"hostname\":\"fixture-host\",\"client\":\"emacs-activity-watch\",\"type\":\"app.editor.activity\"}" (("Content-Type" . "application/json")) t) nil t t nil 1)"#
    ]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_heartbeat_creation_covers_file_project_language_branch_and_org_injection() {
    let elisp_form = r##"(let ((time
                '(fixture-time))
               calls)
         (cl-letf
             (((symbol-function
                'ert--format-time-iso8601)
               (lambda (received-time)
                 (push
                  (list
                   'format-time
                   received-time)
                  calls)
                 "2026-07-26T12:34:56Z"))
              ((symbol-function
                'activity-watch--get-project)
               (lambda
                 (&optional refresh)
                 (push
                  (list
                   'project
                   refresh)
                  calls)
                 "fixture-project"))
              ((symbol-function
                'activity-watch--inject-org-property)
               (lambda (heartbeat)
                 (push
                  (list
                   'inject
                   (copy-tree heartbeat))
                  calls)
                 (append heartbeat
                         '((injected . t)))))
              ((symbol-function
                'magit-get-current-branch)
               (lambda ()
                 (push
                  'branch
                  calls)
                 "feature/test")))
           (list
            (with-temp-buffer
              (setq buffer-file-name
                    "/workspace/src/main.rs"
                    major-mode
                    'rust-mode)
              (activity-watch--create-heartbeat
               time))
            (with-temp-buffer
              (setq buffer-file-name
                    nil
                    major-mode
                    (intern
                     ""))
              (cl-letf
                  (((symbol-function
                     'magit-get-current-branch)
                    (lambda ()
                      nil)))
                (activity-watch--create-heartbeat
                 time)))
            (with-temp-buffer
              (setq buffer-file-name
                    nil
                    major-mode
                    'fundamental-mode)
              (cl-letf
                  (((symbol-function
                     'magit-get-current-branch)
                    nil))
                (activity-watch--create-heartbeat
                 time)))
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (((timestamp . "2026-07-26T12:34:56Z") #1=(duration . 0) (data (language . rust-mode) (project . "fixture-project") (file . "/workspace/src/main.rs") (branch . "feature/test")) . #2=((injected . t))) ((timestamp . "2026-07-26T12:34:56Z") #1# (data (language . "unknown") (project . "fixture-project") (file . "unknown") (branch . "unknown")) . #2#) ((timestamp . "2026-07-26T12:34:56Z") #1# (data (language . fundamental-mode) (project . "fixture-project") (file . "unknown") (branch . "unknown")) . #2#) ((project nil) branch (format-time #3=(fixture-time)) (inject ((timestamp . "2026-07-26T12:34:56Z") (duration . 0) (data (language . rust-mode) (project . "fixture-project") (file . "/workspace/src/main.rs") (branch . "feature/test")))) (project nil) (format-time #3#) (inject ((timestamp . "2026-07-26T12:34:56Z") (duration . 0) (data (language . "unknown") (project . "fixture-project") (file . "unknown") (branch . "unknown")))) (project nil) (format-time #3#) (inject ((timestamp . "2026-07-26T12:34:56Z") (duration . 0) (data (language . fundamental-mode) (project . "fixture-project") (file . "unknown") (branch . "unknown"))))))"#
    ]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_send_heartbeat_builds_exact_request_and_preserves_callbacks() {
    let elisp_form = r##"(let ((activity-watch-api-host
                "https://activity.invalid")
               (activity-watch-pulse-time
                45)
               (heartbeat
                '((timestamp . "time")
                  (duration . 0)
                  (data
                   (project . "fixture"))))
               (on-error
                (lambda
                  (&rest _)
                  'error-callback))
               (on-success
                (lambda
                  (&rest _)
                  'success-callback))
               captured)
         (cl-letf
             (((symbol-function
                'system-name)
               (lambda ()
                 "fixture-host"))
              ((symbol-function
                'request)
               (lambda
                 (url
                  &rest arguments)
                 (setq captured
                       (cons url arguments))
                 'request-result)))
           (let ((explicit-result
                  (activity-watch--send-heartbeat
                   heartbeat
                   :on-error on-error
                   :on-success on-success)))
             (let ((explicit-request
                    captured))
               (setq captured
                     nil)
               (let ((default-result
                      (activity-watch--send-heartbeat
                       heartbeat)))
                 (list
                  explicit-result
                  (car explicit-request)
                  (plist-get
                   (cdr explicit-request)
                   :type)
                  (plist-get
                   (cdr explicit-request)
                   :params)
                  (plist-get
                   (cdr explicit-request)
                   :data)
                  (plist-get
                   (cdr explicit-request)
                   :headers)
                  (eq
                   (plist-get
                    (cdr explicit-request)
                    :success)
                   on-success)
                  (eq
                   (plist-get
                    (cdr explicit-request)
                    :error)
                   on-error)
                  default-result
                  (and
                   (plist-member
                    (cdr captured)
                    :success)
                   t)
                  (plist-get
                   (cdr captured)
                   :success)
                  (and
                   (plist-member
                    (cdr captured)
                    :error)
                   t)
                  (plist-get
                   (cdr captured)
                   :error)))))))"##;
    let expect = expect![[
        r#"OK (request-result "https://activity.invalid/api/0/buckets/aw-watcher-emacs_fixture-host/heartbeat" "POST" (("pulsetime" . 45)) "{\"timestamp\":\"time\",\"duration\":0,\"data\":{\"project\":\"fixture\"}}" (("Content-Type" . "application/json")) t t request-result t nil t nil)"#
    ]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_call_sends_on_file_change_or_elapsed_threshold_and_updates_state() {
    let elisp_form = r##"(let ((activity-watch-last-file-path
                nil)
               (activity-watch-last-heartbeat-time
                nil)
               (activity-watch-max-heartbeat-per-sec
                1)
               (coding-system-for-read
                'auto-save-coding)
               (times
                '(100.0 100.5 101.1 101.2))
               calls)
         (cl-letf
             (((symbol-function
                'float-time)
               (lambda
                 (&optional _)
                 (prog1
                     (car times)
                   (setq times
                         (cdr times)))))
              ((symbol-function
                'current-time)
               (lambda ()
                 '(current-time-value)))
              ((symbol-function
                'activity-watch--create-bucket)
               (lambda ()
                 (push
                  'create-bucket
                  calls)))
              ((symbol-function
                'activity-watch--create-heartbeat)
               (lambda (time)
                 (push
                  (list
                   'create-heartbeat
                   time
                   coding-system-for-read)
                  calls)
                 (list
                  'heartbeat
                  time)))
              ((symbol-function
                'activity-watch--send-heartbeat)
               (lambda
                 (heartbeat
                  &rest arguments)
                 (push
                  (list
                   'send
                   heartbeat
                   (and
                    (plist-member
                     arguments
                     :on-error)
                    t)
                   (functionp
                    (plist-get
                     arguments
                     :on-error))
                   coding-system-for-read)
                  calls)
                 'sent)))
           (with-temp-buffer
             (setq buffer-file-name
                   "/workspace/a.el")
             (let ((first
                    (activity-watch--call)))
               (let ((second
                      (activity-watch--call)))
                 (let ((third
                        (activity-watch--call)))
                   (setq buffer-file-name
                         "/workspace/b.el")
                   (setq coding-system-for-read
                         'utf-8)
                   (let ((fourth
                          (activity-watch--call)))
                     (list
                      first
                      second
                      third
                      fourth
                      activity-watch-last-file-path
                      activity-watch-last-heartbeat-time
                      (nreverse calls)))))))))"##;
    let expect = expect![[
        r#"OK (sent nil sent sent "/workspace/b.el" 101.2 (create-bucket (create-heartbeat #1=(current-time-value) nil) (send (heartbeat #1#) t t nil) create-bucket create-bucket (create-heartbeat #1# nil) (send (heartbeat #1#) t t nil) create-bucket (create-heartbeat #1# utf-8) (send (heartbeat #1#) t t utf-8)))"#
    ]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_call_error_callback_emits_message_and_disables_global_and_local_modes() {
    let elisp_form = r##"(let ((activity-watch-last-file-path
                nil)
               (activity-watch-last-heartbeat-time
                nil)
               callback
               calls)
         (cl-letf
             (((symbol-function
                'float-time)
               (lambda
                 (&optional _)
                 100.0))
              ((symbol-function
                'activity-watch--create-bucket)
               (lambda ()
                 'created))
              ((symbol-function
                'activity-watch--create-heartbeat)
               (lambda (_)
                 'heartbeat))
              ((symbol-function
                'activity-watch--send-heartbeat)
               (lambda
                 (_heartbeat
                  &rest arguments)
                 (setq callback
                       (plist-get
                        arguments
                        :on-error))
                 'sent))
              ((symbol-function
                'global-activity-watch-mode)
               (lambda (argument)
                 (push
                  (list
                   'global
                   argument)
                  calls)))
              ((symbol-function
                'activity-watch-mode)
               (lambda (argument)
                 (push
                  (list
                   'local
                   argument)
                  calls)))
              ((symbol-function
                'message)
               (lambda
                 (format-string
                  &rest arguments)
                 (push
                  (list
                   'message
                   format-string
                   arguments)
                  calls))))
           (with-temp-buffer
             (setq buffer-file-name
                   "/workspace/a.el")
             (let ((call-result
                    (activity-watch--call)))
               (list
                call-result
                (functionp callback)
                (funcall
                 callback
                 :data
                 "server unavailable"
                 :response
                 'ignored)
                (nreverse calls))))))"##;
    let expect = expect![[
        r#"OK (sent t #1=((local 0)) ((message "server unavailable" nil) (global 0) . #1#))"#
    ]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_save_calls_only_for_real_non_auto_save_files_and_preserves_match_data() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function
                'activity-watch--call)
               (lambda ()
                 (push
                  (list
                   (buffer-file-name)
                   (match-data))
                  calls)
                 'called))
              ((symbol-function
                'auto-save-file-name-p)
               (lambda (file)
                 (string-prefix-p
                  "#"
                  (file-name-nondirectory
                   file)))))
           (string-match
            "\\(a\\)"
            "a")
           (let ((original-match
                  (match-data)))
             (list
              (with-temp-buffer
                (setq buffer-file-name
                      nil)
                (activity-watch--save))
              (with-temp-buffer
                (setq buffer-file-name
                      "/workspace/#file#")
                (activity-watch--save))
              (with-temp-buffer
                (setq buffer-file-name
                      "/workspace/file.el")
                (activity-watch--save))
              (nreverse calls)
              (equal
               original-match
               (match-data))))))"##;
    let expect = expect![[r#"OK (nil nil called (("/workspace/file.el" (0 1 0 1))) t)"#]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

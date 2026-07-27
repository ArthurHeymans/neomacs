use expect_test::expect;

use super::assert_agent_shell_parity;

#[test]
fn project_helpers_return_relative_files_custom_cwd_and_stable_name() {
    let elisp_form = r##"
(let ((default-directory "/workspace/fallback/")
      (agent-shell-cwd-function (lambda () "/workspace/custom/"))
      (projectile-mode nil))
  (cl-letf (((symbol-function 'project-current) (lambda (&rest _) 'project))
            ((symbol-function 'project-root)
             (lambda (_) "/workspace/project/"))
            ((symbol-function 'project-name)
             (lambda (_) "neomacs"))
            ((symbol-function 'project-files)
             (lambda (_)
               '("/workspace/project/src/lib.rs"
                 "/workspace/project/Cargo.toml"
                 "/workspace/project/docs/design.org"))))
    (list (agent-shell-cwd)
          (agent-shell--project-name)
          (agent-shell--project-files))))
"##;
    let expect = expect![[
        r#"OK ("/workspace/custom/" "neomacs" ("src/lib.rs" "Cargo.toml" "docs/design.org"))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn devcontainer_paths_round_trip_real_config_and_reject_escape_attempts() {
    let elisp_form = r##"
(let* ((root (file-name-as-directory
              (expand-file-name "agent-shell-devcontainer"
                                (getenv "NEOMACS_TEST_SANDBOX_ROOT"))))
       (config (expand-file-name ".devcontainer/devcontainer.json" root))
       (agent-shell-text-file-capabilities t))
  (make-directory (file-name-directory config) t)
  (make-directory (expand-file-name "src" root) t)
  (with-temp-file config
    (insert "{\"workspaceFolder\":\"/workspaces/neomacs/\"}"))
  (cl-letf (((symbol-function 'agent-shell-cwd) (lambda () root)))
    (mapcar
     (lambda (path)
       (condition-case error
           (agent-shell-devcontainer-resolve-path path)
         (error (list (car error) (error-message-string error)))))
     (list (expand-file-name "src/lib.rs" root)
           "/workspaces/neomacs/src/lib.rs"
           "/workspaces/neomacs/../secret"
           "/another-container/file"))))
"##;
    let expect = expect![[
        r#"OK ("/workspaces/neomacs/src/lib.rs" "[ORACLE-SANDBOX]/agent-shell-devcontainer/src/lib.rs" (error "Resolves to path outside of working directory: /workspaces/neomacs/../secret") (error "Unexpected path outside of workspace folder: /another-container/file"))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn devcontainer_refuses_container_to_host_mapping_without_text_capability() {
    let elisp_form = r##"
(let* ((root (file-name-as-directory
              (expand-file-name "agent-shell-devcontainer-disabled"
                                (getenv "NEOMACS_TEST_SANDBOX_ROOT"))))
       (config (expand-file-name ".devcontainer/devcontainer.json" root))
       (agent-shell-text-file-capabilities nil))
  (make-directory (file-name-directory config) t)
  (with-temp-file config
    (insert "{\"workspaceFolder\":\"/workspace/\"}"))
  (cl-letf (((symbol-function 'agent-shell-cwd) (lambda () root)))
    (list
     (agent-shell-devcontainer-resolve-path
      (expand-file-name "Cargo.toml" root))
     (condition-case error
         (agent-shell-devcontainer-resolve-path "/workspace/Cargo.toml")
       (error (list (car error) (error-message-string error)))))))
"##;
    let expect = expect![[
        r#"OK ("/workspace/Cargo.toml" (error "Refuse to resolve to local filesystem with text file capabilities disabled: /workspace/Cargo.toml"))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn heartbeat_start_is_idempotent_and_stop_finishes_the_callback_contract() {
    let elisp_form = r##"
(let ((events nil)
      (cancelled 0)
      (timer (timer-create)))
  (cl-letf (((symbol-function 'run-at-time)
             (lambda (&rest _) timer))
            ((symbol-function 'cancel-timer)
             (lambda (_) (setq cancelled (1+ cancelled)))))
    (let ((heartbeat
           (agent-shell-heartbeat-make
            :beats-per-second 4
            :on-heartbeat
            (lambda (value status)
              (push (list value status) events)))))
      (agent-shell-heartbeat-start :heartbeat heartbeat)
      (agent-shell-heartbeat-start :heartbeat heartbeat)
      (agent-shell-heartbeat-stop :heartbeat heartbeat)
      (list (nreverse events)
            cancelled
            (map-elt heartbeat :beats-per-second)
            (map-elt heartbeat :heartbeat-timer)
            (map-elt heartbeat :value)
            (map-elt heartbeat :status)))))
"##;
    let expect = expect!["OK (((0 started) (nil ended)) 1 4 nil nil ended)"];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn active_message_pairs_progress_reporter_and_timer_cleanup() {
    let elisp_form = r##"
(let ((events nil)
      (timer (timer-create)))
  (cl-letf (((symbol-function 'make-progress-reporter)
             (lambda (text) (list 'reporter text)))
            ((symbol-function 'run-at-time)
             (lambda (&rest args)
               (push (cons 'scheduled (seq-take args 2)) events)
               timer))
            ((symbol-function 'cancel-timer)
             (lambda (_) (push 'cancelled events)))
            ((symbol-function 'progress-reporter-done)
             (lambda (reporter) (push (list 'done reporter) events)))
            ((symbol-function 'message)
             (lambda (&rest args) (push (cons 'message args) events))))
    (let ((active (agent-shell-active-message-show
                   :text "Waiting for Codex")))
      (agent-shell-active-message-hide :active-message active)
      (list active (nreverse events)))))
"##;
    let expect = expect![[
        r#"OK (((:reporter . #1=(reporter "Waiting for Codex")) (:timer . [t nil nil nil nil nil nil nil nil nil])) ((scheduled 0 0.1) cancelled (done #1#) (message nil)))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn experimental_session_push_tracks_request_and_sends_final_response() {
    let elisp_form = r##"
(let ((responses nil)
      (heartbeats nil)
      (finished 0)
      (agent-shell-show-busy-indicator t)
      (state
       '((:client . client)
         (:active-requests . nil)
         (:heartbeat . heartbeat)
         (:last-entry-type . nil))))
  (cl-letf (((symbol-function 'agent-shell-experimental--remove-trailing-prompt)
             #'ignore)
            ((symbol-function 'agent-shell-heartbeat-start)
             (lambda (&rest args) (push (cons 'start args) heartbeats)))
            ((symbol-function 'agent-shell-heartbeat-stop)
             (lambda (&rest args) (push (cons 'stop args) heartbeats)))
            ((symbol-function 'acp-send-response)
             (lambda (&rest args) (push args responses))))
    (agent-shell-experimental--on-session-push-request
     :state state
     :acp-request
     '((jsonrpc . "2.0") (id . 77) (method . "session/push")
       (params . ((prompt . [((type . "text") (text . "Review"))])))))
    (let ((during (copy-tree state)))
      (agent-shell-experimental--on-session-push-end
       :state state
       :on-finished (lambda () (setq finished (1+ finished))))
      (list during state
            (nreverse responses)
            (nreverse heartbeats)
            finished))))
"##;
    let expect = expect![[
        r#"OK (((:client . client) (:active-requests ((:jsonrpc . "2.0") (:id . 77) (:method . "session/push") (:params (prompt . [((type . "text") (text . "Review"))])))) (:heartbeat . heartbeat) (:last-entry-type . "session/push")) ((:client . client) (:active-requests) (:heartbeat . heartbeat) (:last-entry-type . "session_push_end")) ((:client client :response ((:request-id . 77) (:result)))) ((start :heartbeat heartbeat) (stop :heartbeat heartbeat)) 1)"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn experimental_session_push_busy_path_returns_protocol_error() {
    let elisp_form = r##"
(let ((response nil)
      (state
       '((:client . client)
         (:active-requests .
          (((:id . "active") (:method . "session/prompt")))))))
  (cl-letf (((symbol-function 'acp-make-error)
             (lambda (&rest arguments) arguments))
            ((symbol-function 'acp-send-response)
             (lambda (&rest arguments) (setq response arguments))))
    (agent-shell-experimental--on-session-push-request
     :state state
     :acp-request '((id . 88) (method . "session/push")))
    (list response state)))
"##;
    let expect = expect![[
        r#"OK ((:client client :response ((:request-id . 88) (:error :code -32000 :message "Busy"))) ((:client . client) (:active-requests ((:id . "active") (:method . "session/prompt")))))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn artist_commit_trims_drawing_inserts_it_and_records_history() {
    let elisp_form = r##"
(let ((target (generate-new-buffer " *agent-shell-art-target*"))
      (agent-shell-artist-history-ring (make-ring 4)))
  (unwind-protect
      (let ((scratch (generate-new-buffer " *agent-shell-art-scratch*")))
        (unwind-protect
            (with-current-buffer scratch
              (insert "\n\n  +-----+  \n  | API |  \n  +-----+  \n\n")
              (setq-local agent-shell-artist--target-buffer target)
              (setq-local agent-shell-artist--target-point 1)
              (cl-letf (((symbol-function 'quit-window) #'ignore))
                (agent-shell-artist-commit))
              (list (with-current-buffer target (buffer-string))
                    (ring-elements agent-shell-artist-history-ring)))
          (kill-buffer scratch)))
    (kill-buffer target)))
"##;
    let expect = expect![[
        r#"OK ("\n\n  +-----+  \n  | API |  \n  +-----+\n\n" ("  +-----+  \n  | API |  \n  +-----+"))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn artist_history_navigation_and_tool_labels_form_a_coherent_workflow() {
    let elisp_form = r##"
(let ((agent-shell-artist-history-ring (make-ring 4)))
  (ring-insert agent-shell-artist-history-ring "newest")
  (ring-insert agent-shell-artist-history-ring "latest")
  (with-temp-buffer
    (setq-local agent-shell-artist--history-pos nil)
    (agent-shell-artist-previous-history)
    (let ((first (buffer-string)))
      (agent-shell-artist-previous-history)
      (let ((second (buffer-string)))
        (agent-shell-artist-next-history)
        (list first second (buffer-string)
              (agent-shell-artist--mark-active t "Rectangle" 11)
              (agent-shell-artist--mark-active nil "Line" 11)
              (mapcar #'car
                      (agent-shell-artist--ops-in-column 'erase)))))))
"##;
    let expect = expect![[
        r#"OK ("latest" "newest" "latest" "Rectangle  [✓]" "Line       [ ]" (erase-char erase-rect vaporize-line vaporize-lines))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn work_buffer_macro_isolatedly_returns_body_value_and_discards_content() {
    let elisp_form = r##"
(let ((outside (current-buffer)))
  (list
   (agent-shell-with-work-buffer
     (insert "temporary analysis")
     (list (buffer-string)
           (eq outside (current-buffer))
           (buffer-modified-p)))
   (buffer-string)))
"##;
    let expect = expect![[r#"OK (("temporary analysis" nil t) "")"#]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn worktree_name_generation_uses_both_frozen_word_lists() {
    let elisp_form = r##"
(let* ((name (agent-shell-worktree--generate-name))
       (parts (split-string name "-")))
  (list (length agent-shell-worktree--adjectives)
        (car agent-shell-worktree--adjectives)
        (car (last agent-shell-worktree--adjectives))
        (length agent-shell-worktree--scientists)
        (car agent-shell-worktree--scientists)
        (car (last agent-shell-worktree--scientists))
        (= (length parts) 2)
        (and (member (car parts)
                     agent-shell-worktree--adjectives)
             t)
        (and (member (cadr parts)
                     agent-shell-worktree--scientists)
             t)))
"##;
    let expect = expect![[r#"OK (108 "admiring" "zen" 227 "albattani" "zhukovsky" t t t)"#]];
    assert_agent_shell_parity(elisp_form, expect);
}

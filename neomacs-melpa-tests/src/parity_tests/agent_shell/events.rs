use expect_test::expect;

use super::assert_agent_shell_parity;

#[test]
fn event_bus_routes_filtered_and_unfiltered_events_then_unsubscribes() {
    let elisp_form = r##"
(let ((all nil)
      (turns nil)
      (state (list (cons :buffer (current-buffer))
                   (cons :event-subscriptions nil))))
  (cl-letf (((symbol-function 'agent-shell--state) (lambda () state)))
    (let ((all-token
           (agent-shell-subscribe-to
            :shell-buffer (current-buffer)
            :on-event (lambda (event) (push event all))))
          (turn-token
           (agent-shell-subscribe-to
            :shell-buffer (current-buffer)
            :event 'turn-complete
            :on-event (lambda (event) (push event turns)))))
      (agent-shell--emit-event :event 'input-submitted
                               :data '((:prompt . "review code")))
      (agent-shell--emit-event :event 'turn-complete
                               :data '((:stop-reason . "end_turn")
                                       (:usage . ((:total-tokens . 1500)))))
      (agent-shell-unsubscribe :subscription all-token)
      (agent-shell--emit-event :event 'turn-complete
                               :data '((:stop-reason . "cancelled")))
      (agent-shell-unsubscribe :subscription turn-token)
      (list (nreverse all)
            (nreverse turns)
            (map-elt state :event-subscriptions)))))
"##;
    let expect = expect![[
        r#"OK ((((:data (:prompt . "review code")) (:event . input-submitted)) #1=((:data (:stop-reason . "end_turn") (:usage (:total-tokens . 1500))) (:event . turn-complete))) (#1# ((:data (:stop-reason . "cancelled")) (:event . turn-complete))) nil)"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn event_bus_isolates_a_failing_extension_from_healthy_subscribers() {
    let elisp_form = r##"
(let ((received nil)
      (state (list (cons :buffer (current-buffer))
                   (cons :event-subscriptions nil))))
  (cl-letf (((symbol-function 'agent-shell--state) (lambda () state)))
    (agent-shell-subscribe-to
     :shell-buffer (current-buffer)
     :on-event (lambda (_) (error "extension exploded")))
    (agent-shell-subscribe-to
     :shell-buffer (current-buffer)
     :on-event (lambda (event) (push event received)))
    (let ((result
           (condition-case error
               (progn
                 (agent-shell--emit-event
                  :event 'file-write
                  :data '((:path . "/workspace/src/lib.rs")
                          (:bytes . 128)))
                 'dispatch-returned)
             (error (list 'escaped (error-message-string error))))))
      (list result received))))
"##;
    let expect = expect![[
        r#"OK (dispatch-returned (((:data (:path . "/workspace/src/lib.rs") (:bytes . 128)) (:event . file-write))))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn system_sleep_block_tracks_busy_blocked_ready_and_terminal_states() {
    let elisp_form = r##"
(let ((blocked 0)
      (status 'busy)
      (features (cons 'system-sleep features))
      (state (list (cons :buffer (current-buffer))
                   (cons :event-subscriptions nil)
                   (cons :sleep-token nil)))
      (agent-shell-inhibit-system-sleep t)
      snapshots)
  (cl-letf (((symbol-function 'agent-shell--state) (lambda () state))
            ((symbol-function 'agent-shell-status) (lambda (&rest _) status))
            ((symbol-function 'system-sleep-block-sleep)
             (lambda (&rest _) (setq blocked (1+ blocked)) 'token))
            ((symbol-function 'system-sleep-unblock-sleep)
             (lambda (_) (setq blocked (1- blocked)))))
    (dolist (step '((busy input-submitted)
                    (busy tool-call-update)
                    (blocked permission-request)
                    (busy permission-response)
                    (ready turn-complete)
                    (busy input-submitted)
                    (busy error)))
      (setq status (car step))
      (agent-shell--emit-event :event (cadr step))
      (push (list status (cadr step) blocked
                  (map-elt state :sleep-token))
            snapshots))
    (nreverse snapshots)))
"##;
    let expect = expect![
        "OK ((busy input-submitted 1 token) (busy tool-call-update 1 token) (blocked permission-request 0 nil) (busy permission-response 1 token) (ready turn-complete 0 nil) (busy input-submitted 1 token) (busy error 0 nil))"
    ];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn activity_group_ids_keep_thoughts_and_tools_together_but_split_on_messages() {
    let elisp_form = r##"
(let ((state (list (cons :last-entry-type nil)
                   (cons :activity-group-count 0))))
  (mapcar
   (lambda (entry-type)
     (map-put! state :last-entry-type entry-type)
     (list entry-type
           (agent-shell--activity-group-current-id state)
           (map-elt state :activity-group-count)))
   '(nil "tool_call" "agent_thought_chunk" "tool_call_update"
         "agent_message_chunk" "tool_call" "agent_message_chunk"
         "agent_thought_chunk")))
"##;
    let expect = expect![[
        r#"OK ((nil "activity-1" 1) ("tool_call" "activity-1" 1) ("agent_thought_chunk" "activity-1" 1) ("tool_call_update" "activity-1" 1) ("agent_message_chunk" "activity-2" 2) ("tool_call" "activity-2" 2) ("agent_message_chunk" "activity-3" 3) ("agent_thought_chunk" "activity-3" 3))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn activity_descriptions_conjugate_and_count_real_mixed_tool_runs() {
    let elisp_form = r##"
(let ((cases
       '(((("a" (:kind . "execute") (:status . "completed"))
           ("b" (:kind . "execute") (:status . "completed"))
           ("c" (:kind . "read") (:status . "completed"))) nil)
         ((("a" (:kind . "execute") (:status . "completed"))
           ("b" (:kind . "execute") (:status . "in_progress"))) nil)
         ((("a" (:kind . "search") (:status . "failed"))
           ("b" (:kind . "edit") (:status . "completed"))
           ("c" (:kind . nil) (:status . "completed"))) t)
         (nil t))))
  (list
   (mapcar
    (lambda (case)
      (agent-shell--activity-group-descriptive-text
       :members (car case) :thought (cadr case)))
    cases)
   (mapcar
    (lambda (spec)
      (apply #'agent-shell--tool-call-kind-phrase spec))
    '((:kind "execute" :count 1)
      (:kind "execute" :count 3)
      (:kind "execute" :count 1 :pending t)
      (:kind "read" :count 2)
      (:kind "mystery" :count 4)))))
"##;
    let expect = expect![[
        r#"OK (("Ran 2 commands, read a file" "Run 2 commands" "Thought, ran a search, edited a file, ran a tool call" "Thought") ("ran a command" "ran 3 commands" "run a command" "read 2 files" "ran 4 tool calls"))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn activity_count_descriptive_and_tally_labels_summarize_the_same_state() {
    let elisp_form = r##"
(let* ((state
        '((:tool-calls .
           (("cmd" (:group-id . "g") (:kind . "execute")
             (:status . "completed"))
            ("read1" (:group-id . "g") (:kind . "read")
             (:status . "completed"))
            ("read2" (:group-id . "g") (:kind . "read")
             (:status . "in_progress"))
            ("edit" (:group-id . "g") (:kind . "edit")
             (:status . "failed"))
            ("other" (:group-id . "g") (:kind . nil)
             (:status . "completed"))))
          (:activity-thoughts . (("g" . 2)))))
       (context (list (cons :state state) (cons :group-id "g")))
       (count (agent-shell-activity-group-count-label context))
       (descriptive
        (agent-shell-activity-group-descriptive-label context))
       (tally (agent-shell-activity-group-tally-label context)))
  (list (substring-no-properties count)
        (substring-no-properties descriptive)
        (substring-no-properties tally)
        (get-text-property 0 'font-lock-face tally)
        (get-text-property (1- (length tally))
                           'font-lock-face tally)))
"##;
    let expect = expect![[
        r#"OK ("✗ Activity 5/7" "Thought, ran a tool call, edited a file, read 2 files, ran a command" "Commands: 1 Reads: 2 Edits: 1 Other: 1 Thinking: 2" agent-shell-section-heading default)"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn notification_adapter_is_optional_and_can_normalize_wire_messages() {
    let elisp_form = r##"
(let* ((plain-state
        (agent-shell--make-state
         :agent-config
         (agent-shell-make-agent-config :identifier 'plain)))
       (adapted-state
        (agent-shell--make-state
         :agent-config
         (agent-shell-make-agent-config
          :identifier 'adapted
          :notification-adapter
          (lambda (&key acp-notification)
            (cons '(normalized . t) acp-notification))))))
  (list
   (agent-shell--adapt-notification
    :state plain-state
    :acp-notification '((method . "session/update")
                        (params (value . 1))))
   (agent-shell--adapt-notification
    :state adapted-state
    :acp-notification '((method . "session/update")
                        (params (value . 1))))))
"##;
    let expect = expect![[
        r#"OK (((method . "session/update") (params (value . 1))) ((normalized . t) (method . "session/update") (params (value . 1))))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

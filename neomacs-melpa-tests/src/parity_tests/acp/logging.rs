use expect_test::expect;

use super::assert_acp_parity;

#[test]
fn acp_upstream_unibyte_log_trimming_preserves_whole_message_boundaries() {
    let elisp_form = r##"(let* ((message-one
                     (cons
                      "A"
                      "one"))
                    (message-two
                     (cons
                      "B"
                      "two"))
                    (message-three
                     (cons
                      "C"
                      "three"))
                    (log-one
                     (acp--format-log-message
                      (car message-one)
                      "%s"
                      (cdr message-one)))
                    (log-two
                     (acp--format-log-message
                      (car message-two)
                      "%s"
                      (cdr message-two)))
                    (log-three
                     (acp--format-log-message
                      (car message-three)
                      "%s"
                      (cdr message-three)))
                    (max-bytes
                     (+
                      (string-bytes log-two)
                      (string-bytes log-three)))
                    (acp-logging-enabled t)
                    (acp--log-buffer-max-bytes
                     max-bytes)
                    (client
                     '((:command . "trim-unibyte")
                       (:instance-count . 1)))
                    (buffer
                     (acp-logs-buffer
                      :client client)))
         (unwind-protect
             (progn
               (with-current-buffer buffer
                 (erase-buffer))
               (dolist
                   (message
                    (list
                     message-one
                     message-two
                     message-three))
                 (acp--log
                  client
                  (car message)
                  "%s"
                  (cdr message)))
               (let ((result
                      (with-current-buffer buffer
                        (buffer-string))))
                 (list
                  log-one
                  log-two
                  log-three
                  max-bytes
                  result
                  (equal
                   result
                   (concat
                    log-two
                    log-three))
                  (<=
                   (string-bytes result)
                   max-bytes))))
           (when
               (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK ("A >\n\none\n\n" "B >\n\ntwo\n\n" "C >\n\nthree\n\n" 22 #("B >\n\ntwo\n\nC >\n\nthree\n\n" 0 1 (acp-log-boundary t) 10 11 (acp-log-boundary t)) t t)"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_upstream_multibyte_log_trimming_counts_bytes_and_keeps_whole_messages() {
    let elisp_form = r##"(let* ((message-one
                     (cons
                      "A"
                      "alpha"))
                    (message-two
                     (cons
                      "B"
                      "café ✓"))
                    (message-three
                     (cons
                      "C"
                      "omega"))
                    (log-two
                     (acp--format-log-message
                      (car message-two)
                      "%s"
                      (cdr message-two)))
                    (log-three
                     (acp--format-log-message
                      (car message-three)
                      "%s"
                      (cdr message-three)))
                    (characters
                     (+
                      (length log-two)
                      (length log-three)))
                    (bytes
                     (+
                      (string-bytes log-two)
                      (string-bytes log-three)))
                    (max-bytes
                     (1+ characters))
                    (acp-logging-enabled t)
                    (acp--log-buffer-max-bytes
                     max-bytes)
                    (client
                     '((:command . "trim-multibyte")
                       (:instance-count . 1)))
                    (buffer
                     (acp-logs-buffer
                      :client client)))
         (unwind-protect
             (progn
               (with-current-buffer buffer
                 (erase-buffer))
               (dolist
                   (message
                    (list
                     message-one
                     message-two
                     message-three))
                 (acp--log
                  client
                  (car message)
                  "%s"
                  (cdr message)))
               (let ((result
                      (with-current-buffer buffer
                        (buffer-string))))
                 (list
                  characters
                  bytes
                  max-bytes
                  (< max-bytes bytes)
                  result
                  (equal result log-three)
                  (<=
                   (string-bytes result)
                   max-bytes))))
           (when
               (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect = expect![[r#"OK (25 28 26 t #("C >\n\nomega\n\n" 0 1 (acp-log-boundary t)) t t)"#]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_log_format_and_insert_helpers_cover_labels_nil_labels_arguments_and_boundaries() {
    let elisp_form = r##"(with-temp-buffer
         (let ((labelled
                (acp--format-log-message
                 "OUT"
                 "%s=%d"
                 "x"
                 3))
               (plain
                (acp--format-log-message
                 nil
                 "[%s]"
                 "body")))
           (acp--insert-log-entry
            "ONE"
            "%s"
            "first")
           (let ((second-start
                  (point)))
             (acp--insert-log-entry
              nil
              "%s"
              "second")
             (list
              labelled
              plain
              (buffer-string)
              (get-text-property
               1
               'acp-log-boundary)
              (get-text-property
               second-start
               'acp-log-boundary)
              (next-single-property-change
               1
               'acp-log-boundary
               nil
               (point-max))
              (condition-case error
                  (acp--format-log-message
                   "x"
                   nil)
                (error
                 (list
                  (car error)
                  (cadr error))))))))"##;
    let expect = expect![[
        r#"OK ("OUT >\n\nx=3\n\n" "[body]\n\n" #("ONE >\n\nfirst\n\nsecond\n\n" 0 1 (acp-log-boundary t) 14 15 (acp-log-boundary t)) t t 2 (error ":format-string is required"))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_trim_log_buffer_handles_dead_buffers_defaults_noop_and_inside_boundary_offsets() {
    let elisp_form = r##"(let ((buffer
                (generate-new-buffer
                 " *acp-trim-edges*"))
               (dead
                (generate-new-buffer
                 " *acp-trim-dead*"))
               (acp--log-buffer-max-bytes
                12))
         (kill-buffer dead)
         (unwind-protect
             (progn
               (with-current-buffer buffer
                 (acp--insert-log-entry
                  "A"
                  "%s"
                  "1234")
                 (acp--insert-log-entry
                  "B"
                  "%s"
                  "5678"))
               (let ((before
                      (with-current-buffer buffer
                        (list
                         (buffer-string)
                         (acp--total-buffer-bytes
                          buffer)))))
                 (acp--trim-log-buffer
                  dead
                  1)
                 (acp--trim-log-buffer
                  buffer
                  1000)
                 (let ((noop
                        (with-current-buffer buffer
                          (buffer-string))))
                   (acp--trim-log-buffer
                    buffer)
                   (list
                    before
                    noop
                    (with-current-buffer buffer
                      (list
                       (buffer-string)
                       (acp--total-buffer-bytes
                        buffer)))))))
           (when
               (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK ((#("A >\n\n1234\n\nB >\n\n5678\n\n" 0 1 (acp-log-boundary t) 11 12 (acp-log-boundary t)) 22) #("A >\n\n1234\n\nB >\n\n5678\n\n" 0 1 (acp-log-boundary t) 11 12 (acp-log-boundary t)) (#("B >\n\n5678\n\n" 0 1 (acp-log-boundary t)) 11))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_logging_gate_routes_formatted_messages_and_traffic_only_when_enabled() {
    let elisp_form = r##"(let ((client
                '((:command . "gate")
                  (:instance-count . 1)))
               calls)
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'acp-traffic-log-traffic)
                   (lambda (&rest arguments)
                     (push
                      (cons
                       'traffic
                       arguments)
                      calls))))
               (let ((acp-logging-enabled nil))
                 (acp--log
                  client
                  "NO"
                  "%s"
                  "hidden")
                 (acp--log-traffic
                  client
                  'incoming
                  'request
                  'message))
               (let ((acp-logging-enabled t)
                     (acp--log-buffer-max-bytes
                      1000))
                 (acp--log
                  client
                  "YES"
                  "%s-%d"
                  "visible"
                  2)
                 (acp--log-traffic
                  client
                  'outgoing
                  'notification
                  'message))
               (list
                (with-current-buffer
                    (acp-logs-buffer
                     :client client)
                  (buffer-string))
                (nreverse calls)))
           (when-let*
               ((buffer
                 (get-buffer
                  "*acp-(gate)-1 log*")))
             (kill-buffer buffer))
           (when-let*
               ((buffer
                 (get-buffer
                  "*acp-(gate)-1 traffic*")))
             (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK (#("YES >\n\nvisible-2\n\n" 0 1 (acp-log-boundary t)) ((traffic :buffer (:buffer nil) :direction outgoing :kind notification :message message)))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_json_parse_serialize_and_pretty_print_cover_null_false_unicode_and_logging_gate() {
    let elisp_form = r##"(let ((json
                "{\"text\":\"café\",\"nil\":null,\"false\":false,\"array\":[1,true]}"))
         (list
          (acp--parse-json json)
          (acp--serialize-json
           '((jsonrpc . "2.0")
             (id . 1)
             (result . nil)))
          (let ((acp-logging-enabled nil))
            (acp--json-pretty-print
             "{\"x\":1}"))
          (let ((acp-logging-enabled t))
            (acp--json-pretty-print
             "{\"x\":1,\"y\":[2,3]}"))))"##;
    let expect = expect![[
        r#"OK (((text . "café") (nil) (false) (array . [1 t])) "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n" "{\"x\":1}" "{\n  \"x\": 1,\n  \"y\": [\n    2,\n    3\n  ]\n}")"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_log_and_traffic_buffers_reuse_names_reset_contents_and_show_json_read_only() {
    let elisp_form = r##"(let ((client
                '((:command . "buffers")
                  (:instance-count . 9)))
               displayed)
         (unwind-protect
             (let ((logs-one
                    (acp-logs-buffer
                     :client client))
                   (logs-two
                    (acp-logs-buffer
                     :client client))
                   (traffic-one
                    (acp-traffic-buffer
                     :client client))
                   (traffic-two
                    (acp-traffic-buffer
                     :client client)))
               (with-current-buffer logs-one
                 (let ((inhibit-read-only t))
                   (insert
                    "log")))
               (with-current-buffer traffic-one
                 (let ((inhibit-read-only t))
                   (insert
                    "traffic")))
               (let ((reset-outcome
                      (condition-case error
                          (list
                           'ok
                           (acp-reset-logs
                            :client client))
                        (error
                         (list
                          'error
                          (car error))))))
                 (cl-letf
                   (((symbol-function
                      'display-buffer)
                     (lambda (buffer)
                       (setq displayed
                             (buffer-name buffer))
                       buffer)))
                 (acp--show-json-object
                  '((b . [2 3])
                    (a . 1))))
               (list
                reset-outcome
                (eq logs-one logs-two)
                (buffer-name logs-one)
                (with-current-buffer logs-one
                  (list
                   (buffer-string)
                   buffer-undo-list))
                (eq traffic-one traffic-two)
                (buffer-name traffic-one)
                (with-current-buffer traffic-one
                  (buffer-string))
                displayed
                (with-current-buffer
                    "*acp object*"
                  (list
                   (buffer-string)
                   (point)
                   buffer-read-only)))))
           (dolist
               (name
                '("*acp-(buffers)-9 log*"
                  "*acp-(buffers)-9 traffic*"
                  "*acp object*"))
             (when-let*
                 ((buffer
                   (get-buffer name)))
               (kill-buffer buffer)))))"##;
    let expect = expect![[
        r#"OK ((error buffer-read-only) t "*acp-(buffers)-9 log*" ("" t) t "*acp-(buffers)-9 traffic*" "traffic" "*acp object*" ("{\n  \"b\": [\n    2,\n    3\n  ],\n  \"a\": 1\n}" 1 t))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

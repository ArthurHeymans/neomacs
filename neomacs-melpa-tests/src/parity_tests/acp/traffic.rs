use expect_test::expect;

use super::assert_acp_traffic_parity;

#[test]
fn acp_traffic_modes_install_exact_maps_read_only_state_and_single_highlight_overlay() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (insert
            "one\ntwo\n")
           (goto-char 5)
           (acp-traffic-mode)
           (let ((overlays
                  (overlays-in
                   (point-min)
                   (point-max))))
             (list
              major-mode
              mode-name
              buffer-read-only
              (eq
               (current-local-map)
               acp-traffic-mode-map)
              (mapcar
               (lambda (overlay)
                 (list
                  (overlay-start overlay)
                  (overlay-end overlay)
                  (overlay-get
                   overlay
                   'face)
                  (overlay-get
                   overlay
                   'acp-traffic)))
               overlays))))
         (with-temp-buffer
           (acp-traffic-entry-mode)
           (list
            major-mode
            mode-name
            buffer-read-only
            (eq
             (current-local-map)
             acp-traffic-entry-mode-map)
            (local-variable-p
             'acp-traffic-entry--traffic-buffer)
            acp-traffic-entry--traffic-buffer)))"##;
    let expect = expect![[
        r#"OK ((acp-traffic-mode "ACP-traffic" t t ((5 9 highlight t))) (acp-traffic-entry-mode "ACP-traffic-entry" t t nil nil))"#
    ]];
    assert_acp_traffic_parity(elisp_form, expect);
}

#[test]
fn acp_traffic_get_buffer_validates_name_reuses_buffer_and_initializes_mode_once() {
    let elisp_form = r##"(let ((name
                "*acp-traffic-get-fixture*"))
         (unwind-protect
             (let ((first
                    (acp-traffic-get-buffer
                     :named name))
                   second)
               (setq second
                     (acp-traffic-get-buffer
                      :named name))
               (list
                (eq first second)
                (buffer-name first)
                (with-current-buffer first
                  (list
                   major-mode
                   buffer-read-only
                   buffer-undo-list))
                (condition-case error
                    (acp-traffic-get-buffer)
                  (error
                   (list
                    (car error)
                    (cadr error))))))
           (when-let*
               ((buffer
                 (get-buffer name)))
             (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK (t "*acp-traffic-get-fixture*" (acp-traffic-mode t t) (error ":named is required"))"#
    ]];
    assert_acp_traffic_parity(elisp_form, expect);
}

#[test]
fn acp_traffic_log_formats_all_directions_and_kinds_with_exact_properties() {
    let elisp_form = r##"(let ((buffer
                (generate-new-buffer
                 " *acp-traffic-log-fixture*")))
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'format-time-string)
                   (lambda (format-string &rest arguments)
                     (list format-string arguments)
                     "12:34:56.789")))
               (dolist
                   (entry
                    '((incoming
                       request
                       ((method . "initialize")))
                      (outgoing
                       response
                       ((result . ((ok . t)))))
                      (incoming
                       response
                       ((id . 1)
                        (result . nil)))
                      (incoming
                       response
                       ((error . ((code . -1)))))
                      (outgoing
                       notification
                       ((other . t)))))
                 (acp-traffic-log-traffic
                  :buffer buffer
                  :direction
                  (nth 0 entry)
                  :kind
                  (nth 1 entry)
                  :message
                  `((:object
                     .
                     ,(nth 2 entry)))))
               (with-current-buffer buffer
                 (let (lines)
                   (goto-char
                    (point-min))
                   (while
                       (not
                        (eobp))
                     (push
                      (list
                       (buffer-substring-no-properties
                        (line-beginning-position)
                        (line-end-position))
                       (get-text-property
                        (line-beginning-position)
                        'acp-traffic-object)
                       (get-text-property
                        (+
                         (line-beginning-position)
                         13)
                        'face)
                       (get-text-property
                        (max
                         (line-beginning-position)
                         (1-
                          (line-end-position)))
                        'face))
                      lines)
                     (forward-line 1))
                   (nreverse lines))))
           (when
               (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK (("12:34:56.789 ← request      initialize" ((:direction . incoming) (:kind . request) (:object (method . "initialize"))) success font-lock-function-name-face) ("12:34:56.789 → response     result" ((:direction . outgoing) (:kind . response) (:object (result (ok . t)))) error font-lock-function-name-face) ("12:34:56.789 ← response     unknown" ((:direction . incoming) (:kind . response) (:object (id . 1) (result))) success font-lock-function-name-face) ("12:34:56.789 ← response     error" ((:direction . incoming) (:kind . response) (:object (error (code . -1)))) success font-lock-function-name-face) ("12:34:56.789 → notification unknown" ((:direction . outgoing) (:kind . notification) (:object (other . t))) error font-lock-function-name-face))"#
    ]];
    assert_acp_traffic_parity(elisp_form, expect);
}

#[test]
fn acp_traffic_log_trims_one_hundred_whole_lines_after_crossing_one_thousand() {
    let elisp_form = r##"(let ((buffer
                (generate-new-buffer
                 " *acp-traffic-trim-fixture*")))
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'format-time-string)
                   (lambda (&rest arguments)
                     arguments
                     "00:00:00.000")))
               (dotimes
                   (index 1001)
                 (acp-traffic-log-traffic
                  :buffer buffer
                  :direction 'incoming
                  :kind 'notification
                  :message
                  `((:object
                     (method
                      .
                      ,(format
                        "m-%04d"
                        index))))))
               (with-current-buffer buffer
                 (list
                  (count-lines
                   (point-min)
                   (point-max))
                  (progn
                    (goto-char
                     (point-min))
                    (buffer-substring-no-properties
                     (line-beginning-position)
                     (line-end-position)))
                  (progn
                    (goto-char
                     (point-max))
                    (forward-line -1)
                    (buffer-substring-no-properties
                     (line-beginning-position)
                     (line-end-position)))
                  (map-elt
                   (get-text-property
                    (point-min)
                    'acp-traffic-object)
                   :object))))
           (when
               (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK (901 "00:00:00.000 ← notification m-0100" "00:00:00.000 ← notification m-1000" ((method . "m-0100")))"#
    ]];
    assert_acp_traffic_parity(elisp_form, expect);
}

#[test]
fn acp_traffic_objects_extracts_entries_in_order_and_rejects_other_modes() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (acp-traffic-mode)
           (let ((inhibit-read-only t))
             (insert
              "one\n")
             (add-text-properties
              1 4
              '(acp-traffic-object
                ((:kind . request))))
             (insert
              "blank\n")
             (insert
              "three\n")
             (add-text-properties
              11 16
              '(acp-traffic-object
                ((:kind . response)))))
           (acp-traffic--objects))
         (with-temp-buffer
           (condition-case error
               (acp-traffic--objects)
             (error
              (list
               (car error)
               (cadr error))))))"##;
    let expect = expect![[
        r#"OK ((((:kind . request)) ((:kind . response))) (user-error "Not in a traffic buffer"))"#
    ]];
    assert_acp_traffic_parity(elisp_form, expect);
}

#[test]
fn acp_traffic_display_value_and_max_key_width_cover_all_supported_value_types() {
    let elisp_form = r##"(list
         (mapcar
          #'acp-traffic-display-format-value
          (list
           "text"
           ""
           42
           -1.5
           :false
           :null
           t
           'symbol
           nil
           '(nested)
           [vector]))
         (acp-traffic-display-max-key-width
          '((short . 1)
            (much-longer . 2)
            (mid . 3)))
         (acp-traffic-display-max-key-width
          nil))"##;
    let expect =
        expect![[r#"OK (("text" "" "42" "-1.5" ":false" ":null" "t" "symbol" "nil" "" "") 11 0)"#]];
    assert_acp_traffic_parity(elisp_form, expect);
}

#[test]
fn acp_traffic_display_helper_formats_nested_alists_lists_vectors_and_faces_exactly() {
    let elisp_form = r##"(with-temp-buffer
         (acp-traffic-display-objects-helper
          '((name . "agent")
            (count . 2)
            (enabled . t)
            (missing . :null)
            (nested
             (short . "x")
             (longer . :false))
            (items
             .
             [((id . 1))
              ((id . 2))]))
          0)
         (list
          (buffer-string)
          (let (runs
                (position
                 (point-min)))
            (while
                (< position
                   (point-max))
              (let ((next
                     (next-single-property-change
                      position
                      'face
                      nil
                      (point-max))))
                (when
                    (get-text-property
                     position
                     'face)
                  (push
                   (list
                    position
                    next
                    (buffer-substring-no-properties
                     position
                     next)
                    (get-text-property
                     position
                     'face))
                   runs))
                (setq position next)))
            (nreverse runs))))"##;
    let expect = expect![[
        r#"OK (#("name    agent\ncount   2\nenabled t\nmissing :null\nnested\n        short  x\n        longer :false\nitems\n        id 1\n\n        id 2\n" 0 4 (face font-lock-variable-name-face) 14 19 (face font-lock-variable-name-face) 24 31 (face font-lock-variable-name-face) 34 41 (face font-lock-variable-name-face) 48 54 (face font-lock-variable-name-face) 63 68 (face font-lock-variable-name-face) 80 86 (face font-lock-variable-name-face) 94 99 (face font-lock-variable-name-face) 108 110 (face font-lock-variable-name-face) 122 124 (face font-lock-variable-name-face)) ((1 5 "name" font-lock-variable-name-face) (15 20 "count" font-lock-variable-name-face) (25 32 "enabled" font-lock-variable-name-face) (35 42 "missing" font-lock-variable-name-face) (49 55 "nested" font-lock-variable-name-face) (64 69 "short" font-lock-variable-name-face) (81 87 "longer" font-lock-variable-name-face) (95 100 "items" font-lock-variable-name-face) (109 111 "id" font-lock-variable-name-face) (123 125 "id" font-lock-variable-name-face)))"#
    ]];
    assert_acp_traffic_parity(elisp_form, expect);
}

#[test]
fn acp_traffic_display_objects_builds_entry_buffer_links_source_and_requests_display() {
    let elisp_form = r##"(let ((source
                (generate-new-buffer
                 " *acp-traffic-source*"))
               displayed)
         (unwind-protect
             (with-current-buffer source
               (cl-letf
                   (((symbol-function
                      'display-buffer)
                     (lambda (buffer)
                       (setq displayed
                             (buffer-name buffer))
                       buffer)))
                 (acp-traffic-display-objects
                  '(((:kind . request)
                     (:object
                      (method . "one")))
                    ((:kind . response)
                     (:object
                      (result . 2)))))
                 (with-current-buffer
                     "*ACP traffic entry*"
                   (list
                    displayed
                    major-mode
                    mode-name
                    buffer-read-only
                    (buffer-string)
                    (eq
                     acp-traffic-entry--traffic-buffer
                     source)
                    (point)))))
           (when
               (buffer-live-p source)
             (kill-buffer source))
           (when-let*
               ((buffer
                 (get-buffer
                  "*ACP traffic entry*")))
             (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK ("*ACP traffic entry*" acp-traffic-entry-mode "ACP-traffic-entry" t #(":kind   request\n:object\n        method one\n\n:kind   response\n:object\n        result 2\n\n" 0 5 (face font-lock-variable-name-face) 16 23 (face font-lock-variable-name-face) 32 38 (face font-lock-variable-name-face) 44 49 (face font-lock-variable-name-face) 61 68 (face font-lock-variable-name-face) 77 83 (face font-lock-variable-name-face)) t 1)"#
    ]];
    assert_acp_traffic_parity(elisp_form, expect);
}

#[test]
fn acp_traffic_navigation_and_display_entry_forward_positions_and_nil_properties() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "one\ntwo\nthree\n")
         (add-text-properties
          5 8
          '(acp-traffic-object
            ((:kind . request))))
         (goto-char 1)
         (let (calls)
           (cl-letf
               (((symbol-function
                  'acp-traffic--update-line-highlight)
                 (lambda ()
                   (push
                    (list
                     'highlight
                     (point))
                    calls)))
                ((symbol-function
                  'acp-traffic-display-objects)
                 (lambda (objects)
                   (push
                    (list
                     'display
                     objects
                     (point))
                    calls)
                   'displayed)))
             (list
              (acp-traffic-next-entry)
              (point)
              (acp-traffic-previous-entry)
              (point)
              (progn
                (goto-char 5)
                (acp-traffic-display-entry))
              (progn
                (goto-char 10)
                (condition-case error
                    (acp-traffic-display-entry)
                  (error
                   (list
                    (car error)
                    (cadr error)))))
              (nreverse calls)))))"##;
    let expect = expect![
        "OK (displayed 5 displayed 1 displayed displayed ((highlight 5) (display (#1=((:kind . request))) 5) (highlight 1) (display (nil) 1) (display (#1#) 5) (display (nil) 10)))"
    ];
    assert_acp_traffic_parity(elisp_form, expect);
}

#[test]
fn acp_traffic_display_all_preserves_point_and_save_rejects_mode_and_destination_errors() {
    let elisp_form = r##"(cl-labels
         ((capture
           (function)
           (condition-case error
               (funcall function)
             (error
              (list
               (car error)
               (cadr error))))))
         (list
          (with-temp-buffer
            (capture
             #'acp-traffic-display-all-entries))
          (with-temp-buffer
            (acp-traffic-mode)
            (let ((inhibit-read-only t))
              (insert
               "one\n")
              (add-text-properties
               1 4
               '(acp-traffic-object
                 ((:kind . request))))
              (insert
               "empty\n")
              (insert
               "three\n")
              (add-text-properties
               11 16
               '(acp-traffic-object
                 ((:kind . response)))))
            (goto-char 7)
            (let ((original-point
                   (point))
                  displayed)
              (cl-letf
                  (((symbol-function
                     'acp-traffic-display-objects)
                    (lambda (objects)
                      (setq displayed
                            objects)
                      'displayed)))
                (list
                 (acp-traffic-display-all-entries)
                 (= original-point
                    (point))
                 displayed))))
          (with-temp-buffer
            (capture
             #'acp-traffic-save-to))
          (with-temp-buffer
            (acp-traffic-mode)
            (cl-letf
                (((symbol-function
                   'read-file-name)
                  (lambda (&rest arguments)
                    arguments
                    "   ")))
              (capture
               #'acp-traffic-save-to)))))"##;
    let expect = expect![[
        r#"OK ((user-error "Not in a traffic buffer") (displayed t (((:kind . request)) ((:kind . response)))) (user-error "Not in a traffic buffer") (user-error "No destination file found"))"#
    ]];
    assert_acp_traffic_parity(elisp_form, expect);
}

#[test]
fn acp_traffic_entry_navigation_uses_visible_source_window_or_current_context() {
    let elisp_form = r##"(let ((traffic
                (generate-new-buffer
                 " *acp-entry-source*"))
               calls
               selected)
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'get-buffer-window)
                   (lambda (buffer)
                     buffer
                     selected))
                  ((symbol-function
                    'acp-traffic-next-entry)
                   (lambda ()
                     (push
                      (list
                       'next
                       (buffer-name))
                      calls)))
                  ((symbol-function
                    'acp-traffic-previous-entry)
                   (lambda ()
                     (push
                      (list
                       'previous
                       (buffer-name))
                      calls)))
                  ((symbol-function
                    'acp-traffic--update-line-highlight)
                   (lambda ()
                     (push
                      (list
                       'highlight
                       (buffer-name))
                      calls))))
               (with-temp-buffer
                 (setq acp-traffic-entry--traffic-buffer
                       traffic)
                 (setq selected
                       (selected-window))
                 (acp-traffic-entry-next)
                 (setq selected nil)
                 (acp-traffic-entry-previous)
                 (setq acp-traffic-entry--traffic-buffer
                       nil)
                 (acp-traffic-entry-next)
                 (list
                  (nreverse calls))))
           (when
               (buffer-live-p traffic)
             (kill-buffer traffic))))"##;
    let expect = expect![[
        r#"OK (((next "*scratch*") (highlight "*scratch*") (previous " *temp*") (highlight " *temp*")))"#
    ]];
    assert_acp_traffic_parity(elisp_form, expect);
}

#[test]
fn acp_traffic_save_and_read_file_round_trip_exact_objects_in_sandbox() {
    let elisp_form = r##"(let ((path
                (expand-file-name
                 "acp-traffic-round-trip.traffic"
                 (getenv
                  "TMPDIR")))
               message-text)
         (unwind-protect
             (with-temp-buffer
               (acp-traffic-mode)
               (let ((inhibit-read-only t))
                 (insert
                  "one\n")
                 (add-text-properties
                  1 4
                  '(acp-traffic-object
                    ((:direction . outgoing)
                     (:kind . request)
                     (:object
                      (method . "initialize")))))
                 (insert
                  "two\n")
                 (add-text-properties
                  5 8
                  '(acp-traffic-object
                    ((:direction . incoming)
                     (:kind . response)
                     (:object
                      (id . 1)
                      (result . nil))))))
               (cl-letf
                   (((symbol-function
                      'read-file-name)
                     (lambda (&rest arguments)
                       arguments
                       path))
                    ((symbol-function
                      'message)
                     (lambda (format-string &rest arguments)
                       (setq message-text
                             (apply
                              #'format
                              format-string
                              arguments)))))
                 (acp-traffic-save-to))
               (list
                (acp-traffic-read-file
                 path)
                message-text
                (with-temp-buffer
                  (insert-file-contents
                   path)
                  (and
                   (>
                    (buffer-size)
                    0)
                   t))))
           (when
               (file-exists-p path)
             (delete-file path))))"##;
    let expect = expect![[
        r#"OK ((((:direction . outgoing) (:kind . request) (:object (method . "initialize"))) ((:direction . incoming) (:kind . response) (:object (id . 1) (result)))) "Saved [ORACLE-TMPDIR]/acp-traffic-round-trip.traffic" t)"#
    ]];
    assert_acp_traffic_parity(elisp_form, expect);
}

#[test]
fn acp_traffic_open_file_replays_messages_and_surfaces_cancelled_selection_error() {
    let elisp_form = r##"(let (logged
               popped
               selection)
         (cl-letf
             (((symbol-function
                'read-file-name)
               (lambda (&rest arguments)
                 arguments
                 selection))
              ((symbol-function
                'acp-traffic-read-file)
               (lambda (path)
                 path
                 '(((:direction . incoming)
                    (:kind . notification)
                    (:object
                     (method . "update"))))))
              ((symbol-function
                'acp-traffic-log-traffic)
               (lambda (&rest arguments)
                 (push arguments logged)))
              ((symbol-function
                'pop-to-buffer)
               (lambda (buffer)
                 (setq popped
                       (buffer-name buffer))
                 buffer)))
           (unwind-protect
               (progn
                 (setq selection
                       "/fixture/session.traffic")
                 (let ((success
                        (acp-traffic-open-file)))
                   (setq selection nil)
                   (list
                    (buffer-name success)
                    popped
                    (mapcar
                     (lambda (arguments)
                       (list
                        (buffer-name
                         (plist-get
                          arguments
                          :buffer))
                        (plist-get
                         arguments
                         :direction)
                        (plist-get
                         arguments
                         :kind)
                        (plist-get
                         arguments
                         :message)))
                     (nreverse logged))
                    (condition-case error
                        (acp-traffic-open-file)
                      (error
                       (list
                        (car error)
                        (cadr error)))))))
             (when-let*
                 ((buffer
                   (get-buffer
                    "*ACP traffic (session.traffic)*")))
               (kill-buffer buffer)))))"##;
    let expect = expect![[
        r#"OK ("*ACP traffic (session.traffic)*" "*ACP traffic (session.traffic)*" (("*ACP traffic (session.traffic)*" incoming notification ((:direction . incoming) (:kind . notification) (:object (method . "update"))))) (error "No session messages found"))"#
    ]];
    assert_acp_traffic_parity(elisp_form, expect);
}

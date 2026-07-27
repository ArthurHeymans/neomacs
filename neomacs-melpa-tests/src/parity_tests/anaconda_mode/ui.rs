use expect_test::expect;

use super::assert_anaconda_mode_parity;

#[test]
fn completion_command_avoids_comments_and_dispatches_real_code_to_the_complete_rpc() {
    let elisp_form = r##"(let (events
      in-comment)
  (cl-letf (((symbol-function 'python-syntax-comment-or-string-p)
             (lambda () in-comment))
            ((symbol-function 'anaconda-mode-call)
             (lambda (command callback)
               (push (list command callback) events))))
    (setq in-comment t)
    (let ((comment-result (anaconda-mode-complete)))
      (setq in-comment nil)
      (let ((code-result (anaconda-mode-complete)))
        (list comment-result code-result (nreverse events))))))"##;
    let expect = expect![[r#"OK (nil #1=(("complete" anaconda-mode-complete-callback)) #1#)"#]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn completion_candidates_keep_server_types_as_annotations_without_changing_names() {
    let elisp_form = r##"(let* ((result
        [["fetch" "function"]
         ["FeatureFlag" "class"]
         ["field_name" "statement"]])
       (candidates (anaconda-mode-complete-extract-names result)))
  (mapcar
   (lambda (candidate)
     (list
      (substring-no-properties candidate)
      (get-text-property 0 'type candidate)
      (anaconda-mode-complete-annotation candidate)
      (text-properties-at 0 candidate)))
   candidates))"##;
    let expect = expect![[
        r#"OK (("fetch" "function" " <function>" (type "function")) ("FeatureFlag" "class" " <class>" (type "class")) ("field_name" "statement" " <statement>" (type "statement")))"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn completion_callback_replaces_the_symbol_at_point_with_typed_candidates() {
    let elisp_form = r##"(with-temp-buffer
  (insert "client.fe")
  (goto-char (point-max))
  (let (invocation)
    (cl-letf (((symbol-function 'completion-in-region)
               (lambda (start stop collection &rest arguments)
                 (setq invocation
                       (list
                        start
                        stop
                        (buffer-substring start stop)
                        (mapcar
                         (lambda (candidate)
                           (list
                            (substring-no-properties candidate)
                            (get-text-property 0 'type candidate)
                            (anaconda-mode-complete-annotation candidate)))
                         collection)
                        completion-extra-properties
                        arguments))
                 'completed)))
      (list
       (anaconda-mode-complete-callback
        [["fetch" "function"]
         ["features" "instance"]])
       invocation))))"##;
    let expect = expect![[
        r#"OK (completed (8 10 "fe" (("fetch" "function" " <function>") ("features" "instance" " <instance>")) (:annotation-function anaconda-mode-complete-annotation) nil))"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn documentation_buffer_renders_multiple_signatures_trims_trailing_space_and_becomes_read_only() {
    let elisp_form = r##"(let ((buffer
       (anaconda-mode-documentation-view
        [["Client.fetch(id: int)" "Fetch one record.\n\n  "]
         ["Client.fetch_all()" "Return every record.\n   "]])))
  (unwind-protect
      (with-current-buffer buffer
        (list
         (buffer-name)
         (buffer-string)
         (point)
         view-mode
         buffer-read-only
         (get-text-property (point-min) 'face)
         (get-text-property
          (save-excursion
            (goto-char (point-min))
            (search-forward "Client.fetch_all")
            (match-beginning 0))
          'face)))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK ("*Anaconda*" #("Client.fetch(id: int)\nFetch one record.\n\nClient.fetch_all()\nReturn every record.\n\n" 0 21 (face bold) 41 59 (face bold)) 1 t t bold bold)"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn show_doc_callback_selects_message_buffer_or_posframe_for_real_result_shapes() {
    let elisp_form = r##"(let (events)
  (cl-letf (((symbol-function 'message)
             (lambda (format-string &rest arguments)
               (push (list 'message
                           (apply #'format format-string arguments))
                     events)))
            ((symbol-function 'require)
             (lambda (_feature &optional _filename _noerror) t))
            ((symbol-function 'posframe-workable-p) (lambda () t))
            ((symbol-function 'anaconda-mode-documentation-view)
             (lambda (result)
               (push (list 'buffer-view result) events)
               'documentation-buffer))
            ((symbol-function 'anaconda-mode-documentation-posframe-view)
             (lambda (result) (push (list 'posframe-view result) events)))
            ((symbol-function 'pop-to-buffer)
             (lambda (buffer norecord)
               (push (list 'pop buffer norecord) events))))
    (let ((anaconda-mode-use-posframe-show-doc nil))
      (anaconda-mode-show-doc-callback []))
    (let ((anaconda-mode-use-posframe-show-doc nil))
      (anaconda-mode-show-doc-callback [["first" "buffer docs"]]))
    (let ((anaconda-mode-use-posframe-show-doc t))
      (anaconda-mode-show-doc-callback [["second" "frame docs"]]))
    (nreverse events)))"##;
    let expect = expect![[
        r#"OK ((message "No documentation available") (buffer-view [["first" "buffer docs"]]) (pop documentation-buffer t) (posframe-view [["second" "frame docs"]]))"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn posframe_documentation_preserves_leading_trim_border_colors_position_and_hide_hook_state() {
    let elisp_form = r##"(with-temp-buffer
  (insert "service.lookup")
  (goto-char 8)
  (let ((anaconda-mode-doc-frame-name " *test-anaconda-posframe*")
        (anaconda-mode-doc-frame-background "#101820")
        (anaconda-mode-doc-frame-foreground "#f2aa4c")
        (post-command-hook nil)
        show-arguments)
    (unwind-protect
        (cl-letf (((symbol-function 'posframe-show)
                   (lambda (&rest arguments)
                     (setq show-arguments arguments)))
                  ((symbol-function 'window-start) (lambda (&optional _window) 17)))
          (anaconda-mode-documentation-posframe-view
           [["lookup(key)" "\n   Return a cached value.\n"]
            ["lookup_many(keys)" "  Return many values.\n"]])
          (list
           (with-current-buffer anaconda-mode-doc-frame-name
             (list (buffer-string)
                   (get-text-property (point-min) 'face)))
           show-arguments
           post-command-hook
           anaconda-mode-frame-last-point
           anaconda-mode-frame-last-scroll-offset))
      (when (get-buffer anaconda-mode-doc-frame-name)
        (kill-buffer anaconda-mode-doc-frame-name)))))"##;
    let expect = expect![[
        r##"OK ((#("lookup(key)\nReturn a cached value.\n\n\nlookup_many(keys)\nReturn many values.\n\n\n" 0 11 (face bold) 37 54 (face bold)) bold) (" *test-anaconda-posframe*" :position 8 :internal-border-width 10 :background-color "#101820" :foreground-color "#f2aa4c") (anaconda-mode-hide-frame) 8 17)"##
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn hide_frame_keeps_stationary_docs_but_hides_and_unhooks_after_cursor_or_scroll_changes() {
    let elisp_form = r##"(let ((anaconda-mode-doc-frame-name "*test-frame*")
      (anaconda-mode-frame-last-point 12)
      (anaconda-mode-frame-last-scroll-offset 30)
      (post-command-hook '(anaconda-mode-hide-frame other-hook))
      (current-point 12)
      (current-scroll 30)
      events)
  (cl-letf (((symbol-function 'get-buffer) (lambda (_name) 'frame-buffer))
            ((symbol-function 'point) (lambda () current-point))
            ((symbol-function 'window-start)
             (lambda (&optional _window) current-scroll))
            ((symbol-function 'posframe-hide)
             (lambda (name) (push (list 'hide name) events))))
    (anaconda-mode-hide-frame)
    (let ((stationary (list (copy-tree post-command-hook)
                            (nreverse events))))
      (setq current-point 13
            events nil)
      (anaconda-mode-hide-frame)
      (list stationary
            (copy-tree post-command-hook)
            (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (((anaconda-mode-hide-frame other-hook) nil) (other-hook) ((hide "*test-frame*")))"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn server_locations_become_xrefs_with_readable_paths_precise_coordinates_and_summaries() {
    let elisp_form = r##"(cl-letf (((symbol-function 'pythonic-emacs-readable-file-name)
           (lambda (path) (concat "/mounted" path))))
  (mapcar
   (lambda (xref)
     (let ((location (xref-item-location xref)))
       (list
        (xref-item-summary xref)
        (xref-file-location-file location)
        (xref-file-location-line location)
        (xref-file-location-column location))))
   (anaconda-mode-make-xrefs
    [["/srv/app/models.py" 18 7 "User.save"]
     ["/srv/app/services.py" 42 3 "persist_user"]])))"##;
    let expect = expect![[
        r#"OK (("User.save" "/mounted/srv/app/models.py" 18 7) ("persist_user" "/mounted/srv/app/services.py" 42 3))"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn xref_display_handles_empty_error_string_single_jump_and_multi_result_picker_paths() {
    let elisp_form = r##"(let (events)
  (cl-letf (((symbol-function 'message)
             (lambda (format-string &rest arguments)
               (push (list 'message
                           (apply #'format format-string arguments))
                     events)))
            ((symbol-function 'anaconda-mode-make-xrefs)
             (lambda (result)
               (mapcar (lambda (item) (intern (format "xref-%s" item)))
                       (append result nil))))
            ((symbol-function 'xref-push-marker-stack)
             (lambda () (push '(push-marker) events)))
            ((symbol-function 'xref-pop-to-location)
             (lambda (xref action)
               (push (list 'pop xref action) events)))
            ((symbol-function 'xref--pop-to-location)
             (lambda (xref action)
               (push (list 'legacy-pop xref action) events)))
            ((symbol-function 'xref--show-xrefs)
             (lambda (source action)
               (push (list 'show
                           (if (functionp source)
                               (funcall source)
                             source)
                           action)
                     events))))
    (anaconda-mode-show-xrefs nil nil "Nothing found")
    (anaconda-mode-show-xrefs "Server unavailable" 'window "unused")
    (anaconda-mode-show-xrefs [one] 'frame "unused")
    (anaconda-mode-show-xrefs [one two] 'window "unused")
    (nreverse events)))"##;
    let expect = expect![[
        r#"OK ((message "Nothing found") (message "Server unavailable") (push-marker) (pop xref-one frame) (show (xref-one xref-two) window))"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn eldoc_formats_overloads_highlights_active_arguments_and_truncates_single_line_to_frame_width() {
    let elisp_form = r##"(let ((result
       [["fetch" 1 ["self" "record_id" "timeout"]]
        ["fetch_many" 0 ["records" "limit"]]]))
  (cl-labels
      ((describe
        (text)
        (list
         (substring-no-properties text)
         (mapcar
          (lambda (index)
            (list index (get-text-property index 'face text)))
          (number-sequence 0 (1- (length text)))))))
    (let ((anaconda-mode-eldoc-as-single-line nil)
          multiline
          singleline)
      (setq multiline (anaconda-mode-eldoc-format result))
      (let ((anaconda-mode-eldoc-as-single-line t))
        (cl-letf (((symbol-function 'frame-width) (lambda (&optional _frame) 24)))
          (setq singleline (anaconda-mode-eldoc-format result))))
      (list (describe multiline)
            (describe singleline)))))"##;
    let expect = expect![[
        r#"OK (("fetch(self, record_id, timeout)\nfetch_many(records, limit)" ((0 font-lock-function-name-face) (1 font-lock-function-name-face) (2 font-lock-function-name-face) (3 font-lock-function-name-face) (4 font-lock-function-name-face) (5 nil) (6 nil) (7 nil) (8 nil) (9 nil) (10 nil) (11 nil) (12 eldoc-highlight-function-argument) (13 eldoc-highlight-function-argument) (14 eldoc-highlight-function-argument) (15 eldoc-highlight-function-argument) (16 eldoc-highlight-function-argument) (17 eldoc-highlight-function-argument) (18 eldoc-highlight-function-argument) (19 eldoc-highlight-function-argument) (20 eldoc-highlight-function-argument) (21 nil) (22 nil) (23 nil) (24 nil) (25 nil) (26 nil) (27 nil) (28 nil) (29 nil) (30 nil) (31 nil) (32 font-lock-function-name-face) (33 font-lock-function-name-face) (34 font-lock-function-name-face) (35 font-lock-function-name-face) (36 font-lock-function-name-face) (37 font-lock-function-name-face) (38 font-lock-function-name-face) (39 font-lock-function-name-face) (40 font-lock-function-name-face) (41 font-lock-function-name-face) (42 nil) (43 eldoc-highlight-function-argument) (44 eldoc-highlight-function-argument) (45 eldoc-highlight-function-argument) (46 eldoc-highlight-function-argument) (47 eldoc-highlight-function-argument) (48 eldoc-highlight-function-argument) (49 eldoc-highlight-function-argument) (50 nil) (51 nil) (52 nil) (53 nil) (54 nil) (55 nil) (56 nil) (57 nil))) ("fetch(self, record_id, t" ((0 font-lock-function-name-face) (1 font-lock-function-name-face) (2 font-lock-function-name-face) (3 font-lock-function-name-face) (4 font-lock-function-name-face) (5 nil) (6 nil) (7 nil) (8 nil) (9 nil) (10 nil) (11 nil) (12 eldoc-highlight-function-argument) (13 eldoc-highlight-function-argument) (14 eldoc-highlight-function-argument) (15 eldoc-highlight-function-argument) (16 eldoc-highlight-function-argument) (17 eldoc-highlight-function-argument) (18 eldoc-highlight-function-argument) (19 eldoc-highlight-function-argument) (20 eldoc-highlight-function-argument) (21 nil) (22 nil) (23 nil))))"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn eldoc_function_dispatches_rpc_formats_the_reply_and_delivers_it_to_modern_eldoc_callback() {
    let elisp_form = r##"(let (events)
  (cl-letf (((symbol-function 'anaconda-mode-call)
             (lambda (command callback)
               (push (list 'request command) events)
               (funcall callback
                        [["build" 2 ["self" "target" "release"]]]))))
    (let ((return-value
           (anaconda-mode-eldoc-function
            (lambda (documentation)
              (push (list
                     'callback
                     (substring-no-properties documentation)
                     (get-text-property 0 'face documentation)
                     (get-text-property 20 'face documentation))
                    events))
            :ignored "context")))
      (list return-value (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (nil ((request "eldoc") (callback "build(self, target, release)" font-lock-function-name-face eldoc-highlight-function-argument)))"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

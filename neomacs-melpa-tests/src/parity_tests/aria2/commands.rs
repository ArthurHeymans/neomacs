use expect_test::expect;

use super::assert_aria2_parity;

#[test]
fn aria2_supported_file_predicate_accepts_real_directories_exact_lowercase_extensions_and_rejects_lookalikes()
 {
    let elisp_form = r##"(let ((directory
                (aria2-test-path
                 "chooser-directory"))
               (names
                '("release.torrent"
                  "release.meta4"
                  "release.metalink"
                  "release.TORRENT"
                  "release.Meta4"
                  "release.torrent.bak"
                  ".torrent"
                  "torrent"
                  "release.xml"
                  ""
                  "日本語.metalink")))
         (make-directory
          directory
          t)
         (unwind-protect
             (append
              (list
               (list
                :directory
                (aria2--supported-file-type-p
                 directory)))
              (mapcar
               (lambda (name)
                 (list
                  name
                  (aria2--supported-file-type-p
                   name)))
               names))
           (delete-directory
            directory)))"##;
    let expect = expect![[
        r#"OK ((:directory t) ("release.torrent" 7) ("release.meta4" 7) ("release.metalink" 7) ("release.TORRENT" 7) ("release.Meta4" 7) ("release.torrent.bak" nil) (".torrent" 0) ("torrent" nil) ("release.xml" nil) ("" t) ("日本語.metalink" 3))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_add_file_routes_real_torrent_and_metalink_files_reports_missing_selection_and_reverts() {
    let elisp_form = r##"(let* ((controller
                  (aria2-test-controller))
                 (aria2--cc
                  controller)
                 (torrent
                  (aria2-test-path
                   "selected.torrent"))
                 (metalink
                  (aria2-test-path
                   "selected.meta4"))
                 (choices
                  (list
                   torrent
                   metalink
                   ""
                   (aria2-test-path
                    "missing.torrent")))
                 calls)
         (with-temp-file
             torrent
           (insert
            "torrent"))
         (with-temp-file
             metalink
           (insert
            "metalink"))
         (unwind-protect
             (with-temp-buffer
               (cl-letf
                   (((symbol-function
                      'read-file-name)
                     (lambda (&rest arguments)
                       (push
                        (cons :read arguments)
                        calls)
                       (prog1
                           (car choices)
                         (setq choices
                               (cdr choices)))))
                    ((symbol-function
                      'addTorrent)
                     (lambda (this path &rest arguments)
                       (push
                        (list
                         :torrent
                         (eq this controller)
                         (file-name-nondirectory
                          path)
                         arguments)
                        calls)
                       :torrent-added))
                    ((symbol-function
                      'addMetalink)
                     (lambda (this path &rest arguments)
                       (push
                        (list
                         :metalink
                         (eq this controller)
                         (if
                             (file-directory-p
                              path)
                             :directory
                           (file-name-nondirectory
                            path))
                         arguments)
                        calls)
                       :metalink-added))
                    ((symbol-function
                      'message)
                     (lambda (format-string &rest arguments)
                       (push
                        (list
                         :message
                         (apply
                          #'format
                          format-string
                          arguments))
                        calls)))
                    ((symbol-function
                      'revert-buffer)
                     (lambda (&rest arguments)
                       (push
                        (cons :revert arguments)
                        calls)
                       :reverted)))
                 (list
                  (aria2-add-file nil)
                  (aria2-add-file '(4))
                  (aria2-add-file nil)
                  (aria2-add-file nil)
                  (nreverse calls))))
           (delete-file torrent)
           (delete-file metalink)))"##;
    let expect = expect![[
        r#"OK (:reverted :reverted :reverted :reverted ((:read "Choose .meta4, .metalink or .torrent file: " "[ORACLE-SANDBOX]/" nil nil nil aria2--supported-file-type-p) (:torrent t "selected.torrent" nil) (:revert) (:read "Choose .meta4, .metalink or .torrent file: " "[ORACLE-SANDBOX]/" nil nil nil aria2--supported-file-type-p) (:metalink t "selected.meta4" nil) (:revert) (:read "Choose .meta4, .metalink or .torrent file: " "[ORACLE-SANDBOX]/" nil nil nil aria2--supported-file-type-p) (:metalink t :directory nil) (:revert) (:read "Choose .meta4, .metalink or .torrent file: " "[ORACLE-SANDBOX]/" nil nil nil aria2--supported-file-type-p) (:message "No file selected.") (:revert)))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_pause_resume_and_toggle_use_real_row_id_property_status_cell_messages_and_refresh() {
    let elisp_form = r##"(let ((controller
                (aria2-test-controller))
               (aria2--cc
                nil)
               (entry
                ["name"
                 ("active"
                  face
                  aria2-status-face)])
               calls)
         (setq aria2--cc controller)
         (with-temp-buffer
           (insert
            (propertize
             "row"
             'tabulated-list-id
             "gid-real-row"))
           (goto-char
            (point-min))
           (cl-letf
               (((symbol-function
                  'pause)
                 (lambda (this gid &optional force)
                   (push
                    (list
                     :pause
                     (eq this controller)
                     gid
                     force)
                    calls)
                   :paused))
                ((symbol-function
                  'unpause)
                 (lambda (this gid)
                   (push
                    (list
                     :unpause
                     (eq this controller)
                     gid)
                    calls)
                   :resumed))
                ((symbol-function
                  'message)
                 (lambda (format-string &rest arguments)
                   (push
                    (list
                     :message
                     (apply
                      #'format
                      format-string
                      arguments))
                    calls)))
                ((symbol-function
                  'revert-buffer)
                 (lambda (&rest arguments)
                   (push
                    (cons :revert arguments)
                    calls)
                   :reverted))
                ((symbol-function
                  'tabulated-list-get-entry)
                 (lambda ()
                   entry)))
             (let ((active-p
                    (aria2--is-paused-p))
                   (pause-result
                    (aria2-pause)))
               (setq entry
                     ["name"
                      ("paused"
                       face
                       aria2-status-face)])
               (let ((paused-p
                      (aria2--is-paused-p))
                     (resume-result
                      (aria2-resume))
                     (toggle-resume
                      (aria2-toggle-pause)))
                 (setq entry
                       ["name"
                        ("active"
                         face
                         aria2-status-face)])
                 (list
                  active-p
                  paused-p
                  pause-result
                  resume-result
                  toggle-resume
                  (aria2-toggle-pause)
                  (nreverse calls)
                  (get-text-property
                   (point)
                   'tabulated-list-id)))))))"##;
    let expect = expect![[
        r#"OK (nil t #2=((:message "Pausing download. This may take a moment...") (:unpause t "gid-real-row") (:revert) (:unpause t "gid-real-row") (:revert) (:pause t "gid-real-row" nil) . #1=((:message "Pausing download. This may take a moment..."))) :reverted :reverted #1# ((:pause t "gid-real-row" nil) . #2#) "gid-real-row")"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_add_uris_builds_real_editable_widget_form_with_validation_header_buttons_and_keybindings()
{
    let elisp_form = r##"(save-window-excursion
         (aria2-test-kill-buffers)
         (unwind-protect
             (progn
               (aria2-add-uris)
               (let ((buffer
                      (current-buffer)))
                 (list
                  (buffer-name buffer)
                  major-mode
                  mode-name
                  header-line-format
                  (buffer-substring-no-properties
                   (point-min)
                   (point-max))
                  (widget-value
                   aria2--url-list-widget)
                  (widget-get
                   aria2--url-list-widget
                   :entry-format)
                  (lookup-key
                   aria2-dialog-mode-map
                   (kbd
                    "C-c C-c"))
                  (lookup-key
                   aria2-dialog-mode-map
                   (kbd
                    "C-c C-k"))
                  (lookup-key
                   aria2-dialog-mode-map
                   [mouse-1])
                  (point)
                  (count-lines
                   (point-min)
                   (point-max)))))
           (aria2-test-kill-buffers)))"##;
    let expect = expect![[
        r#"OK ("*aria2: Add http/https/ftp/magnet url(s)*" aria2-dialog-mode "Add urls" #("Add urls, then download with ‘C-c C-c’, or cancel with ‘C-c C-k’" 30 37 (font-lock-face help-key-binding face help-key-binding) 56 63 (font-lock-face help-key-binding face help-key-binding)) "Please input urls to download.\n\nNon \"magnet:\" urls must be mirrors pointing to the same file.\n\n[INS] [DEL] \n[INS]\n\n\n[Cancel]  [Download]\n" ("") "%i %d %v" aria2-dialog-submit aria2-dialog-cancel widget-button-click 108 9)"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_dialog_submit_reads_real_widget_values_sends_urls_then_returns_to_list_and_kills_dialog() {
    let elisp_form = r##"(save-window-excursion
         (aria2-test-kill-buffers)
         (let ((controller
                (aria2-test-controller))
               (aria2--cc
                nil)
               calls)
           (setq aria2--cc controller)
           (get-buffer-create
            aria2-list-buffer-name)
           (unwind-protect
               (progn
                 (aria2-add-uris)
                 (widget-value-set
                  aria2--url-list-widget
                  '("https://one.invalid/release.iso"
                    "ftp://two.invalid/release.iso"))
                 (cl-letf
                     (((symbol-function
                        'addUri)
                       (lambda (this urls)
                         (push
                          (list
                           :add-uri
                           (eq this controller)
                           urls)
                          calls)
                         :submitted)))
                   (let ((result
                          (aria2-dialog-submit)))
                     (list
                      result
                      (nreverse calls)
                      aria2--url-list-widget
                      (buffer-name)
                      (get-buffer
                       aria2-url-list-buffer-name)
                      (get-buffer
                       aria2-list-buffer-name)))))
             (aria2-test-kill-buffers))))"##;
    let expect = expect![[
        r#"OK (t ((:add-uri t ("https://one.invalid/release.iso" "ftp://two.invalid/release.iso"))) nil "*aria2: downloads list*" nil (:buffer nil))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_dialog_cancel_clears_widget_switches_to_downloads_and_kills_only_url_buffer() {
    let elisp_form = r##"(save-window-excursion
         (aria2-test-kill-buffers)
         (let ((list-buffer
                (get-buffer-create
                 aria2-list-buffer-name))
               (dialog-buffer
                (get-buffer-create
                 aria2-url-list-buffer-name)))
           (with-current-buffer dialog-buffer
             (setq aria2--url-list-widget
                   'fixture-widget))
           (switch-to-buffer dialog-buffer)
           (unwind-protect
               (list
                (aria2-dialog-cancel)
                aria2--url-list-widget
                (eq
                 (current-buffer)
                 list-buffer)
                (buffer-live-p
                 list-buffer)
                (buffer-live-p
                 dialog-buffer)
                (buffer-name))
             (aria2-test-kill-buffers))))"##;
    let expect = expect![[r#"OK (t nil t t nil "*aria2: downloads list*")"#]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_remove_download_respects_confirmation_prefix_force_and_deletes_only_confirmed_rows() {
    let elisp_form = r##"(let ((controller
                (aria2-test-controller))
               (aria2--cc
                nil)
               (answers
                '(nil t t))
               calls)
         (setq aria2--cc controller)
         (with-temp-buffer
           (insert
            (propertize
             "row"
             'tabulated-list-id
             "gid-remove"))
           (goto-char
            (point-min))
           (cl-letf
               (((symbol-function
                  'y-or-n-p)
                 (lambda (prompt)
                   (push
                    (list :prompt prompt)
                    calls)
                   (prog1
                       (car answers)
                     (setq answers
                           (cdr answers)))))
                ((symbol-function
                  'remove-download)
                 (lambda (this gid &optional force)
                   (push
                    (list
                     :remove
                     (eq this controller)
                     gid
                     force)
                    calls)
                   :removed))
                ((symbol-function
                  'tabulated-list-delete-entry)
                 (lambda (&rest arguments)
                   (push
                    (cons :delete-row arguments)
                    calls)
                   :deleted)))
             (list
              (aria2-remove-download nil)
              (aria2-remove-download nil)
              (aria2-remove-download '(4))
              (nreverse calls)
              (get-text-property
               (point)
               'tabulated-list-id)))))"##;
    let expect = expect![[
        r#"OK (nil :deleted :deleted ((:prompt "Really remove download? ") (:prompt "Really remove download? ") (:remove t "gid-remove" nil) (:delete-row) (:prompt "Really remove download? ") (:remove t "gid-remove" t) (:delete-row)) "gid-remove")"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_clean_removed_download_selects_single_or_bulk_rpc_then_reverts_each_time() {
    let elisp_form = r##"(let ((controller
                (aria2-test-controller))
               (aria2--cc
                nil)
               calls)
         (setq aria2--cc controller)
         (with-temp-buffer
           (insert
            (propertize
             "row"
             'tabulated-list-id
             "gid-finished"))
           (goto-char
            (point-min))
           (cl-letf
               (((symbol-function
                  'removeDownloadResult)
                 (lambda (this gid)
                   (push
                    (list
                     :remove-result
                     (eq this controller)
                     gid)
                    calls)
                   :single))
                ((symbol-function
                  'purgeDownloadResult)
                 (lambda (this)
                   (push
                    (list
                     :purge
                     (eq this controller))
                    calls)
                   :all))
                ((symbol-function
                  'revert-buffer)
                 (lambda (&rest arguments)
                   (push
                    (cons :revert arguments)
                    calls)
                   :reverted)))
             (list
              (aria2-clean-removed-download nil)
              (aria2-clean-removed-download '(4))
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (:reverted :reverted ((:remove-result t "gid-finished") (:revert) (:purge t) (:revert)))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_move_commands_map_plain_and_prefix_arguments_to_relative_start_and_end_positions() {
    let elisp_form = r##"(let ((controller
                (aria2-test-controller))
               (aria2--cc
                nil)
               calls)
         (setq aria2--cc controller)
         (with-temp-buffer
           (insert
            (propertize
             "row"
             'tabulated-list-id
             "gid-position"))
           (goto-char
            (point-min))
           (cl-letf
               (((symbol-function
                  'changePosition)
                 (lambda (this gid position &optional how)
                   (push
                    (list
                     :position
                     (eq this controller)
                     gid
                     position
                     how)
                    calls)
                   position))
                ((symbol-function
                  'revert-buffer)
                 (lambda (&rest arguments)
                   (push
                    (cons :revert arguments)
                    calls)
                   :reverted)))
             (list
              (aria2-move-up-in-list nil)
              (aria2-move-up-in-list '(4))
              (aria2-move-down-in-list nil)
              (aria2-move-down-in-list '(4))
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (:reverted :reverted :reverted :reverted ((:position t "gid-position" -1 "POS_CUR") (:revert) (:position t "gid-position" 0 "POS_SET") (:revert) (:position t "gid-position" 1 "POS_CUR") (:revert) (:position t "gid-position" 0 "POS_END") (:revert)))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_terminate_respects_confirmation_shuts_down_closes_list_and_stops_timers_in_order() {
    let elisp_form = r##"(save-window-excursion
         (aria2-test-kill-buffers)
         (let ((controller
                (aria2-test-controller))
               (aria2--cc
                nil)
               (answers
                '(nil t))
               calls)
           (setq aria2--cc controller)
           (get-buffer-create
            aria2-list-buffer-name)
           (unwind-protect
               (cl-letf
                   (((symbol-function
                      'y-or-n-p)
                     (lambda (prompt)
                       (push
                        (list :prompt prompt)
                        calls)
                       (prog1
                           (car answers)
                         (setq answers
                               (cdr answers)))))
                    ((symbol-function
                      'shutdown)
                     (lambda (this &optional force)
                       (push
                        (list
                         :shutdown
                         (eq this controller)
                         force)
                        calls)
                       :shutdown))
                    ((symbol-function
                      'aria2--stop-timer)
                     (lambda ()
                       (push
                        (list :stop)
                        calls)
                       :stopped)))
                 (list
                  (aria2-terminate)
                  (buffer-live-p
                   (get-buffer
                    aria2-list-buffer-name))
                  (aria2-terminate)
                  (get-buffer
                   aria2-list-buffer-name)
                  (nreverse calls)))
             (aria2-test-kill-buffers))))"##;
    let expect = expect![[
        r#"OK (nil t :stopped nil ((:prompt "Are you sure yo want to terminate aria2 process? ") (:prompt "Are you sure yo want to terminate aria2 process? ") (:shutdown t nil) (:stop)))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_mode_initializes_real_tabulated_buffer_hooks_timer_headers_entries_highlight_and_modeline()
{
    let elisp_form = r##"(save-window-excursion
         (aria2-test-kill-buffers)
         (let ((controller
                (aria2-test-controller))
               (aria2--cc
                nil)
               (aria2-executable
                (executable-find
                 "true"))
               (aria2-start-rpc-server
                nil)
               (aria2-kill-process-on-emacs-exit
                nil)
               (aria2--master-timer
                nil)
               calls)
           (setq aria2--cc controller)
           (unwind-protect
               (with-current-buffer
                   (get-buffer-create
                    aria2-list-buffer-name)
                 (cl-letf
                     (((symbol-function
                        'tellActive)
                       (lambda (&rest _)
                         nil))
                      ((symbol-function
                        'tellWaiting)
                       (lambda (&rest _)
                         nil))
                      ((symbol-function
                        'tellStopped)
                       (lambda (&rest _)
                         nil))
                      ((symbol-function
                        'run-at-time)
                       (lambda (time repeat function &rest arguments)
                         (push
                          (list
                           :timer
                           time
                           repeat
                           function
                           arguments)
                          calls)
                         'master-fixture)))
                   (aria2-mode)
                   (list
                    major-mode
                    mode-name
                    (derived-mode-p
                     'tabulated-list-mode)
                    tabulated-list-format
                    tabulated-list-entries
                    (buffer-string)
                    (bound-and-true-p
                     hl-line-mode)
                    (equal
                     mode-line-format
                     aria2-mode-line-format)
                    aria2--master-timer
                    (memq
                     'aria2--persist-settings-on-exit
                     kill-emacs-hook)
                    (lookup-key
                     aria2-mode-map
                     "p")
                    (lookup-key
                     aria2-mode-map
                     "=")
                    (lookup-key
                     aria2-mode-map
                     "-")
                    (lookup-key
                     aria2-mode-map
                     "u")
                    (nreverse calls))))
             (aria2-test-kill-buffers))))"##;
    let expect = expect![[
        r#"OK (aria2-mode "Aria2" tabulated-list-mode [("File" 40 t) ("Status" 7 t) ("Type" 13 t) ("Done" 4 t) ("Download" 12 t) ("Upload" 12 t) ("Size" 10 nil) ("Error" 0 nil)] aria2--list-entries "" t t master-fixture (aria2--persist-settings-on-exit) aria2-toggle-pause aria2-move-up-in-list aria2-move-down-in-list aria2-add-uris ((:timer t 5 aria2--manage-refresh-timer nil)))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_mode_rejects_missing_executable_before_creating_controller_timers_or_rows() {
    let elisp_form = r##"(with-temp-buffer
         (let ((aria2-executable
                (aria2-test-path
                 "missing-aria2c"))
               (aria2--cc
                nil)
               (aria2--master-timer
                nil)
               calls)
           (cl-letf
               (((symbol-function
                  'run-at-time)
                 (lambda (&rest arguments)
                   (push
                    arguments
                    calls)
                   :unexpected)))
             (list
              (condition-case error-data
                  (list
                   :ok
                   (aria2-mode))
                (error
                 (list
                  :error
                  (car error-data)
                  (cdr error-data)
                  (error-message-string
                   error-data))))
              major-mode
              aria2--cc
              aria2--master-timer
              calls
              (buffer-string)))))"##;
    let expect = expect![[
        r#"OK ((:error aria2-err-no-executable nil "Couldn’t find ‘aria2c’ executable, aborting") aria2-mode nil nil nil "")"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_downloads_list_opens_named_buffer_enables_mode_and_emits_substituted_usage_message() {
    let elisp_form = r##"(save-window-excursion
         (aria2-test-kill-buffers)
         (let (calls)
           (unwind-protect
               (cl-letf
                   (((symbol-function
                      'aria2-mode)
                     (lambda ()
                       (push
                        (list
                         :mode
                         (buffer-name))
                        calls)
                       (setq major-mode
                             'aria2-mode)
                       :enabled))
                    ((symbol-function
                      'message)
                     (lambda (format-string &rest arguments)
                       (push
                        (list
                         :message
                         (apply
                          #'format
                          format-string
                          arguments))
                        calls))))
                 (list
                  (aria2-downloads-list)
                  (buffer-name)
                  major-mode
                  (nreverse calls)
                  (get-buffer
                   aria2-list-buffer-name)))
             (aria2-test-kill-buffers))))"##;
    let expect = expect![[
        r#"OK (#1=((:message #("Type q to quit, Q to kill aria, C-h m for help" 5 6 (font-lock-face help-key-binding face help-key-binding) 16 17 (font-lock-face help-key-binding face help-key-binding) 32 37 (font-lock-face help-key-binding face help-key-binding)))) "*scratch*" lisp-interaction-mode ((:mode "*aria2: downloads list*") . #1#) (:buffer nil))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

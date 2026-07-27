use expect_test::expect;

use super::assert_aria2_parity;

#[test]
fn aria2_refresh_manager_selects_fast_timer_when_download_list_owns_selected_window() {
    let elisp_form = r##"(let ((aria2--refresh-timer
                'old-refresh)
               (aria2--current-buffer-refresh-speed
                :normal)
               calls)
         (cl-letf
             (((symbol-function
                'get-buffer)
               (lambda (_)
                 'list-buffer))
              ((symbol-function
                'selected-window)
               (lambda ()
                 'selected-window))
              ((symbol-function
                'window-buffer)
               (lambda (window)
                 (push
                  (list :window-buffer window)
                  calls)
                 'list-buffer))
              ((symbol-function
                'get-buffer-window)
               (lambda (&rest arguments)
                 (push
                  (cons :get-buffer-window arguments)
                  calls)
                 nil))
              ((symbol-function
                'cancel-timer)
               (lambda (timer)
                 (push
                  (list :cancel timer)
                  calls)))
              ((symbol-function
                'run-at-time)
               (lambda (time repeat function &rest arguments)
                 (push
                  (list
                   :run
                   time
                   repeat
                   function
                   arguments)
                  calls)
                 'new-fast-timer)))
           (list
            (aria2--manage-refresh-timer)
            aria2--refresh-timer
            aria2--current-buffer-refresh-speed
            (nreverse calls))))"##;
    let expect = expect![
        "OK (:fast new-fast-timer :fast ((:window-buffer selected-window) (:cancel old-refresh) (:run t 3 aria2--refresh nil)))"
    ];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_refresh_manager_selects_normal_timer_for_visible_unfocused_download_list() {
    let elisp_form = r##"(let ((aria2--refresh-timer
                nil)
               (aria2--current-buffer-refresh-speed
                :slow)
               calls)
         (cl-letf
             (((symbol-function
                'get-buffer)
               (lambda (_)
                 'list-buffer))
              ((symbol-function
                'selected-window)
               (lambda ()
                 'selected-window))
              ((symbol-function
                'window-buffer)
               (lambda (_)
                 'other-buffer))
              ((symbol-function
                'get-buffer-window)
               (lambda (buffer &rest arguments)
                 (push
                  (list
                   :visible
                   buffer
                   arguments)
                  calls)
                 'visible-window))
              ((symbol-function
                'cancel-timer)
               (lambda (timer)
                 (push
                  (list :unexpected-cancel timer)
                  calls)))
              ((symbol-function
                'run-at-time)
               (lambda (time repeat function &rest arguments)
                 (push
                  (list
                   :run
                   time
                   repeat
                   function
                   arguments)
                  calls)
                 'new-normal-timer)))
           (list
            (aria2--manage-refresh-timer)
            aria2--refresh-timer
            aria2--current-buffer-refresh-speed
            (nreverse calls))))"##;
    let expect = expect![
        "OK (:normal new-normal-timer :normal ((:visible list-buffer nil) (:run t 8 aria2--refresh nil)))"
    ];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_refresh_manager_selects_slow_timer_for_background_and_skips_unchanged_speed() {
    let elisp_form = r##"(let ((aria2--refresh-timer
                'existing)
               (aria2--current-buffer-refresh-speed
                :normal)
               calls)
         (cl-letf
             (((symbol-function
                'get-buffer)
               (lambda (_)
                 'list-buffer))
              ((symbol-function
                'selected-window)
               (lambda ()
                 'selected-window))
              ((symbol-function
                'window-buffer)
               (lambda (_)
                 'other-buffer))
              ((symbol-function
                'get-buffer-window)
               (lambda (&rest _)
                 nil))
              ((symbol-function
                'cancel-timer)
               (lambda (timer)
                 (push
                  (list :cancel timer)
                  calls)))
              ((symbol-function
                'run-at-time)
               (lambda (time repeat function &rest arguments)
                 (push
                  (list
                   :run
                   time
                   repeat
                   function
                   arguments)
                  calls)
                 'new-slow-timer)))
           (let ((first
                  (aria2--manage-refresh-timer))
                 first-calls)
             (setq first-calls
                   (nreverse calls)
                   calls nil)
             (list
              first
              first-calls
              aria2--refresh-timer
              aria2--current-buffer-refresh-speed
              (aria2--manage-refresh-timer)
              (nreverse calls)
              aria2--refresh-timer
              aria2--current-buffer-refresh-speed))))"##;
    let expect = expect![
        "OK (:slow ((:cancel existing) (:run t 20 aria2--refresh nil)) new-slow-timer :slow nil nil new-slow-timer :slow)"
    ];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_stop_timer_cancels_master_then_refresh_clears_both_and_exposes_partial_state_edges() {
    let elisp_form = r##"(mapcar
         (lambda (spec)
           (let ((aria2--master-timer
                  (car spec))
                 (aria2--refresh-timer
                  (cadr spec))
                 calls)
             (cl-letf
                 (((symbol-function
                    'cancel-timer)
                   (lambda (timer)
                     (push timer calls)
                     (when
                         (eq timer 'invalid)
                       (error
                        "invalid fixture timer")))))
               (list
                spec
                (condition-case error-data
                    (list
                     :ok
                     (aria2--stop-timer))
                  (error
                   (list
                    :error
                    (car error-data)
                    (cdr error-data))))
                (nreverse calls)
                aria2--master-timer
                aria2--refresh-timer))))
         '((master refresh)
           (nil refresh)
           (master nil)
           (invalid refresh)))"##;
    let expect = expect![[
        r#"OK (((master refresh) (:ok nil) (master refresh) nil nil) ((nil refresh) (:ok nil) (nil refresh) nil nil) ((master nil) (:ok nil) nil master nil) ((invalid refresh) (:error error ("invalid fixture timer")) (invalid) invalid refresh))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_refresh_reverts_existing_list_buffer_and_stops_timers_after_buffer_disappears() {
    let elisp_form = r##"(let ((buffer
                (get-buffer-create
                 aria2-list-buffer-name))
               calls)
         (unwind-protect
             (progn
               (with-current-buffer buffer
                 (erase-buffer)
                 (insert
                  "stale row"))
               (cl-letf
                   (((symbol-function
                      'revert-buffer)
                     (lambda (&rest arguments)
                       (push
                        (list
                         :revert
                         (buffer-name)
                         arguments)
                        calls)
                       :reverted))
                    ((symbol-function
                      'aria2--stop-timer)
                     (lambda ()
                       (push
                        (list :stop)
                        calls)
                       :stopped)))
                 (let ((present
                        (aria2--refresh)))
                   (kill-buffer buffer)
                   (setq buffer nil)
                   (list
                    present
                    (aria2--refresh)
                    (nreverse calls)))))
           (when
               (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect =
        expect![[r#"OK (:reverted :stopped ((:revert "*aria2: downloads list*" nil) (:stop)))"#]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_exit_persistence_saves_real_controller_state_when_running_and_deletes_it_when_stopped() {
    let elisp_form = r##"(let* ((aria2-cc-file
                  (aria2-test-path
                   "persisted-controller.eieio"))
                 (aria2--cc
                  (make-instance
                   'aria2-controller
                   "persisted-controller"
                   :file
                   aria2-cc-file
                   :request-id
                   73
                   :rcp-url
                   "http://persist.invalid/jsonrpc"
                   :secret
                   "persist-secret"
                   :pid
                   554))
                 running
                 stop-count)
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'aria2--stop-timer)
                   (lambda ()
                     (setq stop-count
                           (1+ (or stop-count 0)))))
                  ((symbol-function
                    'is-process-running)
                   (lambda (controller)
                     (list
                      (eq controller aria2--cc))
                     running)))
               (setq running t)
               (aria2--persist-settings-on-exit)
               (let* ((saved
                       (with-temp-buffer
                         (insert-file-contents-literally
                          aria2-cc-file)
                         (read
                          (current-buffer))))
                      (saved-contract
                       (list
                        (car saved)
                        (type-of
                         (cadr saved))
                        :opaque-object-name
                        (cddr saved))))
                 (setq running nil)
                 (aria2--persist-settings-on-exit)
                 (list
                  saved-contract
                  (file-exists-p
                   aria2-cc-file)
                  stop-count
                  (oref aria2--cc request-id)
                  (oref aria2--cc pid))))
           (when
               (file-exists-p
                aria2-cc-file)
             (delete-file
              aria2-cc-file))))"##;
    let expect = expect![[
        r#"OK ((aria2-controller string :opaque-object-name (:file "persisted-controller.eieio" :request-id 73 :rcp-url "http://persist.invalid/jsonrpc" :secret "persist-secret" :pid 554)) nil 2 73 554)"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_kill_on_exit_stops_timers_and_force_shuts_down_only_existing_controller() {
    let elisp_form = r##"(let ((controller
                (aria2-test-controller
                 0
                 909))
               calls)
         (cl-letf
             (((symbol-function
                'aria2--stop-timer)
               (lambda ()
                 (push
                  :stop
                  calls)
                 :stopped))
              ((symbol-function
                'shutdown)
               (lambda (this &optional force)
                 (push
                  (list
                   :shutdown
                   (eq this controller)
                   force)
                  calls)
                 :shutdown-result)))
           (let ((aria2--cc nil))
             (list
              (aria2--kill-on-exit)
              (nreverse calls)))
           (setq calls nil)
           (let ((aria2--cc controller))
             (list
              (aria2--kill-on-exit)
              (nreverse calls)))))"##;
    let expect = expect!["OK (:shutdown-result (:stop (:shutdown t t)))"];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_evil_quirks_respect_flag_register_modes_and_bind_control_w_to_evil_window_map() {
    let elisp_form = r##"(progn
         (setq evil-emacs-state-modes
               '(fundamental-mode)
               evil-window-map
               (let ((map
                      (make-sparse-keymap)))
                 (define-key
                  map
                  "x"
                  'ignore)
                 map))
         (let ((before
                (lookup-key
                 aria2-mode-map
                 "\C-w")))
         (provide
          'evil-states)
         (provide
          'evil-maps)
         (let ((aria2-add-evil-quirks
                nil))
           (aria2-maybe-add-evil-quirks))
         (let ((disabled
                (list
                 evil-emacs-state-modes
                 (lookup-key
                  aria2-mode-map
                  "\C-w"))))
           (let ((aria2-add-evil-quirks
                  t))
             (aria2-maybe-add-evil-quirks))
           (list
            before
            disabled
            evil-emacs-state-modes
            (eq
             (lookup-key
              aria2-mode-map
              "\C-w")
             evil-window-map)
            (lookup-key
             aria2-mode-map
             "\C-w")))))"##;
    let expect = expect![
        "OK (nil (#1=(fundamental-mode) nil) (aria2-dialog-mode aria2-mode . #1#) nil evil-window-map)"
    ];

    assert_aria2_parity(elisp_form, expect);
}

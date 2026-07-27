use expect_test::expect;

use super::assert_airplay_parity;

#[test]
fn airplay_image_view_reads_binary_bytes_and_selects_transitions() {
    let elisp_form = r##"(let* ((root (expand-file-name "image"
                                           (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                (image (expand-file-name "frame.jpg" root))
                calls)
         (make-directory root t)
         (with-temp-buffer
           (set-buffer-multibyte nil)
           (insert (unibyte-string 0 255 74 80 69 71 10))
           (write-region (point-min) (point-max) image nil 'silent))
         (cl-letf (((symbol-function 'airplay/protocol:put)
                    (lambda (path &rest args)
                      (push (list path
                                  (plist-get args :headers)
                                  (string-to-list (plist-get args :data))
                                  (multibyte-string-p (plist-get args :data)))
                            calls)
                      'sent)))
           (list
            (airplay/image:view image)
            (airplay/image:view image :dissolve)
            (airplay/image:view image :unknown-transition)
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (sent sent sent (("photo" (("X-Apple-Transition" . "None")) (0 4194303 74 80 69 71 10) t) ("photo" (("X-Apple-Transition" . "Dissolve")) (0 4194303 74 80 69 71 10) t) ("photo" (("X-Apple-Transition" . "None")) (0 4194303 74 80 69 71 10) t)))"#
    ]];
    assert_airplay_parity(elisp_form, expect);
}

#[test]
fn airplay_stop_clears_playing_state_then_shuts_down_and_posts_stop() {
    let elisp_form = r##"(let ((airplay/video->playing? t)
                calls)
         (cl-letf (((symbol-function 'airplay/server:shutdown)
                    (lambda ()
                      (push (list 'shutdown airplay/video->playing?) calls)
                      'closed))
                   ((symbol-function 'airplay/protocol:post)
                    (lambda (path &rest args)
                      (push (list 'post path args airplay/video->playing?) calls)
                      'posted)))
           (list (airplay:stop)
                 airplay/video->playing?
                 (nreverse calls))))"##;
    let expect = expect![[r#"OK (posted nil ((shutdown nil) (post "stop" nil nil)))"#]];
    assert_airplay_parity(elisp_form, expect);
}

#[test]
fn airplay_video_path_preserves_remote_urls_and_serves_existing_local_media() {
    let elisp_form = r##"(let* ((root (expand-file-name "video-path"
                                           (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                (movie (expand-file-name "movie.m4v" root))
                (airplay/video->server-port 7070)
                calls)
         (make-directory root t)
         (with-temp-file movie (insert "movie bytes"))
         (cl-letf (((symbol-function 'airplay/server:boot)
                    (lambda (path)
                      (push (list 'boot path) calls)))
                   ((symbol-function 'airplay/device:client-ip)
                    (lambda ()
                      (push '(client-ip) calls)
                      "10.0.0.8")))
           (list
            (airplay/video:--video-path "https://cdn.test/movie.m4v")
            (airplay/video:--video-path
             (expand-file-name "missing.m4v" root))
            (airplay/video:--video-path movie)
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("https://cdn.test/movie.m4v" "[ORACLE-SANDBOX]/video-path/missing.m4v" "http://10.0.0.8:7070/" ((boot "[ORACLE-SANDBOX]/video-path/movie.m4v") (client-ip)))"#
    ]];
    assert_airplay_parity(elisp_form, expect);
}

#[test]
fn airplay_scrub_request_parser_and_success_callback_form_a_complete_exchange() {
    let elisp_form = r##"(let (request callback-values)
         (cl-letf
             (((symbol-function 'airplay/protocol:get)
               (lambda (path &rest args)
                 (setq request
                       (list path
                             (plist-get args :parser)
                             (functionp (plist-get args :success))))
                 (funcall (plist-get args :success)
                          :data '(:position 12.5 :duration 90.0)
                          :response 'ignored)
                 'deferred-response)))
           (list
            (airplay/video:scrub
             (lambda (position duration)
               (setq callback-values (list position duration))))
            request
            callback-values)))"##;
    let expect =
        expect![[r#"OK (deferred-response ("scrub" airplay/protocol:parse-scrub t) (12.5 90.0))"#]];
    assert_airplay_parity(elisp_form, expect);
}

#[test]
fn airplay_info_forwards_decoded_playback_dictionary_to_callback() {
    let elisp_form = r##"(let (request received)
         (cl-letf
             (((symbol-function 'airplay/protocol:get)
               (lambda (path &rest args)
                 (setq request
                       (list path
                             (plist-get args :parser)
                             (functionp (plist-get args :success))))
                 (funcall (plist-get args :success)
                          :data '(("duration" . "90.0")
                                  ("readyToPlay" . "true")))
                 'request-value)))
           (list
            (airplay/video:info
             (lambda (data) (setq received data)))
            request
            received)))"##;
    let expect = expect![[
        r#"OK (request-value ("playback-info" airplay/protocol:parse-plist-xml t) (("duration" . "90.0") ("readyToPlay" . "true")))"#
    ]];
    assert_airplay_parity(elisp_form, expect);
}

#[test]
fn airplay_seek_pause_resume_and_private_rate_emit_exact_control_requests() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'airplay/protocol:post)
                    (lambda (path &rest args)
                      (push (list path args) calls)
                      (length calls))))
           (list
            (airplay/video:seek 20)
            (airplay/video:seek 12.75)
            (airplay/video:pause)
            (airplay/video:resume)
            (airplay/video:--rate "0.5")
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (1 2 3 4 5 (("scrub" (:params (("position" . "20")))) ("scrub" (:params (("position" . "12.75")))) ("rate" (:params (("value" . "0")))) ("rate" (:params (("value" . "1")))) ("rate" (:params (("value" . "0.5"))))))"#
    ]];
    assert_airplay_parity(elisp_form, expect);
}

#[test]
fn airplay_server_boot_and_shutdown_coordinate_daemon_process_boundaries() {
    let elisp_form = r##"(let ((airplay/video->server-daemon-name "parity-airplay")
                (airplay/video->server-port 8123)
                (buffer-file-name "/package/airplay.el")
                calls)
         (cl-letf
             (((symbol-function 'airplay/server:shutdown)
               (lambda () (push '(shutdown) calls)))
              ((symbol-function 'find-library-name)
               (lambda (library)
                 (push (list 'library library) calls)
                 "/deps/simple-httpd.el"))
              ((symbol-function 'call-process)
               (lambda (program infile destination display &rest args)
                 (push (list 'process
                             (equal program
                                    (concat invocation-directory
                                            invocation-name))
                             infile destination display args)
                       calls)
                 0))
              ((symbol-function 'server-eval-at)
               (lambda (server form)
                 (push (list 'eval server form) calls)
                 'done)))
           (list
            (airplay/server:boot "/media/movie.m4v")
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (done ((shutdown) (library "simple-httpd") (process t nil nil nil ("-Q" "--daemon=parity-airplay" "-l" "/deps/simple-httpd.el" "-l" "/package/airplay-video-server.el" "--eval" "(setq httpd-port 8123)")) (eval "parity-airplay" (airplay/server:start "/media/movie.m4v"))))"#
    ]];
    assert_airplay_parity(elisp_form, expect);
}

#[test]
fn airplay_server_shutdown_only_kills_a_running_named_daemon() {
    let elisp_form = r##"(let ((airplay/video->server-daemon-name "parity-airplay")
                running
                calls)
         (cl-letf (((symbol-function 'server-running-p)
                    (lambda (server)
                      (push (list 'running server) calls)
                      running))
                   ((symbol-function 'server-eval-at)
                    (lambda (server form)
                      (push (list 'eval server form) calls)
                      'killed)))
           (let ((first (airplay/server:shutdown)))
             (setq running t)
             (list first
                   (airplay/server:shutdown)
                   (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (nil killed ((running "parity-airplay") (running "parity-airplay") (eval "parity-airplay" (kill-emacs))))"#
    ]];
    assert_airplay_parity(elisp_form, expect);
}

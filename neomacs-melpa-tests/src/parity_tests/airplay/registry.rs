use expect_test::expect;

use super::{assert_airplay_autoload_parity, assert_airplay_parity};

#[test]
fn airplay_registry_defaults_transitions_and_loaded_feature_match() {
    let elisp_form = r##"(list
         (featurep 'airplay)
         (mapcar
          (lambda (symbol)
            (list symbol (symbol-value symbol)
                  (local-variable-if-set-p symbol)))
          '(airplay->host
            airplay->port
            airplay/video->server-daemon-name
            airplay/video->server-port
            airplay/video->server-lisp-name
            airplay/video->server-buffer
            airplay/video->playing?
            airplay->log-buffer))
         airplay/image->transitions
         (plist-get airplay/image->transitions :none)
         (plist-get airplay/image->transitions :dissolve))"##;
    let expect = expect![[
        r#"OK (t ((airplay->host nil nil) (airplay->port 7000 nil) (airplay/video->server-daemon-name "airplay-server" nil) (airplay/video->server-port 7070 nil) (airplay/video->server-lisp-name "airplay-video-server.el" nil) (airplay/video->server-buffer "*airplay-server*" nil) (airplay/video->playing? nil nil) (airplay->log-buffer "*airplay log*" nil)) (:none "None" :slide_left "SlideLeft" :slide_right "SlideRight" :dissolve "Dissolve") "None" "Dissolve")"#
    ]];
    assert_airplay_parity(elisp_form, expect);
}

#[test]
fn airplay_complete_callable_surface_arglists_and_commands_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list symbol
                 (help-function-arglist symbol t)
                 (commandp symbol)
                 (macrop symbol)
                 (autoloadp (symbol-function symbol))))
         '(airplay/debug-log
           airplay/device:browse
           airplay/device:--available-my-network-list
           airplay/device:client-ip
           airplay/net:request
           airplay/net:--make-query
           airplay/net:--make-url
           airplay/protocol:get
           airplay/protocol:post
           airplay/protocol:put
           airplay/protocol:make-text-parameters
           airplay/protocol:parse-text-parameters
           airplay/protocol:parse-scrub
           airplay/protocol:parse-plist-xml
           airplay/protocol:--parse-plist-xml
           airplay/server:boot
           airplay/server:shutdown
           airplay/image:view
           airplay:stop
           airplay/video:play
           airplay/video:--monitoring-playback
           airplay/video:--monitoring-buffering
           airplay/video:--video-path
           airplay/video:scrub
           airplay/video:seek
           airplay/video:info
           airplay/video:pause
           airplay/video:resume
           airplay/video:--rate))"##;
    let expect = expect![
        "OK ((airplay/debug-log (fmt &rest args) nil nil nil) (airplay/device:browse nil nil nil nil) (airplay/device:--available-my-network-list nil nil nil nil) (airplay/device:client-ip nil nil nil nil) (airplay/net:request (method path &rest args) nil nil nil) (airplay/net:--make-query (args) nil nil nil) (airplay/net:--make-url (path) nil nil nil) (airplay/protocol:get (path &rest args) nil nil nil) (airplay/protocol:post (path &rest args) nil nil nil) (airplay/protocol:put (path &rest args) nil nil nil) (airplay/protocol:make-text-parameters (args) nil nil nil) (airplay/protocol:parse-text-parameters nil nil nil nil) (airplay/protocol:parse-scrub nil nil nil nil) (airplay/protocol:parse-plist-xml nil nil nil nil) (airplay/protocol:--parse-plist-xml (top) nil nil nil) (airplay/server:boot (video) nil nil nil) (airplay/server:shutdown nil nil nil nil) (airplay/image:view (image_file &optional transition) nil nil nil) (airplay:stop nil t nil nil) (airplay/video:play (video_location) nil nil nil) (airplay/video:--monitoring-playback (&optional interval) nil nil nil) (airplay/video:--monitoring-buffering (&optional limit interval) nil nil nil) (airplay/video:--video-path (location) nil nil nil) (airplay/video:scrub (&optional cb) nil nil nil) (airplay/video:seek (position) nil nil nil) (airplay/video:info (&optional callback) nil nil nil) (airplay/video:pause nil t nil nil) (airplay/video:resume nil t nil nil) (airplay/video:--rate (value) nil nil nil))"
    ];
    assert_airplay_parity(elisp_form, expect);
}

#[test]
fn airplay_autoload_contract_exposes_documented_user_commands_without_loading_source() {
    let elisp_form = r##"(list
         (featurep 'airplay)
         (mapcar
          (lambda (symbol)
            (let ((definition (symbol-function symbol)))
              (list symbol
                    (autoloadp definition)
                    (nth 1 definition)
                    (nth 4 definition)
                    (commandp symbol))))
          '(airplay/image:view
            airplay:stop
            airplay/video:play
            airplay/video:scrub
            airplay/video:seek
            airplay/video:info
            airplay/video:pause
            airplay/video:resume
            airplay/net:request)))"##;
    let expect = expect![[
        r#"OK (nil ((airplay/image:view t "airplay" nil nil) (airplay:stop t "airplay" nil t) (airplay/video:play t "airplay" nil nil) (airplay/video:scrub t "airplay" nil nil) (airplay/video:seek t "airplay" nil nil) (airplay/video:info t "airplay" nil nil) (airplay/video:pause t "airplay" nil t) (airplay/video:resume t "airplay" nil t) (airplay/net:request nil nil nil nil)))"#
    ]];
    assert_airplay_autoload_parity(elisp_form, expect);
}

#[test]
fn airplay_debug_log_appends_formatted_protocol_events_to_one_buffer() {
    let elisp_form = r##"(let ((airplay->log-buffer
                (generate-new-buffer-name "*airplay parity log*")))
         (unwind-protect
             (progn
               (airplay/debug-log "PUT %s => %d\n" "photo" 200)
               (airplay/debug-log "scrub: %.1f/%s" 12.5 "90")
               (with-current-buffer airplay->log-buffer
                 (list (buffer-string)
                       (point)
                       major-mode
                       buffer-read-only)))
           (when (get-buffer airplay->log-buffer)
             (kill-buffer airplay->log-buffer))))"##;
    let expect = expect![[r#"OK ("PUT photo => 200\nscrub: 12.5/90" 32 fundamental-mode nil)"#]];
    assert_airplay_parity(elisp_form, expect);
}

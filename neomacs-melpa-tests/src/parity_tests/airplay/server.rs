use expect_test::expect;

use super::assert_airplay_server_parity;

#[test]
fn airplay_video_server_registers_partial_content_and_media_mime_types() {
    let elisp_form = r##"(list
         (assoc 206 httpd-status-codes)
         (mapcar (lambda (extension)
                   (assoc extension httpd-mime-types))
                 '("ts" "mov" "m4v"))
         (mapcar
          (lambda (symbol)
            (list symbol
                  (help-function-arglist symbol t)
                  (commandp symbol)))
          '(airplay/server:--request-ranges
            airplay/server:--accept-ranges
            airplay/server:--response-video
            airplay/server:start)))"##;
    let expect = expect![[
        r#"OK ((206 . "Partial Content") (("ts" . "video/MP2T") ("mov" . "video/quicktime") ("m4v" . "video/mp4")) ((airplay/server:--request-ranges (headers) nil) (airplay/server:--accept-ranges (range file-size) nil) (airplay/server:--response-video (proc path &optional req) nil) (airplay/server:start (media) nil)))"#
    ]];
    assert_airplay_server_parity(elisp_form, expect);
}

#[test]
fn airplay_range_parser_handles_closed_open_suffix_absent_and_invalid_headers() {
    let elisp_form = r##"(let (logs)
         (cl-letf (((symbol-function 'httpd-log)
                    (lambda (event) (push event logs))))
           (list
            (airplay/server:--request-ranges
             '(("Range" "bytes=10-19")))
            (airplay/server:--request-ranges
             '(("Range" "bytes=10-")))
            (airplay/server:--request-ranges
             '(("Range" "bytes=-20")))
            (airplay/server:--request-ranges nil)
            (airplay/server:--request-ranges
             '(("Range" "items=1-2")))
            (nreverse logs))))"##;
    let expect = expect![[
        r#"OK ((10 . 19) (10) (nil . 20) nil #1=((warning (format "Invalid range header: %s" range))) #1#)"#
    ]];
    assert_airplay_server_parity(elisp_form, expect);
}

#[test]
fn airplay_range_acceptance_normalizes_full_open_suffix_and_explicit_ranges() {
    let elisp_form = r##"(mapcar
         (lambda (range)
           (airplay/server:--accept-ranges range 100))
         '((nil)
           (10)
           (nil . 20)
           (10 . 19)
           (0 . 0)))"##;
    let expect = expect!["OK ((0 . 99) (10 . 99) (80 . 99) (10 . 19) (0 . 0))"];
    assert_airplay_server_parity(elisp_form, expect);
}

#[test]
fn airplay_full_video_response_sends_real_file_bytes_and_content_metadata() {
    let elisp_form = r##"(let* ((root (expand-file-name "full-response"
                                           (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                (video (expand-file-name "clip.m4v" root))
                sent
                logs)
         (make-directory root t)
         (with-temp-buffer
           (set-buffer-multibyte nil)
           (insert (unibyte-string 0 1 2 3 250 251 252 253))
           (write-region (point-min) (point-max) video nil 'silent))
         (cl-letf
             (((symbol-function 'httpd-date-string)
               (lambda (_) "Mon, 01 Jan 2001 00:00:00 GMT"))
              ((symbol-function 'httpd-log)
               (lambda (event) (push event logs)))
              ((symbol-function 'httpd-send-header)
               (lambda (proc mime status &rest headers)
                 (setq sent
                       (list proc mime status headers
                             (string-to-list (buffer-string))
                             (multibyte-string-p (buffer-string))))
                 'sent)))
           (list
            (airplay/server:--response-video 'client video)
            sent
            (nreverse logs))))"##;
    let expect = expect![[
        r#"OK (sent (client "video/mp4" 200 (:Last-Modified "Mon, 01 Jan 2001 00:00:00 GMT") (0 1 2 3 250 251 252 253) nil) ((file "[ORACLE-SANDBOX]/full-response/clip.m4v")))"#
    ]];
    assert_airplay_server_parity(elisp_form, expect);
}

#[test]
fn airplay_partial_video_response_slices_bytes_and_reports_exact_content_range() {
    let elisp_form = r##"(let* ((root (expand-file-name "partial-response"
                                           (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                (video (expand-file-name "clip.mov" root))
                sent)
         (make-directory root t)
         (with-temp-buffer
           (set-buffer-multibyte nil)
           (dotimes (byte 16) (insert (unibyte-string byte)))
           (write-region (point-min) (point-max) video nil 'silent))
         (cl-letf
             (((symbol-function 'httpd-date-string)
               (lambda (_) "Mon, 01 Jan 2001 00:00:00 GMT"))
              ((symbol-function 'httpd-log) #'ignore)
              ((symbol-function 'httpd-send-header)
               (lambda (proc mime status &rest headers)
                 (setq sent
                       (list proc mime status headers
                             (string-to-list (buffer-string))))
                 'partial)))
           (list
            (airplay/server:--response-video
             'client video '(("Range" "bytes=4-9")))
            sent)))"##;
    let expect = expect![[
        r#"OK (partial (client "video/quicktime" 206 (:Last-Modified "Mon, 01 Jan 2001 00:00:00 GMT" :Accept-Ranges "bytes" :Content-Range "bytes 4-9/16") (4 5 6 7 8 9)))"#
    ]];
    assert_airplay_server_parity(elisp_form, expect);
}

#[test]
fn airplay_suffix_video_response_serves_last_requested_bytes() {
    let elisp_form = r##"(let* ((root (expand-file-name "suffix-response"
                                           (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                (video (expand-file-name "clip.ts" root))
                sent)
         (make-directory root t)
         (with-temp-file video (insert "abcdefghijkl"))
         (cl-letf
             (((symbol-function 'httpd-date-string)
               (lambda (_) "Mon, 01 Jan 2001 00:00:00 GMT"))
              ((symbol-function 'httpd-log) #'ignore)
              ((symbol-function 'httpd-send-header)
               (lambda (_proc mime status &rest headers)
                 (setq sent
                       (list mime status headers (buffer-string)))
                 'suffix)))
           (list
            (airplay/server:--response-video
             'client video '(("Range" "bytes=-5")))
            sent)))"##;
    let expect = expect![[
        r#"OK (suffix ("video/MP2T" 206 (:Last-Modified "Mon, 01 Jan 2001 00:00:00 GMT" :Accept-Ranges "bytes" :Content-Range "bytes 7-11/12") "hijkl"))"#
    ]];
    assert_airplay_server_parity(elisp_form, expect);
}

#[test]
fn airplay_server_start_installs_root_handler_for_selected_media_and_starts_httpd() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'airplay/server:--response-video)
                    (lambda (proc media request)
                      (push (list 'response proc media request) calls)
                      'served))
                   ((symbol-function 'httpd-start)
                    (lambda ()
                      (push '(start) calls)
                      'started)))
           (let ((result
                  (airplay/server:start "/media/parity.m4v")))
             (list result
                   (httpd/ 'client nil nil
                           '(("Range" "bytes=3-7")))
                   (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (started served ((start) (response client "/media/parity.m4v" (("Range" "bytes=3-7")))))"#
    ]];
    assert_airplay_server_parity(elisp_form, expect);
}

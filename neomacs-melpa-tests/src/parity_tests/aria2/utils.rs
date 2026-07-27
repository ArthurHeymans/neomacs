use expect_test::expect;

use super::assert_aria2_parity;

#[test]
fn aria2_rpc_url_formats_default_boundary_and_invalid_port_values_exactly() {
    let elisp_form = r##"(mapcar
         (lambda (port)
           (let ((aria2-rcp-listen-port
                  port))
             (condition-case error-data
                 (list
                  port
                  :ok
                  (aria2--url))
               (error
                (list
                 port
                 :error
                 (car error-data)
                 (cdr error-data))))))
         '(6800
           0
           65535
           -1
           "6800"
           nil))"##;
    let expect = expect![[
        r#"OK ((6800 :ok "http://localhost:6800/jsonrpc") (0 :ok "http://localhost:0/jsonrpc") (65535 :ok "http://localhost:65535/jsonrpc") (-1 :ok "http://localhost:-1/jsonrpc") ("6800" :error error ("Format specifier doesn’t match argument type")) (nil :error error ("Format specifier doesn’t match argument type")))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_base64_encodes_real_empty_text_unicode_and_binary_files_without_line_breaks() {
    let elisp_form = r##"(let ((specs
                `(("empty.bin" . ,(unibyte-string))
                  ("ascii.txt"
                   . ,(string-make-unibyte
                       "hello aria2\n"))
                  ("binary.bin"
                   . ,(unibyte-string
                       0 1 2 3 127 128 254 255))
                  ("unicode.bin"
                   . ,(encode-coding-string
                       "日本語 λ"
                       'utf-8
                       t))))
               results)
         (unwind-protect
             (progn
               (dolist (spec specs)
                 (let ((path
                        (aria2-test-path
                         (car spec))))
                   (with-temp-file
                       path
                     (set-buffer-multibyte
                      nil)
                     (insert
                      (cdr spec)))
                   (let ((encoded
                          (aria2--base64-encode-file
                           path)))
                     (setq results
                           (append
                            results
                            (list
                             (list
                              (car spec)
                              encoded
                              (string-match-p
                               "\n"
                               encoded)
                              (string-to-list
                               (base64-decode-string
                                encoded)))))))))
               results)
           (dolist (spec specs)
             (let ((path
                    (aria2-test-path
                     (car spec))))
               (when
                   (file-exists-p
                    path)
                 (delete-file
                  path))))))"##;
    let expect = expect![[
        r#"OK (("empty.bin" "" nil nil) ("ascii.txt" "aGVsbG8gYXJpYTIK" nil (104 101 108 108 111 32 97 114 105 97 50 10)) ("binary.bin" "AAECA3+A/v8=" nil (0 1 2 3 127 128 254 255)) ("unicode.bin" "5pel5pys6KqeIM67" nil (230 151 165 230 156 172 232 170 158 32 206 187)))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_base64_missing_directory_and_regular_file_boundaries_signal_exact_errors() {
    let elisp_form = r##"(let ((missing
                (aria2-test-path
                 "missing.bin"))
               (directory
                (aria2-test-path
                 "base64-directory")))
         (make-directory
          directory
          t)
         (unwind-protect
             (mapcar
              (lambda (path)
                (condition-case error-data
                    (list
                     (file-name-nondirectory
                      path)
                     :ok
                     (aria2--base64-encode-file
                      path))
                  (error
                   (list
                    (file-name-nondirectory
                     path)
                    :error
                    (car error-data)
                    (cdr error-data)))))
              (list
               missing
               directory))
           (delete-directory
            directory)))"##;
    let expect = expect![[
        r#"OK (("missing.bin" :error aria2-err-file-doesnt-exist (path)) ("base64-directory" :error file-error ("Read error" "Is a directory" "[ORACLE-SANDBOX]/base64-directory")))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_process_predicate_requires_exact_command_and_real_login_user() {
    let elisp_form = r##"(let ((attributes
                '((101
                   (comm . "aria2c")
                   (user . "fixture-user"))
                  (102
                   (comm . "aria2")
                   (user . "fixture-user"))
                  (103
                   (comm . "aria2c")
                   (user . "other-user"))
                  (104
                   (user . "fixture-user"))
                  (105
                   (comm . "aria2c")))))
         (cl-letf
             (((symbol-function
                'user-real-login-name)
               (lambda ()
                 "fixture-user"))
              ((symbol-function
                'process-attributes)
               (lambda (pid)
                 (cdr
                  (assq
                   pid
                   attributes)))))
           (mapcar
            (lambda (pid)
              (list
               pid
               (process-attributes
                pid)
               (aria2--is-aria-process-p
                pid)))
            '(101 102 103 104 105 999))))"##;
    let expect = expect![[
        r#"OK ((101 ((comm . "aria2c") (user . "fixture-user")) t) (102 ((comm . "aria2") (user . "fixture-user")) nil) (103 ((comm . "aria2c") (user . "other-user")) nil) (104 ((user . "fixture-user")) nil) (105 ((comm . "aria2c")) nil) (999 nil nil))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_error_decoder_covers_every_documented_code_and_unknown_input_type() {
    let elisp_form = r##"(mapcar
         (lambda (code)
           (condition-case error-data
               (list
                code
                :ok
                (aria2--decode-error
                 code))
             (error
              (list
               code
               :error
               (car error-data)
               (cdr error-data)))))
         (append
          (mapcar
           #'number-to-string
           (number-sequence
            0
            30))
          '("31"
            "999"
            ""
            nil
            1
            error)))"##;
    let expect = expect![[
        r#"OK (("0" :ok "All downloads were successful") ("1" :ok "An unknown error occurred") ("2" :ok "Time out occurred") ("3" :ok "A resource was not found") ("4" :ok "Aria2 saw the specified number of \"resource not found\" error. See --max-file-not-found option") ("5" :ok "A download aborted because download speed was too slow. See --lowest-speed-limit option") ("6" :ok "Network problem occurred") ("7" :ok "There were unfinished downloads") ("8" :ok "Remote server did not support resume when resume was required to complete download") ("9" :ok "There was not enough disk space available") ("10" :ok "Piece length was different from one in .aria2 control file. See --allow-piece-length-change option") ("11" :ok "Aria2 was downloading same file at that moment") ("12" :ok "Aria2 was downloading same info hash torrent at that moment") ("13" :ok "File already existed. See --allow-overwrite option") ("14" :ok "Renaming file failed. See --auto-file-renaming option") ("15" :ok "Aria2 could not open existing file") ("16" :ok "Aria2 could not create new file or truncate existing file") ("17" :ok "File I/O error occurred") ("18" :ok "Aria2 could not create directory") ("19" :ok "Name resolution failed") ("20" :ok "Aria2 could not parse Metalink document") ("21" :ok "FTP command failed") ("22" :ok "HTTP response header was bad or unexpected") ("23" :ok "Too many redirects occurred") ("24" :ok "HTTP authorization failed") ("25" :ok "Aria2 could not parse bencoded file (usually \".torrent\" file)") ("26" :ok "A \".torrent\" file was corrupted or missing information that aria2 needed") ("27" :ok "Magnet URI was bad") ("28" :ok "Bad/unrecognized option was given or unexpected option argument was given") ("29" :ok "The remote server was unable to handle the request due to a temporary overloading or maintenance") ("30" :ok "Aria2 could not parse JSON-RPC request") ("31" :ok "Unknown/other error") ("999" :ok "Unknown/other error") ("" :ok "Unknown/other error") (nil :ok "Unknown/other error") (1 :error wrong-type-argument (stringp 1)) (error :ok "Unknown/other error"))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_custom_errors_preserve_parent_conditions_messages_and_signal_payloads() {
    let elisp_form = r##"(mapcar
         (lambda (spec)
           (let ((symbol
                  (car spec))
                 (payload
                  (cadr spec)))
             (condition-case error-data
                 (signal
                  symbol
                  payload)
               (error
                (list
                 symbol
                 (get symbol 'error-conditions)
                 (get symbol 'error-message)
                 (car error-data)
                 (cdr error-data)
                 (error-message-string
                  error-data))))))
         '((aria2-err-too-many-magnet-urls
            ("magnet:?xt=one"
             "magnet:?xt=two"))
           (aria2-err-file-doesnt-exist
            ("/missing/file"))
           (aria2-err-not-a-torrent-file
            nil)
           (aria2-err-not-a-metalink-file
            nil)
           (aria2-err-failed-to-start
            ("aria2c --enable-rpc"))
           (aria2-err-no-executable
            nil)
           (aria2-err-no-such-position-type
            ("WRONG"))))"##;
    let expect = expect![[
        r#"OK ((aria2-err-too-many-magnet-urls (aria2-err-too-many-magnet-urls user-error error) "Only one magnet link per download is allowed" aria2-err-too-many-magnet-urls ("magnet:?xt=one" "magnet:?xt=two") "Only one magnet link per download is allowed: \"magnet:?xt=one\", \"magnet:?xt=two\"") (aria2-err-file-doesnt-exist (aria2-err-file-doesnt-exist user-error error) "File doesn't exist" aria2-err-file-doesnt-exist ("/missing/file") "File doesn’t exist: \"/missing/file\"") (aria2-err-not-a-torrent-file (aria2-err-not-a-torrent-file user-error error) "This is not a .torrent file" aria2-err-not-a-torrent-file nil "This is not a .torrent file") (aria2-err-not-a-metalink-file (aria2-err-not-a-metalink-file user-error error) "This is not a .metalink file" aria2-err-not-a-metalink-file nil "This is not a .metalink file") (aria2-err-failed-to-start (aria2-err-failed-to-start error) "Failed to start" aria2-err-failed-to-start ("aria2c --enable-rpc") "Failed to start: \"aria2c --enable-rpc\"") (aria2-err-no-executable (aria2-err-no-executable error) "Couldn't find `aria2c' executable, aborting" aria2-err-no-executable nil "Couldn’t find ‘aria2c’ executable, aborting") (aria2-err-no-such-position-type (aria2-err-no-such-position-type error) "Wrong position type" aria2-err-no-such-position-type ("WRONG") "Wrong position type: \"WRONG\""))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

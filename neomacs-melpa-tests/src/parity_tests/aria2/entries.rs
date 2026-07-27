use expect_test::expect;

use super::assert_aria2_parity;

#[test]
fn aria2_entry_file_and_type_extractors_cover_torrent_http_ftp_magnet_empty_and_unknown_metadata() {
    let elisp_form = r##"(let ((entries
                '(((gid . "bt")
                   (bittorrent
                    (info
                     (name . "linux.iso")))
                   (files
                    . [((uris
                         . [((uri . "https://ignored.invalid/a"))]))]))
                  ((gid . "http")
                   (files
                    . [((uris
                         . [((uri . "https://cdn.invalid/path/archive.tar.xz?token=1"))]))]))
                  ((gid . "ftp")
                   (files
                    . [((uris
                         . [((uri . "ftp://mirror.invalid/pub/日本語.zip"))]))]))
                  ((gid . "magnet")
                   (files
                    . [((uris
                         . [((uri . "magnet:?xt=urn:btih:abcdef&dn=name"))]))]))
                  ((gid . "empty")
                   (files
                    . [((uris . []))]))
                  ((gid . "bt-without-name")
                   (bittorrent
                    (info))
                   (files
                    . [((uris . []))])))))
         (mapcar
          (lambda (entry)
            (list
             (alist-get
              'gid
              entry)
             (aria2--list-entries-File
              entry)
             (aria2--list-entries-Type
              entry)))
          entries))"##;
    let expect = expect![[
        r#"OK (("bt" "linux.iso" "bittorrent") ("http" "archive.tar.xz?token=1" "https") ("ftp" "日本語.zip" "ftp") ("magnet" "magnet:?xt=urn:btih:abcdef&dn=name" "magnet") ("empty" "unknown" "unknown") ("bt-without-name" "unknown" "bittorrent"))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_entry_progress_handles_zero_negative_fractional_complete_and_overcomplete_lengths() {
    let elisp_form = r##"(mapcar
         (lambda (spec)
           (let ((entry
                  `((totalLength
                     . ,(car spec))
                    (completedLength
                     . ,(cadr spec)))))
             (condition-case error-data
                 (list
                  spec
                  :ok
                  (aria2--list-entries-Done
                   entry))
               (error
                (list
                 spec
                 :error
                 (car error-data)
                 (cdr error-data))))))
         '(("0" "0")
           ("-1" "10")
           ("1" "0")
           ("3" "1")
           ("100" "99")
           ("100" "100")
           ("100" "125")
           ("10485760" "5242880")
           (nil nil)
           ("garbage" "12")))"##;
    let expect = expect![[
        r#"OK ((("0" "0") :ok "-") (("-1" "10") :ok "-") (("1" "0") :ok "0%") (("3" "1") :ok "33%") (("100" "99") :ok "99%") (("100" "100") :ok "100%") (("100" "125") :ok "125%") (("10485760" "5242880") :ok "50%") ((nil nil) :error wrong-type-argument (stringp nil)) (("garbage" "12") :ok "-"))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_entry_transfer_speeds_format_byte_boundaries_and_non_numeric_values_as_kilobytes() {
    let elisp_form = r##"(mapcar
         (lambda (spec)
           (let ((entry
                  `((downloadSpeed
                     . ,(car spec))
                    (uploadSpeed
                     . ,(cadr spec)))))
             (condition-case error-data
                 (list
                  spec
                  :ok
                  (aria2--list-entries-Download
                   entry)
                  (aria2--list-entries-Upload
                   entry))
               (error
                (list
                 spec
                 :error
                 (car error-data)
                 (cdr error-data))))))
         '(("0" "0")
           ("1" "1023")
           ("1024" "1536")
           ("1537" "1048576")
           ("10485760" "2097152")
           ("-1024" "-1")
           (nil "garbage")))"##;
    let expect = expect![[
        r#"OK ((("0" "0") :ok "0.00 kB" "0.00 kB") (("1" "1023") :ok "0.00 kB" "0.00 kB") (("1024" "1536") :ok "1.00 kB" "1.00 kB") (("1537" "1048576") :ok "1.00 kB" "1024.00 kB") (("10485760" "2097152") :ok "10240.00 kB" "2048.00 kB") (("-1024" "-1") :ok "-1.00 kB" "0.00 kB") ((nil "garbage") :error wrong-type-argument (stringp nil)))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_entry_size_formats_every_unit_transition_and_extreme_value_exactly() {
    let elisp_form = r##"(mapcar
         (lambda (value)
           (condition-case error-data
               (list
                value
                :ok
                (aria2--list-entries-Size
                 `((totalLength
                    . ,value))))
             (error
              (list
               value
               :error
               (car error-data)
               (cdr error-data)))))
         '("0"
           "1"
           "1023"
           "1024"
           "1048575"
           "1048576"
           "1073741823"
           "1073741824"
           "1099511627775"
           "1099511627776"
           "1649267441664"
           "-1"
           ""
           nil
           "not-a-number"))"##;
    let expect = expect![[
        r#"OK (("0" :ok "0.00 B") ("1" :ok "1.00 B") ("1023" :ok "1023.00 B") ("1024" :ok "1.00 kB") ("1048575" :ok "1023.00 kB") ("1048576" :ok "1.00 MB") ("1073741823" :ok "1023.00 MB") ("1073741824" :ok "1.00 GB") ("1099511627775" :ok "1023.00 GB") ("1099511627776" :ok " 1 TB") ("1649267441664" :ok " 1 TB") ("-1" :ok "-1.00 B") ("" :ok "0.00 B") (nil :error wrong-type-argument (stringp nil)) ("not-a-number" :ok "0.00 B"))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_entry_status_and_error_extractors_preserve_status_and_decode_known_unknown_absent_codes() {
    let elisp_form = r##"(mapcar
         (lambda (spec)
           (let ((entry
                  `((status
                     . ,(car spec))
                    ,@(when
                          (cadr spec)
                        `((errorCode
                           . ,(cadr spec)))))))
             (list
              spec
              (aria2--list-entries-Status
               entry)
              (aria2--list-entries-Err
               entry))))
         '(("active" nil)
           ("waiting" "0")
           ("paused" "9")
           ("error" "30")
           ("removed" "999")
           (nil "")
           ("" "1")))"##;
    let expect = expect![[
        r#"OK ((("active" nil) "active" " - ") (("waiting" "0") "waiting" "All downloads were successful") (("paused" "9") "paused" "There was not enough disk space available") (("error" "30") "error" "Aria2 could not parse JSON-RPC request") (("removed" "999") "removed" "Unknown/other error") ((nil "") nil "Unknown/other error") (("" "1") "" "An unknown error occurred"))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_list_entries_merges_realistic_active_waiting_stopped_rows_and_builds_exact_tabulated_cells()
 {
    let elisp_form = r##"(let* ((aria2--cc
                  (aria2-test-controller))
                 (active
                  '(((gid . "gid-active")
                     (status . "active")
                     (totalLength . "10485760")
                     (completedLength . "2621440")
                     (downloadSpeed . "524288")
                     (uploadSpeed . "1024")
                     (files
                      . [((uris
                           . [((uri . "https://cdn.invalid/releases/app.tar.xz"))]))])
                     (dir . "/downloads"))))
                 (waiting
                  '(((gid . "gid-waiting")
                     (status . "paused")
                     (totalLength . "2048")
                     (completedLength . "1024")
                     (downloadSpeed . "0")
                     (uploadSpeed . "0")
                     (files
                      . [((uris
                           . [((uri . "magnet:?xt=urn:btih:deadbeef"))]))])
                     (bittorrent
                      (info
                       (name . "dataset λ"))))))
                 (stopped
                  '(((gid . "gid-error")
                     (status . "error")
                     (totalLength . "512")
                     (completedLength . "12")
                     (downloadSpeed . "0")
                     (uploadSpeed . "0")
                     (errorCode . "9")
                     (files
                      . [((uris
                           . [((uri . "ftp://mirror.invalid/broken.bin"))]))]))
                    ((gid . "gid-complete")
                     (status . "complete")
                     (totalLength . "1099511627776")
                     (completedLength . "1099511627776")
                     (downloadSpeed . "0")
                     (uploadSpeed . "0")
                     (files
                      . [((uris . []))]))))
                 calls)
         (cl-letf
             (((symbol-function
                'tellActive)
               (lambda (controller keys)
                 (push
                  (list
                   :active
                   (eq controller aria2--cc)
                   keys)
                  calls)
                 active))
              ((symbol-function
                'tellWaiting)
               (lambda (controller offset num keys)
                 (push
                  (list
                   :waiting
                   (eq controller aria2--cc)
                   offset
                   num
                   keys)
                  calls)
                 waiting))
              ((symbol-function
                'tellStopped)
               (lambda (controller offset num keys)
                 (push
                  (list
                   :stopped
                   (eq controller aria2--cc)
                   offset
                   num
                   keys)
                  calls)
                 stopped)))
           (list
            (aria2--list-entries)
            (nreverse calls)
            aria2--tell-keys)))"##;
    let expect = expect![[
        r#"OK ((("gid-complete" [("unknown" face aria2-file-face) ("complete" face aria2-status-face) ("unknown" face aria2-type-face) ("100%" face aria2-done-face) ("0.00 kB" face aria2-download-face) ("0.00 kB" face aria2-upload-face) " 1 TB" (" - " face aria2-error-face)]) ("gid-error" [("broken.bin" face aria2-file-face) ("error" face aria2-status-face) ("ftp" face aria2-type-face) ("2%" face aria2-done-face) ("0.00 kB" face aria2-download-face) ("0.00 kB" face aria2-upload-face) "512.00 B" ("There was not enough disk space available" face aria2-error-face)]) ("gid-waiting" [("dataset λ" face aria2-file-face) ("paused" face aria2-status-face) ("bittorrent" face aria2-type-face) ("50%" face aria2-done-face) ("0.00 kB" face aria2-download-face) ("0.00 kB" face aria2-upload-face) "2.00 kB" (" - " face aria2-error-face)]) ("gid-active" [("app.tar.xz" face aria2-file-face) ("active" face aria2-status-face) ("https" face aria2-type-face) ("25%" face aria2-done-face) ("512.00 kB" face aria2-download-face) ("1.00 kB" face aria2-upload-face) "10.00 MB" (" - " face aria2-error-face)])) ((:active t #1=["gid" "status" "totalLength" "completedLength" "downloadSpeed" "uploadSpeed" "files" "dir" "bittorrent" "errorCode"]) (:waiting t nil nil #1#) (:stopped t nil nil #1#)) #1#)"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

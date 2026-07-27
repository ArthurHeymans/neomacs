use expect_test::expect;

use super::assert_aria2_parity;

#[test]
fn aria2_add_uri_preserves_mirror_magnet_unicode_empty_and_vector_shapes() {
    let elisp_form = r##"(let ((controller
                (aria2-test-controller))
               calls)
         (cl-letf
             (((symbol-function
                'make-request)
               (lambda (this method &rest params)
                 (push
                  (list
                   (eq this controller)
                   method
                   params)
                  calls)
                 (format
                  "gid-%d"
                  (length calls)))))
           (list
            (addUri
             controller
             '("https://mirror-one.invalid/file.iso"
               "ftp://mirror-two.invalid/file.iso"))
            (addUri
             controller
             '("magnet:?xt=urn:btih:abcdef&dn=日本語"))
            (addUri
             controller
             nil)
            (addUri
             controller
             ["http://vector.invalid/a"
              "http://vector.invalid/b"])
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("gid-1" "gid-2" "gid-3" "gid-4" ((t "aria2.addUri" (["https://mirror-one.invalid/file.iso" "ftp://mirror-two.invalid/file.iso"])) (t "aria2.addUri" (["magnet:?xt=urn:btih:abcdef&dn=日本語"])) (t "aria2.addUri" ([])) (t "aria2.addUri" (["http://vector.invalid/a" "http://vector.invalid/b"]))))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_add_torrent_reads_real_bytes_builds_option_plist_and_reports_path_contract_errors() {
    let elisp_form = r##"(let* ((controller
                  (aria2-test-controller))
                 (torrent
                  (aria2-test-path
                   "fixture.torrent"))
                 (wrong
                  (aria2-test-path
                   "fixture.bin"))
                 (missing
                  (aria2-test-path
                   "missing.torrent"))
                 calls)
         (with-temp-file
             torrent
           (set-buffer-multibyte
            nil)
           (insert
            (unibyte-string
             100 52 58 105 110 102 111 49 58 120 101)))
         (with-temp-file
             wrong
           (insert
            "not a torrent"))
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'make-request)
                   (lambda (this method &rest params)
                     (push
                      (list
                       (eq this controller)
                       method
                       params)
                      calls)
                     :accepted)))
               (list
                (addTorrent
                 controller
                 torrent)
                (addTorrent
                 controller
                 torrent
                 :select-file
                 "1,3-5"
                 :dir
                 "/downloads/日本語")
                (mapcar
                 (lambda (path)
                   (condition-case error-data
                       (list
                        :ok
                        (addTorrent
                         controller
                         path))
                     (error
                      (list
                       :error
                       (car error-data)
                       (cdr error-data)
                       (error-message-string
                        error-data)))))
                 (list
                  wrong
                  missing))
                (nreverse calls)))
           (delete-file torrent)
           (delete-file wrong)))"##;
    let expect = expect![[
        r#"OK (:accepted :accepted ((:error aria2-err-not-a-torrent-file nil "This is not a .torrent file") (:error aria2-err-file-doesnt-exist (path) "File doesn’t exist: path")) ((t "aria2.addTorrent" ("ZDQ6aW5mbzE6eGU=" [] nil)) (t "aria2.addTorrent" ("ZDQ6aW5mbzE6eGU=" [] (:select-file "1,3-5" :dir "/downloads/日本語")))))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_add_metalink_accepts_meta4_and_metalink_real_files_and_rejects_other_paths() {
    let elisp_form = r##"(let* ((controller
                  (aria2-test-controller))
                 (paths
                  (mapcar
                   #'aria2-test-path
                   '("fixture.meta4"
                     "fixture.metalink"
                     "fixture.META4"
                     "fixture.xml")))
                 calls)
         (dolist (path paths)
           (with-temp-file
               path
             (insert
              (format
               "<metalink name=%S>λ</metalink>"
               (file-name-nondirectory
                path)))))
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'make-request)
                   (lambda (this method &rest params)
                     (push
                      (list
                       (eq this controller)
                       method
                       params)
                      calls)
                     :accepted)))
               (list
                (mapcar
                 (lambda (path)
                   (condition-case error-data
                       (list
                        (file-name-nondirectory
                         path)
                        :ok
                        (addMetalink
                         controller
                         path))
                     (error
                      (list
                       (file-name-nondirectory
                        path)
                       :error
                       (car error-data)
                       (cdr error-data)))))
                 (append
                  paths
                  (list
                   (aria2-test-path
                    "missing.meta4"))))
                (nreverse calls)))
           (dolist (path paths)
             (delete-file path))))"##;
    let expect = expect![[
        r#"OK ((("fixture.meta4" :ok :accepted) ("fixture.metalink" :ok :accepted) ("fixture.META4" :ok :accepted) ("fixture.xml" :error aria2-err-not-a-metalink-file nil) ("missing.meta4" :error aria2-err-file-doesnt-exist (path))) ((t "aria2.addMetalink" ("PG1ldGFsaW5rIG5hbWU9ImZpeHR1cmUubWV0YTQiPs67PC9tZXRhbGluaz4=")) (t "aria2.addMetalink" ("PG1ldGFsaW5rIG5hbWU9ImZpeHR1cmUubWV0YWxpbmsiPs67PC9tZXRhbGluaz4=")) (t "aria2.addMetalink" ("PG1ldGFsaW5rIG5hbWU9ImZpeHR1cmUuTUVUQTQiPs67PC9tZXRhbGluaz4="))))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_download_lifecycle_api_selects_normal_and_force_methods_with_exact_arguments() {
    let elisp_form = r##"(let ((controller
                (aria2-test-controller))
               calls)
         (cl-letf
             (((symbol-function
                'make-request)
               (lambda (this method &rest params)
                 (push
                  (list
                   (eq this controller)
                   method
                   params)
                  calls)
                 (intern method))))
           (list
            (remove-download
             controller
             "gid-a")
            (remove-download
             controller
             "gid-b"
             t)
            (pause
             controller
             "gid-c")
            (pause
             controller
             "gid-d"
             :force)
            (pauseAll controller)
            (pauseAll controller t)
            (unpause
             controller
             "gid-e")
            (unpauseAll controller)
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (aria2.remove aria2.forceRemove aria2.pause aria2.forcePause aria2.pauseAll aria2.forcePauseAll aria2.unpause aria2.unpauseAll ((t "aria2.remove" ("gid-a")) (t "aria2.forceRemove" ("gid-b")) (t "aria2.pause" ("gid-c")) (t "aria2.forcePause" ("gid-d")) (t "aria2.pauseAll" nil) (t "aria2.forcePauseAll" nil) (t "aria2.unpause" ("gid-e")) (t "aria2.unpauseAll" nil)))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_status_query_api_applies_defaults_optional_keys_and_maximum_wait_ranges() {
    let elisp_form = r##"(let ((controller
                (aria2-test-controller))
               (keys
                ["gid"
                 "status"
                 "completedLength"])
               calls)
         (cl-letf
             (((symbol-function
                'make-request)
               (lambda (this method &rest params)
                 (push
                  (list
                   (eq this controller)
                   method
                   params)
                  calls)
                 (length calls))))
           (list
            (tellStatus
             controller
             "gid-a")
            (tellStatus
             controller
             "gid-b"
             keys)
            (tellActive controller)
            (tellActive
             controller
             keys)
            (tellWaiting controller)
            (tellWaiting
             controller
             -3
             25
             keys)
            (tellStopped controller)
            (tellStopped
             controller
             9
             11
             keys)
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (1 2 3 4 5 6 7 8 ((t "aria2.tellStatus" ("gid-a" nil)) (t "aria2.tellStatus" ("gid-b" #1=["gid" "status" "completedLength"])) (t "aria2.tellActive" (nil)) (t "aria2.tellActive" (#1#)) (t "aria2.tellWaiting" (0 2305843009213693951 nil)) (t "aria2.tellWaiting" (-3 25 #1#)) (t "aria2.tellStopped" (0 2305843009213693951 nil)) (t "aria2.tellStopped" (9 11 #1#))))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_position_uri_and_option_api_preserves_defaults_shapes_and_validation_failures() {
    let elisp_form = r##"(let ((controller
                (aria2-test-controller))
               calls)
         (cl-letf
             (((symbol-function
                'make-request)
               (lambda (this method &rest params)
                 (push
                  (list
                   (eq this controller)
                   method
                   params)
                  calls)
                 params)))
           (list
            (mapcar
             (lambda (spec)
               (condition-case error-data
                   (list
                    spec
                    :ok
                    (apply
                     #'changePosition
                     controller
                     "gid-pos"
                     spec))
                 (error
                  (list
                   spec
                   :error
                   (car error-data)
                   (cdr error-data)))))
             '((4)
               (0 "POS_SET")
               (-2 "POS_CUR")
               (-1 "POS_END")
               (3 "pos_set")
               (3 "")
               (3 wrong)))
            (changeUri
             controller
             "gid-uri"
             2
             '("old-a"
               "old-b")
             '("new-a"))
            (changeUri
             controller
             "gid-uri"
             1
             []
             ["new-a"
              "new-b"]
             7)
            (getOption
             controller
             "gid-option")
            (changeOption
             controller
             "gid-option"
             '(("max-download-limit" . "1M")
               ("dir" . "/target")))
            (getGlobalOption
             controller)
            (changeGlobalOption
             controller
             '(("max-overall-download-limit" . "5M")))
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ((((4) :ok #1=("gid-pos" 4 "POS_CUR")) ((0 "POS_SET") :ok #2=("gid-pos" 0 "POS_SET")) ((-2 "POS_CUR") :ok #3=("gid-pos" -2 "POS_CUR")) ((-1 "POS_END") :ok #4=("gid-pos" -1 "POS_END")) ((3 "pos_set") :error aria2-err-no-such-position-type ("pos_set")) ((3 "") :error aria2-err-no-such-position-type ("")) ((3 wrong) :error aria2-err-no-such-position-type (wrong))) #5=("gid-uri" 2 ("old-a" "old-b") ("new-a") 0) #6=("gid-uri" 1 [] ["new-a" "new-b"] 7) #7=("gid-option") #8=("gid-option" (("max-download-limit" . "1M") ("dir" . "/target"))) nil #9=((("max-overall-download-limit" . "5M"))) ((t "aria2.changePosition" #1#) (t "aria2.changePosition" #2#) (t "aria2.changePosition" #3#) (t "aria2.changePosition" #4#) (t "aria2.changeUri" #5#) (t "aria2.changeUri" #6#) (t "aria2.getOption" #7#) (t "aria2.changeOption" #8#) (t "aria2.getGlobalOptions" nil) (t "aria2.changeGlobalOption" #9#)))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_global_result_session_and_inspection_api_maps_every_remaining_rpc_method() {
    let elisp_form = r##"(let ((controller
                (aria2-test-controller))
               calls)
         (cl-letf
             (((symbol-function
                'make-request)
               (lambda (this method &rest params)
                 (push
                  (list
                   (eq this controller)
                   method
                   params)
                  calls)
                 (concat
                  "result:"
                  method))))
           (list
            (getGlobalStat controller)
            (purgeDownloadResult controller)
            (removeDownloadResult
             controller
             "gid-removed")
            (saveSession controller)
            (getUris
             controller
             "gid-inspect")
            (getFiles
             controller
             "gid-inspect")
            (getPeers
             controller
             "gid-inspect")
            (getServers
             controller
             "gid-inspect")
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("result:aria2.getGlobalStat" "result:aria2.purgeDownloadResult" "result:aria2.removeDownloadResult" "result:aria2.saveSession" "result:aria2.getUris" "result:aria2.getFiles" "result:aria2.getPeers" "result:aria2.getServers" ((t "aria2.getGlobalStat" nil) (t "aria2.purgeDownloadResult" nil) (t "aria2.removeDownloadResult" ("gid-removed")) (t "aria2.saveSession" nil) (t "aria2.getUris" ("gid-inspect")) (t "aria2.getFiles" ("gid-inspect")) (t "aria2.getPeers" ("gid-inspect")) (t "aria2.getServers" ("gid-inspect"))))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_shutdown_only_calls_rpc_for_running_process_and_always_resets_running_pid() {
    let elisp_form = r##"(let ((controller
                (aria2-test-controller
                 0
                 812))
               running
               calls)
         (cl-letf
             (((symbol-function
                'is-process-running)
               (lambda (this)
                 (push
                  (list
                   :running
                   (eq this controller)
                   running)
                  calls)
                 running))
              ((symbol-function
                'make-request)
               (lambda (this method &rest params)
                 (push
                  (list
                   :request
                   (eq this controller)
                   method
                   params)
                  calls)
                 :stopping)))
           (list
            (shutdown controller)
            (oref controller pid)
            (progn
              (setq running t)
              (oset controller pid 813)
              (shutdown controller))
            (oref controller pid)
            (progn
              (setq running t)
              (oset controller pid 814)
              (shutdown controller t))
            (oref controller pid)
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (nil 812 -1 -1 -1 -1 ((:running t nil) (:running t t) (:request t "aria2.shutdown" nil) (:running t t) (:request t "aria2.forceShutdown" nil)))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

use expect_test::expect;

use super::{assert_aria2_autoload_parity, assert_aria2_parity};

#[test]
fn aria2_exact_pin_descriptor_dependency_origin_and_loaded_features_match() {
    let elisp_form = r##"(let ((descriptor
                (cadr
                 (assq
                  'aria2
                  package-alist))))
         (list
          (package-desc-name
           descriptor)
          (package-version-join
           (package-desc-version
            descriptor))
          (package-desc-summary
           descriptor)
          (package-desc-kind
           descriptor)
          (package-desc-reqs
           descriptor)
          (package-desc-extras
           descriptor)
          (mapcar
           #'featurep
           '(aria2
             eieio-base
             json
             url
             subr-x
             tabulated-list))))"##;
    let expect = expect![[
        r#"OK (aria2 "20230314.2131" "Control aria2c commandline tool from Emacs." nil ((emacs (25 1))) ((:maintainers ("ukasz Gruner" . "lukasz@gruner.lu")) (:authors ("ukasz Gruner" . "lukasz@gruner.lu")) (:keywords "download" "bittorrent" "aria2") (:revdesc . "1f2cbe624f3a") (:commit . "1f2cbe624f3a4e0109b5dc123bb4bbed496b15a7") (:url . "https://bitbucket.org/ukaszg/aria2-mode")) (t t t t t t))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_installed_payload_has_exact_inventory_sizes_and_content_digests() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'aria2
                    package-alist)))
                 (directory
                  (package-desc-dir
                   descriptor)))
         (mapcar
          (lambda (file)
            (let ((path
                   (expand-file-name
                    file
                    directory)))
              (list
               file
               (file-attribute-size
                (file-attributes
                 path))
               (secure-hash
                'sha256
                path))))
          (sort
           (seq-filter
            (lambda (file)
              (file-regular-p
               (expand-file-name
                file
                directory)))
            (directory-files
             directory
             nil
             "\\`[^.]"))
           #'string<)))"##;
    let expect = expect![[
        r#"OK (("aria2-autoloads.el" 786 "dc87bb7d386b0c695a225a18955b094c8e7e672cff56040f0979dfb0eeadb05f") ("aria2-pkg.el" 444 "211843fee3c2ac4fb44f7d9049f08f1e1df6f70503f15ff927ee4a8e95cdff64") ("aria2.el" 38620 "e8172d7b4d505764d73111422bbb0212812c269f43f58fe7b647de86b721a087") ("aria2.elc" 41550 "03e6558820d4578095cfd88dd03db9fd71212f0f95c29da8544ecb569de2ea48"))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_complete_callable_generic_command_arglist_doc_and_source_surface_matches() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (fboundp symbol)
            (commandp symbol)
            (interactive-form
             symbol)
            (help-function-arglist
             symbol
             t)
            (let ((doc
                   (documentation
                    symbol
                    t)))
              (and
               doc
               (secure-hash
                'sha256
                doc)))
            (let ((file
                   (symbol-file
                    symbol
                    'defun)))
              (and
               file
               (file-name-nondirectory
                file)))))
         '(aria2--url
           aria2--base64-encode-file
           aria2--is-aria-process-p
           aria2--decode-error
           get-next-id
           is-process-running
           run-process
           make-request
           addUri
           addTorrent
           addMetalink
           remove-download
           pause
           pauseAll
           unpause
           unpauseAll
           tellStatus
           tellActive
           tellWaiting
           tellStopped
           changePosition
           changeUri
           getOption
           changeOption
           getGlobalOption
           changeGlobalOption
           getGlobalStat
           purgeDownloadResult
           removeDownloadResult
           saveSession
           shutdown
           getUris
           getFiles
           getPeers
           getServers
           aria2--list-entries-File
           aria2--list-entries-Status
           aria2--list-entries-Type
           aria2--list-entries-Done
           aria2--list-entries-Download
           aria2--list-entries-Upload
           aria2--list-entries-Size
           aria2--list-entries-Err
           aria2--list-entries
           aria2--manage-refresh-timer
           aria2--stop-timer
           aria2--refresh
           aria2--persist-settings-on-exit
           aria2--kill-on-exit
           aria2-maybe-add-evil-quirks
           aria2--is-paused-p
           aria2-pause
           aria2-resume
           aria2-toggle-pause
           aria2--supported-file-type-p
           aria2-add-file
           aria2-dialog-cancel
           aria2-dialog-submit
           aria2-dialog-mode
           aria2-add-uris
           aria2-remove-download
           aria2-clean-removed-download
           aria2-move-up-in-list
           aria2-move-down-in-list
           aria2-terminate
           aria2-mode
           aria2-downloads-list))"##;
    let expect = expect![[
        r#"OK ((aria2--url t nil nil nil nil "aria2.el") (aria2--base64-encode-file t nil nil (path) "6ca2cdc508d0179acc6705ddf720ea7951795a0a7256a68d7d2f852b8708457e" "aria2.el") (aria2--is-aria-process-p t nil nil (pid) "56059982b8b13531e09a3f77a95cf2669623f0182f916d50bd0ca80957ab7684" "aria2.el") (aria2--decode-error t nil nil (err) nil "aria2.el") (get-next-id t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (is-process-running t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (run-process t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (make-request t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (addUri t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (addTorrent t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (addMetalink t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (remove-download t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (pause t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (pauseAll t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (unpause t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (unpauseAll t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (tellStatus t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (tellActive t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (tellWaiting t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (tellStopped t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (changePosition t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (changeUri t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (getOption t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (changeOption t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (getGlobalOption t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (changeGlobalOption t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (getGlobalStat t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (purgeDownloadResult t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (removeDownloadResult t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (saveSession t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (shutdown t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (getUris t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (getFiles t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (getPeers t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (getServers t nil nil (&rest args) "e48e292a59807ade8aaec3980a2d0865adf10702ae8db88840ba2169ec0d5cc2" "aria2.el") (aria2--list-entries-File t nil nil (e) nil "aria2.el") (aria2--list-entries-Status t nil nil (e) nil "aria2.el") (aria2--list-entries-Type t nil nil (e) nil "aria2.el") (aria2--list-entries-Done t nil nil (e) nil "aria2.el") (aria2--list-entries-Download t nil nil (e) nil "aria2.el") (aria2--list-entries-Upload t nil nil (e) nil "aria2.el") (aria2--list-entries-Size t nil nil (e) nil "aria2.el") (aria2--list-entries-Err t nil nil (e) nil "aria2.el") (aria2--list-entries t nil nil nil "69718714f102892434330de9c5c32515318ae872fa7c2de8f08a69424267b50b" "aria2.el") (aria2--manage-refresh-timer t nil nil nil "3b8108678fb71a1b275f60f244bdf8206b9b062c2a3d626ad78f665cf7b885c9" "aria2.el") (aria2--stop-timer t nil nil nil "dc6b083e2b0f4d7e2c342ccdf390c100996c68b8313b856db0e25916faf97759" "aria2.el") (aria2--refresh t nil nil nil "f9737a25f22f35837378c283e0b6cda1a1c15bb97bfdfc9719856c9b1836b4db" "aria2.el") (aria2--persist-settings-on-exit t nil nil nil "d25634672e6ccad3c2fa8a1858b8c232ae4030c2f2657e1c4921ac5214f5caba" "aria2.el") (aria2--kill-on-exit t nil nil nil "18949f36dbc0931c9cd4871bda6ff0f54c7145905c3311b7156a80f15833f8e6" "aria2.el") (aria2-maybe-add-evil-quirks t nil nil nil "171494ea2c384a342af458d189bf22b25298ed36ed0749a988e8022acdf4a80f" "aria2.el") (aria2--is-paused-p t nil nil nil nil "aria2.el") (aria2-pause t t (interactive nil) nil "c4069dbd6ecc44203a5d468cafd9514aab315698beb01f7b53f32f2c93e34e84" "aria2.el") (aria2-resume t t (interactive nil) nil "50ccbc45956b01a6373b1fb43358ae0679f86aa40797bfd3784de121212a1097" "aria2.el") (aria2-toggle-pause t t (interactive nil) nil "1610d740cacbef17cba65967609ee20b8678e2ea83fdeaf873c1939bc82b7745" "aria2.el") (aria2--supported-file-type-p t nil nil (f) "c674c12f9e51665de09bdb61b2aa008c8c624ffbabde8cf7853fd1c10677cce3" "aria2.el") (aria2-add-file t t (interactive "P") (arg) "4a75d2e266d8a03db20273302d903e2ae320510a4b68400fe16d3b65535282b7" "aria2.el") (aria2-dialog-cancel t t (interactive nil) nil nil "aria2.el") (aria2-dialog-submit t t (interactive nil) nil nil "aria2.el") (aria2-dialog-mode t t (interactive nil) nil "7fd39ec766d05a6ce2faf3fcc16bf2b7aca39408c33a96f1d304d218d2799dba" "aria2.el") (aria2-add-uris t t (interactive nil) nil "f5e09d0c50832bf4278856a31ce714553861b07ee1b5bcb282155b251cafe76f" "aria2.el") (aria2-remove-download t t (interactive "P") (arg) "e44181f779f85425d9615257cb9dcd8dec6a9d78e330e6e8e4fa6391ccde31b4" "aria2.el") (aria2-clean-removed-download t t (interactive "P") (arg) "444c3a08a72e617b50f2e2c61167aa91682835fa350da658736bf7d303ad1113" "aria2.el") (aria2-move-up-in-list t t (interactive "P") (arg) "de05a79fd9220c002ee8f672a0493030d9ba904fd64e508f2e2f9f51c6a58970" "aria2.el") (aria2-move-down-in-list t t (interactive "P") (arg) "fc6f7dbd863032c282d44621435ab9d4252a2d7e5a519ebc1cced99d2bb366c9" "aria2.el") (aria2-terminate t t (interactive nil) nil "13a0e256c7e73ac0dedb2569928bb490c2354943f6fba8c2253f696992b3a0b5" "aria2.el") (aria2-mode t t (interactive nil) nil "7e493da3657fffb89668e8833b342a930cb63f468f30fdebe9c4349dde2bf112" "aria2.el") (aria2-downloads-list t t (interactive nil) nil "8cc1ea7a7bc9b52b5f2b1915b250c78b434998ad612fa68d872f256300713117" "aria2.el"))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_complete_custom_variable_contracts_match_with_nondeterministic_secret_normalized() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (let ((value
                  (symbol-value
                   symbol)))
             (list
              symbol
              (cond
               ((eq symbol 'aria2-rcp-secret)
                (list
                 (type-of value)
                 (and
                  (stringp value)
                  (length value))))
               ((eq symbol 'aria2-executable)
                (and
                 value
                 (file-name-nondirectory
                  value)))
               (t value))
              (get symbol 'custom-type)
              (get symbol 'custom-group)
              (get symbol 'standard-value)
              (custom-variable-p
               symbol)
              (local-variable-if-set-p
               symbol)
              (let ((doc
                     (documentation-property
                      symbol
                      'variable-documentation
                      t)))
                (and
                 doc
                 (secure-hash
                  'sha256
                  doc))))))
         '(aria2-start-rpc-server
           aria2-kill-process-on-emacs-exit
           aria2-list-buffer-name
           aria2-executable
           aria2-session-file
           aria2-download-directory
           aria2-rcp-listen-port
           aria2-rcp-secret
           aria2-custom-args
           aria2-add-evil-quirks
           aria2-cc-file
           aria2-refresh-fast
           aria2-refresh-normal
           aria2-refresh-slow
           aria2-mode-hook))"##;
    let expect = expect![[
        r#"OK ((aria2-start-rpc-server nil boolean nil #1=(nil) #1# nil "474e645b931695c72974ba7dec084c453830ea6399528c65d69a180f3ec90920") (aria2-kill-process-on-emacs-exit nil boolean nil #2=(nil) #2# nil "51233ac5fdad28bf6c50699422ac125b6e6ae275f79490610b836b11e8e58313") (aria2-list-buffer-name "*aria2: downloads list*" (string :tag "Buffer name") nil #3=("*aria2: downloads list*") #3# nil "c31f374e8dafa98661b150ef43dfb9c1b5312ad2a39c7d726a7083be6fd389db") (aria2-executable nil file nil #4=((executable-find "aria2c")) #4# nil "e59f128e23bc5b60217267d255a9a6d23431fa99053ba02830629920f2aed145") (aria2-session-file "[ORACLE-HOME]/.emacs.d/aria2c.session" file nil #5=((expand-file-name "aria2c.session" user-emacs-directory)) #5# nil "7a0062926937fcdc5f428ffbd306a0ff73fefc741e721f77dc292b203c77af5b") (aria2-download-directory "[ORACLE-HOME]/" directory nil #6=((or (getenv "XDG_DOWNLOAD_DIR") (expand-file-name "~/"))) #6# nil "f3efc018f3ec714c042cfd05898b961137d99923b3a249197220b3c17975bb26") (aria2-rcp-listen-port 6800 (integer :tag "Http port") nil #7=(6800) #7# nil "0fe206f5981ebac6d91cbee81f91765cd235ae49533c8161b59f2c1febc61b4c") (aria2-rcp-secret (string 36) (integer :tag "Http port") nil #8=((or (let ((uuidgen (executable-find "uuidgen"))) (and uuidgen (string-trim (shell-command-to-string uuidgen)))) (sha1 (format "%s%s%s%s%s%s%s%s%s" (user-uid) (emacs-pid) (system-name) (user-full-name) (current-time) (emacs-uptime) (buffer-string) (random) (recent-keys))))) #8# nil "5d85d6626e178a6b4b7af98a40c278396f0230da163da9c3a9abd81616c308ae") (aria2-custom-args nil (repeat (string :tag "Commandline argument.")) nil #9=(nil) #9# nil "4d892a621a4e5add4a476c909899665cf84858ea296432ee7af208cd2e1592bc") (aria2-add-evil-quirks nil nil nil #10=(nil) #10# nil "f547b32a4dfb4835e81802690478445925be55c5627923f883c32c65ac73d0c7") (aria2-cc-file "[ORACLE-HOME]/.emacs.d/aria2-controller.eieio" file nil #11=((expand-file-name "aria2-controller.eieio" user-emacs-directory)) #11# nil "d17828d324a63fe0a7fe3aeaf755ceab03b8698db5c5db8cf09dfd2e328f12df") (aria2-refresh-fast 3 (integer :tag "Number of seconds") nil #12=(3) #12# nil "9447067dd271af67a9e6c9d31b8398ef9151094ccb56113a7064765678f0bbf9") (aria2-refresh-normal 8 (integer :tag "Number of seconds") nil #13=(8) #13# nil "2ba6e6ba33708f3cc4422dc91725d1d2c18d0ba03cd7e571c66a6bda0516d292") (aria2-refresh-slow 20 (integer :tag "Number of seconds") nil #14=(20) #14# nil "82561f7a3244ef836381fe093d41de6c89e1300385452c636e96a1b4ed32a9f8") (aria2-mode-hook nil hook nil #15=(nil) #15# nil "f3f510ba1c248daf157a05304522acdfa907843ce87a116544482f4d7d4fb781"))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_constants_errors_faces_and_internal_state_defaults_match() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (symbol-value
              symbol)
             (documentation-property
              symbol
              'variable-documentation
              t)))
          '(aria2--debug
            aria2--codes-to-errors-alist
            aria2--cc
            aria2--list-format
            aria2--tell-keys
            aria2--master-timer
            aria2--refresh-timer
            aria2--current-buffer-refresh-speed
            aria2-supported-file-extension-regexp
            aria2-supported-url-protocols-regexp
            aria2-url-list-buffer-name
            aria2--url-list-widget))
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (get symbol 'error-conditions)
             (get symbol 'error-message)))
          '(aria2-err-too-many-magnet-urls
            aria2-err-file-doesnt-exist
            aria2-err-not-a-torrent-file
            aria2-err-not-a-metalink-file
            aria2-err-failed-to-start
            aria2-err-no-executable
            aria2-err-no-such-position-type))
         (mapcar
          (lambda (face)
            (list
             face
             (facep face)
             (face-attribute
              face
              :inherit
              nil
              'default)))
          '(aria2-file-face
            aria2-status-face
            aria2-type-face
            aria2-done-face
            aria2-download-face
            aria2-upload-face
            aria2-error-face
            aria2-modeline-key-face
            aria2-modeline-mouse-face)))"##;
    let expect = expect![[
        r#"OK (((aria2--debug nil "Should json commands and replies be printed.") (aria2--codes-to-errors-alist (("0" . "All downloads were successful") ("1" . "An unknown error occurred") ("2" . "Time out occurred") ("3" . "A resource was not found") ("4" . "Aria2 saw the specified number of \"resource not found\" error. See --max-file-not-found option") ("5" . "A download aborted because download speed was too slow. See --lowest-speed-limit option") ("6" . "Network problem occurred") ("7" . "There were unfinished downloads") ("8" . "Remote server did not support resume when resume was required to complete download") ("9" . "There was not enough disk space available") ("10" . "Piece length was different from one in .aria2 control file. See --allow-piece-length-change option") ("11" . "Aria2 was downloading same file at that moment") ("12" . "Aria2 was downloading same info hash torrent at that moment") ("13" . "File already existed. See --allow-overwrite option") ("14" . "Renaming file failed. See --auto-file-renaming option") ("15" . "Aria2 could not open existing file") ("16" . "Aria2 could not create new file or truncate existing file") ("17" . "File I/O error occurred") ("18" . "Aria2 could not create directory") ("19" . "Name resolution failed") ("20" . "Aria2 could not parse Metalink document") ("21" . "FTP command failed") ("22" . "HTTP response header was bad or unexpected") ("23" . "Too many redirects occurred") ("24" . "HTTP authorization failed") ("25" . "Aria2 could not parse bencoded file (usually \".torrent\" file)") ("26" . "A \".torrent\" file was corrupted or missing information that aria2 needed") ("27" . "Magnet URI was bad") ("28" . "Bad/unrecognized option was given or unexpected option argument was given") ("29" . "The remote server was unable to handle the request due to a temporary overloading or maintenance") ("30" . "Aria2 could not parse JSON-RPC request")) "Mapping of aria2 error codes to error messages.") (aria2--cc nil "Control center object container.") (aria2--list-format [("File" 40 t) ("Status" 7 t) ("Type" 13 t) ("Done" 4 t) ("Download" 12 t) ("Upload" 12 t) ("Size" 10 nil) ("Error" 0 nil)] "Format for downloads list columns.") (aria2--tell-keys ["gid" "status" "totalLength" "completedLength" "downloadSpeed" "uploadSpeed" "files" "dir" "bittorrent" "errorCode"] "Default list of keys for use in aria2.tell* calls.") (aria2--master-timer nil "Holds a timer object that dynamically manages frequency of `aria2--refresh-timer', depending on visibility and focused state.") (aria2--refresh-timer nil "Holds a timer object that refreshes downloads list periodically.") (aria2--current-buffer-refresh-speed nil "One of :fast :normal :slow or nil if not refreshing. Used to manage refresh timers.") (aria2-supported-file-extension-regexp "\\.\\(?:meta\\(?:4\\|link\\)\\|torrent\\)$" "Regexp matching .torrent .meta4 and .metalink files.") (aria2-supported-url-protocols-regexp "\\(?:ftp://\\|http\\(?:s?://\\)\\|magnet:\\)" "Regexp matching frp, http, https and magnet urls.") (aria2-url-list-buffer-name "*aria2: Add http/https/ftp/magnet url(s)*" "Name of a buffer for inputting url's to download.") (aria2--url-list-widget nil nil)) ((aria2-err-too-many-magnet-urls (aria2-err-too-many-magnet-urls user-error error) "Only one magnet link per download is allowed") (aria2-err-file-doesnt-exist (aria2-err-file-doesnt-exist user-error error) "File doesn't exist") (aria2-err-not-a-torrent-file (aria2-err-not-a-torrent-file user-error error) "This is not a .torrent file") (aria2-err-not-a-metalink-file (aria2-err-not-a-metalink-file user-error error) "This is not a .metalink file") (aria2-err-failed-to-start (aria2-err-failed-to-start error) "Failed to start") (aria2-err-no-executable (aria2-err-no-executable error) "Couldn't find `aria2c' executable, aborting") (aria2-err-no-such-position-type (aria2-err-no-such-position-type error) "Wrong position type")) ((aria2-file-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] mode-line-buffer-id) (aria2-status-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] font-lock-constant-face) (aria2-type-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] font-lock-builtin-face) (aria2-done-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] font-lock-doc-face) (aria2-download-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] font-lock-string-face) (aria2-upload-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] font-lock-comment-face) (aria2-error-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] font-lock-warning-face) (aria2-modeline-key-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] font-lock-warning-face) (aria2-modeline-mouse-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] default)))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_controller_class_inheritance_slots_defaults_and_explicit_instance_values_match() {
    let elisp_form = r##"(let ((default
                (make-instance
                 'aria2-controller
                 "default-controller"
                 :file
                 (aria2-test-path
                  "default.eieio")))
               (explicit
                (aria2-test-controller
                 7
                 4321)))
         (list
          (child-of-class-p
           'aria2-controller
           'eieio-persistent)
          (object-class-name
           default)
          (mapcar
           (lambda (slot)
             (list
              slot
              (slot-boundp
               default
               slot)
              (let ((value
                     (slot-value
                      default
                      slot)))
                (if
                    (eq slot 'secret)
                    (list
                     (type-of value)
                     (length value))
                  value))
              (slot-value
               explicit
               slot)))
           '(request-id
             rcp-url
             secret
             pid))
          (file-name-nondirectory
           (oref explicit file))))"##;
    let expect = expect![[
        r#"OK (t aria2-controller ((request-id t 0 7) (rcp-url t "http://localhost:6800/jsonrpc" "http://fixture.invalid:6800/jsonrpc") (secret t (string 36) "fixture-secret") (pid t -1 4321)) "controller.eieio")"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_autoload_and_keymap_ui_contracts_match_without_loading_source() {
    let elisp_form = r##"(list
         (featurep
          'aria2)
         (let ((function
                (symbol-function
                 'aria2-downloads-list)))
           (list
            (fboundp
             'aria2-downloads-list)
            (autoloadp function)
            (and
             (autoloadp function)
             (nth 1 function))
            (and
             (autoloadp function)
             (nth 4 function))
            (commandp
             'aria2-downloads-list)
            (interactive-form
             'aria2-downloads-list)))
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (fboundp symbol)
             (commandp symbol)))
          '(aria2-mode
            aria2-add-file
            aria2-add-uris)))"##;
    let expect = expect![[
        r#"OK (nil (t t "aria2" nil t (interactive nil)) ((aria2-mode t t) (aria2-add-file t t) (aria2-add-uris t t)))"#
    ]];

    assert_aria2_autoload_parity(elisp_form, expect);
}

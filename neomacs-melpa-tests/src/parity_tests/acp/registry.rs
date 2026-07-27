use expect_test::expect;

use super::{
    assert_acp_autoload_parity, assert_acp_fakes_parity, assert_acp_parity,
    assert_acp_traffic_parity,
};

#[test]
fn acp_exact_pin_metadata_features_constants_group_and_custom_option_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'acp
                      package-alist)))
                   (standard
                    (get
                     'acp-logging-enabled
                     'standard-value)))
         (list
          (package-desc-name descriptor)
          (package-version-join
           (package-desc-version descriptor))
          (package-desc-reqs descriptor)
          (package-desc-summary descriptor)
          (copy-tree
           (package-desc-extras descriptor))
          acp-package-version
          acp--jsonrpc-version
          (featurep
           'acp)
          (featurep
           'acp-traffic)
          (featurep
           'acp-fakes)
          (get
           'acp
           'group-documentation)
          (copy-tree
           (get
            'acp
            'custom-group))
          (and
           (member
            '(acp custom-group)
            (get
             'tools
             'custom-group))
           t)
          acp-logging-enabled
          (default-value
           'acp-logging-enabled)
          (list
           (and standard t)
           (eval
            (car standard)
            t))
          (get
           'acp-logging-enabled
           'custom-type)
          (documentation-property
           'acp-logging-enabled
           'variable-documentation
           t)))"##;
    let expect = expect![[
        r#"OK (acp "20260719.342" ((emacs (28 1))) "An ACP (Agent Client Protocol) implementation." ((:revdesc . "a29cb161ac95") (:commit . "a29cb161ac95f1819f34481a98666707661c5cf8") (:url . "https://github.com/xenodium/acp.el")) "0.13.1" "2.0" t t nil "ACP (Agent Client Protocol) implementation." ((acp-logging-enabled custom-variable)) t nil nil (t nil) boolean "Whether to log ACP traffic and messages to the logs buffer.\nSee `acp-logs-buffer' to view the resulting log.")"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_and_traffic_callable_surface_arglists_commands_docs_and_sources_match() {
    let elisp_form = r##"(progn
         (load
          (expand-file-name
           "acp-traffic.el"
           (file-name-directory
            (getenv
             "NEOMACS_PACKAGE_SOURCE")))
          nil t t)
         (mapcar
         (lambda (symbol)
           (list
            symbol
            (and
             (fboundp symbol)
             t)
            (help-function-arglist
             symbol
             t)
            (and
             (commandp symbol)
             t)
            (interactive-form
             symbol)
            (car
             (split-string
              (documentation
               symbol
               t)
              "\n"))
            (let ((file
                   (symbol-file
                    symbol
                    'defun)))
              (and file
                   (file-name-nondirectory
                    file)))))
         '(acp-make-client
           acp--client-started-p
           acp--start-client
           acp--make-internal-error
           acp--call-request-failure
           acp--fail-pending-requests
           acp-subscribe-to-notifications
           acp-subscribe-to-requests
           acp-subscribe-to-errors
           acp-shutdown
           acp-send-request
           acp-send-notification
           acp--request-sender
           acp-send-response
           acp--response-sender
           acp--notification-sender
           acp-make-initialize-request
           acp-make-authenticate-request
           acp-make-session-new-request
           acp-make-session-prompt-request
           acp-make-session-set-mode-request
           acp-make-session-set-model-request
           acp-make-session-set-config-option-request
           acp-make-session-resume-request
           acp-make-session-fork-request
           acp-make-session-list-request
           acp-make-session-load-request
           acp-make-session-delete-request
           acp-make-session-request-permission-response
           acp-make-fs-read-text-file-response
           acp-make-fs-write-text-file-response
           acp-make-error
           acp-make-session-cancel-notification
           acp--request-resolver
           acp--make-message
           acp--route-incoming-message
           acp--parse-stderr-api-error
           acp--format-log-message
           acp--insert-log-entry
           acp--log
           acp--total-buffer-bytes
           acp--trim-log-buffer
           acp--json-pretty-print
           acp--log-traffic
           acp--show-json-object
           acp-reset-logs
           acp-logs-buffer
           acp-traffic-buffer
           acp--increment-instance-count
           acp--parse-json
           acp--serialize-json
           acp-traffic-save-to
           acp-traffic-read-file
           acp-traffic-open-file
           acp-traffic-next-entry
           acp-traffic-previous-entry
           acp-traffic-display-entry
           acp-traffic-display-all-entries
           acp-traffic-get-buffer
           acp-traffic--update-line-highlight
           acp-traffic-mode
           acp-traffic-log-traffic
           acp-traffic--objects
           acp-traffic-entry-mode
           acp-traffic-entry-next
           acp-traffic-entry-previous
           acp-traffic-display-objects
           acp-traffic-display-objects-helper
           acp-traffic-display-max-key-width
           acp-traffic-display-format-value)))"##;
    let expect = expect![[
        r#"OK ((acp-make-client t (&rest --cl-rest--) nil nil "Create an ACP client." "acp.el") (acp--client-started-p t (client) nil nil "Return non-nil if CLIENT process has been started." "acp.el") (acp--start-client t (&rest --cl-rest--) nil nil "Start CLIENT." "acp.el") (acp--make-internal-error t (message) nil nil "Build a synthetic JSON-RPC-shaped error alist with MESSAGE." "acp.el") (acp--call-request-failure t (&rest --cl-rest--) nil nil "Invoke INCOMING-RESPONSE's failure callback with ERROR-DATA and MESSAGE." "acp.el") (acp--fail-pending-requests t (&rest --cl-rest--) nil nil "Invoke `:on-failure' for any pending requests on CLIENT." "acp.el") (acp-subscribe-to-notifications t (&rest --cl-rest--) nil nil "Subscribe to incoming CLIENT notifications." "acp.el") (acp-subscribe-to-requests t (&rest --cl-rest--) nil nil "Subscribe to incoming CLIENT requests." "acp.el") (acp-subscribe-to-errors t (&rest --cl-rest--) nil nil "Subscribe to agent errors using CLIENT." "acp.el") (acp-shutdown t (&rest --cl-rest--) nil nil "Shutdown ACP CLIENT and release resources." "acp.el") (acp-send-request t (&rest --cl-rest--) nil nil "Send REQUEST from CLIENT." "acp.el") (acp-send-notification t (&rest --cl-rest--) nil nil "Send NOTIFICATION from CLIENT." "acp.el") (acp--request-sender t (&rest --cl-rest--) nil nil "Send REQUEST from CLIENT." "acp.el") (acp-send-response t (&rest --cl-rest--) nil nil "Send a request RESPONSE from CLIENT." "acp.el") (acp--response-sender t (&rest --cl-rest--) nil nil "Send a request RESPONSE from CLIENT." "acp.el") (acp--notification-sender t (&rest --cl-rest--) nil nil "Send NOTIFICATION from CLIENT." "acp.el") (acp-make-initialize-request t (&rest --cl-rest--) nil nil "Instantiate an \"initialize\" request." "acp.el") (acp-make-authenticate-request t (&rest --cl-rest--) nil nil "Instantiate an \"authenticate\" request." "acp.el") (acp-make-session-new-request t (&rest --cl-rest--) nil nil "Instantiate a \"session/new\" request." "acp.el") (acp-make-session-prompt-request t (&rest --cl-rest--) nil nil "Instantiate a \"session/prompt\" request." "acp.el") (acp-make-session-set-mode-request t (&rest --cl-rest--) nil nil "Instantiate a \"session/set_mode\" request." "acp.el") (acp-make-session-set-model-request t (&rest --cl-rest--) nil nil "Instantiate a \"session/set_model\" request." "acp.el") (acp-make-session-set-config-option-request t (&rest --cl-rest--) nil nil "Instantiate a \"session/set_config_option\" request." "acp.el") (acp-make-session-resume-request t (&rest --cl-rest--) nil nil "Instantiate a \"session/resume\" request." "acp.el") (acp-make-session-fork-request t (&rest --cl-rest--) nil nil "Instantiate a \"session/fork\" request." "acp.el") (acp-make-session-list-request t (&rest --cl-rest--) nil nil "Instantiate a \"session/list\" request." "acp.el") (acp-make-session-load-request t (&rest --cl-rest--) nil nil "Instantiate a \"session/load\" request." "acp.el") (acp-make-session-delete-request t (&rest --cl-rest--) nil nil "Instantiate a \"session/delete\" request." "acp.el") (acp-make-session-request-permission-response t (&rest --cl-rest--) nil nil "Instantiate a \"session/request_permission\" response." "acp.el") (acp-make-fs-read-text-file-response t (&rest --cl-rest--) nil nil "Instantiate a \"fs/read_text_file\" response." "acp.el") (acp-make-fs-write-text-file-response t (&rest --cl-rest--) nil nil "Instantiate a \"fs/write_text_file\" response." "acp.el") (acp-make-error t (&rest --cl-rest--) nil nil "Create a JSON-RPC error object." "acp.el") (acp-make-session-cancel-notification t (&rest --cl-rest--) nil nil "Instantiate a \"session/cancel\" request." "acp.el") (acp--request-resolver t (&rest --cl-rest--) nil nil "Resolve CLIENT request with ID to a handler." "acp.el") (acp--make-message t (&rest --cl-rest--) nil nil "Create message with JSON and OBJECT." "acp.el") (acp--route-incoming-message t (&rest --cl-rest--) nil nil "Parse CLIENT's incoming MESSAGE with json/object and route accordingly." "acp.el") (acp--parse-stderr-api-error t (raw-output) nil nil "Parse RAW-OUTPUT, typically from stderr." "acp.el") (acp--format-log-message t (label format-string &rest args) nil nil "Return a log message formatted like `acp--log'." "acp.el") (acp--insert-log-entry t (label format-string &rest args) nil nil "Insert a log message at point and add a boundary marker." "acp.el") (acp--log t (client label format-string &rest args) nil nil "Log CLIENT message using LABEL, FORMAT-STRING, and ARGS." "acp.el") (acp--total-buffer-bytes t (buffer) nil nil "Return the total number of bytes in BUFFER." "acp.el") (acp--trim-log-buffer t (buffer &optional max-bytes) nil nil "Trim BUFFER to a maximum size in bytes at log message boundaries." "acp.el") (acp--json-pretty-print t (json) nil nil "Return a pretty-printed JSON string." "acp.el") (acp--log-traffic t (client direction kind message) nil nil "Log CLIENT traffic MESSAGE to \"*acp traffic*\" buffer." "acp.el") (acp--show-json-object t (object) nil nil "Display OBJECT in a pretty-printed buffer." "acp.el") (acp-reset-logs t (&rest --cl-rest--) nil nil "Reset CLIENT log buffers." "acp.el") (acp-logs-buffer t (&rest --cl-rest--) nil nil "Get CLIENT logs buffer." "acp.el") (acp-traffic-buffer t (&rest --cl-rest--) nil nil "Get CLIENT traffic buffer." "acp.el") (acp--increment-instance-count t nil nil nil "Increment variable `acp-instance-count'." "acp.el") (acp--parse-json t (json) nil nil "Parse JSON using a consistent configuration." "acp.el") (acp--serialize-json t (object) nil nil "Serialize OBJECT to JSON using a consistent configuration." "acp.el") (acp-traffic-save-to t nil t (interactive nil) "Save traffic objects to a file." "acp-traffic.el") (acp-traffic-read-file t (traffic-file) nil nil "Read TRAFFIC-FILE into message objects." "acp-traffic.el") (acp-traffic-open-file t nil t (interactive nil) "Select and open a traffic file." "acp-traffic.el") (acp-traffic-next-entry t nil t (interactive nil) "Move to next traffic entry." "acp-traffic.el") (acp-traffic-previous-entry t nil t (interactive nil) "Move to previous traffic entry." "acp-traffic.el") (acp-traffic-display-entry t nil t (interactive nil) "Display expanded entry at point." "acp-traffic.el") (acp-traffic-display-all-entries t nil t (interactive nil) "Display all entries expanded." "acp-traffic.el") (acp-traffic-get-buffer t (&rest --cl-rest--) nil nil "Get or create a buffer for ACP traffic." "acp-traffic.el") (acp-traffic--update-line-highlight t nil nil nil "Update the line highlight overlay to current line." "acp-traffic.el") (acp-traffic-mode t nil t (interactive nil) "Major mode for ACP traffic monitoring." "acp-traffic.el") (acp-traffic-log-traffic t (&rest --cl-rest--) nil nil "Log MESSAGE to BUFFER." "acp-traffic.el") (acp-traffic--objects t nil nil nil "Extract all the traffic objects from current traffic buffer." "acp-traffic.el") (acp-traffic-entry-mode t nil t (interactive nil) "Major mode for ACP traffic entry display." "acp-traffic.el") (acp-traffic-entry-next t nil t (interactive nil) "Move to next traffic entry in the traffic buffer." "acp-traffic.el") (acp-traffic-entry-previous t nil t (interactive nil) "Move to previous traffic entry in the traffic buffer." "acp-traffic.el") (acp-traffic-display-objects t (objects) nil nil "Display OBJECTS." "acp-traffic.el") (acp-traffic-display-objects-helper t (object indent) nil nil "Display OBJECT with INDENT." "acp-traffic.el") (acp-traffic-display-max-key-width t (alist) nil nil "Return longest key width in ALIST." "acp-traffic.el") (acp-traffic-display-format-value t (value) nil nil "Format display of VALUE." "acp-traffic.el"))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_fakes_callable_surface_arglists_docs_and_sources_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (and
             (fboundp symbol)
             t)
            (help-function-arglist
             symbol
             t)
            (and
             (commandp symbol)
             t)
            (car
             (split-string
              (documentation
               symbol
               t)
              "\n"))
            (let ((file
                   (symbol-file
                    symbol
                    'defun)))
              (and file
                   (file-name-nondirectory
                    file)))))
         '(acp-fakes-make-client
           acp-fakes--request-sender
           acp-fakes--response-sender
           acp-fakes--request-resolver
           acp-fakes--test-fake-client
           acp-fakes-replay
           acp-fakes--get-authenticate-request
           acp-fakes--get-related-incoming-traffic))"##;
    let expect = expect![[
        r#"OK ((acp-fakes-make-client t (messages) nil "Create a fake ACP client that responds using traffic MESSAGES." "acp-fakes.el") (acp-fakes--request-sender t (&rest --cl-rest--) nil "Send request using CLIENT, ON-SUCCESS, and ON-FAILURE." "acp-fakes.el") (acp-fakes--response-sender t (&rest --cl-rest--) nil "Fake response sender." "acp-fakes.el") (acp-fakes--request-resolver t (&rest --cl-rest--) nil "Fake request resolver." "acp-fakes.el") (acp-fakes--test-fake-client t nil nil "Test a fake client." "acp-fakes.el") (acp-fakes-replay t (&rest --cl-rest--) nil "Replay messages from CLIENT's message queue." "acp-fakes.el") (acp-fakes--get-authenticate-request t (&rest --cl-rest--) nil "Find the first authentication object in MESSAGES." "acp-fakes.el") (acp-fakes--get-related-incoming-traffic t (&rest --cl-rest--) nil "Extract all the incoming MESSAGES related to incoming request with REQUEST-ID." "acp-fakes.el"))"#
    ]];
    assert_acp_fakes_parity(elisp_form, expect);
}

#[test]
fn acp_variables_modes_keymaps_and_buffer_local_contract_match() {
    let elisp_form = r##"(progn
         (load
          (expand-file-name
           "acp-traffic.el"
           (file-name-directory
            (getenv
             "NEOMACS_PACKAGE_SOURCE")))
          nil t t)
         (list
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (boundp symbol)
             (default-boundp symbol)
             (default-value symbol)
             (local-variable-if-set-p
              symbol)
             (documentation-property
              symbol
              'variable-documentation
              t)
             (let ((file
                    (symbol-file
                     symbol
                     'defvar)))
               (and file
                    (file-name-nondirectory
                     file)))))
          '(acp-instance-count
            acp--log-buffer-max-bytes
            acp-traffic-entry--traffic-buffer))
         (mapcar
          (lambda (map)
            (list
             map
             (keymapp
              (symbol-value map))
             (lookup-key
              (symbol-value map)
              (kbd "n"))
             (lookup-key
              (symbol-value map)
              (kbd "p"))
             (lookup-key
              (symbol-value map)
              (kbd "RET"))
             (lookup-key
              (symbol-value map)
              (kbd "C-x C-s"))
             (eq
              (keymap-parent
               (symbol-value map))
              special-mode-map)))
          '(acp-traffic-mode-map
            acp-traffic-entry-mode-map))
         (with-temp-buffer
           (acp-traffic-mode)
           (list
            major-mode
            mode-name
            buffer-read-only
            (local-variable-p
             'acp-traffic-entry--traffic-buffer)))
         (with-temp-buffer
           (acp-traffic-entry-mode)
           (list
            major-mode
            mode-name
            buffer-read-only
            (local-variable-p
             'acp-traffic-entry--traffic-buffer)))))"##;
    let expect = expect![[
        r#"OK (((acp-instance-count t t 0 nil nil "acp.el") (acp--log-buffer-max-bytes t t 100000000 nil "Maximum size of the log buffer in bytes." "acp.el") (acp-traffic-entry--traffic-buffer t t nil t "Buffer-local variable pointing to the associated traffic buffer." "acp-traffic.el")) ((acp-traffic-mode-map t acp-traffic-next-entry acp-traffic-previous-entry acp-traffic-display-entry acp-traffic-save-to t) (acp-traffic-entry-mode-map t acp-traffic-entry-next acp-traffic-entry-previous nil 1 t)) (acp-traffic-mode "ACP-traffic" t nil) (acp-traffic-entry-mode "ACP-traffic-entry" t nil))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_installed_package_inventory_and_content_assets_match_exactly() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'acp
                    package-alist)))
                 (directory
                  (package-desc-dir
                   descriptor))
                 (names
                  (sort
                   (directory-files
                    directory
                    nil
                    "\\`[^.]")
                   #'string<)))
         (list
          names
          (mapcar
           (lambda (name)
             (let ((path
                    (expand-file-name
                     name
                     directory)))
               (if
                   (string-suffix-p
                    ".elc"
                    name)
                   (list
                    name
                    (file-regular-p path)
                    (and
                     (>
                      (nth
                       7
                       (file-attributes
                        path))
                      0)
                     t))
                 (with-temp-buffer
                   (set-buffer-multibyte nil)
                   (insert-file-contents-literally
                    path)
                   (list
                    name
                    (file-regular-p path)
                    (buffer-size)
                    (secure-hash
                     'sha256
                     (current-buffer)))))))
           names)))"##;
    let expect = expect![[
        r#"OK (("README-elpa" "acp-autoloads.el" "acp-fakes.el" "acp-fakes.elc" "acp-pkg.el" "acp-traffic.el" "acp-traffic.elc" "acp.el" "acp.elc") (("README-elpa" t 324 "ed1121c7c454b129320fed1fe1804b9cfd8845e8a4000ad6ef7c8297c1e41b3b") ("acp-autoloads.el" t 869 "767ed2848c4ecb780da83321be8987dbdb1e6e427cf426df2910d659e484641f") ("acp-fakes.el" t 11102 "e09da4a9ec5d43044f7ab3d56c8b85b3092c1ca7b9174ae52b723fe40be82204") ("acp-fakes.elc" t t) ("acp-pkg.el" t 284 "1691c5950e79baa025a5fa291f7481e289b88d7be52cbd620c0a187693519ebd") ("acp-traffic.el" t 11860 "4f2c04b48a83ee369b820d65d66b16d86395673f8362bd8967d0bd74ca868a5a") ("acp-traffic.elc" t t) ("acp.el" t 45045 "a7177f9f376f7531e174c41fbcdf5da3746696a95f1369c0bbe76208b0038181") ("acp.elc" t t)))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_autoload_file_provides_feature_but_registers_no_callable_entries() {
    let elisp_form = r##"(list
         (featurep
          'acp)
         (featurep
          'acp-traffic)
         (featurep
          'acp-autoloads)
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (and
              (autoloadp
               (symbol-function
                symbol))
              t)
             (let ((file
                    (symbol-file
                     symbol
                     'defun)))
               (and file
                    (file-name-nondirectory
                     file)))))
          '(acp-traffic-save-to
            acp-traffic-open-file
            acp-traffic-display-all-entries
            acp-traffic-mode
            acp-traffic-entry-mode)))"##;
    let expect = expect![
        "OK (nil nil t ((acp-traffic-save-to nil nil) (acp-traffic-open-file nil nil) (acp-traffic-display-all-entries nil nil) (acp-traffic-mode nil nil) (acp-traffic-entry-mode nil nil)))"
    ];
    assert_acp_autoload_parity(elisp_form, expect);
}

#[test]
fn acp_direct_traffic_source_registers_feature_modes_and_source_ownership() {
    let elisp_form = r##"(list
         (featurep
          'acp-traffic)
         (featurep
          'acp)
         (let ((file
                (symbol-file
                 'acp-traffic-log-traffic
                 'defun)))
           (and file
                (file-name-nondirectory
                 file)))
         (let ((file
                (symbol-file
                 'acp-traffic-mode-map
                 'defvar)))
           (and file
                (file-name-nondirectory
                 file))))"##;
    let expect = expect![[r#"OK (t nil "acp-traffic.el" "acp-traffic.el")"#]];
    assert_acp_traffic_parity(elisp_form, expect);
}

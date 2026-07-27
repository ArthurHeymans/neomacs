use expect_test::expect;

use super::{assert_ada_ts_mode_eglot_parity, assert_ada_ts_mode_lsp_mode_parity};

#[test]
fn ada_ts_mode_eglot_complete_adapter_callable_and_custom_surface_matches() {
    let elisp_form = r##"(let (functions
               variables)
         (mapatoms
          (lambda (symbol)
            (when
                (string-prefix-p
                 "ada-ts-lspclient-eglot"
                 (symbol-name
                  symbol))
              (when
                  (and
                   (fboundp
                    symbol)
                   (equal
                    (file-name-base
                     (symbol-file
                      symbol
                      'defun))
                    "ada-ts-lspclient-eglot"))
                (push
                 (list
                  symbol
                  (help-function-arglist
                   symbol
                   t)
                  (commandp
                   symbol))
                 functions))
              (when
                  (and
                   (boundp
                    symbol)
                   (equal
                    (file-name-base
                     (symbol-file
                      symbol
                      'defvar))
                    "ada-ts-lspclient-eglot"))
                (push
                 (list
                  symbol
                  (default-value
                   symbol)
                  (copy-tree
                   (get
                    symbol
                    'custom-type))
                  (get
                   symbol
                   'custom-group)
                  (get
                   symbol
                   'risky-local-variable))
                 variables)))))
         (list
          (sort
           functions
           (lambda (left right)
             (string<
              (symbol-name
               (car
                left))
              (symbol-name
               (car
                right)))))
          (sort
           variables
           (lambda (left right)
             (string<
              (symbol-name
               (car
                left))
              (symbol-name
               (car
                right)))))))"##;
    let expect = expect![[
        r#"OK (((ada-ts-lspclient-eglot--config nil nil) (ada-ts-lspclient-eglot--find-mode-config (mode-to-find) nil) (ada-ts-lspclient-eglot--normalize (value) nil) (ada-ts-lspclient-eglot--setup nil nil) (ada-ts-lspclient-eglot-try nil nil)) ((ada-ts-lspclient-eglot-ignored-server-capabilities (:documentOnTypeFormattingProvider) (repeat symbol) nil nil) (ada-ts-lspclient-eglot-semantic-token-face-overrides (("namespace" (:foreground . default)) ("modifier" . font-lock-keyword-face)) (alist :key-type (string :tag "Semantic Token Name") :value-type (choice (face :tag "Face Name") (alist :key-type (symbol :tag "Face Attribute") :value-type (face :tag "Face Name") :tag "Face Attributes")) :tag "Semantic Token Face Overrides") nil nil) (ada-ts-lspclient-eglot-semantic-token-modifiers ("readonly" "deprecated") (repeat string) nil nil) (ada-ts-lspclient-eglot-semantic-token-types ("namespace" "type" "class" "enum" "interface" "struct" "typeParameter" "parameter" "variable" "property" "enumMember" "event" "function" "method" "macro" "keyword" "modifier" "comment" "string" "number" "regexp" "operator" "decorator") (repeat string) nil nil) (ada-ts-lspclient-eglot-stay-out-of (imenu) (repeat symbol) nil nil)))"#
    ]];
    assert_ada_ts_mode_eglot_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_eglot_generic_client_methods_execute_format_configure_and_resolve_roots() {
    let elisp_form = r##"(let (events)
         (cl-letf
             (((symbol-function
                'eglot-current-server)
               (lambda ()
                 'fixture-server))
              ((symbol-function
                'eglot-execute-command)
               (lambda (server command arguments)
                 (push
                  (list
                   'execute
                   server
                   command
                   arguments)
                  events)
                 [1
                  :json-false
                  (:nested
                   [2])]))
              ((symbol-function
                'eglot-server-capable)
               (lambda (capability)
                 (push
                  (list
                   'capable
                   capability)
                  events)
                 '(:commands
                   ["supported"
                    "other"])))
              ((symbol-function
                'eglot-path-to-uri)
               (lambda (path)
                 (concat
                  "uri:"
                  path)))
              ((symbol-function
                'eglot-format-buffer)
               (lambda ()
                 (push
                  'format-buffer
                  events)
                 'buffer-formatted))
              ((symbol-function
                'eglot-format)
               (lambda (beg end)
                 (push
                  (list
                   'format-region
                   beg
                   end)
                  events)
                 'region-formatted))
              ((symbol-function
                'eglot--workspace-configuration-plist)
               (lambda (_server)
                 '(:ada
                   (:projectFile
                    "fixture.gpr"
                    :enabled
                    :json-false))))
              ((symbol-function
                'eglot-workspace-folders)
               (lambda (_server)
                 '((:name
                    "/workspace/")
                   (:name
                    "/other/")))))
           (with-temp-buffer
             (insert
              "abcdef")
             (setq
              buffer-file-name
              "/workspace/src/main.adb")
             (list
              (ada-ts-lspclient-command-execute
               'eglot
               "fixture"
               1
               :json-false)
              (ada-ts-lspclient-command-supported-p
               'eglot
               "supported")
              (ada-ts-lspclient-command-supported-p
               'eglot
               "missing")
              (ada-ts-lspclient-document-id
               'eglot)
              (ada-ts-lspclient-format-region
               'eglot
               (point-min)
               (point-max))
              (ada-ts-lspclient-format-region
               'eglot
               2
               5)
              (ada-ts-lspclient-workspace-configuration
               'eglot
               "ada"
               :false)
              (ada-ts-lspclient-workspace-configuration
               'eglot
               "ada.enabled"
               :false)
              (ada-ts-lspclient-workspace-root
               'eglot
               buffer-file-name)
              (nreverse
               events)))))"##;
    let expect = expect![[
        r#"OK ((1 nil (:nested (2))) t nil (:uri "uri:/workspace/src/main.adb") buffer-formatted region-formatted (:projectFile "fixture.gpr" :enabled :false) :false "/workspace/" ((execute fixture-server "fixture" [1 :json-false]) (capable :executeCommandProvider) (capable :executeCommandProvider) format-buffer (format-region 2 5)))"#
    ]];
    assert_ada_ts_mode_eglot_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_eglot_setup_merges_defaults_and_applies_semantic_face_overrides() {
    let elisp_form = r##"(let ((ada-ts-lspclient-eglot-stay-out-of
                '(imenu
                  hover))
               (ada-ts-lspclient-eglot-ignored-server-capabilities
                '(:documentOnTypeFormattingProvider
                  :foldingRangeProvider))
               (ada-ts-lspclient-eglot-semantic-token-types
                '("namespace"
                  "modifier"))
               (ada-ts-lspclient-eglot-semantic-token-modifiers
                '("readonly"))
               (ada-ts-lspclient-eglot-semantic-token-face-overrides
                '(("namespace"
                   .
                   font-lock-type-face)
                  ("modifier"
                   .
                   ((:weight
                     .
                     bold)))))
               events)
         (set-default
          'eglot-stay-out-of
          '(flymake))
         (set-default
          'eglot-ignored-server-capabilities
          '(:hoverProvider))
         (set-default
          'eglot-semantic-token-types
          '("default"))
         (set-default
          'eglot-semantic-token-modifiers
          '("default"))
         (cl-letf
             (((symbol-function
                'face-remap-set-base)
               (lambda (face specification)
                 (push
                  (list
                   face
                   specification)
                  events)
                 specification))
              ((symbol-function
                'face-attribute)
               (lambda (face attribute &rest _)
                 (list
                  face
                  attribute))))
           (with-temp-buffer
             (ada-ts-lspclient-eglot--setup)
             (list
              eglot-stay-out-of
              eglot-ignored-server-capabilities
              eglot-semantic-token-types
              eglot-semantic-token-modifiers
              (mapcar
               #'local-variable-p
               '(eglot-stay-out-of
                 eglot-ignored-server-capabilities
                 eglot-semantic-token-types
                 eglot-semantic-token-modifiers))
              (nreverse
               events)))))"##;
    let expect = expect![[
        r#"OK ((flymake imenu hover) (:hoverProvider :documentOnTypeFormattingProvider :foldingRangeProvider) ("namespace" "modifier") ("readonly") (t t t t) ((eglot-semantic-namespace (:inherit font-lock-type-face)) (eglot-semantic-modifier (:weight bold :weight))))"#
    ]];
    assert_ada_ts_mode_eglot_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_eglot_server_configuration_covers_existing_extended_and_default_entries() {
    let elisp_form = r##"(cl-labels
         ((configure
           (programs)
           (let ((eglot-server-programs
                  programs))
             (ada-ts-lspclient-eglot--config)
             eglot-server-programs)))
         (list
          (configure
           '(((ada-mode
               ada-ts-mode)
              "existing-server")))
          (configure
           '((ada-mode
              "ada-server")))
          (configure
           '((python-mode
              "python-server")))))"##;
    let expect = expect![[
        r#"OK ((((ada-mode ada-ts-mode) "existing-server")) (((ada-mode #2=(ada-ts-mode :language-id "ada")) . #1=("ada-server")) (ada-mode . #1#)) (((ada-mode #2#) "ada_language_server") (python-mode "python-server")))"#
    ]];
    assert_ada_ts_mode_eglot_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_lsp_mode_complete_adapter_callable_and_custom_surface_matches() {
    let elisp_form = r##"(let (functions
               variables)
         (mapatoms
          (lambda (symbol)
            (when
                (string-prefix-p
                 "ada-ts-lspclient-lsp-mode"
                 (symbol-name
                  symbol))
              (when
                  (and
                   (fboundp
                    symbol)
                   (equal
                    (file-name-base
                     (symbol-file
                      symbol
                      'defun))
                    "ada-ts-lspclient-lsp-mode"))
                (push
                 (list
                  symbol
                  (help-function-arglist
                   symbol
                   t)
                  (commandp
                   symbol))
                 functions))
              (when
                  (and
                   (boundp
                    symbol)
                   (equal
                    (file-name-base
                     (symbol-file
                      symbol
                      'defvar))
                    "ada-ts-lspclient-lsp-mode"))
                (push
                 (list
                  symbol
                  (default-value
                   symbol)
                  (copy-tree
                   (get
                    symbol
                    'custom-type))
                  (get
                   symbol
                   'custom-group)
                  (get
                   symbol
                   'risky-local-variable))
                 variables)))))
         (list
          (sort
           functions
           (lambda (left right)
             (string<
              (symbol-name
               (car
                left))
              (symbol-name
               (car
                right)))))
          (sort
           variables
           (lambda (left right)
             (string<
              (symbol-name
               (car
                left))
              (symbol-name
               (car
                right)))))))"##;
    let expect = expect![
        "OK (((ada-ts-lspclient-lsp-mode--extra-folders (workspace) nil) (ada-ts-lspclient-lsp-mode--initialized nil nil) (ada-ts-lspclient-lsp-mode--normalize (value) nil) (ada-ts-lspclient-lsp-mode--setup nil nil) (ada-ts-lspclient-lsp-mode-try nil nil)) ((ada-ts-lspclient-lsp-mode--library-folders-fn nil nil nil nil) (ada-ts-lspclient-lsp-mode-settings-alist ((lsp-enable-imenu) (lsp-enable-indentation) (lsp-enable-on-type-formatting) (lsp-semantic-tokens-enable . t)) (alist :key-type symbol :value-type boolean) nil t)))"
    ];
    assert_ada_ts_mode_lsp_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_lsp_mode_generic_methods_execute_format_configure_and_manage_workspace_dirs() {
    let elisp_form = r##"(let (events
               (workspace
                'fixture-workspace)
               (ada-ts-lspclient--lsp-workspace-extra-dirs-alist
                nil))
         (cl-letf
             (((symbol-function
                'lsp-workspace-command-execute)
               (lambda (command arguments)
                 (push
                  (list
                   'execute
                   command
                   arguments)
                  events)
                 [1
                  :json-false
                  (:nested
                   2)]))
              ((symbol-function
                'lsp-can-execute-command?)
               (lambda (command)
                 (equal
                  command
                  "supported")))
              ((symbol-function
                'lsp-text-document-identifier)
               (lambda ()
                 '(:uri
                   "file:///workspace/main.adb")))
              ((symbol-function
                'lsp-format-buffer)
               (lambda ()
                 (push
                  'format-buffer
                  events)
                 'buffer-formatted))
              ((symbol-function
                'lsp-format-region)
               (lambda (beg end)
                 (push
                  (list
                   'format-region
                   beg
                   end)
                  events)
                 'region-formatted))
              ((symbol-function
                'lsp-configuration-section)
               (lambda (section)
                 (list
                  (intern
                   (concat
                    ":"
                    section))
                  '(:nested
                    (:enabled
                     t
                     :disabled
                     :json-false)))))
              ((symbol-function
                'lsp-workspaces)
               (lambda ()
                 (list
                  workspace)))
              ((symbol-function
                'lsp--workspace-root)
               (lambda (_workspace)
                 "/workspace"))
              ((symbol-function
                'lsp-workspace-root)
               (lambda (path)
                 (push
                  (list
                   'workspace-root
                   path)
                  events)
                 "/workspace")))
           (with-temp-buffer
             (insert
              "abcdef")
             (let ((values
                    (list
                     (ada-ts-lspclient-command-execute
                      'lsp-mode
                      "fixture"
                      1
                      :json-false)
                     (ada-ts-lspclient-command-supported-p
                      'lsp-mode
                      "supported")
                     (ada-ts-lspclient-command-supported-p
                      'lsp-mode
                      "missing")
                     (ada-ts-lspclient-document-id
                      'lsp-mode)
                     (ada-ts-lspclient-format-region
                      'lsp-mode
                      (point-min)
                      (point-max))
                     (ada-ts-lspclient-format-region
                      'lsp-mode
                      2
                      5)
                     (ada-ts-lspclient-workspace-configuration
                      'lsp-mode
                      "ada.nested"
                      :false)
                     (ada-ts-lspclient-workspace-root
                      'lsp-mode
                      "/workspace/src/main.adb"))))
               (ada-ts-lspclient-workspace-dirs-add
                'lsp-mode
                '("/workspace/src"
                  "/external/lib"
                  "/external/include"))
               (list
                values
                ada-ts-lspclient--lsp-workspace-extra-dirs-alist
                (nreverse
                 events))))))"##;
    let expect = expect![[
        r#"OK (((1 nil (:nested 2)) t nil (:uri "file:///workspace/main.adb") buffer-formatted region-formatted (:enabled t :disabled :false) "/workspace/") (("/workspace" "/external/lib" "/external/include")) ((execute "fixture" [1 :json-false]) format-buffer (format-region 2 5) (workspace-root "/workspace/src/main.adb")))"#
    ]];
    assert_ada_ts_mode_lsp_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_lsp_mode_setup_session_notification_and_extra_folder_composition_match() {
    let elisp_form = r##"(let* ((workspace
                  'fixture-workspace)
                 (source-buffer
                  (generate-new-buffer
                   " *ada-lsp-source*"))
                 (ada-ts-lspclient-lsp-mode-settings-alist
                  '((lsp-enable-imenu
                     .
                     nil)
                    (lsp-semantic-tokens-enable
                     .
                     t)))
                 (ada-ts-lspclient--lsp-workspace-extra-dirs-alist
                  '(("/workspace"
                     "/external/a"
                     "/external/b")))
                 (ada-ts-lspclient-lsp-mode--library-folders-fn
                  (lambda (_workspace)
                    '("/library/a")))
                 events)
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'lsp-workspaces)
                   (lambda ()
                     (list
                      workspace)))
                  ((symbol-function
                    'lsp--workspace-buffers)
                   (lambda (_workspace)
                     (list
                      source-buffer)))
                  ((symbol-function
                    'lsp--workspace-root)
                   (lambda (_workspace)
                     "/workspace")))
               (with-current-buffer source-buffer
                 (add-hook
                  'ada-ts-lspclient-session-hook
                  (lambda ()
                    (push
                     (buffer-name)
                     events))
                  nil
                  t))
               (ada-ts-lspclient-lsp-mode--initialized)
               (with-temp-buffer
                 (ada-ts-lspclient-lsp-mode--setup)
                 (list
                  lsp-enable-imenu
                  lsp-semantic-tokens-enable
                  (local-variable-p
                   'lsp-enable-imenu)
                  (local-variable-p
                   'lsp-semantic-tokens-enable)
                  (ada-ts-lspclient-lsp-mode--extra-folders
                   workspace)
                  (nreverse
                   events))))
           (kill-buffer
            source-buffer)))"##;
    let expect = expect![[
        r#"OK (nil t t t ("/library/a" "/external/a" "/external/b") (" *ada-lsp-source*"))"#
    ]];
    assert_ada_ts_mode_lsp_mode_parity(elisp_form, expect);
}

use expect_test::expect;

use super::{
    assert_ada_ts_mode_eglot_parity, assert_ada_ts_mode_lsp_mode_parity, assert_ada_ts_mode_parity,
};

#[test]
fn ada_ts_mode_case_format_word_covers_formatter_dictionary_subwords_and_point() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (let ((word
                  (nth 0 case))
                 (formatter
                  (nth 1 case))
                 (dictionary
                  (nth 2 case))
                 (point-offset
                  (nth 3 case)))
             (with-temp-buffer
               (insert word)
               (goto-char
                (+
                 (point-min)
                 point-offset))
               (set-buffer-modified-p
                nil)
               (ada-ts-mode--case-format-word
                (point-min)
                (point-max)
                formatter
                dictionary)
               (list
                word
                formatter
                dictionary
                (buffer-string)
                (point)
                (buffer-modified-p)))))
         '(("hello_world" upcase nil 3)
           ("hello_world" upcase-initials nil 5)
           ("ascii" downcase ("ASCII") 2)
           ("my_ascii_io" upcase-initials ("ASCII" "IO") 4)
           ("Already" capitalize nil 1)
           ("gnat_io" downcase ("GNAT" "IO") 7)))"##;
    let expect = expect![[
        r#"OK (("hello_world" upcase nil "HELLO_WORLD" 4 t) ("hello_world" upcase-initials nil "Hello_World" 6 t) ("ascii" downcase ("ASCII") "ASCII" 3 t) ("my_ascii_io" upcase-initials ("ASCII" "IO") "My_ASCII_IO" 5 t) ("Already" capitalize nil "Already" 2 nil) ("gnat_io" downcase ("GNAT" "IO") "GNAT_IO" 8 t))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_case_formatting_safe_local_predicate_accepts_only_safe_formatters() {
    let elisp_form = r##"(let ((predicate
                (get
                 'ada-ts-mode-case-formatting
                 'safe-local-variable)))
         (mapcar
          (lambda (rules)
            (list
             rules
             (funcall
              predicate
              rules)))
          '(((identifier
              :formatter downcase))
            ((identifier
              :formatter upcase)
             (keyword
              :formatter capitalize))
            ((identifier
              :formatter shell-command))
            ((identifier))
            (identifier)
            nil)))"##;
    let expect = expect![
        "OK ((((identifier :formatter downcase)) t) (((identifier :formatter upcase) (keyword :formatter capitalize)) t) (((identifier :formatter shell-command)) nil) (((identifier)) nil) ((identifier) nil) (nil t))"
    ];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_case_settings_watcher_builds_default_and_buffer_local_rules() {
    let elisp_form = r##"(let ((original
                (default-value
                 'ada-ts-mode-case-formatting)))
         (unwind-protect
             (list
              (progn
                (set-default
                 'ada-ts-mode-case-formatting
                 '((identifier
                    :formatter upcase
                    :dictionary
                    ("GNAT"))
                   (keyword
                    :formatter downcase)))
                (default-value
                 'ada-ts-mode--case-formatting))
              (with-temp-buffer
                (setq-local
                 ada-ts-mode-case-formatting
                 '((identifier
                    :formatter capitalize
                    :dictionary
                    (:words
                     ("ASCII"
                      "IO")))))
                (list
                 ada-ts-mode--case-formatting
                 (local-variable-p
                  'ada-ts-mode--case-formatting))))
           (set-default
            'ada-ts-mode-case-formatting
            original)))"##;
    let expect = expect![[
        r#"OK (((identifier :formatter upcase :dictionary ("GNAT")) (keyword :formatter downcase)) (((identifier :formatter capitalize :dictionary ("ASCII" "IO"))) t))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_dictionary_loader_parses_whitespace_duplicates_and_star_separators() {
    let elisp_form = r##"(let* ((directory
                 (expand-file-name
                  "ada-ts-dictionary"
                  (getenv
                   "HOME")))
                (file
                 (expand-file-name
                  "words.txt"
                  directory))
                (ada-ts-mode--case-dictionary-file-alist
                 nil))
         (make-directory
          directory
          t)
         (with-temp-file file
           (insert
            "\n"
            "  ASCII\n"
            "GNAT*IO   API\n"
            "ascii\n"
            "\tHTTP   URL\n"))
         (ada-ts-mode--case-dictionary-load
          file)
         (let ((entry
                (cdr
                 (assoc-string
                  file
                  ada-ts-mode--case-dictionary-file-alist))))
           (list
            (plist-get
             entry
             :words)
            (and
             (plist-get
              entry
              :modification-time)
             t)
            (length
             ada-ts-mode--case-dictionary-file-alist))))"##;
    let expect = expect![[r#"OK (("ASCII" "GNAT" "IO   API" "HTTP   URL") t 1)"#]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_imenu_nesting_strategies_preserve_markers_and_subtrees_exactly() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "0123456789")
         (let ((marker
                (copy-marker
                 4))
               (subtrees
                '(("Inner" . 8)
                  ("Other" . 10))))
           (list
            (ada-ts-mode-imenu-nesting-strategy-before
             "Outer"
             marker
             subtrees)
            (let ((ada-ts-mode-imenu-nesting-strategy-placeholder
                   "<self>"))
              (ada-ts-mode-imenu-nesting-strategy-within
               "Outer"
               marker
               subtrees))
            (marker-position
             marker)
            subtrees)))"##;
    let expect = expect![[
        r#"OK ((("Outer" :marker nil nil) ("Outer" . #1=(("Inner" . 8) ("Other" . 10)))) (("Outer" ("<self>" :marker nil nil) . #1#)) 4 #1#)"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_imenu_sort_alphabetically_orders_placeholder_and_case_variants() {
    let elisp_form = r##"(let ((ada-ts-mode-imenu-nesting-strategy-placeholder
                "<<parent>>"))
         (mapcar
          #'car
          (ada-ts-mode-imenu-sort-alphabetically
           '(("zulu" . 1)
             ("Alpha" . 2)
             ("<<parent>>" . 3)
             ("beta" . 4)
             ("Zulu" . 5)
             ("alpha" . 6)))))"##;
    let expect = expect![[r#"OK ("<<parent>>" "Alpha" "Zulu" "alpha" "beta" "zulu")"#]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_als_json_reader_strips_comments_commas_and_controls_false_value() {
    let elisp_form = r##"(let* ((directory
                 (expand-file-name
                  "ada-ts-json"
                  (getenv
                   "HOME")))
                (file
                 (expand-file-name
                  "config.json"
                  directory))
                (empty
                 (expand-file-name
                  "empty.json"
                  directory)))
         (make-directory
          directory
          t)
         (with-temp-file file
           (insert
            "{\n"
            "  // comment\n"
            "  \"enabled\": false,\n"
            "  \"text\": \"// not a comment\",\n"
            "  \"nested\": {\"value\": 2,},\n"
            "  /* block */\n"
            "  \"items\": [1, 2,],\n"
            "}\n"))
         (with-temp-file empty)
         (list
          (ada-ts-als--read-json-file
           file)
          (ada-ts-als--read-json-file
           file
           :false)
          (ada-ts-als--read-json-file
           empty
           :false)))"##;
    let expect = expect![[
        r#"OK ((:enabled nil :text "// not a comment" :nested (:value 2) :items [1 2]) (:enabled :false :text "// not a comment" :nested (:value 2) :items [1 2]) nil)"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_als_composite_configuration_merges_user_workspace_and_client_layers() {
    let elisp_form = r##"(let* ((directory
                 (expand-file-name
                  "ada-ts-composite"
                  (getenv
                   "HOME")))
                (user-file
                 (expand-file-name
                  "user.json"
                  directory))
                (workspace-file
                 (expand-file-name
                  "workspace.json"
                  directory)))
         (make-directory
          directory
          t)
         (with-temp-file user-file
           (insert
            "{\"ada\":{\"projectFile\":\"user.gpr\",\"scenario\":{\"A\":1,\"B\":2}},\"userOnly\":true}"))
         (with-temp-file workspace-file
           (insert
            "{\"ada\":{\"projectFile\":\"workspace.gpr\",\"scenario\":{\"B\":20,\"C\":30}},\"workspaceOnly\":true}"))
         (cl-letf
             (((symbol-function
                'ada-ts-als-user-config-file)
               (lambda ()
                 user-file))
              ((symbol-function
                'ada-ts-als-workspace-config-file)
               (lambda ()
                 workspace-file))
              ((symbol-function
                'ada-ts-lspclient-current)
               (lambda ()
                 'fake-client))
              ((symbol-function
                'ada-ts-lspclient-workspace-configuration)
               (lambda
                 (_client _scope false)
                 (list
                  :projectFile
                  "client.gpr"
                  :scenario
                  (list
                   :C
                   300
                   :D
                   400)
                  :clientFalse
                  false))))
           (list
            (ada-ts-als-composite-config)
            (ada-ts-als-composite-config
             :false))))"##;
    let expect = expect![[
        r#"OK ((:ada (:projectFile "user.gpr" :scenario (:B 2 :C 30 :A 1)) :workspaceOnly t :userOnly t :projectFile "client.gpr" :scenario (:C 300 :D 400) :clientFalse nil) (:ada (:projectFile "user.gpr" :scenario (:B 2 :C 30 :A 1)) :workspaceOnly t :userOnly t :projectFile "client.gpr" :scenario (:C 300 :D 400) :clientFalse :false))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_als_formatting_covers_absence_blank_success_error_and_point_rules() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (insert
            "value")
           (cl-letf
               (((symbol-function
                  'ada-ts-lspclient-current)
                 (lambda ()
                   nil)))
             (ada-ts-als-format-line
              3)))
         (with-temp-buffer
           (insert
            "   ")
           (cl-letf
               (((symbol-function
                  'ada-ts-lspclient-current)
                 (lambda ()
                   'client))
                ((symbol-function
                  'ada-ts-lspclient-format-region)
                 (lambda
                   (&rest _)
                   (error
                    "must not format blank"))))
             (ada-ts-als-format-line
              3)))
         (with-temp-buffer
           (insert
            "  value")
           (goto-char
            2)
           (cl-letf
               (((symbol-function
                  'ada-ts-lspclient-current)
                 (lambda ()
                   'client))
                ((symbol-function
                  'ada-ts-lspclient-format-region)
                 (lambda
                   (_client beg end)
                   (delete-region
                    beg
                    end)
                   (goto-char beg)
                   (insert
                    "    value"))))
             (list
              (ada-ts-als-format-line
               4)
              (buffer-string)
              (point)
              tab-width
              standard-indent)))
         (with-temp-buffer
           (insert
            "value")
           (cl-letf
               (((symbol-function
                  'ada-ts-lspclient-current)
                 (lambda ()
                   'client))
                ((symbol-function
                  'ada-ts-lspclient-format-region)
                 (lambda
                   (&rest _)
                   (error
                    "format failed"))))
             (ada-ts-als-format-region
              (point-min)
              (point-max)
              2))))"##;
    let expect = expect![[r#"OK (nil nil (success "    value" 5 8 4) nil)"#]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_als_command_helpers_cover_support_results_normalization_and_errors() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function
                'ada-ts-lspclient-current)
               (lambda ()
                 'client))
              ((symbol-function
                'ada-ts-lspclient-command-supported-p)
               (lambda
                 (_client command)
                 (not
                  (string-equal
                   command
                   "als-object-dir"))))
              ((symbol-function
                'ada-ts-lspclient-document-id)
               (lambda
                 (_client)
                 '(:uri
                   "file:///source.adb")))
              ((symbol-function
                'ada-ts-lspclient-command-execute)
               (lambda
                 (_client command &rest arguments)
                 (push
                  (cons
                   command
                   arguments)
                  calls)
                 (pcase command
                   ("als-executables"
                    '("./bin/app"))
                   ("als-get-project-attribute-value"
                    '("src"))
                   ("als-mains"
                    '("main.adb"))
                   ("als-other-file"
                    t)
                   ("als-source-dirs"
                    '((:uri
                       "file:///workspace/src/")
                      (:uri
                       "file:///workspace/lib/")))
                   (_
                    (error
                     "unknown"))))))
           (list
            (ada-ts-als-executables)
            (ada-ts-als-get-project-attribute-value
             "Source_Dirs"
             "Compiler"
             "Ada")
            (ada-ts-als-mains)
            (ada-ts-als-object-dir)
            (ada-ts-als-other-file)
            (ada-ts-als-source-dirs)
            (nreverse
             calls))))"##;
    let expect = expect![[
        r#"OK (("[ORACLE-SANDBOX]/bin/app") ("src") ("[ORACLE-SANDBOX]/main.adb") nil t ("/workspace/src" "/workspace/lib") (("als-executables") ("als-get-project-attribute-value" (:attribute "Source_Dirs" :pkg "Compiler" :index "Ada")) ("als-mains") ("als-other-file" (:uri "file:///source.adb")) ("als-source-dirs")))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_lspclient_current_honors_special_hook_order_and_first_success() {
    let elisp_form = r##"(let ((ada-ts-lspclient-find-functions
                (list
                 (lambda ()
                   nil)
                 (lambda ()
                   'selected)
                 (lambda ()
                   'unreachable))))
         (list
          (ada-ts-lspclient-current)
          (let ((ada-ts-lspclient-find-functions
                 nil))
            (ada-ts-lspclient-current))))"##;
    let expect = expect!["OK (selected nil)"];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_eglot_adapter_normalizes_nested_json_sequences_and_false_values() {
    let elisp_form = r##"(list
         (ada-ts-lspclient-eglot--normalize
          '(:root
            [1
             :json-false
             (:nested
              [2
               :json-false])]))
         (mapcar
          (lambda (mode)
            (let ((entry
                   (ada-ts-lspclient-eglot--find-mode-config
                    mode)))
              (and entry
                   (car entry))))
          '(ada-mode
            ada-ts-mode
            nonexistent-mode))
         (memq
          'ada-ts-lspclient-eglot-try
          ada-ts-lspclient-find-functions)
         (memq
          'ada-ts-lspclient-eglot--setup
          ada-ts-lspclient-setup-hook))"##;
    let expect = expect![
        "OK ((:root (1 nil (:nested (2 nil)))) (#1=(ada-mode ada-ts-mode) #1# nil) (ada-ts-lspclient-eglot-try) (ada-ts-lspclient-eglot--setup))"
    ];
    assert_ada_ts_mode_eglot_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_lsp_mode_adapter_normalizes_hashes_vectors_false_and_empty_values() {
    let elisp_form = r##"(let ((table
                (make-hash-table
                 :test
                 #'equal)))
         (puthash
          "present"
          [1
           :json-false
           (:nested
            2)]
          table)
         (puthash
          "false"
          :json-false
          table)
         (puthash
          "empty"
          nil
          table)
         (list
          (ada-ts-lspclient-lsp-mode--normalize
           table)
          (with-temp-buffer
            (setq-local
             lsp-mode
             t)
            (ada-ts-lspclient-lsp-mode-try))
          (with-temp-buffer
            (setq-local
             lsp-mode
             nil)
            (ada-ts-lspclient-lsp-mode-try))
          (memq
           'ada-ts-lspclient-lsp-mode-try
           ada-ts-lspclient-find-functions)
          (memq
           'ada-ts-lspclient-lsp-mode--setup
           ada-ts-lspclient-setup-hook)))"##;
    let expect = expect![
        "OK ((:present (1 nil (:nested 2))) lsp-mode nil (ada-ts-lspclient-lsp-mode-try) (ada-ts-lspclient-lsp-mode--setup))"
    ];
    assert_ada_ts_mode_lsp_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_case_dictionary_watcher_loads_reloads_merges_and_reports_missing_files() {
    let elisp_form = r##"(let* ((home
                 (file-name-as-directory
                  (getenv
                   "HOME")))
                (root
                 (expand-file-name
                  "case-watcher/"
                  home))
                (source
                 (expand-file-name
                  "src/main.adb"
                  root))
                (dictionary
                 (expand-file-name
                  "words.txt"
                  root))
                (rules
                 '((identifier
                    :formatter upcase-initials
                    :dictionary
                    (:files
                     ("words.txt"
                      "missing.txt")
                     :words
                     ("LOCAL")))))
                (ada-ts-mode--case-dictionary-file-alist
                 nil)
                messages)
         (make-directory
          (file-name-directory
           source)
          t)
         (with-temp-file dictionary
           (insert
            "ASCII\n"
            "GNAT*IO\n"))
         (cl-letf
             (((symbol-function
                'message)
               (lambda (format-string &rest arguments)
                 (push
                  (apply
                   #'format
                   format-string
                   arguments)
                  messages))))
           (with-temp-buffer
             (setq
              buffer-file-name
              source)
             (ada-ts-mode--case-settings-process
              'ada-ts-mode-case-formatting
              rules
              'set
              (current-buffer))
             (let ((initial
                    ada-ts-mode--case-formatting)
                   (initial-cache
                    (mapcar
                     (lambda (entry)
                       (list
                        (file-relative-name
                         (car
                          entry)
                         home)
                        (plist-get
                         (cdr
                          entry)
                         :words)))
                     ada-ts-mode--case-dictionary-file-alist)))
               (with-temp-file dictionary
                 (insert
                  "HTTP\n"
                  "URL\n"))
               (set-file-times
                dictionary
                (time-add
                 (current-time)
                 20))
               (ada-ts-mode--case-settings-process
                'ada-ts-mode-case-formatting
                rules
                'set
                (current-buffer))
               (list
                initial
                initial-cache
                ada-ts-mode--case-formatting
                (mapcar
                 (lambda (entry)
                   (list
                    (file-relative-name
                     (car
                      entry)
                     home)
                    (plist-get
                     (cdr
                      entry)
                     :words)))
                 ada-ts-mode--case-dictionary-file-alist)
                (nreverse
                 messages))))))"##;
    let expect = expect![[
        r#"OK (((identifier :formatter upcase-initials :dictionary ("ASCII" "GNAT" "IO" "LOCAL" . #1=("LOCAL")))) (("case-watcher/words.txt" ("ASCII" "GNAT" "IO"))) ((identifier :formatter upcase-initials :dictionary ("HTTP" "URL" "LOCAL" . #1#))) (("case-watcher/words.txt" ("HTTP" "URL"))) ("Cannot read missing.txt, skipping dictionary file." "Cannot read missing.txt, skipping dictionary file."))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

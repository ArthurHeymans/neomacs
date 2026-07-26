use expect_test::expect;

use super::{assert_ac_php_core_autoload_parity, assert_ac_php_core_parity};

#[test]
fn ac_php_core_exact_pin_dependencies_features_group_and_packaged_assets_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ac-php-core
                      package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (mapcar
                 #'featurep
                 '(ac-php-core
                   json
                   s
                   f
                   xcscope
                   popup
                   dash
                   eldoc
                   cl-lib))
                (get
                 'ac-php
                 'group-documentation)
                (get
                 'ac-php
                 'custom-prefix)
                (mapcar
                 (lambda (parent)
                   (assq
                    'ac-php
                    (get parent
                         'custom-group)))
                 '(auto-complete
                   completion
                   convenience))
                (get
                 'ac-php
                 'custom-links)
                (mapcar
                 (lambda (file)
                   (list
                    file
                    (file-exists-p
                     (expand-file-name
                      file
                      ac-php-root-directory))))
                 '("ac-php-core.el"
                   "ac-php-comm-tags-data.el"
                   "ac-php-comm-tags-data.json"
                   "phpctags"))))"##;
    let expect = expect![[
        r#"OK (ac-php-core "20260210.846" ((emacs (24 4)) (dash (1)) (php-mode (1)) (s (1)) (f (0 17 0)) (popup (0 5 0)) (xcscope (1 0))) (t t t t t t t t t) "Auto Completion source for PHP." "ac-php-" ((ac-php custom-group) (ac-php custom-group) (ac-php custom-group)) ((emacs-commentary-link :tag "Commentary" "ac-php") (url-link :tag "GitHub Page" "https://github.com/xcwen/ac-php") (url-link :tag "Bug Tracker" "https://github.com/xcwen/ac-php/issues")) (("ac-php-core.el" t) ("ac-php-comm-tags-data.el" t) ("ac-php-comm-tags-data.json" t) ("phpctags" t)))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_custom_variables_preserve_defaults_types_groups_docs_and_risk() {
    let elisp_form = r##"(mapcar
               (lambda (variable)
                 (list
                  variable
                  (get variable
                       'standard-value)
                  (get variable
                       'custom-type)
                  (assq
                   variable
                   (get
                    'ac-php
                    'custom-group))
                  (get variable
                       'variable-documentation)
                  (get variable
                       'risky-local-variable)
                  (cond
                   ((memq
                     variable
                     '(ac-php-php-executable
                       ac-php-cscope
                       ac-php-tags-path))
                    (stringp
                     (symbol-value
                      variable)))
                   (t
                    (symbol-value
                     variable)))))
               '(ac-php-php-executable
                 ac-php-cscope
                 ac-php-use-cscope-flag
                 ac-php-auto-update-intval
                 ac-php-project-root-dir-use-truename
                 ac-php-mode-line
                 ac-php-tags-path))"##;
    let expect = expect![[
        r#"OK ((ac-php-php-executable ((executable-find "php")) string (ac-php-php-executable custom-variable) "Set PHP command line interpreter executable path.\nFor more see URL `http://php.net/manual/en/features.commandline.php'." nil nil) (ac-php-cscope ((executable-find "cscope")) string (ac-php-cscope custom-variable) "Set the Csope executable path.\nFor more see URL `http://cscope.sourceforge.net/'." nil nil) (ac-php-use-cscope-flag (nil) boolean (ac-php-use-cscope-flag custom-variable) "Non-nil means use Cscope if it is possible.\nTo use this feature you'll need to set cscope executable path in\n`ac-php-cscope'.  For more see URL `http://cscope.sourceforge.net'." nil nil) (ac-php-auto-update-intval (3600) integer (ac-php-auto-update-intval custom-variable) "The interval between automatic re-indexing project's files (in seconds)." nil 3600) (ac-php-project-root-dir-use-truename (t) boolean (ac-php-project-root-dir-use-truename custom-variable) "Non-nil means always expand filenames using function `file-truename'." nil t) (ac-php-mode-line ('#1=(:eval (format "AP%s" (ac-php-mode-line-project-status)))) sexp (ac-php-mode-line custom-variable) "Mode line lighter for ac-php.\nSet this variable to nil to disable the lighter." t #1#) (ac-php-tags-path ((concat (getenv "HOME") "/.cache/ac-php")) string (ac-php-tags-path custom-variable) "Use this directory as a base path for the per-projects tags directories..\n\nThe idea is to have a common local directory for the all projects.  This path\nget extended with the directory tree of the project that you are indexing the\ntags for." nil t))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_internal_defaults_regexps_and_package_assets_have_exact_snapshots() {
    let elisp_form = r##"(mapcar
               (lambda (variable)
                 (list
                  variable
                  (symbol-value variable)
                  (get variable
                       'standard-value)
                  (get variable
                       'variable-documentation)))
               '(ac-php-config-file
                 ac-php-debug-flag
                 ac-php-gen-tags-flag
                 ac-php-phptags-index-progress
                 ac-php-tag-last-data-list
                 ac-php-re-classlike-pattern
                 ac-php-re-namespace-unit-pattern
                 ac-php-re-namespace-pattern
                 ac-php-re-beginning-of-defun-pattern
                 ac-php-re-annotated-var-pattern
                 ac-php-prefix-str
                 ac-php-location-stack-index
                 ac-php-location-stack
                 ac-php--php-key-list
                 ac-php-rebuild-tmp-error-msg
                 ac-php-max-bookmark-count))"##;
    let expect = expect![[
        r#"OK ((ac-php-config-file ".ac-php-conf.json" nil "Per-project configuration file.") (ac-php-debug-flag nil nil "Non-nil means enable verbose mode when processing autocomplete.\nPlease notice, enabling this option entails detailed output of debugging\ninformation to the ‘*Messages*’ buffer.  This feature is designed for\nac-php developer only.") (ac-php-gen-tags-flag nil nil "Non-nil means that remaking tags currently is under process.") (ac-php-phptags-index-progress 0 nil "The re-index progress indicator.\nMeant for `ac-php-mode-line-project-status'") (ac-php-tag-last-data-list nil nil "Holds in-memory database for per-project tags.") (ac-php-re-classlike-pattern "^\\(?:<\\(?:\\?\\(?:php\\)?\\|%\\)\\)?\\s-*\\(?:\\(?:abstract\\|final\\)\\s-+\\)?\\(?:class\\|trait\\|enum\\)\\s-+\\([a-zA-Z_\177-ÿ][a-zA-Z0-9_\177-ÿ]*\\)" nil "The regular expression for classlike.") (ac-php-re-namespace-unit-pattern "\\(?:\\\\\\)?\\(?:[a-zA-Z_\177-ÿ][a-zA-Z0-9_\177-ÿ\\]*\\)" nil "The regular expression for a part of a namespace.") (ac-php-re-namespace-pattern "^\\(?:<\\(?:\\?\\(?:php\\)?\\|%\\)\\)?\\s-*namespace\\s-+\\(\\(?:\\\\\\)?\\(?:[a-zA-Z_\177-ÿ][a-zA-Z0-9_\177-ÿ\\]*\\)\\)\\s-*;" nil "The regular expression for a namespace.") (ac-php-re-beginning-of-defun-pattern "^\\s-*\\(?:\\(?:abstract\\|final\\|private\\|protected\\|public\\|static\\)\\s-+\\)*function\\s-+&?\\(\\(?:\\sw\\|\\s_\\)+\\)\\s-*(" nil "Regular expression for a PHP function.") (ac-php-re-annotated-var-pattern "@var\\s-+\\(\\(?:\\\\\\)?\\(?:[a-zA-Z_\177-ÿ][a-zA-Z0-9_\177-ÿ\\]*\\)\\)\\>\\s-+" nil "The regular expression for a class inside an annotated variable.") (ac-php-prefix-str "" nil nil) (ac-php-location-stack-index 0 nil nil) (ac-php-location-stack nil nil nil) (ac-php--php-key-list ("public" "class" "namespace" "protected" "private" "function" "while" "extends" "return" "static" "global" "continue" "abstract" "finally" "instanceof") nil nil) (ac-php-rebuild-tmp-error-msg nil nil nil) (ac-php-max-bookmark-count 500 nil nil))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_package_root_ctags_and_generated_minor_mode_metadata_match() {
    let elisp_form = r##"(list
               (list
                (file-name-nondirectory
                 (directory-file-name
                  ac-php-root-directory))
                (file-directory-p
                 ac-php-root-directory)
                (get
                 'ac-php-root-directory
                 'standard-value)
                (get
                 'ac-php-root-directory
                 'variable-documentation))
               (list
                (file-relative-name
                 ac-php-ctags-executable
                 ac-php-root-directory)
                (equal
                 ac-php-ctags-executable
                 (concat
                  ac-php-root-directory
                  "phpctags"))
                (file-exists-p
                 ac-php-ctags-executable)
                (get
                 'ac-php-ctags-executable
                 'standard-value)
                (get
                 'ac-php-ctags-executable
                 'variable-documentation))
               (list
                (default-value
                 'ac-php-mode)
                (get
                 'ac-php-mode
                 'standard-value)
                (get
                 'ac-php-mode
                 'custom-type)
                (get
                 'ac-php-mode
                 'custom-group)
                (assq
                 'ac-php-mode
                 (get
                  'ac-php
                  'custom-group))
                (get
                 'ac-php-mode
                 'variable-documentation)
                (assq
                 'ac-php-mode
                 minor-mode-alist)
                (boundp
                 'ac-php-mode-hook)
                (get
                 'ac-php-mode-hook
                 'variable-documentation)
                (with-temp-buffer
                  (list
                   (local-variable-p
                    'ac-php-mode)
                   (progn
                     (ac-php-mode 1)
                     (local-variable-p
                      'ac-php-mode))
                   ac-php-mode
                   ac-php-gen-tags-flag
                   (assq
                    'ac-php-mode
                    minor-mode-alist)))))"##;
    let expect = expect![[
        r#"OK (("ac-php-core-20260210.846" t nil "The ac-php package location.") ("phpctags" t t nil "Set the Phpctags executable path.  Don't change the value of this variable.") (nil nil nil nil nil "Non-nil if Ac-Php mode is enabled.\nUse the command `ac-php-mode' to change this variable." #1=(ac-php-mode ac-php-mode-line) t "Hook run after entering or leaving `ac-php-mode'.\nNo problems result if this variable is not bound.\n`add-hook' automatically binds it.  (This is true for all hook variables.)" (nil t t t #1#)))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_fresh_autoload_file_registers_custom_loads_and_interactive_eldoc_entrypoint() {
    let elisp_form = r##"(list
               (featurep
                'ac-php-core)
               (featurep
                'ac-php-core-autoloads)
               (get
                'ac-php
                'custom-loads)
               (get
                'auto-complete
                'custom-loads)
               (fboundp
                'ac-php-core-eldoc-setup)
               (autoloadp
                (symbol-function
                 'ac-php-core-eldoc-setup))
               (commandp
                'ac-php-core-eldoc-setup)
               (symbol-function
                'ac-php-core-eldoc-setup)
               (get
                'ac-php-core-eldoc-setup
                'function-documentation))"##;
    let expect = expect![[
        r#"OK (nil t ("ac-php-core") (ac-php) t t t (autoload "ac-php-core" "Enable the ElDoc support for the PHP language.\nConfigure the variable `eldoc-documentation-function' and\ncall the command `eldoc-mode'." t nil) nil)"#
    ]];

    assert_ac_php_core_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_packaged_elisp_and_json_data_assets_have_exact_hashes() {
    let elisp_form = r##"(mapcar
               (lambda (file)
                 (let ((path
                        (expand-file-name
                         file
                         ac-php-root-directory)))
                   (with-temp-buffer
                     (insert-file-contents-literally
                      path)
                     (list
                      file
                      (buffer-size)
                      (secure-hash
                       'sha256
                       (current-buffer))))))
               '("ac-php-comm-tags-data.el"
                 "ac-php-comm-tags-data.json"))"##;
    let expect = expect![[
        r#"OK (("ac-php-comm-tags-data.el" 1 "01ba4719c80b6fe911b091a7c05124b64eeece964e09c058ef8f9805daca546b") ("ac-php-comm-tags-data.json" 0 "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_phpctags_literal_binary_buffer_hash_matches_gnu() {
    let elisp_form = r##"(let ((path
                    (expand-file-name
                     "phpctags"
                     ac-php-root-directory)))
               (with-temp-buffer
                 (insert-file-contents-literally
                  path)
                 (list
                  (buffer-size)
                  (string-bytes
                   (buffer-string))
                  enable-multibyte-characters
                  (secure-hash
                   'sha256
                   (current-buffer)))))"##;
    let expect = expect![[
        r#"OK (3490904 3492570 t "95fe7c745b57803e1dfc1cb15c9fc7d178cfeb54b42717f05e5b9e1cadce609c")"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_parser_search_and_completion_callable_surface_matches() {
    let elisp_form = r##"(let ((functions
                    '(ac-php--get-timestamp
                      ac-php--reduce-path
                      ac-php-g--project-root-dir
                      ac-php--in-comment-p
                      ac-php--in-string-or-comment-p
                      ac-php--beginning-of-defun
                      ac-php--end-of-defun
                      ac-php--in-function-p
                      ac-php-toggle-debug
                      ac-php-mode-line-project-status
                      ac-php-location-stack-push
                      ac-php-goto-line-col
                      ac-php--get-common-json-file
                      ac-php-current-location
                      ac-php--string=-ignore-care
                      ac-php-find-file-or-buffer
                      ac-php-goto-location
                      ac-php-clean-document
                      ac-php--tag-name-is-function
                      ac-php-split-string-with-separator
                      ac-php--get-clean-node
                      ac-php--get-node-parser-data
                      ac-php--get-key-list-from-parser-data
                      ac-php-remove-unnecessary-items-4-complete-method
                      ac-php--get-class-full-name-in-cur-buffer
                      ac-php-split-line-4-complete-method
                      ac-php-get-syntax-backward
                      ac-php-get-cur-class-name
                      ac-php-get-cur-namespace-name
                      ac-php-clean-namespace-name
                      ac-php-get-cur-full-class-name
                      ac-php-get-use-as-name
                      ac-php--get-all-use-as-name-in-cur-buffer
                      ac-php-get-annotated-var-class
                      ac-php-get-class-at-point
                      ac-php-candidate-class
                      ac-php--get-item-from-funtion-map
                      ac-php-candidate-other
                      ac-php--get-cur-function-vars)))
               (list
                (mapcar
                 (lambda (function)
                   (list
                    function
                    (help-function-arglist
                     function t)
                    (interactive-form
                     function)
                    (macrop function)))
                 functions)
                (secure-hash
                 'sha256
                 (mapconcat
                  (lambda (function)
                    (or
                     (documentation
                      function t)
                     ""))
                  functions
                  "\0"))))"##;
    let expect = expect![[
        r#"OK (((ac-php--get-timestamp (time-spec) nil nil) (ac-php--reduce-path (path max-len) nil nil) (ac-php-g--project-root-dir (tags-data) nil nil) (ac-php--in-comment-p (&optional pos) nil nil) (ac-php--in-string-or-comment-p (&optional pos) nil nil) (ac-php--beginning-of-defun (&optional arg) nil nil) (ac-php--end-of-defun (&optional arg) nil nil) (ac-php--in-function-p (&optional pos) nil nil) (ac-php-toggle-debug nil (interactive nil) nil) (ac-php-mode-line-project-status nil nil nil) (ac-php-location-stack-push nil nil nil) (ac-php-goto-line-col (line column) nil nil) (ac-php--get-common-json-file nil nil nil) (ac-php-current-location (&optional offset) nil nil) (ac-php--string=-ignore-care (str1 str2) nil nil) (ac-php-find-file-or-buffer (file-or-buffer &optional other-window) nil nil) (ac-php-goto-location (location &optional other-window) nil nil) (ac-php-clean-document (s) nil nil) (ac-php--tag-name-is-function (tag-name) nil nil) (ac-php-split-string-with-separator (str regexp &optional replacement omit-nulls) nil nil) (ac-php--get-clean-node (parser-data &optional check-len) nil nil) (ac-php--get-node-parser-data (parser-data) nil nil) (ac-php--get-key-list-from-parser-data (parser-data) nil nil) (ac-php-remove-unnecessary-items-4-complete-method (splited-line-items) nil nil) (ac-php--get-class-full-name-in-cur-buffer (first-key function-map get-return-type-flag) nil nil) (ac-php-split-line-4-complete-method (line-string) nil nil) (ac-php-get-syntax-backward (regexp &rest args) nil nil) (ac-php-get-cur-class-name nil nil nil) (ac-php-get-cur-namespace-name (&optional trim-trailing-backslash-p) nil nil) (ac-php-clean-namespace-name (namespace-name) nil nil) (ac-php-get-cur-full-class-name nil nil nil) (ac-php-get-use-as-name (item-name) nil nil) (ac-php--get-all-use-as-name-in-cur-buffer nil nil nil) (ac-php-get-annotated-var-class (variable &optional pos) nil nil) (ac-php-get-class-at-point (tags-data &optional pos) nil nil) (ac-php-candidate-class (tags-data key-str-list) nil nil) (ac-php--get-item-from-funtion-map (key-word function-map) nil nil) (ac-php-candidate-other (tags-data) nil nil) (ac-php--get-cur-function-vars nil nil nil)) "f819369fdeb0c6793b8cb84a0a7dc4f2c35e07a239653a1dfdf62f148aade465")"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_index_storage_and_inheritance_callable_surface_matches() {
    let elisp_form = r##"(let ((functions
                    '(ac-php-find-php-files
                      ac-php--clean-return-type
                      ac-php--json-save-data
                      ac-php--cache-files-save
                      ac-php--ctags-opts
                      ac-php--rebuild-file-list
                      ac-php-phptags-index-process-filter
                      ac-php--remake-tags
                      ac-php--remake-tags-ex
                      ac-php-gen-el-func
                      ac-php--get-tags-save-dir
                      ac-php-get-tags-file
                      ac-php--get-config-path-noti-str
                      ac-php--get-config
                      ac-php--get-use-cscope-from-config-file
                      ac-php-remake-tags
                      ac-php-remake-tags-all
                      ac-php--remake-cscope
                      ac-php--get-obj-tags-dir
                      ac-php--get-obj-tags-file-list
                      ac-php-save-data
                      case-fold-string=
                      case-fold-string-hash
                      ac-php-load-data
                      ac-php-g--class-map
                      ac-php-g--function-map
                      ac-php-g--inherit-map
                      ac-php-g--file-list
                      ac-php-get-tags-data
                      ac-php--get-project-root-dir
                      ac-php--get-check-class-list
                      ac-php--check-global-name
                      ac-php--as-global-name
                      ac-php--get-check-class-list-ex
                      ac-php--get-item-info
                      ac-php-get-class-member-info
                      ac-php-get-class-member-list
                      ac-php--get-class-name-from-parent-define
                      ac-php-get-class-name-by-key-list
                      ac-php--get-namespace-from-classname)))
               (list
                (mapcar
                 (lambda (function)
                   (list
                    function
                    (help-function-arglist
                     function t)
                    (interactive-form
                     function)
                    (macrop function)))
                 functions)
                (secure-hash
                 'sha256
                 (mapconcat
                  (lambda (function)
                    (or
                     (documentation
                      function t)
                     ""))
                  functions
                  "\0"))))"##;
    let expect = expect![[
        r#"OK (((ac-php-find-php-files (project-root-dir regex also-find-subdir) nil nil) (ac-php--clean-return-type (return-type) nil nil) (ac-php--json-save-data (conf-file data-list) nil nil) (ac-php--cache-files-save (file-path cache1-files) nil nil) (ac-php--ctags-opts (project-root-dir rebuild) nil nil) (ac-php--rebuild-file-list (project-root-dir save-tags-dir rebuild) nil nil) (ac-php-phptags-index-process-filter (process strings) nil nil) (ac-php--remake-tags (project-root-dir force) nil nil) (ac-php--remake-tags-ex (project-root-dir force) nil nil) (ac-php-gen-el-func (doc) nil nil) (ac-php--get-tags-save-dir (project-root-dir) nil nil) (ac-php-get-tags-file nil nil nil) (ac-php--get-config-path-noti-str (project-root-dir path-str) nil nil) (ac-php--get-config (project-root-dir) nil nil) (ac-php--get-use-cscope-from-config-file (project-root-dir) nil nil) (ac-php-remake-tags nil (interactive nil) nil) (ac-php-remake-tags-all nil (interactive nil) nil) (ac-php--remake-cscope (project-root-dir all-file-list) nil nil) (ac-php--get-obj-tags-dir (save-tags-dir) nil nil) (ac-php--get-obj-tags-file-list (save-tags-dir) nil nil) (ac-php-save-data (file data) nil nil) (case-fold-string= (a b) nil nil) (case-fold-string-hash (a) nil nil) (ac-php-load-data (tags-file tags-vendor-file project-root-dir) nil nil) (ac-php-g--class-map (tags-data) nil nil) (ac-php-g--function-map (tags-data) nil nil) (ac-php-g--inherit-map (tags-data) nil nil) (ac-php-g--file-list (tags-data) nil nil) (ac-php-get-tags-data nil nil nil) (ac-php--get-project-root-dir nil nil nil) (ac-php--get-check-class-list (class-name inherit-map class-map) nil nil) (ac-php--check-global-name (name) nil nil) (ac-php--as-global-name (name) nil nil) (ac-php--get-check-class-list-ex (class-name parent-namespace inherit-map class-map cur-list) nil nil) (ac-php--get-item-info (member) nil nil) (ac-php-get-class-member-info (class-map inherit-map class-name member) nil nil) (ac-php-get-class-member-list (class-map inherit-map class-name) nil nil) (ac-php--get-class-name-from-parent-define (parent-list-str) nil nil) (ac-php-get-class-name-by-key-list (tags-data key-list-str) nil nil) (ac-php--get-namespace-from-classname (classname) nil nil)) "c4f3eb8aca1c75caa5d2030a33b1577ffce5c8f17ce72fcbc62df160c531ef77")"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_navigation_display_and_mode_callable_surface_matches() {
    let elisp_form = r##"(let ((functions
                    '(ac-php-find-symbol-at-point-pri
                      ac-php--goto-local-var-def
                      ac-php-find-symbol-at-point
                      ac-php-gen-def
                      ac-php-location-stack-forward
                      ac-php-location-stack-back
                      ac-php-location-stack-jump
                      ac-php--get-array-string
                      ac-php-candidate
                      ac-php--get-cur-word
                      ac-php--get-cur-word-with-function-flag
                      ac-php-get-cur-word-with-dollar
                      ac-php-get-cur-word-without-clean
                      ac-php-show-tip
                      ac-php-cscope-find-egrep-pattern
                      ac-php-eldoc-documentation-function
                      ac-php-show-cur-project-info
                      ac-php-mode
                      ac-php-core-eldoc-setup)))
               (list
                (mapcar
                 (lambda (function)
                   (list
                    function
                    (help-function-arglist
                     function t)
                    (interactive-form
                     function)
                    (macrop function)))
                 functions)
                (secure-hash
                 'sha256
                 (mapconcat
                  (lambda (function)
                    (or
                     (documentation
                      function t)
                     ""))
                  functions
                  "\0"))
                (list
                 (macrop
                  'ac-php--debug)
                 (help-function-arglist
                  'ac-php--debug t)
                 (documentation
                  'ac-php--debug t))))"##;
    let expect = expect![[
        r#"OK (((ac-php-find-symbol-at-point-pri (tags-data &optional as-fn-p as-id-p) nil nil) (ac-php--goto-local-var-def (local-var) nil nil) (ac-php-find-symbol-at-point (&optional prefix) (interactive "P") nil) (ac-php-gen-def nil (interactive nil) nil) (ac-php-location-stack-forward nil (interactive nil) nil) (ac-php-location-stack-back nil (interactive nil) nil) (ac-php-location-stack-jump (by) nil nil) (ac-php--get-array-string (arr arr-len index) nil nil) (ac-php-candidate nil nil nil) (ac-php--get-cur-word nil nil nil) (ac-php--get-cur-word-with-function-flag nil nil nil) (ac-php-get-cur-word-with-dollar nil nil nil) (ac-php-get-cur-word-without-clean nil nil nil) (ac-php-show-tip (&optional prefix) (interactive "P") nil) (ac-php-cscope-find-egrep-pattern (symbol) (interactive (list (let (cscope-no-mouse-prompts) (cscope-prompt-for-symbol "Find this egrep pattern " nil t t)))) nil) (ac-php-eldoc-documentation-function nil (interactive "P") nil) (ac-php-show-cur-project-info nil (interactive nil) nil) (ac-php-mode (&optional arg) (interactive (list (if current-prefix-arg (prefix-numeric-value current-prefix-arg) 'toggle))) nil) (ac-php-core-eldoc-setup nil (interactive nil) nil)) "8a8c3a9156ba75617d5507585d5411171a2fc91472ba86cfc809bee322765771" (t (format-string &rest args) "Display a debug message at the bottom of the screen.\nThe message also goes into the ‘*Messages*’ buffer, if ‘message-log-max’\nis non-nil.  Return the debug message.  For FORMAT-STRING and ARGS explanation\nrefer to `message' function."))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

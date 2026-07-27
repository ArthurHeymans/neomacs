use expect_test::expect;

use super::assert_apples_mode_parity;

#[test]
fn installed_descriptor_source_and_feature_identify_the_exact_melpa_build() {
    let elisp_form = r##"(let* ((descriptor (cadr (assq 'apples-mode package-alist)))
                     (source (getenv "NEOMACS_PACKAGE_SOURCE")))
                (list
                 (featurep 'apples-mode)
                 (package-desc-name descriptor)
                 (package-version-join (package-desc-version descriptor))
                 (package-desc-reqs descriptor)
                 (package-desc-summary descriptor)
                 (file-name-nondirectory source)
                 (file-name-nondirectory
                  (symbol-file 'apples-mode 'defun))
                 apples-mode-version))"##;
    let expect = expect![[
        r#"OK (t apples-mode "20110121.418" nil "Major mode for editing and executing AppleScript code." "apples-mode.el" "apples-mode.el" "0.0.2")"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn complete_callable_surface_has_exact_arglists_command_and_macro_status() {
    let elisp_form = r##"(mapcar
                (lambda (symbol)
                  (list symbol
                        (copy-tree (help-function-arglist symbol t))
                        (commandp symbol)
                        (macrop symbol)))
                '(apples-replace-re-comma->spaces
                  apples-replace-re-space->spaces
                  apples-plist
                  apples-plist-put
                  apples-plist-get
                  apples-tmp-files-setup
                  apples-follow-error-position
                  apples-show-mode-version
                  apples-show-last-result
                  apples-show-last-raw-result
                  apples-delete-overlay
                  apples-error-overlay-setup
                  apples-delete-result
                  apples-display-result
                  apples-parse-error
                  apples-result
                  apples-proc-live-p
                  apples-buffer-string
                  apples-proc-failed-p
                  apples-proc-failed
                  apples-encode-string
                  apples-do-applescript
                  apples-compile
                  apples-handle-decompile
                  apples-decompile
                  apples-quoted-string
                  apples-send-to-applescript-editor
                  apples-run-file
                  apples-run-buffer
                  apples-run-region
                  apples-run-region/buffer
                  apples-run-minibuf
                  apples-open-dict-index
                  apples-continuation-char
                  apples-insert-continuation-char
                  apples-insert-continuation-char-and-newline
                  apples-save-scratch
                  apples-open-scratch
                  apples-lookup-key->key-code
                  apples-lookup-key-code->key
                  apples-comment-or-uncomment-region
                  apples-comment-dwim
                  apples-in-string/comment-p
                  apples-in-string-p
                  apples-ideal-prev-bol
                  apples-leading-word-of-line
                  apples-line-string
                  apples-string-match
                  apples-parse-lines
                  apples-indent-line
                  apples-toggle-indent
                  apples-parse-statement
                  apples-end-completion
                  apples-end-completion-hl
                  apples-end-completion-hl-setup
                  apples-keywords
                  apples-keymap-setup
                  apples-applescript-version
                  apples-show-applescript-version
                  apples-customize-group
                  apples-visit-project
                  apples-mode))"##;
    let expect = expect![
        "OK ((apples-replace-re-comma->spaces (re) nil nil) (apples-replace-re-space->spaces (re) nil nil) (apples-plist t nil nil) (apples-plist-put (prop val) nil nil) (apples-plist-get (prop) nil nil) (apples-tmp-files-setup nil nil nil) (apples-follow-error-position t nil nil) (apples-show-mode-version nil t nil) (apples-show-last-result nil t nil) (apples-show-last-raw-result nil t nil) (apples-delete-overlay (ov) nil nil) (apples-error-overlay-setup nil nil nil) (apples-delete-result (&rest _) nil nil) (apples-display-result (&optional result) nil nil) (apples-parse-error (result) nil nil) (apples-result (result status f/s) nil nil) (apples-proc-live-p (proc) nil nil) (apples-buffer-string (&optional buffer-or-name) nil nil) (apples-proc-failed-p (proc) nil nil) (apples-proc-failed (msg buf) nil nil) (apples-encode-string (str) nil nil) (apples-do-applescript (filename-or-script &optional callback) nil nil) (apples-compile (&optional filename output) t nil) (apples-handle-decompile (script filename) nil nil) (apples-decompile (filename) t nil) (apples-quoted-string (str) nil nil) (apples-send-to-applescript-editor nil t nil) (apples-run-file (&optional filename) t nil) (apples-run-buffer (&optional buffer-or-name) t nil) (apples-run-region (beg end) t nil) (apples-run-region/buffer nil t nil) (apples-run-minibuf (script) t nil) (apples-open-dict-index nil t nil) (apples-continuation-char nil nil nil) (apples-insert-continuation-char nil t nil) (apples-insert-continuation-char-and-newline nil t nil) (apples-save-scratch nil nil nil) (apples-open-scratch nil t nil) (apples-lookup-key->key-code nil t nil) (apples-lookup-key-code->key (key-code) t nil) (apples-comment-or-uncomment-region (beg end &optional arg) t nil) (apples-comment-dwim (arg) t nil) (apples-in-string/comment-p (&optional pos) nil nil) (apples-in-string-p (&optional pos) nil nil) (apples-ideal-prev-bol nil nil nil) (apples-leading-word-of-line nil nil nil) (apples-line-string nil nil nil) (apples-string-match (regexps string) nil nil) (apples-parse-lines nil nil nil) (apples-indent-line nil t nil) (apples-toggle-indent nil t nil) (apples-parse-statement nil nil nil) (apples-end-completion nil t nil) (apples-end-completion-hl (bol bword eword) nil nil) (apples-end-completion-hl-setup nil nil nil) (apples-keywords (&optional type) nil nil) (apples-keymap-setup nil nil nil) (apples-applescript-version nil nil nil) (apples-show-applescript-version nil t nil) (apples-customize-group nil t nil) (apples-visit-project nil t nil) (apples-mode nil t nil))"
    ];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn configuration_defaults_and_core_data_have_an_exact_contract() {
    let elisp_form = r##"(list
                apples-identifier
                apples-tmp-dir
                apples-prefer-coding-system
                apples-compile-create-file-flag
                apples-decompile-callback
                apples-decompile-query
                apples-continuation-char
                apples-indent-offset
                apples-continuation-offset
                apples-indenters
                apples-deindenters
                apples-indent-regexps
                apples-noindent-regexps
                apples-keymap
                apples-underline-syntax-class
                apples-end-completion-hl
                apples-end-completion-hl-duration
                apples-statements
                (length apples-key-codes)
                (length (apples-keywords))
                (mapcar
                 (lambda (type)
                   (cons type (length (apples-keywords type))))
                 '(reserved-words statements commands operators
                   handler-parameter-labels standard-folders)))"##;
    let expect = expect![[
        r#"OK ("\\(?:\\sw\\|\\s_\\)+" nil nil nil apples-handle-decompile nil 172 4 4 ("considering" "else" "if" "ignoring" "on" "repeat" "tell" "try") ("else" "end") ("^script\\s-+\\<" "^using\\s-+terms\\s-+from" "^with\\s-+timeout" "^with\\s-+transaction") ("^if\\>.+\\<then\\s-+\\<" "^tell\\>.+\\<to\\s-+\\<") (("<S-tab>" . apples-toggle-indent) ("C-c t r" . apples-run-region/buffer) ("C-c t k" . apples-compile) ("C-c t d" . apples-decompile) ("C-c t 3" . apples-show-last-result) ("C-c t l" . apples-insert-continuation-char) ("C-c t RET" . apples-insert-continuation-char-and-newline) ("C-c t o" . apples-open-dict-index) ("C-c t s" . apples-send-to-applescript-editor) ("C-c t e" . apples-end-completion)) nil words 0.3 (("considering" . "considering") ("ignoring" . "ignoring") ("try" . "try") ("if" . "if") ("repeat" . "repeat") ("tell" . "tell") ("using terms from" . "using terms from") ("with timeout" . "timeout") ("with transaction" . "transaction") ("on adding folder items to" . "adding folder items to") ("on closing folder window for" . "closing folder window for") ("on moving folder window for" . "moving folder window for") ("on opening folder" . "opening folder") ("on removing folder items from" . "removing folder items from")) 57 303 ((reserved-words . 103) (statements . 28) (commands . 64) (operators . 82) (handler-parameter-labels . 25) (standard-folders . 1)))"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn all_distributed_snippet_assets_have_exact_names_sizes_and_content_hashes() {
    let elisp_form = r##"(let* ((source (getenv "NEOMACS_PACKAGE_SOURCE"))
                     (directory
                      (expand-file-name
                       "apples-mode"
                       (file-name-directory source)))
                     (files
                      (sort
                       (directory-files directory t "^[^.]" t)
                       #'string<)))
                (mapcar
                 (lambda (file)
                   (with-temp-buffer
                     (insert-file-contents-literally file)
                     (list
                      (file-name-nondirectory file)
                      (buffer-size)
                      (secure-hash 'sha256 (current-buffer)))))
                 files))"##;
    let expect = expect![[
        r#"OK (("comment" 53 "5136fd93762a72510133e5ca0650d96377caed07968b13ede7184194d303aabc") ("considering" 82 "29275ff72c3846e26bbde635c81aa28f47623560b612bfe62227caf5256e315a") ("considering-application-responses" 123 "5e0a323cc0c1de3451ca8e497ab9f48e1ea3296362746aa434a3034b629b4ae5") ("display-dialog" 71 "b59b323023b32a683936419709239aac9d5d204201724356a36aa9b53f43baa1") ("if" 68 "deb3971cadbf936a5a20b2fee939b029b475298b8f542ad4834b59f1381ed0f2") ("ignoring" 73 "48f9b84a55209ce836bdaa8b12f7a1ba709a178226022a7f3dc82da698d0a692") ("ignoring-application-responses" 114 "18ea18bfff2314f2536511a3bd62016f25705ab4a78786f3b46f98b940b7ffe4") ("on" 55 "9e356e5c4c055475d9fefb324456bdd702c3b5160b11087a0ddc8ef0a35f4bf7") ("repeat" 67 "3352fa3f1855bff0fcb84b32eeb6eefac13ab5fef701a6c00ad0f2acb0f76ec1") ("repeat-until" 79 "37d568b5aac27ddb72d031d4c21b52d6f8e89f075ca92d67b84ccbfbbca0af32") ("repeat-while" 79 "6708c7fbb4dd0821f20d27e4405f72e4e63a14d3a912965774401d2196eeef97") ("repeat-with" 77 "846f04c69a98a957fc36d7273a048fd66df779121f0e8048a5c620a5fcbb6150") ("tell-application" 87 "79121053afd3b62992b447e0a134df657e19eca442b67439774121748998ae2e") ("tell-application-to-activate" 102 "3a92fc2dca4f362cc0a9c68edac5734dd71fca5aefcbbc0fb4517aa628d995d5") ("try" 70 "f6c52d671f250f4541e1cc60f0e0949e0979688983a2cc891bedcb9ad429186d") ("using-terms-from-application" 123 "9818dace449733de865f59722987887f82e085e32fce030468b9858343b9c296") ("with-timeout-of-seconds" 102 "d4f447de9c37c05907f3545e640ee949ab8bfc23de8c0273f537b9a718698bd9") ("with-transaction" 89 "3b89cfae4427c77cadf5e27d64725d21e580ef4d193e7e50589ca1626c09a728"))"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn faces_keep_their_documentation_inheritance_and_default_attributes() {
    let elisp_form = r##"(mapcar
                (lambda (face)
                  (list
                   face
                   (face-documentation face)
                   (face-attribute face :inherit nil 'default)
                   (face-attribute face :foreground nil 'default)
                   (face-attribute face :background nil 'default)
                   (face-attribute face :underline nil 'default)
                   (face-attribute face :weight nil 'default)))
                '(apples-statements apples-commands apples-operators
                  apples-labels apples-records apples-reserved-words
                  apples-error apples-standard-folders
                  apples-continuation-char apples-error-highlight
                  apples-result-prompt apples-error-prompt
                  apples-end-completion))"##;
    let expect = expect![[
        r#"OK ((apples-statements "Face for statements." font-lock-keyword-face "unspecified-fg" "unspecified-bg" nil bold) (apples-commands "Face for commands." font-lock-keyword-face "unspecified-fg" "unspecified-bg" nil bold) (apples-operators "Face for operators." font-lock-type-face "unspecified-fg" "unspecified-bg" t bold) (apples-labels "Face for labels." font-lock-type-face "unspecified-fg" "unspecified-bg" t bold) (apples-records "Face for records." font-lock-builtin-face "unspecified-fg" "unspecified-bg" nil bold) (apples-reserved-words "Face for reserved words." font-lock-keyword-face "unspecified-fg" "unspecified-bg" nil bold) (apples-error "Face for error." font-lock-warning-face "unspecified-fg" "unspecified-bg" nil bold) (apples-standard-folders "Face for standard folders." font-lock-constant-face "unspecified-fg" "unspecified-bg" t bold) (apples-continuation-char "Face for continuation char." escape-glyph "cyan" "unspecified-bg" nil normal) (apples-error-highlight "Face for error highlight." nil "unspecified-fg" "DeepPink3" nil normal) (apples-result-prompt "Face for result prompt." minibuffer-prompt "cyan" "unspecified-bg" nil normal) (apples-error-prompt "Face for error prompt." font-lock-warning-face "unspecified-fg" "unspecified-bg" nil bold) (apples-end-completion "Face for end completion." apples-error-highlight "unspecified-fg" "DeepPink3" nil normal))"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

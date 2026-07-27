use expect_test::expect;

use super::{assert_anzu_autoload_parity, assert_anzu_parity};

#[test]
fn anzu_registers_exact_feature_custom_defaults_faces_and_internal_state() {
    let elisp_form = r##"(list
         (featurep 'anzu)
         (get 'anzu 'custom-group)
         (mapcar
          (lambda (symbol)
            (list symbol (symbol-value symbol)
                  (get symbol 'custom-type)))
          '(anzu-mode-lighter anzu-cons-mode-line-p
            anzu-minimum-input-length anzu-search-threshold
            anzu-replace-threshold anzu-use-migemo
            anzu-mode-line-update-function
            anzu-regexp-search-commands anzu-input-idle-delay
            anzu-deactivate-region anzu-replace-at-cursor-thing
            anzu-replace-to-string-separator))
         (mapcar
          (lambda (symbol)
            (list symbol (facep symbol)
                  (get symbol 'face-defface-spec)))
          '(anzu-mode-line anzu-mode-line-no-match
            anzu-replace-highlight anzu-match-1 anzu-match-2
            anzu-match-3 anzu-replace-to))
         (mapcar
          (lambda (symbol)
            (list symbol (symbol-value symbol)))
          '(anzu--total-matched anzu--current-position
            anzu--overflow-p anzu--last-isearch-string
            anzu--cached-positions anzu--last-command anzu--state
            anzu--cached-count anzu--last-replace-input
            anzu--last-search-state anzu--last-replaced-count
            anzu--outside-point anzu--history anzu--query-defaults
            anzu--region-noncontiguous anzu--update-timer
            anzu--replaced-markers)))"##;
    let expect = expect![[
        r#"OK (t ((anzu-mode-lighter custom-variable) (anzu-cons-mode-line-p custom-variable) (anzu-minimum-input-length custom-variable) (anzu-search-threshold custom-variable) (anzu-replace-threshold custom-variable) (anzu-use-migemo custom-variable) (anzu-mode-line-update-function custom-variable) (anzu-regexp-search-commands custom-variable) (anzu-input-idle-delay custom-variable) (anzu-deactivate-region custom-variable) (anzu-replace-at-cursor-thing custom-variable) (anzu-replace-to-string-separator custom-variable) (anzu-mode-line custom-face) (anzu-mode-line-no-match custom-face) (anzu-replace-highlight custom-face) (anzu-match-1 custom-face) (anzu-match-2 custom-face) (anzu-match-3 custom-face) (anzu-replace-to custom-face) (global-anzu-mode custom-variable)) ((anzu-mode-lighter " Anzu" string) (anzu-cons-mode-line-p t boolean) (anzu-minimum-input-length 1 integer) (anzu-search-threshold 1000 (choice (integer :tag "Threshold of search") (const :tag "No threshold" nil))) (anzu-replace-threshold 1000 (choice (integer :tag "Threshold of replacement overlays") (const :tag "No threshold" nil))) (anzu-use-migemo nil boolean) (anzu-mode-line-update-function anzu--update-mode-line-default function) (anzu-regexp-search-commands (isearch-forward-regexp isearch-backward-regexp) (repeat function)) (anzu-input-idle-delay 0.05 number) (anzu-deactivate-region nil boolean) (anzu-replace-at-cursor-thing defun symbol) (anzu-replace-to-string-separator "" string)) ((anzu-mode-line [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:foreground "magenta" :weight bold)))) (anzu-mode-line-no-match [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:inherit anzu-mode-line)))) (anzu-replace-highlight [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t :inherit query-replace))) (anzu-match-1 [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((((class color) (background light)) :background "aquamarine" :foreground "black") (((class color) (background dark)) :background "limegreen" :foreground "black") (t :inverse-video t))) (anzu-match-2 [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((((class color) (background light)) :background "springgreen" :foreground "black") (((class color) (background dark)) :background "yellow" :foreground "black") (t :inverse-video t))) (anzu-match-3 [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((((class color) (background light)) :background "yellow" :foreground "black") (((class color) (background dark)) :background "aquamarine" :foreground "black") (t :inverse-video t))) (anzu-replace-to [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((((class color) (background light)) :foreground "red") (((class color) (background dark)) :foreground "yellow")))) ((anzu--total-matched 0) (anzu--current-position 0) (anzu--overflow-p nil) (anzu--last-isearch-string nil) (anzu--cached-positions nil) (anzu--last-command nil) (anzu--state nil) (anzu--cached-count 0) (anzu--last-replace-input "") (anzu--last-search-state nil) (anzu--last-replaced-count nil) (anzu--outside-point nil) (anzu--history nil) (anzu--query-defaults nil) (anzu--region-noncontiguous nil) (anzu--update-timer nil) (anzu--replaced-markers nil)))"#
    ]];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_complete_callable_surface_arities_and_command_flags_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list symbol (fboundp symbol) (commandp symbol)
                 (help-function-arglist symbol t)))
         '(anzu--validate-regexp anzu--construct-position-info
           anzu--case-fold-search anzu--word-search-p
           anzu--isearch-regexp-function anzu--transform-input
           anzu--use-migemo-p anzu--search-all-position
           anzu--where-is-here anzu--use-result-cache-p anzu--update
           anzu--update-post-hook anzu--mode-line-not-set-p
           anzu--cons-mode-line-search anzu--cons-mode-line
           anzu--reset-status anzu--reset-mode-line
           anzu--format-here-position anzu--update-mode-line-default
           anzu--update-mode-line anzu-mode anzu--turn-on
           global-anzu-mode anzu--query-prompt-base anzu--query-prompt
           anzu--set-marker anzu--make-overlay
           anzu--add-match-group-overlay anzu--add-overlay
           anzu--cleanup-markers anzu2--put-overlay-p
           anzu--convert-for-lax-whitespace
           anzu--count-and-highlight-matched
           anzu--search-outside-visible anzu--separator
           anzu--check-minibuffer-input anzu--clear-overlays
           anzu--transform-from-to-history anzu--read-from-string
           anzu--query-validate-from-regexp anzu--query-from-string
           anzu--compile-replace-text anzu--evaluate-occurrence
           anzu--overlay-sort anzu--overlays-in-range
           anzu--propertize-to-string anzu--replaced-literal-string
           anzu--append-replaced-string anzu--outside-overlay-limit
           anzu--read-to-string anzu--query-replace-read-to
           anzu--overlay-limit anzu--query-from-at-cursor
           anzu--query-from-isearch-string anzu--thing-begin
           anzu--thing-end anzu--region-begin anzu--line-end-position
           anzu--region-end anzu--begin-thing
           anzu--replace-backward-p
           anzu--construct-perform-replace-arguments
           anzu--construct-query-replace-arguments
           anzu--current-replaced-index anzu--set-replaced-markers
           anzu--query-replace-common anzu-query-replace-at-cursor
           anzu-query-replace-at-cursor-thing anzu-query-replace
           anzu-query-replace-regexp anzu-replace-at-cursor-thing
           anzu--isearch-query-replace-common
           anzu-isearch-query-replace
           anzu-isearch-query-replace-regexp))"##;
    let expect = expect![
        "OK ((anzu--validate-regexp t nil (regexp)) (anzu--construct-position-info t nil (count overflow positions)) (anzu--case-fold-search t nil nil) (anzu--word-search-p t nil nil) (anzu--isearch-regexp-function t nil nil) (anzu--transform-input t nil (str)) (anzu--use-migemo-p t nil nil) (anzu--search-all-position t nil (str)) (anzu--where-is-here t nil (positions here)) (anzu--use-result-cache-p t nil (input)) (anzu--update t nil (query)) (anzu--update-post-hook t nil nil) (anzu--mode-line-not-set-p t nil nil) (anzu--cons-mode-line-search t nil nil) (anzu--cons-mode-line t nil (state)) (anzu--reset-status t nil nil) (anzu--reset-mode-line t nil nil) (anzu--format-here-position t nil (here total)) (anzu--update-mode-line-default t nil (here total)) (anzu--update-mode-line t nil nil) (anzu-mode t t (&optional arg)) (anzu--turn-on t nil nil) (global-anzu-mode t t (&optional arg)) (anzu--query-prompt-base t nil (use-region use-regexp)) (anzu--query-prompt t nil (use-region use-regexp at-cursor isearch-p)) (anzu--set-marker t nil (beg buf)) (anzu--make-overlay t nil (begin end face prio)) (anzu--add-match-group-overlay t nil (match-data groups)) (anzu--add-overlay t nil (beg end)) (anzu--cleanup-markers t nil nil) (anzu2--put-overlay-p t nil (beg end overlay-beg overlay-end)) (anzu--convert-for-lax-whitespace t nil (str use-regexp)) (anzu--count-and-highlight-matched t nil (buf str replace-beg replace-end use-regexp overlay-limit case-sensitive)) (anzu--search-outside-visible t nil (buf input beg end use-regexp)) (anzu--separator t nil nil) (anzu--check-minibuffer-input t nil (buf beg end use-regexp overlay-limit)) (anzu--clear-overlays t nil (buf beg end)) (anzu--transform-from-to-history t nil nil) (anzu--read-from-string t nil (prompt beg end use-regexp overlay-limit)) (anzu--query-validate-from-regexp t nil (from)) (anzu--query-from-string t nil (prompt beg end use-regexp overlay-limit)) (anzu--compile-replace-text t nil (str)) (anzu--evaluate-occurrence t nil (ov to-regexp replacements fixed-case from-regexp)) (anzu--overlay-sort t nil (a b)) (anzu--overlays-in-range t nil (beg end)) (anzu--propertize-to-string t nil (str)) (anzu--replaced-literal-string t nil (ov replaced from)) (anzu--append-replaced-string t nil (content buf beg end use-regexp overlay-limit from)) (anzu--outside-overlay-limit t nil (orig-beg orig-limit)) (anzu--read-to-string t nil (from prompt beg end use-regexp overlay-limit)) (anzu--query-replace-read-to t nil (from prompt beg end use-regexp overlay-limit)) (anzu--overlay-limit t nil (backward)) (anzu--query-from-at-cursor t nil (buf beg end overlay-limit)) (anzu--query-from-isearch-string t nil (buf beg end use-regexp overlay-limit)) (anzu--thing-begin t nil (thing)) (anzu--thing-end t nil (thing)) (anzu--region-begin t nil (use-region thing backward)) (anzu--line-end-position t nil (num)) (anzu--region-end t nil (use-region thing backward)) (anzu--begin-thing t nil (at-cursor thing)) (anzu--replace-backward-p t nil (prefix)) (anzu--construct-perform-replace-arguments t nil (from to delimited beg end backward query)) (anzu--construct-query-replace-arguments t nil (from to delimited beg end backward)) (anzu--current-replaced-index t nil (curpoint)) (anzu--set-replaced-markers t nil (from beg end use-regexp)) (anzu--query-replace-common t nil (use-regexp &rest --cl-rest--)) (anzu-query-replace-at-cursor t t nil) (anzu-query-replace-at-cursor-thing t t nil) (anzu-query-replace t t (arg)) (anzu-query-replace-regexp t t (arg)) (anzu-replace-at-cursor-thing t t nil) (anzu--isearch-query-replace-common t nil (use-regexp arg)) (anzu-isearch-query-replace t t (arg)) (anzu-isearch-query-replace-regexp t t (arg)))"
    ];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_descriptor_records_exact_pin_requirement_and_installed_payload() {
    let elisp_form = r##"(let* ((description (cadr (assq 'anzu package-alist)))
               (directory (package-desc-dir description)))
         (list
          (package-desc-name description)
          (package-version-join (package-desc-version description))
          (package-desc-kind description)
          (package-desc-summary description)
          (package-desc-reqs description)
          (sort
           (mapcar
            (lambda (file)
              (let ((relative (file-relative-name file directory)))
                (list relative
                      (file-attribute-size (file-attributes file))
                      (secure-hash 'sha256 file))))
            (directory-files-recursively directory "." nil))
           (lambda (a b) (string< (car a) (car b))))))"##;
    let expect = expect![[
        r#"OK (anzu "20240929.201" nil "Show number of matches in mode-line while searching." ((emacs (25 1))) (("README-elpa" 368 "e720bbf5f3f77125009d15bea6e28539aa2f360fc1ec81d83e47303c8266571c") ("anzu-autoloads.el" 2945 "2b235b7f0356023ce765cee549ef68a563c1bc45e6154768f2246b61121bfa95") ("anzu-pkg.el" 411 "9cf0841afc7cec3c0425c5aaac918347aed316e163010d8615d0e67b91bc9949") ("anzu.el" 36890 "19fbf38c3e6a5aa577a94cc8938abdfcd0f877b79f19df63c74b4fae02db33c4") ("anzu.elc" 38688 "3a655c71d5fa933efa77561cf40def476d360e77582db17c2e858f23d7b80918")))"#
    ]];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_autoloads_expose_exact_public_modes_and_replace_commands() {
    let elisp_form = r##"(list
         (featurep 'anzu)
         (mapcar
          (lambda (symbol)
            (list symbol (fboundp symbol) (commandp symbol)
                  (autoloadp (symbol-function symbol))
                  (symbol-function symbol)))
          '(anzu-mode global-anzu-mode anzu-query-replace-at-cursor
            anzu-query-replace-at-cursor-thing anzu-query-replace
            anzu-query-replace-regexp anzu-replace-at-cursor-thing
            anzu-isearch-query-replace
            anzu-isearch-query-replace-regexp))
         (fboundp 'anzu--search-all-position)
         (boundp 'anzu-search-threshold)
         (boundp 'anzu--state))"##;
    let expect = expect![[
        r#"OK (nil ((anzu-mode t t t (autoload "anzu" "Minor mode which displays the current search's match count in the mode-line.\n\nThis is a minor mode.  If called interactively, toggle the `Anzu mode'\nmode.  If the prefix argument is positive, enable the mode, and if it is\nzero or negative, disable the mode.\n\nIf called from Lisp, toggle the mode if ARG is `toggle'.  Enable the\nmode if ARG is nil, omitted, or is a positive number.  Disable the mode\nif ARG is a negative number.\n\nTo check whether the minor mode is enabled in the current buffer,\nevaluate the variable `anzu-mode'.\n\nThe mode's hook is called both when the mode is enabled and when it is\ndisabled.\n\n(fn &optional ARG)" t nil)) (global-anzu-mode t t t (autoload "anzu" "Toggle Anzu mode in many buffers.\nSpecifically, Anzu mode is enabled in all buffers where\n`anzu--turn-on' would do it.\n\nWith prefix ARG, enable Global Anzu mode if ARG is positive;\notherwise, disable it.\n\nIf called from Lisp, toggle the mode if ARG is `toggle'.\nEnable the mode if ARG is nil, omitted, or is a positive number.\nDisable the mode if ARG is a negative number.\n\nSee `anzu-mode' for more information on Anzu mode.\n\n(fn &optional ARG)" t nil)) (anzu-query-replace-at-cursor t t t (autoload "anzu" "Replace the symbol at point." t nil)) (anzu-query-replace-at-cursor-thing t t t (autoload "anzu" "Replace the thing at point, determined by variable `anzu-replace-at-cursor-thing'." t nil)) (anzu-query-replace t t t (autoload "anzu" "Anzu version of `query-replace'.\n\n(fn ARG)" t nil)) (anzu-query-replace-regexp t t t (autoload "anzu" "Anzu version of `query-replace-regexp'.\n\n(fn ARG)" t nil)) (anzu-replace-at-cursor-thing t t t (autoload "anzu" "Like `anzu-query-replace-at-cursor-thing', but without the query." t nil)) (anzu-isearch-query-replace t t t (autoload "anzu" "Anzu version of `isearch-query-replace'.\n\n(fn ARG)" t nil)) (anzu-isearch-query-replace-regexp t t t (autoload "anzu" "Anzu version of `isearch-query-replace-regexp'.\n\n(fn ARG)" t nil))) nil nil nil)"#
    ]];
    assert_anzu_autoload_parity(elisp_form, expect);
}

#[test]
fn anzu_constants_separator_and_mode_line_form_preserve_properties() {
    let elisp_form = r##"(let ((separator (anzu--separator)))
         (list
          anzu--mode-line-format
          anzu--from-to-separator
          (text-properties-at 0 anzu--from-to-separator)
          separator
          (length separator)
          (text-properties-at 0 separator)
          (get-text-property 0 'display separator)
          (get-text-property 0 'separator separator)))"##;
    let expect = expect![[
        r#"OK ((:eval (anzu--update-mode-line)) #(" → " 0 3 (face minibuffer-prompt)) (face minibuffer-prompt) #("\0" 0 1 (display #(" → " 0 3 (face minibuffer-prompt)) separator t)) 1 (display #(" → " 0 3 (face minibuffer-prompt)) separator t) #(" → " 0 3 (face minibuffer-prompt)) t)"#
    ]];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_replace_highlight_advice_is_installed_once_with_expected_identity() {
    let elisp_form = r##"(list
         (advice-member-p 'anzu-replace-highlight
                          'replace-highlight)
         (advice-mapc
          (lambda (function properties)
            (when (eq function 'anzu-replace-highlight)
              (push properties anzu--history)))
          'replace-highlight)
         anzu--history)"##;
    let expect = expect!["OK (nil nil nil)"];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_reload_preserves_customization_and_does_not_duplicate_feature_or_advice() {
    let elisp_form = r##"(let ((anzu-search-threshold 17)
               (anzu-mode-lighter " Matches")
               (source (getenv "NEOMACS_PACKAGE_SOURCE")))
         (load source nil t t)
         (load source nil t t)
         (let ((advice-count 0))
           (advice-mapc
            (lambda (function _properties)
              (when (eq function 'anzu-replace-highlight)
                (setq advice-count (1+ advice-count))))
            'replace-highlight)
           (list anzu-search-threshold anzu-mode-lighter
                 advice-count
                 (length
                  (cl-remove-if-not
                   (lambda (feature) (eq feature 'anzu))
                   features)))))"##;
    let expect = expect![[r#"OK (17 " Matches" 0 1)"#]];
    assert_anzu_parity(elisp_form, expect);
}

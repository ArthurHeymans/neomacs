use expect_test::expect;

use super::{assert_ag_autoload_parity, assert_ag_parity};

#[test]
fn ag_registry_defaults_custom_metadata_faces_and_safe_local_contract_match() {
    let elisp_form = r##"(list
         (featurep 'ag)
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (symbol-value symbol)
             (get symbol 'custom-type)
             (get symbol 'custom-group)
             (local-variable-if-set-p symbol)
             (get symbol 'safe-local-variable)))
          '(ag-executable
            ag-arguments
            ag-dired-arguments
            ag-context-lines
            ag-group-matches
            ag-highlight-search
            ag-reuse-buffers
            ag-reuse-window
            ag-project-root-function
            ag-ignore-list))
         (mapcar
          (lambda (face)
            (list
             face
             (facep face)
             (get face 'face-defface-spec)
             (get face 'face-documentation)))
          '(ag-hit-face ag-match-face))
         ag-search-finished-hook
         ag/file-column-pattern-nogroup
         ag/file-column-pattern-group)"##;
    let expect = expect![[
        r#"OK (t ((ag-executable "ag" string nil nil nil) (ag-arguments ("--smart-case" "--stats") (repeat (string)) nil nil nil) (ag-dired-arguments ("--nocolor" "-S") (repeat (string)) nil nil nil) (ag-context-lines nil integer nil nil nil) (ag-group-matches t boolean nil nil nil) (ag-highlight-search nil boolean nil nil nil) (ag-reuse-buffers nil boolean nil nil nil) (ag-reuse-window nil boolean nil nil nil) (ag-project-root-function nil (choice (const :tag "Default (VCS root)" nil) (function :tag "Function")) nil nil nil) (ag-ignore-list nil (repeat (string)) nil t listp)) ((ag-hit-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t :inherit compilation-info)) "Face name to use for ag matches.") (ag-match-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t :inherit match)) "Face name to use for ag matches.")) nil "^\\(.+?\\):\\([1-9][0-9]*\\):\\([1-9][0-9]*\\):" "^\\([[:digit:]]+\\):\\([[:digit:]]+\\):")"#
    ]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_complete_callable_surface_arglists_commands_and_macro_status_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (help-function-arglist symbol t)
            (commandp symbol)
            (macrop symbol)
            (autoloadp (symbol-function symbol))))
         '(ag/run-finished-hook
           ag/with-patch-function
           ag/next-error-function
           ag/compilation-match-grouped-filename
           ag-mode
           ag/buffer-name
           ag/format-ignore
           ag/search
           ag/dwim-at-point
           ag/buffer-extension-regex
           ag/longest-string
           ag/replace-first
           vc-svn-root
           ag/project-root
           ag/dired-align-size-column
           ag/dired-filter
           ag/dired-sentinel
           ag/kill-process
           ag/escape-pcre
           ag
           ag-files
           ag-regexp
           ag-project
           ag-project-files
           ag/read-from-minibuffer
           ag-project-regexp
           ag-project-at-point
           ag-regexp-project-at-point
           ag-dired
           ag-dired-regexp
           ag-project-dired
           ag-project-dired-regexp
           ag-kill-buffers
           ag-kill-other-buffers
           ag-filter
           ag/get-supported-types
           ag/read-file-type))"##;
    let expect = expect![
        "OK ((ag/run-finished-hook (buffer how-finished) nil nil nil) (ag/with-patch-function (fun-name fun-args fun-body &rest body) nil t nil) (ag/next-error-function (n &optional reset) nil nil nil) (ag/compilation-match-grouped-filename nil nil nil nil) (ag-mode nil t nil nil) (ag/buffer-name (search-string directory regexp) nil nil nil) (ag/format-ignore (ignores) nil nil nil) (ag/search (string directory &rest --cl-rest--) nil nil nil) (ag/dwim-at-point nil nil nil nil) (ag/buffer-extension-regex nil nil nil nil) (ag/longest-string (&rest strings) nil nil nil) (ag/replace-first (string before after) nil nil nil) (vc-svn-root (file) nil nil nil) (ag/project-root (file-path) nil nil nil) (ag/dired-align-size-column nil nil nil nil) (ag/dired-filter (proc string) nil nil nil) (ag/dired-sentinel (proc state) nil nil nil) (ag/kill-process nil t nil nil) (ag/escape-pcre (regexp) nil nil nil) (ag (string directory) t nil nil) (ag-files (string file-type directory) t nil nil) (ag-regexp (string directory) t nil nil) (ag-project #1=(string) t nil nil) (ag-project-files (string file-type) t nil nil) (ag/read-from-minibuffer (prompt) nil nil nil) (ag-project-regexp #2=(regexp) t nil nil) (ag-project-at-point #1# t nil nil) (ag-regexp-project-at-point #2# t nil nil) (ag-dired (dir string) t nil nil) (ag-dired-regexp (dir regexp) t nil nil) (ag-project-dired (pattern) t nil nil) (ag-project-dired-regexp (regexp) t nil nil) (ag-kill-buffers nil t nil nil) (ag-kill-other-buffers nil t nil nil) (ag-filter nil nil nil nil) (ag/get-supported-types nil nil nil nil) (ag/read-file-type nil nil nil nil))"
    ];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_aliases_obsolescence_mode_map_and_mode_metadata_match() {
    let elisp_form = r##"(list
         (eq (indirect-function 'ag-project-at-point)
             (indirect-function 'ag-project))
         (eq (indirect-function 'ag-regexp-project-at-point)
             (indirect-function 'ag-project-regexp))
         (get 'ag-project-at-point 'byte-obsolete-info)
         (get 'ag-regexp-project-at-point 'byte-obsolete-info)
         (get 'ag-mode 'mode-class)
         (get 'ag-mode 'derived-mode-parent)
         (mapcar
          (lambda (key)
            (list
             key
             (lookup-key ag-mode-map (kbd key))))
          '("p" "n" "k" "RET" "g" "q")))"##;
    let expect = expect![[
        r#"OK (t t (ag-project nil "0.19") (ag-project-regexp nil "0.46") nil compilation-mode (("p" compilation-previous-error) ("n" compilation-next-error) ("k" (lambda nil (interactive) (let (kill-buffer-query-functions) (kill-buffer)))) ("RET" nil) ("g" nil) ("q" nil)))"#
    ]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_autoload_surface_exposes_every_public_command_without_eager_loading() {
    let elisp_form = r##"(list
         (featurep 'ag)
         (mapcar
          (lambda (symbol)
            (let ((definition
                   (symbol-function symbol)))
              (list
               symbol
               (autoloadp definition)
               (cond
                ((autoloadp definition)
                 (nth 1 definition))
                ((symbolp definition)
                 definition)
                (t 'loaded-definition))
               (commandp symbol))))
          '(ag
            ag-files
            ag-regexp
            ag-project
            ag-project-files
            ag-project-regexp
            ag-project-at-point
            ag-regexp-project-at-point
            ag-dired
            ag-dired-regexp
            ag-project-dired
            ag-project-dired-regexp
            ag-kill-buffers
            ag-kill-other-buffers))
         (file-name-nondirectory
          (getenv "NEOMACS_PACKAGE_SOURCE")))"##;
    let expect = expect![[
        r#"OK (nil ((ag t "ag" t) (ag-files t "ag" t) (ag-regexp t "ag" t) (ag-project t "ag" t) (ag-project-files t "ag" t) (ag-project-regexp t "ag" t) (ag-project-at-point nil ag-project t) (ag-regexp-project-at-point nil ag-project-regexp t) (ag-dired t "ag" t) (ag-dired-regexp t "ag" t) (ag-project-dired t "ag" t) (ag-project-dired-regexp t "ag" t) (ag-kill-buffers t "ag" t) (ag-kill-other-buffers t "ag" t)) "ag-autoloads.el")"#
    ]];
    assert_ag_autoload_parity(elisp_form, expect);
}

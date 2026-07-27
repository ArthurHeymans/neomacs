use expect_test::expect;

use super::assert_ahg_parity;

#[test]
fn ahg_exact_pin_feature_version_and_core_defaults_match() {
    let elisp_form = r##"(let ((descriptor (cadr (assq 'ahg package-alist))))
                      (list
                       (package-desc-name descriptor)
                       (package-version-join (package-desc-version descriptor))
                       (package-desc-reqs descriptor)
                       (featurep 'ahg)
                       ahg-version-string
                       ahg-hg-command
                       ahg-diff-use-git-format
                       ahg-log-revrange-size
                       ahg-i18n
                       ahg-manifest-grep-use-xargs-grep))"##;
    let expect = expect![[r#"OK (ahg "20241113.748" nil t "1.0.0" "hg" t 100 t t)"#]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_complete_callable_surface_arglists_and_command_flags_match() {
    let elisp_form = r##"(let (surface)
                      (mapatoms
                       (lambda (symbol)
                         (when (and (fboundp symbol)
                                    (string-prefix-p "ahg-" (symbol-name symbol)))
                           (push
                            (list symbol
                                  (help-function-arglist symbol t)
                                  (commandp symbol))
                            surface))))
                      (sort surface
                            (lambda (left right)
                              (string-lessp (symbol-name (car left))
                                            (symbol-name (car right))))))"##;
    let expect = expect![
        "OK ((ahg-abspath (pth &optional root) nil) (ahg-add-log-commands (map) nil) (ahg-annotate (&optional file rev line) nil) (ahg-annotate-annotate nil t) (ahg-annotate-cur-file nil t) (ahg-annotate-diff nil t) (ahg-annotate-goto-line nil t) (ahg-annotate-line-at-line nil nil) (ahg-annotate-lines (limit) nil) (ahg-annotate-log nil t) (ahg-annotate-mode nil t) (ahg-annotate-mode-menu (arg1) t) (ahg-annotate-region (start end rev) nil) (ahg-annotate-revision-at-line nil nil) (ahg-annotate-uncover nil t) (ahg-args-add-revs (r1 r2 &optional disjoint) nil) (ahg-bookmarks nil t) (ahg-buffer-quit nil t) (ahg-call-process (cmd &optional args global-opts) nil) (ahg-cd (dir) nil) (ahg-command-help (command) t) (ahg-command-mode nil t) (ahg-command-mode-menu (arg1) t) (ahg-command-prompt nil t) (ahg-commit (files &optional msg logmsg extra-args) nil) (ahg-commit-callback (extra-args) nil) (ahg-commit-cur-file nil t) (ahg-complete-command (command) nil) (ahg-complete-command-name (command) nil) (ahg-complete-mq-patch-name (patchname) nil) (ahg-complete-shell-command (command) nil) (ahg-diff (&optional r1 r2 files) t) (ahg-diff-c (r &optional files) t) (ahg-diff-cur-file (ask-other-rev) t) (ahg-diff-ediff (&optional filename) t) (ahg-diff-ediff-cur-file (ask-other-rev) t) (ahg-diff-get-ediff-buffer (root filename rev) nil) (ahg-diff-mode nil t) (ahg-do-command (cmdstring) t) (ahg-do-command-filter (interactive) nil) (ahg-do-record (selected-files commit-func &rest commit-func-args) nil) (ahg-do-short-log (root buffer-name-prefix last-colunm-title command-list) nil) (ahg-dynamic-completion-table (fun) nil) (ahg-face-from-status (status-code) nil) (ahg-file-status (filename) nil) (ahg-first-parent-of-rev (rev) nil) (ahg-format-log-buffer nil nil) (ahg-generic-command (command args sentinel &optional buffer use-shell no-show-message report-untrusted filterfunc is-interactive global-opts no-hgplain) nil) (ahg-get-bookmarks (rev) nil) (ahg-get-status-buffer (&optional root create) nil) (ahg-get-status-ewoc (root) nil) (ahg-glog (r1 r2 &optional extra-flags) t) (ahg-glog-goto-revision-line nil nil) (ahg-glog-histedit-drop nil t) (ahg-glog-histedit-fold nil t) (ahg-glog-histedit-mess nil t) (ahg-glog-histedit-roll nil t) (ahg-glog-histedit-xtract nil t) (ahg-glog-mode nil t) (ahg-glog-mode-menu (arg1) t) (ahg-glog-next (n) t) (ahg-glog-previous (n) t) (ahg-glog-revision-at-point nil nil) (ahg-glog-update-to-rev nil t) (ahg-glog-view-details nil t) (ahg-glog-view-diff nil t) (ahg-glog-view-diff-select-rev (rev) t) (ahg-goto-line (line) nil) (ahg-goto-line-point (lp) nil) (ahg-grep-filename (filename regexp) nil) (ahg-grep-filename-grep (filename regexp) nil) (ahg-grep-filename-setup (filename) nil) (ahg-grep-regexp-quote (s) nil) (ahg-heads nil t) (ahg-hg-command nil nil) (ahg-histedit-backup (root op rev) nil) (ahg-histedit-check-ok (root rev) nil) (ahg-histedit-do-drop (rev keep) nil) (ahg-histedit-do-fold (op msg backupfile root rev) nil) (ahg-histedit-drop (rev) t) (ahg-histedit-edit-phase2-helper (op msg root rev backupfile head strip-parent) nil) (ahg-histedit-fold (rev) t) (ahg-histedit-fold-callback (root rev) nil) (ahg-histedit-get-message (rev) nil) (ahg-histedit-goto (rev parent) nil) (ahg-histedit-is-head (rev) nil) (ahg-histedit-mess (rev) t) (ahg-histedit-mess-callback (root rev) nil) (ahg-histedit-rev-id (rev) nil) (ahg-histedit-roll (rev) t) (ahg-histedit-rollback (op root rev backupfile &optional process) nil) (ahg-histedit-setup (root op rev) nil) (ahg-histedit-xtract (rev) t) (ahg-hsv-to-hex (hue saturation value) nil) (ahg-identify (&optional root) t) (ahg-line-point-pos nil nil) (ahg-log (r1 r2 &optional extra-flags) t) (ahg-log-cur-file (&optional prefix) t) (ahg-log-edit (callback file-list-function buffer &optional msg content) nil) (ahg-log-edit-hook (&optional extra-message content) nil) (ahg-log-filename-at-point (point &optional relative) t) (ahg-log-goto-revision (rev) nil) (ahg-log-histedit-drop nil t) (ahg-log-histedit-fold nil t) (ahg-log-histedit-mess nil t) (ahg-log-histedit-roll nil t) (ahg-log-histedit-xtract nil t) (ahg-log-mode nil t) (ahg-log-mode-menu (arg1) t) (ahg-log-next (n) t) (ahg-log-prepare-style-map (root) nil) (ahg-log-previous (n) t) (ahg-log-read-args (is-on-selected-files read-extra-flags &optional reverse) nil) (ahg-log-revision-at-point (&optional short-id) nil) (ahg-log-revrange-end nil nil) (ahg-log-update-to-rev nil t) (ahg-log-view-diff nil t) (ahg-log-view-diff-select-rev (rev) t) (ahg-manifest-grep (pattern glob) t) (ahg-manifest-grep-get-files (glob) nil) (ahg-manifest-grep-read (root) nil) (ahg-maybe-revset (rev) nil) (ahg-mq-applied-patches-p (&optional root) nil) (ahg-mq-convert-patch-to-changeset nil t) (ahg-mq-convert-patch-to-changeset-callback nil t) (ahg-mq-do-command nil t) (ahg-mq-edit-series nil t) (ahg-mq-get-current-patch nil nil) (ahg-mq-get-patches-buffer (root &optional dont-create) nil) (ahg-mq-list-patches (&optional root) t) (ahg-mq-log-callback (cmdname &optional extraargs) t) (ahg-mq-patch-list-refresh nil t) (ahg-mq-patch-pp (data) nil) (ahg-mq-patches-apply-patch nil t) (ahg-mq-patches-convert-patch-to-changeset nil t) (ahg-mq-patches-create-ewoc nil nil) (ahg-mq-patches-delete-patch nil t) (ahg-mq-patches-goto-patch nil t) (ahg-mq-patches-goto-patch-mouse (event) t) (ahg-mq-patches-insert-contents (ewoc patches applied guards) nil) (ahg-mq-patches-maybe-refresh (root) nil) (ahg-mq-patches-mode nil t) (ahg-mq-patches-mode-menu (arg1) t) (ahg-mq-patches-moveto-patch nil t) (ahg-mq-patches-patch-at-point nil nil) (ahg-mq-patches-qrefresh (get-log-message) t) (ahg-mq-patches-switchto-patch nil t) (ahg-mq-patches-view-patch nil t) (ahg-mq-patches-view-patch-mouse (event) t) (ahg-mq-series-mode nil t) (ahg-mq-show-patches-buffer (buf patches applied guards curdir no-pop point-pos) nil) (ahg-parse-commit-message nil nil) (ahg-pop-window-configuration nil nil) (ahg-push-window-configuration nil nil) (ahg-qapply (patchname force) t) (ahg-qdelete (patchname) t) (ahg-qdiff (files) t) (ahg-qgoto (patchname force) t) (ahg-qmove (patchname force) t) (ahg-qnew (patchname force edit-log-message) t) (ahg-qpop-all (force) t) (ahg-qrefresh (get-log-message) t) (ahg-qswitch (patchname force) t) (ahg-qtop nil t) (ahg-record (selected-files) t) (ahg-record-commit (root backup curpatch parent) nil) (ahg-record-commit-callback (commit-cmd commit-args root backup curpatch parent patchbuf) nil) (ahg-record-mq-commit (root backup curpatch parent mq-command mq-args) nil) (ahg-record-qnew (patchname selected-files) t) (ahg-record-setup (root selected-files) nil) (ahg-remove-^M nil nil) (ahg-rev-id (rev &optional which) nil) (ahg-revert-cur-file nil t) (ahg-rm-cur-file nil t) (ahg-root (&optional noerror) nil) (ahg-set-diff-mode (&optional revs) nil) (ahg-short-log (r1 r2 &optional extra-flags) t) (ahg-short-log-create-ewoc (header-prefix last-colunm-title root) nil) (ahg-short-log-goto-revision (rev) t) (ahg-short-log-histedit-drop nil t) (ahg-short-log-histedit-fold nil t) (ahg-short-log-histedit-mess nil t) (ahg-short-log-histedit-roll nil t) (ahg-short-log-histedit-xtract nil t) (ahg-short-log-impl (buffer-name-prefix template-extra last-column-title r1 r2 extra-flags) nil) (ahg-short-log-insert-contents (ewoc contents) nil) (ahg-short-log-mode nil t) (ahg-short-log-mode-menu (arg1) t) (ahg-short-log-next (n) t) (ahg-short-log-pp (data) nil) (ahg-short-log-previous (n) t) (ahg-short-log-propertize (s) nil) (ahg-short-log-refresh nil t) (ahg-short-log-revision-at-point nil nil) (ahg-short-log-update-to-rev nil t) (ahg-short-log-view-details nil t) (ahg-short-log-view-details-mouse (event) t) (ahg-short-log-view-diff nil t) (ahg-short-log-view-diff-mouse (event) t) (ahg-short-log-view-diff-select-rev (rev) t) (ahg-show-error (process) nil) (ahg-show-error-msg (msg &optional buf) nil) (ahg-start-file-process-shell-command (name buffer &rest args) nil) (ahg-status (&rest extra-switches) t) (ahg-status-add nil t) (ahg-status-add-to-hgignore nil t) (ahg-status-addremove nil t) (ahg-status-commit (&optional logmsg) t) (ahg-status-commit-amend nil t) (ahg-status-commit-secret nil t) (ahg-status-delete nil t) (ahg-status-diff (askrev &optional all) t) (ahg-status-diff-all (askrev) t) (ahg-status-diff-ediff (askrev) t) (ahg-status-dired-find (files) t) (ahg-status-do-command nil t) (ahg-status-do-mark (yes) nil) (ahg-status-get-marked (action-if-empty &optional filter) nil) (ahg-status-get-root (is-interactive) nil) (ahg-status-glog nil t) (ahg-status-log nil t) (ahg-status-mark nil t) (ahg-status-maybe-refresh (root) nil) (ahg-status-mode nil t) (ahg-status-mode-menu (arg1) t) (ahg-status-next-file nil t) (ahg-status-pp (data) nil) (ahg-status-prev-file nil t) (ahg-status-refresh nil t) (ahg-status-remove nil t) (ahg-status-sentinel (process status &optional no-pop point-pos) nil) (ahg-status-shell-command (command files refresh) t) (ahg-status-short-log nil t) (ahg-status-show-added nil t) (ahg-status-show-all nil t) (ahg-status-show-clean nil t) (ahg-status-show-default nil t) (ahg-status-show-deleted nil t) (ahg-status-show-ignored nil t) (ahg-status-show-modified nil t) (ahg-status-show-removed nil t) (ahg-status-show-tracked nil t) (ahg-status-show-unknown nil t) (ahg-status-toggle-mark nil t) (ahg-status-undo nil t) (ahg-status-unmark nil t) (ahg-status-unmark-all nil t) (ahg-status-visit-file (&optional other-window) t) (ahg-status-visit-file-other-window nil t) (ahg-string-match-p (&rest args) nil) (ahg-summary-info (root) nil) (ahg-tags nil t) (ahg-uncommitted-changes-p (&optional root) nil) (ahg-update-to-rev (rev force) t) (ahg-version nil t) (ahg-y-or-n-p (prompt) nil))"
    ];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_customization_defaults_types_and_risky_process_controls_match() {
    let elisp_form = r##"(mapcar
                      (lambda (variable)
                        (list variable
                              (symbol-value variable)
                              (get variable 'custom-type)
                              (get variable 'custom-group)))
                      '(ahg-hg-command
                        ahg-global-key-prefix
                        ahg-do-command-insert-header
                        ahg-do-command-show-buffer-immediately
                        ahg-do-command-interactive-regexp
                        ahg-auto-refresh-status-buffer
                        ahg-restore-window-configuration-on-quit
                        ahg-diff-use-git-format
                        ahg-qrefresh-use-short-flag
                        ahg-yesno-short-prompt
                        ahg-i18n
                        ahg-subprocess-coding-system
                        ahg-log-revrange-size
                        ahg-map-cmdline-file
                        ahg-summary-remote
                        ahg-summary-git-svn-info
                        ahg-manifest-grep-use-xargs-grep
                        ahg-diff-keep-current-buffer))"##;
    let expect = expect![[
        r#"OK ((ahg-hg-command "hg" string nil) (ahg-global-key-prefix "\3hg" string nil) (ahg-do-command-insert-header t boolean nil) (ahg-do-command-show-buffer-immediately t boolean nil) (ahg-do-command-interactive-regexp "\\<\\(in\\|incoming\\|out\\|outgoing\\|pull\\|push\\)\\>" regexp nil) (ahg-auto-refresh-status-buffer t boolean nil) (ahg-restore-window-configuration-on-quit t boolean nil) (ahg-diff-use-git-format t boolean nil) (ahg-qrefresh-use-short-flag t boolean nil) (ahg-yesno-short-prompt t boolean nil) (ahg-i18n t boolean nil) (ahg-subprocess-coding-system nil symbol nil) (ahg-log-revrange-size 100 integer nil) (ahg-map-cmdline-file nil string nil) (ahg-summary-remote nil boolean nil) (ahg-summary-git-svn-info nil boolean nil) (ahg-manifest-grep-use-xargs-grep t boolean nil) (ahg-diff-keep-current-buffer nil boolean nil))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_major_modes_install_practical_navigation_and_action_keymaps() {
    let elisp_form = r##"(mapcar
                      (lambda (entry)
                        (let ((mode (car entry))
                              (keys (cdr entry)))
                          (with-temp-buffer
                            (funcall mode)
                            (list
                             mode
                             major-mode
                             mode-name
                             buffer-read-only
                             truncate-lines
                             (mapcar
                              (lambda (key)
                                (cons key (lookup-key (current-local-map)
                                                      (kbd key))))
                              keys)))))
                      '((ahg-status-mode "m" "u" "c" "=" "g" "Q n" "i c" "C a")
                        (ahg-short-log-mode "g" "=" "SPC" "RET" "E m" "n" "p")
                        (ahg-log-mode "g" "=" "D" "TAB" "RET" "E d")
                        (ahg-glog-mode "g" "=" "SPC" "RET" "E f")
                        (ahg-diff-mode "q")
                        (ahg-annotate-mode "=" "l" "a" "u" "RET")
                        (ahg-command-mode "h" "q" "!" "C-i")
                        (ahg-mq-patches-mode "=" "RET" "m" "s" "a" "D" "r")))"##;
    let expect = expect![[
        r#"OK ((ahg-status-mode ahg-status-mode "aHg-status" t nil (("m" . ahg-status-mark) ("u" . ahg-status-unmark) ("c" . ahg-status-commit) ("=" . ahg-status-diff) ("g" . ahg-status-refresh) ("Q n" . ahg-qnew) ("i c" . ahg-record) ("C a" . ahg-status-commit-amend))) (ahg-short-log-mode ahg-short-log-mode "ahg-short-log" t nil (("g" . ahg-short-log-refresh) ("=" . ahg-short-log-view-diff) ("SPC" . ahg-short-log-view-details) ("RET" . ahg-short-log-update-to-rev) ("E m" . ahg-short-log-histedit-mess) ("n" . ahg-short-log-next) ("p" . ahg-short-log-previous))) (ahg-log-mode ahg-log-mode "ahg-log" t nil (("g" . ahg-log) ("=" . ahg-log-view-diff) ("D" . ahg-log-view-diff-select-rev) ("TAB" . ahg-log-next) ("RET" . ahg-log-update-to-rev) ("E d" . ahg-log-histedit-drop))) (ahg-glog-mode ahg-glog-mode "ahg-glog" t t (("g" . ahg-glog) ("=" . ahg-glog-view-diff) ("SPC" . ahg-glog-view-details) ("RET" . ahg-glog-update-to-rev) ("E f" . ahg-glog-histedit-fold))) (ahg-diff-mode ahg-diff-mode "aHg Diff" t nil (("q"))) (ahg-annotate-mode ahg-annotate-mode "Annotate" t t (("=" . ahg-annotate-diff) ("l" . ahg-annotate-log) ("a" . ahg-annotate-annotate) ("u" . ahg-annotate-uncover) ("RET" . ahg-annotate-goto-line))) (ahg-command-mode ahg-command-mode "aHg command" t nil (("h" . ahg-command-help) ("q" . ahg-buffer-quit) ("!" . ahg-do-command) ("C-i" . ahg-command-prompt))) (ahg-mq-patches-mode ahg-mq-patches-mode "ahg-mq-patches" t t (("=" . ahg-mq-patches-view-patch) ("RET" . ahg-mq-patches-goto-patch) ("m" . ahg-mq-patches-moveto-patch) ("s" . ahg-mq-patches-switchto-patch) ("a" . ahg-mq-patches-apply-patch) ("D" . ahg-mq-patches-delete-patch) ("r" . ahg-mq-patches-qrefresh))))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_status_faces_log_regexps_and_global_command_map_match() {
    let elisp_form = r##"(list
                      (mapcar
                       (lambda (status)
                         (list status (ahg-face-from-status status)))
                       '("M" "A" "R" "C" "=" "!" "I" "?" "X"))
                      (mapcar
                       (lambda (key)
                         (cons key (lookup-key ahg-global-map (kbd key))))
                       '("s" "l" "L" "G" "H" "T" "B" "!" "c" "=" "e"
                         "a" "r" "R" "f" "C-l" "Q n" "Q r" "Q e"))
                      ahg-short-log-start-regexp
                      ahg-log-start-regexp
                      ahg-glog-start-regexp
                      (mapcar
                       (lambda (face)
                         (list face
                               (get face 'face-defface-spec)
                               (get face 'face-documentation)))
                       '(ahg-status-marked-face
                         ahg-status-modified-face
                         ahg-short-log-revision-face
                         ahg-log-revision-face
                         ahg-header-line-face
                         ahg-header-line-root-face)))"##;
    let expect = expect![[
        r#"OK ((("M" ahg-status-modified-face) ("A" ahg-status-added-face) ("R" ahg-status-removed-face) ("C" ahg-status-clean-face) ("=" ahg-status-clean-face) ("!" ahg-status-deleted-face) ("I" ahg-status-ignored-face) ("?" ahg-status-unknown-face) ("X" default)) (("s" . ahg-status) ("l" . ahg-short-log) ("L" . ahg-log) ("G" . ahg-glog) ("H" . ahg-heads) ("T" . ahg-tags) ("B" . ahg-bookmarks) ("!" . ahg-do-command) ("c" . ahg-commit-cur-file) ("=" . ahg-diff-cur-file) ("e" . ahg-diff-ediff-cur-file) ("a" . ahg-annotate-cur-file) ("r" . ahg-revert-cur-file) ("R" . ahg-rm-cur-file) ("f" . ahg-manifest-grep) ("C-l" . ahg-log-cur-file) ("Q n" . ahg-qnew) ("Q r" . ahg-qrefresh) ("Q e" . ahg-mq-edit-series)) "^ +\\([0-9]+\\) |" "^changeset: +\\([0-9]+:[0-9a-f]+\\)" "^\\([+|@o\\\\/ -]\\)+\\([0-9]+\\)" ((ahg-status-marked-face ((default (:inherit font-lock-preprocessor-face))) "Face for marked files in aHg status buffers.") (ahg-status-modified-face ((default (:inherit font-lock-function-name-face))) "Face for modified files in aHg status buffers.") (ahg-short-log-revision-face ((default (:inherit font-lock-function-name-face))) "Face for revision field in aHg short log buffers.") (ahg-log-revision-face ((default (:inherit font-lock-variable-name-face))) "Face for revision field in aHg log buffers.") (ahg-header-line-face ((default (:inherit font-lock-comment-face))) "Face for header lines in aHg buffers.") (ahg-header-line-root-face ((default (:inherit font-lock-constant-face))) "Face for repository path in header lines of aHg buffers.")))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

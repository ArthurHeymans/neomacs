use expect_test::expect;

use super::{assert_anyins_autoload_parity, assert_anyins_parity};

#[test]
fn package_descriptor_preserves_the_exact_frozen_release_and_dependency_contract() {
    let elisp_form = r##"(let* ((description (cadr (assq 'anyins package-alist)))
       (directory (package-desc-dir description)))
  (list
   (featurep 'anyins)
   (package-installed-p 'anyins)
   (package-desc-name description)
   (package-version-join (package-desc-version description))
   (package-desc-summary description)
   (package-desc-reqs description)
   (file-name-nondirectory (directory-file-name directory))))"##;
    let expect = expect![[
        r#"OK (t t anyins "20131229.1041" "Insert content at multiple places from shell command or kill-ring." nil "anyins-20131229.1041")"#
    ]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn installed_library_and_descriptor_match_the_exact_frozen_archive_bytes() {
    let elisp_form = r##"(let* ((description (cadr (assq 'anyins package-alist)))
       (directory (package-desc-dir description)))
  (mapcar
   (lambda (name)
     (let ((file (expand-file-name name directory)))
       (list
        name
        (file-attribute-size (file-attributes file))
        (with-temp-buffer
          (insert-file-contents-literally file)
          (secure-hash 'sha256 (current-buffer))))))
   '("anyins.el" "anyins-pkg.el")))"##;
    let expect = expect![[
        r#"OK (("anyins.el" 7966 "89171d6b7f256f57438d7575c0469e2330e2bee0a2457f11cde0902227551152") ("anyins-pkg.el" 447 "1136003e240df5a35b4264feab2f297e0c57ff6cb2ca5a780ef4eb5b36da2d0e"))"#
    ]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn source_preserves_revision_definition_counts_autoload_and_feature_contracts() {
    let elisp_form = r##"(let ((source (locate-library "anyins")))
  (with-temp-buffer
    (insert-file-contents-literally source)
    (let ((contents (buffer-string)))
      (list
       (file-name-nondirectory source)
       (count-lines (point-min) (point-max))
       (how-many "^(defun anyins-")
       (how-many "^(defvar anyins-")
       (how-many "^(defface anyins-")
       (how-many "^;;;###autoload")
       (string-match-p "Package-Version: 20131229\\.1041" contents)
       (string-match-p "Package-Revision: cd5e3c1abd47" contents)
       (string-match-p "(provide 'anyins)" contents)))))"##;
    let expect = expect![[r#"OK ("anyins.el" 232 18 3 1 1 213 247 7852)"#]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn complete_callable_surface_preserves_arglists_interactivity_and_documentation() {
    let elisp_form = r##"(mapcar
 (lambda (symbol)
   (list
    symbol
    (fboundp symbol)
    (commandp symbol)
    (help-function-arglist symbol t)
    (interactive-form symbol)
    (documentation symbol t)))
 '(anyins-record-position
   anyins-remove-positions
   anyins-prepare-content-to-insert
   anyins-goto-position
   anyins-get-current-position
   anyins-record-current-position
   anyins-create-overlay
   anyins-delete-overlays
   anyins-goto-or-create-position
   anyins-compute-position-offset
   anyins-insert-at-recorded-positions
   anyins-insert-from-current-position
   anyins-insert
   anyins-turn-on-mode
   anyins-turn-off-mode
   anyins-disable-mode
   anyins-yank
   anyins-insert-command
   anyins-mode))"##;
    let expect = expect![[
        r#"OK ((anyins-record-position t nil (position) nil "Record cursor line and offset, return true if POSITION doesn't exist yet.") (anyins-remove-positions t nil nil nil "Delete recorded positions.") (anyins-prepare-content-to-insert t nil (content) nil "Transform CONTENT to list to be inserted.") (anyins-goto-position t nil (position) nil "Move cursor at POSITION.") (anyins-get-current-position t nil nil nil "Get current cursor position.") (anyins-record-current-position t t nil (interactive nil) "Record current cursor position.") (anyins-create-overlay t nil (point) nil "Create an overlay at POINT.") (anyins-delete-overlays t nil nil nil "Delete overlays.") (anyins-goto-or-create-position t nil (position) nil "Create POSITION if it doesn't exist, filling with space to do so.") (anyins-compute-position-offset t nil (rows positions) nil "Compute offset for ROWS linked to POSITIONS.") (anyins-insert-at-recorded-positions t nil (rows positions) nil "Insert ROWS at recorded POSITIONS.") (anyins-insert-from-current-position t nil (rows) nil "Insert ROWS from current position.") (anyins-insert t nil (content) nil "Insert CONTENT in buffer.") (anyins-turn-on-mode t nil nil nil "Turn on anyins mode.") (anyins-turn-off-mode t nil nil nil "Turn off anyins mode.") (anyins-disable-mode t t nil (interactive nil) "Disable anyins mode.") (anyins-yank t t nil (interactive nil) "Yank the contents of the kill ring.") (anyins-insert-command t t (command) (interactive "sShell command: ") "Insert the output of COMMAND.") (anyins-mode t t (&optional arg) (interactive (list (if current-prefix-arg (prefix-numeric-value current-prefix-arg) 'toggle))) "Anyins minor mode.\n\nThis is a minor mode.  If called interactively, toggle the `Anyins mode'\nmode.  If the prefix argument is positive, enable the mode, and if it is\nzero or negative, disable the mode.\n\nIf called from Lisp, toggle the mode if ARG is `toggle'.  Enable the\nmode if ARG is nil, omitted, or is a positive number.  Disable the mode\nif ARG is a negative number.\n\nTo check whether the minor mode is enabled in the current buffer,\nevaluate the variable `anyins-mode'.\n\nThe mode's hook is called both when the mode is enabled and when it is\ndisabled.\n\n\\{anyins-mode-map}"))"#
    ]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn face_buffer_local_state_and_mode_map_preserve_the_complete_user_interface() {
    let elisp_form = r##"(list
 (facep 'anyins-recorded-positions)
 (get 'anyins-recorded-positions 'face-defface-spec)
 (face-documentation 'anyins-recorded-positions)
 (mapcar
  (lambda (symbol)
    (list
     symbol
     (default-value symbol)
     (local-variable-if-set-p symbol)
     (get symbol 'variable-documentation)))
  '(anyins-buffers-positions anyins-buffers-overlays))
 (mapcar
  (lambda (key)
    (cons key (lookup-key anyins-mode-map (kbd key))))
  '("q" "RET" "y" "!" "C-g"))
 (list
  (get 'anyins-mode 'variable-documentation)
  (get 'anyins-mode 'custom-type)
  (get 'anyins-mode 'custom-group)))"##;
    let expect = expect![[
        r#"OK ([face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((((background dark)) :background "green" :foreground "white") (((background light)) :background "green" :foreground "white")) "Marker for recorded position" ((anyins-buffers-positions nil t "Positions recorded in buffers.") (anyins-buffers-overlays nil t "Overlays recorded in buffers.")) (("q" . anyins-disable-mode) ("RET" . anyins-record-current-position) ("y" . anyins-yank) ("!" . anyins-insert-command) ("C-g")) ("Non-nil if Anyins mode is enabled.\nUse the command `anyins-mode' to change this variable." nil nil))"#
    ]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn generated_autoload_registers_only_the_minor_mode_entry_point() {
    let elisp_form = r##"(list
 (featurep 'anyins)
 (featurep 'anyins-autoloads)
 (mapcar
  (lambda (symbol)
    (list
     symbol
     (fboundp symbol)
     (and (fboundp symbol) (autoloadp (symbol-function symbol)))
     (and (fboundp symbol) (commandp symbol))))
  '(anyins-mode
    anyins-yank
    anyins-insert-command
    anyins-record-current-position))
 (boundp 'anyins-buffers-positions)
 (boundp 'anyins-mode-map))"##;
    let expect = expect![
        "OK (nil t ((anyins-mode t t t) (anyins-yank nil nil nil) (anyins-insert-command nil nil nil) (anyins-record-current-position nil nil nil)) nil nil)"
    ];
    assert_anyins_autoload_parity(elisp_form, expect);
}

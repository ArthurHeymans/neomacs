use expect_test::expect;

use super::{assert_ansi_autoload_parity, assert_ansi_parity};

#[test]
fn package_descriptor_preserves_the_exact_frozen_release_and_dependency_contract() {
    let elisp_form = r##"(let* ((description (cadr (assq 'ansi package-alist)))
       (directory (package-desc-dir description)))
  (list
   (featurep 'ansi)
   (package-installed-p 'ansi)
   (package-desc-name description)
   (package-version-join (package-desc-version description))
   (package-desc-summary description)
   (package-desc-reqs description)
   (mapcar
    (lambda (requirement)
      (list
       (car requirement)
       (package-version-join (cadr requirement))
       (or (package-installed-p (car requirement))
           (package-built-in-p (car requirement)))))
    (package-desc-reqs description))
   (file-name-nondirectory (directory-file-name directory))))"##;
    let expect = expect![[
        r#"OK (t t ansi "20251118.230" "Turn string into ansi strings." ((emacs (24 1)) (cl-lib (0 6))) ((emacs "24.1" t) (cl-lib "0.6" t)) "ansi-20251118.230")"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn installed_library_and_descriptor_match_the_exact_frozen_archive_bytes() {
    let elisp_form = r##"(let* ((description (cadr (assq 'ansi package-alist)))
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
   '("ansi.el" "ansi-pkg.el")))"##;
    let expect = expect![[
        r#"OK (("ansi.el" 7964 "eff58165c0dfb8949398d67bd2cc09b20865ef3d93f45ce6bac4e3fe5faee349") ("ansi-pkg.el" 453 "0332c2164fd1f27f945994b32368b920f6f03f5c7c0861ddbf31e0b83b3eb116"))"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn installed_source_preserves_revision_requirements_and_complete_definition_counts() {
    let elisp_form = r##"(let ((source (locate-library "ansi")))
  (with-temp-buffer
    (insert-file-contents-literally source)
    (let ((contents (buffer-string)))
      (list
       (file-name-nondirectory source)
       (count-lines (point-min) (point-max))
       (how-many "^(defun ansi-")
       (how-many "^(defmacro \\(?:ansi-\\|with-ansi\\)")
       (how-many "^(ansi--define ")
       (string-match-p "Package-Version: 20251118\\.230" contents)
       (string-match-p "Package-Revision: a3aa9daa37a7" contents)
       (string-match-p
        (regexp-quote
         "Package-Requires: ((emacs \"24.1\") (cl-lib \"0.6\"))")
        contents)
       (string-match-p "(provide 'ansi)" contents)))))"##;
    let expect = expect![[r#"OK ("ansi.el" 285 14 3 41 231 264 370 7925)"#]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn complete_effect_and_csi_registries_preserve_names_codes_order_and_variable_kinds() {
    let elisp_form = r##"(list
 (copy-tree ansi-colors)
 (copy-tree ansi-bright-colors)
 (copy-tree ansi-on-colors)
 (copy-tree ansi-on-bright-colors)
 (copy-tree ansi-styles)
 (copy-tree ansi-csis)
 ansi-reset
 (mapcar
  (lambda (symbol)
    (list
     symbol
     (get symbol 'variable-documentation)
     (get symbol 'risky-local-variable)
     (get symbol 'constant)))
  '(ansi-colors
    ansi-bright-colors
    ansi-on-colors
    ansi-on-bright-colors
    ansi-styles
    ansi-csis
    ansi-reset)))"##;
    let expect = expect![[
        r#"OK (((black . 30) (red . 31) (green . 32) (yellow . 33) (blue . 34) (magenta . 35) (cyan . 36) (white . 37)) ((bright-black . 90) (bright-red . 91) (bright-green . 92) (bright-yellow . 93) (bright-blue . 94) (bright-magenta . 95) (bright-cyan . 96) (bright-white . 97)) ((on-black . 40) (on-red . 41) (on-green . 42) (on-yellow . 43) (on-blue . 44) (on-magenta . 45) (on-cyan . 46) (on-white . 47)) ((on-bright-black . 100) (on-bright-red . 101) (on-bright-green . 102) (on-bright-yellow . 103) (on-bright-blue . 104) (on-bright-magenta . 105) (on-bright-cyan . 106) (on-bright-white . 107)) ((bold . 1) (dark . 2) (italic . 3) (underscore . 4) (blink . 5) (rapid . 6) (contrary . 7) (concealed . 8) (strike . 9)) ((up . "A") (down . "B") (forward . "C") (backward . "D") (next-line . "E") (previous-line . "F") (column . "G") (kill . "K")) 0 ((ansi-colors "List of text colors." t nil) (ansi-bright-colors "List of text colors." t nil) (ansi-on-colors "List of colors to draw text on." t nil) (ansi-on-bright-colors "List of colors to draw text on." t nil) (ansi-styles "List of styles." t nil) (ansi-csis "CSI (Control Sequence Introducer) sequences." nil nil) (ansi-reset "Ansi code for reset." t nil)))"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn all_forty_one_generated_effect_functions_preserve_arglists_docs_and_noninteractive_contracts() {
    let elisp_form = r##"(mapcar
 (lambda (symbol)
   (list
    symbol
    (fboundp symbol)
    (commandp symbol)
    (help-function-arglist symbol t)
    (interactive-form symbol)
    (documentation symbol t)))
 '(ansi-black ansi-red ansi-green ansi-yellow
   ansi-blue ansi-magenta ansi-cyan ansi-white
   ansi-bright-black ansi-bright-red ansi-bright-green ansi-bright-yellow
   ansi-bright-blue ansi-bright-magenta ansi-bright-cyan ansi-bright-white
   ansi-on-black ansi-on-red ansi-on-green ansi-on-yellow
   ansi-on-blue ansi-on-magenta ansi-on-cyan ansi-on-white
   ansi-on-bright-black ansi-on-bright-red
   ansi-on-bright-green ansi-on-bright-yellow
   ansi-on-bright-blue ansi-on-bright-magenta
   ansi-on-bright-cyan ansi-on-bright-white
   ansi-bold ansi-dark ansi-italic ansi-underscore ansi-blink
   ansi-rapid ansi-contrary ansi-concealed ansi-strike))"##;
    let expect = expect![[
        r#"OK ((ansi-black t nil #1=(format-string &rest objects) nil "Add \\='black\\=' ansi effect to text.") (ansi-red t nil #1# nil "Add \\='red\\=' ansi effect to text.") (ansi-green t nil #1# nil "Add \\='green\\=' ansi effect to text.") (ansi-yellow t nil #1# nil "Add \\='yellow\\=' ansi effect to text.") (ansi-blue t nil #1# nil "Add \\='blue\\=' ansi effect to text.") (ansi-magenta t nil #1# nil "Add \\='magenta\\=' ansi effect to text.") (ansi-cyan t nil #1# nil "Add \\='cyan\\=' ansi effect to text.") (ansi-white t nil #1# nil "Add \\='white\\=' ansi effect to text.") (ansi-bright-black t nil #1# nil "Add \\='bright-black\\=' ansi effect to text.") (ansi-bright-red t nil #1# nil "Add \\='bright-red\\=' ansi effect to text.") (ansi-bright-green t nil #1# nil "Add \\='bright-green\\=' ansi effect to text.") (ansi-bright-yellow t nil #1# nil "Add \\='bright-yellow\\=' ansi effect to text.") (ansi-bright-blue t nil #1# nil "Add \\='bright-blue\\=' ansi effect to text.") (ansi-bright-magenta t nil #1# nil "Add \\='bright-magenta\\=' ansi effect to text.") (ansi-bright-cyan t nil #1# nil "Add \\='bright-cyan\\=' ansi effect to text.") (ansi-bright-white t nil #1# nil "Add \\='bright-white\\=' ansi effect to text.") (ansi-on-black t nil #1# nil "Add \\='on-black\\=' ansi effect to text.") (ansi-on-red t nil #1# nil "Add \\='on-red\\=' ansi effect to text.") (ansi-on-green t nil #1# nil "Add \\='on-green\\=' ansi effect to text.") (ansi-on-yellow t nil #1# nil "Add \\='on-yellow\\=' ansi effect to text.") (ansi-on-blue t nil #1# nil "Add \\='on-blue\\=' ansi effect to text.") (ansi-on-magenta t nil #1# nil "Add \\='on-magenta\\=' ansi effect to text.") (ansi-on-cyan t nil #1# nil "Add \\='on-cyan\\=' ansi effect to text.") (ansi-on-white t nil #1# nil "Add \\='on-white\\=' ansi effect to text.") (ansi-on-bright-black t nil #1# nil "Add \\='on-bright-black\\=' ansi effect to text.") (ansi-on-bright-red t nil #1# nil "Add \\='on-bright-red\\=' ansi effect to text.") (ansi-on-bright-green t nil #1# nil "Add \\='on-bright-green\\=' ansi effect to text.") (ansi-on-bright-yellow t nil #1# nil "Add \\='on-bright-yellow\\=' ansi effect to text.") (ansi-on-bright-blue t nil #1# nil "Add \\='on-bright-blue\\=' ansi effect to text.") (ansi-on-bright-magenta t nil #1# nil "Add \\='on-bright-magenta\\=' ansi effect to text.") (ansi-on-bright-cyan t nil #1# nil "Add \\='on-bright-cyan\\=' ansi effect to text.") (ansi-on-bright-white t nil #1# nil "Add \\='on-bright-white\\=' ansi effect to text.") (ansi-bold t nil #1# nil "Add \\='bold\\=' ansi effect to text.") (ansi-dark t nil #1# nil "Add \\='dark\\=' ansi effect to text.") (ansi-italic t nil #1# nil "Add \\='italic\\=' ansi effect to text.") (ansi-underscore t nil #1# nil "Add \\='underscore\\=' ansi effect to text.") (ansi-blink t nil #1# nil "Add \\='blink\\=' ansi effect to text.") (ansi-rapid t nil #1# nil "Add \\='rapid\\=' ansi effect to text.") (ansi-contrary t nil #1# nil "Add \\='contrary\\=' ansi effect to text.") (ansi-concealed t nil #1# nil "Add \\='concealed\\=' ansi effect to text.") (ansi-strike t nil #1# nil "Add \\='strike\\=' ansi effect to text."))"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn core_helpers_macros_and_cursor_functions_preserve_complete_callable_metadata() {
    let elisp_form = r##"(mapcar
 (lambda (symbol)
   (list
    symbol
    (fboundp symbol)
    (macrop symbol)
    (commandp symbol)
    (help-function-arglist symbol t)
    (interactive-form symbol)
    (documentation symbol t)))
 '(ansi--concat
   ansi--code
   ansi--is-alias
   ansi--char
   ansi--define
   ansi--substitute
   with-ansi
   with-ansi-princ
   ansi-apply
   ansi-csi-apply
   ansi-up
   ansi-down
   ansi-forward
   ansi-backward
   ansi-next-line
   ansi-previous-line
   ansi-column
   ansi-kill))"##;
    let expect = expect![[
        r#"OK ((ansi--concat t nil nil (&rest sequences) nil "Concat string elements in SEQUENCES.") (ansi--code t nil nil (effect) nil "Return code for EFFECT.") (ansi--is-alias t nil nil (effect) nil "Return non-nil if EFFECT is available in DSL.") (ansi--char t nil nil (effect) nil "Return char for EFFECT.") (ansi--define t t nil (effect) nil "Define ansi function with EFFECT.") (ansi--substitute t nil nil (body) nil nil) (with-ansi t t nil (&rest body) nil "Shortcut names (without ansi- prefix) can be used in this BODY.") (with-ansi-princ t t nil (&rest body) nil "Shortcut names (without ansi- prefix) can be used in this BODY and princ.") (ansi-apply t nil nil (effect-or-code format-string &rest objects) nil "Apply EFFECT-OR-CODE to text.\nFORMAT-STRING and OBJECTS are processed same as `apply'.") (ansi-csi-apply t nil nil (effect-or-char &optional reps) nil "Apply EFFECT-OR-CHAR REPS (1 default) number of times.") (ansi-up t nil nil (&optional n) nil "Move N steps (1 step default) up.") (ansi-down t nil nil (&optional n) nil "Move N steps (1 step default) down.") (ansi-forward t nil nil (&optional n) nil "Move N steps (1 step default) forward.") (ansi-backward t nil nil (&optional n) nil "Move N steps (1 step default) backward.") (ansi-next-line t nil nil (&optional n) nil "Move cursor to beginning of the line N (default 1) lines down.") (ansi-previous-line t nil nil (&optional n) nil "Move cursor to beginning of the line N (default 1) lines up.") (ansi-column t nil nil (&optional n) nil "Move the cursor to column N (default 1).") (ansi-kill t nil nil (&optional n) nil "Erase part of the line.\n\nIf N is 0 (or missing), clear from cursor to the end of the line.\n\nIf N is 1, clear from cursor to beginning of the line.\n\nIf N is 2, clear entire line.  Cursor position does not change."))"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn inhibit_option_preserves_default_custom_metadata_documentation_and_global_scope() {
    let elisp_form = r##"(list
 (get 'ansi 'custom-group)
 (get 'ansi 'group-documentation)
 (list
  ansi-inhibit-ansi
  (get 'ansi-inhibit-ansi 'custom-type)
  (get 'ansi-inhibit-ansi 'custom-group)
  (get 'ansi-inhibit-ansi 'standard-value)
  (get 'ansi-inhibit-ansi 'variable-documentation)
  (local-variable-if-set-p 'ansi-inhibit-ansi))
 (get 'ansi-inhibit-ansi 'custom-requests))"##;
    let expect = expect![[
        r#"OK (((ansi-inhibit-ansi custom-variable)) "Turn string into ansi strings." (nil boolean nil ((funcall #'#[nil (nil) (t)])) "If non-nil, no apply ANSI code.\nThis variable affects `with-ansi', `with-ansi-princ'." nil) nil)"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn generated_autoload_file_registers_no_callables_because_upstream_has_no_autoload_cookies() {
    let elisp_form = r##"(list
 (featurep 'ansi)
 (featurep 'ansi-autoloads)
 (mapcar
  (lambda (symbol)
    (list
     symbol
     (fboundp symbol)
     (and (fboundp symbol)
          (autoloadp (symbol-function symbol)))))
  '(ansi-red ansi-bold ansi-up ansi-apply with-ansi with-ansi-princ))
 (boundp 'ansi-colors)
 (boundp 'ansi-inhibit-ansi))"##;
    let expect = expect![
        "OK (nil t ((ansi-red nil nil) (ansi-bold nil nil) (ansi-up nil nil) (ansi-apply nil nil) (with-ansi nil nil) (with-ansi-princ nil nil)) nil nil)"
    ];
    assert_ansi_autoload_parity(elisp_form, expect);
}

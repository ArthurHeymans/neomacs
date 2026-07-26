use super::assert_ace_pinyin_parity;
use expect_test::expect;

#[test]
fn ace_pinyin_callable_surface_metadata_matches() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (help-function-arglist symbol t)
            (commandp symbol)
            (interactive-form symbol)
            (documentation symbol)
            (file-name-nondirectory
             (symbol-file symbol 'defun))))
         '(avy-jump
           ace-pinyin--build-regexp
           ace-pinyin--jump-impl
           ace-pinyin-jump-char
           ace-pinyin-jump-char-2
           ace-pinyin-jump-char-in-line
           ace-pinyin-goto-word-0
           ace-pinyin-goto-word-1
           ace-pinyin-goto-subword-0
           ace-pinyin-goto-subword-1
           ace-pinyin--jump-word-1
           ace-pinyin-jump-word
           ace-pinyin-dwim
           ace-pinyin-mode
           ace-pinyin-global-mode
           turn-on-ace-pinyin-mode
           turn-off-ace-pinyin-mode))"##;
    let expect = expect![[
        r#"OK ((avy-jump (regex &rest --cl-rest--) nil nil "Jump to REGEX.\nThe window scope is determined by ‘avy-all-windows’.\nWhen WINDOW-FLIP is non-nil, do the opposite of ‘avy-all-windows’.\nBEG and END narrow the scope where candidates are searched.\nACTION is a function that takes point position as an argument.\n\n(fn REGEX &key WINDOW-FLIP BEG END ACTION)" "ace-pinyin.el") (ace-pinyin--build-regexp (query-char &optional prefix) nil nil nil "ace-pinyin.el") (ace-pinyin--jump-impl (query-char &optional prefix) nil nil "Internal implementation of ‘ace-pinyin-jump-char’." "ace-pinyin.el") (ace-pinyin-jump-char (query-char) t (interactive (list (if ace-pinyin-use-avy (read-char "char: ") (read-char "Query Char:")))) "AceJump with pinyin by QUERY-CHAR." "ace-pinyin.el") (ace-pinyin-jump-char-2 (char1 char2 &optional arg) t (interactive (list (read-char "char 1: ") (read-char "char 2: ") current-prefix-arg)) "Ace-pinyin replacement of ‘avy-goto-char-2’." "ace-pinyin.el") (ace-pinyin-jump-char-in-line (char) t (interactive (list (read-char "char: " t))) "Ace-pinyn replacement of ‘avy-goto-char-in-line’." "ace-pinyin.el") (ace-pinyin-goto-word-0 (arg) t (interactive "P") "Ace-pinyin replacement of ‘avy-goto-word-0’." "ace-pinyin.el") (ace-pinyin-goto-word-1 (char &optional arg) t (interactive (list (read-char "char: " t) current-prefix-arg)) "Ace-pinyin replacement of ‘avy-goto-word-1’." "ace-pinyin.el") (ace-pinyin-goto-subword-0 (&optional arg predicate) t (interactive "P") "Ace-pinyin replacement of ‘avy-goto-subword-0’." "ace-pinyin.el") (ace-pinyin-goto-subword-1 (char &optional arg) t (interactive (list (read-char "char: " t) current-prefix-arg)) "Ace-pinyin replacement of ‘avy-goto-subword-1’." "ace-pinyin.el") (ace-pinyin--jump-word-1 (query) nil nil nil "ace-pinyin.el") (ace-pinyin-jump-word (arg) t (interactive "P") "Jump to Chinese word.\nIf ARG is non-nil, read input from Minibuffer." "ace-pinyin.el") (ace-pinyin-dwim (&optional prefix) t (interactive "P") "With PREFIX, only search Chinese.\nWithout PREFIX, search both Chinese and English." "ace-pinyin.el") (ace-pinyin-mode (&optional arg) t (interactive #1=(list (if current-prefix-arg (prefix-numeric-value current-prefix-arg) 'toggle))) "Toggle ‘ace-pinyin-mode’.\n\nThis is a minor mode.  If called interactively, toggle the ‘Ace-Pinyin\nmode’ mode.  If the prefix argument is positive, enable the mode, and if\nit is zero or negative, disable the mode.\n\nIf called from Lisp, toggle the mode if ARG is ‘toggle’.  Enable the\nmode if ARG is nil, omitted, or is a positive number.  Disable the mode\nif ARG is a negative number.\n\nTo check whether the minor mode is enabled in the current buffer,\nevaluate the variable ‘ace-pinyin-mode’.\n\nThe mode’s hook is called both when the mode is enabled and when it is\ndisabled." "ace-pinyin.el") (ace-pinyin-global-mode (&optional arg) t (interactive #1#) "Toggle Ace-Pinyin mode in many buffers.\nSpecifically, Ace-Pinyin mode is enabled in all buffers where\n‘turn-on-ace-pinyin-mode’ would do it.\n\nWith prefix ARG, enable Ace-Pinyin-Global mode if ARG is positive;\notherwise, disable it.\n\nIf called from Lisp, toggle the mode if ARG is ‘toggle’.\nEnable the mode if ARG is nil, omitted, or is a positive number.\nDisable the mode if ARG is a negative number.\n\nSee ‘ace-pinyin-mode’ for more information on Ace-Pinyin mode." "ace-pinyin.el") (turn-on-ace-pinyin-mode nil t (interactive nil) "Turn on ‘ace-pinyin-mode’." "ace-pinyin.el") (turn-off-ace-pinyin-mode nil t (interactive nil) "Turn off ‘ace-pinyin-mode’." "ace-pinyin.el"))"#
    ]];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_variable_defaults_custom_metadata_and_source_ownership_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (if (memq symbol
                      '(ace-pinyin--original-ace
                        ace-pinyin--original-ace-word
                        ace-pinyin--original-avy
                        ace-pinyin--original-avy-2
                        ace-pinyin--original-avy-in-line
                        ace-pinyin--original-avy-word-0
                        ace-pinyin--original-avy-word-1
                        ace-pinyin--original-avy-subword-0
                        ace-pinyin--original-avy-subword-1))
                (if (symbol-value symbol)
                    'function
                  nil)
              (symbol-value symbol))
            (special-variable-p symbol)
            (get symbol 'standard-value)
            (get symbol 'custom-type)
            (get symbol 'custom-group)
            (documentation-property
             symbol
             'variable-documentation
             t)
            (let ((file
                   (symbol-file symbol 'defvar)))
              (and file
                   (file-name-nondirectory file)))))
         '(ace-pinyin--jump-word-timeout
           ace-pinyin-use-avy
           ace-pinyin-simplified-chinese-only-p
           ace-pinyin-treat-word-as-char
           ace-pinyin-enable-punctuation-translation
           ace-pinyin--original-ace
           ace-pinyin--original-ace-word
           ace-pinyin--original-avy
           ace-pinyin--original-avy-2
           ace-pinyin--original-avy-in-line
           ace-pinyin--original-avy-word-0
           ace-pinyin--original-avy-word-1
           ace-pinyin--original-avy-subword-0
           ace-pinyin--original-avy-subword-1
           ace-pinyin-mode
           ace-pinyin-global-mode))"##;
    let expect = expect![[
        r#"OK ((ace-pinyin--jump-word-timeout 1 t (1) number nil "Seconds to wait for input." "ace-pinyin.el") (ace-pinyin-use-avy t t nil nil nil "Use `avy' or `ace-jump-mode'. Default value is to use `avy'.\nChanged since 2016-05-01." "ace-pinyin.el") (ace-pinyin-simplified-chinese-only-p t t nil nil nil "Whether `ace-pinyin' should use only simplified Chinese or not.\nDefault value is only using simplified Chinese characters." "ace-pinyin.el") (ace-pinyin-treat-word-as-char t t nil nil nil "Whether word related `avy-*' commands should be remampped." "ace-pinyin.el") (ace-pinyin-enable-punctuation-translation t t nil nil nil "Enable punctuation support or not. " "ace-pinyin.el") (ace-pinyin--original-ace nil t nil nil nil "Original definition of `ace-jump-char-mode'." "ace-pinyin.el") (ace-pinyin--original-ace-word nil t nil nil nil "Original definition of `ace-jump-word-mode'." "ace-pinyin.el") (ace-pinyin--original-avy function t nil nil nil "Original definition of `avy-goto-char'." "ace-pinyin.el") (ace-pinyin--original-avy-2 function t nil nil nil "Original definition of `avy-goto-char-2'." "ace-pinyin.el") (ace-pinyin--original-avy-in-line function t nil nil nil "Original definition of `avy-goto-char-in-line'." "ace-pinyin.el") (ace-pinyin--original-avy-word-0 function t nil nil nil "Original definition of `avy-goto-word-0'." "ace-pinyin.el") (ace-pinyin--original-avy-word-1 function t nil nil nil "Original definition of `avy-goto-word-1'." "ace-pinyin.el") (ace-pinyin--original-avy-subword-0 function t nil nil nil "Original definition of `avy-goto-subword-0'." "ace-pinyin.el") (ace-pinyin--original-avy-subword-1 function t nil nil nil "Original definition of `avy-goto-subword-1'." "ace-pinyin.el") (ace-pinyin-mode nil t nil nil nil "Non-nil if Ace-Pinyin mode is enabled.\nUse the command `ace-pinyin-mode' to change this variable." "ace-pinyin.el") (ace-pinyin-global-mode nil t (nil) boolean nil "Non-nil if Ace-Pinyin-Global mode is enabled.\nSee the `ace-pinyin-global-mode' command\nfor a description of this minor mode.\nSetting this variable directly does not take effect;\neither customize it (see the info node `Easy Customization')\nor call the function `ace-pinyin-global-mode'." "ace-pinyin.el"))"#
    ]];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_captures_each_dependency_function_cell_at_initial_load() {
    let elisp_form = r##"(list
         (eq ace-pinyin--original-ace
             (symbol-function
              'ace-jump-char-mode))
         (eq ace-pinyin--original-ace-word
             (symbol-function
              'ace-jump-word-mode))
         (eq ace-pinyin--original-avy
             (symbol-function 'avy-goto-char))
         (eq ace-pinyin--original-avy-2
             (symbol-function 'avy-goto-char-2))
         (eq ace-pinyin--original-avy-in-line
             (symbol-function
              'avy-goto-char-in-line))
         (eq ace-pinyin--original-avy-word-0
             (symbol-function 'avy-goto-word-0))
         (eq ace-pinyin--original-avy-word-1
             (symbol-function 'avy-goto-word-1))
         (eq ace-pinyin--original-avy-subword-0
             (symbol-function 'avy-goto-subword-0))
         (eq ace-pinyin--original-avy-subword-1
             (symbol-function 'avy-goto-subword-1)))"##;
    let expect = expect!["OK (t t t t t t t t t)"];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_source_reload_preserves_all_prebound_configuration_and_originals() {
    let elisp_form = r##"(let ((path
              (symbol-file
               'ace-pinyin-jump-char
               'defun)))
         (setq ace-pinyin--jump-word-timeout 9
               ace-pinyin-use-avy 'prebound-use
               ace-pinyin-simplified-chinese-only-p 'prebound-simple
               ace-pinyin-treat-word-as-char 'prebound-word
               ace-pinyin-enable-punctuation-translation 'prebound-punctuation
               ace-pinyin--original-ace 'prebound-ace
               ace-pinyin--original-ace-word 'prebound-ace-word
               ace-pinyin--original-avy 'prebound-avy
               ace-pinyin--original-avy-2 'prebound-avy-2
               ace-pinyin--original-avy-in-line 'prebound-line
               ace-pinyin--original-avy-word-0 'prebound-word-0
               ace-pinyin--original-avy-word-1 'prebound-word-1
               ace-pinyin--original-avy-subword-0 'prebound-subword-0
               ace-pinyin--original-avy-subword-1 'prebound-subword-1)
         (load path nil t)
         (list
          ace-pinyin--jump-word-timeout
          ace-pinyin-use-avy
          ace-pinyin-simplified-chinese-only-p
          ace-pinyin-treat-word-as-char
          ace-pinyin-enable-punctuation-translation
          ace-pinyin--original-ace
          ace-pinyin--original-ace-word
          ace-pinyin--original-avy
          ace-pinyin--original-avy-2
          ace-pinyin--original-avy-in-line
          ace-pinyin--original-avy-word-0
          ace-pinyin--original-avy-word-1
          ace-pinyin--original-avy-subword-0
          ace-pinyin--original-avy-subword-1
          (featurep 'ace-pinyin)))"##;
    let expect = expect![
        "OK (9 prebound-use prebound-simple prebound-word prebound-punctuation prebound-ace prebound-ace-word prebound-avy prebound-avy-2 prebound-line prebound-word-0 prebound-word-1 prebound-subword-0 prebound-subword-1 t)"
    ];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_compatibility_avy_jump_sets_action_and_forwards_keyword_scope() {
    let elisp_form = r##"(let ((events nil)
             (avy-action 'original-action))
         (cl-letf
             (((symbol-function 'avy--generic-jump)
               (lambda (regexp window-flip beg end)
                 (push
                  (list regexp
                        window-flip
                        beg
                        end
                        avy-action)
                  events)
                 'generic-result)))
           (list
            (avy-jump "fixture-regexp"
                      :window-flip t
                      :beg 3
                      :end 11
                      :action 'fixture-action)
            avy-action
            (nreverse events))))"##;
    let expect = expect![[
        r#"OK (generic-result fixture-action (("fixture-regexp" t 3 11 fixture-action)))"#
    ]];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_compatibility_avy_jump_preserves_action_when_keyword_is_nil() {
    let elisp_form = r##"(let ((events nil)
             (avy-action 'original-action))
         (cl-letf
             (((symbol-function 'avy--generic-jump)
               (lambda (regexp window-flip beg end)
                 (push
                  (list regexp
                        window-flip
                        beg
                        end
                        avy-action)
                  events)
                 'generic-result)))
           (list
            (avy-jump "fixture-regexp")
            avy-action
            (nreverse events))))"##;
    let expect = expect![[
        r#"OK (generic-result original-action (("fixture-regexp" nil nil nil original-action)))"#
    ]];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_packaged_source_descriptor_autoload_and_readme_assets_match() {
    let elisp_form = r##"(let* ((descriptor
                      (cadr
                       (assq
                        'ace-pinyin
                        package-alist)))
                     (directory
                      (package-desc-dir descriptor)))
               (mapcar
                (lambda (name)
                  (let ((path
                         (expand-file-name
                          name
                          directory)))
                    (with-temp-buffer
                      (set-buffer-multibyte nil)
                      (insert-file-contents-literally path)
                      (list
                       name
                       (buffer-size)
                       (secure-hash
                        'sha256
                        (current-buffer))))))
                '("ace-pinyin.el"
                  "ace-pinyin-pkg.el"
                  "ace-pinyin-autoloads.el"
                  "README-elpa")))"##;
    let expect = expect![[
        r#"OK (("ace-pinyin.el" 20037 "413ea467e01b0b246f927803d08be133b4dc42b8eb2b49d38a9eca4cf7470604") ("ace-pinyin-pkg.el" 475 "75ca05e31bdfa5710e979c7ed43c1a3c6a6f362ecaf683ed6010c80e5df392f2") ("ace-pinyin-autoloads.el" 2751 "9541b3fcd3f4565320e5f4e8c6f8ab764f0ebc11fe87fcf550e77a8e9a460d44") ("README-elpa" 7253 "57bd911e27bc1e9e4884a5f969228e94517a2f1092b13ebb2eb6df6a781eb826"))"#
    ]];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_installation_produces_a_local_byte_compilation_artifact() {
    let elisp_form = r##"(let* ((descriptor
                      (cadr
                       (assq
                        'ace-pinyin
                        package-alist)))
                     (directory
                      (package-desc-dir descriptor))
                     (path
                      (expand-file-name
                       "ace-pinyin.elc"
                       directory)))
               (list
                (file-exists-p path)
                (file-regular-p path)
                (> (file-attribute-size
                    (file-attributes path))
                   0)))"##;
    let expect = expect!["OK (t t t)"];
    assert_ace_pinyin_parity(elisp_form, expect);
}

use expect_test::expect;

use super::assert_amx_parity;

#[test]
fn keybinding_hash_roundtrips_local_commands_and_augmented_completion_labels() {
    let elisp_form = r##"
(let ((map (make-sparse-keymap)))
  (define-key map (kbd "C-c a") #'amx-test-alpha)
  (define-key map (kbd "C-c b") #'amx-test-beta)
  (define-key map (kbd "C-c n") #'amx-test-noncommand)
  (let* ((hash (amx-make-keybind-hash map))
         (alpha-key
          (gethash 'amx-test-alpha hash))
         (beta-key
          (gethash 'amx-test-beta hash))
         (alpha-label
          (format "amx-test-alpha (%s)"
                  alpha-key))
         (beta-label
          (format "amx-test-beta (%s)"
                  beta-key)))
    (list
     alpha-key
     beta-key
     (gethash alpha-label hash)
     (gethash beta-label hash)
     (gethash 'amx-test-noncommand hash)
     (amx-augment-commands-with-keybinds
      '(amx-test-alpha
        (amx-test-beta . 4)
        amx-test-gamma)
      hash)
     (amx-clean-command-name alpha-label)
     (amx-clean-command-name beta-label))))
"##;
    let expect = expect![[
        r#"OK ("C-c a" "C-c b" amx-test-alpha amx-test-beta nil ("amx-test-alpha (C-c a)" "amx-test-beta (C-c b)" "amx-test-gamma") amx-test-alpha amx-test-beta)"#
    ]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn augmenting_commands_respects_ignore_rules_and_preserves_unbound_names() {
    let elisp_form = r##"
(let ((hash (make-hash-table :test 'equal))
      (amx-ignored-command-matchers
       '("\\`amx-test-beta\\'")))
  (puthash 'amx-test-alpha "C-c a" hash)
  (puthash 'amx-test-beta "C-c b" hash)
  (list
   (mapcar
    (lambda (command)
      (amx-augment-command-with-keybind
       command hash))
    '(amx-test-alpha
      "amx-test-beta"
      (amx-test-gamma . 8)))
   (amx-augment-commands-with-keybinds
    '(amx-test-alpha amx-test-beta
      amx-test-gamma)
    hash)))
"##;
    let expect = expect![[
        r#"OK (("amx-test-alpha (C-c a)" "amx-test-beta" "amx-test-gamma") ("amx-test-alpha (C-c a)" "amx-test-beta" "amx-test-gamma"))"#
    ]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn cleaning_completion_labels_uses_hash_chopping_and_first_token_fallbacks() {
    let elisp_form = r##"
(let ((hash (make-hash-table :test 'equal)))
  (puthash
   "Friendly alpha label"
   'amx-test-alpha hash)
  (let ((amx-command-keybind-hash hash))
    (mapcar
     (lambda (label)
       (condition-case error-data
           (amx-clean-command-name label)
         (error
          (cons (car error-data)
                (cdr error-data)))))
     '("Friendly alpha label"
       "amx-test-beta (C-c b)"
       "future-command extra metadata"
       ""
       "   "))))
"##;
    let expect = expect![[
        r#"OK (amx-test-alpha amx-test-beta future-command (error "Could not find command: \"\"") (error "Could not find command: \"   \""))"#
    ]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn ignored_command_matchers_cover_regexp_function_property_obsolete_and_mouse_cases() {
    let elisp_form = r##"
(unwind-protect
    (progn
      (put 'amx-test-alpha 'amx-ignored t)
      (put 'amx-test-beta
           'byte-obsolete-info
           '(amx-test-gamma "1.0"))
      (let ((amx-ignored-command-matchers
             '("\\`amx-test-gamma\\'"
               (lambda (command)
                 (eq command
                     'amx-test-noncommand))
               amx-command-marked-ignored-p
               amx-command-obsolete-p
               amx-command-mouse-interactive-p)))
        (mapcar
         (lambda (command)
           (list
            command
            (amx-command-ignored-p command)
            (amx-command-marked-ignored-p
             command)
            (amx-command-obsolete-p command)
            (and
             (commandp
              (amx-get-command-symbol
               command))
             (amx-command-mouse-interactive-p
              command))))
         '(amx-test-alpha
           "amx-test-beta"
           (amx-test-gamma . 5)
           amx-test-mouse
           ignore))))
  (put 'amx-test-alpha 'amx-ignored nil)
  (put 'amx-test-beta
       'byte-obsolete-info nil))
"##;
    let expect = expect![[
        r#"OK ((amx-test-alpha t t nil nil) ("amx-test-beta" t nil (amx-test-gamma "1.0") nil) ((amx-test-gamma . 5) t nil nil nil) (amx-test-mouse t nil nil 0) (ignore nil nil nil nil))"#
    ]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn ignore_and_unignore_accept_strings_symbols_and_lists_without_touching_functions() {
    let elisp_form = r##"
(unwind-protect
    (progn
      (amx-ignore-command
       '("amx-test-alpha"
         amx-test-beta))
      (let ((ignored
             (mapcar
              (lambda (command)
                (list
                 command
                 (get command 'amx-ignored)
                 (commandp command)))
              '(amx-test-alpha
                amx-test-beta
                amx-test-gamma))))
        (amx-unignore-command
         'amx-test-alpha)
        (amx-ignore-command
         'amx-test-beta nil)
        (list
         ignored
         (mapcar
          (lambda (command)
            (get command 'amx-ignored))
          '(amx-test-alpha
            amx-test-beta
            amx-test-gamma))
         (mapcar #'commandp
                 '(amx-test-alpha
                   amx-test-beta
                   amx-test-gamma)))))
  (mapc
   (lambda (command)
     (put command 'amx-ignored nil))
   '(amx-test-alpha amx-test-beta
     amx-test-gamma)))
"##;
    let expect = expect![
        "OK (((amx-test-alpha t t) (amx-test-beta t t) (amx-test-gamma nil t)) (nil nil nil) (t t t))"
    ];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn nested_keymap_extraction_returns_interactive_symbols_and_rejects_strings() {
    let elisp_form = r##"
(let ((map (make-sparse-keymap))
      (prefix (make-sparse-keymap)))
  (define-key map (kbd "a")
              #'amx-test-alpha)
  (define-key map (kbd "b")
              "keyboard macro text")
  (define-key map (kbd "n")
              #'amx-test-noncommand)
  (define-key prefix (kbd "b")
              #'amx-test-beta)
  (define-key prefix (kbd "g")
              #'amx-test-gamma)
  (define-key map (kbd "C-c") prefix)
  (sort
   (amx-extract-commands-from-keymap map)
   (lambda (left right)
     (string< (symbol-name left)
              (symbol-name right)))))
"##;
    let expect = expect!["OK nil"];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn feature_extraction_matches_mode_library_and_related_load_history_files() {
    let elisp_form = r##"
(let ((load-history
       '(("/fixture/lisp/amx-test-mode.el"
          (defun . amx-test-alpha)
          (defun . amx-test-noncommand))
         ("/fixture/lisp/amx-test-extra.el"
          (defun . amx-test-beta))
         ("/fixture/lisp/unrelated.el"
          (defun . amx-test-gamma))
         (nil
          (defun . amx-test-gamma)))))
  (cl-letf
      (((symbol-function 'symbol-file)
        (lambda (&rest _)
          "/fixture/lisp/amx-test-mode.el")))
    (amx-extract-commands-from-features
     'amx-test-mode)))
"##;
    let expect = expect!["OK (amx-test-alpha amx-test-beta)"];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn major_mode_command_flow_merges_keymap_features_cache_order_and_ignore_filter() {
    let elisp_form = r##"
(let ((local-map (make-sparse-keymap))
      (amx-ignored-command-matchers
       '("\\`amx-test-gamma\\'"))
      events)
  (setq amx-cache
        '((amx-test-beta . 8)
          (amx-test-alpha . 5)
          (amx-test-gamma . 2)))
  (define-key local-map
              (kbd "C-c a")
              #'amx-test-alpha)
  (cl-letf
      (((symbol-function 'current-local-map)
        (lambda () local-map))
       ((symbol-function
         'amx-extract-commands-from-features)
        (lambda (_)
          '(amx-test-gamma
            amx-test-beta
            amx-test-alpha)))
       ((symbol-function 'amx-initialize)
        (lambda (&rest _)
          (push 'initialize events)))
       ((symbol-function 'amx-read-and-run)
        (lambda (commands &optional initial)
          (push
           (list
            'read
            (all-completions "" commands)
            initial)
           events)
          'read-result)))
    (list
     (amx-major-mode-commands)
     (nreverse events)
     major-mode)))
"##;
    let expect = expect![[
        r#"OK (read-result (initialize (read ("amx-test-beta" "amx-test-alpha") nil)) lisp-interaction-mode)"#
    ]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn unbound_command_report_sorts_counts_and_writes_only_commands_without_keys() {
    let elisp_form = r##"
(let (report)
  (setq amx-data
        '((amx-test-alpha . 9)
          (amx-test-beta . 3)
          (amx-test-gamma . 7)))
  (cl-letf
      (((symbol-function 'where-is-internal)
        (lambda (command &rest _)
          (and
           (eq command 'amx-test-alpha)
           (list (kbd "C-c a")))))
       ((symbol-function
         'view-buffer-other-window)
        (lambda (name)
          (switch-to-buffer
           (get-buffer-create name)))))
    (amx-show-unbound-commands)
    (setq report
          (list
           (buffer-name)
           buffer-read-only
           (buffer-modified-p)
           (point)
           (buffer-string)
           amx-data))
    (kill-buffer (current-buffer))
    report))
"##;
    let expect = expect![[
        r#"OK ("*Amx: Unbound Commands*" t nil 1 "\n;; ----- unbound-commands -----\n(\n (amx-test-gamma . 7)\n (amx-test-beta . 3)\n)\n" ((amx-test-alpha . 9) (amx-test-gamma . 7) (amx-test-beta . 3)))"#
    ]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn ido_binding_composition_and_default_text_reader_preserve_parent_maps_and_input() {
    let elisp_form = r##"
(let ((map (make-sparse-keymap)))
  (define-key map (kbd "RET")
              #'fixture-return)
  (setq ido-completion-map map)
  (amx-prepare-ido-bindings)
  (list
   (lookup-key ido-completion-map
               (kbd "C-a"))
   (lookup-key ido-completion-map
               (kbd "C-h f"))
   (lookup-key ido-completion-map
               (kbd "C-h w"))
   (lookup-key ido-completion-map
               (kbd "M-."))
   (lookup-key ido-completion-map
               (kbd "RET"))
   (with-temp-buffer
     (insert "M-x amx-test-beta")
     (cl-letf
         (((symbol-function
            'minibuffer-prompt-end)
           (lambda () 5)))
       (amx-default-get-text)))))
"##;
    let expect = expect![
        "OK (move-beginning-of-line amx-describe-function amx-where-is amx-find-function fixture-return \"amx-test-beta\")"
    ];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn active_state_compares_owned_depth_against_live_minibuffer_depth() {
    let elisp_form = r##"
(mapcar
 (lambda (pair)
   (let ((amx-minibuffer-depth
          (car pair))
         (live-depth (cadr pair)))
     (cl-letf
         (((symbol-function
            'minibuffer-depth)
           (lambda () live-depth)))
       (list pair (amx-active)))))
 '((-1 0) (0 0) (1 0)
   (1 1) (1 2) (3 2)))
"##;
    let expect = expect!["OK (((-1 0) nil) ((0 0) t) ((1 0) t) ((1 1) t) ((1 2) nil) ((3 2) t))"];
    assert_amx_parity(elisp_form, expect);
}

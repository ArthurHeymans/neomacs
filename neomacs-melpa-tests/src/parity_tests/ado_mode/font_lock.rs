use expect_test::expect;

use super::assert_ado_mode_parity;

#[test]
fn ado_mode_builtin_font_lock_table_has_exact_shape_hash_and_representative_entries() {
    let elisp_form = r##"(progn
         (ado-set-font-lock-keywords)
         (list
          (length ado-font-lock-keywords)
          (secure-hash 'sha256
                       (encode-coding-string
                        (let ((print-circle nil))
                          (prin1-to-string ado-font-lock-keywords))
                        'utf-8-unix))
          (mapcar
           (lambda (entry)
             (secure-hash
              'sha256
              (let ((print-circle nil))
                (prin1-to-string entry))))
           (list (car ado-font-lock-keywords)
                 (nth 1 ado-font-lock-keywords)
                 (nth 10 ado-font-lock-keywords)
                 (nth 50 ado-font-lock-keywords)
                 (nth 100 ado-font-lock-keywords)
                 (car (last ado-font-lock-keywords))))
          (seq-count
           (lambda (entry)
             (and (listp entry)
                  (stringp (car entry))))
           ado-font-lock-keywords)))"##;
    let expect = expect![[
        r#"OK (493 "1a88af69b191553e362aa4e0629677004aa34cfbcb097a9968b40cc90017d265" ("b20b1fa2180fa51dd91006063ef0d5691a87f8b933a45c4a6ca54ba2e3935020" "983e7bcbdba2f724bef6f0496acf0f9763addf88a436ff460d82640c50006804" "2f9305641c50af296f4b03c824da3c51b5c7a513edb41a33d4e71387bf778873" "a78c8b7145bdad1d825a4fa891e98201aee7224f9300389efdf70f68be192533" "2108cdb5c567e9b111c3d02d4cdc122ee78594d0b699f2676783676fa6239577" "5c548f91bfffafa98b3082a1652ef560bdfd530dd34d47a5ee4cfbeb4c952793") 493)"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_representative_stata_source_fontifies_commands_macros_comments_strings_and_mata() {
    let elisp_form = r##"(with-temp-buffer
         (setq buffer-file-name "/work/sample.ado"
               ado-add-sysdir-font-lock nil
               ado-mode-home "/virtual/ado-mode/"
               ado-site-template-dir "/virtual/templates/"
               ado-script-dir "/virtual/scripts/"
               ado-new-dir "/virtual/new/"
               ado-personal-dir "/virtual/personal/")
         (insert "program define sample\n"
                 "    local value = 42\n"
                 "    display \"hello\" `value' // comment\n"
                 "    regress mpg weight\n"
                 "    mata:\n"
                 "    real scalar answer\n"
                 "    end\n"
                 "end\n")
         (ado-mode)
         (font-lock-ensure)
         (let (runs start)
           (setq start (point-min))
           (while (< start (point-max))
             (let* ((face (get-text-property start 'face))
                    (end (or (next-single-property-change
                              start 'face nil (point-max))
                             (point-max))))
               (when face
                 (push (list (buffer-substring-no-properties start end)
                             face)
                       runs))
               (setq start end)))
           (nreverse runs)))"##;
    let expect = expect![[
        r#"OK (("program" ado-builtin-harmful-face) ("define" ado-subcommand-face) ("sample" ado-builtin-harmful-face) ("local" ado-builtin-harmless-face) ("value" ado-variable-name-face) ("display" ado-builtin-harmless-face) ("\"hello\"" font-lock-string-face) ("`value'" ado-variable-name-face) (" // comment" ado-comment-face) ("regress" ado-builtin-harmless-face) ("mata" ado-builtin-harmful-face) (":" ado-constant-face) ("real" ado-mata-keyword-face) ("end" ado-builtin-harmful-face) ("end" ado-builtin-harmful-face))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_sysdir_add_remove_update_wrappers_dispatch_in_exact_order() {
    let elisp_form = r##"(let ((ado-plus-dir "/dirs/plus/")
               (ado-personal-dir "/dirs/personal/")
               (ado-site-dir "/dirs/site/")
               (ado-oldplace-dir "/dirs/oldplace/")
               calls)
         (cl-letf (((symbol-function 'ado-add-font-lock-keywords)
                    (lambda (&rest arguments)
                      (push (cons 'add arguments) calls)))
                   ((symbol-function 'ado-remove-font-lock-keywords)
                    (lambda (&rest arguments)
                      (push (cons 'remove arguments) calls)))
                   ((symbol-function 'ado-font-lock-refresh)
                    (lambda () (push '(refresh) calls))))
           (ado-add-sysdir-font-lock-keywords "plus" t t)
           (ado-add-sysdir-all nil)
           (ado-remove-sysdir-all)
           (ado-update-sysdir-all)
           (ado-remove-personal)
           (ado-remove-plus)
           (ado-remove-oldplace)
           (ado-remove-site)
           (nreverse calls)))"##;
    let expect = expect![[
        r#"OK ((add plus "/dirs/plus" ado-plus-harmless-face t t) (add site "/dirs/site" ado-site-harmless-face nil nil) (add plus "/dirs/plus" ado-plus-harmless-face nil nil) (add personal "/dirs/personal" ado-personal-harmless-face nil nil) (add oldplace "/dirs/oldplace" ado-oldplace-harmless-face nil nil) #1=(refresh) (remove site) (remove plus) (remove personal) (remove oldplace) #1# (add site "/dirs/site" ado-site-harmless-face t nil) (add plus "/dirs/plus" ado-plus-harmless-face t nil) (add personal "/dirs/personal" ado-personal-harmless-face t nil) (add oldplace "/dirs/oldplace" ado-oldplace-harmless-face t nil) #1# (remove personal) (remove plus) (remove oldplace) (remove site))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_missing_sysdir_wrappers_reset_only_unset_directories_before_addition() {
    let elisp_form = r##"(let ((ado-plus-dir nil)
               (ado-personal-dir "/already/personal/")
               (ado-site-dir nil)
               (ado-oldplace-dir nil)
               calls)
         (cl-letf (((symbol-function 'ado-reset-plus-dir)
                    (lambda ()
                      (setq ado-plus-dir "/reset/plus/")
                      (push 'reset-plus calls)))
                   ((symbol-function 'ado-reset-personal-dir)
                    (lambda ()
                      (setq ado-personal-dir "/reset/personal/")
                      (push 'reset-personal calls)))
                   ((symbol-function 'ado-reset-site-dir)
                    (lambda ()
                      (setq ado-site-dir "/reset/site/")
                      (push 'reset-site calls)))
                   ((symbol-function 'ado-reset-oldplace-dir)
                    (lambda ()
                      (setq ado-oldplace-dir "/reset/oldplace/")
                      (push 'reset-oldplace calls)))
                   ((symbol-function 'ado-add-sysdir-font-lock-keywords)
                    (lambda (&rest arguments)
                      (push (cons 'add arguments) calls))))
           (ado-add-plus t)
           (ado-add-personal nil)
           (ado-add-site t)
           (ado-add-oldplace nil)
           (list ado-plus-dir ado-personal-dir
                 ado-site-dir ado-oldplace-dir
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("/reset/plus/" "/already/personal/" "/reset/site/" "/reset/oldplace/" (reset-plus (add "plus" t) (add "personal" nil) reset-site (add "site" t) reset-oldplace (add "oldplace" nil)))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_dynamic_font_lock_add_covers_defaults_subdirectories_extensions_update_and_refresh() {
    let elisp_form = r##"(let ((ado-added-names nil)
               added removed refreshed directory-queries file-queries)
         (cl-letf (((symbol-function 'file-directory-p)
                    (lambda (directory)
                      (push directory directory-queries)
                      t))
                   ((symbol-function 'ado-find-ado-dirs)
                    (lambda (directory subdir)
                      (push (list directory subdir) directory-queries)
                      (list directory
                            (concat (file-name-as-directory directory) "a"))))
                   ((symbol-function 'directory-files)
                    (lambda (directory &rest arguments)
                      (push (cons directory arguments) file-queries)
                      (if (string-suffix-p "/a" directory)
                          '("alpha.ado" "another.ado")
                        '("root.ado"))))
                   ((symbol-function 'font-lock-add-keywords)
                    (lambda (&rest arguments)
                      (push arguments added)))
                   ((symbol-function 'font-lock-remove-keywords)
                    (lambda (&rest arguments)
                      (push arguments removed)))
                   ((symbol-function 'ado-font-lock-refresh)
                    (lambda () (push t refreshed))))
           (ado-add-font-lock-keywords
            'custom "/virtual/commands" 'ado-personal-harmless-face
            nil t nil nil nil)
           (let ((first ado-added-names))
             (ado-add-font-lock-keywords
              'custom "/ignored" 'ado-personal-harmless-face
              nil t nil "self" "mata")
             (let ((after-noop ado-added-names))
               (ado-add-font-lock-keywords
                'custom "/virtual/commands" 'ado-plus-harmful-face
                t nil nil "sub" "mata")
               (list first after-noop ado-added-names
                     (nreverse added) (nreverse removed)
                     (nreverse refreshed)
                     (nreverse directory-queries)
                     (nreverse file-queries))))))"##;
    let expect = expect![[
        r#"OK (#1=((custom . #2=(("^\\(?:\\(?:.*:\\)*\\|\\(?:[ \11]*\\(?:\\(?:cap\\(?:t\\(?:u\\(?:re?\\)?\\)?\\)?\\|mata\\|n\\(?:o\\(?:i\\(?:s\\(?:i\\(?:ly?\\)?\\)?\\)?\\)?\\)?\\|python\\|qui\\(?:e\\(?:t\\(?:ly?\\)?\\)?\\)?\\)\\|\\(?:cap\\(?:t\\(?:u\\(?:re?\\)?\\)?\\)?\\)[ /t]+\\(?:n\\(?:o\\(?:i\\(?:s\\(?:i\\(?:ly?\\)?\\)?\\)?\\)?\\)?\\)\\)\\(?:[ \11]*:\\)?\\)?\\)[ \11]*\\<\\(a\\(?:lpha\\|nother\\)\\|root\\)\\>\\([ \11]+\\|,\\|;\\|:\\|$\\)" 1 ado-personal-harmless-face)))) #1# ((custom . #3=(("^\\(?:\\(?:.*:\\)*\\|\\(?:[ \11]*\\(?:\\(?:cap\\(?:t\\(?:u\\(?:re?\\)?\\)?\\)?\\|mata\\|n\\(?:o\\(?:i\\(?:s\\(?:i\\(?:ly?\\)?\\)?\\)?\\)?\\)?\\|python\\|qui\\(?:e\\(?:t\\(?:ly?\\)?\\)?\\)?\\)\\|\\(?:cap\\(?:t\\(?:u\\(?:re?\\)?\\)?\\)?\\)[ /t]+\\(?:n\\(?:o\\(?:i\\(?:s\\(?:i\\(?:ly?\\)?\\)?\\)?\\)?\\)?\\)\\)\\(?:[ \11]*:\\)?\\)?\\)[ \11]*\\<\\(a\\(?:lpha\\|nother\\)\\|root\\)\\>\\([ \11]+\\|,\\|;\\|:\\|$\\)" 1 ado-plus-harmful-face)))) ((ado-mode #2#) (ado-mode #3#)) ((ado-mode #2#)) (t) ("/virtual/commands" "/virtual/commands" ("/virtual/commands" "all") "/virtual/commands" "/virtual/commands" ("/virtual/commands" "sub")) (("/virtual/commands" nil ".*[.]ado$") ("/virtual/commands/a" nil ".*[.]ado$") ("/virtual/commands" nil ".*[.]ado$") ("/virtual/commands/a" nil ".*[.]ado$")))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_dynamic_font_lock_missing_directory_policy_covers_nil_warn_error_and_invalid_values() {
    let elisp_form = r##"(let ((ado-added-names
                '((nil-policy ("old-nil" 1 old-face))
                  (warn-policy ("old-warn" 1 old-face))
                  (error-policy ("old-error" 1 old-face))
                  (bad-policy ("old-bad" 1 old-face))))
               removals warnings)
         (cl-letf (((symbol-function 'file-directory-p)
                    (lambda (&rest _arguments) nil))
                   ((symbol-function 'font-lock-remove-keywords)
                    (lambda (&rest arguments)
                      (push arguments removals)))
                   ((symbol-function 'display-warning)
                    (lambda (&rest arguments)
                      (push arguments warnings))))
           (let (results)
             (dolist (case
                      '((nil-policy nil)
                        (warn-policy "warn")
                        (error-policy "error")
                        (bad-policy "bogus")))
               (push
                (condition-case error-data
                    (ado-add-font-lock-keywords
                     (car case) "/missing" 'ado-comment-face
                     t nil (cadr case))
                  (error (list 'signal (car error-data)
                               (cdr error-data))))
                results))
             (list (nreverse results)
                   ado-added-names
                   (nreverse removals)
                   (nreverse warnings)))))"##;
    let expect = expect![[
        r#"OK ((nil nil (signal error ("Attempted to add directory ‘/missing’ for fontlocking, but it does not exist")) (signal error ("Bad ‘BADDIR’ specified: bogus"))) ((error-policy ("old-error" 1 old-face)) (bad-policy ("old-bad" 1 old-face))) ((ado-mode (("old-nil" 1 old-face))) (ado-mode (("old-warn" 1 old-face)))) ((ado-mode "Attempted to add directory ‘/missing’ for fontlocking, but it does not exist")))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_font_lock_remove_refresh_and_compatibility_adapter_match() {
    let elisp_form = r##"(let ((ado-added-names
                '((one ("one-regexp" 1 one-face))
                  (two ("two-regexp" 1 two-face))))
               (font-lock-major-mode 'ado-mode)
               calls)
         (cl-letf (((symbol-function 'font-lock-remove-keywords)
                    (lambda (&rest arguments)
                      (push (cons 'remove arguments) calls)))
                   ((symbol-function 'font-lock-flush)
                    (lambda (&rest arguments)
                      (push (cons 'flush arguments) calls)))
                   ((symbol-function 'ado-add-font-lock-keywords)
                    (lambda (&rest arguments)
                      (push (cons 'add arguments) calls))))
           (ado-remove-font-lock-keywords 'missing)
           (ado-remove-font-lock-keywords 'one)
           (let ((after-remove ado-added-names))
             (ado-font-lock-refresh)
             (ado-modify-font-lock-keywords
              'three "/dir" 'face nil "self" "mata" "warn")
             (ado-modify-font-lock-keywords
              'two "/dir" 'face t "sub" "ado" "error")
             (list after-remove
                   ado-added-names
                   font-lock-major-mode
                   (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (((two . #1=(("two-regexp" 1 two-face)))) nil nil ((remove ado-mode (("one-regexp" 1 one-face))) (flush) (add three "/dir" face t nil "warn" "self" "mata") (remove ado-mode #1#)))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

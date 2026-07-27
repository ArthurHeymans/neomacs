use expect_test::expect;

use super::assert_amx_parity;

#[test]
fn pretty_printer_serializes_history_data_empty_strings_and_nested_cells_exactly() {
    let elisp_form = r##"
(with-temp-buffer
  (let ((amx-history
         '(amx-test-alpha
           ""
           amx-test-beta))
        (amx-data
         '((amx-test-alpha . 9)
           ("")
           (amx-test-beta . 4))))
    (amx-pp amx-history)
    (amx-pp amx-data)
    (buffer-string)))
"##;
    let expect = expect![[
        r#"OK "\n;; ----- amx-history -----\n(\n amx-test-alpha\n amx-test-beta\n)\n\n;; ----- amx-data -----\n(\n (amx-test-alpha . 9)\n (amx-test-beta . 4)\n)\n""#
    ]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn save_and_load_roundtrip_uses_real_file_and_restores_ranked_state() {
    let elisp_form = r##"
(let* ((root (amx-test-root "save-roundtrip"))
       (file (expand-file-name "state/amx-items" root))
       (amx-save-file file)
       (init-file-user "amx-test-user")
       (amx-history-length 3))
  (setq amx-cache
        '((amx-test-beta . 7)
          (amx-test-alpha . 5)
          (amx-test-gamma . 2))
        amx-data
        '((amx-test-alpha . 5)
          (amx-test-beta . 7)
          (amx-test-gamma . 2))
        amx-history '(stale))
  (make-directory (file-name-directory file) t)
  (amx-save-to-file)
  (let ((contents (amx-test-read file))
        (saved-history (copy-tree amx-history))
        (saved-data (copy-tree amx-data)))
    (setq amx-history nil
          amx-data nil)
    (amx-load-save-file)
    (list
     (file-exists-p file)
     contents
     saved-history
     saved-data
     amx-history
     amx-data)))
"##;
    let expect = expect![[
        r#"OK (t "\n;; ----- amx-history -----\n(\n amx-test-beta\n amx-test-alpha\n amx-test-gamma\n)\n\n;; ----- amx-data -----\n(\n (amx-test-alpha . 5)\n (amx-test-beta . 7)\n (amx-test-gamma . 2)\n)\n" (amx-test-beta amx-test-alpha amx-test-gamma) ((amx-test-alpha . 5) (amx-test-beta . 7) (amx-test-gamma . 2)) (amx-test-beta amx-test-alpha amx-test-gamma) ((amx-test-alpha . 5) (amx-test-beta . 7) (amx-test-gamma . 2)))"#
    ]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn nonexistent_and_empty_save_files_reset_state_without_signaling() {
    let elisp_form = r##"
(let* ((root (amx-test-root "empty-files"))
       (missing
        (expand-file-name "missing/amx-items" root))
       (empty
        (amx-test-write
         (expand-file-name "empty/amx-items" root)
         "")))
  (mapcar
   (lambda (file)
     (let ((amx-save-file file))
       (setq amx-history '(old-history)
             amx-data '((old-data . 4)))
       (list
        (condition-case error-data
            (amx-load-save-file)
          (error
           (cons (car error-data)
                 (cdr error-data))))
        amx-history
        amx-data
        (file-exists-p file))))
   (list missing empty)))
"##;
    let expect = expect!["OK ((nil nil nil nil) (nil nil nil t))"];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn malformed_nonempty_save_file_reports_exact_context_and_leaves_reset_state() {
    let elisp_form = r##"
(let* ((root (amx-test-root "malformed-save"))
       (file
        (amx-test-write
         (expand-file-name "state/amx-items" root)
         "(amx-test-alpha\n((amx-test-alpha . 9))\n"))
       (amx-save-file file))
  (setq amx-history '(old)
        amx-data '((old . 1)))
  (list
   (condition-case error-data
       (amx-load-save-file)
     (error
      (cons (car error-data)
            (cdr error-data))))
   amx-history
   amx-data
   (amx-test-read file)))
"##;
    let expect = expect![[
        r#"OK ((error "Invalid data in amx-save-file ([ORACLE-SANDBOX]/malformed-save/state/amx-items). Can’t restore history") nil nil "(amx-test-alpha\n((amx-test-alpha . 9))\n")"#
    ]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn missing_amx_file_migrates_state_from_real_smex_file_and_emits_notice() {
    let elisp_form = r##"
(let* ((root (amx-test-root "smex-migration"))
       (amx-save-file
        (expand-file-name "amx/amx-items" root))
       events)
  (setq smex-save-file
        (amx-test-write
         (expand-file-name "smex/smex-items" root)
         "(amx-test-beta amx-test-alpha)\n((amx-test-alpha . 3) (amx-test-beta . 8))\n"))
  (cl-letf
      (((symbol-function 'message)
        (lambda (&rest arguments)
          (push arguments events))))
    (amx-load-save-file)
    (list
     amx-history
     amx-data
     (file-exists-p amx-save-file)
     (file-exists-p smex-save-file)
     (nreverse events))))
"##;
    let expect = expect![
        "OK ((amx-test-beta amx-test-alpha) ((amx-test-alpha . 3) (amx-test-beta . 8)) nil t ((\"Amx is loading your saved data from smex.\")))"
    ];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn changing_save_file_copies_existing_state_when_destination_is_missing() {
    let elisp_form = r##"
(let* ((root (amx-test-root "copy-save-file"))
       (old
        (amx-test-write
         (expand-file-name "old/amx-items" root)
         "ranked-state\n"))
       (new
        (expand-file-name "new/amx-items" root))
       (amx-save-file old)
       (amx-initialized t))
  (make-directory (file-name-directory new) t)
  (amx-set-save-file 'amx-save-file new)
  (list
   amx-save-file
   (file-exists-p old)
   (file-exists-p new)
   (amx-test-read old)
   (amx-test-read new)
   (file-attribute-size
    (file-attributes old))
   (file-attribute-size
    (file-attributes new))
   (file-modes old)
   (file-modes new)))
"##;
    let expect = expect![[
        r#"OK ("[ORACLE-SANDBOX]/copy-save-file/new/amx-items" t t "ranked-state\n" "ranked-state\n" 13 13 420 420)"#
    ]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn changing_to_existing_save_file_reinitializes_and_restores_its_state() {
    let elisp_form = r##"
(let* ((root (amx-test-root "switch-existing"))
       (old
        (amx-test-write
         (expand-file-name "old/amx-items" root)
         "(old-history)\n((old-data . 1))\n"))
       (new
        (amx-test-write
         (expand-file-name "new/amx-items" root)
         "(amx-test-gamma amx-test-beta)\n((amx-test-beta . 4) (amx-test-gamma . 9))\n"))
       (amx-save-file old)
       (amx-initialized t))
  (setq amx-history '(before)
        amx-data '((before . 2)))
  (amx-set-save-file 'amx-save-file new)
  (list
   amx-save-file
   amx-history
   amx-data
   (mapcar
    (lambda (command)
      (assq command amx-cache))
    '(amx-test-gamma amx-test-beta))
   amx-initialized))
"##;
    let expect = expect![[
        r#"OK ("[ORACLE-SANDBOX]/switch-existing/new/amx-items" (amx-test-gamma amx-test-beta) (#2=(amx-test-beta . 4) #1=(amx-test-gamma . 9)) (#1# #2#) t)"#
    ]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn quick_session_and_nil_destination_never_write_and_warning_is_exact() {
    let elisp_form = r##"
(let* ((root (amx-test-root "no-save"))
       (file (expand-file-name "state/amx-items" root))
       events)
  (cl-letf
      (((symbol-function 'display-warning)
        (lambda (&rest arguments)
          (push arguments events))))
    (let ((amx-save-file file)
          (init-file-user nil))
      (setq amx-cache
            '((amx-test-alpha . 2)))
      (amx-save-to-file))
    (let ((amx-save-file nil)
          (init-file-user "amx-test-user"))
      (amx-save-to-file))
    (list
     (file-exists-p file)
     (nreverse events))))
"##;
    let expect = expect![[r#"OK (nil ((amx "Not saving amx state from \"emacs -Q\".")))"#]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn buffer_content_predicate_distinguishes_whitespace_from_real_state_data() {
    let elisp_form = r##"
(mapcar
 (lambda (contents)
   (with-temp-buffer
     (insert contents)
     (list contents
           (amx-buffer-not-empty-p))))
 '("" " " "\n\t " "()" "  command  " "\n;\n"))
"##;
    let expect = expect![[
        r#"OK (("" nil) (" " nil) ("\n\11 " nil) ("()" 0) ("  command  " 2) ("\n;\n" 1))"#
    ]];
    assert_amx_parity(elisp_form, expect);
}

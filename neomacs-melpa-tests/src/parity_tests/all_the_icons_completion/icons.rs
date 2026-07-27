use expect_test::expect;

use super::assert_all_the_icons_completion_parity;

#[test]
fn real_all_the_icons_file_method_covers_known_special_hidden_unicode_and_unknown_files() {
    let elisp_form = r##"
(mapcar
 (lambda (candidate)
   (let ((icon
          (all-the-icons-completion-get-icon
           candidate
           'file)))
     (list
      candidate
      icon
      (substring-no-properties icon)
      (length icon)
      (get-text-property 0 'face icon)
      (get-text-property 0 'display icon)
      (substring-no-properties icon -1))))
 '("invoice.pdf"
   "app.js"
   "package.json"
   "README.md"
   ".env"
   "archive.tar.gz"
   "zażółć.rs"
   "unknown.zzz"))
"##;
    let expect = expect![[
        r#"OK (("invoice.pdf" #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #1=(:family #4="github-octicons" :height 1.2 :inherit all-the-icons-dred) face #1#)) " " 2 (:family "github-octicons" :height 1.2 :inherit all-the-icons-dred) (raise 0.0) " ") ("app.js" #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #2=(:family #8="all-the-icons" :height 1.2 :inherit all-the-icons-yellow) face #2#)) " " 2 (:family "all-the-icons" :height 1.2 :inherit all-the-icons-yellow) (raise 0.0) " ") ("package.json" #(" " 0 1 (rear-nonsticky t display (raise -0.24) font-lock-face #3=(:family "file-icons" :height 1.2 :inherit all-the-icons-red) face #3#)) " " 2 (:family "file-icons" :height 1.2 :inherit all-the-icons-red) (raise -0.24) " ") ("README.md" #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #5=(:family #4# :height 1.2 :inherit all-the-icons-lcyan) face #5#)) " " 2 (:family "github-octicons" :height 1.2 :inherit all-the-icons-lcyan) (raise 0.0) " ") (".env" #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #6=(:family #4# :height 1.2) face #6#)) " " 2 (:family "github-octicons" :height 1.2) (raise 0.0) " ") ("archive.tar.gz" #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #7=(:family #4# :height 1.2 :inherit all-the-icons-lmaroon) face #7#)) " " 2 (:family "github-octicons" :height 1.2 :inherit all-the-icons-lmaroon) (raise 0.0) " ") ("zażółć.rs" #(" " 0 1 (rear-nonsticky t display (raise -0.24) font-lock-face #9=(:family #8# :height 1.44 :inherit all-the-icons-maroon) face #9#)) " " 2 (:family "all-the-icons" :height 1.44 :inherit all-the-icons-maroon) (raise -0.24) " ") ("unknown.zzz" #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #10=(:family "FontAwesome" :height 1.2 :inherit all-the-icons-dsilver) face #10#)) " " 2 (:family "FontAwesome" :height 1.2 :inherit all-the-icons-dsilver) (raise 0.0) " "))"#
    ]];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn real_directory_icons_use_trailing_slash_and_directory_face_override_in_sandbox() {
    let elisp_form = r##"
(let ((root (make-temp-file "all-icons-completion-dirs-" t)))
  (unwind-protect
      (progn
        (dolist (name '("documents" "code" "mystery"))
          (make-directory (expand-file-name name root)))
        (mapcar
         (lambda (name)
           (let* ((candidate
                   (file-name-as-directory
                    (expand-file-name name root)))
                  (icon
                   (all-the-icons-completion-get-icon
                    candidate
                    'file)))
             (list
              name
              icon
              (substring-no-properties icon)
              (get-text-property 0 'face icon)
              (get-text-property 0 'display icon)
              (substring-no-properties icon -1))))
         '("documents" "code" "mystery")))
    (delete-directory root t)))
"##;
    let expect = expect![[
        r#"OK (("documents" #(" " 0 1 (rear-nonsticky t display (raise -0.12) font-lock-face #1=(:family "FontAwesome" :height 1.2 :inherit all-the-icons-completion-dir-face) face #1#)) " " (:family "FontAwesome" :height 1.2 :inherit all-the-icons-completion-dir-face) (raise -0.12) " ") ("code" #(" " 0 1 (rear-nonsticky t display (raise -0.12) font-lock-face #2=(:family #3="github-octicons" :height 1.32 :inherit all-the-icons-completion-dir-face) face #2#)) " " (:family "github-octicons" :height 1.32 :inherit all-the-icons-completion-dir-face) (raise -0.12) " ") ("mystery" #(" " 0 1 (rear-nonsticky t display (raise -0.12) font-lock-face #4=(:family #3# :height 1.2 :inherit all-the-icons-completion-dir-face) face #4#)) " " (:family "github-octicons" :height 1.2 :inherit all-the-icons-completion-dir-face) (raise -0.12) " "))"#
    ]];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn directory_name_without_trailing_slash_uses_file_path_branch() {
    let elisp_form = r##"
(let* ((candidate "documents")
       (file-icon
        (all-the-icons-completion-get-icon candidate 'file))
       (directory-icon
        (all-the-icons-completion-get-icon "documents/" 'file)))
  (list
   file-icon
   directory-icon
   (equal file-icon directory-icon)
   (substring-no-properties file-icon)
   (substring-no-properties directory-icon)))
"##;
    let expect = expect![[
        r#"OK (#(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #1=(:family #2="FontAwesome" :height 1.2 :inherit all-the-icons-dsilver) face #1#)) #(" " 0 1 (rear-nonsticky t display (raise -0.12) font-lock-face #3=(:family #2# :height 1.2 :inherit all-the-icons-completion-dir-face) face #3#)) nil " " " ")"#
    ]];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn project_file_category_delegates_exactly_to_real_file_category_for_practical_names() {
    let elisp_form = r##"
(mapcar
 (lambda (candidate)
   (let ((file
          (all-the-icons-completion-get-icon candidate 'file))
         (project
          (all-the-icons-completion-get-icon
           candidate
           'project-file)))
     (list
      candidate
      file
      project
      (equal file project))))
 '("src/lib.rs" "README.org" "package.json" "assets/"))
"##;
    let expect = expect![[
        r#"OK (("src/lib.rs" #(" " 0 1 (rear-nonsticky t display #2=(raise -0.24) font-lock-face #1=(:family "all-the-icons" :height 1.44 :inherit all-the-icons-maroon) face #1#)) #(" " 0 1 (rear-nonsticky t display #2# font-lock-face #1# face #1#)) t) ("README.org" #(" " 0 1 (rear-nonsticky t display #4=(raise 0.0) font-lock-face #3=(:family #7="github-octicons" :height 1.2 :inherit all-the-icons-lcyan) face #3#)) #(" " 0 1 (rear-nonsticky t display #4# font-lock-face #3# face #3#)) t) ("package.json" #(" " 0 1 (rear-nonsticky t display #6=(raise -0.24) font-lock-face #5=(:family "file-icons" :height 1.2 :inherit all-the-icons-red) face #5#)) #(" " 0 1 (rear-nonsticky t display #6# font-lock-face #5# face #5#)) t) ("assets/" #(" " 0 1 (rear-nonsticky t display #9=(raise -0.12) font-lock-face #8=(:family #7# :height 1.2 :inherit all-the-icons-completion-dir-face) face #8#)) #(" " 0 1 (rear-nonsticky t display #9# font-lock-face #8# face #8#)) t))"#
    ]];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn buffer_category_uses_real_dependency_filename_mode_and_unknown_mode_fallbacks() {
    let elisp_form = r##"
(let ((file-buffer
       (generate-new-buffer " all-icons-file-buffer"))
      (mode-buffer
       (generate-new-buffer " all-icons-mode-buffer"))
      (unknown-buffer
       (generate-new-buffer " all-icons-unknown-buffer")))
  (unwind-protect
      (progn
        (with-current-buffer file-buffer
          (setq buffer-file-name "/workspace/src/example.rs"
                major-mode 'rust-mode
                auto-mode-alist '(("\\.rs\\'" . rust-mode))))
        (with-current-buffer mode-buffer
          (setq buffer-file-name nil
                major-mode 'emacs-lisp-mode))
        (with-current-buffer unknown-buffer
          (setq buffer-file-name nil
                major-mode 'all-icons-unknown-mode))
        (mapcar
         (lambda (case)
           (let ((icon
                  (all-the-icons-completion-get-icon
                   (buffer-name (cdr case))
                   'buffer)))
             (list
              (car case)
              icon
              (substring-no-properties icon)
              (get-text-property 0 'face icon)
              (get-text-property 0 'display icon))))
         `((filename . ,file-buffer)
           (mode . ,mode-buffer)
           (unknown-fallback . ,unknown-buffer))))
    (mapc
     (lambda (buffer)
       (when (buffer-live-p buffer)
         (kill-buffer buffer)))
     (list file-buffer mode-buffer unknown-buffer))))
"##;
    let expect = expect![[
        r#"OK ((filename #(" " 0 1 (rear-nonsticky t display (raise -0.24) font-lock-face #1=(:family "all-the-icons" :height 1.44 :inherit all-the-icons-maroon) face #1#)) " " (:family "all-the-icons" :height 1.44 :inherit all-the-icons-maroon) (raise -0.24)) (mode #(" " 0 1 (rear-nonsticky t display (raise -0.12) font-lock-face #2=(:family "file-icons" :height 1.2 :inherit all-the-icons-purple) face #2#)) " " (:family "file-icons" :height 1.2 :inherit all-the-icons-purple) (raise -0.12)) (unknown-fallback #(" " 0 1 (rear-nonsticky t display (raise -0.24) font-lock-face #3=(:family "FontAwesome" :height 1.2) face #3#)) " " (:family "FontAwesome" :height 1.2) (raise -0.24)))"#
    ]];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn nonexistent_buffer_candidate_surfaces_exact_with_current_buffer_signal() {
    let elisp_form = r##"
(condition-case error-data
    (list
     'value
     (all-the-icons-completion-get-icon
      " all-icons-missing-buffer"
      'buffer))
  (error
   (list
    'signal
    (car error-data)
    (cadr error-data))))
"##;
    let expect = expect![[r#"OK (signal error "No buffer named  all-icons-missing-buffer")"#]];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn bookmark_category_uses_real_bookmark_records_for_file_and_nonfile_entries() {
    let elisp_form = r##"
(require 'bookmark)
(let ((bookmark-alist
       '(("invoice"
          (filename . "/workspace/accounts/invoice.pdf")
          (position . 1))
         ("source"
          (filename . "/workspace/src/lib.rs")
          (position . 1))
         ("abstract"
          (handler . ignore)))))
  (mapcar
   (lambda (name)
     (let ((filename (bookmark-get-filename name))
           (icon
            (all-the-icons-completion-get-icon
             name
             'bookmark)))
       (list
        name
        filename
        icon
        (substring-no-properties icon)
        (get-text-property 0 'face icon)
        (get-text-property 0 'display icon))))
   '("invoice" "source" "abstract")))
"##;
    let expect = expect![[
        r#"OK (("invoice" "/workspace/accounts/invoice.pdf" #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #1=(:family #3="github-octicons" :height 1.2 :inherit all-the-icons-dred) face #1#)) " " (:family "github-octicons" :height 1.2 :inherit all-the-icons-dred) (raise 0.0)) ("source" "/workspace/src/lib.rs" #(" " 0 1 (rear-nonsticky t display (raise -0.24) font-lock-face #2=(:family "all-the-icons" :height 1.44 :inherit all-the-icons-maroon) face #2#)) " " (:family "all-the-icons" :height 1.44 :inherit all-the-icons-maroon) (raise -0.24)) ("abstract" nil #(" " 0 1 (rear-nonsticky t display (raise -0.24) font-lock-face #4=(:family #3# :height 1.2 :inherit all-the-icons-completion-dir-face) face #4#)) " " (:family "github-octicons" :height 1.2 :inherit all-the-icons-completion-dir-face) (raise -0.24)))"#
    ]];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn documented_custom_generic_method_extension_receives_candidate_and_category() {
    let elisp_form = r##"
(cl-defmethod all-the-icons-completion-get-icon
  (candidate (_category (eql deployment)))
  (format
   "[deploy:%s:%s]"
   candidate
   (if (string-match-p "\\`prod-" candidate)
       "critical"
     "normal")))
(mapcar
 (lambda (candidate)
   (list
    candidate
    (all-the-icons-completion-get-icon
     candidate
     'deployment)))
 '("prod-api" "staging-web" "prod-worker"))
"##;
    let expect = expect![[
        r#"OK (("prod-api" "[deploy:prod-api:critical]") ("staging-web" "[deploy:staging-web:normal]") ("prod-worker" "[deploy:prod-worker:critical]"))"#
    ]];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

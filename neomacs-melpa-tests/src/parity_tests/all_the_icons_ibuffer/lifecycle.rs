use expect_test::expect;

use super::assert_all_the_icons_ibuffer_parity;

#[test]
fn minor_mode_enables_custom_formats_and_requests_silent_full_ibuffer_update() {
    let elisp_form = r##"(let ((buffer (generate-new-buffer "*ati-ibuffer*"))
      calls)
  (unwind-protect
      (with-current-buffer buffer
        (ibuffer-mode)
        (setq-local ibuffer-formats '((mark " " name)))
        (cl-letf (((symbol-function 'ibuffer-update)
                   (lambda (&rest args)
                     (push (list args
                                 all-the-icons-ibuffer-mode
                                 ibuffer-formats)
                           calls))))
          (all-the-icons-ibuffer-mode 1)
          (list all-the-icons-ibuffer-mode
                (local-variable-p 'ibuffer-formats)
                (equal ibuffer-formats
                       all-the-icons-ibuffer-formats)
                (nreverse calls))))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK (t t t (((nil t) t ((mark modified read-only locked " " (icon 2 2) (name 18 18 :left :elide) " " (size-h 9 -1 :right) " " (mode+ 16 16 :left :elide) " " filename-and-process+) (mark " " (name 16 -1) " " filename)))))"#
    ]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn minor_mode_disable_restores_original_global_formats_and_updates_again() {
    let elisp_form = r##"(let ((buffer (generate-new-buffer "*ati-toggle*"))
      calls)
  (unwind-protect
      (with-current-buffer buffer
        (ibuffer-mode)
        (setq-local ibuffer-formats '((mark " local " name)))
        (cl-letf (((symbol-function 'ibuffer-update)
                   (lambda (&rest args)
                     (push (list args
                                 all-the-icons-ibuffer-mode
                                 ibuffer-formats)
                           calls))))
          (all-the-icons-ibuffer-mode 1)
          (all-the-icons-ibuffer-mode -1)
          (list all-the-icons-ibuffer-mode
                (equal ibuffer-formats
                       all-the-icons-ibuffer-old-formats)
                (equal ibuffer-formats
                       '((mark " local " name)))
                (nreverse calls))))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK (nil t nil (((nil t) t ((mark modified read-only locked " " (icon 2 2) (name 18 18 :left :elide) " " (size-h 9 -1 :right) " " (mode+ 16 16 :left :elide) " " filename-and-process+) (mark " " (name 16 -1) " " filename))) ((nil t) nil ((mark modified read-only locked " " (name 18 18 :left :elide) " " (size 9 -1 :right) " " (mode 16 16 :left :elide) " " filename-and-process) (mark " " (name 16 -1) " " filename)))))"#
    ]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn minor_mode_outside_ibuffer_changes_mode_state_without_mutating_formats_or_updating() {
    let elisp_form = r##"(with-temp-buffer
  (text-mode)
  (let ((before ibuffer-formats)
        (updates 0))
    (cl-letf (((symbol-function 'ibuffer-update)
               (lambda (&rest _)
                 (setq updates (1+ updates)))))
      (all-the-icons-ibuffer-mode 1)
      (let ((enabled
             (list all-the-icons-ibuffer-mode
                   (local-variable-p 'ibuffer-formats)
                   (eq before ibuffer-formats)
                   updates)))
        (all-the-icons-ibuffer-mode -1)
        (list enabled
              all-the-icons-ibuffer-mode
              (local-variable-p 'ibuffer-formats)
              (eq before ibuffer-formats)
              updates)))))"##;
    let expect = expect!["OK ((t nil t 0) nil nil t 0)"];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn minor_mode_real_ibuffer_update_renders_a_practical_file_row_and_summary() {
    let elisp_form = r##"(let ((target (generate-new-buffer "ati-target.el"))
      (listing (generate-new-buffer "*ati-real-ibuffer*")))
  (unwind-protect
      (progn
        (with-current-buffer target
          (setq buffer-file-name
                (expand-file-name "projects/ati-target.el"
                                  (getenv "HOME"))
                major-mode 'emacs-lisp-mode
                mode-name "Emacs-Lisp")
          (insert "(message \"all-the-icons-ibuffer\")\n"))
        (with-current-buffer listing
          (ibuffer-mode)
          (setq-local ibuffer-maybe-show-predicates
                      (list
                       (lambda (buffer)
                         (not (eq buffer target)))))
          (setq-local ibuffer-display-maybe-show-predicates nil)
          (let ((all-the-icons-ibuffer-display-predicate
                 (lambda () nil)))
            (all-the-icons-ibuffer-mode 1))
          (goto-char (point-min))
          (let ((found (text-property-search-forward
                        'ibuffer-properties
                        (list target ?\s)
                        #'equal)))
            (list
             all-the-icons-ibuffer-mode
             (equal ibuffer-formats
                    all-the-icons-ibuffer-formats)
             (and found
                  (buffer-substring-no-properties
                   (line-beginning-position)
                   (line-end-position)))
             (buffer-substring-no-properties
              (line-beginning-position 0)
              (line-end-position 0))))))
    (kill-buffer target)
    (kill-buffer listing)))"##;
    let expect = expect![[
        r#"OK (t t " *     ati-target.el             34                  ~/projects/ati-target.el" "[ Default ]")"#
    ]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn two_independent_ibuffer_views_keep_mode_formats_buffer_local() {
    let elisp_form = r##"(let ((first (generate-new-buffer "*ati-first*"))
      (second (generate-new-buffer "*ati-second*"))
      calls)
  (unwind-protect
      (progn
        (dolist (buffer (list first second))
          (with-current-buffer buffer
            (ibuffer-mode)))
        (cl-letf (((symbol-function 'ibuffer-update)
                   (lambda (&rest args)
                     (push (list (buffer-name)
                                 args
                                 all-the-icons-ibuffer-mode)
                           calls))))
          (with-current-buffer first
            (all-the-icons-ibuffer-mode 1))
          (list
           (with-current-buffer first
             (list all-the-icons-ibuffer-mode
                   (equal ibuffer-formats
                          all-the-icons-ibuffer-formats)))
           (with-current-buffer second
             (list all-the-icons-ibuffer-mode
                   (equal ibuffer-formats
                          all-the-icons-ibuffer-formats)))
           (nreverse calls))))
    (kill-buffer first)
    (kill-buffer second)))"##;
    let expect = expect![[r#"OK ((t t) (nil nil) (("*ati-first*" (nil t) t)))"#]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

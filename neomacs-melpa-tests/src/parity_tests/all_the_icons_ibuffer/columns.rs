use expect_test::expect;

use super::assert_all_the_icons_ibuffer_parity;

#[test]
fn mode_column_tracks_real_batch_format_mode_line_results_for_major_modes() {
    let elisp_form = r##"(mapcar
 (lambda (case)
   (let ((buffer (generate-new-buffer (format "*ati-%s*" (car case)))))
     (unwind-protect
         (progn
           (with-current-buffer buffer
             (setq major-mode (cadr case)
                   mode-name (caddr case)))
           (with-temp-buffer
             (funcall (ibuffer-compile-format '(mode+)) buffer ?\s)
             (let ((rendered (buffer-string)))
               (list case rendered
                     (get-text-property 0 'font-lock-face rendered)
                     (get-text-property 0 'mouse-face rendered)
                     (keymapp (get-text-property 0 'keymap rendered))
                     (get-text-property 0 'help-echo rendered)))))
       (kill-buffer buffer))))
 '((text text-mode "Text")
   (elisp emacs-lisp-mode "Emacs-Lisp")
   (compilation compilation-mode ("Comp" mode-line-process))
   (custom ati-mode ("ATI:" (:eval "active")))))"##;
    let expect = expect![[
        r#"OK (((text text-mode "Text") "" nil nil nil nil) ((elisp emacs-lisp-mode "Emacs-Lisp") "" nil nil nil nil) ((compilation compilation-mode ("Comp" mode-line-process)) "" nil nil nil nil) ((custom ati-mode ("ATI:" (:eval "active"))) "" nil nil nil nil))"#
    ]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn mode_column_calls_format_mode_line_for_target_buffer_and_applies_interaction_properties() {
    let elisp_form = r##"(let ((buffer (generate-new-buffer "*ati-mode-target*"))
      calls)
  (unwind-protect
      (progn
        (with-current-buffer buffer
          (setq major-mode 'ati-practical-mode
                mode-name '("ATI" mode-line-process)))
        (cl-letf (((symbol-function 'format-mode-line)
                   (lambda (&rest args)
                     (setq calls args)
                     "ATI Worker")))
          (with-temp-buffer
            (funcall (ibuffer-compile-format '(mode+)) buffer ?\s)
            (let ((rendered (buffer-string)))
              (list rendered
                    (list (car calls)
                          (cadr calls)
                          (caddr calls)
                          (eq (cadddr calls) buffer))
                    (get-text-property 0 'font-lock-face rendered)
                    (get-text-property 0 'mouse-face rendered)
                    (keymapp (get-text-property 0 'keymap rendered))
                    (get-text-property 0 'help-echo rendered))))))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK (#("ATI Worker" 0 10 (font-lock-face all-the-icons-ibuffer-mode-face mouse-face highlight keymap (keymap (13 . ibuffer-interactive-filter-by-mode) (mouse-2 . ibuffer-mouse-filter-by-mode)) help-echo "mouse-2: filter by this mode")) (("ATI" mode-line-process) nil nil t) all-the-icons-ibuffer-mode-face highlight t "mouse-2: filter by this mode")"#
    ]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn filename_column_renders_real_file_buffer_without_a_process() {
    let elisp_form = r##"(let ((buffer (generate-new-buffer "ati-report.md")))
  (unwind-protect
      (progn
        (with-current-buffer buffer
          (setq buffer-file-name
                (expand-file-name
                 "projects/docs/quarterly report.md"
                 (getenv "HOME"))))
        (with-temp-buffer
          (funcall
           (ibuffer-compile-format '(filename-and-process+))
           buffer ?\s)
          (list (buffer-string)
                (get-text-property
                 0 'font-lock-face (buffer-string))
                (get-text-property
                 0 'ibuffer-process (buffer-string)))))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK (#("~/projects/docs/quarterly report.md" 0 35 (font-lock-face all-the-icons-ibuffer-file-face)) all-the-icons-ibuffer-file-face nil)"#
    ]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn filename_column_renders_live_process_status_and_file_in_one_practical_row() {
    let elisp_form = r##"(let ((buffer (generate-new-buffer "ati-worker.log"))
      process)
  (unwind-protect
      (progn
        (with-current-buffer buffer
          (setq buffer-file-name
                (expand-file-name "logs/worker.log" (getenv "HOME"))))
        (setq process
              (start-process
               "ati-worker" buffer
               shell-file-name shell-command-switch "sleep 30"))
        (with-temp-buffer
          (funcall
           (ibuffer-compile-format '(filename-and-process+))
           buffer ?\s)
          (let* ((rendered (buffer-string))
                 (process-value
                  (get-text-property 1 'ibuffer-process rendered)))
            (list rendered
                  (processp process-value)
                  (and (processp process-value)
                       (process-name process-value))
                  (and (processp process-value)
                       (process-status process-value))
                  (get-text-property 0 'font-lock-face rendered)))))
    (when (process-live-p process)
      (delete-process process))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK (#("(ati-worker run) ~/logs/worker.log" 0 16 (ibuffer-process #<process ati-worker> font-lock-face all-the-icons-ibuffer-file-face) 16 34 (font-lock-face all-the-icons-ibuffer-file-face)) t "ati-worker" run all-the-icons-ibuffer-file-face)"#
    ]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn filename_process_summarizer_counts_real_mixed_rows_and_ignores_empty_rows() {
    let elisp_form = r##"(let* ((summarizer
        (get 'ibuffer-make-column-filename-and-process+
             'ibuffer-column-summarizer))
       (process-a (propertize "(worker run) ~/a.log"
                              'ibuffer-process 'worker))
       (process-b (propertize "(server listen)"
                              'ibuffer-process 'server)))
  (mapcar
   (lambda (rows)
     (list rows (funcall summarizer (copy-sequence rows))))
   (list
    nil
    '("")
    '("~/one.el")
    (list process-a)
    (list "~/one.el" "~/two.rs" "")
    (list process-a process-b "~/plain.md" ""))))"##;
    let expect = expect![[
        r#"OK ((nil "No files, no processes") (("") "No files, no processes") (("~/one.el") "1 file, no processes") ((#("(worker run) ~/a.log" 0 20 (ibuffer-process worker))) "1 file, 1 process") (("~/one.el" "~/two.rs" "") "2 files, no processes") ((#("(worker run) ~/a.log" 0 20 (ibuffer-process worker)) #("(server listen)" 0 15 (ibuffer-process server)) "~/plain.md" "") "3 files, 2 processes"))"#
    ]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn complete_custom_format_renders_practical_file_row_with_stubbed_icon() {
    let elisp_form = r##"(let ((buffer (generate-new-buffer "quarterly.rs")))
  (unwind-protect
      (progn
        (with-current-buffer buffer
          (setq buffer-file-name
                (expand-file-name "src/quarterly.rs" (getenv "HOME"))
                major-mode 'rust-mode
                mode-name "Rust")
          (insert "fn main() {\n    println!(\"quarterly\");\n}\n"))
        (cl-letf (((symbol-function 'all-the-icons-auto-mode-match?)
                   (lambda (&optional _) t))
                  ((symbol-function 'all-the-icons-icon-for-file)
                   (lambda (&rest _)
                     (propertize "R" 'face '(:family "RustIcon")))))
          (let ((all-the-icons-ibuffer-display-predicate (lambda () t))
                (all-the-icons-ibuffer-human-readable-size nil)
                (format '(mark modified read-only locked
                          " " (icon 2 2)
                          (name 18 18 :left :elide)
                          " " (size-h 9 -1 :right)
                          " " (mode+ 16 16 :left :elide)
                          " " filename-and-process+)))
            (with-temp-buffer
              (funcall (ibuffer-compile-format format) buffer ?>)
              (let ((rendered (buffer-string)))
                (list rendered
                      (length rendered)
                      (text-properties-at 4 rendered)
                      (text-properties-at 27 rendered)
                      (text-properties-at 37 rendered)))))))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK (#(">*   R quarterly.rs              41                  ~/src/quarterly.rs" 3 4 (font-lock-face ibuffer-locked-buffer) 5 6 (face (:family "RustIcon")) 6 7 (display ((space :relative-width 0.5))) 7 19 (font-lock-face ibuffer-marked help-echo (if tooltip-mode "mouse-1: mark this buffer\nmouse-2: select this buffer\nmouse-3: operate on this buffer" "mouse-1: mark buffer   mouse-2: select buffer   mouse-3: operate") ibuffer-name-column t keymap (keymap (down-mouse-3 . ibuffer-mouse-popup-menu) (mouse-2 . ibuffer-mouse-visit-buffer) (mouse-1 . ibuffer-mouse-toggle-mark)) mouse-face highlight) 33 35 (font-lock-face all-the-icons-ibuffer-size-face) 53 71 (font-lock-face all-the-icons-ibuffer-file-face)) 71 nil nil nil)"#
    ]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn size_and_filename_summaries_compose_into_real_ibuffer_summary_line() {
    let elisp_form = r##"(let ((size-summary
       (get 'ibuffer-make-column-size-h 'ibuffer-column-summarizer))
      (file-summary
       (get 'ibuffer-make-column-filename-and-process+
            'ibuffer-column-summarizer))
      (sizes '("1k" "512" "2M"))
      (rows (list "~/one.el"
                  (propertize "(worker run) ~/two.rs"
                              'ibuffer-process 'worker)
                  "~/three.md")))
  (let ((all-the-icons-ibuffer-human-readable-size t))
    (format "Workspace total: %s across %s"
            (funcall size-summary sizes)
            (funcall file-summary rows))))"##;
    let expect = expect![[r#"OK "Workspace total: 2M across 3 files, 1 process""#]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

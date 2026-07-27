use expect_test::expect;

use super::assert_advent_mode_parity;

#[test]
fn advent_mode_mode_line_uses_explicit_context_real_path_context_and_fallback() {
    let elisp_form = r##"(let* ((root (make-temp-file "advent-mode-" t))
               (day (expand-file-name "year2024/day05/src/" root))
               (advent-root-dir root)
               (advent-mode-line-format "AoC<%s|%s|%s>"))
         (make-directory day t)
         (cl-letf (((symbol-function 'advent--cookie-status-string)
                    (lambda () "COOKIE"))
                   ((symbol-function 'advent--current-year)
                    (lambda () 2026)))
           (list
            (advent--mode-line 2023 9)
            (let ((default-directory day)
                  (buffer-file-name nil))
              (advent--mode-line))
            (let ((default-directory root)
                  (buffer-file-name nil))
              (advent--mode-line))
            (let ((default-directory day)
                  (buffer-file-name nil))
              (advent--mode-line nil 7))
            (let ((default-directory day)
                  (buffer-file-name nil))
              (advent--mode-line 2022 nil)))))"##;
    let expect = expect![[
        r#"OK ("AoC<2023|9|COOKIE>" "AoC<2024|5|COOKIE>" "AoC<xxxx|xx|COOKIE>" "AoC<2024|7|COOKIE>" "AoC<2022|5|COOKIE>")"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_local_minor_mode_changes_real_buffer_state_keymap_and_lighter() {
    let elisp_form = r##"(with-temp-buffer
         (let (updates)
           (cl-letf (((symbol-function 'force-mode-line-update)
                      (lambda (&optional all)
                        (push all updates))))
             (list
              advent-mode
              (advent-mode 1)
              advent-mode
              (assq 'advent-mode minor-mode-alist)
              (assq 'advent-mode minor-mode-map-alist)
              (key-binding (kbd "C-c a p"))
              (key-binding (kbd "C-c a i"))
              (key-binding (kbd "C-c a s"))
              (advent-mode -1)
              advent-mode
              (key-binding (kbd "C-c a p"))
              (nreverse updates)))))"##;
    let expect = expect![
        "OK (nil t t (advent-mode (:eval (advent--mode-line))) (advent-mode keymap (3 keymap (97 keymap (99 . advent-create-day) (100 . advent-open-day) (115 . advent-submit-answer) (105 . advent-fetch-input) (112 . advent-browse-problem-page)))) advent-browse-problem-page advent-fetch-input advent-submit-answer nil nil nil (nil nil nil nil))"
    ];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_maybe_enable_uses_real_inside_and_outside_directories() {
    let elisp_form = r##"(let* ((root (make-temp-file "advent-mode-" t))
               (inside (expand-file-name "year2024/day05/" root))
               (outside (make-temp-file "advent-outside-" t))
               (advent-root-dir root))
         (make-directory inside t)
         (list
          (with-temp-buffer
            (setq default-directory inside)
            (list (advent--maybe-enable)
                  advent-mode
                  (key-binding (kbd "C-c a p"))))
          (with-temp-buffer
            (setq default-directory outside)
            (list (advent--maybe-enable)
                  advent-mode
                  (key-binding (kbd "C-c a p"))))
          (with-temp-buffer
            (setq default-directory inside
                  buffer-file-name
                  (expand-file-name "solution.el" outside))
            (list (advent--maybe-enable)
                  advent-mode
                  (key-binding (kbd "C-c a p"))))))"##;
    let expect = expect!["OK ((t t advent-browse-problem-page) (nil nil nil) (nil nil nil))"];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_global_mode_updates_existing_buffers_and_future_major_mode_changes() {
    let elisp_form = r##"(let* ((root (make-temp-file "advent-mode-" t))
               (inside (expand-file-name "year2024/day05/" root))
               (outside (make-temp-file "advent-outside-" t))
               (inside-buffer
                (generate-new-buffer " *advent-global-inside*"))
               (outside-buffer
                (generate-new-buffer " *advent-global-outside*"))
               (future-buffer
                (generate-new-buffer " *advent-global-future*"))
               (advent-root-dir root))
         (make-directory inside t)
         (unwind-protect
             (progn
               (with-current-buffer inside-buffer
                 (setq default-directory inside))
               (with-current-buffer outside-buffer
                 (setq default-directory outside))
               (with-current-buffer future-buffer
                 (setq default-directory inside))
               (global-advent-mode 1)
               (let ((initial
                      (list
                       global-advent-mode
                       (with-current-buffer inside-buffer advent-mode)
                       (with-current-buffer outside-buffer advent-mode)
                       (with-current-buffer future-buffer advent-mode))))
                 (with-current-buffer future-buffer
                   (fundamental-mode))
                 (let ((after-major-mode
                        (with-current-buffer future-buffer advent-mode)))
                   (global-advent-mode -1)
                   (list
                    initial
                    after-major-mode
                    global-advent-mode
                    (with-current-buffer inside-buffer advent-mode)
                    (with-current-buffer outside-buffer advent-mode)
                    (with-current-buffer future-buffer advent-mode)))))
           (when global-advent-mode
             (global-advent-mode -1))
           (kill-buffer inside-buffer)
           (kill-buffer outside-buffer)
           (kill-buffer future-buffer)))"##;
    let expect = expect!["OK ((t t nil t) t nil nil nil nil)"];
    assert_advent_mode_parity(elisp_form, expect);
}

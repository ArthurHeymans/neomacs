use expect_test::expect;

use super::{assert_advent_mode_autoload_parity, assert_advent_mode_parity};

#[test]
fn advent_mode_defaults_constants_and_custom_metadata_match() {
    let elisp_form = r##"(list
         (featurep 'advent-mode)
         advent-root-dir
         advent-year-dir-format
         advent-day-dir-format
         advent-input-file-name
         advent-new-files
         advent-mode-line-format
         advent-timezone
         advent-open-day-format
         advent-submit-level-history
         advent-session-provider
         advent--first-year
         advent--max-day-exceptions
         (mapcar
          (lambda (symbol)
            (list symbol
                  (get symbol 'custom-type)
                  (get symbol 'custom-group)))
          '(advent-root-dir
            advent-year-dir-format
            advent-day-dir-format
            advent-input-file-name
            advent-new-files
            advent-mode-line-format
            advent-timezone
            advent-open-day-format
            advent-session-provider)))"##;
    let expect = expect![[
        r#"OK (t nil "year%04d" "day%02d" "input.txt" nil " AoC[Y%s/D%s %s]" "America/New_York" "Y%04d/D%02d" nil advent-session-from-auth-source 2015 ((2025 . 12)) ((advent-root-dir directory nil) (advent-year-dir-format string nil) (advent-day-dir-format string nil) (advent-input-file-name string nil) (advent-new-files (repeat file) nil) (advent-mode-line-format string nil) (advent-timezone string nil) (advent-open-day-format string nil) (advent-session-provider function nil)))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_callable_surface_arglists_and_command_status_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list symbol
                 (help-function-arglist symbol t)
                 (commandp symbol)
                 (subrp (symbol-function symbol))))
         '(advent-session-from-auth-source
           advent-session-prompt
           advent--default-aoc-year-day
           advent--max-day
           advent--current-year
           advent--valid-year-day-p
           advent--ensure-valid-year-day-or-error
           advent--problem-dir
           advent--input-path
           advent--format-year-day
           advent--problem-url
           advent--input-url
           advent--answer-url
           advent--normalize-dir
           advent--root
           advent--current-buffer-dir
           advent--relative-dir
           advent--infer-year-day-from-path
           advent--parse-int-segment
           advent--split-int-format
           advent--context-year-day
           advent--ensure-context-or-error
           advent--default-answer
           advent--existing-day-entries
           advent--existing-days
           advent--read-existing-year-day
           advent--ensure-cookie-or-error
           advent--cookie-ok-p
           advent--cookie-status-string
           advent--cookie-get
           advent--refresh-mode-lines
           advent--maybe-create-dir
           advent--copy-templates
           advent--http--status
           advent--http--body
           advent--http-request
           advent--write-url-to-file
           advent--http-post
           advent--prompt-year-day
           advent-login
           advent-browse-problem-page
           advent-fetch-input
           advent-submit-answer
           advent-open-day
           advent-create-day
           advent--mode-line
           advent-mode
           advent--maybe-enable
           global-advent-mode))"##;
    let expect = expect![
        "OK ((advent-session-from-auth-source nil nil nil) (advent-session-prompt nil nil nil) (advent--default-aoc-year-day (time) nil nil) (advent--max-day (year) nil nil) (advent--current-year nil nil nil) (advent--valid-year-day-p (year day) nil nil) (advent--ensure-valid-year-day-or-error (year day) nil nil) (advent--problem-dir (year day root) nil nil) (advent--input-path (year day root) nil nil) (advent--format-year-day (year day) nil nil) (advent--problem-url (year day) nil nil) (advent--input-url (year day) nil nil) (advent--answer-url (year day) nil nil) (advent--normalize-dir (dir) nil nil) (advent--root nil nil nil) (advent--current-buffer-dir nil nil nil) (advent--relative-dir (dir) nil nil) (advent--infer-year-day-from-path (path) nil nil) (advent--parse-int-segment (segment format-string) nil nil) (advent--split-int-format (format-string) nil nil) (advent--context-year-day nil nil nil) (advent--ensure-context-or-error (year day) nil nil) (advent--default-answer nil nil nil) (advent--existing-day-entries (root) nil nil) (advent--existing-days (root) nil nil) (advent--read-existing-year-day (root &optional time) nil nil) (advent--ensure-cookie-or-error nil nil nil) (advent--cookie-ok-p nil nil nil) (advent--cookie-status-string nil nil nil) (advent--cookie-get nil nil nil) (advent--refresh-mode-lines nil nil nil) (advent--maybe-create-dir (dir) nil nil) (advent--copy-templates (paths target root) nil nil) (advent--http--status nil nil nil) (advent--http--body (require-nonempty) nil nil) (advent--http-request (url &optional method data require-nonempty) nil nil) (advent--write-url-to-file (url file) nil nil) (advent--http-post (url data) nil nil) (advent--prompt-year-day (time) nil nil) (advent-login (&optional session) t nil) (advent-browse-problem-page (&optional year day) t nil) (advent-fetch-input (&optional year day) t nil) (advent-submit-answer (answer level &optional year day) t nil) (advent-open-day (&optional year day root) t nil) (advent-create-day (year day &optional root) t nil) (advent--mode-line (&optional year day) nil nil) (advent-mode (&optional arg) t nil) (advent--maybe-enable nil nil nil) (global-advent-mode (&optional arg) t nil))"
    ];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_keymaps_bind_the_complete_command_surface() {
    let elisp_form = r##"(list
         (keymapp advent-command-map)
         (mapcar
          (lambda (key)
            (cons key (lookup-key advent-command-map (kbd key))))
          '("p" "i" "s" "d" "c" "x"))
         (keymapp advent-mode-map)
         (lookup-key advent-mode-map (kbd "C-c a"))
         (eq (lookup-key advent-mode-map (kbd "C-c a"))
             advent-command-map)
         (mapcar
          (lambda (key)
            (cons key (lookup-key advent-mode-map (kbd key))))
          '("C-c a p" "C-c a i" "C-c a s"
            "C-c a d" "C-c a c" "C-c a x")))"##;
    let expect = expect![[
        r#"OK (t (("p" . advent-browse-problem-page) ("i" . advent-fetch-input) ("s" . advent-submit-answer) ("d" . advent-open-day) ("c" . advent-create-day) ("x")) t (keymap (99 . advent-create-day) (100 . advent-open-day) (115 . advent-submit-answer) (105 . advent-fetch-input) (112 . advent-browse-problem-page)) t (("C-c a p" . advent-browse-problem-page) ("C-c a i" . advent-fetch-input) ("C-c a s" . advent-submit-answer) ("C-c a d" . advent-open-day) ("C-c a c" . advent-create-day) ("C-c a x")))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_autoload_registry_exposes_commands_and_globalized_mode() {
    let elisp_form = r##"(list
         (featurep 'advent-mode)
         (mapcar
          (lambda (symbol)
            (list symbol
                  (autoloadp (symbol-function symbol))
                  (nth 1 (symbol-function symbol))
                  (nth 4 (symbol-function symbol))
                  (commandp symbol)))
          '(advent-login
            advent-browse-problem-page
            advent-fetch-input
            advent-submit-answer
            advent-open-day
            advent-create-day
            advent-mode
            global-advent-mode))
         (assq 'advent-mode minor-mode-alist)
         (assq 'advent-mode minor-mode-map-alist))"##;
    let expect = expect![[
        r#"OK (nil ((advent-login t "advent-mode" nil t) (advent-browse-problem-page t "advent-mode" nil t) (advent-fetch-input t "advent-mode" nil t) (advent-submit-answer t "advent-mode" nil t) (advent-open-day t "advent-mode" nil t) (advent-create-day t "advent-mode" nil t) (advent-mode t "advent-mode" nil t) (global-advent-mode t "advent-mode" nil t)) nil nil)"#
    ]];
    assert_advent_mode_autoload_parity(elisp_form, expect);
}

use expect_test::expect;

use super::assert_advent_mode_parity;

#[test]
fn advent_mode_browse_problem_exercises_explicit_context_and_prompt_resolution() {
    let elisp_form = r##"(let (opened prompts context)
         (cl-letf (((symbol-function 'eww-browse-url)
                    (lambda (url &rest arguments)
                      (push (list url arguments) opened)
                      'opened))
                   ((symbol-function 'advent--context-year-day)
                    (lambda () context))
                   ((symbol-function 'advent--prompt-year-day)
                    (lambda (time)
                      (push time prompts)
                      '(2022 9)))
                   ((symbol-function 'advent--current-year)
                    (lambda () 2026)))
           (setq context '(2023 8))
           (let ((explicit (advent-browse-problem-page 2024 5))
                 (inferred (advent-browse-problem-page nil nil)))
             (setq context nil)
             (let ((prompted (advent-browse-problem-page nil nil)))
               (list explicit
                     inferred
                     prompted
                     (nreverse opened)
                     (length prompts)
                     (mapcar #'consp prompts))))))"##;
    let expect = expect![[
        r#"OK (opened opened opened (("https://adventofcode.com/2024/day/5" nil) ("https://adventofcode.com/2023/day/8" nil) ("https://adventofcode.com/2022/day/9" nil)) 1 (t))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_fetch_input_exercises_download_then_cached_file_workflow() {
    let elisp_form = r##"(let* ((root
                (expand-file-name
                 "advent-command/"
                 temporary-file-directory))
               (advent-root-dir root)
               (file (advent--input-path 2024 5 root))
               (working-directory
                (expand-file-name "year2024/day05/src/" root))
               http-calls opened messages cookie-checks)
         (make-directory working-directory t)
         (cl-letf (((symbol-function 'advent--ensure-cookie-or-error)
                    (lambda ()
                      (push 'checked cookie-checks)))
                   ((symbol-function 'advent--http-request)
                    (lambda (&rest arguments)
                      (push arguments http-calls)
                      "first line\nsecond line\n"))
                   ((symbol-function 'find-file-other-window)
                    (lambda (path)
                      (push
                       (list path
                             (file-exists-p path)
                             (and (file-exists-p path)
                                  (with-temp-buffer
                                    (insert-file-contents path)
                                    (buffer-string))))
                       opened)
                      'opened))
                   ((symbol-function 'message)
                    (lambda (format-string &rest arguments)
                      (push (apply #'format format-string arguments)
                            messages)))
                   ((symbol-function 'advent--current-year)
                    (lambda () 2026)))
           (let ((default-directory working-directory)
                 (buffer-file-name nil))
             (let ((downloaded (advent-fetch-input)))
               (with-temp-file file
                 (insert "locally changed\n"))
               (let ((cached (advent-fetch-input)))
                 (list
                  downloaded
                  cached
                  (advent--context-year-day)
                  (nreverse cookie-checks)
                  (nreverse http-calls)
                  (nreverse opened)
                  (nreverse messages)
                  (with-temp-buffer
                    (insert-file-contents file)
                    (buffer-string))))))))"##;
    let expect = expect![[
        r#"OK (opened opened (2024 5) (checked checked) (("https://adventofcode.com/2024/day/5/input" "GET" nil t)) (("[ORACLE-TMPDIR]/advent-command/year2024/day05/input.txt" t "first line\nsecond line\n") ("[ORACLE-TMPDIR]/advent-command/year2024/day05/input.txt" t "locally changed\n")) ("[ORACLE-TMPDIR]/advent-command/year2024/day05/input.txt saved.") "locally changed\n")"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_submit_answer_exercises_encoding_response_buffer_and_display_state() {
    let elisp_form = r##"(let (posts displays messages cookie-checks)
         (when (get-buffer "*AoC Submit*")
           (kill-buffer "*AoC Submit*"))
         (cl-letf (((symbol-function 'advent--ensure-cookie-or-error)
                    (lambda ()
                      (push 'checked cookie-checks)))
                   ((symbol-function 'advent--http-post)
                    (lambda (&rest arguments)
                      (push arguments posts)
                      "<article>accepted &amp; complete</article>"))
                   ((symbol-function 'display-buffer)
                    (lambda (buffer &rest arguments)
                      (push (list (buffer-name buffer) arguments)
                            displays)
                      'window))
                   ((symbol-function 'message)
                    (lambda (format-string &rest arguments)
                      (push (apply #'format format-string arguments)
                            messages)))
                   ((symbol-function 'advent--current-year)
                    (lambda () 2026)))
           (unwind-protect
               (let ((response
                      (advent-submit-answer
                       "answer with spaces & symbols/✓"
                       "2"
                       2024
                       5)))
                 (with-current-buffer "*AoC Submit*"
                   (list
                    response
                    (buffer-string)
                    (point)
                    (point-min)
                    (point-max)
                    (nreverse cookie-checks)
                    (nreverse posts)
                    (nreverse displays)
                    (nreverse messages))))
             (when (get-buffer "*AoC Submit*")
               (kill-buffer "*AoC Submit*")))))"##;
    let expect = expect![[
        r#"OK ("<article>accepted &amp; complete</article>" "<article>accepted &amp; complete</article>" 1 1 43 (checked) (("https://adventofcode.com/2024/day/5/answer" "level=2&answer=answer%20with%20spaces%20%26%20symbols%2F%E2%9C%93")) (("*AoC Submit*" nil)) ("Submitted answer for 2024 day 5 (level 2)"))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_interactive_submission_uses_real_buffer_answer_context_and_prompt_defaults() {
    let elisp_form = r##"(let* ((root
                (expand-file-name
                 "advent-command/"
                 temporary-file-directory))
               (day
                (expand-file-name
                 "year2024/day05/"
                 root))
               (advent-root-dir root)
               read-calls completion-calls posts displays)
         (make-directory day t)
         (when (get-buffer "*AoC Submit*")
           (kill-buffer "*AoC Submit*"))
         (with-temp-buffer
           (setq default-directory day)
           (insert "candidate answer: 12345\n")
           (goto-char 20)
           (cl-letf (((symbol-function 'read-string)
                      (lambda (&rest arguments)
                        (push arguments read-calls)
                        (nth 1 arguments)))
                     ((symbol-function 'completing-read)
                      (lambda (&rest arguments)
                        (push arguments completion-calls)
                        "2"))
                     ((symbol-function 'advent--ensure-cookie-or-error)
                      (lambda () 'cookie-present))
                     ((symbol-function 'advent--http-post)
                      (lambda (&rest arguments)
                        (push arguments posts)
                        "accepted"))
                     ((symbol-function 'display-buffer)
                      (lambda (buffer &rest arguments)
                        (push (list (buffer-name buffer) arguments)
                              displays)
                        'displayed))
                     ((symbol-function 'advent--current-year)
                      (lambda () 2026))
                     ((symbol-function 'message)
                      (lambda (&rest _arguments) nil)))
             (unwind-protect
                 (list
                  (call-interactively #'advent-submit-answer)
                  (advent--context-year-day)
                  (nreverse read-calls)
                  (mapcar
                   (lambda (arguments)
                     (list
                      (nth 0 arguments)
                      (nth 1 arguments)
                      (nth 3 arguments)
                      (nth 6 arguments)))
                   (nreverse completion-calls))
                  (nreverse posts)
                  (nreverse displays)
                  (with-current-buffer "*AoC Submit*"
                    (list (buffer-string)
                          (point))))
               (when (get-buffer "*AoC Submit*")
                 (kill-buffer "*AoC Submit*"))))))"##;
    let expect = expect![[
        r#"OK ("accepted" (2024 5) (("Answer: " "12345" nil)) (("Level: " ("1" "2") t "1")) (("https://adventofcode.com/2024/day/5/answer" "level=2&answer=12345")) (("*AoC Submit*" nil)) ("accepted" 1))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_open_day_exercises_explicit_and_discovered_directory_workflows() {
    let elisp_form = r##"(let* ((root
                (expand-file-name
                 "advent-command/"
                 temporary-file-directory))
               (advent-root-dir root)
               (day-five (advent--problem-dir 2024 5 root))
               (day-six (advent--problem-dir 2024 6 root))
               dired-calls completion-calls)
         (make-directory day-five t)
         (make-directory day-six t)
         (cl-letf (((symbol-function 'dired)
                    (lambda (directory &rest arguments)
                      (push (list directory arguments) dired-calls)
                      'dired-opened))
                   ((symbol-function 'completing-read)
                    (lambda (&rest arguments)
                      (push arguments completion-calls)
                      "Y2024/D06"))
                   ((symbol-function 'advent--default-aoc-year-day)
                    (lambda (_time) '(2024 5)))
                   ((symbol-function 'advent--current-year)
                    (lambda () 2026)))
           (list
            (advent-open-day 2024 5 root)
            (advent-open-day nil nil root)
            (condition-case error-data
                (advent-open-day 2024 7 root)
              (error
               (list 'signal
                     (car error-data)
                     (cdr error-data))))
            (nreverse dired-calls)
            (mapcar
             (lambda (arguments)
               (list (nth 0 arguments)
                     (nth 1 arguments)
                     (nth 3 arguments)
                     (nth 6 arguments)))
             (nreverse completion-calls)))))"##;
    let expect = expect![[
        r#"OK (dired-opened dired-opened (signal user-error ("Day dir does not exist: [ORACLE-TMPDIR]/advent-command/year2024/day07 (use M-x advent-create-day)")) (("[ORACLE-TMPDIR]/advent-command/year2024/day05" nil) ("[ORACLE-TMPDIR]/advent-command/year2024/day06/" nil)) (("Open year/day: " ("Y2024/D05" "Y2024/D06") t "Y2024/D05")))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_create_day_exercises_real_template_copy_and_followup_actions() {
    let elisp_form = r##"(let* ((root
                (expand-file-name
                 "advent-command/"
                 temporary-file-directory))
               (template (expand-file-name "templates/solution.el" root))
               (absolute-root
                (expand-file-name
                 "advent-absolute/"
                 temporary-file-directory))
               (absolute (expand-file-name "helper.py" absolute-root))
               (advent-root-dir root)
               (advent-new-files
                (list "templates/solution.el" absolute))
               answers events messages)
         (make-directory (file-name-directory template) t)
         (make-directory absolute-root t)
         (with-temp-file template (insert "(message \"template\")\n"))
         (with-temp-file absolute (insert "print('helper')\n"))
         (setq answers '(t t t))
         (cl-letf (((symbol-function 'y-or-n-p)
                    (lambda (prompt)
                      (push (list 'prompt prompt) events)
                      (pop answers)))
                   ((symbol-function 'dired)
                    (lambda (directory)
                      (push (list 'dired directory) events)
                      'dired-opened))
                   ((symbol-function 'advent-browse-problem-page)
                    (lambda (&rest arguments)
                      (push (cons 'browse arguments) events)
                      'browsed))
                   ((symbol-function 'advent-fetch-input)
                    (lambda (&rest arguments)
                      (push (cons 'fetch arguments) events)
                      'fetched))
                   ((symbol-function 'message)
                    (lambda (format-string &rest arguments)
                      (push (apply #'format format-string arguments)
                            messages)))
                   ((symbol-function 'advent--current-year)
                    (lambda () 2026)))
           (let* ((result (advent-create-day 2024 5 root))
                  (day (advent--problem-dir 2024 5 root)))
             (list
              result
              (file-directory-p day)
              (sort
               (directory-files day nil
                                directory-files-no-dot-files-regexp)
               #'string<)
              (with-temp-buffer
                (insert-file-contents
                 (expand-file-name "solution.el" day))
                (buffer-string))
              (with-temp-buffer
                (insert-file-contents
                 (expand-file-name "helper.py" day))
                (buffer-string))
              (nreverse events)
              (nreverse messages)))))"##;
    let expect = expect![[
        r#"OK (fetched t ("helper.py" "solution.el") "(message \"template\")\n" "print('helper')\n" ((prompt "Dir created.  Copy template files into it? ") (dired "[ORACLE-TMPDIR]/advent-command/year2024/day05") (prompt "Open the problem page in EWW? ") (browse 2024 5) (prompt "Download and open the input file? ") (fetch 2024 5)) ("Created [ORACLE-TMPDIR]/advent-command/year2024/day05"))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_create_day_existing_directory_decline_and_open_paths_match() {
    let elisp_form = r##"(let* ((root
                (expand-file-name
                 "advent-command/"
                 temporary-file-directory))
               (day (advent--problem-dir 2024 5 root))
               answers events)
         (make-directory day t)
         (cl-letf (((symbol-function 'y-or-n-p)
                    (lambda (prompt)
                      (push (list 'prompt prompt) events)
                      (pop answers)))
                   ((symbol-function 'advent-open-day)
                    (lambda (&rest arguments)
                      (push (cons 'open arguments) events)
                      'opened))
                   ((symbol-function 'advent--current-year)
                    (lambda () 2026)))
           (setq answers '(nil))
           (let ((declined (advent-create-day 2024 5 root)))
             (setq answers '(t))
             (let ((opened (advent-create-day 2024 5 root)))
               (list declined
                     opened
                     (nreverse events))))))"##;
    let expect = expect![[
        r#"OK (nil opened ((prompt "Day dir already exists.  Open it? ") (prompt "Day dir already exists.  Open it? ") (open 2024 5 "[ORACLE-TMPDIR]/advent-command/")))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

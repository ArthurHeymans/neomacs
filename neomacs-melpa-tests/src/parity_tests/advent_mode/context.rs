use expect_test::expect;

use super::assert_advent_mode_parity;

#[test]
fn advent_mode_relative_context_covers_root_day_deep_outside_and_nil_root() {
    let elisp_form = r##"(let* ((root (make-temp-file "advent-root-" t))
               (other (make-temp-file "advent-other-" t))
               (day (expand-file-name "year2024/day05/" root))
               (deep (expand-file-name "year2024/day05/src/lib/" root)))
         (make-directory deep t)
         (cl-letf (((symbol-function 'advent--current-year)
                    (lambda () 2026)))
           (list
            (let ((advent-root-dir nil)
                  (default-directory day)
                  (buffer-file-name nil))
              (list (advent--relative-dir day)
                    (advent--context-year-day)))
            (let ((advent-root-dir root)
                  (default-directory day)
                  (buffer-file-name nil))
              (list (advent--relative-dir root)
                    (advent--relative-dir day)
                    (advent--context-year-day)))
            (let ((advent-root-dir (file-name-as-directory root))
                  (default-directory deep)
                  (buffer-file-name nil))
              (list (advent--relative-dir deep)
                    (advent--context-year-day)))
            (let ((advent-root-dir root)
                  (default-directory other)
                  (buffer-file-name nil))
              (list (advent--relative-dir other)
                    (advent--context-year-day))))))"##;
    let expect = expect![[
        r#"OK ((nil nil) ("./" "year2024/day05/" (2024 5)) ("year2024/day05/src/lib/" (2024 5)) (nil nil))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_current_buffer_directory_prefers_file_then_default_directory() {
    let elisp_form = r##"(let* ((root
                (expand-file-name
                 "advent-root/"
                 temporary-file-directory))
               (day-one (expand-file-name "year2024/day01/" root))
               (day-two (expand-file-name "year2024/day02/" root))
               (file (expand-file-name "solution.el" day-two)))
         (make-directory day-one t)
         (make-directory day-two t)
         (with-temp-file file (insert "answer"))
         (cl-letf (((symbol-function 'advent--current-year)
                    (lambda () 2026)))
           (let ((advent-root-dir root)
                 (default-directory day-one)
                 (buffer-file-name file))
             (list
              (advent--current-buffer-dir)
              (advent--context-year-day)
              (let ((buffer-file-name nil))
                (list (advent--current-buffer-dir)
                      (advent--context-year-day)))))))"##;
    let expect = expect![[
        r#"OK ("[ORACLE-TMPDIR]/advent-root/year2024/day02/" (2024 2) ("[ORACLE-TMPDIR]/advent-root/year2024/day01/" (2024 1)))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_context_resolution_combines_explicit_partial_inferred_and_error_cases() {
    let elisp_form = r##"(cl-letf (((symbol-function 'advent--context-year-day)
                    (lambda () '(2024 7)))
                   ((symbol-function 'advent--current-year)
                    (lambda () 2026)))
         (mapcar
          (lambda (arguments)
            (condition-case error-data
                (apply #'advent--ensure-context-or-error arguments)
              (error
               (list 'signal
                     (car error-data)
                     (cdr error-data)))))
          '((nil nil)
            (2023 nil)
            (nil 9)
            (2022 3)
            (2025 13)
            (2014 1))))"##;
    let expect = expect![[
        r#"OK ((2024 7) (2023 7) (2024 9) (2022 3) (signal user-error ("Invalid AoC year/day: year=2025 day=13 (year 2015..2026, day 1..12)")) (signal user-error ("Invalid AoC year/day: year=2014 day=1 (year 2015..2026, day 1..25)")))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_context_resolution_reports_missing_problem_without_context() {
    let elisp_form = r##"(cl-letf (((symbol-function 'advent--context-year-day)
                    (lambda () nil))
                   ((symbol-function 'advent--current-year)
                    (lambda () 2026)))
         (mapcar
          (lambda (arguments)
            (condition-case error-data
                (apply #'advent--ensure-context-or-error arguments)
              (error
               (list 'signal
                     (car error-data)
                     (cdr error-data)))))
          '((nil nil)
            (2024 nil)
            (nil 5)
            (2024 5))))"##;
    let expect = expect![[
        r#"OK ((signal user-error ("Problem not detected")) (signal user-error ("Problem not detected")) (signal user-error ("Problem not detected")) (2024 5))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_existing_day_discovery_filters_layout_and_preserves_directory_contract() {
    let elisp_form = r##"(let* ((root
                (expand-file-name
                 "advent-root/"
                 temporary-file-directory))
               (advent-open-day-format "Y%04d/D%02d"))
         (make-directory root t)
         (dolist (relative
                  '("year2015/day01/"
                    "year2024/day05/"
                    "year2024/day25/"
                    "year2025/day12/"
                    "year2025/day13/"
                    "year2024/day00/"
                    "year2024/not-a-day/"
                    "not-a-year/day01/"))
           (make-directory (expand-file-name relative root) t))
         (with-temp-file
             (expand-file-name "year2024/day06" root)
           (insert "not a directory"))
         (cl-letf (((symbol-function 'advent--current-year)
                    (lambda () 2026)))
           (list
            (sort (advent--existing-day-entries root)
                  (lambda (left right)
                    (string< (prin1-to-string left)
                             (prin1-to-string right))))
            (sort (advent--existing-days root)
                  (lambda (left right)
                    (string< (car left) (car right)))))))"##;
    let expect = expect![[
        r#"OK (((2015 1 "[ORACLE-TMPDIR]/advent-root/year2015/day01/") (2024 25 "[ORACLE-TMPDIR]/advent-root/year2024/day25/") (2024 5 "[ORACLE-TMPDIR]/advent-root/year2024/day05/") (2025 12 "[ORACLE-TMPDIR]/advent-root/year2025/day12/")) (("Y2015/D01" 2015 1 "[ORACLE-TMPDIR]/advent-root/year2015/day01/") ("Y2024/D05" 2024 5 "[ORACLE-TMPDIR]/advent-root/year2024/day05/") ("Y2024/D25" 2024 25 "[ORACLE-TMPDIR]/advent-root/year2024/day25/") ("Y2025/D12" 2025 12 "[ORACLE-TMPDIR]/advent-root/year2025/day12/")))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_existing_day_reader_uses_calendar_default_then_first_choice() {
    let elisp_form = r##"(let* ((root
                (expand-file-name
                 "advent-root/"
                 temporary-file-directory))
               calls)
         (make-directory root t)
         (dolist (relative
                  '("year2023/day02/"
                    "year2024/day05/"
                    "year2025/day12/"))
           (make-directory (expand-file-name relative root) t))
         (cl-letf (((symbol-function 'advent--current-year)
                    (lambda () 2026))
                   ((symbol-function 'completing-read)
                    (lambda (&rest arguments)
                      (push arguments calls)
                      (nth 6 arguments))))
           (list
            (cl-letf (((symbol-function 'advent--default-aoc-year-day)
                       (lambda (_time) '(2024 5))))
              (advent--read-existing-year-day root 'clock))
            (cl-letf (((symbol-function 'advent--default-aoc-year-day)
                       (lambda (_time) '(2020 1))))
              (advent--read-existing-year-day root 'clock))
            (mapcar
             (lambda (arguments)
               (list (nth 0 arguments)
                     (nth 1 arguments)
                     (nth 3 arguments)
                     (nth 6 arguments)))
             (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK ((2024 5 "[ORACLE-TMPDIR]/advent-root/year2024/day05/") (2023 2 "[ORACLE-TMPDIR]/advent-root/year2023/day02/") (("Open year/day: " ("Y2023/D02" "Y2024/D05" "Y2025/D12") t "Y2024/D05") ("Open year/day: " ("Y2023/D02" "Y2024/D05" "Y2025/D12") t "Y2023/D02")))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_existing_day_reader_signals_exactly_for_empty_roots() {
    let elisp_form = r##"(let ((root
                (expand-file-name
                 "advent-root/"
                 temporary-file-directory)))
         (make-directory root t)
         (condition-case error-data
             (advent--read-existing-year-day root 'clock)
           (error
            (list 'signal
                  (car error-data)
                  (cdr error-data)))))"##;
    let expect = expect![[
        r#"OK (signal user-error ("No AoC day directories found under [ORACLE-TMPDIR]/advent-root/"))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

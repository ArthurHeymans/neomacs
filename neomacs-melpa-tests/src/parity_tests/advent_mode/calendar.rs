use expect_test::expect;

use super::assert_advent_mode_parity;

#[test]
fn advent_mode_current_year_decodes_the_clock_in_the_configured_timezone() {
    let elisp_form = r##"(let ((advent-timezone "Pacific/Kiritimati")
               calls)
         (cl-letf (((symbol-function 'decode-time)
                    (lambda (&rest arguments)
                      (push arguments calls)
                      (make-decoded-time
                       :second 3
                       :minute 2
                       :hour 1
                       :day 31
                       :month 12
                       :year 2026
                       :zone 50400))))
           (list
            (advent--current-year)
            (nreverse calls))))"##;
    let expect = expect![[r#"OK (2026 ((nil "Pacific/Kiritimati")))"#]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_max_day_and_default_calendar_boundaries_match() {
    let elisp_form = r##"(let ((advent-timezone "America/New_York"))
         (list
          (mapcar #'advent--max-day '(2015 2024 2025 2026))
          (mapcar
           (lambda (spec)
             (pcase-let ((`(,year ,month ,day ,hour) spec))
               (list spec
                     (advent--default-aoc-year-day
                      (encode-time 0 0 hour day month year
                                   "America/New_York")))))
           '((2026 12 1 0)
             (2026 12 12 12)
             (2026 12 25 23)
             (2026 12 26 0)
             (2026 12 31 23)
             (2026 11 30 12)
             (2026 1 1 0)
             (2027 1 1 0)))))"##;
    let expect = expect![
        "OK ((25 25 12 25) (((2026 12 1 0) (2026 1)) ((2026 12 12 12) (2026 12)) ((2026 12 25 23) (2026 25)) ((2026 12 26 0) (2026 25)) ((2026 12 31 23) (2026 25)) ((2026 11 30 12) (2025 12)) ((2026 1 1 0) (2025 12)) ((2027 1 1 0) (2026 25))))"
    ];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_year_day_validation_covers_types_ranges_and_exception_year() {
    let elisp_form = r##"(cl-letf (((symbol-function 'advent--current-year)
                    (lambda () 2026)))
         (mapcar
          (lambda (coordinate)
            (list coordinate
                  (advent--valid-year-day-p
                   (car coordinate)
                   (cadr coordinate))))
          '((2015 1)
            (2015 25)
            (2024 25)
            (2025 12)
            (2026 1)
            (2014 1)
            (2027 1)
            (2025 13)
            (2024 0)
            (2024 26)
            (-1 1)
            (2024 -2)
            ("2024" 1)
            (2024 "1")
            (nil 1)
            (2024 nil))))"##;
    let expect = expect![[
        r#"OK (((2015 1) t) ((2015 25) t) ((2024 25) t) ((2025 12) t) ((2026 1) t) ((2014 1) nil) ((2027 1) nil) ((2025 13) nil) ((2024 0) nil) ((2024 26) nil) ((-1 1) nil) ((2024 -2) nil) (("2024" 1) nil) ((2024 "1") nil) ((nil 1) nil) ((2024 nil) nil))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_validation_returns_coordinates_and_preserves_exact_user_errors() {
    let elisp_form = r##"(cl-letf (((symbol-function 'advent--current-year)
                    (lambda () 2026)))
         (mapcar
          (lambda (coordinate)
            (condition-case error-data
                (advent--ensure-valid-year-day-or-error
                 (car coordinate)
                 (cadr coordinate))
              (error
               (list 'signal
                     (car error-data)
                     (cdr error-data)))))
          '((2015 1)
            (2025 12)
            (2026 25)
            (2014 1)
            (2027 1)
            (2025 13)
            (2024 0)
            ("2024" 1))))"##;
    let expect = expect![[
        r#"OK ((2015 1) (2025 12) (2026 25) (signal user-error ("Invalid AoC year/day: year=2014 day=1 (year 2015..2026, day 1..25)")) (signal user-error ("Invalid AoC year/day: year=2027 day=1 (year 2015..2026, day 1..25)")) (signal user-error ("Invalid AoC year/day: year=2025 day=13 (year 2015..2026, day 1..12)")) (signal user-error ("Invalid AoC year/day: year=2024 day=0 (year 2015..2026, day 1..25)")) (signal user-error ("Invalid AoC year/day: year=\"2024\" day=1 (year 2015..2026, day 1..25)")))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_year_day_prompt_forwards_calendar_defaults_and_validates_answers() {
    let elisp_form = r##"(let (calls answers)
         (cl-letf (((symbol-function 'advent--default-aoc-year-day)
                    (lambda (_time) '(2024 17)))
                   ((symbol-function 'read-number)
                    (lambda (prompt default)
                      (push (list prompt default) calls)
                      (pop answers)))
                   ((symbol-function 'advent--current-year)
                    (lambda () 2026)))
           (setq answers '(2023 8))
           (let ((valid (advent--prompt-year-day 'clock)))
             (setq answers '(2025 13))
             (list
              valid
              (condition-case error-data
                  (advent--prompt-year-day 'clock)
                (error
                 (list 'signal
                       (car error-data)
                       (cdr error-data))))
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK ((2023 8) (signal user-error ("Invalid AoC year/day: year=2025 day=13 (year 2015..2026, day 1..12)")) (("Year: " 2024) ("Day: " 17) ("Year: " 2024) ("Day: " 17)))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

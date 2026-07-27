use expect_test::expect;

use super::assert_advent_mode_parity;

#[test]
fn advent_mode_integer_format_splitting_accepts_one_supported_directive_only() {
    let elisp_form = r##"(mapcar
         (lambda (format-string)
           (list format-string
                 (advent--split-int-format format-string)))
         '("%d"
           "%02d"
           "year%04d"
           "day%d-end"
           "prefix%9dsuffix"
           "plain"
           "%%d"
           "%s"
           "%04d-%02d"
           "pre%d%s"
           "x%03dx"))"##;
    let expect = expect![[
        r#"OK (("%d" ("" "")) ("%02d" ("" "")) ("year%04d" ("year" "")) ("day%d-end" ("day" "-end")) ("prefix%9dsuffix" ("prefix" "suffix")) ("plain" nil) ("%%d" nil) ("%s" nil) ("%04d-%02d" nil) ("pre%d%s" nil) ("x%03dx" ("x" "x")))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_integer_segment_parser_enforces_padding_prefix_suffix_and_digits() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (list case
                 (advent--parse-int-segment
                  (car case)
                  (cadr case))))
         '(("year2024" "year%04d")
           ("year0024" "year%04d")
           ("year24" "year%04d")
           ("day05" "day%02d")
           ("day5" "day%02d")
           ("day005" "day%02d")
           ("pre17post" "pre%dpost")
           ("pre017post" "pre%dpost")
           ("pre0post" "pre%dpost")
           ("pre-1post" "pre%dpost")
           ("pre12xpost" "pre%dpost")
           ("12" "%d")
           ("" "%d")
           ("year2024" "year%s")
           ("year2024x" "year%04d")))"##;
    let expect = expect![[
        r#"OK ((("year2024" "year%04d") 2024) (("year0024" "year%04d") 24) (("year24" "year%04d") nil) (("day05" "day%02d") 5) (("day5" "day%02d") nil) (("day005" "day%02d") nil) (("pre17post" "pre%dpost") 17) (("pre017post" "pre%dpost") nil) (("pre0post" "pre%dpost") 0) (("pre-1post" "pre%dpost") nil) (("pre12xpost" "pre%dpost") nil) (("12" "%d") 12) (("" "%d") nil) (("year2024" "year%s") nil) (("year2024x" "year%04d") nil))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_path_inference_covers_default_custom_deep_and_invalid_layouts() {
    let elisp_form = r##"(cl-letf (((symbol-function 'advent--current-year)
                    (lambda () 2026)))
         (list
          (mapcar
           (lambda (path)
             (list path (advent--infer-year-day-from-path path)))
           '("year2015/day01/"
             "year2024/day05/"
             "year2024/day05/src/lib/"
             "year2025/day12/"
             "year2025/day13/"
             "year2024/day00/"
             "year2024/day26/"
             "year2014/day01/"
             "year2027/day01/"
             "2024/05/"
             "year2024/05/"
             "2024/day05/"
             "src/year2024/day05/"
             "year2024/"
             ""
             "/year2024//day05//"))
          (let ((advent-year-dir-format "y%04d")
                (advent-day-dir-format "d%02d"))
            (mapcar
             (lambda (path)
               (list path (advent--infer-year-day-from-path path)))
             '("y2024/d05/"
               "y2024/d05/src/"
               "year2024/day05/"
               "y2024/day05/"
               "y24/d05/")))))"##;
    let expect = expect![[
        r#"OK ((("year2015/day01/" (2015 1)) ("year2024/day05/" (2024 5)) ("year2024/day05/src/lib/" (2024 5)) ("year2025/day12/" (2025 12)) ("year2025/day13/" nil) ("year2024/day00/" nil) ("year2024/day26/" nil) ("year2014/day01/" nil) ("year2027/day01/" nil) ("2024/05/" nil) ("year2024/05/" nil) ("2024/day05/" nil) ("src/year2024/day05/" nil) ("year2024/" nil) ("" nil) ("/year2024//day05//" (2024 5))) (("y2024/d05/" (2024 5)) ("y2024/d05/src/" (2024 5)) ("year2024/day05/" nil) ("y2024/day05/" nil) ("y24/d05/" nil)))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_path_and_url_builders_respect_every_configurable_format() {
    let elisp_form = r##"(let ((advent-year-dir-format "Y-%d")
               (advent-day-dir-format "D-%03d")
               (advent-input-file-name "puzzle.in")
               (advent-open-day-format "%04d::%02d"))
         (list
          (advent--problem-dir 2024 5 "/root/base")
          (advent--problem-dir 2024 5 "/root/base/")
          (advent--input-path 2024 5 "/root/base")
          (advent--format-year-day 2024 5)
          (advent--problem-url 2024 5)
          (advent--input-url 2024 5)
          (advent--answer-url 2024 5)
          (advent--normalize-dir "./relative/../path")
          (let ((advent-root-dir "./relative/../path"))
            (advent--root))
          (let ((advent-root-dir nil))
            (advent--root))))"##;
    let expect = expect![[
        r#"OK ("/root/base/Y-2024/D-005" "/root/base/Y-2024/D-005" "/root/base/Y-2024/D-005/puzzle.in" "2024::05" "https://adventofcode.com/2024/day/5" "https://adventofcode.com/2024/day/5/input" "https://adventofcode.com/2024/day/5/answer" "[ORACLE-SANDBOX]/path/" "[ORACLE-SANDBOX]/path/" nil)"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

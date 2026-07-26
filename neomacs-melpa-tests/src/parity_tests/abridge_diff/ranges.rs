use expect_test::expect;

use super::{assert_abridge_diff_parity, assert_abridge_diff_signal_parity};

#[test]
fn abridge_diff_merge_exclude_merges_only_gaps_below_the_invisible_minimum() {
    let elisp_form = r##"(let ((abridge-diff-invisible-min 5))
               (mapcar
                (lambda (input)
                  (let ((copy (copy-tree input)))
                    (list
                     (abridge-diff-merge-exclude copy)
                     copy)))
                '(((10 20))
                  ((10 20) (24 30))
                  ((10 20) (25 30))
                  ((10 20) (26 30))
                  ((1 3) (4 8) (10 12) (17 20))
                  ((1 10) (2 3) (4 20)))))"##;
    let expect = expect![
        "OK ((nil ((10 20))) (nil ((10 30))) (nil ((10 20) (25 30))) (nil ((10 20) (26 30))) (nil ((1 12) (17 20))) (nil ((1 20))))"
    ];

    assert_abridge_diff_parity(elisp_form, expect);
}

#[test]
fn abridge_diff_merge_exclude_uses_runtime_thresholds_and_preserves_outer_bounds() {
    let elisp_form = r##"(mapcar
               (lambda (minimum)
                 (let ((abridge-diff-invisible-min minimum)
                       (ranges
                        (copy-tree
                         '((4 8) (8 12) (13 15) (21 30)))))
                   (abridge-diff-merge-exclude ranges)
                   (list minimum ranges)))
               '(0 1 6 100))"##;
    let expect = expect![
        "OK ((0 ((4 8) (8 12) (13 15) (21 30))) (1 ((4 12) (13 15) (21 30))) (6 ((4 15) (21 30))) (100 ((4 30))))"
    ];

    assert_abridge_diff_parity(elisp_form, expect);
}

#[test]
fn abridge_diff_compute_hidden_returns_only_strictly_large_outer_and_inner_gaps() {
    let elisp_form = r##"(let ((abridge-diff-invisible-min 5))
               (list
                (abridge-diff-compute-hidden
                 1 60
                 '((10 15) (20 30) (36 40)))
                (abridge-diff-compute-hidden
                 1 46
                 '((7 12) (17 23) (28 40)))
                (let ((abridge-diff-invisible-min 0))
                  (abridge-diff-compute-hidden
                   1 10
                   '((1 2) (2 3) (9 10))))))"##;
    let expect = expect!["OK (((1 10) (30 36) (40 60)) ((1 7) (40 46)) ((3 9)))"];

    assert_abridge_diff_parity(elisp_form, expect);
}

#[test]
fn abridge_diff_compute_hidden_preserves_unsorted_and_overlapping_input_quirks() {
    let elisp_form = r##"(let ((abridge-diff-invisible-min -1))
               (list
                (abridge-diff-compute-hidden
                 0 30
                 '((20 25) (5 10)))
                (abridge-diff-compute-hidden
                 0 30
                 '((5 20) (10 25)))))"##;
    let expect = expect!["OK (((0 20) (10 30)) ((0 5) (25 30)))"];

    assert_abridge_diff_parity(elisp_form, expect);
}

#[test]
fn abridge_diff_compute_hidden_rejects_an_empty_exclusion_list() {
    let elisp_form = r##"(abridge-diff-compute-hidden 1 10 nil)"##;
    let expect = expect!["ERR (wrong-type-argument number-or-marker-p nil)"];

    assert_abridge_diff_signal_parity(elisp_form, expect);
}

#[test]
fn abridge_diff_abridge_processes_both_refined_regions_line_by_line_and_ignores_rest() {
    let elisp_form = r##"(let (events)
               (with-temp-buffer
                 (insert
                  "one\n"
                  "two\n"
                  "three\n"
                  "four\n"
                  "five\n")
                 (cl-letf
                     (((symbol-function
                        'abridge-diff-make-invisible)
                       (lambda (beg end)
                         (push
                          (list
                           beg
                           end
                           (buffer-substring-no-properties
                            beg end))
                          events))))
                   (list
                    (abridge-diff-abridge 1 9 15 24 999 1000)
                    (nreverse events)))))"##;
    let expect = expect![[r#"OK (nil ((1 4 "one") (5 8 "two") (15 19 "four") (20 24 "five")))"#]];

    assert_abridge_diff_parity(elisp_form, expect);
}

#[test]
fn abridge_diff_abridge_honors_optional_magit_file_exclusions_before_processing() {
    let elisp_form = r##"(let ((abridge-diff-exclude-files-matching
                    '("\\.lock\\'" "generated/"))
                   (files
                    '("src/main.rs"
                      "Cargo.lock"
                      "generated/code.rs"))
                   events)
               (cl-letf
                   (((symbol-function 'magit)
                     (lambda () 'magit))
                    ((symbol-function 'magit-file-at-point)
                     (lambda ()
                       (let ((file (pop files)))
                         (push (list 'file file) events)
                         file)))
                    ((symbol-function
                      'abridge-diff-make-invisible)
                     (lambda (beg end)
                       (push (list 'hide beg end) events))))
                 (with-temp-buffer
                   (insert "first line\nsecond line\n")
                   (list
                    (abridge-diff-abridge 1 (point-max))
                    (abridge-diff-abridge 1 (point-max))
                    (abridge-diff-abridge 1 (point-max))
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (nil nil nil ((file "src/main.rs") (hide 1 11) (hide 12 23) (file "Cargo.lock") (file "generated/code.rs")))"#
    ]];

    assert_abridge_diff_parity(elisp_form, expect);
}

#[test]
fn abridge_diff_abridge_skips_magit_lookup_when_the_entry_point_is_unbound() {
    let elisp_form = r##"(let ((abridge-diff-exclude-files-matching '(".*"))
                    events)
               (when (fboundp 'magit)
                 (fmakunbound 'magit))
               (cl-letf
                   (((symbol-function 'magit-file-at-point)
                     (lambda ()
                       (push 'unexpected-file-lookup events)
                       "excluded"))
                    ((symbol-function
                      'abridge-diff-make-invisible)
                     (lambda (beg end)
                       (push (list beg end) events))))
                 (with-temp-buffer
                   (insert "line\n")
                   (list
                    (abridge-diff-abridge 1 (point-max))
                    (nreverse events)))))"##;
    let expect = expect!["OK (nil ((1 5)))"];

    assert_abridge_diff_parity(elisp_form, expect);
}

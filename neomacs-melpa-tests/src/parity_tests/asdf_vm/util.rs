use expect_test::expect;

use super::assert_asdf_vm_batch;

#[test]
fn util_public_surface_batch() {
    assert_asdf_vm_batch(&[
        (
            "asdf_vm_message_prefixes_format_and_forwards_arguments_exactly",
            r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'message)
                     (lambda (&rest arguments)
                       (push arguments calls)
                       :displayed)))
                 (list
                  (asdf-vm-message
                   "Installed %s %d"
                   "ruby"
                   3)
                  (asdf-vm-message
                   "%s"
                   "資料 λ")
                  (nreverse calls))))"##,
            true,
            expect![[
        r#"OK (:displayed :displayed (("[asdf-vm] Installed %s %d" "ruby" 3) ("[asdf-vm] %s" "資料 λ")))"#
    ]],
        ),
        (
            "asdf_vm_parse_skip_list_line_handles_real_current_output_and_whitespace",
            r##"(mapcar
               #'asdf-vm--parse-skip-list-line
               '("ruby          3.3.1          /work/.tool-versions"
                 "nodejs\t20.11.0\t/work/project/.tool-versions"
                 "python  3.12.2   Not installed. Run \"asdf install python 3.12.2\""
                 "資料 λ-version origin with spaces"
                 ""
                 "   leading value tail"
                 "single"))"##,
            true,
            expect![[
        r#"OK (("ruby" "3.3.1" "/work/.tool-versions") ("nodejs" "20.11.0" "/work/project/.tool-versions") ("python" "3.12.2" "Not installed. Run \"asdf install python 3.12.2\"") ("資料" "λ-version" "origin with spaces") nil ("" "leading" "value tail") ("single"))"#
    ]],
        ),
        (
            "asdf_vm_parse_skip_list_line_obeys_token_count_boundaries_without_truncating_tail",
            r##"(mapcar
               (lambda (count)
                 (list
                  count
                  (asdf-vm--parse-skip-list-line
                   "alpha  beta   gamma delta"
                   count)))
               '(0 1 2 3 4 8))"##,
            true,
            expect![[
        r#"OK ((0 ("alpha  beta   gamma delta")) (1 ("alpha  beta   gamma delta")) (2 ("alpha" "beta   gamma delta")) (3 ("alpha" "beta" "gamma delta")) (4 ("alpha" "beta" "gamma" "delta")) (8 ("alpha" "beta" "gamma" "delta")))"#
    ]],
        ),
        (
            "asdf_vm_parse_skip_list_supports_custom_delimiters_and_preserves_unicode_payloads",
            r##"(let ((keep
                    (lambda (character)
                      (not
                       (memq character
                             '(?: ?,)))))
                   (skip
                    (lambda (character)
                      (memq character
                            '(?: ?,)))))
               (list
                (asdf-vm--parse-skip-list-line
                 "ruby::3.3.1,stable"
                 3 keep skip)
                (asdf-vm--parse-skip-list
                 "nodejs::20.0,lts\n資料::λ,nightly"
                 3 keep skip)))"##,
            true,
            expect![[
        r#"OK (("ruby" "3.3.1" "stable") (("nodejs" "20.0" "lts") ("資料" "λ" "nightly")))"#
    ]],
        ),
        (
            "asdf_vm_format_skip_list_aligns_real_tables_and_handles_single_column_and_unicode",
            r##"(list
               (asdf-vm--format-skip-list
                '(("ruby" "3.3.1" "/work/a")
                  ("nodejs" "20.11.0" "/work/long project")
                  ("資料" "λ" "origin"))
                3)
               (asdf-vm--format-skip-list
                '(("ruby")
                  ("nodejs")
                  ("資料")))
               (asdf-vm--format-skip-list
                '(("a" "1")
                  ("long-name" "2"))
                0)
               (asdf-vm-test-error-data
                (lambda ()
                  (asdf-vm--format-skip-list
                   nil))))"##,
            true,
            expect![[
        r#"OK ("ruby     3.3.1     /work/a\nnodejs   20.11.0   /work/long project\n資料       λ         origin" "ruby\nnodejs\n資料" "a        1\nlong-name2" (:ok ""))"#
    ]],
        ),
    ]);
}

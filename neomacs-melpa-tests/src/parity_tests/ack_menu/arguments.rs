use super::{assert_ack_menu_parity, assert_ack_menu_signal_parity};
use expect_test::expect;

#[test]
fn ack_menu_option_and_xor_cover_complete_boolean_tables() {
    let elisp_form = r##"(list
         (ack-option
          "color" t)
         (ack-option
          "color" nil)
         (mapcar
          (lambda (pair)
            (list
             pair
             (ack-xor
              (car pair)
              (cadr pair))))
          '((nil nil)
            (nil t)
            (t nil)
            (t t)
            (fixture nil)
            (fixture value))))"##;
    let expect = expect![[
        r#"OK ("--color" "--nocolor" (((nil nil) nil) ((nil t) t) ((t nil) t) ((t t) nil) ((fixture nil) t) ((fixture value) nil)))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_filter_args_partitions_by_key_and_preserves_relative_order() {
    let elisp_form = r##"(list
         (ack-filter-args
          '(("--match" . "x")
            ("--directory" . "/one")
            ("--ignore-case")
            ("-c")
            ("--directory" . "/two"))
          '("-c" "-bd" "-bp" "--directory"))
         (ack-filter-args
          '(("a" . 1)
            ("b" . 2))
          nil)
         (ack-filter-args
          nil
          '("a")))"##;
    let expect = expect![[
        r#"OK (((("--match" . "x") ("--ignore-case")) (("--directory" . "/one") ("-c"))) ((("a" . 1) ("b" . 2)) nil) (nil nil))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_form_args_list_formats_switches_values_empty_values_and_non_strings() {
    let elisp_form = r##"(ack-form-args-list
         '(("--all")
           ("--match" . "a b")
           ("--empty" . "")
           ("--number" . 12)
           ("--false" . nil)))"##;
    let expect = expect![[r#"OK ("--all" "--match=a b" "--empty=" "--number=12" "--false")"#]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_process_args_filters_controls_adds_fixed_args_and_honors_file_listing() {
    let elisp_form = r##"(let ((directory
                (file-name-as-directory
                 temporary-file-directory))
               (ack-arguments
                '(("--config" . "fixture")
                  ("--extra"))))
         (list
          (ack-process-args
           `(( "--directory" . ,directory)
             ("--match" . "needle")
             ("--ignore-case")
             ("-c")))
          (ack-process-args
           `(( "--directory" . ,directory)
             ("--match" . "removed")
             ("-f")
             ("--word-regexp")))
          (let ((ack-arguments nil))
            (ack-process-args
             `(( "--directory" . ,directory))))))"##;
    let expect = expect![[
        r#"OK (("[ORACLE-TMPDIR]/" ("--color" "--nopager" "--config=fixture" "--extra" "--match=needle" "--ignore-case")) ("[ORACLE-TMPDIR]/" ("--color" "--nopager" "--config=fixture" "--extra" "-f" "--word-regexp")) ("[ORACLE-TMPDIR]/" ("--color" "--nopager")))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_process_args_signals_for_missing_directory() {
    let elisp_form = r##"(cl-letf
         (((symbol-function
            'file-exists-p)
           (lambda (path)
             path
             nil)))
         (ack-process-args
          '(("--directory"
             .
             "/fixture/missing/")
            ("--match"
             .
             "needle"))))"##;
    let expect = expect![[r#"ERR (error "No such directory /fixture/missing/")"#]];
    assert_ack_menu_signal_parity(elisp_form, expect);
}

use expect_test::expect;

use super::{assert_compat_batch};

#[test]
fn core_public_surface_batch() {
    assert_compat_batch(&[
        (
            "compat_error_api_reports_hierarchy_slots_and_fresh_condition_objects",
            r##"(let ((plain
                    (condition-case error
                        (error "boom")
                      (error error)))
                   (type-error
                    (condition-case error
                        (car 5)
                      (error error))))
               (list
                (mapcar
                 (lambda (type)
                   (and (error-type-p type) t))
                 '(error
                   wrong-type-argument
                   car))
                (mapcar
                 (lambda (type)
                   (and
                    (error-has-type-p plain type)
                    t))
                 '(t error wrong-type-argument))
                (mapcar
                 (lambda (type)
                   (and
                    (error-has-type-p type-error type)
                    t))
                 '(error
                   wrong-type-argument
                   wrong-number-of-arguments))
                (error-slot-value plain 1)
                (error-slot-value type-error 1)
                (error-slot-value type-error 2)
                (equal type-error
                       (condition-case error
                           (car 5)
                         (error error)))
                (eq type-error
                    (condition-case error
                        (car 5)
                      (error error)))))"##,
            true,
            expect![[r#"OK ((t t nil) (t t nil) (t t nil) "boom" listp 5 t nil)"#]],
        ),
        (
            "compat_ignore_error_accepts_single_type_and_type_list",
            r##"(list
               (ignore-error end-of-file
                 (read ""))
               (ignore-error (end-of-file wrong-type-argument)
                 (read ""))
               (ignore-error wrong-type-argument
                 (car 3))
               (ignore-error end-of-file
                 (+ 20 22)))"##,
            true,
            expect![[r#"OK (nil nil nil 42)"#]],
        ),
        (
            "compat_ignore_error_propagates_unlisted_condition",
            r##"(ignore-error end-of-file
               (car 3))"##,
            false,
            expect![[r#"ERR (wrong-type-argument listp 3)"#]],
        ),
        (
            "compat_numeric_predicates_cover_fixnums_floats_markers_and_type_errors",
            r##"(with-temp-buffer
               (insert "abc")
               (let ((marker (copy-marker (point-max))))
                 (list
                  (mapcar #'oddp
                          '(1 -1 3 0 -2 10))
                  (mapcar #'evenp
                          '(0 -2 10 1 -1 3))
                  (mapcar #'plusp
                          (list 1 1.5 0 0.0 -1 -1.5 marker))
                  (mapcar #'minusp
                          (list -1 -1.5 0 0.0 1 1.5 marker)))))"##,
            true,
            expect![[
        r#"OK ((t t t nil nil nil) (t t t nil nil nil) (t t nil nil nil nil t) (t t nil nil nil nil nil))"#
    ]],
        ),
        (
            "compat_oddp_rejects_float",
            "(oddp 1.0)",
            false,
            expect![[r#"ERR (wrong-type-argument integer-or-marker-p 1.0)"#]],
        ),
        (
            "compat_string_pad_and_replace_preserve_unicode_and_replacement_literals",
            r##"(list
               (string-pad "λ" 4 ?.)
               (string-pad "abcdef" 3 ?.)
               (string-replace "aa" "X" "baaaac")
               (string-replace "." "\\&" "a.b.c")
               (string-lines "a\nb\n" t)
               (string-lines "a\nb\n" nil))"##,
            true,
            expect![[r#"OK ("λ..." "abcdef" "bXXc" "a\\&b\\&c" ("a" "b") ("a" "b"))"#]],
        ),
        (
            "compat_seconds_to_string_handles_negative_readable_and_precision_modes",
            r##"(list
               (compat-call seconds-to-string -1 'readable)
               (compat-call seconds-to-string 999 'readable)
               (compat-call
                seconds-to-string 999 'readable 'abbrev)
               (compat-call
                seconds-to-string 999 'readable 'abbrev 2)
               (compat-call
                seconds-to-string 999999 'expanded)
               (compat-call
                seconds-to-string 999999 'expanded 'abbrev)
               (compat-call
                seconds-to-string
                999999 'readable 'abbrev 4))"##,
            true,
            expect![[
        r#"OK ("-1 second" "17 minutes" "17m" "16.65m" "1 week 5 days" "1w 5d" "1.6534w")"#
    ]],
        ),
    ]);
}

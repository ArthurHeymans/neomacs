use expect_test::expect;

use super::assert_ac_math_parity;

#[test]
fn ac_math_face_predicate_accepts_only_exact_or_first_math_face() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "abcdef")
               (mapcar
                (lambda (face)
                  (remove-text-properties
                   (point-min)
                   (point-max)
                   '(face nil))
                  (when face
                    (put-text-property
                     3 4
                     'face face))
                  (goto-char
                   3)
                  (list
                   face
                   (ac-math-latex-math-face-p)))
                '(font-latex-math-face
                  (font-latex-math-face
                   bold)
                  (bold
                   font-latex-math-face)
                  bold
                  nil)))"##;
    let expect = expect![[
        r#"OK ((font-latex-math-face t) ((font-latex-math-face bold) t) ((bold font-latex-math-face) nil) (bold nil) (nil nil))"#
    ]];

    assert_ac_math_parity(elisp_form, expect);
}

#[test]
fn ac_math_candidate_sources_gate_exact_constant_identity_by_face_and_toggle() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "x")
               (goto-char
                (point-min))
               (let ((ac-math-unicode-in-math-p
                      nil))
                 (let ((outside-latex
                        (ac-math-candidates-latex))
                       (outside-unicode
                        (ac-math-candidates-unicode)))
                   (put-text-property
                    (point-min)
                    (point-max)
                    'face
                    'font-latex-math-face)
                   (let ((inside-latex
                          (ac-math-candidates-latex))
                         (inside-unicode
                          (ac-math-candidates-unicode)))
                     (setq
                      ac-math-unicode-in-math-p
                      t)
                     (let ((enabled-unicode
                            (ac-math-candidates-unicode)))
                       (list
                        outside-latex
                        (eq
                         outside-unicode
                         ac-math-symbols-unicode)
                        (eq
                         inside-latex
                         ac-math-symbols-latex)
                        inside-unicode
                        (eq
                         enabled-unicode
                         ac-math-symbols-unicode)))))))"##;
    let expect = expect![[r#"OK (nil t t nil t)"#]];

    assert_ac_math_parity(elisp_form, expect);
}

#[test]
fn ac_math_prefix_finds_last_backslash_on_line_and_leaves_point_at_match_start() {
    let elisp_form = r##"(mapcar
               (lambda (fixture)
                 (with-temp-buffer
                   (insert
                    (car fixture))
                   (goto-char
                    (or
                     (cdr fixture)
                     (point-max)))
                   (let ((before
                          (point))
                         (prefix
                          (ac-math-prefix)))
                     (list
                      (car fixture)
                      before
                      prefix
                      (point)
                      (and prefix
                           (buffer-substring-no-properties
                            prefix
                            before))))))
               '(("plain text")
                 ("before \\alpha")
                 ("one \\alpha two \\be")
                 ("\\first\nsecond \\third")
                 ("before \\alpha after"
                  . 14)))"##;
    let expect = expect![[
        r#"OK (("plain text" 11 nil 1 nil) ("before \\alpha" 14 9 8 "alpha") ("one \\alpha two \\be" 19 17 16 "be") ("\\first\nsecond \\third" 21 16 15 "third") ("before \\alpha after" 14 9 8 "alpha"))"#
    ]];

    assert_ac_math_parity(elisp_form, expect);
}

#[test]
fn ac_math_prefix_obeys_live_regexp_and_current_line_boundary() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "@old\ntext @current")
               (goto-char
                (point-max))
               (let ((ac-math-prefix-regexp
                      "@\\([[:alpha:]]*\\)"))
                 (list
                  (ac-math-prefix)
                  (point)
                  (match-string-no-properties
                   1))))"##;
    let expect = expect![[r#"OK (12 11 "current")"#]];

    assert_ac_math_parity(elisp_form, expect);
}

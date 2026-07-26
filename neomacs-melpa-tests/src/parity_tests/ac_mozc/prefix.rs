use expect_test::expect;

use super::assert_ac_mozc_parity;

#[test]
fn ac_mozc_prefix_ports_every_upstream_fixture_across_all_tested_major_modes() {
    let elisp_form = r##"(let* ((fixtures
                     '("emacs"
                       "emacs lisp"
                       "emacs "
                       "one two three"
                       "mo-doresu"
                       "so-suko-do"
                       "変換kouho"
                       "(def"
                       "[def"
                       "{def"
                       "\"def"
                       "'def"
                       "#def"
                       "です."
                       "です,"
                       "はい!"
                       "はい?"
                       "はい!?"
                       "はい!!"
                       "はい!!!"
                       "はい..."
                       " ."
                       " ,"))
                    (probe
                     (lambda (mode)
                       (mapcar
                        (lambda (fixture)
                          (with-temp-buffer
                            (funcall mode)
                            (insert fixture)
                            (let ((prefix
                                   (ac-mozc-prefix)))
                              (and prefix
                                   (buffer-substring-no-properties
                                    prefix
                                    (point-max))))))
                        fixtures)))
                    (fundamental
                     (funcall probe
                              'fundamental-mode)))
               (list
                fundamental
                (mapcar
                 (lambda (mode)
                   (list
                    mode
                    (equal
                     fundamental
                     (funcall probe mode))))
                 '(fundamental-mode
                   text-mode
                   perl-mode
                   ruby-mode))))"##;
    let expect = expect![[
        r#"OK (("emacs" "lisp" nil "three" "mo-doresu" "so-suko-do" "kouho" "def" "def" "def" "def" "def" "def" "." "," "!" "?" "!?" "!!" "!!!" "..." "." ",") ((fundamental-mode t) (text-mode t) (perl-mode t) (ruby-mode t)))"#
    ]];

    assert_ac_mozc_parity(elisp_form, expect);
}

#[test]
fn ac_mozc_prefix_obeys_current_line_boundaries_allowed_punctuation_and_match_data() {
    let elisp_form = r##"(mapcar
               (lambda (fixture)
                 (with-temp-buffer
                   (insert fixture)
                   (goto-char
                    (point-max))
                   (let ((before
                          (point))
                         (prefix
                          (ac-mozc-prefix)))
                     (list
                      fixture
                      before
                      prefix
                      (point)
                      (match-data)
                      (and prefix
                           (buffer-substring-no-properties
                            prefix
                            before))))))
               '("old alpha\nnew beta"
                 "abc_def"
                 "abc/def"
                 "abc,def"
                 "abc.def"
                 "abc-def"
                 "abc123"
                 "日本abc"
                 "abc 日本"))"##;
    let expect = expect![[
        r#"OK (("old alpha\nnew beta" 19 15 19 ((:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil)) "beta") ("abc_def" 8 5 8 ((:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil)) "def") ("abc/def" 8 5 8 ((:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil)) "def") ("abc,def" 8 1 8 ((:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil)) "abc,def") ("abc.def" 8 1 8 ((:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil)) "abc.def") ("abc-def" 8 1 8 ((:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil)) "abc-def") ("abc123" 7 nil 7 ((:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil)) nil) ("日本abc" 6 3 6 ((:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil)) "abc") ("abc 日本" 7 nil 7 ((:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil)) nil))"#
    ]];

    assert_ac_mozc_parity(elisp_form, expect);
}

#[test]
fn ac_mozc_prefix_anchors_at_a_middle_point_ignores_trailing_text_and_restores_point() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "pre abcdef tail")
               (goto-char
                8)
               (let ((before
                      (point))
                     (prefix
                      (ac-mozc-prefix)))
                 (list
                  (buffer-string)
                  before
                  prefix
                  (point)
                  (buffer-substring-no-properties
                   prefix
                   before))))"##;
    let expect = expect![[r#"OK ("pre abcdef tail" 8 5 8 "abc")"#]];

    assert_ac_mozc_parity(elisp_form, expect);
}

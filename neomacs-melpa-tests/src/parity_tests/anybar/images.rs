use expect_test::expect;

use super::assert_anybar_parity;

#[test]
fn image_reset_clears_stale_styles_when_anybar_image_directory_is_absent() {
    let elisp_form = r##"(let ((anybar-images
                         '("stale" "old")))
                     (list
                      (file-directory-p
                       "~/.AnyBar")
                      (anybar-images-reset)
                      anybar-images))"##;
    let expect = expect!["OK (nil nil nil)"];
    assert_anybar_parity(elisp_form, expect);
}

#[test]
fn image_reset_discovers_normal_retina_dark_and_dark_retina_variants_once() {
    let elisp_form = r##"(let ((directory
                         (expand-file-name
                          ".AnyBar"
                          (getenv "HOME"))))
                     (make-directory
                      directory t)
                     (dolist
                         (name
                          '("alpha.png"
                            "ghost.png"
                            "ghost@2x.png"
                            "ghost_alt.png"
                            "ghost_alt@2x.png"
                            "status-red.png"
                            "status-red@2x.png"
                            "ignored.txt"
                            "UPPER.PNG"))
                       (with-temp-file
                           (expand-file-name
                            name
                            directory)
                         (insert name)))
                     (let ((result
                            (anybar-images-reset)))
                       (list
                        result
                        anybar-images
                        (equal
                         result
                         anybar-images)
                        (length
                         (member
                          "ghost"
                          anybar-images)))))"##;
    let expect = expect![[r#"OK (#1=("alpha" "ghost" "status-red") #1# t 2)"#]];
    assert_anybar_parity(elisp_form, expect);
}

#[test]
fn image_reset_exposes_historical_loose_png_regexp_and_directory_entry_behavior() {
    let elisp_form = r##"(let ((directory
                         (expand-file-name
                          ".AnyBar"
                          (getenv "HOME"))))
                     (make-directory
                      directory t)
                     (make-directory
                      (expand-file-name
                       "folder.png"
                       directory)
                      t)
                     (dolist
                         (name
                          '("normal.png"
                            "surpriseXpng"
                            "almost.apng"
                            "no-png-here"))
                       (with-temp-file
                           (expand-file-name
                            name
                            directory)
                         (insert name)))
                     (anybar-images-reset))"##;
    let expect = expect![[r#"OK ("almost." "folder" "normal" "surprise")"#]];
    assert_anybar_parity(elisp_form, expect);
}

#[test]
fn image_reset_uses_exact_home_lookup_protocol_and_preserves_callers_match_data() {
    let elisp_form = r##"(let ((events nil)
                         (anybar-images
                          '("before")))
                     (string-match
                      "\\(outer\\)-\\(match\\)"
                      "outer-match")
                     (let ((before
                            (match-data t)))
                       (cl-letf
                           (((symbol-function
                              'file-directory-p)
                             (lambda (path)
                               (push
                                (list
                                 'directory
                                 path)
                                events)
                               t))
                            ((symbol-function
                              'directory-files)
                             (lambda
                               (directory full match)
                               (push
                                (list
                                 'files
                                 directory
                                 full
                                 match)
                                events)
                               '("alpha.png"
                                 "alpha@2x.png"
                                 "beta_alt.png"))))
                         (let ((result
                                (anybar-images-reset)))
                           (list
                            result
                            (nreverse events)
                            before
                            (match-data t)
                            (match-string 1
                                          "outer-match")
                            (match-string 2
                                          "outer-match"))))))"##;
    let expect = expect![[
        r#"OK (("alpha" "beta") ((directory "~/.AnyBar") (files "~/.AnyBar" nil ".png$")) (0 11 0 5 6 11) (0 11 0 5 6 11) "outer" "match")"#
    ]];
    assert_anybar_parity(elisp_form, expect);
}

#[test]
fn image_reset_short_circuits_directory_listing_when_directory_probe_is_false() {
    let elisp_form = r##"(let ((events nil)
                         (anybar-images
                          '("stale")))
                     (cl-letf
                         (((symbol-function
                            'file-directory-p)
                           (lambda (path)
                             (push
                              (list
                               'directory
                               path)
                              events)
                             nil))
                          ((symbol-function
                            'directory-files)
                           (lambda
                             (&rest arguments)
                             (push
                              (cons
                               'unexpected-files
                               arguments)
                              events)
                             '("wrong.png"))))
                       (list
                        (anybar-images-reset)
                        anybar-images
                        (nreverse events))))"##;
    let expect = expect![[r#"OK (nil nil ((directory "~/.AnyBar")))"#]];
    assert_anybar_parity(elisp_form, expect);
}

#[test]
fn repeated_image_reset_replaces_removed_styles_and_retains_directory_order() {
    let elisp_form = r##"(let ((round 0)
                         (events nil))
                     (cl-letf
                         (((symbol-function
                            'file-directory-p)
                           (lambda (_path)
                             t))
                          ((symbol-function
                            'directory-files)
                           (lambda
                             (&rest _arguments)
                             (setq round
                                   (1+ round))
                             (let ((files
                                    (if
                                        (= round 1)
                                        '("zeta.png"
                                          "alpha.png"
                                          "alpha@2x.png")
                                      '("beta.png"
                                        "alpha_alt.png"))))
                               (push files events)
                               files))))
                       (let ((first
                              (copy-sequence
                               (anybar-images-reset)))
                             (second
                              (copy-sequence
                               (anybar-images-reset))))
                         (list
                          first
                          second
                          anybar-images
                          (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (("zeta" "alpha") ("beta" "alpha") ("beta" "alpha") (("zeta.png" "alpha.png" "alpha@2x.png") ("beta.png" "alpha_alt.png")))"#
    ]];
    assert_anybar_parity(elisp_form, expect);
}

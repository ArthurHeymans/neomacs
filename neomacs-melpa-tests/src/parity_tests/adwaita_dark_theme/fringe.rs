use expect_test::expect;

use super::assert_adwaita_dark_theme_parity;

#[test]
fn adwaita_dark_theme_bitmap_constants_and_load_time_registration_match() {
    let elisp_form = r##"(list
         adwaita-dark-theme--right-arrow-bmp
         adwaita-dark-theme--left-arrow-bmp
         adwaita-dark-theme--down-arrow-bmp
         adwaita-dark-theme--empty-bmp
         adwaita-dark-theme--dot-bmp
         adwaita-dark-theme--diff-hl-bmp
         (mapcar
          (lambda (symbol)
            (list symbol
                  (fringe-bitmap-p symbol)
                  (get symbol 'fringe)))
          '(adwaita-dark-theme--diff-hl-bmp
            adwaita-dark-theme--marker-bmp
            right-arrow
            left-arrow
            right-curly-arrow
            left-curly-arrow)))"##;
    let expect = expect![
        "OK ([0 0 48 56 60 56 48 0] [0 0 12 28 60 28 12 0] [0 0 0 0 0 126 60 24] [0] [96 96] adwaita-dark-theme--diff-hl-bmp ((adwaita-dark-theme--diff-hl-bmp 25 25) (adwaita-dark-theme--marker-bmp 26 26) (right-arrow 4 4) (left-arrow 3 3) (right-curly-arrow 8 8) (left-curly-arrow 7 7)))"
    ];
    assert_adwaita_dark_theme_parity(elisp_form, expect);
}

#[test]
fn adwaita_dark_theme_arrow_setup_redefines_all_four_real_fringe_roles() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'define-fringe-bitmap)
                    (lambda (&rest arguments)
                      (push arguments calls)
                      (car arguments))))
           (list
            (adwaita-dark-theme-arrow-fringe-bmp-enable)
            (nreverse calls))))"##;
    let expect = expect![
        "OK (left-curly-arrow ((right-arrow [0 0 48 56 60 56 48 0]) (left-arrow [0 0 12 28 60 28 12 0]) (right-curly-arrow [0 0 0 0 0 126 60 24]) (left-curly-arrow [0])))"
    ];
    assert_adwaita_dark_theme_parity(elisp_form, expect);
}

#[test]
fn adwaita_dark_theme_diff_hl_setup_installs_callable_bitmap_adapter() {
    let elisp_form = r##"(let ((diff-hl-fringe-bmp-function
                (lambda (&rest _arguments) 'old-bitmap)))
         (adwaita-dark-theme-diff-hl-fringe-bmp-enable)
         (list
          diff-hl-fringe-bmp-function
          (functionp diff-hl-fringe-bmp-function)
          (help-function-arglist
           diff-hl-fringe-bmp-function t)
          (mapcar
           (lambda (arguments)
             (apply diff-hl-fringe-bmp-function arguments))
           '((insert 10)
             (delete 0)
             (change 999)))))"##;
    let expect = expect![
        "OK (#[(&rest _arguments) ('old-bitmap) (t)] t (&rest _arguments) (old-bitmap old-bitmap old-bitmap))"
    ];
    assert_adwaita_dark_theme_parity(elisp_form, expect);
}

#[test]
fn adwaita_dark_theme_flycheck_setup_redefines_levels_and_continuation_bitmap() {
    let elisp_form = r##"(let (level-calls bitmap-calls)
         (cl-letf (((symbol-function
                     'flycheck-redefine-standard-error-levels)
                    (lambda (&rest arguments)
                      (push arguments level-calls)
                      'levels-redefined))
                   ((symbol-function 'define-fringe-bitmap)
                    (lambda (&rest arguments)
                      (push arguments bitmap-calls)
                      (car arguments))))
           (list
            (adwaita-dark-theme-flycheck-fringe-bmp-enable)
            (nreverse level-calls)
            (nreverse bitmap-calls))))"##;
    let expect = expect![
        "OK (flycheck-fringe-bitmap-continuation ((nil adwaita-dark-theme--marker-bmp)) ((flycheck-fringe-bitmap-continuation [96 96])))"
    ];
    assert_adwaita_dark_theme_parity(elisp_form, expect);
}

#[test]
fn adwaita_dark_theme_flymake_setup_replaces_all_diagnostic_bitmap_levels() {
    let elisp_form = r##"(let ((flymake-error-bitmap '(old error))
               (flymake-warning-bitmap '(old warning))
               (flymake-note-bitmap '(old note)))
         (list
          (adwaita-dark-theme-flymake-fringe-bmp-enable)
          flymake-error-bitmap
          flymake-warning-bitmap
          flymake-note-bitmap))"##;
    let expect = expect![
        "OK ((adwaita-dark-theme--marker-bmp compilation-info) (old error) (old warning) (old note))"
    ];
    assert_adwaita_dark_theme_parity(elisp_form, expect);
}

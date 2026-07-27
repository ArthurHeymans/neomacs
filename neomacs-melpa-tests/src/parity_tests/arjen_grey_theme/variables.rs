use expect_test::expect;

use super::assert_arjen_grey_theme_parity;

#[test]
fn arjen_grey_theme_hl_paren_setting_has_exact_quoted_palette_contract() {
    let elisp_form = r##"(let ((setting
                    (seq-find
                     (lambda (candidate)
                       (and
                        (eq (car candidate) 'theme-value)
                        (eq (cadr candidate)
                            'hl-paren-colors)))
                     (get 'arjen-grey 'theme-settings))))
               (list
                setting
                (length (nth 3 setting))
                (car (nth 3 setting))
                (eval (nth 3 setting))
                (length (eval (nth 3 setting)))))"##;
    let expect = expect![[
        r##"OK ((theme-value hl-paren-colors arjen-grey '#1=("#B9F" "#B8D" "#B7B" "#B69" "#B57" "#B45" "#B33" "#B11")) 2 quote #1# 8)"##
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_hl_paren_palette_drives_nested_delimiter_color_lookup() {
    let elisp_form = r##"(let* ((setting
                     (seq-find
                      (lambda (candidate)
                        (eq (cadr candidate)
                            'hl-paren-colors))
                      (get 'arjen-grey 'theme-settings)))
                    (palette (eval (nth 3 setting))))
               (mapcar
                (lambda (depth)
                  (list
                   depth
                   (nth
                    (% depth (length palette))
                    palette)))
                '(0 1 2 3 4 5 6 7 8 9 15 16 23)))"##;
    let expect = expect![[
        r##"OK ((0 "#B9F") (1 "#B8D") (2 "#B7B") (3 "#B69") (4 "#B57") (5 "#B45") (6 "#B33") (7 "#B11") (8 "#B9F") (9 "#B8D") (15 "#B11") (16 "#B9F") (23 "#B11"))"##
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_hl_paren_user_value_is_overridden_then_restored() {
    let elisp_form = r##"(let ((was-bound
                    (boundp 'hl-paren-colors))
                   (old-value
                    (and
                     (boundp 'hl-paren-colors)
                     (default-value
                      'hl-paren-colors)))
                    during
                    after)
               (unwind-protect
                   (progn
                     (set-default
                      'hl-paren-colors
                      '("user-a" "user-b"))
                     (enable-theme 'arjen-grey)
                     (setq during
                           (copy-tree
                            (default-value
                             'hl-paren-colors)))
                     (disable-theme 'arjen-grey)
                     (setq after
                           (copy-tree
                            (default-value
                             'hl-paren-colors))))
                 (when
                     (custom-theme-enabled-p 'arjen-grey)
                   (disable-theme 'arjen-grey))
                 (if was-bound
                     (set-default
                      'hl-paren-colors old-value)
                   (makunbound 'hl-paren-colors)))
               (list during after))"##;
    let expect = expect![[
        r##"OK (("#B9F" "#B8D" "#B7B" "#B69" "#B57" "#B45" "#B33" "#B11") ("user-a" "user-b"))"##
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_runtime_variable_mutation_survives_disable_then_theme_reapplies() {
    let elisp_form = r##"(let ((was-bound
                    (boundp 'hl-paren-colors))
                   (old-value
                    (and
                     (boundp 'hl-paren-colors)
                     (default-value
                      'hl-paren-colors)))
                    mutated
                    disabled
                    reapplied
                    restored)
               (unwind-protect
                   (progn
                     (set-default
                      'hl-paren-colors '(before))
                     (enable-theme 'arjen-grey)
                     (set-default
                      'hl-paren-colors
                      '(runtime mutation))
                     (setq mutated
                           (copy-tree
                            (default-value
                             'hl-paren-colors)))
                     (disable-theme 'arjen-grey)
                     (setq disabled
                           (copy-tree
                            (default-value
                             'hl-paren-colors)))
                     (enable-theme 'arjen-grey)
                     (setq reapplied
                           (copy-tree
                            (default-value
                             'hl-paren-colors)))
                     (disable-theme 'arjen-grey)
                     (setq restored
                           (copy-tree
                            (default-value
                             'hl-paren-colors))))
                 (when
                     (custom-theme-enabled-p 'arjen-grey)
                   (disable-theme 'arjen-grey))
                 (if was-bound
                     (set-default
                      'hl-paren-colors old-value)
                   (makunbound 'hl-paren-colors)))
               (list mutated disabled reapplied
                     restored))"##;
    let expect = expect![[
        r##"OK ((runtime mutation) (before) ("#B9F" "#B8D" "#B7B" "#B69" "#B57" "#B45" "#B33" "#B11") (before))"##
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

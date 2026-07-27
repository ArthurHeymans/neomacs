use expect_test::expect;

use super::assert_arjen_grey_theme_parity;

#[test]
fn arjen_grey_theme_builtin_ui_face_specs_are_complete_and_exact() {
    let elisp_form = r##"(let ((settings
                    (get 'arjen-grey 'theme-settings)))
               (mapcar
                (lambda (face)
                  (seq-find
                   (lambda (setting)
                     (and
                      (eq (car setting) 'theme-face)
                      (eq (cadr setting) face)))
                   settings))
                '(default cursor fringe mode-line region linum
                  secondary-selection minibuffer-prompt)))"##;
    let expect = expect![[
        r##"OK ((theme-face default arjen-grey ((t (:foreground "#bdc3ce" :background "#2a2f38")))) (theme-face cursor arjen-grey ((t (:background "#e1cb8c")))) (theme-face fringe arjen-grey ((t (:background "#2b303a")))) (theme-face mode-line arjen-grey ((t (:foreground "#bdc3ce" :background "#242a34")))) (theme-face region arjen-grey ((t (:background "#3c4449")))) (theme-face linum arjen-grey ((t (:foreground "#595e66" :background "#2a2f38")))) (theme-face secondary-selection arjen-grey ((t (:background "#464a4d")))) (theme-face minibuffer-prompt arjen-grey ((t (:foreground "#a8c194" :bold t)))))"##
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_font_lock_face_specs_are_complete_and_exact() {
    let elisp_form = r##"(let ((settings
                    (get 'arjen-grey 'theme-settings)))
               (mapcar
                (lambda (face)
                  (seq-find
                   (lambda (setting)
                     (and
                      (eq (car setting) 'theme-face)
                      (eq (cadr setting) face)))
                   settings))
                '(font-lock-builtin-face
                  font-lock-comment-face
                  font-lock-function-name-face
                  font-lock-keyword-face
                  font-lock-string-face
                  font-lock-type-face
                  font-lock-constant-face
                  font-lock-variable-name-face
                  font-lock-warning-face)))"##;
    let expect = expect![[
        r##"OK ((theme-face font-lock-builtin-face arjen-grey ((t (:foreground "#eacc8c")))) (theme-face font-lock-comment-face arjen-grey ((t (:foreground "#63747c")))) (theme-face font-lock-function-name-face arjen-grey ((t (:foreground "#909fab")))) (theme-face font-lock-keyword-face arjen-grey ((t (:foreground "#b894b0")))) (theme-face font-lock-string-face arjen-grey ((t (:foreground "#a8c194")))) (theme-face font-lock-type-face arjen-grey ((t (:foreground "#a0a5a0")))) (theme-face font-lock-constant-face arjen-grey ((t (:foreground "#8b9db0")))) (theme-face font-lock-variable-name-face arjen-grey ((t (:foreground "#8294ac")))) (theme-face font-lock-warning-face arjen-grey ((t (:foreground "red" :bold t)))))"##
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_helm_and_perspective_face_specs_are_complete_and_exact() {
    let elisp_form = r##"(let ((settings
                    (get 'arjen-grey 'theme-settings)))
               (mapcar
                (lambda (face)
                  (seq-find
                   (lambda (setting)
                     (and
                      (eq (car setting) 'theme-face)
                      (eq (cadr setting) face)))
                   settings))
                '(helm-header
                  helm-source-header
                  helm-ff-directory
                  helm-selection
                  helm-selection-line
                  persp-selected-face)))"##;
    let expect = expect![[
        r##"OK ((theme-face helm-header arjen-grey ((t (:foreground "#bdc3ce" :background "#2a2f38" :underline nil :box nil)))) (theme-face helm-source-header arjen-grey ((t (:foreground "#bdc3ce" :background "#2a2f38" :underline nil :weight bold :box (:line-width -1 :style released-button))))) (theme-face helm-ff-directory arjen-grey ((t (:foreground "#bdc3ce" :background "#2a2f38" :underline nil :weight bold)))) (theme-face helm-selection arjen-grey ((t (:background "#3c4449" :underline nil)))) (theme-face helm-selection-line arjen-grey ((t (:background "#2a2f38")))) (theme-face persp-selected-face arjen-grey ((t (:foreground "#eacc8c")))))"##
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_company_face_specs_are_complete_and_exact() {
    let elisp_form = r##"(let ((settings
                    (get 'arjen-grey 'theme-settings)))
               (mapcar
                (lambda (face)
                  (seq-find
                   (lambda (setting)
                     (and
                      (eq (car setting) 'theme-face)
                      (eq (cadr setting) face)))
                   settings))
                '(company-tooltip
                  company-tooltip-annotation
                  company-tooltip-selection
                  company-tooltip-mouse
                  company-tooltip-common
                  company-scrollbar-fg
                  company-scrollbar-bg
                  company-preview
                  company-preview-common)))"##;
    let expect = expect![[
        r##"OK ((theme-face company-tooltip arjen-grey ((t (:foreground "#bdc3ce" :background "#242a34")))) (theme-face company-tooltip-annotation arjen-grey ((t (:foreground "#eacc8c")))) (theme-face company-tooltip-selection arjen-grey ((t (:background "#464a4d")))) (theme-face company-tooltip-mouse arjen-grey ((t (:background "#464a4d")))) (theme-face company-tooltip-common arjen-grey ((t (:foreground "#909fab")))) (theme-face company-scrollbar-fg arjen-grey ((t (:background "#464a4d")))) (theme-face company-scrollbar-bg arjen-grey ((t (:background "#242a34")))) (theme-face company-preview arjen-grey ((t (:foreground "#bdc3ce" :background "#242a34")))) (theme-face company-preview-common arjen-grey ((t (:foreground "#909fab")))))"##
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_gnus_and_widget_face_specs_are_complete_and_exact() {
    let elisp_form = r##"(let ((settings
                    (get 'arjen-grey 'theme-settings)))
               (mapcar
                (lambda (face)
                  (seq-find
                   (lambda (setting)
                     (and
                      (eq (car setting) 'theme-face)
                      (eq (cadr setting) face)))
                   settings))
                '(gnus-header-name
                  gnus-header-content
                  gnus-header-subject
                  widget-button
                  gnus-summary-normal-read)))"##;
    let expect = expect![[
        r##"OK ((theme-face gnus-header-name arjen-grey ((t (:foreground "#909fab")))) (theme-face gnus-header-content arjen-grey ((t (:foreground "#bdc3ce")))) (theme-face gnus-header-subject arjen-grey ((t (:foreground "#eacc8c")))) (theme-face widget-button arjen-grey ((t (:foreground "#909fab")))) (theme-face gnus-summary-normal-read arjen-grey ((t (:foreground "#909fab")))))"##
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_every_face_has_one_unconditional_branch_and_valid_plist() {
    let elisp_form = r##"(let* ((settings
                     (seq-filter
                      (lambda (setting)
                        (eq (car setting) 'theme-face))
                      (get 'arjen-grey 'theme-settings)))
                    malformed)
               (dolist (setting settings)
                 (let ((spec (nth 3 setting)))
                   (unless
                       (and
                        (= (length spec) 1)
                        (eq (caar spec) t)
                        (listp (cadar spec))
                        (= (% (length (cadar spec)) 2) 0))
                     (push (cadr setting) malformed))))
               (list
                (length settings)
                (length
                 (delete-dups
                  (mapcar #'cadr settings)))
                (nreverse malformed)
                (secure-hash
                 'sha256
                 (prin1-to-string settings))))"##;
    let expect = expect![[
        r#"OK (37 37 nil "17a3f3791ba3724975ee47c903efc5c6adb634d5c31f823ca39663ac583850b6")"#
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_unconditional_specs_choose_identically_for_color_capabilities() {
    let elisp_form = r##"(let ((settings
                    (seq-filter
                     (lambda (setting)
                       (eq (car setting) 'theme-face))
                     (get 'arjen-grey 'theme-settings))))
               (mapcar
                (lambda (capability)
                  (cl-letf
                      (((symbol-function 'display-color-p)
                        (lambda (&optional _display)
                          (car capability)))
                       ((symbol-function 'display-graphic-p)
                        (lambda (&optional _display)
                          (cadr capability)))
                       ((symbol-function 'display-color-cells)
                        (lambda (&optional _display)
                          (caddr capability))))
                    (list
                     capability
                     (mapcar
                      (lambda (setting)
                        (list
                         (cadr setting)
                         (face-spec-choose
                          (nth 3 setting))))
                      settings))))
                '((t t 16777216)
                  (t nil 256)
                  (t nil 16)
                  (nil nil 2))))"##;
    let expect = expect![[
        r##"OK (((t t 16777216) ((gnus-summary-normal-read #1=(:foreground "#909fab")) (widget-button #2=(:foreground "#909fab")) (gnus-header-subject #3=(:foreground "#eacc8c")) (gnus-header-content #4=(:foreground "#bdc3ce")) (gnus-header-name #5=(:foreground "#909fab")) (company-preview-common #6=(:foreground "#909fab")) (company-preview #7=(:foreground "#bdc3ce" :background "#242a34")) (company-scrollbar-bg #8=(:background "#242a34")) (company-scrollbar-fg #9=(:background "#464a4d")) (company-tooltip-common #10=(:foreground "#909fab")) (company-tooltip-mouse #11=(:background "#464a4d")) (company-tooltip-selection #12=(:background "#464a4d")) (company-tooltip-annotation #13=(:foreground "#eacc8c")) (company-tooltip #14=(:foreground "#bdc3ce" :background "#242a34")) (persp-selected-face #15=(:foreground "#eacc8c")) (helm-selection-line #16=(:background "#2a2f38")) (helm-selection #17=(:background "#3c4449" :underline nil)) (helm-ff-directory #18=(:foreground "#bdc3ce" :background "#2a2f38" :underline nil :weight bold)) (helm-source-header #19=(:foreground "#bdc3ce" :background "#2a2f38" :underline nil :weight bold :box (:line-width -1 :style released-button))) (helm-header #20=(:foreground "#bdc3ce" :background "#2a2f38" :underline nil :box nil)) (font-lock-warning-face #21=(:foreground "red" :bold t)) (minibuffer-prompt #22=(:foreground "#a8c194" :bold t)) (font-lock-variable-name-face #23=(:foreground "#8294ac")) (font-lock-constant-face #24=(:foreground "#8b9db0")) (font-lock-type-face #25=(:foreground "#a0a5a0")) (font-lock-string-face #26=(:foreground "#a8c194")) (font-lock-keyword-face #27=(:foreground "#b894b0")) (font-lock-function-name-face #28=(:foreground "#909fab")) (font-lock-comment-face #29=(:foreground "#63747c")) (font-lock-builtin-face #30=(:foreground "#eacc8c")) (secondary-selection #31=(:background "#464a4d")) (linum #32=(:foreground "#595e66" :background "#2a2f38")) (region #33=(:background "#3c4449")) (mode-line #34=(:foreground "#bdc3ce" :background "#242a34")) (fringe #35=(:background "#2b303a")) (cursor #36=(:background "#e1cb8c")) (default #37=(:foreground "#bdc3ce" :background "#2a2f38")))) ((t nil 256) ((gnus-summary-normal-read #1#) (widget-button #2#) (gnus-header-subject #3#) (gnus-header-content #4#) (gnus-header-name #5#) (company-preview-common #6#) (company-preview #7#) (company-scrollbar-bg #8#) (company-scrollbar-fg #9#) (company-tooltip-common #10#) (company-tooltip-mouse #11#) (company-tooltip-selection #12#) (company-tooltip-annotation #13#) (company-tooltip #14#) (persp-selected-face #15#) (helm-selection-line #16#) (helm-selection #17#) (helm-ff-directory #18#) (helm-source-header #19#) (helm-header #20#) (font-lock-warning-face #21#) (minibuffer-prompt #22#) (font-lock-variable-name-face #23#) (font-lock-constant-face #24#) (font-lock-type-face #25#) (font-lock-string-face #26#) (font-lock-keyword-face #27#) (font-lock-function-name-face #28#) (font-lock-comment-face #29#) (font-lock-builtin-face #30#) (secondary-selection #31#) (linum #32#) (region #33#) (mode-line #34#) (fringe #35#) (cursor #36#) (default #37#))) ((t nil 16) ((gnus-summary-normal-read #1#) (widget-button #2#) (gnus-header-subject #3#) (gnus-header-content #4#) (gnus-header-name #5#) (company-preview-common #6#) (company-preview #7#) (company-scrollbar-bg #8#) (company-scrollbar-fg #9#) (company-tooltip-common #10#) (company-tooltip-mouse #11#) (company-tooltip-selection #12#) (company-tooltip-annotation #13#) (company-tooltip #14#) (persp-selected-face #15#) (helm-selection-line #16#) (helm-selection #17#) (helm-ff-directory #18#) (helm-source-header #19#) (helm-header #20#) (font-lock-warning-face #21#) (minibuffer-prompt #22#) (font-lock-variable-name-face #23#) (font-lock-constant-face #24#) (font-lock-type-face #25#) (font-lock-string-face #26#) (font-lock-keyword-face #27#) (font-lock-function-name-face #28#) (font-lock-comment-face #29#) (font-lock-builtin-face #30#) (secondary-selection #31#) (linum #32#) (region #33#) (mode-line #34#) (fringe #35#) (cursor #36#) (default #37#))) ((nil nil 2) ((gnus-summary-normal-read #1#) (widget-button #2#) (gnus-header-subject #3#) (gnus-header-content #4#) (gnus-header-name #5#) (company-preview-common #6#) (company-preview #7#) (company-scrollbar-bg #8#) (company-scrollbar-fg #9#) (company-tooltip-common #10#) (company-tooltip-mouse #11#) (company-tooltip-selection #12#) (company-tooltip-annotation #13#) (company-tooltip #14#) (persp-selected-face #15#) (helm-selection-line #16#) (helm-selection #17#) (helm-ff-directory #18#) (helm-source-header #19#) (helm-header #20#) (font-lock-warning-face #21#) (minibuffer-prompt #22#) (font-lock-variable-name-face #23#) (font-lock-constant-face #24#) (font-lock-type-face #25#) (font-lock-string-face #26#) (font-lock-keyword-face #27#) (font-lock-function-name-face #28#) (font-lock-comment-face #29#) (font-lock-builtin-face #30#) (secondary-selection #31#) (linum #32#) (region #33#) (mode-line #34#) (fringe #35#) (cursor #36#) (default #37#))))"##
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_repeated_equal_attribute_tails_remain_independent_objects() {
    let elisp_form = r##"(let* ((settings
                     (get 'arjen-grey 'theme-settings))
                    (attributes
                     (lambda (face)
                       (cadar
                        (nth
                         3
                         (seq-find
                          (lambda (setting)
                            (eq (cadr setting) face))
                          settings)))))
                    (selection
                     (funcall attributes
                              'company-tooltip-selection))
                    (mouse
                     (funcall attributes
                              'company-tooltip-mouse))
                    (common
                     (funcall attributes
                              'company-tooltip-common))
                    (preview-common
                     (funcall attributes
                              'company-preview-common)))
               (list
                (equal selection mouse)
                (eq selection mouse)
                (equal common preview-common)
                (eq common preview-common)
                selection
                mouse
                common
                preview-common))"##;
    let expect = expect![[
        r##"OK (t nil t nil (:background "#464a4d") (:background "#464a4d") (:foreground "#909fab") (:foreground "#909fab"))"##
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

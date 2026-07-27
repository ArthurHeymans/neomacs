use expect_test::expect;

use super::{
    assert_ample_flat_theme_parity, assert_ample_light_theme_parity, assert_ample_theme_parity,
};

#[test]
fn ample_theme_dark_settings_capture_every_face_and_variable_exactly() {
    let elisp_form = r##"(let ((settings
                        (get 'ample
                             'theme-settings)))
         (list
          (length settings)
          (cl-count 'theme-face settings
                    :key #'car)
          (cl-count 'theme-value settings
                    :key #'car)
          (secure-hash
           'sha256
           (let ((print-circle nil))
             (prin1-to-string settings)))
          (secure-hash
           'sha256
           (prin1-to-string
            (sort
             (mapcar
              (lambda (setting)
                (list (car setting)
                      (cadr setting)))
              settings)
             (lambda (left right)
               (string<
                (prin1-to-string left)
                (prin1-to-string right))))))))"##;
    let expect = expect![[
        r#"OK (551 550 1 "10282b0829fd4cf2ed7aeab2b2cd951408b94ed94599a551a9cb2e84df7d6945" "4980f1ae21b6fe4668d02e1359177d7392555d34db21c1b62f6a975332990763")"#
    ]];
    assert_ample_theme_parity(elisp_form, expect);
}

#[test]
fn ample_theme_flat_settings_capture_every_face_and_variable_exactly() {
    let elisp_form = r##"(let ((settings
                        (get 'ample-flat
                             'theme-settings)))
         (list
          (length settings)
          (cl-count 'theme-face settings
                    :key #'car)
          (cl-count 'theme-value settings
                    :key #'car)
          (secure-hash
           'sha256
           (let ((print-circle nil))
             (prin1-to-string settings)))
          (secure-hash
           'sha256
           (prin1-to-string
            (sort
             (mapcar
              (lambda (setting)
                (list (car setting)
                      (cadr setting)))
              settings)
             (lambda (left right)
               (string<
                (prin1-to-string left)
                (prin1-to-string right))))))))"##;
    let expect = expect![[
        r#"OK (519 518 1 "c88518382e4163ac0b3e88e3dfb856aa88c98a5b2b827ba5deff187fa6d5a290" "6802c926f18451f24a79d979cec7ff61125e193db3f6172dc62f55478065cac7")"#
    ]];
    assert_ample_flat_theme_parity(elisp_form, expect);
}

#[test]
fn ample_theme_light_settings_capture_every_face_and_variable_exactly() {
    let elisp_form = r##"(let ((settings
                        (get 'ample-light
                             'theme-settings)))
         (list
          (length settings)
          (cl-count 'theme-face settings
                    :key #'car)
          (cl-count 'theme-value settings
                    :key #'car)
          (secure-hash
           'sha256
           (let ((print-circle nil))
             (prin1-to-string settings)))
          (secure-hash
           'sha256
           (prin1-to-string
            (sort
             (mapcar
              (lambda (setting)
                (list (car setting)
                      (cadr setting)))
              settings)
             (lambda (left right)
               (string<
                (prin1-to-string left)
                (prin1-to-string right))))))))"##;
    let expect = expect![[
        r#"OK (490 489 1 "4229e84af717198f28e65aa8b8fed8700f773286eb47388a10ba5ac3e5e65414" "87e62abadc1a035052dd5556c73f9399c6a582876bec2d1e2cacbb659592e4b7")"#
    ]];
    assert_ample_light_theme_parity(elisp_form, expect);
}

#[test]
fn ample_theme_triplet_has_auditable_shared_and_variant_specific_face_sets() {
    let elisp_form = r##"(let* ((directory
                          (file-name-directory
                           (getenv
                            "NEOMACS_PACKAGE_SOURCE")))
               (_flat
                (load
                 (expand-file-name
                  "ample-flat-theme.el"
                  directory)
                 nil t t))
               (_light
                (load
                 (expand-file-name
                  "ample-light-theme.el"
                  directory)
                 nil t t))
               (face-names
                (lambda (theme)
                  (sort
                   (mapcar
                    #'cadr
                    (seq-filter
                     (lambda (setting)
                       (eq (car setting)
                           'theme-face))
                     (get theme
                          'theme-settings)))
                   (lambda (left right)
                     (string<
                      (symbol-name left)
                      (symbol-name right))))))
               (dark
                (funcall face-names 'ample))
               (flat
                (funcall face-names 'ample-flat))
               (light
                (funcall face-names 'ample-light)))
         (list
          (mapcar #'length
                  (list dark flat light))
          (length
           (seq-intersection
            dark
            (seq-intersection flat light)))
          (seq-difference dark flat)
          (seq-difference flat dark)
          (seq-difference light dark)
          (secure-hash
           'sha256
           (prin1-to-string
            (list dark flat light)))))"##;
    let expect = expect![[
        r#"OK ((550 518 489) 478 (TeX-error-description-error TeX-error-description-tex-said TeX-error-description-warning avy-goto-char-timer-face avy-lead-face-2 font-latex-verbatim-face helm-bookmark-addressbook helm-bookmark-file helm-bookmark-gnus helm-bookmark-info helm-bookmark-man helm-bookmark-w3m helm-buffer-directory helm-grep-finish helm-locate-finish helm-moccur-buffer helm-prefarg helm-visible-mark magit-bisect-bad magit-bisect-good magit-bisect-skip magit-cherry-equivalent magit-cherry-unmatched magit-diff-file-heading-selection magit-diff-hunk-heading-selection magit-diff-lines-heading magit-diffstat-added magit-diffstat-removed magit-popup-argument magit-process-ng magit-process-ok magit-reflog-amend magit-reflog-checkout magit-reflog-cherry-pick magit-reflog-commit magit-reflog-merge magit-reflog-other magit-reflog-rebase magit-reflog-remote magit-reflog-reset magit-section-heading-selection magit-sequence-head magit-sequence-part magit-sequence-stop magit-signature-error magit-signature-good magit-signature-revoked magit-signature-untrusted powerline-active2 powerline-inactive1 powerline-inactive2 shadow) (font-latex-sectioning-0-face font-latex-sectioning-1-face font-latex-sectioning-2-face font-latex-sectioning-3-face font-latex-sectioning-4-face font-latex-subscript-face font-latex-superscript-face helm-grep-match slime-apropos-label slime-apropos-symbol slime-error-face slime-highlight-face slime-inspector-action-face slime-inspector-label-face slime-inspector-topline-face slime-inspector-type-face slime-inspector-value-face slime-note-face slime-style-warning-face slime-warning-face) (company-tooltip-annotation font-latex-sectioning-0-face font-latex-sectioning-1-face font-latex-sectioning-2-face font-latex-sectioning-3-face font-latex-sectioning-4-face font-latex-subscript-face font-latex-superscript-face lsp-face-highlight-read lsp-face-highlight-textual lsp-face-highlight-write) "3effeff1494475cf8ec4d6365008ea044d94a438087d1f829db06fbcc8c849e6")"#
    ]];
    assert_ample_theme_parity(elisp_form, expect);
}

#[test]
fn ample_theme_triplet_defines_exact_ansi_color_vectors() {
    let elisp_form = r##"(let ((directory
                        (file-name-directory
                         (getenv
                          "NEOMACS_PACKAGE_SOURCE"))))
         (load
          (expand-file-name
           "ample-flat-theme.el" directory)
          nil t t)
         (load
          (expand-file-name
           "ample-light-theme.el" directory)
          nil t t)
         (mapcar
          (lambda (theme)
            (let ((setting
                   (seq-find
                    (lambda (entry)
                      (and
                       (eq (car entry)
                           'theme-value)
                       (eq (cadr entry)
                           'ansi-color-names-vector)))
                    (get theme
                         'theme-settings))))
              (list theme setting)))
          '(ample ample-flat ample-light)))"##;
    let expect = expect![[
        r##"OK ((ample (theme-value ansi-color-names-vector ample ["#454545" "#cd5542" "#6aaf50" "#baba36" "#5180b3" "#ab75c3" "#68a5e9" "#bdbdb3"])) (ample-flat (theme-value ansi-color-names-vector ample-flat ["#504545" "#ad8572" "#a9df90" "#aaca86" "#91a0b3" "#ab85a3" "#afcfef" "#bdbdb3"])) (ample-light (theme-value ansi-color-names-vector ample-light ["#757575" "#CD5542" "#4A8F30" "#7D7C21" "#4170B3" "#9B55C3" "#68A5E9" "gray43"])))"##
    ]];
    assert_ample_theme_parity(elisp_form, expect);
}

#[test]
fn ample_theme_representative_optional_face_specs_cover_major_ecosystem_groups() {
    let elisp_form = r##"(mapcar
         (lambda (face)
           (assq
            face
            (mapcar
             (lambda (setting)
               (cons (cadr setting)
                     (cddr setting)))
             (seq-filter
              (lambda (setting)
                (eq (car setting)
                    'theme-face))
              (get 'ample
                   'theme-settings)))))
         '(org-level-1
           magit-diff-added
           company-tooltip
           helm-selection
           ivy-current-match
           lsp-headerline-breadcrumb-path-error-face
           neo-vc-unlocked-changes-face
           realgud-backtrace-number
           term-color-red
           widget-field))"##;
    let expect = expect![[
        r##"OK (nil (magit-diff-added ample ((t (:background unspecified :foreground "#6aaf50")))) (company-tooltip ample ((t (:foreground "gray13" :background "#bdbdb3")))) (helm-selection ample ((t (:foreground "#baba36" :background "#303030" :bold t)))) nil (lsp-headerline-breadcrumb-path-error-face ample ((t (:foreground "gray13" :background unspecified :underline "#cd5542")))) (neo-vc-unlocked-changes-face ample ((t (:foreground "#cd5542" :background "Blue")))) (realgud-backtrace-number ample ((t (:foreground "#baba36" :weight bold)))) (term-color-red ample ((t (:foreground "#cd5542" :background "#cd5542")))) (widget-field ample ((t (:foreground "#bdbdb3" :background "#656565")))))"##
    ]];
    assert_ample_theme_parity(elisp_form, expect);
}

#[test]
fn ample_theme_generated_face_specs_do_not_share_mutable_cons_tails_across_faces() {
    let elisp_form = r##"(let ((directory
                        (file-name-directory
                         (getenv
                          "NEOMACS_PACKAGE_SOURCE"))))
         (load
          (expand-file-name
           "ample-flat-theme.el" directory)
          nil t t)
         (load
          (expand-file-name
           "ample-light-theme.el" directory)
          nil t t)
         (mapcar
          (lambda (theme)
            (let ((owners
                   (make-hash-table :test 'eq))
                  nodes)
              (dolist
                  (setting
                   (get theme 'theme-settings))
                (let ((owner (cadr setting))
                      (pending (list setting))
                      (seen
                       (make-hash-table
                        :test 'eq)))
                  (while pending
                    (let ((value (pop pending)))
                      (when
                          (and
                           (consp value)
                           (not
                            (gethash value seen)))
                        (puthash value t seen)
                        (unless
                            (gethash value owners)
                          (push value nodes))
                        (puthash
                         value
                         (cons
                          owner
                          (gethash value owners))
                         owners)
                        (push (car value) pending)
                        (push (cdr value) pending))))))
              (let (shared)
                (dolist (node (nreverse nodes))
                  (let ((node-owners
                         (nreverse
                          (gethash node owners))))
                    (when
                        (> (length node-owners) 1)
                      (push node-owners shared))))
                (setq shared (nreverse shared))
                (list
                 theme
                 (length shared)
                 (seq-take shared 5)))))
          '(ample ample-flat ample-light)))"##;
    let expect = expect!["OK ((ample 0 nil) (ample-flat 0 nil) (ample-light 0 nil))"];
    assert_ample_theme_parity(elisp_form, expect);
}

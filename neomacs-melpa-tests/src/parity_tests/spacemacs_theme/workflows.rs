use expect_test::expect;

use super::ParityBatchCase;

fn defcustoms_match_upstream_defaults() -> ParityBatchCase {
    ParityBatchCase::value(
        "defcustoms_match_upstream_defaults",
        r####"
(list :comment-bg spacemacs-theme-comment-bg
      :comment-italic spacemacs-theme-comment-italic
      :org-height spacemacs-theme-org-height
      :custom-colors spacemacs-theme-custom-colors
      :underline-parens spacemacs-theme-underline-parens)
"####,
        expect![[
            r#"OK (:comment-bg t :comment-italic nil :org-height t :custom-colors nil :underline-parens t)"#
        ]],
    )
}

fn load_theme_registers_and_enables_spacemacs_dark() -> ParityBatchCase {
    ParityBatchCase::value(
        "load_theme_registers_and_enables_spacemacs_dark",
        r####"
(progn
  (load-theme 'spacemacs-dark t)
  (list :theme-p (and (custom-theme-p 'spacemacs-dark) t)
        :enabled (and (custom-theme-enabled-p 'spacemacs-dark) t)
        :in-enabled (and (memq 'spacemacs-dark custom-enabled-themes) t)
        :feature (get 'spacemacs-dark 'theme-feature)))
"####,
        expect![[r#"OK (:theme-p t :enabled t :in-enabled t :feature spacemacs-dark-theme)"#]],
    )
}

fn theme_settings_include_default_and_font_lock_faces() -> ParityBatchCase {
    ParityBatchCase::value(
        "theme_settings_include_default_and_font_lock_faces",
        r####"
(progn
  (load-theme 'spacemacs-dark t)
  (let ((faces
         (mapcar #'cadr
                 (cl-remove-if-not
                  (lambda (s) (eq (car s) 'theme-face))
                  (get 'spacemacs-dark 'theme-settings)))))
    (list :has-default (and (memq 'default faces) t)
          :has-comment (and (memq 'font-lock-comment-face faces) t)
          :has-keyword (and (memq 'font-lock-keyword-face faces) t)
          :many-faces (> (length faces) 50))))
"####,
        expect![[r#"OK (:has-default t :has-comment t :has-keyword t :many-faces t)"#]],
    )
}

pub(super) fn workflow_batch_cases() -> Vec<ParityBatchCase> {
    vec![
        defcustoms_match_upstream_defaults(),
        load_theme_registers_and_enables_spacemacs_dark(),
        theme_settings_include_default_and_font_lock_faces(),
    ]
}

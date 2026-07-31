use expect_test::expect;

use super::{assert_all_the_icons_nerd_fonts_autoload_batch, assert_all_the_icons_nerd_fonts_batch};

#[test]
fn registry_all_the_icons_nerd_fonts_batch() {
    assert_all_the_icons_nerd_fonts_batch(&[
        (
            "package_loads_real_dependencies_and_immediately_renders_a_nerd_font_icon",
            r##"(let ((icon
                      (all-the-icons-nerd-fa
                       "github"
                       :face 'font-lock-constant-face)))
               (list
                (featurep 'all-the-icons-nerd-fonts)
                (featurep 'all-the-icons)
                (featurep 'nerd-icons-data)
                (substring-no-properties icon)
                (string-to-list
                 (substring-no-properties icon))
                (all-the-icons-icon-family icon)
                (get-text-property 0 'face icon)
                (get-text-property 0 'display icon)))"##,
            true,
            expect![[
        r#"OK (t t t "" (61595) "Symbols Nerd Font" (:family "Symbols Nerd Font" :height 1.2 :inherit font-lock-constant-face) (raise -0.24))"#
    ]],
        ),
    ]);
}

#[test]
fn registry_all_the_icons_nerd_fonts_autoload_batch() {
    assert_all_the_icons_nerd_fonts_autoload_batch(&[
        (
            "autoloaded_commands_run_a_complete_prefer_render_and_restore_workflow",
            r##"(progn
         (require 'all-the-icons)
         (let* ((history
                      (seq-find
                       (lambda (entry)
                         (and
                          (stringp (car entry))
                          (string-suffix-p
                           "all-the-icons-nerd-fonts-autoloads.el"
                           (car entry))))
                       load-history))
                (autoload-events
                 (seq-filter
                  (lambda (event)
                    (memq
                     (car-safe event)
                     '(defun provide)))
                  (cdr history)))
                (was-autoloaded
                 (and
                  (autoloadp
                   (symbol-function
                    'all-the-icons-nerd-fonts-prefer))
                  (autoloadp
                   (symbol-function
                    'all-the-icons-nerd-fonts-unprefer))))
                (describe
                 (lambda (icon)
                   (list
                    (substring-no-properties icon)
                    (string-to-list
                     (substring-no-properties icon))
                    (all-the-icons-icon-family icon)
                    (get-text-property 0 'face icon))))
                (before
                 (funcall
                  describe
                  (all-the-icons-faicon
                   "github" :face 'success)))
                preferred
                restored)
           (unwind-protect
               (progn
                 (all-the-icons-nerd-fonts-prefer '())
                 (setq
                  preferred
                  (funcall
                   describe
                   (all-the-icons-faicon
                    "github" :face 'success)))
                 (all-the-icons-nerd-fonts-unprefer)
                 (setq
                  restored
                  (funcall
                   describe
                   (all-the-icons-faicon
                    "github" :face 'success)))
                 (list
                  was-autoloaded
                  (featurep 'all-the-icons-nerd-fonts)
                  before
                  preferred
                  restored
                  (equal before restored)
                  all-the-icons-nerd-fonts--advice-enabled
                  autoload-events))
             (when
                 (featurep 'all-the-icons-nerd-fonts)
               (all-the-icons-nerd-fonts-unprefer)))))"##,
            true,
            expect![[
        r#"OK (t t ("" (61595) "FontAwesome" (:family "FontAwesome" :height 1.2 :inherit success)) ("" (60036) "Symbols Nerd Font" (:family "Symbols Nerd Font" :height 1.2 :inherit success)) ("" (61595) "FontAwesome" (:family "FontAwesome" :height 1.2 :inherit success)) t nil ((defun . all-the-icons-nerd-fonts-prefer) (defun . all-the-icons-nerd-fonts-unprefer) (provide . all-the-icons-nerd-fonts-autoloads)))"#
    ]],
        ),
    ]);
}

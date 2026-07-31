use expect_test::expect;

use super::{ParityBatchCase, assert_ah_batch};

fn loading_a_real_custom_theme_observes_hook_state_and_applies_its_setting() -> ParityBatchCase {
    ParityBatchCase::new(
        "loading_a_real_custom_theme_observes_hook_state_and_applies_its_setting",
        r##"
(let* ((root
        (file-name-as-directory
         (expand-file-name
          "ah-theme-workflow"
          (getenv "NEOMACS_TEST_SANDBOX_ROOT"))))
       (theme-file
        (expand-file-name "ah-practical-theme-theme.el" root))
       (custom-theme-load-path (cons root custom-theme-load-path))
       (events nil)
       (ah-before-enable-theme-hook
        (list
         (lambda ()
           (push
            (list
             'before
             (copy-sequence custom-enabled-themes)
             fill-column)
            events))))
       (ah-after-enable-theme-hook
        (list
         (lambda ()
           (push
            (list
             'after
             (copy-sequence custom-enabled-themes)
             fill-column)
            events))))
       snapshot)
  (unwind-protect
      (progn
        (make-directory root t)
        (write-region
         (concat
          "(deftheme ah-practical-theme)\n"
          "(custom-theme-set-variables\n"
          " 'ah-practical-theme\n"
          " '(fill-column 91))\n"
          "(provide-theme 'ah-practical-theme)\n")
         nil theme-file nil 'silent)
        (ah-mode 1)
        (load-theme 'ah-practical-theme t)
        (setq
         snapshot
         (list
          (and (memq 'ah-practical-theme custom-enabled-themes) t)
          fill-column
          (nreverse events)
          ah-mode))
        snapshot)
    (when (custom-theme-p 'ah-practical-theme)
      (disable-theme 'ah-practical-theme))
    (ah-mode -1)
    (when (file-directory-p root)
      (delete-directory root t))))
"##,
        true,
        expect!["OK (t 91 ((before (ah-practical-theme) 91) (after (ah-practical-theme) 91)) t)"],
    )
}

#[test]
fn theme_public_surface_batch() {
    let cases: Vec<ParityBatchCase> = vec![
        loading_a_real_custom_theme_observes_hook_state_and_applies_its_setting(),
    ];
    assert_ah_batch(&cases);
}

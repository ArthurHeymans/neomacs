use expect_test::expect;

use super::assert_abridge_diff_parity;

#[test]
fn abridge_diff_public_surface_defaults_metadata_and_commands_match_the_pin() {
    let elisp_form = r##"(list
               (featurep 'abridge-diff)
               (mapcar
                #'fboundp
                '(abridge-diff-merge-exclude
                  abridge-diff-compute-hidden
                  abridge-diff-make-invisible
                  abridge-diff-abridge
                  abridge-diff-enable-hiding
                  abridge-diff-disable-hiding
                  abridge-diff-toggle-hiding
                  abridge-diff-enable
                  abridge-diff-disable
                  abridge-diff-mode))
               (mapcar
                #'commandp
                '(abridge-diff-merge-exclude
                  abridge-diff-compute-hidden
                  abridge-diff-make-invisible
                  abridge-diff-abridge
                  abridge-diff-enable-hiding
                  abridge-diff-disable-hiding
                  abridge-diff-toggle-hiding
                  abridge-diff-enable
                  abridge-diff-disable
                  abridge-diff-mode))
               (mapcar
                #'symbol-value
                '(abridge-diff-word-buffer
                  abridge-diff-invisible-min
                  abridge-diff-no-change-line-words
                  abridge-diff-first-words-preserve
                  abridge-diff-exclude-files-matching
                  abridge-diff-hiding
                  abridge-diff-mode))
               (mapcar
                (lambda (variable)
                  (list
                   variable
                   (get variable 'custom-group)
                   (get variable 'custom-type)))
                '(abridge-diff-word-buffer
                  abridge-diff-invisible-min
                  abridge-diff-no-change-line-words
                  abridge-diff-first-words-preserve
                  abridge-diff-exclude-files-matching))
               (local-variable-if-set-p 'abridge-diff-hiding)
               (assq 'abridge-diff-mode minor-mode-alist))"##;
    let expect = expect![[
        r#"OK (t (t t t t t t t t t t) (nil nil nil nil t t t nil nil t) (3 5 12 4 nil nil nil) ((abridge-diff-word-buffer nil integer) (abridge-diff-invisible-min nil integer) (abridge-diff-no-change-line-words nil integer) (abridge-diff-first-words-preserve nil integer) (abridge-diff-exclude-files-matching nil (repeat regexp))) t (abridge-diff-mode " abridge-diff"))"#
    ]];

    assert_abridge_diff_parity(elisp_form, expect);
}

#[test]
fn abridge_diff_hiding_enable_disable_is_buffer_local_and_updates_invisibility_spec() {
    let elisp_form = r##"(let ((default-value
                    (default-value 'abridge-diff-hiding)))
               (with-temp-buffer
                 (let ((buffer-invisibility-spec '(base)))
                   (list
                    (copy-tree
                     (abridge-diff-enable-hiding))
                    abridge-diff-hiding
                    (local-variable-p 'abridge-diff-hiding)
                    (copy-tree buffer-invisibility-spec)
                    (copy-tree
                     (abridge-diff-enable-hiding))
                    (copy-tree buffer-invisibility-spec)
                    (copy-tree
                     (abridge-diff-disable-hiding))
                    abridge-diff-hiding
                    (copy-tree buffer-invisibility-spec)
                    (default-value
                     'abridge-diff-hiding)
                    default-value))))"##;
    let expect = expect![
        "OK (((abridge-diff-invisible . t) base) t t ((abridge-diff-invisible . t) base) ((abridge-diff-invisible . t) (abridge-diff-invisible . t) base) ((abridge-diff-invisible . t) (abridge-diff-invisible . t) base) (base) nil (base) nil nil)"
    ];

    assert_abridge_diff_parity(elisp_form, expect);
}

#[test]
fn abridge_diff_toggle_hiding_reports_plain_on_and_off_messages_without_magit() {
    let elisp_form = r##"(let (events)
               (when (fboundp 'magit)
                 (fmakunbound 'magit))
               (with-temp-buffer
                 (cl-letf
                     (((symbol-function 'message)
                       (lambda (text &rest arguments)
                         (let ((rendered
                                (apply #'format text arguments)))
                           (push rendered events)
                           rendered))))
                   (list
                    (abridge-diff-toggle-hiding)
                    abridge-diff-hiding
                    (abridge-diff-toggle-hiding)
                    abridge-diff-hiding
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK ("Diff Abridging On" t "Diff Abridging Off" nil ("Diff Abridging On" "Diff Abridging Off"))"#
    ]];

    assert_abridge_diff_parity(elisp_form, expect);
}

#[test]
fn abridge_diff_toggle_hiding_adds_the_exact_magit_refinement_warning() {
    let elisp_form = r##"(let (events)
               (setq magit-diff-refine-hunk nil)
               (cl-letf
                   (((symbol-function 'magit)
                     (lambda () 'magit))
                    ((symbol-function 'message)
                     (lambda (text &rest arguments)
                       (let ((rendered
                              (apply #'format text arguments)))
                         (push rendered events)
                         rendered))))
                 (with-temp-buffer
                   (list
                    (abridge-diff-toggle-hiding)
                    (progn
                      (setq magit-diff-refine-hunk 'all)
                      (abridge-diff-toggle-hiding))
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK ("Diff Abridging On [WARNING: Hunk Refining Disabled!]" "Diff Abridging Off" ("Diff Abridging On [WARNING: Hunk Refining Disabled!]" "Diff Abridging Off"))"#
    ]];

    assert_abridge_diff_parity(elisp_form, expect);
}

#[test]
fn abridge_diff_enable_disable_installs_and_removes_the_real_smerge_advice() {
    let elisp_form = r##"(progn
               (when (fboundp 'magit)
                 (fmakunbound 'magit))
               (unwind-protect
                   (list
                    (not
                     (null
                      (advice-member-p
                       #'abridge-diff-abridge
                       #'smerge-refine-regions)))
                    (abridge-diff-enable)
                    (not
                     (null
                      (advice-member-p
                       #'abridge-diff-abridge
                       #'smerge-refine-regions)))
                    (abridge-diff-disable)
                    (not
                     (null
                      (advice-member-p
                       #'abridge-diff-abridge
                       #'smerge-refine-regions))))
                 (advice-remove
                  #'smerge-refine-regions
                  #'abridge-diff-abridge)))"##;
    let expect = expect!["OK (nil nil t nil nil)"];

    assert_abridge_diff_parity(elisp_form, expect);
}

#[test]
fn abridge_diff_optional_magit_integration_adds_and_removes_hooks_and_suffixes_exactly() {
    let elisp_form = r##"(let ((magit-diff-mode-hook '(existing-diff))
                    (magit-status-mode-hook '(existing-status))
                    events)
               (provide 'magit-diff)
               (cl-letf
                   (((symbol-function 'magit)
                     (lambda () 'magit))
                    ((symbol-function 'advice-add)
                     (lambda (&rest arguments)
                       (push (cons 'advice-add arguments) events)
                       'advised))
                    ((symbol-function 'advice-remove)
                     (lambda (&rest arguments)
                       (push
                        (cons 'advice-remove arguments)
                        events)
                       'unadvised))
                    ((symbol-function 'transient-append-suffix)
                     (lambda (&rest arguments)
                       (push (cons 'append arguments) events)
                       'appended))
                    ((symbol-function 'transient-remove-suffix)
                     (lambda (&rest arguments)
                       (push (cons 'remove arguments) events)
                       'removed)))
                 (list
                  (abridge-diff-enable)
                  (copy-tree magit-diff-mode-hook)
                  (copy-tree magit-status-mode-hook)
                  (abridge-diff-disable)
                  (copy-tree magit-diff-mode-hook)
                  (copy-tree magit-status-mode-hook)
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK (appended (existing-diff) (existing-status) removed (existing-diff) (existing-status) ((advice-add smerge-refine-regions :after abridge-diff-abridge) (append magit-diff-refresh magit-diff-toggle-refine-hunk ("a" "abridge refined diffs" abridge-diff-toggle-hiding)) (advice-remove smerge-refine-regions abridge-diff-abridge) (remove magit-diff-refresh abridge-diff-toggle-hiding)))"#
    ]];

    assert_abridge_diff_parity(elisp_form, expect);
}

#[test]
fn abridge_diff_global_mode_lifecycle_delegates_once_per_state_transition() {
    let elisp_form = r##"(let ((abridge-diff-mode nil)
                    events)
               (cl-letf
                   (((symbol-function 'abridge-diff-enable)
                     (lambda ()
                       (push 'enable events)
                       'enabled))
                    ((symbol-function 'abridge-diff-disable)
                     (lambda ()
                       (push 'disable events)
                       'disabled)))
                 (list
                  (abridge-diff-mode 1)
                  abridge-diff-mode
                  (abridge-diff-mode 1)
                  abridge-diff-mode
                  (abridge-diff-mode -1)
                  abridge-diff-mode
                  (abridge-diff-mode)
                  abridge-diff-mode
                  (nreverse events))))"##;
    let expect = expect!["OK (t t t t nil nil t t (enable enable disable enable))"];

    assert_abridge_diff_parity(elisp_form, expect);
}

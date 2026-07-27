use expect_test::expect;

use super::assert_all_the_icons_completion_parity;

#[test]
fn global_mode_enable_disable_cycle_tracks_variable_and_real_advice_membership() {
    let elisp_form = r##"
(unwind-protect
    (progn
      (all-the-icons-completion-mode -1)
      (let ((initial
             (list
              all-the-icons-completion-mode
              (and
               (advice-member-p
                #'all-the-icons-completion-completion-metadata-get
                #'completion-metadata-get)
               t))))
        (all-the-icons-completion-mode 1)
        (let ((enabled
               (list
                all-the-icons-completion-mode
                (and
                 (advice-member-p
                  #'all-the-icons-completion-completion-metadata-get
                  #'completion-metadata-get)
                 t))))
          (all-the-icons-completion-mode -1)
          (list
           initial
           enabled
           (list
            all-the-icons-completion-mode
            (advice-member-p
             #'all-the-icons-completion-completion-metadata-get
             #'completion-metadata-get))))))
  (all-the-icons-completion-mode -1))
"##;
    let expect = expect!["OK ((nil nil) (t t) (nil nil))"];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn repeated_explicit_mode_requests_are_idempotent_and_do_not_stack_advice() {
    let elisp_form = r##"
(let ((matching-advices 0)
      (all-advices 0))
  (unwind-protect
      (progn
        (all-the-icons-completion-mode 1)
        (all-the-icons-completion-mode 1)
        (all-the-icons-completion-mode 1)
        (advice-mapc
         (lambda (function _properties)
           (setq all-advices (1+ all-advices))
           (when
               (eq
                function
                #'all-the-icons-completion-completion-metadata-get)
             (setq matching-advices
                   (1+ matching-advices))))
         #'completion-metadata-get)
        (list
         all-the-icons-completion-mode
         matching-advices
         all-advices
         (completion-metadata-get
          '(metadata (category . file))
          'category)
         (and
          (advice-member-p
           #'all-the-icons-completion-completion-metadata-get
           #'completion-metadata-get)
          t)))
    (all-the-icons-completion-mode -1)))
"##;
    let expect = expect!["OK (t 1 1 file t)"];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn global_mode_affects_completion_metadata_equally_from_multiple_buffers() {
    let elisp_form = r##"
(let ((first (generate-new-buffer " all-icons-mode-first"))
      (second (generate-new-buffer " all-icons-mode-second"))
      results)
  (unwind-protect
      (progn
        (all-the-icons-completion-mode 1)
        (dolist (buffer (list first second))
          (with-current-buffer buffer
            (let* ((metadata
                    '(metadata (category . file)))
                   (affix
                    (completion-metadata-get
                     metadata
                     'affixation-function)))
              (push
               (list
                (and affix t)
                (funcall affix '("README.org")))
               results))))
        (nreverse results))
    (all-the-icons-completion-mode -1)
    (mapc
     (lambda (buffer)
       (when (buffer-live-p buffer)
         (kill-buffer buffer)))
     (list first second))))
"##;
    let expect = expect![[
        r#"OK ((t (("README.org" #(" " 0 1 (rear-nonsticky t display #2=(raise 0.0) font-lock-face #1=(:family "github-octicons" :height 1.2 :inherit all-the-icons-lcyan) face #1#)) ""))) (t (("README.org" #(" " 0 1 (rear-nonsticky t display #2# font-lock-face #1# face #1#)) ""))))"#
    ]];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn marginalia_setup_mirrors_marginalia_state_across_on_off_on_transitions() {
    let elisp_form = r##"
(setq marginalia-mode nil)
(let (states)
  (unwind-protect
      (progn
        (dolist (state '(t nil t))
          (setq marginalia-mode state)
          (all-the-icons-completion-marginalia-setup)
          (push
           (list
            state
            all-the-icons-completion-mode
            (and
             (advice-member-p
              #'all-the-icons-completion-completion-metadata-get
              #'completion-metadata-get)
             t))
           states))
        (nreverse states))
    (all-the-icons-completion-mode -1)))
"##;
    let expect = expect!["OK ((t t t) (nil nil nil) (t t t))"];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn disabling_mode_removes_only_package_advice_and_preserves_unrelated_advice() {
    let elisp_form = r##"
(let ((audit-calls nil))
  (cl-labels
      ((audit-advice
        (original metadata property)
        (push property audit-calls)
        (funcall original metadata property)))
    (unwind-protect
        (progn
          (advice-add
           #'completion-metadata-get
           :around
           #'audit-advice)
          (all-the-icons-completion-mode 1)
          (let ((both
                 (list
                  (and
                   (advice-member-p
                    #'audit-advice
                    #'completion-metadata-get)
                   t)
                  (and
                   (advice-member-p
                    #'all-the-icons-completion-completion-metadata-get
                    #'completion-metadata-get)
                   t))))
            (completion-metadata-get
             '(metadata (category . file))
             'category)
            (all-the-icons-completion-mode -1)
            (let ((after
                   (list
                    (and
                     (advice-member-p
                      #'audit-advice
                      #'completion-metadata-get)
                     t)
                    (advice-member-p
                     #'all-the-icons-completion-completion-metadata-get
                     #'completion-metadata-get))))
              (completion-metadata-get
               '(metadata (category . file))
               'category)
              (list
               both
               after
               (nreverse audit-calls)))))
      (all-the-icons-completion-mode -1)
      (advice-remove
       #'completion-metadata-get
       #'audit-advice))))
"##;
    let expect = expect!["OK ((t t) (t nil) (category category))"];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn mode_toggle_return_values_follow_define_minor_mode_global_semantics() {
    let elisp_form = r##"
(unwind-protect
    (list
     (all-the-icons-completion-mode -1)
     all-the-icons-completion-mode
     (all-the-icons-completion-mode)
     all-the-icons-completion-mode
     (all-the-icons-completion-mode)
     all-the-icons-completion-mode
     (all-the-icons-completion-mode 'toggle)
     all-the-icons-completion-mode)
  (all-the-icons-completion-mode -1))
"##;
    let expect = expect!["OK (nil nil t t t t nil nil)"];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

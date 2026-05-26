use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_repeated_cycle_preserves_visibility_and_text_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (org-mode)
    (insert "* A\nbody A\n** B\nbody B\n*** C\nbody C\n* D\nbody D\n")
    (goto-char (point-min))
    (let ((snapshot
           (lambda ()
             (list
              (buffer-substring-no-properties (point-min) (point-max))
              (mapcar
               (lambda (needle)
                 (invisible-p
                  (save-excursion
                    (goto-char (point-min))
                    (search-forward needle)
                    (point))))
               '("body A" "B" "body B" "C" "body C" "D" "body D")))))
          states)
      (dotimes (_ 6)
        (org-cycle)
        (push (funcall snapshot) states))
      (org-fold-show-all)
      (push (funcall snapshot) states)
      (nreverse states))))"#,
    );
}

#[test]
fn org_fold_subtree_show_sublevels_recovery_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (org-mode)
    (insert "* A\nbody A\n** B\nbody B\n*** C\nbody C\n* D\nbody D\n")
    (goto-char (point-min))
    (search-forward "B")
    (beginning-of-line)
    (org-fold-hide-subtree)
    (let ((hidden-b
           (mapcar
            (lambda (needle)
              (invisible-p
               (save-excursion
                 (goto-char (point-min))
                 (search-forward needle)
                 (point))))
            '("body B" "C" "body C" "D"))))
      (org-fold-show-subtree)
      (let ((shown-b
             (mapcar
              (lambda (needle)
                (invisible-p
                 (save-excursion
                   (goto-char (point-min))
                   (search-forward needle)
                   (point))))
              '("body B" "C" "body C" "D"))))
        (goto-char (point-min))
        (org-fold-hide-sublevels 2)
        (let ((sublevels
               (mapcar
                (lambda (needle)
                  (invisible-p
                   (save-excursion
                     (goto-char (point-min))
                     (search-forward needle)
                     (point))))
                '("A" "body A" "B" "body B" "C" "body C" "D" "body D"))))
          (list hidden-b
                shown-b
                sublevels
                (buffer-substring-no-properties (point-min) (point-max))))))))"#,
    );
}

#[test]
fn org_font_lock_heading_faces_level_four_plus_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* L1\n** L2\n*** L3\n**** L4\n***** L5\n****** L6\n")
    (font-lock-ensure (point-min) (point-max))
    (goto-char (point-min))
    (let (out)
      (while (re-search-forward "^\\*+ \\(L[0-9]\\)" nil t)
        (push (list (substring-no-properties (match-string 1))
                    (get-text-property (match-beginning 1) 'face)
                    (get-text-property (line-beginning-position) 'face))
              out))
      (nreverse out))))"#,
    );
}

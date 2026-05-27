use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_num_skip_property_comment_tags_update_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-num)
  (with-temp-buffer
    (let ((org-num-max-level 3)
          (org-num-skip-commented t)
          (org-num-skip-tags '("noexport" "skip"))
          (org-num-skip-unnumbered t)
          (org-num-format-function
           (lambda (numbers)
             (propertize (format "[%s] " (mapconcat #'number-to-string numbers "."))
                         'face 'bold))))
      (org-mode)
      (insert "* Alpha\n")
      (insert "** Beta\n")
      (insert "*** Leaf\n")
      (insert "** COMMENT Hidden\n")
      (insert "*** Hidden child\n")
      (insert "** Tagged :skip:\n")
      (insert "*** Tagged child\n")
      (insert "** Property\n:PROPERTIES:\n:UNNUMBERED: t\n:END:\n")
      (insert "*** Property child\n")
      (insert "* Omega\n")
      (org-num-mode 1)
      (let ((snapshot
             (lambda ()
               (let (out)
                 (dolist (ov (sort (copy-sequence org-num--overlays)
                                   (lambda (a b)
                                     (< (overlay-start a) (overlay-start b)))))
                   (push (list (buffer-substring-no-properties
                                (overlay-start ov)
                                (line-end-position))
                               (overlay-get ov 'level)
                               (overlay-get ov 'skip)
                               (substring-no-properties
                                (or (overlay-get ov 'after-string) ""))
                               (get-text-property
                                0 'face
                                (or (overlay-get ov 'after-string) "")))
                         out))
                 (nreverse out)))))
        (let ((before (funcall snapshot)))
          (goto-char (point-min))
          (search-forward "Beta")
          (end-of-line)
          (insert "\n*** Inserted")
          (org-num--verify (line-beginning-position) (point) 0)
          (goto-char (point-min))
          (search-forward "Tagged :skip:")
          (org-toggle-tag "skip" 'off)
          (org-num--verify (line-beginning-position) (line-end-position) 0)
          (list before
                (funcall snapshot)
                org-num--invalid-flag
                (buffer-substring-no-properties
                 (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_num_odd_levels_and_footnotes_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-num)
  (with-temp-buffer
    (let ((org-odd-levels-only t)
          (org-num-max-level 3)
          (org-num-skip-footnotes t)
          (org-num-format-function #'org-num-default-format))
      (org-mode)
      (insert "* Top\n")
      (insert "*** Odd child\n")
      (insert "***** Odd grandchild\n")
      (insert "* Footnotes\n")
      (insert "[fn:1] Footnote text.\n")
      (insert "* Tail\n")
      (org-num-mode 1)
      (let (out)
        (dolist (ov (sort (copy-sequence org-num--overlays)
                          (lambda (a b)
                            (< (overlay-start a) (overlay-start b)))))
          (push (list (buffer-substring-no-properties
                       (overlay-start ov)
                       (line-end-position))
                      (overlay-get ov 'level)
                      (overlay-get ov 'skip)
                      (substring-no-properties
                       (or (overlay-get ov 'after-string) "")))
                out))
        (nreverse out)))))"##,
    );
}

#[test]
fn org_num_mode_toggle_clear_reenable_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-num)
  (with-temp-buffer
    (org-mode)
    (insert "* A\n** B\n* C\n")
    (let ((org-num-format-function
           (lambda (numbers) (format "%s|" (mapconcat #'number-to-string numbers "-")))))
      (org-num-mode 1)
      (let ((first (mapcar (lambda (ov)
                             (list (overlay-start ov)
                                   (overlay-end ov)
                                   (substring-no-properties
                                    (or (overlay-get ov 'after-string) ""))))
                           org-num--overlays)))
        (org-num-mode -1)
        (let ((after-disable (list org-num--overlays
                                   (overlays-in (point-min) (point-max)))))
          (goto-char (point-max))
          (insert "** D\n")
          (org-num-mode 1)
          (list first
                after-disable
                (mapcar (lambda (ov)
                          (list (buffer-substring-no-properties
                                 (overlay-start ov)
                                 (line-end-position))
                                (substring-no-properties
                                 (or (overlay-get ov 'after-string) ""))))
                        org-num--overlays)
                (buffer-substring-no-properties
                 (point-min) (point-max))))))))"##,
    );
}

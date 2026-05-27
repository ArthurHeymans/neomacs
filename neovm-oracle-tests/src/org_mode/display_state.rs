use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_num_overlay_update_after_heading_edit_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-num)
  (with-temp-buffer
    (org-mode)
    (let ((org-num-skip-tags '("noexport"))
          (org-num-max-level 4))
      (insert "* Alpha\n** Beta\n*** COMMENT Skip\n*** Gamma :noexport:\n**** Delta\n* Epsilon\n")
      (org-num-mode 1)
      (let ((snapshot
             (lambda ()
               (let (out)
                 (goto-char (point-min))
                 (while (re-search-forward "^\\*+ \\(.*\\)" nil t)
                   (let* ((pos (line-beginning-position))
                          (ovs (overlays-at pos))
                          (nums (delq nil
                                      (mapcar
                                       (lambda (ov)
                                         (when (overlay-get ov 'org-num)
                                           (list (overlay-get ov 'org-num)
                                                 (substring-no-properties
                                                  (or (overlay-get ov 'before-string)
                                                      "")))))
                                       ovs))))
                     (push (list (match-string-no-properties 1) nums) out)))
                 (nreverse out)))))
        (let ((before (funcall snapshot)))
          (goto-char (point-min))
          (search-forward "Beta")
          (end-of-line)
          (insert "\n*** Inserted\n")
          (org-num--verify (point-min) (point-max) 0)
          (list before
                (funcall snapshot)
                (buffer-substring-no-properties
                 (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_indent_mode_prefix_after_deep_edit_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-indent)
  (with-temp-buffer
    (let ((org-indent-indentation-per-level 2)
          (org-indent-mode-turns-on-hiding-stars t))
      (org-mode)
      (insert "* A\nbody A\n** B\nbody B\n*** C\nbody C\n**** D\nbody D\n")
      (org-indent-mode 1)
      (font-lock-ensure (point-min) (point-max))
      (goto-char (point-min))
      (search-forward "body C")
      (end-of-line)
      (insert "\nmore C")
      (font-lock-ensure (point-min) (point-max))
      (let (out)
        (goto-char (point-min))
        (while (not (eobp))
          (let* ((lp (get-text-property (line-beginning-position) 'line-prefix))
                 (wp (get-text-property (line-beginning-position) 'wrap-prefix)))
            (push (list (buffer-substring-no-properties
                         (line-beginning-position) (line-end-position))
                        (and (stringp lp)
                             (list (length lp)
                                   (get-text-property 0 'face lp)))
                        (and (stringp wp)
                             (list (length wp)
                                   (get-text-property 0 'face wp))))
                  out))
          (forward-line 1))
        (nreverse out)))))"##,
    );
}

#[test]
fn org_font_lock_deep_headline_markup_faces_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "#+TODO: TODO WAIT | DONE\n")
    (insert "* TODO [#A] L1 :work:\n")
    (insert "** WAIT L2 with [[https://example.org][link]]\n")
    (insert "*** DONE L3 with =code= and ~verbatim~\n")
    (insert "**** TODO [#B] L4 with *bold* /italic/ :deep:work:\n")
    (font-lock-ensure (point-min) (point-max))
    (let (out)
      (dolist (needle '("TODO" "[#A]" "work" "WAIT" "link"
                        "DONE" "code" "verbatim" "[#B]" "bold"
                        "italic" "deep"))
        (goto-char (point-min))
        (search-forward needle)
        (push (list needle
                    (get-text-property (match-beginning 0) 'face)
                    (get-text-property (match-beginning 0) 'font-lock-fontified)
                    (get-text-property (match-beginning 0) 'invisible))
              out))
      (nreverse out))))"##,
    );
}

#[test]
fn org_indent_inlinetask_list_property_refresh_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-indent)
  (require 'org-inlinetask)
  (with-temp-buffer
    (let ((org-indent-indentation-per-level 3)
          (org-adapt-indentation 'headline-data)
          (org-indent-mode-turns-off-org-adapt-indentation nil)
          (org-indent-mode-turns-on-hiding-stars t)
          (org-inlinetask-min-level 4)
          (org-inlinetask-show-first-star t))
      (org-mode)
      (insert "* Project\n")
      (insert "SCHEDULED: <2026-05-27 Wed>\n")
      (insert ":PROPERTIES:\n:Owner: Ada\n:END:\n")
      (insert "Paragraph one\n")
      (insert "- item alpha\n")
      (insert "  continuation alpha\n")
      (insert "** Area\n")
      (insert "Area body\n")
      (insert "**** Inline task\n")
      (insert "Inline body\n")
      (insert "**** END\n")
      (org-indent-mode 1)
      (org-indent-indent-buffer)
      (let ((snapshot
             (lambda (label)
               (let (out)
                 (goto-char (point-min))
                 (while (not (eobp))
                   (let* ((pos (line-beginning-position))
                          (lp (get-text-property pos 'line-prefix))
                          (wp (get-text-property pos 'wrap-prefix)))
                     (push
                      (list (buffer-substring-no-properties
                             pos (line-end-position))
                            (and (stringp lp)
                                 (list (length lp)
                                       (substring-no-properties lp)
                                       (get-text-property 0 'face lp)))
                            (and (stringp wp)
                                 (list (length wp)
                                       (substring-no-properties wp)
                                       (get-text-property 0 'face wp))))
                      out))
                   (forward-line 1))
                 (list label (nreverse out))))))
        (let ((before (funcall snapshot 'before)))
          (goto-char (point-min))
          (search-forward "Area")
          (beginning-of-line)
          (insert "*** Inserted\nInserted body\n")
          (goto-char (point-min))
          (search-forward "item alpha")
          (end-of-line)
          (insert "\n  new continuation")
          (let* ((after-edit (funcall snapshot 'after-edit))
                 (copied (filter-buffer-substring
                          (point-min) (point-max) nil))
                 (copied-props
                  (list (text-property-any 0 (length copied)
                                           'line-prefix nil copied)
                        (text-property-any 0 (length copied)
                                           'wrap-prefix nil copied))))
            (org-indent-mode -1)
            (list before
                  after-edit
                  copied-props
                  (substring-no-properties copied)
                  (let (props)
                    (goto-char (point-min))
                    (while (not (eobp))
                      (push (list (buffer-substring-no-properties
                                   (line-beginning-position)
                                   (line-end-position))
                                  (get-text-property
                                   (line-beginning-position)
                                   'line-prefix)
                                  (get-text-property
                                   (line-beginning-position)
                                   'wrap-prefix))
                            props)
                      (forward-line 1))
                    (nreverse props)))))))))"##,
    );
}

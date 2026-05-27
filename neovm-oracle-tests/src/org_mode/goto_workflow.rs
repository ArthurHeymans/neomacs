use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_goto_local_search_keymap_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-goto)
  (with-temp-buffer
    (let ((org-goto-auto-isearch nil))
      (org-mode)
      (insert "* Alpha\n")
      (insert "Body mentions Deep Target but is not a heading.\n")
      (insert "** TODO Deep Target :tag:\n")
      (insert "*** Leaf 42\n")
      (insert "* Beta Deep Target\n")
      (org-goto--set-map)
      (goto-char (point-min))
      (let* ((forward-body-ignored
              (save-excursion
                (let ((isearch-forward t))
                  (org-goto--local-search-headings "Deep Target" nil t))
                (list (line-number-at-pos)
                      (thing-at-point 'line t))))
             (backward-heading
              (save-excursion
                (goto-char (point-max))
                (let ((isearch-forward nil))
                  (org-goto--local-search-headings "Deep Target" nil t))
                (list (line-number-at-pos)
                      (thing-at-point 'line t))))
             (missing
              (save-excursion
                (let ((isearch-forward t))
                  (org-goto--local-search-headings "missing" nil t))))
             (bindings
              (mapcar (lambda (key)
                        (list key (lookup-key org-goto-map key)))
                      (list "q" "n" "p" "f" "b" "u" "/" "\C-m"))))
        (list forward-body-ignored
              backward-heading
              missing
              bindings
              (keymapp org-goto-map))))))"##,
    );
}

#[test]
fn org_goto_location_indirect_return_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-goto)
  (with-temp-buffer
    (let ((org-goto-auto-isearch nil)
          (org-startup-folded 'showeverything))
      (org-mode)
      (insert "* Project\n")
      (insert "** Area\n")
      (insert "*** Target\nBody\n")
      (insert "* Other\n")
      (goto-char (point-min))
      (search-forward "Area")
      (beginning-of-line)
      (let ((origin (point))
            (org-goto-start-pos (point))
            selected)
        (org-goto--set-map)
        (cl-letf (((symbol-function 'recursive-edit)
                   (lambda ()
                     (goto-char (point-min))
                     (search-forward "*** Target")
                     (beginning-of-line)
                     (setq selected (list (buffer-name)
                                          (point)
                                          (thing-at-point 'line t)))
                     (org-goto-ret))))
                  ((symbol-function 'pop-to-buffer)
                   (lambda (buffer-or-name &optional _action _norecord)
                     (switch-to-buffer buffer-or-name)))
                  ((symbol-function 'org-fit-window-to-buffer)
                   (lambda (&rest _) nil)))
          (let ((result (org-goto-location nil "Help %s")))
            (list (list (- (car result) (point-min)) (cdr result))
                  selected
                  (= (point) origin)
                  (get-buffer "*org-goto*")
                  (get-buffer "*Org Help*")
                  (thing-at-point 'line t)))))))"##,
    );
}

#[test]
fn org_goto_outline_path_completion_command_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-goto)
  (with-temp-buffer
    (let ((org-goto-interface 'outline-path-completion)
          (org-goto-max-level 4)
          (org-mark-ring nil))
      (org-mode)
      (insert "* Project\n")
      (insert "** Area\n")
      (insert "*** Target\nBody\n")
      (insert "**** Too deep\n")
      (insert "* Other\n")
      (goto-char (point-min))
      (search-forward "Project")
      (let ((origin (line-number-at-pos))
            (target (save-excursion
                      (search-forward "*** Target")
                      (beginning-of-line)
                      (point)))
            captured-targets)
        (cl-letf (((symbol-function 'org-refile-get-location)
                   (lambda (&rest _)
                     (setq captured-targets org-refile-targets)
                     (list "Project/Area/Target" nil nil target)))
                  ((symbol-function 'org-refile-check-position)
                   (lambda (location)
                     (list 'checked (car location) (nth 3 location)))))
          (org-goto)
          (let ((after-path (list (line-number-at-pos)
                                  (thing-at-point 'line t)
                                  captured-targets
                                  (mapcar #'marker-position org-mark-ring))))
            (goto-char (point-min))
            (search-forward "Other")
            (cl-letf (((symbol-function 'org-goto-location)
                       (lambda (&rest _)
                         (cons target 'return))))
              (let ((before-alt (line-number-at-pos)))
                (org-goto t)
                (list origin
                      after-path
                      before-alt
                      (line-number-at-pos)
                      (thing-at-point 'line t)
                      (mapcar #'marker-position org-mark-ring))))))))"##,
    );
}

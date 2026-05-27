use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_keys_speed_babel_keymap_dispatch_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-keys)
  (require 'ob-core)
  (with-temp-buffer
    (let ((org-replace-disputed-keys t)
          (org-disputed-keys
           `((,(kbd "S-<left>") . ,(kbd "C-c <left>"))
             (,(kbd "S-<right>") . ,(kbd "C-c <right>"))))
          (org-use-speed-commands t))
      (org-mode)
      (insert "* Alpha\n")
      (insert "#+begin_src emacs-lisp\n(+ 1 2)\n#+end_src\n")
      (insert "** Beta\nBody\n")
      (let* ((local-map (make-sparse-keymap))
             (remap-map (make-sparse-keymap))
             (speed-head nil)
             (speed-body nil)
             (babel-head nil)
             (babel-body nil)
             (safe-forward nil)
             help)
        (org-defkey local-map (kbd "S-<left>") 'translated-left)
        (org-defkey local-map (kbd "S-<right>") 'translated-right)
        (org-defkey local-map (kbd "TAB") 'local-tab)
        (org-remap remap-map
                   'forward-word 'org-forward-element
                   'backward-word 'org-backward-element)
        (goto-char (point-min))
        (setq speed-head
              (mapcar (lambda (key)
                        (let ((handler
                               (org-speed-command-activate key)))
                          (list key
                                (cond ((symbolp handler) handler)
                                      ((consp handler) (car handler))
                                      ((functionp handler) 'function)
                                      (t handler)))))
                      '("n" "p" "?" "x")))
        (forward-char 2)
        (setq speed-body
              (mapcar #'org-speed-command-activate '("n" "?")))
        (goto-char (point-min))
        (search-forward "#+begin_src")
        (beginning-of-line)
        (setq babel-head
              (mapcar (lambda (key)
                        (let ((handler
                               (org-babel-speed-command-activate key)))
                          (list key
                                (cond ((symbolp handler) handler)
                                      ((consp handler) (car handler))
                                      ((functionp handler) 'function)
                                      (t handler)))))
                      '("n" "e" "v" "x" "z")))
        (forward-char 2)
        (setq babel-body
              (mapcar #'org-babel-speed-command-activate '("n" "e")))
        (goto-char (point-min))
        (setq safe-forward
              (condition-case err
                  (progn
                    (org-speed-move-safe 'org-next-visible-heading)
                    (list 'ok
                          (buffer-substring-no-properties
                           (line-beginning-position)
                           (line-end-position))))
                (error (cons (car err) (cdr err)))))
        (setq help
              (let ((org-speed-commands
                     '(("Group")
                       ("n" . org-next-visible-heading)
                       ("e" . (org-entry-put (point) "X" "Y"))
                       ("?" . org-speed-command-help))))
                (org-speed-command-help)
                (prog1
                    (with-current-buffer "*Help*"
                      (buffer-substring-no-properties
                       (point-min) (point-max)))
                  (when (get-buffer "*Help*")
                    (kill-buffer "*Help*")))))
        (list (mapcar (lambda (key)
                        (list key
                              (key-description (org-key key))))
                      (list (kbd "S-<left>")
                            (kbd "S-<right>")
                            (kbd "TAB")))
              (mapcar (lambda (key)
                        (list (key-description key)
                              (lookup-key local-map key)))
                      (list (kbd "C-c <left>")
                            (kbd "C-c <right>")
                            (kbd "TAB")
                            (kbd "S-<left>")))
              (list (lookup-key remap-map [remap forward-word])
                    (lookup-key remap-map [remap backward-word]))
              (mapcar (lambda (key)
                        (list key (lookup-key org-mode-map (kbd key))))
                      '("TAB" "C-c C-x" "C-c C-x C-b"
                        "C-c C-v n" "C-c C-v e" "C-c C-v v"))
              speed-head
              speed-body
              babel-head
              babel-body
              safe-forward
              help)))))"##,
    );
}

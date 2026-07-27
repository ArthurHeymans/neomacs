;;; pgo-train.el --- training workload for `fresh-build --profile release-pgo'  -*- lexical-binding: t; -*-

;; Drives the paths a PGO build should optimise for. Committed (rather than
;; left to whatever benchmark happens to be lying around) so the profile is
;; reproducible and reviewable: a PGO profile bakes in assumptions about what
;; is hot, and code on unprofiled paths gets pessimised, so what is trained on
;; is part of the build's semantics.
;;
;; Deliberately covers MORE than the editing benchmarks used to discover the
;; win. Training only on font-lock measured better on font-lock (-24%) but
;; risks biasing against everything else; byte-compilation and startup are
;; included so the common non-editing paths keep their profile too.

(defun nm-pgo--edit-pass (file mode-fn iters)
  "Fontify and edit around FILE the way interactive editing does."
  (let ((buf (find-file-noselect file)))
    (with-current-buffer buf
      (funcall mode-fn)
      (font-lock-set-defaults)
      (let* ((sz (buffer-size))
             (step (max 1 (/ sz (max 1 iters)))))
        (dotimes (i iters)
          (let ((pos (min (max (point-min) (* i step))
                          (max (point-min) (- (point-max) 2)))))
            (goto-char pos)
            (beginning-of-line)
            (let* ((win-start (point))
                   (win-end (save-excursion (forward-line 50) (point))))
              (font-lock-unfontify-region win-start win-end)
              (font-lock-fontify-region win-start win-end)
              (goto-char win-start)
              (insert "x")
              (let ((ins (point)))
                (font-lock-fontify-region (line-beginning-position)
                                          (line-end-position))
                (syntax-ppss ins)
                (delete-region (1- ins) ins))))))
      (set-buffer-modified-p nil)
      (kill-buffer buf))))

(defun nm-pgo--org-pass (iters)
  "Same shape as `nm-pgo--edit-pass' on a generated org buffer.
Org exercises a different frontier (Lisp-heavy fontification, text
properties, GC) than an elisp buffer does."
  (let ((buf (get-buffer-create "*pgo-org*")))
    (with-current-buffer buf
      (erase-buffer)
      (dotimes (n 80)
        (insert (format "* Heading %d :tag:\n" n))
        (insert "Text with *bold*, /italic/, =code= and a [[https://e.com][link]].\n")
        (insert "#+begin_src emacs-lisp\n(defun f (x) (* x x))\n#+end_src\n")
        (insert "| a | 1 |\n|---+---|\n| b | 2 |\n\n"))
      (when (fboundp 'org-mode) (org-mode))
      (font-lock-set-defaults)
      (let* ((sz (buffer-size))
             (step (max 1 (/ sz (max 1 iters)))))
        (dotimes (i iters)
          (let ((pos (min (max (point-min) (* i step))
                          (max (point-min) (- (point-max) 2)))))
            (goto-char pos)
            (beginning-of-line)
            (let* ((win-start (point))
                   (win-end (save-excursion (forward-line 50) (point))))
              (font-lock-unfontify-region win-start win-end)
              (font-lock-fontify-region win-start win-end)))))
      (kill-buffer buf))))

(defun nm-pgo--byte-compile-pass (files)
  "Byte-compile FILES into a scratch dir, then discard it.
Byte-compilation is the other workload users wait on, and it stresses
the reader, macro expansion and the compiler rather than redisplay."
  (let ((dir (make-temp-file "nm-pgo-bc" t)))
    (unwind-protect
        (dolist (f files)
          (when (file-readable-p f)
            (let ((copy (expand-file-name (file-name-nondirectory f) dir)))
              (copy-file f copy t)
              (byte-compile-file copy))))
      (delete-directory dir t))))

(let* ((root (or (getenv "NEOMACS_RUNTIME_ROOT") default-directory))
       (el (expand-file-name "lisp/emacs-lisp/cl-macs.el" root))
       (bc (mapcar (lambda (n) (expand-file-name (concat "lisp/emacs-lisp/" n) root))
                   '("seq.el" "map.el" "pcase.el" "rx.el"))))
  (when (file-readable-p el)
    (nm-pgo--edit-pass el #'emacs-lisp-mode 60))
  (ignore-errors (require 'org))
  (ignore-errors (nm-pgo--org-pass 40))
  (nm-pgo--byte-compile-pass bc)
  (message "pgo-train: done"))

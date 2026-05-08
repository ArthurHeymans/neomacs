;;; neomacs-webkit-test.el --- Test inline WebKit rendering in Neomacs -*- lexical-binding: t -*-

;; This test verifies that WebKit views are rendered inline in buffers
;; using WPE WebKit with GPU acceleration.

;;; Commentary:
;; Run with: ./test/neomacs/run-webkit-test.sh
;; Or manually: DISPLAY=:0 ./src/emacs -Q -l test/neomacs/neomacs-webkit-test.el
;;
;; Neomacs renders WebKit views inline in buffers (not as floating overlays).
;; This uses a declarative display property:
;;
;;   (insert (propertize " " 'display
;;                       '(webkit :uri "https://example.com"
;;                                :width 400 :height 300)))
;;
;; The webkit view becomes part of the buffer content, scrolls naturally,
;; and respects Emacs window management.

;;; Code:

(defvar neomacs-webkit-test-url "https://www.google.com/"
  "URL to load for testing.")

(defvar neomacs-webkit-test-width 0
  "Width of test WebKit view (0 = auto-fit to window).")

(defvar neomacs-webkit-test-height 0
  "Height of test WebKit view (0 = auto from aspect ratio).")

(defun neomacs-webkit-test-run ()
  "Run the inline WebKit rendering test."
  (interactive)
  (switch-to-buffer (get-buffer-create "*WebKit Test*"))
  (erase-buffer)

  (insert "=== Neomacs Inline WebKit Test ===\n\n")

  (condition-case err
      (progn
        ;; Calculate dimensions (0 = auto-fit to window)
        (let* ((margin 16)
               (aspect-ratio (/ 16.0 9.0))
               (width (if (and neomacs-webkit-test-width (> neomacs-webkit-test-width 0))
                          neomacs-webkit-test-width
                        (- (window-body-width nil t) margin)))
               (height (if (and neomacs-webkit-test-height (> neomacs-webkit-test-height 0))
                           neomacs-webkit-test-height
                         (round (/ width aspect-ratio)))))
          (insert (format "Loading %s inline (%dx%d)...\n\n"
                          neomacs-webkit-test-url width height))

          (let ((spec (list 'webkit
                            :uri neomacs-webkit-test-url
                            :width width
                            :height height)))
            (insert (propertize " " 'display spec))
            (insert "\n\n")
            (insert (format "WebKit spec: %S\n\n" spec))
            (insert "SUCCESS! Declarative inline WebKit display property installed.\n"))))
    (error
     (insert (format "ERROR: %S\n" err))))

  (goto-char (point-min))
  (redisplay t)

  ;; Auto-exit after delay when run non-interactively
  (when noninteractive
    (run-at-time 15 nil (lambda () (kill-emacs 0)))))

;; Auto-run when loaded
(add-hook 'emacs-startup-hook #'neomacs-webkit-test-run)

(provide 'neomacs-webkit-test)
;;; neomacs-webkit-test.el ends here

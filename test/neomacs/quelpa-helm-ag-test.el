;;; quelpa-helm-ag-test.el --- Test use-package + quelpa installing helm-ag from GitHub -*- lexical-binding: t -*-

;; Test that neomacs can bootstrap quelpa, quelpa-use-package, and use-package,
;; then use them to install helm-ag directly from its GitHub repository via
;; HTTPS.  This exercises the full pipeline: TLS handshake, git clone via
;; subprocess, package.el build, and use-package activation.
;;
;; Usage: ./target/release/neomacs -Q -l test/neomacs/quelpa-helm-ag-test.el
;;        or:  ./test/neomacs/run-quelpa-helm-ag-test.sh

;;; Code:

(require 'package)

(defvar quelpa-helm-ag-test--buffer-name "*Quelpa Helm-ag Test*"
  "Name of the test buffer.")

(defvar quelpa-helm-ag-test--failures 0)
(defvar quelpa-helm-ag-test--passes 0)

(defun quelpa-helm-ag-test--heading (title)
  (insert "\n")
  (let ((start (point)))
    (insert (format "=== %s ===\n" title))
    (put-text-property start (point) 'face '(:weight bold :foreground "gold" :height 1.2))))

(defun quelpa-helm-ag-test--ok (description)
  (setq quelpa-helm-ag-test--passes (1+ quelpa-helm-ag-test--passes))
  (let ((start (point)))
    (insert (format "  PASS: %s\n" description))
    (put-text-property start (point) 'face '(:foreground "lime green"))))

(defun quelpa-helm-ag-test--fail (description &optional detail)
  (setq quelpa-helm-ag-test--failures (1+ quelpa-helm-ag-test--failures))
  (let ((start (point)))
    (insert (format "  FAIL: %s\n" description))
    (put-text-property start (point) 'face '(:foreground "red" :weight bold)))
  (when detail
    (insert (format "        %s\n" detail))))

(defun quelpa-helm-ag-test--check-tls ()
  (quelpa-helm-ag-test--heading "1. TLS Availability")
  (if (gnutls-available-p)
      (quelpa-helm-ag-test--ok "gnutls-available-p => non-nil (TLS ready)")
    (quelpa-helm-ag-test--fail "gnutls-available-p => nil -- cannot proceed without TLS")
    (kill-emacs 1)))

(defun quelpa-helm-ag-test--configure-package ()
  (quelpa-helm-ag-test--heading "2. Configure package.el")
  (setq package-archives
        '(("gnu"    . "https://mirrors.tuna.tsinghua.edu.cn/elpa/gnu/")
          ("nongnu" . "https://mirrors.tuna.tsinghua.edu.cn/elpa/nongnu/")
          ("melpa"  . "https://mirrors.tuna.tsinghua.edu.cn/elpa/melpa/")))
  (setq package-check-signature nil)
  (setq package-archive-priorities
        '(("melpa" . 1) ("nongnu" . 5) ("gnu" . 10)))
  (package-initialize)
  (quelpa-helm-ag-test--ok "package-archives configured (Tsinghua mirror)")
  (quelpa-helm-ag-test--ok "package-initialize done"))

(defun quelpa-helm-ag-test--bootstrap-quelpa ()
  (quelpa-helm-ag-test--heading "3. Bootstrap quelpa")
  (setq quelpa-update-melpa-p nil)
  (condition-case err
      (progn
        (unless (package-installed-p 'quelpa)
          (with-temp-buffer
            (url-insert-file-contents
             "https://raw.githubusercontent.com/quelpa/quelpa/master/quelpa.el")
            (eval-buffer)
            (quelpa-self-upgrade)))
        (require 'quelpa)
        (quelpa-helm-ag-test--ok "quelpa bootstrapped and loaded"))
    (error
     (quelpa-helm-ag-test--fail "quelpa bootstrap failed"
                                (error-message-string err)))))

(defun quelpa-helm-ag-test--install-quelpa-use-package ()
  (quelpa-helm-ag-test--heading "4. Install quelpa-use-package")
  (condition-case err
      (progn
        (require 'use-package)
        (quelpa-helm-ag-test--ok "use-package loaded")
        (quelpa '(quelpa-use-package
                  :fetcher git
                  :url "https://github.com/quelpa/quelpa-use-package.git"))
        (require 'quelpa-use-package)
        (quelpa-helm-ag-test--ok "quelpa-use-package installed and loaded")
        (if (assoc :quelpa use-package-keywords)
            (quelpa-helm-ag-test--ok ":quelpa keyword registered with use-package")
          (quelpa-helm-ag-test--fail ":quelpa keyword not registered with use-package")))
    (error
     (quelpa-helm-ag-test--fail "quelpa-use-package install failed"
                                (error-message-string err)))))

(defun quelpa-helm-ag-test--install-helm-ag ()
  (quelpa-helm-ag-test--heading "5. Install helm-ag via use-package + quelpa")
  (condition-case err
      (progn
        (use-package helm-ag
          :quelpa (helm-ag :fetcher github :repo "emacsattic/helm-ag"))
        (quelpa-helm-ag-test--ok "use-package helm-ag with :quelpa completed"))
    (error
     (quelpa-helm-ag-test--fail "use-package helm-ag failed"
                                (error-message-string err)))))

(defun quelpa-helm-ag-test--verify-helm-ag ()
  (quelpa-helm-ag-test--heading "6. Verify helm-ag installation")
  (if (require 'helm-ag nil t)
      (quelpa-helm-ag-test--ok "(require 'helm-ag) succeeded")
    (quelpa-helm-ag-test--fail "(require 'helm-ag) failed"))
  (if (fboundp 'helm-ag)
      (quelpa-helm-ag-test--ok "helm-ag command is defined")
    (quelpa-helm-ag-test--fail "helm-ag command is not defined"))
  (if (package-installed-p 'helm-ag)
      (quelpa-helm-ag-test--ok "helm-ag is registered as installed package")
    (quelpa-helm-ag-test--fail "helm-ag not in package-installed-p"))
  (let ((feat (featurep 'helm-ag)))
    (if feat
        (quelpa-helm-ag-test--ok "helm-ag feature is provided")
      (quelpa-helm-ag-test--fail "helm-ag feature not yet provided (may need load)"))))

(defun quelpa-helm-ag-test-run ()
  (let ((buf (get-buffer-create quelpa-helm-ag-test--buffer-name)))
    (switch-to-buffer buf)
    (let ((inhibit-read-only t))
      (erase-buffer)
      (setq quelpa-helm-ag-test--failures 0)
      (setq quelpa-helm-ag-test--passes 0)

      (let ((start (point)))
        (insert "QUELPA + USE-PACKAGE + HELM-AG TEST\n")
        (put-text-property start (point)
                           'face '(:weight bold :height 1.8 :foreground "cyan")))
      (insert (format "Window system: %s\n" window-system))
      (insert (make-string 72 ?-) "\n")
      (insert "Bootstraps quelpa, installs quelpa-use-package,\n")
      (insert "then uses use-package + :quelpa to install helm-ag from GitHub.\n")

      (quelpa-helm-ag-test--check-tls)
      (quelpa-helm-ag-test--configure-package)
      (quelpa-helm-ag-test--bootstrap-quelpa)
      (quelpa-helm-ag-test--install-quelpa-use-package)
      (quelpa-helm-ag-test--install-helm-ag)
      (quelpa-helm-ag-test--verify-helm-ag)

      (insert "\n")
      (let ((start (point)))
        (insert (make-string 72 ?=) "\n")
        (put-text-property start (point) 'face '(:foreground "gold")))
      (let ((total (+ quelpa-helm-ag-test--passes quelpa-helm-ag-test--failures)))
        (insert (format "Results: %d/%d passed, %d failed\n"
                        quelpa-helm-ag-test--passes total
                        quelpa-helm-ag-test--failures))
        (if (zerop quelpa-helm-ag-test--failures)
            (message "ALL QUELPA HELM-AG TESTS PASSED (%d checks)" total)
          (message "QUELPA HELM-AG TESTS: %d FAILURES out of %d checks"
                   quelpa-helm-ag-test--failures total)))

      (goto-char (point-min))
      (setq buffer-read-only t)))

  (if (zerop quelpa-helm-ag-test--failures)
      (kill-emacs 0)
    (kill-emacs 1)))

(quelpa-helm-ag-test-run)

;;; quelpa-helm-ag-test.el ends here

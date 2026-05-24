;;; real-user-workflow-dependency.el --- Verify dependency resolution  -*- lexical-binding: t -*-

;; This file simulates a real user installing a package that has dependencies.
;; Emacs's package-install automatically resolves and installs any missing deps.

(require 'package)

(setq package-archives '(("melpa" . "https://melpa.org/packages/")))
(setq package-check-signature nil)

(package-initialize)
(package-refresh-contents)
(package-install 'flycheck)

;; flycheck should be usable after install
(unless (fboundp 'flycheck-mode)
  (error "flycheck-mode not available after package-install"))

(with-temp-buffer
  (flycheck-mode 1)
  (unless flycheck-mode
    (error "flycheck-mode did not enable in buffer"))
  (flycheck-mode -1)
  (when flycheck-mode
    (error "flycheck-mode did not disable in buffer")))

(message "USER-WORKFLOW-DEPENDENCY-OK")

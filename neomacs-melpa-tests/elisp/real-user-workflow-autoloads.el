;;; real-user-workflow-autoloads.el --- Verify autoloads work after package-install  -*- lexical-binding: t -*-

;; This file simulates a real user who:
;;   1. Installs which-key from MELPA
;;   2. Restarts Emacs (simulated by this fresh batch session)
;;   3. Calls package-initialize
;;   4. Expects autoloaded commands to be available WITHOUT (require 'which-key)

(require 'package)

(setq package-archives '(("melpa" . "https://melpa.org/packages/")))
(setq package-check-signature nil)

(package-initialize)
(package-refresh-contents)
(package-install 'which-key)

;; which-key-mode is autoloaded — it should be fboundp without (require 'which-key)
(unless (fboundp 'which-key-mode)
  (error "which-key-mode not autoloaded after package-install"))

(which-key-mode 1)
(unless which-key-mode
  (error "which-key-mode did not enable via autoload"))

(which-key-mode -1)
(when which-key-mode
  (error "which-key-mode did not disable"))

(message "USER-WORKFLOW-AUTOLOADS-OK")

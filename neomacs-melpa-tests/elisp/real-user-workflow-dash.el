;;; real-user-workflow-dash.el --- Real user workflow: install & use dash from MELPA  -*- lexical-binding: t -*-

;; This file simulates exactly what a real Emacs user does:
;;   1. Add MELPA to package-archives
;;   2. package-refresh-contents
;;   3. package-install 'dash
;;   4. (require 'dash)
;;   5. Use dash functions

(require 'package)

(setq package-archives '(("melpa" . "https://melpa.org/packages/")))
(setq package-check-signature nil)

(package-initialize)
(package-refresh-contents)
(package-install 'dash)

;; After installation, package-initialize should make dash available.
(require 'dash)

(let ((result (-map (lambda (n) (* n 2)) '(1 2 3 4))))
  (unless (equal result '(2 4 6 8))
    (error "dash -map failed: got %S" result)))

(let ((result (-filter (lambda (n) (> n 2)) '(1 2 3 4))))
  (unless (equal result '(3 4))
    (error "dash -filter failed: got %S" result)))

(let ((result (-reduce '+ '(1 2 3 4))))
  (unless (equal result 10)
    (error "dash -reduce failed: got %S" result)))

(message "USER-WORKFLOW-DASH-OK")

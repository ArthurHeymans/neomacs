;; This probe runs in a new Neomacs process after one live package-install
;; session has installed the selected ecosystem matrix.

(dolist (package '(dash s hydra ivy flycheck projectile))
  (unless (package-installed-p package)
    (error "live MELPA package was not installed: %S" package)))

(dolist (autoload '(flycheck-mode projectile-mode))
  (unless (fboundp autoload)
    (error "package autoload was unavailable after restart: %S" autoload)))

(require 'dash)
(require 's)
(require 'hydra)
(require 'ivy)

(unless (equal (-map (lambda (number) (1+ number)) '(1 2 3))
               '(2 3 4))
  (error "live dash package execution differed"))
(unless (string= (s-replace "world" "Neomacs" "hello world")
                 "hello Neomacs")
  (error "live s package execution differed"))

(defhydra neomacs-live-hydra ()
  "live"
  ("x" ignore))
(unless (fboundp 'neomacs-live-hydra/body)
  (error "live hydra package did not define its body command"))
(unless (fboundp 'ivy-read)
  (error "ivy-read was unavailable after require"))

(with-temp-buffer
  (flycheck-mode 1)
  (unless flycheck-mode
    (error "flycheck-mode did not enable after restart"))
  (flycheck-mode -1))

(princ "NEOMACS-MELPA-RESULT:(:live-packages t :dependencies t :autoloads t :macros t :minor-mode t :restart t)")

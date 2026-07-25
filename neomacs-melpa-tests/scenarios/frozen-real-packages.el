;; Exercise real, pinned MELPA package code after the installing editor has
;; exited and a fresh editor has initialized the same package home.

(dolist (package '(dash s hydra lv))
  (unless (package-installed-p package)
    (error "expected frozen MELPA package was not installed: %S" package)))

(unless (fboundp 'defhydra)
  (error "defhydra autoload was unavailable after restart"))

(require 'dash)
(require 's)
(require 'hydra)

(unless (equal (-map (lambda (number) (* number 2)) '(1 2 3 4))
               '(2 4 6 8))
  (error "dash execution differed after restart"))
(unless (string= (s-trim "  neomacs  ") "neomacs")
  (error "s execution differed after restart"))

(defhydra neomacs-melpa-hydra ()
  "frozen"
  ("x" ignore))
(unless (fboundp 'neomacs-melpa-hydra/body)
  (error "hydra macro did not define its body command"))

(dolist (package '(dash s hydra lv))
  (let* ((description (cadr (assq package package-alist)))
         (package-directory (and description
                                 (package-desc-dir description)))
         (bytecode (and package-directory
                        (expand-file-name
                         (concat (symbol-name package) ".elc")
                         package-directory))))
    (unless (and bytecode (file-exists-p bytecode))
      (error "package was not byte-compiled: %S (%S)"
             package bytecode))))

(princ "NEOMACS-MELPA-RESULT:(:real-packages t :dependency-resolution t :autoloads t :byte-compiled t :restart t)")

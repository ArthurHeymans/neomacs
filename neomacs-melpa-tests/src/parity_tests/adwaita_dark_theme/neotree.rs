use expect_test::expect;

use super::assert_adwaita_dark_theme_parity;

#[test]
fn adwaita_dark_theme_neotree_root_renderer_builds_real_button_text_properties_and_action() {
    let elisp_form = r##"(let (events rendered)
         (cl-letf (((symbol-function 'neo-path--file-short-name)
                    (lambda (node)
                      (push (list 'short-name node) events)
                      "project"))
                   ((symbol-function 'neo-buffer--newline-and-begin)
                    (lambda ()
                      (push 'newline events)
                      (insert "\n")
                      (beginning-of-line)))
                   ((symbol-function 'neotree-hidden-file-toggle)
                    (lambda ()
                      (push 'toggle-hidden events)
                      'toggled)))
           (with-temp-buffer
             (adwaita-dark-theme--neotree-insert-root
              "/workspace/project/")
             (let* ((button (next-button (point-min)))
                    (keymap (button-get button 'keymap))
                    (mouse-two (lookup-key keymap [mouse-2])))
               (funcall mouse-two)
               (list
                (buffer-substring-no-properties
                 (point-min)
                 (point-max))
                (point)
                (button-label button)
                (button-start button)
                (button-end button)
                (button-get button 'face)
                (button-get button 'follow-link)
                (button-get button 'neo-full-path)
                (button-get button 'help-echo)
                (commandp mouse-two)
                (help-function-arglist mouse-two t)
                (delete-dups
                 (mapcar
                  (lambda (position)
                    (copy-tree
                     (list
                      (get-text-property position 'face)
                      (get-text-property position 'display))))
                  (number-sequence
                   (button-start button)
                   (1- (button-end button)))))
                (nreverse events))))))"##;
    let expect = expect![[
        r#"OK (" 🖿 project \n" 13 "🖿 project " 2 12 (nil) t "/workspace/project" "mouse-1: Toggle hidden files\nmouse-3: Move root up one directory" t nil (((:inherit (neo-root-dir-face) :height 1.5) nil) (nil nil) (neo-root-dir-face nil) (nil ((space :align-to (- right 0 1))))) ((short-name "/workspace/project/") newline toggle-hidden))"#
    ]];
    assert_adwaita_dark_theme_parity(elisp_form, expect);
}

#[test]
fn adwaita_dark_theme_neotree_directory_renderer_covers_depth_expansion_hidden_face_and_action() {
    let elisp_form = r##"(let (events rendered)
         (cl-letf (((symbol-function 'neo-path--file-short-name)
                    (lambda (node)
                      (push (list 'short-name node) events)
                      (file-name-nondirectory
                       (directory-file-name node))))
                   ((symbol-function 'neo-filepath-hidden-p)
                    (lambda (node)
                      (push (list 'hidden-p node) events)
                      (string-prefix-p "." (file-name-nondirectory node))))
                   ((symbol-function 'neo-buffer--node-list-set)
                    (lambda (&rest arguments)
                      (push (cons 'node-list arguments) events)))
                   ((symbol-function 'neo-buffer--newline-and-begin)
                    (lambda ()
                      (push 'newline events)
                      (insert "\n")
                      (beginning-of-line)))
                   ((symbol-function 'neo-open-dir)
                    (lambda (&rest arguments)
                      (push (cons 'open-dir arguments) events)
                      'opened)))
           (setq
            rendered
            (mapcar
             (lambda (case)
               (pcase-let ((`(,node ,depth ,expanded) case))
                 (with-temp-buffer
                   (adwaita-dark-theme--neotree-insert-dir
                    node depth expanded)
                   (let* ((button (next-button (point-min)))
                          (keymap (button-get button 'keymap))
                          (mouse-two
                           (lookup-key keymap [mouse-2])))
                     (funcall mouse-two)
                     (list
                     case
                      (buffer-substring-no-properties
                       (point-min)
                       (point-max))
                      (point)
                      (button-label button)
                      (button-get button 'face)
                      (button-get button 'neo-full-path)
                      (button-get button 'follow-link)
                      (button-get button 'help-echo)
                      (commandp mouse-two)
                      (delete-dups
                       (mapcar
                        (lambda (position)
                          (copy-tree
                           (list
                            (get-text-property position 'face)
                            (get-text-property position 'display))))
                        (number-sequence
                         (button-start button)
                         (1- (button-end button))))))))))
             '(("/workspace/src" 1 nil)
               ("/workspace/lib" 3 t)
               ("/workspace/.cache" 2 nil)))))
         (list rendered (nreverse events)))"##;
    let expect = expect![[
        r#"OK (((("/workspace/src" 1 nil) "  🖿 src   \n" 12 " 🖿 src   " #1=(nil) "/workspace/src" t "mouse-1: Fold/unfold directory\nmouse-3: Change root to directory" t ((nil nil) (neo-dir-link-face nil) (nil ((space :align-to (- right 0 3)))) ((:inherit neo-expand-btn-face :height 1.2) nil) (nil ((space :align-to (- right 0 1)))))) (("/workspace/lib" 3 t) "      🖿 lib ◢ \n" 16 "     🖿 lib ◢ " #1# "/workspace/lib" t "mouse-1: Fold/unfold directory\nmouse-3: Change root to directory" t ((nil nil) (neo-dir-link-face nil) (nil ((space :align-to (- right 0 3)))) ((:inherit neo-expand-btn-face :height 1.2) nil) (nil ((space :align-to (- right 0 1)))))) (("/workspace/.cache" 2 nil) "    🖿 .cache   \n" 17 "   🖿 .cache   " #1# "/workspace/.cache" t "mouse-1: Fold/unfold directory\nmouse-3: Change root to directory" t ((nil nil) ((:inherit shadow neo-dir-link-face) nil) (nil ((space :align-to (- right 0 3)))) ((:inherit neo-expand-btn-face :height 1.2) nil) (nil ((space :align-to (- right 0 1))))))) ((short-name "/workspace/src") (hidden-p "/workspace/src") (node-list nil "/workspace/src") newline (open-dir "/workspace/src") (short-name "/workspace/lib") (hidden-p "/workspace/lib") (node-list nil "/workspace/lib") newline (open-dir "/workspace/lib") (short-name "/workspace/.cache") (hidden-p "/workspace/.cache") (node-list nil "/workspace/.cache") newline (open-dir "/workspace/.cache")))"#
    ]];
    assert_adwaita_dark_theme_parity(elisp_form, expect);
}

#[test]
fn adwaita_dark_theme_neotree_file_renderer_covers_indentation_hidden_face_and_open_action() {
    let elisp_form = r##"(let (events rendered)
         (cl-letf (((symbol-function 'neo-path--file-short-name)
                    (lambda (node)
                      (push (list 'short-name node) events)
                      (file-name-nondirectory node)))
                   ((symbol-function 'neo-filepath-hidden-p)
                    (lambda (node)
                      (push (list 'hidden-p node) events)
                      (string-prefix-p "." (file-name-nondirectory node))))
                   ((symbol-function 'neo-buffer--node-list-set)
                    (lambda (&rest arguments)
                      (push (cons 'node-list arguments) events)))
                   ((symbol-function 'neo-buffer--newline-and-begin)
                    (lambda ()
                      (push 'newline events)
                      (insert "\n")
                      (beginning-of-line)))
                   ((symbol-function 'neo-open-file)
                    (lambda (&rest arguments)
                      (push (cons 'open-file arguments) events)
                      'opened)))
           (setq
            rendered
            (mapcar
             (lambda (case)
               (pcase-let ((`(,node ,depth) case))
                 (with-temp-buffer
                   (adwaita-dark-theme--neotree-insert-file
                    node depth)
                   (let* ((button (next-button (point-min)))
                          (keymap (button-get button 'keymap))
                          (mouse-two
                           (lookup-key keymap [mouse-2])))
                     (funcall mouse-two)
                     (list
                     case
                      (buffer-substring-no-properties
                       (point-min)
                       (point-max))
                      (point)
                      (button-label button)
                      (button-get button 'face)
                      (button-get button 'neo-full-path)
                      (button-get button 'follow-link)
                      (button-get button 'help-echo)
                      (commandp mouse-two)
                      (delete-dups
                       (mapcar
                        (lambda (position)
                          (copy-tree
                           (list
                            (get-text-property position 'face)
                            (get-text-property position 'display))))
                        (number-sequence
                         (button-start button)
                         (1- (button-end button))))))))))
             '(("/workspace/main.el" 1)
               ("/workspace/.env" 4)))))
         (list rendered (nreverse events)))"##;
    let expect = expect![[
        r#"OK (((("/workspace/main.el" 1) "  main.el \n" 12 " main.el " neo-file-link-face "/workspace/main.el" t "mouse-1: Open file" t ((nil nil) (nil ((space :align-to (- right 0 1)))))) (("/workspace/.env" 4) "        .env \n" 15 "       .env " (:inherit shadow neo-file-link-face) "/workspace/.env" t "mouse-1: Open file" t ((nil nil) (nil ((space :align-to (- right 0 1))))))) ((short-name "/workspace/main.el") (hidden-p "/workspace/main.el") (node-list nil "/workspace/main.el") newline (open-file "/workspace/main.el") (short-name "/workspace/.env") (hidden-p "/workspace/.env") (node-list nil "/workspace/.env") newline (open-file "/workspace/.env")))"#
    ]];
    assert_adwaita_dark_theme_parity(elisp_form, expect);
}

#[test]
fn adwaita_dark_theme_neotree_configuration_installs_and_runs_real_advice() {
    let elisp_form = r##"(let (fringe-calls)
         (fset 'neo-global--create-window
               (lambda () 'created-window))
         (fset 'neo-buffer--insert-root-entry
               (lambda (&rest arguments)
                 (cons 'original-root arguments)))
         (fset 'neo-buffer--insert-dir-entry
               (lambda (&rest arguments)
                 (cons 'original-dir arguments)))
         (fset 'neo-buffer--insert-file-entry
               (lambda (&rest arguments)
                 (cons 'original-file arguments)))
         (setq neo-global--window 'neotree-window)
         (cl-letf (((symbol-function 'set-window-fringes)
                    (lambda (&rest arguments)
                      (push arguments fringe-calls))))
           (adwaita-dark-theme-neotree-configuration-enable)
           (let ((advice-state
                  (list
                   (and
                    (advice-member-p
                     #'adwaita-dark-theme--neotree-insert-root
                     'neo-buffer--insert-root-entry)
                    t)
                   (and
                    (advice-member-p
                     #'adwaita-dark-theme--neotree-insert-dir
                     'neo-buffer--insert-dir-entry)
                    t)
                   (and
                    (advice-member-p
                     #'adwaita-dark-theme--neotree-insert-file
                     'neo-buffer--insert-file-entry)
                    t))))
             (with-temp-buffer
               (setq cursor-type 'box
                     line-spacing nil
                     mode-line-format '("mode")
                     auto-hscroll-mode t
                     visual-line-mode t)
               (let ((result (neo-global--create-window)))
                 (list
                  result
                  advice-state
                  cursor-type
                  line-spacing
                  mode-line-format
                  auto-hscroll-mode
                  visual-line-mode
                  (display-table-slot
                   buffer-display-table
                   'truncation)
                  (nreverse fringe-calls)))))))"##;
    let expect = expect![
        "OK (created-window (t t t) nil 0.25 nil nil nil 180363302 ((neotree-window 0 0)))"
    ];
    assert_adwaita_dark_theme_parity(elisp_form, expect);
}

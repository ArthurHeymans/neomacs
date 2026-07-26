use expect_test::expect;

use super::assert_evil_parity;

#[test]
fn evil_define_key_creates_and_reuses_state_auxiliary_keymaps() {
    let elisp_form = r##"(progn
               (defvar neomacs-evil-aux-map)
               (setq neomacs-evil-aux-map (make-sparse-keymap))
               (evil-define-key 'normal neomacs-evil-aux-map
                 "f" #'forward-char
                 "b" #'backward-char)
               (let ((aux
                      (evil-get-auxiliary-keymap
                       neomacs-evil-aux-map 'normal)))
                 (list
                  (evil-auxiliary-keymap-p aux)
                  (lookup-key aux "f")
                  (lookup-key aux "b")
                  (eq aux
                      (evil-get-auxiliary-keymap
                       neomacs-evil-aux-map 'normal)))))"##;
    let expect = expect!["OK (t forward-char backward-char t)"];

    assert_evil_parity(elisp_form, expect);
}

#[test]
fn evil_define_key_supports_global_local_and_multiple_state_targets() {
    let elisp_form = r##"(with-temp-buffer
               (let ((evil-normal-state-map
                      (copy-keymap evil-normal-state-map))
                     (evil-insert-state-map
                      (copy-keymap evil-insert-state-map))
                     (evil-normal-state-local-map
                      (make-sparse-keymap))
                     (global-map (copy-keymap global-map)))
                 (use-local-map (make-sparse-keymap))
                 (evil-define-key 'normal 'global "f" #'forward-char)
                 (evil-define-key 'normal 'local "b" #'backward-char)
                 (evil-define-key nil 'global "n" #'next-line)
                 (evil-define-key nil 'local "p" #'previous-line)
                 (evil-define-key '(normal insert) 'global "x" #'ignore)
                 (list
                  (lookup-key evil-normal-state-map "f")
                  (lookup-key evil-normal-state-local-map "b")
                  (lookup-key global-map "n")
                  (lookup-key (current-local-map) "p")
                  (lookup-key evil-normal-state-map "x")
                  (lookup-key evil-insert-state-map "x"))))"##;
    let expect = expect!["OK (forward-char backward-char next-line previous-line ignore ignore)"];

    assert_evil_parity(elisp_form, expect);
}

#[test]
fn evil_define_key_star_updates_existing_maps_without_auxiliary_indirection() {
    let elisp_form = r##"(let ((map (make-sparse-keymap)))
               (evil-define-key* 'normal map
                 "a" #'forward-char
                 "b" #'backward-char)
               (list
                (lookup-key map [normal-state ?a])
                (lookup-key map [normal-state ?b])
                (evil-get-auxiliary-keymap map 'normal)))"##;
    let expect = expect![[
        r#"OK (forward-char backward-char (keymap "Auxiliary keymap for Normal state" (98 . backward-char) (97 . forward-char)))"#
    ]];

    assert_evil_parity(elisp_form, expect);
}

#[test]
fn evil_overriding_and_intercept_maps_record_requested_state_and_precedence() {
    let elisp_form = r##"(let ((override (make-sparse-keymap))
                    (intercept (make-sparse-keymap))
                    (evil-overriding-maps nil)
                    (evil-intercept-maps nil))
               (evil-make-overriding-map override 'normal)
               (evil-make-intercept-map intercept 'insert)
               (list
                (evil-get-property
                 evil-overriding-maps override :states)
                (evil-get-property
                 evil-intercept-maps intercept :states)
                (evil-get-property
                 evil-overriding-maps override :copy)
                (evil-get-property
                 evil-intercept-maps intercept :copy)
                (eq override
                    (caar evil-overriding-maps))
                (eq intercept
                    (caar evil-intercept-maps))))"##;
    let expect = expect!["OK (nil nil nil nil nil nil)"];

    assert_evil_parity(elisp_form, expect);
}

#[test]
fn evil_define_minor_mode_key_builds_state_specific_mode_bindings() {
    let elisp_form = r##"(progn
               (defvar neomacs-evil-minor-mode nil)
               (defvar neomacs-evil-minor-mode-map
                 (make-sparse-keymap))
               (setq neomacs-evil-minor-mode-map
                     (make-sparse-keymap))
               (evil-define-minor-mode-key
                'normal 'neomacs-evil-minor-mode
                "a" #'forward-char
                "b" #'backward-char)
               (let ((aux
                      (evil-get-auxiliary-keymap
                       neomacs-evil-minor-mode-map 'normal)))
                 (list
                  (keymapp aux)
                  (lookup-key aux "a")
                  (lookup-key aux "b")
                  (assq 'neomacs-evil-minor-mode
                        evil-minor-mode-keymaps-alist))))"##;
    let expect = expect!["OK (nil nil nil nil)"];

    assert_evil_parity(elisp_form, expect);
}

#[test]
fn evil_keymap_for_mode_resolves_direct_parent_and_missing_mode_maps() {
    let elisp_form = r##"(progn
               (defvar neomacs-evil-parent-mode-map
                 (make-sparse-keymap))
               (defvar neomacs-evil-child-mode-map nil)
               (put 'neomacs-evil-child-mode
                    'derived-mode-parent
                    'neomacs-evil-parent-mode)
               (list
                (eq (evil-keymap-for-mode 'neomacs-evil-parent-mode)
                    neomacs-evil-parent-mode-map)
                (eq (evil-keymap-for-mode 'neomacs-evil-child-mode)
                    neomacs-evil-parent-mode-map)
                (evil-keymap-for-mode 'neomacs-evil-missing-mode)
                (evil-keymap-for-mode
                 'neomacs-evil-child-mode t)))"##;
    let expect = expect!["OK (nil nil nil nil)"];

    assert_evil_parity(elisp_form, expect);
}

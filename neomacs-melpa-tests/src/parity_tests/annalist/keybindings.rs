use expect_test::expect;

use super::assert_annalist_parity;

#[test]
fn annalist_keybinding_default_view_reports_current_and_previous_definitions() {
    let elisp_form = r##"(progn
         (defvar annalist-test-map (make-sparse-keymap))
         (define-key annalist-test-map (kbd "C-c d") #'delete-region)
         (define-key annalist-test-map (kbd "C-c r") #'replace-string)
         (annalist-record
          'workspace
          'keybindings
          (list
           'annalist-test-map
           nil
           (kbd "C-c d")
           #'duplicate-dwim
           nil))
         (annalist-record
          'workspace
          'keybindings
          (list
           'annalist-test-map
           nil
           (kbd "C-c r")
           '(lambda ()
              (interactive)
              (message "release promoted"))
           nil))
         (let ((stored
                (annalist-test-keybinding-records
                 'workspace)))
           (list
            stored
            (annalist-test-description
             'workspace
             'keybindings))))"##;
    let expect = expect![[
        r#"OK (((annalist-test-map nil "\3d" duplicate-dwim delete-region nil) (annalist-test-map nil "\3r" (lambda nil (interactive) (message "release promoted")) replace-string nil)) (org-mode t 1 318 "* ~annalist-test-map~\n| Key     | Definition       | Previous         |\n|---------+------------------+------------------|\n| =C-c d= | ~duplicate-dwim~ | ~delete-region~  |\n| =C-c r= | [fn:1]           | ~replace-string~ |\n\n[fn:1]\n#+begin_src emacs-lisp\n(lambda nil (interactive) (message release promoted))\n#+end_src\n"))"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_on_change_policy_tracks_the_last_distinct_live_definition() {
    let elisp_form = r##"(progn
         (defvar annalist-test-map (make-sparse-keymap))
         (let ((key (kbd "C-c p")))
           (define-key annalist-test-map key #'previous-line)
           (annalist-record
            'workspace
            'keybindings
            (list
             'annalist-test-map nil key #'project-switch-project nil))
           (define-key
            annalist-test-map
            key
            #'project-switch-project)
           (annalist-record
            'workspace
            'keybindings
            (list
             'annalist-test-map nil key #'project-find-file nil))
           (define-key annalist-test-map key #'project-find-file)
           (annalist-record
            'workspace
            'keybindings
            (list
             'annalist-test-map nil key #'project-find-file nil))
           (let ((record
                  (car
                   (annalist-test-keybinding-records
                    'workspace))))
             (list
              record
              (lookup-key annalist-test-map key)
              (annalist-test-description
               'workspace
               'keybindings)))))"##;
    let expect = expect![[
        r#"OK ((annalist-test-map nil "\3p" project-find-file project-switch-project nil) project-find-file (org-mode t 1 206 "* ~annalist-test-map~\n| Key     | Definition          | Previous                 |\n|---------+---------------------+--------------------------|\n| =C-c p= | ~project-find-file~ | ~project-switch-project~ |\n"))"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_nil_update_policy_retains_the_first_previous_definition() {
    let elisp_form = r##"(progn
         (defvar annalist-test-map (make-sparse-keymap))
         (let ((annalist-update-previous-key-definition nil)
               (key (kbd "C-c f")))
           (define-key annalist-test-map key #'forward-char)
           (annalist-record
            'workspace
            'keybindings
            (list
             'annalist-test-map nil key #'find-file nil))
           (define-key annalist-test-map key #'find-file)
           (annalist-record
            'workspace
            'keybindings
            (list
             'annalist-test-map nil key #'find-file-other-window nil))
           (define-key
            annalist-test-map
            key
            #'find-file-other-window)
           (annalist-record
            'workspace
            'keybindings
            (list
             'annalist-test-map nil key #'find-file-literally nil))
           (let ((record
                  (car
                   (annalist-test-keybinding-records
                    'workspace))))
             (list
              record
              (annalist-test-description
               'workspace
               'keybindings)))))"##;
    let expect = expect![[
        r#"OK ((annalist-test-map nil "\3f" find-file-literally forward-char nil) (org-mode t 1 182 "* ~annalist-test-map~\n| Key     | Definition            | Previous       |\n|---------+-----------------------+----------------|\n| =C-c f= | ~find-file-literally~ | ~forward-char~ |\n"))"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_deferred_keymap_record_becomes_valid_after_the_map_is_defined() {
    let elisp_form = r##"(progn
         (makunbound 'annalist-future-mode-map)
         (annalist-record
          'workspace
          'keybindings
          (list
           'annalist-future-mode-map
           nil
           (kbd "C-c n")
           #'next-error
           nil))
         (let ((before
                (annalist-test-description
                 'workspace
                 'keybindings
                 'valid)))
           (set
            'annalist-future-mode-map
            (make-sparse-keymap))
           (let ((after
                  (annalist-test-description
                   'workspace
                   'keybindings
                   'valid)))
             (list
              before
              after
              (annalist-test-keybinding-records
               'workspace)))))"##;
    let expect = expect![[
        r#"OK ((org-mode t 1 2 "\n") (org-mode t 1 144 "* ~annalist-future-mode-map~\n| Key     | Definition   | Previous |\n|---------+--------------+----------|\n| =C-c n= | ~next-error~ | ~nil~    |\n") ((annalist-future-mode-map nil "\3n" next-error nil nil)))"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_preprocesses_evil_global_and_local_states_through_state_properties() {
    let elisp_form = r##"(cl-letf
         (((symbol-function 'evil-state-property)
           (lambda (state property &optional _noerror)
             (intern
              (format
               "test-%s-%s"
               state
               (if
                   (eq property :keymap)
                   "global-map"
                 "local-map"))))))
         (list
          (annalist--preprocess-keybinding
           (list
            'global
            'normal
            (kbd "g d")
            #'xref-find-definitions
            nil)
           nil)
          (annalist--preprocess-keybinding
           (list
            'local
            'insert
            (kbd "C-c c")
            #'compile
            nil)
           nil)
          (annalist--preprocess-keybinding
           (list
            'project-prefix-map
            nil
            (kbd "f")
            #'project-find-file
            nil)
           nil)))"##;
    let expect = expect![[
        r#"OK ((test-normal-global-map nil "gd" xref-find-definitions nil) (test-insert-local-map nil "\3c" compile nil) (project-prefix-map nil "f" project-find-file nil))"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_get_keymap_and_lookup_cover_global_direct_minor_and_auxiliary_maps() {
    let elisp_form = r##"(let ((annalist-direct-map
                (make-sparse-keymap))
               (annalist-prefix-map
                (make-sparse-keymap)))
         (define-key
          annalist-direct-map
          (kbd "C-c a")
          #'align-regexp)
         (define-key
          annalist-prefix-map
          (kbd "C-c")
          (make-sparse-keymap))
         (cl-letf
             (((symbol-function 'evil-state-property)
               (lambda (state property &optional _noerror)
                 (list :state state :property property)))
              ((symbol-function 'evil-get-minor-mode-keymap)
               (lambda (state mode)
                 (list :minor state mode)))
              ((symbol-function 'evil-get-auxiliary-keymap)
               (lambda (map state create aux)
                 (list
                  :auxiliary
                  (keymapp map)
                  state
                  create
                  aux))))
           (list
            (eq
             (annalist--get-keymap
              nil
              'annalist-direct-map)
             annalist-direct-map)
            (keymapp
             (annalist--get-keymap nil 'global))
            (annalist--get-keymap
             'normal
             'global)
            (annalist--get-keymap
             'insert
             'annalist-test-mode
             t)
            (annalist--get-keymap
             'visual
             'annalist-direct-map)
            (annalist--lookup-key
             annalist-direct-map
             (kbd "C-c a"))
            (annalist--lookup-key
             annalist-prefix-map
             (kbd "C-c x"))
            (annalist--lookup-key nil (kbd "C-c a"))
            (annalist--lookup-key
             annalist-direct-map
             nil))))"##;
    let expect = expect![
        "OK (nil t (:state normal :property :keymap) (:minor insert annalist-test-mode) nil align-regexp nil nil nil)"
    ];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_valid_and_active_views_filter_deferred_and_dormant_maps() {
    let elisp_form = r##"(progn
         (defvar annalist-active-mode-map
           (make-sparse-keymap))
         (defvar annalist-dormant-mode-map
           (make-sparse-keymap))
         (makunbound 'annalist-deferred-mode-map)
         (define-key
          annalist-active-mode-map
          (kbd "C-c a")
          #'align-regexp)
         (define-key
          annalist-dormant-mode-map
          (kbd "C-c d")
          #'delete-duplicate-lines)
         (dolist
             (record
              (list
               (list
                'annalist-active-mode-map
                nil
                (kbd "C-c a")
                #'align-regexp
                nil)
               (list
                'annalist-dormant-mode-map
                nil
                (kbd "C-c d")
                #'delete-duplicate-lines
                nil)
               (list
                'annalist-deferred-mode-map
                nil
                (kbd "C-c f")
                #'find-file
                nil)))
           (annalist-record
            'workspace
            'keybindings
            record))
         (let ((annalist-active-mode t)
               (minor-mode-map-alist
                (cons
                 (cons
                  'annalist-active-mode
                  annalist-active-mode-map)
                 minor-mode-map-alist)))
           (list
            (annalist-test-description
             'workspace
             'keybindings
             'valid)
            (annalist-test-description
             'workspace
             'keybindings
             'active))))"##;
    let expect = expect![[
        r#"OK ((org-mode t 1 397 "* ~annalist-active-mode-map~\n| Key     | Definition     | Previous       |\n|---------+----------------+----------------|\n| =C-c a= | ~align-regexp~ | ~align-regexp~ |\n\n* ~annalist-dormant-mode-map~\n| Key     | Definition               | Previous                 |\n|---------+--------------------------+--------------------------|\n| =C-c d= | ~delete-duplicate-lines~ | ~delete-duplicate-lines~ |\n") (org-mode t 1 2 "\n"))"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_local_keybindings_are_scoped_to_the_buffer_that_recorded_them() {
    let elisp_form = r##"(progn
         (defvar annalist-test-map (make-sparse-keymap))
         (annalist-record
          'workspace
          'keybindings
          (list
           'annalist-test-map
           nil
           (kbd "C-c g")
           #'goto-line
           nil))
         (let ((recording-buffer
                (generate-new-buffer
                 " *annalist-keybinding-context*"))
               local-description
               other-description)
           (unwind-protect
               (progn
                 (with-current-buffer recording-buffer
                   (annalist-record
                    'workspace
                    'keybindings
                    (list
                     'annalist-test-map
                     nil
                     (kbd "C-c l")
                     #'count-lines-page
                     nil)
                    :local t)
                   (setq local-description
                         (annalist-test-description
                          'workspace
                          'keybindings)))
                 (with-temp-buffer
                   (setq other-description
                         (annalist-test-description
                          'workspace
                          'keybindings)))
                 (list
                  local-description
                  other-description))
             (kill-buffer recording-buffer))))"##;
    let expect = expect![[
        r#"OK ((org-mode t 1 308 "* Local\n** ~annalist-test-map~\n| Key     | Definition         | Previous |\n|---------+--------------------+----------|\n| =C-c l= | ~count-lines-page~ | ~nil~    |\n\n* Global\n** ~annalist-test-map~\n| Key     | Definition  | Previous |\n|---------+-------------+----------|\n| =C-c g= | ~goto-line~ | ~nil~    |\n") (org-mode t 1 134 "* ~annalist-test-map~\n| Key     | Definition  | Previous |\n|---------+-------------+----------|\n| =C-c g= | ~goto-line~ | ~nil~    |\n"))"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_keymap_state_validation_handles_no_evil_and_stubbed_evil_sessions() {
    let elisp_form = r##"(let ((annalist-live-map
                (make-sparse-keymap)))
         (set 'annalist-live-map annalist-live-map)
         (list
          (mapcar
           (lambda (keymap)
             (list
              keymap
              (annalist--valid-keymap-p keymap)
              (annalist--active-keymap-p keymap)))
           '(global
             local
             annalist-live-map
             annalist-missing-map))
          (mapcar
           #'annalist--valid-state-p
           '(nil normal insert))
          (cl-letf
              (((symbol-function 'evil-state-p)
                (lambda (state)
                  (memq state '(normal insert)))))
            (let ((features
                   (cons 'evil features))
                  (evil-local-mode t))
              (list
               (mapcar
                #'annalist--valid-state-p
                '(nil normal insert operator))
               (mapcar
                #'annalist--valid-state-and-evil-on-p
                '(nil normal operator)))))))"##;
    let expect = expect![
        "OK (((global #1=(global) #2=(global)) (local (local . #1#) (local . #2#)) (annalist-live-map t nil) (annalist-missing-map nil nil)) (t nil nil) ((t nil nil nil) (t nil nil)))"
    ];

    assert_annalist_parity(elisp_form, expect);
}
